// Data Binding Example
// Demonstrates data sources, bindings, and live UI updates

app {
  window {
    title = "Nemo Data Binding Demo"

    header_bar {
      github_url = "https://github.com/geoffjay/nemo/tree/main/examples/data-binding"
      theme_toggle = true
    }
  }

  theme {
    name = "nord"
    mode = "dark"
  }
}

scripts {
  path = "./scripts"
}

// Data sources configuration
data {
  // Timer source - works without external dependencies
  source "ticker" {
    type     = "timer"
    interval = 1
  }

  // HTTP source - polls a public API periodically
  source "api" {
    type     = "http"
    url      = "https://httpbin.org/get"
    interval = 30
  }

  // Uncomment to use MQTT (requires running broker - see docker-compose.yml)
  source "sensors" {
    type      = "mqtt"
    host      = "localhost"
    port      = 1883
    topics    = ["sensors/#"]
    client_id = "nemo-demo"
  }

  // Uncomment to use Redis pub/sub (requires running Redis)
  source "events" {
    type     = "redis"
    url      = "redis://127.0.0.1:6379"
    channels = ["app-events", "notifications"]
  }

  // Uncomment to use NATS (requires running NATS server)
  source "messages" {
    type     = "nats"
    url      = "nats://127.0.0.1:4222"
    subjects = ["updates.>"]
  }

  // Outbound sink for publishing data
  sink "commands" {
    type  = "mqtt"
    host  = "localhost"
    port  = 1883
    topic = "commands"
  }
}

// Layout
layout {
  type = "stack"

  component "root_panel" {
    type = "stack"
    direction = "vertical"
    margin = 16
    spacing = 8

    component "title" {
      type = "label"
      text = "Data Binding Demo"
    }

    component "main_panel" {
      type    = "panel"
      padding = 16
      shadow = "md"

      component "main_content" {
        type = "stack"
        direction = "vertical"
        spacing = 16

        // ── Timer source ────────────────────────────────────────────────
        // Emits: { tick: i64, timestamp: string }
        component "timer_section" {
          type    = "panel"
          padding = 16
          border  = 2
          border_color = "theme.border"
          shadow = "md"

          component "timer_heading" {
            type = "label"
            text = "Timer Source (ticker)"
          }

          component "tick_count" {
            type = "label"
            text = "Tick: waiting..."

            binding {
              source    = "data.ticker"
              target    = "text"
              transform = "tick"
            }
          }

          component "tick_timestamp" {
            type = "label"
            text = "Timestamp: waiting..."

            binding {
              source    = "data.ticker"
              target    = "text"
              transform = "timestamp"
            }
          }
        }

        // ── HTTP source ─────────────────────────────────────────────────
        // Polls httpbin.org and emits the full JSON response
        component "api_section" {
          type    = "panel"
          padding = 16
          border  = 2
          border_color = "theme.border"
          shadow = "md"

          component "api_heading" {
            type = "label"
            text = "HTTP Source (api)"
          }

          component "api_origin" {
            type = "label"
            text = "Origin: waiting..."

            binding {
              source    = "data.api"
              target    = "text"
              transform = "origin"
            }
          }

          component "api_url" {
            type = "label"
            text = "URL: waiting..."

            binding {
              source    = "data.api"
              target    = "text"
              transform = "url"
            }
          }

          component "api_raw" {
            type         = "text"
            content      = "Waiting for API response..."
            bind_content = "data.api"
          }
        }

        // ── MQTT source ─────────────────────────────────────────────────
        // Emits: { topic: string, payload: string|object }
        // Requires: docker compose up -d mosquitto
        component "mqtt_section" {
          type    = "panel"
          padding = 16
          border  = 2
          border_color = "theme.border"
          shadow = "md"

          component "mqtt_heading" {
            type = "label"
            text = "MQTT Source (sensors)"
          }

          component "mqtt_topic" {
            type = "label"
            text = "Topic: waiting for message..."

            binding {
              source    = "data.sensors"
              target    = "text"
              transform = "topic"
            }
          }

          component "mqtt_payload" {
            type = "label"
            text = "Payload: --"

            binding {
              source    = "data.sensors"
              target    = "text"
              transform = "payload"
            }
          }

          component "mqtt_raw" {
            type         = "text"
            content      = "No MQTT data yet"
            bind_content = "data.sensors"
          }
        }

        // ── Redis source ────────────────────────────────────────────────
        // Emits: { channel: string, payload: string|object }
        // Requires: docker compose up -d redis
        component "redis_section" {
          type    = "panel"
          padding = 16
          border  = 2
          border_color = "theme.border"
          shadow = "md"

          component "redis_heading" {
            type = "label"
            text = "Redis Source (events)"
          }

          component "redis_channel" {
            type = "label"
            text = "Channel: waiting for message..."

            binding {
              source    = "data.events"
              target    = "text"
              transform = "channel"
            }
          }

          component "redis_payload" {
            type = "label"
            text = "Payload: --"

            binding {
              source    = "data.events"
              target    = "text"
              transform = "payload"
            }
          }

          component "redis_raw" {
            type         = "text"
            content      = "No Redis data yet"
            bind_content = "data.events"
          }
        }

        // ── NATS source ─────────────────────────────────────────────────
        // Emits: { subject: string, payload: string|object }
        // Requires: docker compose up -d nats
        component "nats_section" {
          type    = "panel"
          padding = 16
          border  = 2
          border_color = "theme.border"
          shadow = "md"

          component "nats_heading" {
            type = "label"
            text = "NATS Source (messages)"
          }

          component "nats_subject" {
            type = "label"
            text = "Subject: waiting for message..."

            binding {
              source    = "data.messages"
              target    = "text"
              transform = "subject"
            }
          }

          component "nats_payload" {
            type = "label"
            text = "Payload: --"

            binding {
              source    = "data.messages"
              target    = "text"
              transform = "payload"
            }
          }

          component "nats_raw" {
            type         = "text"
            content      = "No NATS data yet"
            bind_content = "data.messages"
          }
        }

        // ── Interactive section ─────────────────────────────────────────
        component "actions_section" {
          type    = "panel"
          padding = 16
          border  = 2
          border_color = "theme.border"
          shadow = "md"

          component "actions_heading" {
            type = "label"
            text = "Actions"
          }

          component "refresh_btn" {
            type     = "button"
            label    = "Read Data"
            on_click = "on_read_data"
          }

          component "write_btn" {
            type     = "button"
            label    = "Write Data"
            on_click = "on_write_data"
          }

          component "result_display" {
            type = "label"
            text = "Click a button to interact with data sources"
          }
        }
      }
    }
  }
}
