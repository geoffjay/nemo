//! HTTP server data source.
//!
//! Runs a lightweight HTTP/1.1 server (hand-rolled over tokio's `TcpListener`,
//! no extra dependencies) inside the data-source machinery. Each inbound
//! request becomes a `DataUpdate` carrying a request object, and the app's
//! script responds via [`send_response`]:
//!
//! ```text
//! client → HttpServerSource accept loop → DataUpdate(full, req)
//!        → DataFlowEngine.process_update → repository data.<id>
//!        → runtime bridge calls the `on-request` Rhai handler
//!        → nemo_data::send_response → dispatch task → HTTP client
//! ```
//!
//! The repository holds the **latest** request at `data.<id>` (so bindings and
//! `n:for` see it); responses are matched by request id, not repository state.
//!
//! Limitations by design: HTTP/1.1 only (no TLS, no HTTP/2), chunked request
//! bodies unsupported (Content-Length only), requests that go unanswered within
//! the timeout get a 408. Requests handed to scripts run on the main/UI thread
//! (see the runtime bridge in `crates/nemo/src/runtime.rs`), so a slow handler
//! delays UI updates — keep handlers light.

use crate::error::DataSourceError;
use crate::source::{DataSchema, DataSource, DataUpdate, SchemaType, SourceStatus};
use async_trait::async_trait;
use nemo_config::Value;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::sync::{broadcast, oneshot, Mutex as AsyncMutex, RwLock, Semaphore};
use tokio::task::JoinHandle;

/// A computed HTTP response for one pending request.
#[derive(Debug, Clone)]
pub struct ServerResponse {
    /// Request id this answers (`id` field of the emitted request object).
    pub request_id: String,
    /// HTTP status code.
    pub status: u16,
    /// Response body.
    pub body: String,
    /// `Content-Type` (default `application/json`).
    pub content_type: Option<String>,
}

/// Registry of response channels by server id. Populated at `start()`, cleared
/// at `stop()`. A global because `create_source` builds sources without
/// runtime-owned wiring, and `ExtensionManager` is `!Send` so channels can't
/// ride the `PluginContext`.
static RESPONSE_SENDERS: LazyLock<Mutex<HashMap<String, mpsc::Sender<ServerResponse>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Sends a scripted response for a pending HTTP-server request.
///
/// Called by the runtime bridge after the `on-request` handler returns.
/// Returns `false` when no such server is running or the request already
/// timed out.
pub fn send_response(server_id: &str, response: ServerResponse) -> bool {
    match RESPONSE_SENDERS.lock().unwrap().get(server_id) {
        Some(tx) => tx.try_send(response).is_ok(),
        None => false,
    }
}

/// Configuration for an HTTP server data source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpServerSourceConfig {
    /// Unique ID for this source.
    pub id: String,
    /// Bind address. Default `127.0.0.1`.
    #[serde(default = "default_host")]
    pub host: String,
    /// Bind port. Default `8080`.
    #[serde(default = "default_port")]
    pub port: u16,
    /// How long a request may await a scripted response before 408.
    #[serde(default = "default_timeout", with = "duration_millis")]
    pub timeout: Duration,
    /// Maximum concurrent in-flight requests (429 above this).
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: usize,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_max_concurrent() -> usize {
    16
}

mod duration_millis {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub(crate) fn serialize<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u64(d.as_millis() as u64)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
        Ok(Duration::from_millis(u64::deserialize(d)?))
    }
}

impl Default for HttpServerSourceConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            host: default_host(),
            port: default_port(),
            timeout: default_timeout(),
            max_concurrent: default_max_concurrent(),
        }
    }
}

/// Parsed inbound request, as emitted into the repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerRequest {
    /// Unique per-server request id (`r1`, `r2`, …).
    pub id: String,
    /// HTTP method.
    pub method: String,
    /// Path without the query string.
    pub path: String,
    /// Raw query string without the leading `?` (may be empty).
    #[serde(default)]
    pub query: String,
    /// Request headers, keyed by lowercase name.
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Parsed JSON body when valid JSON, else the raw body string.
    #[serde(default)]
    pub body: Value,
}

