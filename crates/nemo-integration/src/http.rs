//! HTTP client integration.

use crate::error::IntegrationError;
use reqwest::{Client, Method, Response};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// HTTP client for making requests to external services.
pub struct HttpClient {
    /// The underlying reqwest client.
    client: Client,
    /// Base URL for requests.
    base_url: Option<String>,
    /// Default headers.
    default_headers: HashMap<String, String>,
    /// Default timeout.
    timeout: Duration,
}

/// HTTP request builder.
pub struct HttpRequest {
    /// HTTP method.
    pub method: Method,
    /// URL path (appended to base URL).
    pub path: String,
    /// Query parameters.
    pub query: HashMap<String, String>,
    /// Headers.
    pub headers: HashMap<String, String>,
    /// Request body.
    pub body: Option<String>,
    /// Timeout override.
    pub timeout: Option<Duration>,
}

/// HTTP response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    /// Status code.
    pub status: u16,
    /// Response headers.
    pub headers: HashMap<String, String>,
    /// Response body as string.
    pub body: String,
}

impl HttpClient {
    /// Creates a new HTTP client.
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: None,
            default_headers: HashMap::new(),
            timeout: Duration::from_secs(30),
        }
    }

    /// Creates an HTTP client with a base URL.
    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: Some(base_url.into()),
            default_headers: HashMap::new(),
            timeout: Duration::from_secs(30),
        }
    }

    /// Sets the default timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Adds a default header.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.default_headers.insert(name.into(), value.into());
        self
    }

    /// Executes an HTTP request.
    pub async fn execute(&self, request: HttpRequest) -> Result<HttpResponse, IntegrationError> {
        let url = match &self.base_url {
            Some(base) => format!("{}{}", base.trim_end_matches('/'), request.path),
            None => request.path.clone(),
        };

        let mut builder = self.client.request(request.method, &url);

        // Add default headers
        for (name, value) in &self.default_headers {
            builder = builder.header(name, value);
        }

        // Add request headers
        for (name, value) in &request.headers {
            builder = builder.header(name, value);
        }

        // Add query parameters
        if !request.query.is_empty() {
            builder = builder.query(&request.query);
        }

        // Add body
        if let Some(body) = request.body {
            builder = builder.body(body);
        }

        // Set timeout
        let timeout = request.timeout.unwrap_or(self.timeout);
        builder = builder.timeout(timeout);

        let response = builder.send().await?;
        Self::convert_response(response).await
    }

    /// Performs a GET request.
    pub async fn get(&self, path: &str) -> Result<HttpResponse, IntegrationError> {
        self.execute(HttpRequest {
            method: Method::GET,
            path: path.to_string(),
            query: HashMap::new(),
            headers: HashMap::new(),
            body: None,
            timeout: None,
        })
        .await
    }

    /// Performs a POST request with JSON body.
    pub async fn post_json<T: Serialize>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<HttpResponse, IntegrationError> {
        let body = serde_json::to_string(body)?;
        self.execute(HttpRequest {
            method: Method::POST,
            path: path.to_string(),
            query: HashMap::new(),
            headers: HashMap::from([("Content-Type".to_string(), "application/json".to_string())]),
            body: Some(body),
            timeout: None,
        })
        .await
    }

    /// Performs a PUT request with JSON body.
    pub async fn put_json<T: Serialize>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<HttpResponse, IntegrationError> {
        let body = serde_json::to_string(body)?;
        self.execute(HttpRequest {
            method: Method::PUT,
            path: path.to_string(),
            query: HashMap::new(),
            headers: HashMap::from([("Content-Type".to_string(), "application/json".to_string())]),
            body: Some(body),
            timeout: None,
        })
        .await
    }

    /// Performs a DELETE request.
    pub async fn delete(&self, path: &str) -> Result<HttpResponse, IntegrationError> {
        self.execute(HttpRequest {
            method: Method::DELETE,
            path: path.to_string(),
            query: HashMap::new(),
            headers: HashMap::new(),
            body: None,
            timeout: None,
        })
        .await
    }

    /// Converts a reqwest Response to our HttpResponse.
    async fn convert_response(response: Response) -> Result<HttpResponse, IntegrationError> {
        let status = response.status().as_u16();
        let headers: HashMap<String, String> = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let body = response.text().await?;

        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpRequest {
    /// Creates a new GET request.
    pub fn get(path: impl Into<String>) -> Self {
        Self {
            method: Method::GET,
            path: path.into(),
            query: HashMap::new(),
            headers: HashMap::new(),
            body: None,
            timeout: None,
        }
    }

    /// Creates a new POST request.
    pub fn post(path: impl Into<String>) -> Self {
        Self {
            method: Method::POST,
            path: path.into(),
            query: HashMap::new(),
            headers: HashMap::new(),
            body: None,
            timeout: None,
        }
    }

    /// Adds a query parameter.
    pub fn query(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.query.insert(name.into(), value.into());
        self
    }

    /// Adds a header.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    /// Sets the body.
    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Sets the timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
}

impl HttpResponse {
    /// Parses the body as JSON.
    pub fn json<T: for<'de> Deserialize<'de>>(&self) -> Result<T, IntegrationError> {
        serde_json::from_str(&self.body).map_err(|e| e.into())
    }

    /// Returns true if the status is successful (2xx).
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = HttpClient::new();
        assert!(client.base_url.is_none());
    }

    #[test]
    fn test_client_with_base_url() {
        let client = HttpClient::with_base_url("https://api.example.com");
        assert_eq!(client.base_url, Some("https://api.example.com".to_string()));
    }

    #[test]
    fn test_request_builder() {
        let request = HttpRequest::get("/users")
            .query("page", "1")
            .header("Accept", "application/json");

        assert_eq!(request.path, "/users");
        assert_eq!(request.query.get("page"), Some(&"1".to_string()));
        assert_eq!(
            request.headers.get("Accept"),
            Some(&"application/json".to_string())
        );
    }
}
