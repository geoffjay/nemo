// Data Streaming Example
// Demonstrates live data streaming with NATS, real-time charts, and plugin-computed statistics

app {
  window {
    title = "Data Streaming Dashboard"

    header_bar {
      github_url = "https://github.com/geoffjay/nemo/tree/main/examples/data-streaming"
      theme_toggle = true
    }
  }

  theme {
    name = "kanagawa"
    mode = "dark"
  }

  plugins = ["streaming-stats"]
}

// Data sources configuration
data {
  source "metrics" {
    type     = "nats"
    url      = "nats://127.0.0.1:4222"
    subjects = ["metrics.>"]
  }
}

// Layout
layout {
  type = "stack"

  component "dashboard" {
    type      = "stack"
    direction = "vertical"
    margin    = 16
    spacing   = 16
    scroll    = true

    // ── Header ──────────────────────────────────────────────────────
    component "header" {
      type = "stack"
      direction = "horizontal"
      spacing = 16

      component "title" {
        type = "label"
        text = "Data Streaming Dashboard"
        size = "xl"
      }

      component "status_label" {
        type = "label"
        text = "NATS: waiting for data..."

        binding {
          source    = "data.metrics"
          target    = "text"
          transform = "subject"
        }
      }
    }

    // ── Charts Section ──────────────────────────────────────────────
    component "charts_heading" {
      type = "label"
      text = "Live Metrics"
      size = "lg"
    }

    // Chart: Mean / Min / Max across all channels
    component "chart_group_1" {
      type    = "panel"
      padding = 16
      border  = 2
      border_color = "theme.border"
      shadow = "md"

      component "chart_group_1_content" {
        type      = "stack"
        direction = "vertical"
        spacing   = 8

        component "chart_1_label" {
          type = "label"
          text = "Channel Statistics (Mean / Min / Max)"
          size = "sm"
        }

        component "chart_1" {
          type     = "area_chart"
          x_field  = "channel"
          y_fields = ["mean", "min", "max"]
          height   = 250

          binding {
            source = "data.stats.summary"
            target = "data"
          }
        }
      }
    }

    // Chart: Standard Deviation across all channels
    component "chart_group_2" {
      type    = "panel"
      padding = 16
      border  = 2
      border_color = "theme.border"
      shadow = "md"

      component "chart_group_2_content" {
        type      = "stack"
        direction = "vertical"
        spacing   = 8

        component "chart_2_label" {
          type = "label"
          text = "Channel Variability (StdDev)"
          size = "sm"
        }

        component "chart_2" {
          type     = "area_chart"
          x_field  = "channel"
          y_fields = ["stddev"]
          height   = 250

          binding {
            source = "data.stats.summary"
            target = "data"
          }
        }
      }
    }

    // ── Statistics Section ───────────────────────────────────────────
    component "stats_heading" {
      type = "label"
      text = "Channel Statistics (60s Window)"
      size = "lg"
    }

    // Quick stats for channel_0
    component "quick_stats" {
      type    = "panel"
      padding = 16
      border  = 2
      border_color = "theme.border"
      shadow = "md"

      component "quick_stats_content" {
        type      = "stack"
        direction = "vertical"
        spacing   = 8

        component "quick_stats_title" {
          type = "label"
          text = "Channel 0 Detail"
          size = "sm"
        }

        component "stats_row" {
          type      = "stack"
          direction = "horizontal"
          spacing   = 16

          component "ch0_mean" {
            type      = "label"
            text      = "Mean: --"
            bind_text = "data.stats.channel_0.mean"
          }

          component "ch0_min" {
            type      = "label"
            text      = "Min: --"
            bind_text = "data.stats.channel_0.min"
          }

          component "ch0_max" {
            type      = "label"
            text      = "Max: --"
            bind_text = "data.stats.channel_0.max"
          }

          component "ch0_stddev" {
            type      = "label"
            text      = "StdDev: --"
            bind_text = "data.stats.channel_0.stddev"
          }

          component "ch0_count" {
            type      = "label"
            text      = "Samples: --"
            bind_text = "data.stats.channel_0.count"
          }
        }
      }
    }

    // ── Stats Table ─────────────────────────────────────────────────
    component "table_section" {
      type    = "panel"
      padding = 16
      border  = 2
      border_color = "theme.border"
      shadow = "md"

      component "table_content" {
        type      = "stack"
        direction = "vertical"
        spacing   = 8

        component "table_heading" {
          type = "label"
          text = "All Channels Summary"
          size = "sm"
        }

        component "stats_table" {
          type   = "table"
          stripe = true
          height = 400
          columns = [
            { key = "channel", label = "Channel", width = 140 },
            { key = "mean",    label = "Mean",    width = 100 },
            { key = "min",     label = "Min",     width = 100 },
            { key = "max",     label = "Max",     width = 100 },
            { key = "stddev",  label = "StdDev",  width = 100 },
            { key = "count",   label = "Samples", width = 100 }
          ]

          binding {
            source = "data.stats.summary"
            target = "data"
          }
        }
      }
    }
  }
}
