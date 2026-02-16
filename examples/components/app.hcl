# Component Gallery
# Interactive showcase of all Nemo components with sidebar navigation.

app {
  title = "Nemo Component Gallery"

  window {
    title = "Component Gallery"
    width = 1200
    height = 800

    header_bar {
      github_url = "https://github.com/geoffjay/nemo/tree/main/components/basic"
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

# ── Templates ─────────────────────────────────────────────────
templates {
  template "nav_item" {
    type         = "button"
    variant      = "ghost"
    size         = "sm"
    text_color   = "theme.muted_foreground"
    full_width   = true
    align        = "left"
    padding_left = 2
    on_click     = "on_nav"
  }

  template "content_page" {
    type    = "panel"
    visible = false

    component "inner" {
      type      = "stack"
      direction = "vertical"
      spacing   = 12
      padding   = 32
      slot      = true
    }
  }
}

layout {
  type = "stack"

  # Root horizontal stack: sidebar + content
  component "root_row" {
    type = "stack"
    direction = "horizontal"
    spacing = 0

    # ── Sidebar ──────────────────────────────────────────────
    component "sidebar" {
      type = "panel"

      component "sidebar_inner" {
        type = "stack"
        direction = "vertical"
        spacing = 4
        padding = 16
        scroll = true
        border_right = 1
        border_color = "theme.border"

        component "sidebar_title" {
          type = "label"
          text = "Components"
          size = "md"
        }

        # ── Reference ─────────────────────────────────────────
        component "cat_reference" {
          type = "label"
          text = "Reference"
          size = "sm"
        }

        component "nav_styling" {
          template = "nav_item"
          label    = "Styling"
        }

        # ── Basic ──────────────────────────────────────────────
        component "cat_basic" {
          type = "label"
          text = "Basic"
          size = "sm"
        }

        component "nav_button" {
          template = "nav_item"
          label    = "Button"
        }

        component "nav_label" {
          template = "nav_item"
          label    = "Label"
        }

        component "nav_icon" {
          template = "nav_item"
          label    = "Icon"
        }

        component "nav_text" {
          template = "nav_item"
          label    = "Text"
        }

        component "nav_image" {
          template = "nav_item"
          label    = "Image"
        }

        component "nav_checkbox" {
          template = "nav_item"
          label    = "Checkbox"
        }

        component "nav_progress" {
          template = "nav_item"
          label    = "Progress"
        }

        component "nav_accordion" {
          template = "nav_item"
          label    = "Accordion"
        }

        component "nav_alert" {
          template = "nav_item"
          label    = "Alert"
        }

        component "nav_avatar" {
          template = "nav_item"
          label    = "Avatar"
        }

        component "nav_badge" {
          template = "nav_item"
          label    = "Badge"
        }

        component "nav_collapsible" {
          template = "nav_item"
          label    = "Collapsible"
        }

        component "nav_dropdown_button" {
          template = "nav_item"
          label    = "Dropdown Button"
        }

        component "nav_spinner" {
          template = "nav_item"
          label    = "Spinner"
        }

        component "nav_tag" {
          template = "nav_item"
          label    = "Tag"
        }

        component "nav_tooltip" {
          template = "nav_item"
          label    = "Tooltip"
        }

        # ── Form ───────────────────────────────────────────────
        component "cat_form" {
          type = "label"
          text = "Form"
          size = "sm"
        }

        component "nav_input" {
          template = "nav_item"
          label    = "Input"
        }

        component "nav_select" {
          template = "nav_item"
          label    = "Select"
        }

        component "nav_radio" {
          template = "nav_item"
          label    = "Radio"
        }

        component "nav_slider" {
          template = "nav_item"
          label    = "Slider"
        }

        component "nav_switch" {
          template = "nav_item"
          label    = "Switch"
        }

        component "nav_toggle" {
          template = "nav_item"
          label    = "Toggle"
        }

        # ── Layout ─────────────────────────────────────────────
        component "cat_layout" {
          type = "label"
          text = "Layout"
          size = "sm"
        }

        component "nav_panel" {
          template = "nav_item"
          label    = "Panel & Stack"
        }

        component "nav_list" {
          template = "nav_item"
          label    = "List"
        }

        component "nav_notification" {
          template = "nav_item"
          label    = "Notification"
        }

        component "nav_modal" {
          template = "nav_item"
          label    = "Modal"
        }

        # ── Advanced ───────────────────────────────────────────
        component "cat_advanced" {
          type = "label"
          text = "Advanced"
          size = "sm"
        }

        component "nav_table" {
          template = "nav_item"
          label    = "Table"
        }

        component "nav_tree" {
          template = "nav_item"
          label    = "Tree"
        }

        component "nav_charts" {
          template = "nav_item"
          label    = "Charts"
        }

        # ── Navigation ────────────────────────────────────────────
        component "cat_navigation" {
          type = "label"
          text = "Navigation"
          size = "sm"
        }

        component "nav_sidenavbar" {
          template = "nav_item"
          label    = "Sidenav Bar"
        }
      }
    }

    # ── Content area ─────────────────────────────────────────
    component "content" {
      type = "stack"
      direction = "vertical"
      spacing = 0
      padding = 16
      scroll = true

      # ── Button page (visible by default) ───────────────────
      component "page_button" {
        template = "content_page"
        visible  = true

        component "btn_title" {
          type = "label"
          text = "Button"
          size = "lg"
        }

        component "btn_desc" {
          type = "text"
          content = "Buttons trigger actions. Available variants: primary, secondary, danger, ghost, warning, success, info."
        }

        # Variants row
        component "btn_section_variants" {
          type = "label"
          text = "Variants"
          size = "sm"
        }

        component "btn_variants_row" {
          type = "stack"
          direction = "horizontal"
          spacing = 8

          component "btn_primary" {
            type = "button"
            label = "Primary"
            variant = "primary"
          }

          component "btn_secondary" {
            type = "button"
            label = "Secondary"
            variant = "secondary"
          }

          component "btn_danger" {
            type = "button"
            label = "Danger"
            variant = "danger"
          }

          component "btn_ghost" {
            type = "button"
            label = "Ghost"
            variant = "ghost"
          }

          component "btn_warning" {
            type = "button"
            label = "Warning"
            variant = "warning"
          }

          component "btn_success" {
            type = "button"
            label = "Success"
            variant = "success"
          }

          component "btn_info" {
            type = "button"
            label = "Info"
            variant = "info"
          }
        }

        # Disabled state
        component "btn_section_disabled" {
          type = "label"
          text = "Disabled"
          size = "sm"
        }

        component "btn_disabled_row" {
          type = "stack"
          direction = "horizontal"
          spacing = 8

          component "btn_disabled_1" {
            type = "button"
            label = "Disabled Primary"
            variant = "primary"
            disabled = true
          }

          component "btn_disabled_2" {
            type = "button"
            label = "Disabled Secondary"
            disabled = true
          }
        }
      }

      # ── Label page ─────────────────────────────────────────
      component "page_label" {
        template = "content_page"

        component "lbl_title" {
          type = "label"
          text = "Label"
          size = "lg"
        }

        component "lbl_desc" {
          type = "text"
          content = "Labels display text at various sizes: xs, sm, md (default), lg, xl."
        }

        component "lbl_section_sizes" {
          type = "label"
          text = "Sizes"
          size = "sm"
        }

        component "lbl_xs" {
          type = "label"
          text = "Extra small label (xs)"
          size = "xs"
        }

        component "lbl_sm" {
          type = "label"
          text = "Small label (sm)"
          size = "sm"
        }

        component "lbl_md" {
          type = "label"
          text = "Medium label (md) — default"
          size = "md"
        }

        component "lbl_lg" {
          type = "label"
          text = "Large label (lg)"
          size = "lg"
        }

        component "lbl_xl" {
          type = "label"
          text = "Extra large label (xl)"
          size = "xl"
        }
      }

      # ── Icon page ──────────────────────────────────────────
      component "page_icon" {
        template = "content_page"

        component "icon_title" {
          type = "label"
          text = "Icon"
          size = "lg"
        }

        component "icon_desc" {
          type = "text"
          content = "Icons render named vector icons from the gpui-component icon set."
        }

        component "icon_section_grid" {
          type = "label"
          text = "Available Icons"
          size = "sm"
        }

        component "icon_row1" {
          type = "stack"
          direction = "horizontal"
          spacing = 16

          component "icon_search" {
            type = "icon"
            name = "search"
          }

          component "icon_settings" {
            type = "icon"
            name = "settings"
          }

          component "icon_bell" {
            type = "icon"
            name = "bell"
          }

          component "icon_check" {
            type = "icon"
            name = "check"
          }

          component "icon_close" {
            type = "icon"
            name = "close"
          }

          component "icon_file" {
            type = "icon"
            name = "file"
          }

          component "icon_folder" {
            type = "icon"
            name = "folder"
          }
        }

        component "icon_row2" {
          type = "stack"
          direction = "horizontal"
          spacing = 16

          component "icon_github" {
            type = "icon"
            name = "github"
          }

          component "icon_globe" {
            type = "icon"
            name = "globe"
          }

          component "icon_user" {
            type = "icon"
            name = "user"
          }

          component "icon_heart" {
            type = "icon"
            name = "heart"
          }

          component "icon_star" {
            type = "icon"
            name = "star"
          }

          component "icon_plus" {
            type = "icon"
            name = "plus"
          }

          component "icon_minus" {
            type = "icon"
            name = "minus"
          }
        }

        component "icon_row3" {
          type = "stack"
          direction = "horizontal"
          spacing = 16

          component "icon_sun" {
            type = "icon"
            name = "sun"
          }

          component "icon_moon" {
            type = "icon"
            name = "moon"
          }

          component "icon_inbox" {
            type = "icon"
            name = "inbox"
          }

          component "icon_copy" {
            type = "icon"
            name = "copy"
          }

          component "icon_trash" {
            type = "icon"
            name = "trash"
          }

          component "icon_eye" {
            type = "icon"
            name = "eye"
          }

          component "icon_menu" {
            type = "icon"
            name = "menu"
          }
        }
      }

      # ── Checkbox page ──────────────────────────────────────
      component "page_checkbox" {
        template = "content_page"

        component "cb_title" {
          type = "label"
          text = "Checkbox"
          size = "lg"
        }

        component "cb_desc" {
          type = "text"
          content = "Checkboxes toggle boolean state. Support labels and disabled state."
        }

        component "cb_section_states" {
          type = "label"
          text = "States"
          size = "sm"
        }

        component "cb_unchecked" {
          type = "checkbox"
          label = "Unchecked"
          checked = false
        }

        component "cb_checked" {
          type = "checkbox"
          label = "Checked"
          checked = true
        }

        component "cb_disabled" {
          type = "checkbox"
          label = "Disabled checkbox"
          checked = false
          disabled = true
        }

        component "cb_disabled_checked" {
          type = "checkbox"
          label = "Disabled checked"
          checked = true
          disabled = true
        }
      }

      # ── Input page ─────────────────────────────────────────
      component "page_input" {
        template = "content_page"

        component "inp_title" {
          type = "label"
          text = "Input"
          size = "lg"
        }

        component "inp_desc" {
          type = "text"
          content = "Text inputs for user data entry. Support placeholder text and disabled state."
        }

        component "inp_section_basic" {
          type = "label"
          text = "Basic Input"
          size = "sm"
        }

        component "inp_basic" {
          type = "input"
          placeholder = "Type something..."
        }

        component "inp_section_disabled" {
          type = "label"
          text = "Disabled Input"
          size = "sm"
        }

        component "inp_disabled" {
          type = "input"
          placeholder = "Cannot edit"
          disabled = true
        }
      }

      # ── Select page ────────────────────────────────────────
      component "page_select" {
        template = "content_page"

        component "sel_title" {
          type = "label"
          text = "Select"
          size = "lg"
        }

        component "sel_desc" {
          type = "text"
          content = "Select presents a list of options for the user to choose from."
        }

        component "sel_section_basic" {
          type = "label"
          text = "Basic Select"
          size = "sm"
        }

        component "sel_basic" {
          type = "select"
          options = ["Apple", "Banana", "Cherry", "Date", "Elderberry"]
          value = "Cherry"
        }
      }

      # ── Progress page ──────────────────────────────────────
      component "page_progress" {
        template = "content_page"

        component "prog_title" {
          type = "label"
          text = "Progress"
          size = "lg"
        }

        component "prog_desc" {
          type = "text"
          content = "Progress bars show completion percentage. Set value (0-100) and optional max."
        }

        component "prog_section_values" {
          type = "label"
          text = "Various Values"
          size = "sm"
        }

        component "prog_label_0" {
          type = "label"
          text = "0%"
          size = "xs"
        }

        component "prog_0" {
          type = "progress"
          value = 0
        }

        component "prog_label_25" {
          type = "label"
          text = "25%"
          size = "xs"
        }

        component "prog_25" {
          type = "progress"
          value = 25
        }

        component "prog_label_50" {
          type = "label"
          text = "50%"
          size = "xs"
        }

        component "prog_50" {
          type = "progress"
          value = 50
        }

        component "prog_label_75" {
          type = "label"
          text = "75%"
          size = "xs"
        }

        component "prog_75" {
          type = "progress"
          value = 75
        }

        component "prog_label_100" {
          type = "label"
          text = "100%"
          size = "xs"
        }

        component "prog_100" {
          type = "progress"
          value = 100
        }
      }

      # ── Notification page ──────────────────────────────────
      component "page_notification" {
        template = "content_page"

        component "notif_title" {
          type = "label"
          text = "Notification"
          size = "lg"
        }

        component "notif_desc" {
          type = "text"
          content = "Notifications display status messages. Types: info, success, warning, error."
        }

        component "notif_section_types" {
          type = "label"
          text = "Notification Types"
          size = "sm"
        }

        component "notif_info" {
          type = "notification"
          message = "This is an informational message."
          kind = "info"
        }

        component "notif_success" {
          type = "notification"
          message = "Operation completed successfully!"
          kind = "success"
        }

        component "notif_warning" {
          type = "notification"
          message = "Warning: please review before proceeding."
          kind = "warning"
        }

        component "notif_error" {
          type = "notification"
          message = "Error: something went wrong."
          kind = "error"
        }
      }

      # ── Modal page ─────────────────────────────────────────
      component "page_modal" {
        template = "content_page"

        component "modal_title" {
          type = "label"
          text = "Modal"
          size = "lg"
        }

        component "modal_desc" {
          type = "text"
          content = "Modals display overlay dialogs. Toggle the open property to show or hide."
        }

        component "modal_toggle_btn" {
          type = "button"
          label = "Open Modal"
          variant = "primary"
          on_click = "on_toggle_modal"
        }

        component "demo_modal" {
          type = "modal"
          title = "Example Modal"
          open = false

          component "modal_content_text" {
            type = "text"
            content = "This is the modal content. Click the button again to close."
          }

          component "modal_close_btn" {
            type = "button"
            label = "Close"
            variant = "danger"
            on_click = "on_toggle_modal"
          }
        }
      }

      # ── Text page ──────────────────────────────────────────
      component "page_text" {
        template = "content_page"

        component "text_title" {
          type = "label"
          text = "Text"
          size = "lg"
        }

        component "text_desc" {
          type = "text"
          content = "Text displays plain content blocks. Use for paragraphs or descriptions."
        }

        component "text_section_examples" {
          type = "label"
          text = "Examples"
          size = "sm"
        }

        component "text_example_1" {
          type = "text"
          content = "This is a simple text block. It renders plain content without any special formatting."
        }

        component "text_example_2" {
          type = "text"
          content = "Text components are useful for displaying descriptions, help text, and other informational content within your application layout."
        }
      }

      # ── List page ──────────────────────────────────────────
      component "page_list" {
        template = "content_page"

        component "list_title" {
          type = "label"
          text = "List"
          size = "lg"
        }

        component "list_desc" {
          type = "text"
          content = "Lists render arrays of items as a vertical list with hover highlighting."
        }

        component "list_section_basic" {
          type = "label"
          text = "Basic List"
          size = "sm"
        }

        component "list_basic" {
          type = "list"
          items = ["First item", "Second item", "Third item", "Fourth item", "Fifth item"]
        }
      }

      # ── Panel & Stack page ─────────────────────────────────
      component "page_panel" {
        template = "content_page"

        component "panel_title" {
          type = "label"
          text = "Panel & Stack"
          size = "lg"
        }

        component "panel_desc" {
          type = "text"
          content = "Panels are containers with padding and background. Stacks arrange children vertically or horizontally with configurable spacing."
        }

        component "panel_section_nested" {
          type = "label"
          text = "Nested Panels"
          size = "sm"
        }

        component "panel_outer" {
          type = "panel"

          component "panel_outer_label" {
            type = "label"
            text = "Outer panel"
          }

          component "panel_inner" {
            type = "panel"

            component "panel_inner_label" {
              type = "label"
              text = "Inner nested panel"
            }
          }
        }

        component "panel_section_stacks" {
          type = "label"
          text = "Horizontal Stack"
          size = "sm"
        }

        component "panel_h_stack" {
          type = "stack"
          direction = "horizontal"
          spacing = 8

          component "panel_h_item1" {
            type = "panel"

            component "panel_h_item1_label" {
              type = "label"
              text = "Column 1"
            }
          }

          component "panel_h_item2" {
            type = "panel"

            component "panel_h_item2_label" {
              type = "label"
              text = "Column 2"
            }
          }

          component "panel_h_item3" {
            type = "panel"

            component "panel_h_item3_label" {
              type = "label"
              text = "Column 3"
            }
          }
        }
      }

      # ── Table page ─────────────────────────────────────────
      component "page_table" {
        template = "content_page"

        component "table_title" {
          type = "label"
          text = "Table"
          size = "lg"
        }

        component "table_desc" {
          type = "text"
          content = "Table displays tabular data with columns, sorting, and row selection."
        }

        component "table_demo" {
          type = "table"
          stripe = true
          columns = [
            { key = "name", label = "Name", width = 150 },
            { key = "email", label = "Email", width = 220 },
            { key = "role", label = "Role", width = 120 },
            { key = "status", label = "Status", width = 100 }
          ]
          data = [
            { name = "Alice Johnson", email = "alice@example.com", role = "Admin", status = "Active" },
            { name = "Bob Smith", email = "bob@example.com", role = "Editor", status = "Active" },
            { name = "Carol White", email = "carol@example.com", role = "Viewer", status = "Inactive" },
            { name = "Dave Brown", email = "dave@example.com", role = "Editor", status = "Active" },
            { name = "Eve Davis", email = "eve@example.com", role = "Admin", status = "Active" }
          ]
        }
      }

      # ── Tree page ──────────────────────────────────────────
      component "page_tree" {
        template = "content_page"

        component "tree_title" {
          type = "label"
          text = "Tree"
          size = "lg"
        }

        component "tree_desc" {
          type = "text"
          content = "Tree displays hierarchical data with expand/collapse and keyboard navigation."
        }

        component "tree_demo" {
          type = "tree"
          items = [
            {
              id = "src"
              label = "src"
              expanded = true
              children = [
                {
                  id = "src/main.rs"
                  label = "main.rs"
                },
                {
                  id = "src/lib.rs"
                  label = "lib.rs"
                },
                {
                  id = "src/components"
                  label = "components"
                  expanded = true
                  children = [
                    { id = "src/components/mod.rs", label = "mod.rs" },
                    { id = "src/components/table.rs", label = "table.rs" },
                    { id = "src/components/tree.rs", label = "tree.rs" }
                  ]
                }
              ]
            },
            { id = "Cargo.toml", label = "Cargo.toml" },
            { id = "README.md", label = "README.md" }
          ]
        }
      }

      # ── Image page ─────────────────────────────────────────
      component "page_image" {
        template = "content_page"

        component "image_title" {
          type = "label"
          text = "Image"
          size = "lg"
        }

        component "image_desc" {
          type = "text"
          content = "Image displays an image from a source path. Falls back to alt text when no source is provided."
        }

        component "image_section_fallback" {
          type = "label"
          text = "Alt Text Fallback"
          size = "sm"
        }

        component "image_no_src" {
          type = "image"
          alt = "Placeholder: no image source provided"
        }

        component "image_another" {
          type = "image"
          alt = "Another image placeholder with alt text"
        }
      }

      # ── Charts page ────────────────────────────────────────
      component "page_charts" {
        template = "content_page"

        component "charts_title" {
          type = "label"
          text = "Charts"
          size = "lg"
        }

        component "charts_desc" {
          type = "text"
          content = "Chart components for data visualization. Supports line, bar, area, pie, and candlestick charts."
        }

        # ── Line Chart ──────────────────────────────────────
        component "chart_section_line" {
          type = "label"
          text = "Line Chart"
          size = "sm"
        }

        component "line_chart_demo" {
          type = "line_chart"
          x_field = "month"
          y_field = "revenue"
          dot = true
          height = 300
          data = [
            { month = "Jan", revenue = 186 },
            { month = "Feb", revenue = 305 },
            { month = "Mar", revenue = 237 },
            { month = "Apr", revenue = 73 },
            { month = "May", revenue = 209 },
            { month = "Jun", revenue = 214 }
          ]
        }

        # ── Bar Chart ───────────────────────────────────────
        component "chart_section_bar" {
          type = "label"
          text = "Bar Chart"
          size = "sm"
        }

        component "bar_chart_demo" {
          type = "bar_chart"
          x_field = "month"
          y_field = "visitors"
          show_label = true
          height = 300
          data = [
            { month = "Jan", visitors = 275 },
            { month = "Feb", visitors = 200 },
            { month = "Mar", visitors = 187 },
            { month = "Apr", visitors = 173 },
            { month = "May", visitors = 90 },
            { month = "Jun", visitors = 301 }
          ]
        }

        # ── Area Chart ──────────────────────────────────────
        component "chart_section_area" {
          type = "label"
          text = "Area Chart (Multi-Series)"
          size = "sm"
        }

        component "area_chart_demo" {
          type = "area_chart"
          x_field = "month"
          y_fields = ["desktop", "mobile"]
          height = 300
          data = [
            { month = "Jan", desktop = 186, mobile = 80 },
            { month = "Feb", desktop = 305, mobile = 200 },
            { month = "Mar", desktop = 237, mobile = 120 },
            { month = "Apr", desktop = 73, mobile = 190 },
            { month = "May", desktop = 209, mobile = 130 },
            { month = "Jun", desktop = 214, mobile = 140 }
          ]
        }

        # ── Pie Chart ───────────────────────────────────────
        component "chart_section_pie" {
          type = "label"
          text = "Pie Chart"
          size = "sm"
        }

        component "pie_chart_demo" {
          type = "pie_chart"
          value_field = "amount"
          height = 300
          data = [
            { label = "Chrome", amount = 275 },
            { label = "Safari", amount = 200 },
            { label = "Firefox", amount = 187 },
            { label = "Edge", amount = 173 },
            { label = "Other", amount = 90 }
          ]
        }

        # ── Donut Chart ─────────────────────────────────────
        component "chart_section_donut" {
          type = "label"
          text = "Donut Chart (Pie with inner radius)"
          size = "sm"
        }

        component "donut_chart_demo" {
          type = "pie_chart"
          value_field = "value"
          inner_radius = 60.0
          height = 300
          data = [
            { name = "Rent", value = 1200 },
            { name = "Food", value = 450 },
            { name = "Transport", value = 200 },
            { name = "Utilities", value = 150 }
          ]
        }

        # ── Candlestick Chart ───────────────────────────────
        component "chart_section_candlestick" {
          type = "label"
          text = "Candlestick Chart"
          size = "sm"
        }

        component "candlestick_chart_demo" {
          type = "candlestick_chart"
          x_field = "date"
          open_field = "open"
          high_field = "high"
          low_field = "low"
          close_field = "close"
          height = 300
          data = [
            { date = "Mon", open = 100.0, high = 110.0, low = 95.0, close = 108.0 },
            { date = "Tue", open = 108.0, high = 115.0, low = 102.0, close = 104.0 },
            { date = "Wed", open = 104.0, high = 112.0, low = 100.0, close = 111.0 },
            { date = "Thu", open = 111.0, high = 120.0, low = 108.0, close = 118.0 },
            { date = "Fri", open = 118.0, high = 122.0, low = 110.0, close = 112.0 }
          ]
        }
      }

      # ── Accordion page ────────────────────────────────────
      component "page_accordion" {
        template = "content_page"

        component "accordion_title" {
          type = "label"
          text = "Accordion"
          size = "lg"
        }

        component "accordion_desc" {
          type = "text"
          content = "Accordions show collapsible content sections. Supports multiple open items and bordered styles."
        }

        component "accordion_section_basic" {
          type = "label"
          text = "Basic Accordion"
          size = "sm"
        }

        component "accordion_demo" {
          type = "accordion"
          items = [
            { title = "What is Nemo?", content = "Nemo is a GPUI desktop application framework configured with HCL.", open = true },
            { title = "How do I get started?", content = "Create an app.hcl file and define your layout with components." },
            { title = "Can I use custom themes?", content = "Yes, Nemo supports theme configuration via the app.theme block." }
          ]
        }

        component "accordion_section_multi" {
          type = "label"
          text = "Multiple Open (no border)"
          size = "sm"
        }

        component "accordion_multi" {
          type = "accordion"
          multiple = true
          bordered = false
          items = [
            { title = "Section A", content = "Content for section A.", open = true },
            { title = "Section B", content = "Content for section B.", open = true },
            { title = "Section C", content = "Content for section C." }
          ]
        }
      }

      # ── Alert page ────────────────────────────────────────
      component "page_alert" {
        template = "content_page"

        component "alert_title" {
          type = "label"
          text = "Alert"
          size = "lg"
        }

        component "alert_desc" {
          type = "text"
          content = "Alerts display important messages. Variants: info, success, warning, error."
        }

        component "alert_section_variants" {
          type = "label"
          text = "Variants"
          size = "sm"
        }

        component "alert_info" {
          type = "alert"
          title = "Information"
          message = "This is an informational alert message."
          variant = "info"
        }

        component "alert_success" {
          type = "alert"
          title = "Success"
          message = "The operation completed successfully."
          variant = "success"
        }

        component "alert_warning" {
          type = "alert"
          title = "Warning"
          message = "Please review the changes before continuing."
          variant = "warning"
        }

        component "alert_error" {
          type = "alert"
          title = "Error"
          message = "Something went wrong. Please try again."
          variant = "error"
        }
      }

      # ── Avatar page ───────────────────────────────────────
      component "page_avatar" {
        template = "content_page"

        component "avatar_title" {
          type = "label"
          text = "Avatar"
          size = "lg"
        }

        component "avatar_desc" {
          type = "text"
          content = "Avatars display user initials. Provide a name to generate initials automatically."
        }

        component "avatar_section_examples" {
          type = "label"
          text = "Examples"
          size = "sm"
        }

        component "avatar_row" {
          type = "stack"
          direction = "horizontal"
          spacing = 12

          component "avatar_alice" {
            type = "avatar"
            name = "Alice Johnson"
          }

          component "avatar_bob" {
            type = "avatar"
            name = "Bob Smith"
          }

          component "avatar_carol" {
            type = "avatar"
            name = "Carol White"
          }

          component "avatar_dave" {
            type = "avatar"
            name = "Dave Brown"
          }
        }
      }

      # ── Badge page ────────────────────────────────────────
      component "page_badge" {
        template = "content_page"

        component "badge_title" {
          type = "label"
          text = "Badge"
          size = "lg"
        }

        component "badge_desc" {
          type = "text"
          content = "Badges show notification counts or dot indicators on child elements."
        }

        component "badge_section_count" {
          type = "label"
          text = "Count Badge"
          size = "sm"
        }

        component "badge_count_row" {
          type = "stack"
          direction = "horizontal"
          spacing = 16

          component "badge_count_demo" {
            type = "badge"
            count = 5

            component "badge_count_btn" {
              type = "button"
              label = "Notifications"
              variant = "secondary"
            }
          }

          component "badge_count_large" {
            type = "badge"
            count = 99

            component "badge_count_large_btn" {
              type = "button"
              label = "Messages"
              variant = "secondary"
            }
          }
        }

        component "badge_section_dot" {
          type = "label"
          text = "Dot Badge"
          size = "sm"
        }

        component "badge_dot_demo" {
          type = "badge"
          dot = true

          component "badge_dot_btn" {
            type = "button"
            label = "Updates"
            variant = "secondary"
          }
        }
      }

      # ── Collapsible page ──────────────────────────────────
      component "page_collapsible" {
        template = "content_page"

        component "collapsible_title" {
          type = "label"
          text = "Collapsible"
          size = "lg"
        }

        component "collapsible_desc" {
          type = "text"
          content = "Collapsible sections expand and collapse to reveal child content."
        }

        component "collapsible_section_examples" {
          type = "label"
          text = "Examples"
          size = "sm"
        }

        component "collapsible_open" {
          type = "collapsible"
          title = "Click to collapse"
          open = true

          component "collapsible_open_content" {
            type = "text"
            content = "This content is visible by default because open = true."
          }
        }

        component "collapsible_closed" {
          type = "collapsible"
          title = "Click to expand"
          open = false

          component "collapsible_closed_content" {
            type = "text"
            content = "This content is hidden by default and will show when expanded."
          }
        }
      }

      # ── Dropdown Button page ──────────────────────────────
      component "page_dropdown_button" {
        template = "content_page"

        component "dropdown_title" {
          type = "label"
          text = "Dropdown Button"
          size = "lg"
        }

        component "dropdown_desc" {
          type = "text"
          content = "Dropdown buttons combine a button with a dropdown menu indicator."
        }

        component "dropdown_section_variants" {
          type = "label"
          text = "Variants"
          size = "sm"
        }

        component "dropdown_row" {
          type = "stack"
          direction = "horizontal"
          spacing = 8

          component "dropdown_default" {
            type = "dropdown_button"
            label = "Actions"
            items = ["Copy", "Paste", "Cut"]
          }

          component "dropdown_primary" {
            type = "dropdown_button"
            label = "Save"
            variant = "primary"
            items = ["Save as Draft", "Save and Publish", "Save as Template"]
          }

          component "dropdown_danger" {
            type = "dropdown_button"
            label = "Delete"
            variant = "danger"
            items = ["Move to Trash", "Delete Permanently"]
          }
        }
      }

      # ── Spinner page ──────────────────────────────────────
      component "page_spinner" {
        template = "content_page"

        component "spinner_title" {
          type = "label"
          text = "Spinner"
          size = "lg"
        }

        component "spinner_desc" {
          type = "text"
          content = "Spinners indicate loading state. Available sizes: xs, sm, md (default), lg."
        }

        component "spinner_section_sizes" {
          type = "label"
          text = "Sizes"
          size = "sm"
        }

        component "spinner_row" {
          type = "stack"
          direction = "horizontal"
          spacing = 24

          component "spinner_xs" {
            type = "spinner"
            size = "xs"
          }

          component "spinner_sm" {
            type = "spinner"
            size = "sm"
          }

          component "spinner_md" {
            type = "spinner"
          }

          component "spinner_lg" {
            type = "spinner"
            size = "lg"
          }
        }
      }

      # ── Tag page ──────────────────────────────────────────
      component "page_tag" {
        template = "content_page"

        component "tag_title" {
          type = "label"
          text = "Tag"
          size = "lg"
        }

        component "tag_desc" {
          type = "text"
          content = "Tags are small labels for categorization. Variants: primary, secondary, danger, success, warning, info."
        }

        component "tag_section_filled" {
          type = "label"
          text = "Filled Tags"
          size = "sm"
        }

        component "tag_filled_row" {
          type = "stack"
          direction = "horizontal"
          spacing = 8

          component "tag_primary" {
            type = "tag"
            label = "Primary"
            variant = "primary"
          }

          component "tag_secondary" {
            type = "tag"
            label = "Secondary"
            variant = "secondary"
          }

          component "tag_danger" {
            type = "tag"
            label = "Danger"
            variant = "danger"
          }

          component "tag_success" {
            type = "tag"
            label = "Success"
            variant = "success"
          }

          component "tag_warning" {
            type = "tag"
            label = "Warning"
            variant = "warning"
          }

          component "tag_info" {
            type = "tag"
            label = "Info"
            variant = "info"
          }
        }

        component "tag_section_outline" {
          type = "label"
          text = "Outline Tags"
          size = "sm"
        }

        component "tag_outline_row" {
          type = "stack"
          direction = "horizontal"
          spacing = 8

          component "tag_outline_primary" {
            type = "tag"
            label = "Primary"
            variant = "primary"
            outline = true
          }

          component "tag_outline_danger" {
            type = "tag"
            label = "Danger"
            variant = "danger"
            outline = true
          }

          component "tag_outline_success" {
            type = "tag"
            label = "Success"
            variant = "success"
            outline = true
          }
        }
      }

      # ── Tooltip page ──────────────────────────────────────
      component "page_tooltip" {
        template = "content_page"

        component "tooltip_title" {
          type = "label"
          text = "Tooltip"
          size = "lg"
        }

        component "tooltip_desc" {
          type = "text"
          content = "Tooltips show additional information on hover."
        }

        component "tooltip_section_examples" {
          type = "label"
          text = "Hover the buttons below"
          size = "sm"
        }

        component "tooltip_row" {
          type = "stack"
          direction = "horizontal"
          spacing = 8

          component "tooltip_demo1" {
            type = "tooltip"
            content = "This is a helpful tooltip"

            component "tooltip_btn1" {
              type = "button"
              label = "Hover me"
              variant = "primary"
            }
          }

          component "tooltip_demo2" {
            type = "tooltip"
            content = "Another tooltip with more info"

            component "tooltip_btn2" {
              type = "button"
              label = "More info"
              variant = "secondary"
            }
          }
        }
      }

      # ── Radio page ────────────────────────────────────────
      component "page_radio" {
        template = "content_page"

        component "radio_title" {
          type = "label"
          text = "Radio"
          size = "lg"
        }

        component "radio_desc" {
          type = "text"
          content = "Radio groups allow selecting one option from a set. Support vertical and horizontal layouts."
        }

        component "radio_section_vertical" {
          type = "label"
          text = "Vertical (default)"
          size = "sm"
        }

        component "radio_vertical" {
          type = "radio"
          options = ["Option A", "Option B", "Option C"]
          value = "Option A"
        }

        component "radio_section_horizontal" {
          type = "label"
          text = "Horizontal"
          size = "sm"
        }

        component "radio_horizontal" {
          type = "radio"
          options = ["Small", "Medium", "Large"]
          value = "Medium"
          direction = "horizontal"
        }
      }

      # ── Slider page ───────────────────────────────────────
      component "page_slider" {
        template = "content_page"

        component "slider_title" {
          type = "label"
          text = "Slider"
          size = "lg"
        }

        component "slider_desc" {
          type = "text"
          content = "Sliders allow selecting a numeric value within a range. Supports min, max, step, and default value."
        }

        component "slider_section_basic" {
          type = "label"
          text = "Basic Slider (0-100)"
          size = "sm"
        }

        component "slider_basic" {
          type = "slider"
          min = 0
          max = 100
          step = 1
          value = 50
        }

        component "slider_section_fine" {
          type = "label"
          text = "Fine Step (0-1, step 0.1)"
          size = "sm"
        }

        component "slider_fine" {
          type = "slider"
          min = 0.0
          max = 1.0
          step = 0.1
          value = 0.5
        }
      }

      # ── Switch page ───────────────────────────────────────
      component "page_switch" {
        template = "content_page"

        component "switch_title" {
          type = "label"
          text = "Switch"
          size = "lg"
        }

        component "switch_desc" {
          type = "text"
          content = "Switches toggle boolean state, similar to checkboxes but with a sliding visual style."
        }

        component "switch_section_states" {
          type = "label"
          text = "States"
          size = "sm"
        }

        component "switch_off" {
          type = "switch"
          label = "Off by default"
          checked = false
        }

        component "switch_on" {
          type = "switch"
          label = "On by default"
          checked = true
        }

        component "switch_disabled" {
          type = "switch"
          label = "Disabled switch"
          checked = false
          disabled = true
        }

        component "switch_disabled_on" {
          type = "switch"
          label = "Disabled (on)"
          checked = true
          disabled = true
        }
      }

      # ── Toggle page ───────────────────────────────────────
      component "page_toggle" {
        template = "content_page"

        component "toggle_title" {
          type = "label"
          text = "Toggle"
          size = "lg"
        }

        component "toggle_desc" {
          type = "text"
          content = "Toggle buttons switch between on/off states. Support labels and icons."
        }

        component "toggle_section_basic" {
          type = "label"
          text = "Basic Toggles"
          size = "sm"
        }

        component "toggle_row" {
          type = "stack"
          direction = "horizontal"
          spacing = 8

          component "toggle_off" {
            type = "toggle"
            label = "Off"
            checked = false
          }

          component "toggle_on" {
            type = "toggle"
            label = "On"
            checked = true
          }

          component "toggle_disabled" {
            type = "toggle"
            label = "Disabled"
            checked = false
            disabled = true
          }
        }

        component "toggle_section_icons" {
          type = "label"
          text = "With Icons"
          size = "sm"
        }

        component "toggle_icon_row" {
          type = "stack"
          direction = "horizontal"
          spacing = 8

          component "toggle_star" {
            type = "toggle"
            label = "Star"
            icon = "star"
            checked = false
          }

          component "toggle_heart" {
            type = "toggle"
            label = "Heart"
            icon = "heart"
            checked = true
          }

          component "toggle_bell" {
            type = "toggle"
            label = "Bell"
            icon = "bell"
            checked = false
          }
        }
      }

      # ── Styling page ────────────────────────────────────────
      component "page_styling" {
        template = "content_page"

        component "styling_title" {
          type = "label"
          text = "Styling"
          size = "lg"
        }

        component "styling_desc" {
          type = "text"
          content = "All components support shared styling attributes via apply_layout_styles: padding, margin, border, shadow, and rounded. These wrap any component in a styled container."
        }

        # ── Padding ──────────────────────────────────────────
        component "styling_padding_title" {
          type = "label"
          text = "Padding"
          size = "sm"
        }

        component "styling_padding_tabs" {
          type = "tabs"
          tabs = ["Preview", "HCL"]

          component "styling_padding_preview" {
            type = "stack"
            direction = "vertical"
            spacing = 8

            component "styling_pad_uniform" {
              type = "label"
              text = "padding = 16"
              padding = 16
              border = 1
              border_color = "theme.border"
            }

            component "styling_pad_xy_row" {
              type = "stack"
              direction = "horizontal"
              spacing = 8

              component "styling_pad_x" {
                type = "label"
                text = "padding_x = 24"
                padding_x = 24
                border = 1
                border_color = "theme.border"
              }

              component "styling_pad_y" {
                type = "label"
                text = "padding_y = 24"
                padding_y = 24
                border = 1
                border_color = "theme.border"
              }
            }

            component "styling_pad_dir_row" {
              type = "stack"
              direction = "horizontal"
              spacing = 8

              component "styling_pad_left" {
                type = "label"
                text = "padding_left = 32"
                padding_left = 32
                border = 1
                border_color = "theme.border"
              }

              component "styling_pad_right" {
                type = "label"
                text = "padding_right = 32"
                padding_right = 32
                border = 1
                border_color = "theme.border"
              }

              component "styling_pad_top" {
                type = "label"
                text = "padding_top = 32"
                padding_top = 32
                border = 1
                border_color = "theme.border"
              }

              component "styling_pad_bottom" {
                type = "label"
                text = "padding_bottom = 32"
                padding_bottom = 32
                border = 1
                border_color = "theme.border"
              }
            }
          }

          component "styling_padding_hcl" {
            type = "panel"

            component "styling_padding_code" {
              type = "text"
              content = "# Uniform padding on all sides\npadding = 16\n\n# Axis padding (horizontal / vertical)\npadding_x = 24\npadding_y = 24\n\n# Directional padding\npadding_left   = 32\npadding_right  = 32\npadding_top    = 32\npadding_bottom = 32"
            }
          }
        }

        # ── Margin ───────────────────────────────────────────
        component "styling_margin_title" {
          type = "label"
          text = "Margin"
          size = "sm"
        }

        component "styling_margin_tabs" {
          type = "tabs"
          tabs = ["Preview", "HCL"]

          component "styling_margin_preview" {
            type = "stack"
            direction = "vertical"
            spacing = 12

            component "styling_margin_uniform_row" {
              type = "stack"
              direction = "horizontal"
              spacing = 0
              border = 1
              border_color = "theme.border"

              component "styling_margin_box1" {
                type = "label"
                text = "margin = 4"
                margin = 4
                border = 1
                border_color = "theme.primary"
                padding = 8
              }

              component "styling_margin_box2" {
                type = "label"
                text = "margin = 12"
                margin = 12
                border = 1
                border_color = "theme.primary"
                padding = 8
              }

              component "styling_margin_box3" {
                type = "label"
                text = "margin = 24"
                margin = 24
                border = 1
                border_color = "theme.primary"
                padding = 8
              }
            }

            component "styling_margin_axis_row" {
              type = "stack"
              direction = "horizontal"
              spacing = 0
              border = 1
              border_color = "theme.border"

              component "styling_margin_x" {
                type = "label"
                text = "margin_x = 20"
                margin_x = 20
                border = 1
                border_color = "theme.primary"
                padding = 8
              }

              component "styling_margin_y" {
                type = "label"
                text = "margin_y = 20"
                margin_y = 20
                border = 1
                border_color = "theme.primary"
                padding = 8
              }
            }

            component "styling_margin_dir_row" {
              type = "stack"
              direction = "horizontal"
              spacing = 0
              border = 1
              border_color = "theme.border"

              component "styling_margin_left" {
                type = "label"
                text = "margin_left = 24"
                margin_left = 24
                border = 1
                border_color = "theme.primary"
                padding = 8
              }

              component "styling_margin_right" {
                type = "label"
                text = "margin_right = 24"
                margin_right = 24
                border = 1
                border_color = "theme.primary"
                padding = 8
              }

              component "styling_margin_top" {
                type = "label"
                text = "margin_top = 16"
                margin_top = 16
                border = 1
                border_color = "theme.primary"
                padding = 8
              }

              component "styling_margin_bottom" {
                type = "label"
                text = "margin_bottom = 16"
                margin_bottom = 16
                border = 1
                border_color = "theme.primary"
                padding = 8
              }
            }
          }

          component "styling_margin_hcl" {
            type = "panel"

            component "styling_margin_code" {
              type = "text"
              content = "# Uniform margin on all sides\nmargin = 12\n\n# Axis margin (horizontal / vertical)\nmargin_x = 20\nmargin_y = 20\n\n# Directional margin\nmargin_left   = 24\nmargin_right  = 24\nmargin_top    = 16\nmargin_bottom = 16"
            }
          }
        }

        # ── Border ───────────────────────────────────────────
        component "styling_border_title" {
          type = "label"
          text = "Border"
          size = "sm"
        }

        component "styling_border_tabs" {
          type = "tabs"
          tabs = ["Preview", "HCL"]

          component "styling_border_preview" {
            type = "stack"
            direction = "vertical"
            spacing = 12

            component "styling_border_uniform_row" {
              type = "stack"
              direction = "horizontal"
              spacing = 12

              component "styling_border_1" {
                type = "label"
                text = "border = 1"
                border = 1
                border_color = "theme.border"
                padding = 12
              }

              component "styling_border_2" {
                type = "label"
                text = "border = 2"
                border = 2
                border_color = "theme.border"
                padding = 12
              }

              component "styling_border_primary" {
                type = "label"
                text = "theme.primary"
                border = 2
                border_color = "theme.primary"
                padding = 12
              }

              component "styling_border_danger" {
                type = "label"
                text = "theme.danger"
                border = 2
                border_color = "theme.danger"
                padding = 12
              }
            }

            component "styling_border_axis_row" {
              type = "stack"
              direction = "horizontal"
              spacing = 12

              component "styling_border_x" {
                type = "label"
                text = "border_x = 2"
                border_x = 2
                border_color = "theme.primary"
                padding = 12
              }

              component "styling_border_y" {
                type = "label"
                text = "border_y = 2"
                border_y = 2
                border_color = "theme.primary"
                padding = 12
              }
            }

            component "styling_border_dir_row" {
              type = "stack"
              direction = "horizontal"
              spacing = 12

              component "styling_border_left" {
                type = "label"
                text = "border_left = 3"
                border_left = 3
                border_color = "theme.primary"
                padding = 12
              }

              component "styling_border_right" {
                type = "label"
                text = "border_right = 3"
                border_right = 3
                border_color = "theme.primary"
                padding = 12
              }

              component "styling_border_top" {
                type = "label"
                text = "border_top = 3"
                border_top = 3
                border_color = "theme.primary"
                padding = 12
              }

              component "styling_border_bottom" {
                type = "label"
                text = "border_bottom = 3"
                border_bottom = 3
                border_color = "theme.primary"
                padding = 12
              }
            }
          }

          component "styling_border_hcl" {
            type = "panel"

            component "styling_border_code" {
              type = "text"
              content = "# Uniform border on all sides\nborder = 1\nborder = 2\n\n# Axis border (horizontal / vertical)\nborder_x = 2\nborder_y = 2\n\n# Directional border\nborder_left   = 3\nborder_right  = 3\nborder_top    = 3\nborder_bottom = 3\n\n# Border color (theme references or hex)\nborder_color = \"theme.border\"\nborder_color = \"theme.primary\"\nborder_color = \"#FF6600\""
            }
          }
        }

        # ── Shadow ───────────────────────────────────────────
        component "styling_shadow_title" {
          type = "label"
          text = "Shadow"
          size = "sm"
        }

        component "styling_shadow_tabs" {
          type = "tabs"
          tabs = ["Preview", "HCL"]

          component "styling_shadow_preview" {
            type = "stack"
            direction = "horizontal"
            spacing = 20

            component "styling_shadow_sm" {
              type = "label"
              text = "sm"
              shadow = "sm"
              padding = 16
              rounded = "md"
            }

            component "styling_shadow_md" {
              type = "label"
              text = "md"
              shadow = "md"
              padding = 16
              rounded = "md"
            }

            component "styling_shadow_lg" {
              type = "label"
              text = "lg"
              shadow = "lg"
              padding = 16
              rounded = "md"
            }

            component "styling_shadow_xl" {
              type = "label"
              text = "xl"
              shadow = "xl"
              padding = 16
              rounded = "md"
            }

            component "styling_shadow_2xl" {
              type = "label"
              text = "2xl"
              shadow = "2xl"
              padding = 16
              rounded = "md"
            }
          }

          component "styling_shadow_hcl" {
            type = "panel"

            component "styling_shadow_code" {
              type = "text"
              content = "# Shadow sizes: sm, md, lg, xl, 2xl\nshadow = \"sm\"\nshadow = \"md\"\nshadow = \"lg\"\nshadow = \"xl\"\nshadow = \"2xl\""
            }
          }
        }

        # ── Rounded ──────────────────────────────────────────
        component "styling_rounded_title" {
          type = "label"
          text = "Rounded"
          size = "sm"
        }

        component "styling_rounded_tabs" {
          type = "tabs"
          tabs = ["Preview", "HCL"]

          component "styling_rounded_preview" {
            type = "stack"
            direction = "horizontal"
            spacing = 12

            component "styling_rounded_sm" {
              type = "label"
              text = "sm"
              rounded = "sm"
              border = 2
              border_color = "theme.border"
              padding = 16
            }

            component "styling_rounded_md" {
              type = "label"
              text = "md"
              rounded = "md"
              border = 2
              border_color = "theme.border"
              padding = 16
            }

            component "styling_rounded_lg" {
              type = "label"
              text = "lg"
              rounded = "lg"
              border = 2
              border_color = "theme.border"
              padding = 16
            }

            component "styling_rounded_xl" {
              type = "label"
              text = "xl"
              rounded = "xl"
              border = 2
              border_color = "theme.border"
              padding = 16
            }

            component "styling_rounded_full" {
              type = "label"
              text = "full"
              rounded = "full"
              border = 2
              border_color = "theme.border"
              padding = 16
            }
          }

          component "styling_rounded_hcl" {
            type = "panel"

            component "styling_rounded_code" {
              type = "text"
              content = "# Rounded sizes: sm, md, lg, xl, full\nrounded = \"sm\"\nrounded = \"md\"\nrounded = \"lg\"\nrounded = \"xl\"\nrounded = \"full\""
            }
          }
        }

        # ── Combined ─────────────────────────────────────────
        component "styling_combined_title" {
          type = "label"
          text = "Combined"
          size = "sm"
        }

        component "styling_combined_tabs" {
          type = "tabs"
          tabs = ["Preview", "HCL"]

          component "styling_combined_preview" {
            type = "stack"
            direction = "horizontal"
            spacing = 16

            component "styling_card1" {
              type = "stack"
              direction = "vertical"
              spacing = 8
              padding = 24
              border = 1
              border_color = "theme.border"
              shadow = "md"
              rounded = "lg"

              component "styling_card1_title" {
                type = "label"
                text = "Styled Card"
                size = "md"
              }

              component "styling_card1_body" {
                type = "text"
                content = "This card combines border, shadow, rounded, and padding to create a polished container."
              }
            }

            component "styling_card2" {
              type = "stack"
              direction = "vertical"
              spacing = 8
              padding = 24
              border = 2
              border_color = "theme.primary"
              shadow = "lg"
              rounded = "xl"

              component "styling_card2_title" {
                type = "label"
                text = "Accent Card"
                size = "md"
              }

              component "styling_card2_body" {
                type = "text"
                content = "An accent-bordered card with a larger shadow and rounder corners."
              }
            }
          }

          component "styling_combined_hcl" {
            type = "panel"

            component "styling_combined_code" {
              type = "text"
              content = "component \"styled_card\" {\n  type         = \"stack\"\n  direction    = \"vertical\"\n  spacing      = 8\n  padding      = 24\n  border       = 1\n  border_color = \"theme.border\"\n  shadow       = \"md\"\n  rounded      = \"lg\"\n\n  component \"title\" {\n    type = \"label\"\n    text = \"Styled Card\"\n  }\n\n  component \"body\" {\n    type    = \"text\"\n    content = \"Card content here.\"\n  }\n}"
            }
          }
        }
      }

      # ── Sidenav Bar page ─────────────────────────────────────
      component "page_sidenavbar" {
        template = "content_page"

        component "sidenav_title" {
          type = "label"
          text = "Sidenav Bar"
          size = "xl"
        }

        component "sidenav_desc" {
          type    = "text"
          content = "A vertical navigation sidebar with icon and label items. Supports collapsed (icon-only) and expanded (icon + label) modes."
        }

        # Three sidenav bars side by side in a horizontal stack
        component "sidenav_demos" {
          type      = "stack"
          direction = "horizontal"
          spacing   = 24

          # ── Left: collapsed (icons only) ──────────────────────
          component "sidenav_demo_left_collapsed" {
            type = "stack"
            direction = "vertical"
            spacing = 8

            component "sidenav_demo_left_collapsed_label" {
              type = "label"
              text = "Collapsed (icons only)"
              size = "sm"
            }

            component "sidenav_left_collapsed_wrap" {
              type      = "stack"
              direction = "horizontal"
              spacing   = 0
              width     = 240
              height    = 300
              border    = 1

              component "sidenav_left_collapsed" {
                type      = "sidenav_bar"
                collapsed = true

                component "snlc_item_home" {
                  type  = "sidenav_bar_item"
                  icon  = "globe"
                  label = "Home"
                }

                component "snlc_item_inbox" {
                  type  = "sidenav_bar_item"
                  icon  = "inbox"
                  label = "Inbox"
                }

                component "snlc_item_search" {
                  type  = "sidenav_bar_item"
                  icon  = "search"
                  label = "Search"
                }

                component "snlc_item_settings" {
                  type  = "sidenav_bar_item"
                  icon  = "settings"
                  label = "Settings"
                }
              }
            }
          }

          # ── Center: expanded with toggle button ───────────────
          component "sidenav_demo_center_expanded" {
            type = "stack"
            direction = "vertical"
            spacing = 8

            component "sidenav_demo_center_label" {
              type = "label"
              text = "Expanded (with toggle)"
              size = "sm"
            }

            component "sidenav_center_expanded_wrap" {
              type      = "stack"
              direction = "horizontal"
              spacing   = 0
              width     = 240
              height    = 300
              border    = 1

              component "sidenav_center_expanded" {
                type      = "sidenav_bar"
                collapsed = false

                component "snce_item_home" {
                  type  = "sidenav_bar_item"
                  icon  = "globe"
                  label = "Home"
                }

                component "snce_item_inbox" {
                  type  = "sidenav_bar_item"
                  icon  = "inbox"
                  label = "Inbox"
                }

                component "snce_item_search" {
                  type  = "sidenav_bar_item"
                  icon  = "search"
                  label = "Search"
                }

                component "snce_item_settings" {
                  type  = "sidenav_bar_item"
                  icon  = "settings"
                  label = "Settings"
                }

                component "snce_toggle_btn" {
                  type     = "button"
                  label    = "Toggle"
                  variant  = "ghost"
                  size     = "sm"
                  icon     = "menu"
                  on_click = "on_toggle_sidenav"
                }
              }
            }
          }

          # ── Right: collapsed (icons only) ─────────────────────
          component "sidenav_demo_right_collapsed" {
            type = "stack"
            direction = "vertical"
            spacing = 8

            component "sidenav_demo_right_collapsed_label" {
              type = "label"
              text = "Collapsed (right side)"
              size = "sm"
            }

            component "sidenav_right_collapsed_wrap" {
              type   = "stack"
              direction = "horizontal"
              spacing = 0
              width  = 240
              height = 300
              border = 1

              # Spacer pushes sidenav to the right
              component "snrc_spacer" {
                type = "panel"
                flex = 1
              }

              component "sidenav_right_collapsed" {
                type      = "sidenav_bar"
                collapsed = true

                component "snrc_item_user" {
                  type  = "sidenav_bar_item"
                  icon  = "user"
                  label = "Profile"
                }

                component "snrc_item_bell" {
                  type  = "sidenav_bar_item"
                  icon  = "bell"
                  label = "Notifications"
                }

                component "snrc_item_star" {
                  type  = "sidenav_bar_item"
                  icon  = "star"
                  label = "Favorites"
                }

                component "snrc_item_settings" {
                  type  = "sidenav_bar_item"
                  icon  = "settings"
                  label = "Settings"
                }
              }
            }
          }
        }

        # HCL example
        component "sidenav_hcl_tabs" {
          type = "tabs"
          tabs = ["Preview", "HCL"]

          component "sidenav_preview_placeholder" {
            type    = "text"
            content = "See the sidenav bar demos above."
          }

          component "sidenav_hcl_code" {
            type = "panel"

            component "sidenav_hcl_text" {
              type    = "text"
              content = "component \"my_sidenav\" {\n  type      = \"sidenav_bar\"\n  collapsed = false\n\n  component \"item_home\" {\n    type  = \"sidenav_bar_item\"\n    icon  = \"globe\"\n    label = \"Home\"\n  }\n\n  component \"item_inbox\" {\n    type  = \"sidenav_bar_item\"\n    icon  = \"inbox\"\n    label = \"Inbox\"\n  }\n}"
            }
          }
        }
      }
    }
  }
}