/// HTTP server data source.
pub struct HttpServerSource {
    config: HttpServerSourceConfig,
    status: Arc<RwLock<SourceStatus>>,
    sender: broadcast::Sender<DataUpdate>,
    schema: DataSchema,
    task: Option<JoinHandle<()>>,
    next_request_id: Arc<AsyncMutex<u64>>,
}

impl HttpServerSource {
    /// Creates a new HTTP server source.
    pub fn new(config: HttpServerSourceConfig) -> Self {
        Self {
            schema: DataSchema {
                name: format!("http-server:{}", config.id),
                description: "Inbound HTTP requests served by a Rhai handler".to_string(),
                value_type: SchemaType::Object,
            },
            status: Arc::new(RwLock::new(SourceStatus::Disconnected)),
            config,
            sender: broadcast::channel(64).0,
            task: None,
            next_request_id: Arc::new(AsyncMutex::new(1)),
        }
    }
}

/// Status text for common codes.
fn status_text(code: u16) -> &'static str {
    match code {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        _ => "",
    }
}

/// Reads one HTTP/1.1 request from `stream`. Content-Length bodies only.
async fn read_request(stream: &mut TcpStream) -> Result<ServerRequest, String> {
    let mut buf = Vec::with_capacity(1024);
    let mut chunk = [0u8; 4096];
    let header_end = loop {
        let n = stream
            .read(&mut chunk)
            .await
            .map_err(|e| format!("read error: {e}"))?;
        if n == 0 {
            return Err("connection closed before request completed".to_string());
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(pos) = find_subslice(&buf, b"\r\n\r\n") {
            break pos;
        }
        if buf.len() > 64 * 1024 {
            return Err("request head too large".to_string());
        }
    };

    let head = String::from_utf8_lossy(&buf[..header_end]).to_string();
    let mut lines = head.split("\r\n");
    let request_line = lines.next().unwrap_or("");
    let mut parts = request_line.split_whitespace();
    let (method, target, _version) = match (parts.next(), parts.next(), parts.next()) {
        (Some(m), Some(t), Some(v)) if v.starts_with("HTTP/") => (m, t, v),
        _ => return Err(format!("malformed request line: {request_line}")),
    };

    let mut headers = HashMap::new();
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }

    let content_length: usize = headers
        .get("content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let mut body_bytes = buf[header_end + 4..].to_vec();
    while body_bytes.len() < content_length {
        let n = stream
            .read(&mut chunk)
            .await
            .map_err(|e| format!("read error: {e}"))?;
        if n == 0 {
            break;
        }
        body_bytes.extend_from_slice(&chunk[..n]);
    }
    body_bytes.truncate(content_length);

    let (path, query) = match target.split_once('?') {
        Some((p, q)) => (p.to_string(), q.to_string()),
        None => (target.to_string(), String::new()),
    };
    let body_text = String::from_utf8_lossy(&body_bytes).to_string();
    let body = serde_json::from_str::<serde_json::Value>(&body_text)
        .map(Value::from)
        .unwrap_or_else(|_| Value::String(body_text));

    Ok(ServerRequest {
        id: String::new(), // filled by the accept loop
        method: method.to_string(),
        path,
        query,
        headers,
        body,
    })
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Writes an HTTP response.
async fn write_response(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &str,
) -> Result<(), String> {
    let text = status_text(status);
    let reason = if text.is_empty() { "" } else { text };
    let head = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        status,
        if reason.is_empty() { "OK" } else { reason },
        content_type,
        body.len()
    );
    stream
        .write_all(head.as_bytes())
        .await
        .map_err(|e| format!("write error: {e}"))?;
    stream
        .write_all(body.as_bytes())
        .await
        .map_err(|e| format!("write error: {e}"))?;
    stream
        .flush()
        .await
        .map_err(|e| format!("write error: {e}"))?;
    Ok(())
}

#[async_trait]
impl DataSource for HttpServerSource {
    fn id(&self) -> &str {
        &self.config.id
    }

    fn schema(&self) -> &DataSchema {
        &self.schema
    }

    async fn start(&mut self) -> Result<(), DataSourceError> {
        if self.task.is_some() {
            return Err(DataSourceError::AlreadyRunning);
        }

        let addr = format!("{}:{}", self.config.host, self.config.port);
        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| DataSourceError::Connection(e.to_string()))?;
        *self.status.write().await = SourceStatus::Connected;

        let config = self.config.clone();
        let status = self.status.clone();
        let sender = self.sender.clone();
        let next_request_id = self.next_request_id.clone();

        let (response_tx, mut response_rx) = mpsc::channel::<ServerResponse>(64);
        RESPONSE_SENDERS
            .lock()
            .unwrap()
            .insert(config.id.clone(), response_tx);

        let task = tokio::spawn(async move {
            // Pending per-request reply channels, shared with connection tasks.
            let pending: Arc<AsyncMutex<HashMap<String, oneshot::Sender<ServerResponse>>>> =
                Arc::new(AsyncMutex::new(HashMap::new()));
            let permits = Arc::new(Semaphore::new(config.max_concurrent));

            // Response dispatcher: routes scripted responses to waiting
            // connection tasks.
            tokio::spawn({
                let pending = Arc::clone(&pending);
                async move {
                    while let Some(response) = response_rx.recv().await {
                        if let Some(tx) = pending.lock().await.remove(&response.request_id) {
                            let _ = tx.send(response);
                        }
                    }
                }
            });

            loop {
                let permit = permits.clone().acquire_owned().await.unwrap();
                let (stream, _) = match listener.accept().await {
                    Ok(pair) => pair,
                    Err(_) => {
                        drop(permit);
                        continue;
                    }
                };
                let config = config.clone();
                let status = status.clone();
                let sender = sender.clone();
                let next_request_id = next_request_id.clone();
                let pending = Arc::clone(&pending);

                tokio::spawn(async move {
                    let mut stream = stream;
                    let request = match read_request(&mut stream).await {
                        Ok(req) => req,
                        Err(err) => {
                            let _ = write_response(&mut stream, 400, "text/plain", &err).await;
                            return;
                        }
                    };

                    let mut counter = next_request_id.lock().await;
                    let request_id = format!("r{}", *counter);
                    *counter += 1;
                    drop(counter);
                    let request = ServerRequest {
                        id: request_id.clone(),
                        ..request
                    };

                    // Overload guard.
                    if pending.lock().await.len() >= config.max_concurrent {
                        let _ = write_response(
                            &mut stream,
                            429,
                            "text/plain",
                            "too many in-flight requests",
                        )
                        .await;
                        return;
                    }

                    let (reply_tx, reply_rx) = oneshot::channel::<ServerResponse>();
                    pending.lock().await.insert(request_id.clone(), reply_tx);

                    // Emit the request as a full update: repository data.<id>
                    // becomes the latest request object.
                    let payload = request_payload_value(&request);
                    let _ = sender.send(DataUpdate::full(&config.id, payload));

                    match tokio::time::timeout(config.timeout, reply_rx).await {
                        Ok(Ok(response)) => {
                            let content_type = response
                                .content_type
                                .unwrap_or_else(|| "application/json".to_string());
                            let _ = write_response(
                                &mut stream,
                                response.status,
                                &content_type,
                                &response.body,
                            )
                            .await;
                        }
                        _ => {
                            pending.lock().await.remove(&request_id);
                            let _ = write_response(
                                &mut stream,
                                408,
                                "text/plain",
                                "request timed out awaiting response",
                            )
                            .await;
                        }
                    }

                    *status.write().await = SourceStatus::Connected;
                    drop(permit);
                });
            }
        });

        self.task = Some(task);
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), DataSourceError> {
        if let Some(task) = self.task.take() {
            task.abort();
        }
        RESPONSE_SENDERS.lock().unwrap().remove(&self.config.id);
        *self.status.write().await = SourceStatus::Disconnected;
        Ok(())
    }

    fn subscribe(&self) -> broadcast::Receiver<DataUpdate> {
        self.sender.subscribe()
    }

    fn status(&self) -> SourceStatus {
        // Use try_read to avoid blocking
        self.status
            .try_read()
            .map(|s| s.clone())
            .unwrap_or(SourceStatus::Disconnected)
    }

    async fn refresh(&mut self) -> Result<(), DataSourceError> {
        // Nothing to poll: requests arrive by connection.
        Ok(())
    }
}

