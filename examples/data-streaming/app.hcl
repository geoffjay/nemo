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

scripts {
  path = "./scripts"
}

// Data sources configuration
data {
  source "metrics" {
    type     = "nats"
    url      = "nats://127.0.0.1:4222"
    subjects = ["metrics.>"]
  }
}

// Layout: horizontal stack with sidenav + content area
layout {
  type = "stack"

  component "root_row" {
    type      = "stack"
    direction = "horizontal"
    spacing   = 0

    // ── Side Navigation ──────────────────────────────────────────────
    component "sidenav" {
      type         = "sidenav_bar"
      collapsed    = true
      border_left  = false

      component "nav_charts" {
        type     = "sidenav_bar_item"
        icon     = "chart-pie"
        label    = "Charts"
        on_click = "on_nav"
      }

      component "nav_timeseries" {
        type     = "sidenav_bar_item"
        icon     = "frame"
        label    = "Time Series"
        on_click = "on_nav"
      }

      component "nav_tables" {
        type     = "sidenav_bar_item"
        icon     = "layout-dashboard"
        label    = "Data Tables"
        on_click = "on_nav"
      }
    }

    // ── Content Area ─────────────────────────────────────────────────
    component "content" {
      type      = "stack"
      direction = "vertical"
      flex      = 1

      // ── Charts Page ────────────────────────────────────────────────
      component "page_charts" {
        type    = "panel"
        visible = true

        component "charts_stack" {
          type      = "stack"
          direction = "vertical"
          spacing   = 16
          padding   = 16
          scroll    = true

          component "charts_heading" {
            type = "label"
            text = "Live Metrics"
            size = "lg"
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

          // Stacked column: Mean / Min / Max per channel
          component "chart_1_label" {
            type = "label"
            text = "Channel Statistics (Mean / Min / Max)"
            size = "sm"
          }

          component "chart_1" {
            type     = "stacked_column_chart"
            x_field  = "channel"
            y_fields = ["mean", "min", "max"]
            height   = 300

            binding {
              source = "data.stats.summary"
              target = "data"
            }
          }

          // Column chart: StdDev per channel
          component "chart_2_label" {
            type = "label"
            text = "Channel Variability (StdDev)"
            size = "sm"
          }

          component "chart_2" {
            type    = "column_chart"
            x_field = "channel"
            y_field = "stddev"
            height  = 300

            binding {
              source = "data.stats.summary"
              target = "data"
            }
          }
        }
      }

      // ── Time Series Page ──────────────────────────────────────────
      component "page_timeseries" {
        type    = "panel"
        visible = false

        component "timeseries_stack" {
          type      = "stack"
          direction = "vertical"
          spacing   = 16
          padding   = 16
          scroll    = true

          component "timeseries_heading" {
            type = "label"
            text = "Live Time Series (30s window)"
            size = "lg"
          }

          // ── Row 1: Temperature + Humidity ──────────────────────
          component "ts_row_1" {
            type      = "stack"
            direction = "horizontal"
            spacing   = 16

            component "ts_temperature_panel" {
              type    = "panel"
              flex    = 1
              padding = 12
              border  = 1
              border_color = "theme.border"

              component "ts_temperature_stack" {
                type      = "stack"
                direction = "vertical"
                spacing   = 4

                component "ts_temperature_label" {
                  type = "label"
                  text = "Temperature (\u00B0C)"
                  size = "sm"
                }

                component "ts_temperature" {
                  type        = "realtime_chart"
                  x_field     = "time"
                  y_fields    = ["channel_0", "channel_5", "channel_10", "channel_15"]
                  linear      = true
                  tick_margin = 5
                  height      = 250

                  binding {
                    source = "data.stats.timeseries.temperature"
                    target = "data"
                  }
                }
              }
            }

            component "ts_humidity_panel" {
              type    = "panel"
              flex    = 1
              padding = 12
              border  = 1
              border_color = "theme.border"

              component "ts_humidity_stack" {
                type      = "stack"
                direction = "vertical"
                spacing   = 4

                component "ts_humidity_label" {
                  type = "label"
                  text = "Humidity (%)"
                  size = "sm"
                }

                component "ts_humidity" {
                  type        = "realtime_chart"
                  x_field     = "time"
                  y_fields    = ["channel_1", "channel_6", "channel_11", "channel_16"]
                  linear      = true
                  tick_margin = 5
                  height      = 250

                  binding {
                    source = "data.stats.timeseries.humidity"
                    target = "data"
                  }
                }
              }
            }
          }

          // ── Row 2: Pressure + Speed ────────────────────────────
          component "ts_row_2" {
            type      = "stack"
            direction = "horizontal"
            spacing   = 16

            component "ts_pressure_panel" {
              type    = "panel"
              flex    = 1
              padding = 12
              border  = 1
              border_color = "theme.border"

              component "ts_pressure_stack" {
                type      = "stack"
                direction = "vertical"
                spacing   = 4

                component "ts_pressure_label" {
                  type = "label"
                  text = "Pressure (PSI)"
                  size = "sm"
                }

                component "ts_pressure" {
                  type        = "realtime_chart"
                  x_field     = "time"
                  y_fields    = ["channel_2", "channel_7", "channel_12", "channel_17"]
                  linear      = true
                  tick_margin = 5
                  height      = 250

                  binding {
                    source = "data.stats.timeseries.pressure"
                    target = "data"
                  }
                }
              }
            }

            component "ts_speed_panel" {
              type    = "panel"
              flex    = 1
              padding = 12
              border  = 1
              border_color = "theme.border"

              component "ts_speed_stack" {
                type      = "stack"
                direction = "vertical"
                spacing   = 4

                component "ts_speed_label" {
                  type = "label"
                  text = "Speed (RPM)"
                  size = "sm"
                }

                component "ts_speed" {
                  type        = "realtime_chart"
                  x_field     = "time"
                  y_fields    = ["channel_3", "channel_8", "channel_13", "channel_18"]
                  linear      = true
                  tick_margin = 5
                  height      = 250

                  binding {
                    source = "data.stats.timeseries.speed"
                    target = "data"
                  }
                }
              }
            }
          }
        }
      }

      // ── Data Tables Page ───────────────────────────────────────────
      component "page_tables" {
        type    = "panel"
        visible = false

        component "tables_stack" {
          type      = "stack"
          direction = "vertical"
          spacing   = 16
          padding   = 16
          scroll    = true

          component "tables_heading" {
            type = "label"
            text = "Data Tables"
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

          // Summary table
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
                height = 500
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
    }
  }
}
