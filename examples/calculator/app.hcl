# Calculator Application Configuration

app {
  title = "Nemo Calculator"

  window {
    title = "Calculator"
    width = 320
    height = 480
    min_width = 280
    min_height = 400

    header_bar {
      github_url = "https://github.com/geoffjay/nemo"
      theme_toggle = true
    }
  }

  theme {
    name = "nord"
    mode = "dark"
  }
}

# Scripts configuration
scripts {
  path = "./scripts"
}

# Layout configuration
layout {
  type = "stack"

  # Display showing current input and result
  component "display" {
    type = "panel"
    margin = 8

    component "result" {
      type = "label"
      id = "display_result"
      text = "0"
      margin = 16
    }
  }

  # Button rows for calculator
  component "buttons" {
    type = "stack"
    direction = "vertical"
    spacing = 6
    padding = 8
    flex = 1

    # Row 1: Clear, +/-, %, /
    component "row1" {
      type = "stack"
      direction = "horizontal"
      spacing = 6
      flex = 1

      component "btn_clear" {
        type = "button"
        label = "C"
        flex = 1
        min_height = 64
        on_click = "on_clear"
      }

      component "btn_negate" {
        type = "button"
        label = "+/-"
        flex = 1
        min_height = 64
        on_click = "on_negate"
      }

      component "btn_percent" {
        type = "button"
        label = "%"
        flex = 1
        min_height = 64
        on_click = "on_percent"
      }

      component "btn_divide" {
        type = "button"
        label = "/"
        flex = 1
        min_height = 64
        on_click = "on_operator"
      }
    }

    # Row 2: 7, 8, 9, *
    component "row2" {
      type = "stack"
      direction = "horizontal"
      spacing = 6
      flex = 1

      component "btn_7" {
        type = "button"
        label = "7"
        flex = 1
        min_height = 64
        on_click = "on_digit"
      }

      component "btn_8" {
        type = "button"
        label = "8"
        flex = 1
        min_height = 64
        on_click = "on_digit"
      }

      component "btn_9" {
        type = "button"
        label = "9"
        flex = 1
        min_height = 64
        on_click = "on_digit"
      }

      component "btn_multiply" {
        type = "button"
        label = "*"
        flex = 1
        min_height = 64
        on_click = "on_operator"
      }
    }

    # Row 3: 4, 5, 6, -
    component "row3" {
      type = "stack"
      direction = "horizontal"
      spacing = 6
      flex = 1

      component "btn_4" {
        type = "button"
        label = "4"
        flex = 1
        min_height = 64
        on_click = "on_digit"
      }

      component "btn_5" {
        type = "button"
        label = "5"
        flex = 1
        min_height = 64
        on_click = "on_digit"
      }

      component "btn_6" {
        type = "button"
        label = "6"
        flex = 1
        min_height = 64
        on_click = "on_digit"
      }

      component "btn_subtract" {
        type = "button"
        label = "-"
        flex = 1
        min_height = 64
        on_click = "on_operator"
      }
    }

    # Row 4: 1, 2, 3, +
    component "row4" {
      type = "stack"
      direction = "horizontal"
      spacing = 6
      flex = 1

      component "btn_1" {
        type = "button"
        label = "1"
        flex = 1
        min_height = 64
        on_click = "on_digit"
      }

      component "btn_2" {
        type = "button"
        label = "2"
        flex = 1
        min_height = 64
        on_click = "on_digit"
      }

      component "btn_3" {
        type = "button"
        label = "3"
        flex = 1
        min_height = 64
        on_click = "on_digit"
      }

      component "btn_add" {
        type = "button"
        label = "+"
        flex = 1
        min_height = 64
        on_click = "on_operator"
      }
    }

    # Row 5: 0, ., =
    component "row5" {
      type = "stack"
      direction = "horizontal"
      spacing = 6
      flex = 1

      component "btn_0" {
        type = "button"
        label = "0"
        flex = 1
        min_height = 64
        on_click = "on_digit"
      }

      component "btn_decimal" {
        type = "button"
        label = "."
        flex = 1
        min_height = 64
        on_click = "on_decimal"
      }

      component "btn_equals" {
        type = "button"
        label = "="
        flex = 1
        min_height = 64
        on_click = "on_equals"
      }
    }
  }
}
