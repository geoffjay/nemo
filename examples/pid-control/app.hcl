// PID Control Demo
// Demonstrates plugin template registration, PID control loop, and live tuning

app {
  window {
    title = "PID Control Demo"

    header_bar {
      github_url = "https://github.com/geoffjay/nemo/tree/main/examples/pid-control"
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

// Timer source drives the simulated plant
data {
  source "plant" {
    type     = "timer"
    interval = 1
  }
}

// Layout: two panels side by side
layout {
  type = "stack"

  component "root_panel" {
    type      = "stack"
    direction = "vertical"
    margin    = 16
    spacing   = 8

    component "title" {
      type = "label"
      text = "PID Control Demo"
    }

    component "main_content" {
      type      = "stack"
      direction = "horizontal"
      spacing   = 16

      // Left: PID control panel (uses plugin template)
      component "pid_panel" {
        template = "pid_control"
      }

      // Right: Output display panel
      component "output_panel" {
        type         = "panel"
        padding      = 16
        border       = 2
        border_color = "theme.border"
        shadow       = "md"

        component "output_title" {
          type = "label"
          text = "Process Status"
        }

        component "output_stack" {
          type      = "stack"
          direction = "vertical"
          spacing   = 8

          component "pv_display" {
            type     = "label"
            text     = "PV: 0.0"
            bind_text = "data.pid.pv_display"
          }

          component "error_display" {
            type     = "label"
            text     = "Error: 0.0"
            bind_text = "data.pid.error_display"
          }

          component "output_display" {
            type     = "label"
            text     = "Output: 0.0"
            bind_text = "data.pid.output_display"
          }

          component "setpoint_display" {
            type     = "label"
            text     = "Setpoint: 0.0"
            bind_text = "data.pid.sp_display"
          }
        }
      }
    }
  }
}
