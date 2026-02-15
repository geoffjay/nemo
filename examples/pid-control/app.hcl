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
    name = "kanagawa"
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
      size = "xl"
    }

    component "main_content" {
      type      = "stack"
      direction = "horizontal"
      spacing   = 16

      // Left: Motor 1 PID control panel
      component "motor1_pid" {
        template = "pid_control"
        vars {
          ns = "pid.motor1"
        }
      }

      // Center: Motor 2 PID control panel
      component "motor2_pid" {
        template = "pid_control"
        vars {
          ns = "pid.motor2"
        }
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

          component "motor1_output" {
            type      = "label"
            text      = "Motor 1: 0.0"
            bind_text = "data.pid.motor1.output_display"
          }

          component "motor2_output" {
            type      = "label"
            text      = "Motor 2: 0.0"
            bind_text = "data.pid.motor2.output_display"
          }
        }
      }
    }
  }
}
