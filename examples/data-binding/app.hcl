// Data Binding Example
// Demonstrates data sources, bindings, and live UI updates

app {
  window {
    title = "Nemo Data Binding Demo"
    width = 900
    height = 600
    header_bar {
      theme_toggle = true
    }
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
  // source "sensors" {
  //   type      = "mqtt"
  //   host      = "localhost"
  //   port      = 1883
  //   topics    = ["sensors/#"]
  //   client_id = "nemo-demo"
  // }

  // Uncomment to use Redis pub/sub (requires running Redis)
  // source "events" {
  //   type     = "redis"
  //   url      = "redis://127.0.0.1:6379"
  //   channels = ["app-events", "notifications"]
  // }

  // Uncomment to use NATS (requires running NATS server)
  // source "messages" {
  //   type     = "nats"
  //   url      = "nats://127.0.0.1:4222"
  //   subjects = ["updates.>"]
  // }

  // Outbound sink for publishing data
  // sink "commands" {
  //   type  = "mqtt"
  //   host  = "localhost"
  //   port  = 1883
  //   topic = "commands"
  // }
}

// Layout
layout {
  type = "stack"

  component "main_panel" {
    type    = "panel"
    padding = 16

    component "title" {
      type = "label"
      text = "Data Binding Demo"
    }

    // Timer display - bound to the ticker data source
    component "timer_section" {
      type    = "panel"
      padding = 8

      component "timer_label" {
        type = "label"
        text = "Timer Tick Count:"
      }

      component "tick_display" {
        type     = "label"
        text     = "Waiting for data..."
        bind_text = "data.ticker"

        binding {
          source    = "data.ticker"
          target    = "text"
          transform = "payload.tick"
        }
      }
    }

    // API response display
    component "api_section" {
      type    = "panel"
      padding = 8

      component "api_label" {
        type = "label"
        text = "HTTP API Status:"
      }

      component "api_display" {
        type     = "label"
        text     = "Waiting for API response..."
        bind_text = "data.api"
      }
    }

    // Interactive section
    component "actions_section" {
      type    = "panel"
      padding = 8

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
