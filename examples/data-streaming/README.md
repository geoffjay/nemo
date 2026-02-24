# Data Streaming Example

Live data streaming with NATS, real-time charts, and plugin-computed rolling statistics.

## Architecture

```
┌─────────────────┐     NATS      ┌──────────────────────────────┐
│  Streaming      │──────────────▶│  Nemo                        │
│  Service        │  metrics.>    │                              │
│  (axum + NATS)  │               │  ┌────────────────────────┐  │
│                 │               │  │ NATS Data Source       │  │
│  REST API:      │               │  │ (data.metrics)         │  │
│  /streaming/*   │               │  └──────────┬─────────────┘  │
└─────────────────┘               │             │                │
                                  │  ┌──────────▼─────────────┐  │
                                  │  │ streaming-stats plugin │  │
                                  │  │ (60s rolling window)   │  │
                                  │  │ → stats.channel_N.*    │  │
                                  │  │ → stats.summary        │  │
                                  │  └──────────┬─────────────┘  │
                                  │             │                │
                                  │  ┌──────────▼─────────────┐  │
                                  │  │ Dashboard (app.xml)    │  │
                                  │  │ - Area charts          │  │
                                  │  │ - Stats labels         │  │
                                  │  │ - Summary table        │  │
                                  │  └────────────────────────┘  │
                                  └──────────────────────────────┘
```

## Prerequisites

- Docker and Docker Compose
- Rust toolchain (for building Nemo)

## Quick Start

1. Start the infrastructure:

   ```sh
   cd examples/data-streaming
   docker compose up -d
   ```

2. Begin streaming metrics:

   ```sh
   curl http://localhost:3000/streaming/start
   ```

3. Build the plugin and set up the extension directory:

   ```sh
   # From the project root
   cargo build -p streaming-stats-plugin
   mkdir -p target/debug/plugins
   ln -sf ../libstreaming_stats_plugin.dylib target/debug/plugins/
   ```

4. Launch the dashboard:

   ```sh
   cargo run -p nemo -- --app-config examples/data-streaming/app.xml -e target/debug
   ```

5. Stop streaming:

   ```sh
   curl http://localhost:3000/streaming/stop
   ```

6. Tear down:

   ```sh
   cd examples/data-streaming
   docker compose down
   ```

## Streaming Service API

| Endpoint               | Description                    |
|------------------------|--------------------------------|
| `GET /streaming/start` | Start publishing metrics       |
| `GET /streaming/stop`  | Stop publishing metrics        |
| `GET /streaming/status`| Check current streaming state  |

## Environment Variables

| Variable       | Default                     | Description                          |
|----------------|-----------------------------|--------------------------------------|
| `NATS_URL`     | `nats://127.0.0.1:4222`    | NATS server connection URL           |
| `NUM_CHANNELS` | `20`                        | Number of metric channels to publish |
| `DATA_RATE_MS` | `250`                       | Milliseconds between publish rounds  |
| `SERVICE_PORT` | `3000`                      | HTTP port for the REST API           |

## NATS Monitoring

NATS exposes a monitoring endpoint at [http://localhost:8222](http://localhost:8222) for inspecting connections, subscriptions, and message rates.

## Plugin: streaming-stats

The `streaming-stats` plugin reads incoming NATS messages from `data.metrics`, maintains a per-channel sliding window of 60 seconds, and computes:

- **mean** - Average value over the window
- **min** / **max** - Extremes over the window
- **stddev** - Standard deviation (sample)
- **count** - Number of samples in the window

Statistics are written to the data repository at `stats.channel_N.*` and a summary array at `stats.summary` for table display.
