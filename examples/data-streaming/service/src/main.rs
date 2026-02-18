use axum::{extract::State, response::Json, routing::get, Router};
use rand::Rng;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    streaming: Arc<AtomicBool>,
}

#[derive(Serialize)]
struct StatusResponse {
    status: &'static str,
}

async fn start_streaming(State(state): State<AppState>) -> Json<StatusResponse> {
    state.streaming.store(true, Ordering::Relaxed);
    Json(StatusResponse { status: "started" })
}

async fn stop_streaming(State(state): State<AppState>) -> Json<StatusResponse> {
    state.streaming.store(false, Ordering::Relaxed);
    Json(StatusResponse { status: "stopped" })
}

async fn streaming_status(State(state): State<AppState>) -> Json<StatusResponse> {
    let active = state.streaming.load(Ordering::Relaxed);
    Json(StatusResponse {
        status: if active { "streaming" } else { "stopped" },
    })
}

/// Generate a metric value for a given channel using various waveforms.
fn generate_metric(channel: usize, tick: u64) -> f64 {
    let t = tick as f64 * 0.1;
    let mut rng = rand::thread_rng();
    let noise = rng.gen_range(-0.5..0.5);

    match channel % 5 {
        // Sine wave with varying frequency
        0 => 50.0 + 20.0 * (t * (0.3 + channel as f64 * 0.05)).sin() + noise,
        // Cosine wave
        1 => 40.0 + 15.0 * (t * (0.2 + channel as f64 * 0.03)).cos() + noise,
        // Sawtooth
        2 => {
            let period = 20.0 + channel as f64 * 2.0;
            let phase = (t % period) / period;
            30.0 + 25.0 * phase + noise
        }
        // Triangle wave
        3 => {
            let period = 15.0 + channel as f64 * 1.5;
            let phase = (t % period) / period;
            let tri = if phase < 0.5 {
                phase * 2.0
            } else {
                2.0 - phase * 2.0
            };
            35.0 + 20.0 * tri + noise
        }
        // Square wave with smoothing
        _ => {
            let period = 25.0 + channel as f64 * 3.0;
            let phase = (t % period) / period;
            (if phase < 0.5 { 60.0 } else { 20.0 }) + noise
        }
    }
}

fn unit_for_channel(channel: usize) -> &'static str {
    match channel % 5 {
        0 => "celsius",
        1 => "percent",
        2 => "psi",
        3 => "rpm",
        _ => "volts",
    }
}

#[tokio::main]
async fn main() {
    let nats_url =
        std::env::var("NATS_URL").unwrap_or_else(|_| "nats://127.0.0.1:4222".to_string());
    let num_channels: usize = std::env::var("NUM_CHANNELS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20);
    let data_rate_ms: u64 = std::env::var("DATA_RATE_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(250);
    let service_port: u16 = std::env::var("SERVICE_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3000);

    let state = AppState {
        streaming: Arc::new(AtomicBool::new(false)),
    };

    // Spawn the NATS publishing task
    let streaming_flag = state.streaming.clone();
    let nats_url_clone = nats_url.clone();
    tokio::spawn(async move {
        // Retry NATS connection with backoff
        let client = loop {
            match async_nats::connect(&nats_url_clone).await {
                Ok(c) => {
                    eprintln!("Connected to NATS at {nats_url_clone}");
                    break c;
                }
                Err(e) => {
                    eprintln!("NATS connect failed ({e}), retrying in 2s...");
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        };

        let mut tick: u64 = 0;
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(data_rate_ms)).await;

            if !streaming_flag.load(Ordering::Relaxed) {
                continue;
            }

            tick += 1;
            let timestamp = chrono::Utc::now().to_rfc3339();

            // Build a single batch message containing all channels
            let mut channels = serde_json::Map::new();
            for ch in 0..num_channels {
                let value = generate_metric(ch, tick);
                let channel_name = format!("channel_{ch}");
                channels.insert(
                    channel_name,
                    serde_json::json!({
                        "value": (value * 1000.0).round() / 1000.0,
                        "unit": unit_for_channel(ch),
                    }),
                );
            }

            let payload = serde_json::json!({
                "channels": channels,
                "timestamp": timestamp,
            });

            if let Err(e) = client
                .publish("metrics.batch", payload.to_string().into())
                .await
            {
                eprintln!("NATS publish error: {e}");
            }
        }
    });

    let app = Router::new()
        .route("/streaming/start", get(start_streaming))
        .route("/streaming/stop", get(stop_streaming))
        .route("/streaming/status", get(streaming_status))
        .with_state(state);

    let addr = format!("0.0.0.0:{service_port}");
    eprintln!("Streaming service listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