/// Builds the repository payload for a request.
fn request_payload_value(request: &ServerRequest) -> Value {
    use indexmap::IndexMap;
    let mut obj = IndexMap::new();
    obj.insert("id".to_string(), Value::String(request.id.clone()));
    obj.insert("method".to_string(), Value::String(request.method.clone()));
    obj.insert("path".to_string(), Value::String(request.path.clone()));
    if !request.query.is_empty() {
        obj.insert("query".to_string(), Value::String(request.query.clone()));
    }
    let headers: IndexMap<String, Value> = request
        .headers
        .iter()
        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
        .collect();
    obj.insert("headers".to_string(), Value::Object(headers));
    obj.insert("body".to_string(), request.body.clone());
    Value::Object(obj)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source_on(port: u16) -> HttpServerSource {
        HttpServerSource::new(HttpServerSourceConfig {
            id: "api".to_string(),
            host: "127.0.0.1".to_string(),
            port,
            timeout: Duration::from_secs(5),
            max_concurrent: 4,
        })
    }

    #[tokio::test]
    async fn serves_scripted_response() {
        // Reserve a port, then release it for the source.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let mut source = source_on(port);
        let mut rx = source.subscribe();
        source.start().await.unwrap();

        let client = tokio::spawn(async move {
            let response = reqwest::get(format!("http://127.0.0.1:{port}/tasks"))
                .await
                .unwrap();
            (response.status(), response.text().await.unwrap())
        });

        // The request surfaced as a full update.
        let request = rx.recv().await.unwrap().data;
        let obj = request.as_object().unwrap();
        let id = obj.get("id").and_then(|v| v.as_str()).unwrap().to_string();
        let id_for_stop_check = id.clone();
        assert_eq!(obj.get("method").and_then(|v| v.as_str()), Some("GET"));
        assert_eq!(obj.get("path").and_then(|v| v.as_str()), Some("/tasks"));

        assert!(send_response(
            "api",
            ServerResponse {
                request_id: id,
                status: 200,
                body: "{\"ok\":true}".to_string(),
                content_type: None,
            },
        ));

        let (status, body) = client.await.unwrap();
        assert_eq!(status, 200);
        assert_eq!(body, "{\"ok\":true}");
        source.stop().await.unwrap();
        assert!(!send_response(
            "api",
            ServerResponse {
                request_id: id_for_stop_check,
                status: 200,
                body: String::new(),
                content_type: None,
            },
        ));
    }

    #[tokio::test]
    async fn times_out_unanswered_request() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let mut source = HttpServerSource::new(HttpServerSourceConfig {
            id: "slow".to_string(),
            host: "127.0.0.1".to_string(),
            port,
            timeout: Duration::from_millis(150),
            max_concurrent: 4,
        });
        let mut rx = source.subscribe();
        source.start().await.unwrap();

        let response = reqwest::get(format!("http://127.0.0.1:{port}/ping"))
            .await
            .unwrap();
        assert_eq!(response.status(), 408);
        // The request still surfaced as an update.
        let update = rx.recv().await.unwrap();
        assert_eq!(
            update
                .data
                .as_object()
                .unwrap()
                .get("path")
                .and_then(|v| v.as_str()),
            Some("/ping")
        );
        source.stop().await.unwrap();
    }
}
