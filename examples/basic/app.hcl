# Example Nemo Application Configuration

app {
  title = "My Nemo App"

  window {
    title = "Nemo Example"
    width = 1024
    height = 768
  }

  theme {
    background = "#1e1e2e"
    foreground = "#cdd6f4"
  }
}

# Scripts configuration
scripts {
  path = "./scripts"
}

# Layout configuration
layout {
  type = "stack"

  component "header" {
    type = "label"
    text = "Welcome to Nemo"
  }

  component "content" {
    type = "panel"

    component "button" {
      type = "button"
      label = "Click Me"
      on_click = "on_button_click"
    }
  }
}

# Data sources
data {
  source "api" {
    type = "http"
    url = "https://api.example.com"
    refresh = 30
  }
}
