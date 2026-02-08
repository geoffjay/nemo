# Calculator Application Configuration

app {
  title = "Nemo Calculator"

  window {
    title = "Calculator"
    width = 320
    height = 480
  }

  theme {
    background = "#2b2b2b"
    foreground = "#ffffff"
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
    padding = 15

    component "result" {
      type = "label"
      id = "display_result"
      text = "0"
    }
  }

  # Button rows for calculator
  component "buttons" {
    type = "stack"
    direction = "vertical"
    spacing = 6

    # Row 1: Clear, +/-, %, /
    component "row1" {
      type = "stack"
      direction = "horizontal"
      spacing = 6

      component "btn_clear" {
        type = "button"
        label = "C"
        width = 64
        height = 64
        on_click = "on_clear"
      }

      component "btn_negate" {
        type = "button"
        label = "+/-"
        width = 64
        height = 64
        on_click = "on_negate"
      }

      component "btn_percent" {
        type = "button"
        label = "%"
        width = 64
        height = 64
        on_click = "on_percent"
      }

      component "btn_divide" {
        type = "button"
        label = "/"
        width = 64
        height = 64
        on_click = "on_operator"
      }
    }

    # Row 2: 7, 8, 9, *
    component "row2" {
      type = "stack"
      direction = "horizontal"
      spacing = 6

      component "btn_7" {
        type = "button"
        label = "7"
        width = 64
        height = 64
        on_click = "on_digit"
      }

      component "btn_8" {
        type = "button"
        label = "8"
        width = 64
        height = 64
        on_click = "on_digit"
      }

      component "btn_9" {
        type = "button"
        label = "9"
        width = 64
        height = 64
        on_click = "on_digit"
      }

      component "btn_multiply" {
        type = "button"
        label = "*"
        width = 64
        height = 64
        on_click = "on_operator"
      }
    }

    # Row 3: 4, 5, 6, -
    component "row3" {
      type = "stack"
      direction = "horizontal"
      spacing = 6

      component "btn_4" {
        type = "button"
        label = "4"
        width = 64
        height = 64
        on_click = "on_digit"
      }

      component "btn_5" {
        type = "button"
        label = "5"
        width = 64
        height = 64
        on_click = "on_digit"
      }

      component "btn_6" {
        type = "button"
        label = "6"
        width = 64
        height = 64
        on_click = "on_digit"
      }

      component "btn_subtract" {
        type = "button"
        label = "-"
        width = 64
        height = 64
        on_click = "on_operator"
      }
    }

    # Row 4: 1, 2, 3, +
    component "row4" {
      type = "stack"
      direction = "horizontal"
      spacing = 6

      component "btn_1" {
        type = "button"
        label = "1"
        width = 64
        height = 64
        on_click = "on_digit"
      }

      component "btn_2" {
        type = "button"
        label = "2"
        width = 64
        height = 64
        on_click = "on_digit"
      }

      component "btn_3" {
        type = "button"
        label = "3"
        width = 64
        height = 64
        on_click = "on_digit"
      }

      component "btn_add" {
        type = "button"
        label = "+"
        width = 64
        height = 64
        on_click = "on_operator"
      }
    }

    # Row 5: 0, ., =
    component "row5" {
      type = "stack"
      direction = "horizontal"
      spacing = 6

      component "btn_0" {
        type = "button"
        label = "0"
        width = 64
        height = 64
        on_click = "on_digit"
      }

      component "btn_decimal" {
        type = "button"
        label = "."
        width = 64
        height = 64
        on_click = "on_decimal"
      }

      component "btn_equals" {
        type = "button"
        label = "="
        width = 64
        height = 64
        on_click = "on_equals"
      }
    }
  }
}
