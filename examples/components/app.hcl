# Component Gallery
# Interactive showcase of all Nemo components with sidebar navigation.

app {
  title = "Nemo Component Gallery"

  window {
    title = "Component Gallery"
    width = 1200
    height = 800
  }

  theme {
    background = "#1e1e2e"
    foreground = "#cdd6f4"
  }
}

scripts {
  path = "./scripts"
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

        component "sidebar_title" {
          type = "label"
          text = "Components"
          size = "lg"
        }

        component "nav_button" {
          type = "button"
          label = "Button"
          variant = "ghost"
          on_click = "on_nav"
        }

        component "nav_label" {
          type = "button"
          label = "Label"
          variant = "ghost"
          on_click = "on_nav"
        }

        component "nav_icon" {
          type = "button"
          label = "Icon"
          variant = "ghost"
          on_click = "on_nav"
        }

        component "nav_checkbox" {
          type = "button"
          label = "Checkbox"
          variant = "ghost"
          on_click = "on_nav"
        }

        component "nav_input" {
          type = "button"
          label = "Input"
          variant = "ghost"
          on_click = "on_nav"
        }

        component "nav_select" {
          type = "button"
          label = "Select"
          variant = "ghost"
          on_click = "on_nav"
        }

        component "nav_progress" {
          type = "button"
          label = "Progress"
          variant = "ghost"
          on_click = "on_nav"
        }

        component "nav_notification" {
          type = "button"
          label = "Notification"
          variant = "ghost"
          on_click = "on_nav"
        }

        component "nav_modal" {
          type = "button"
          label = "Modal"
          variant = "ghost"
          on_click = "on_nav"
        }

        component "nav_text" {
          type = "button"
          label = "Text"
          variant = "ghost"
          on_click = "on_nav"
        }

        component "nav_list" {
          type = "button"
          label = "List"
          variant = "ghost"
          on_click = "on_nav"
        }

        component "nav_panel" {
          type = "button"
          label = "Panel & Stack"
          variant = "ghost"
          on_click = "on_nav"
        }

        component "nav_table" {
          type = "button"
          label = "Table"
          variant = "ghost"
          on_click = "on_nav"
        }

        component "nav_tree" {
          type = "button"
          label = "Tree"
          variant = "ghost"
          on_click = "on_nav"
        }

        component "nav_image" {
          type = "button"
          label = "Image"
          variant = "ghost"
          on_click = "on_nav"
        }
      }
    }

    # ── Content area ─────────────────────────────────────────
    component "content" {
      type = "stack"
      direction = "vertical"
      spacing = 0

      # ── Button page (visible by default) ───────────────────
      component "page_button" {
        type = "panel"
        visible = true

        component "page_button_inner" {
          type = "stack"
          direction = "vertical"
          spacing = 12

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
      }

      # ── Label page ─────────────────────────────────────────
      component "page_label" {
        type = "panel"
        visible = false

        component "page_label_inner" {
          type = "stack"
          direction = "vertical"
          spacing = 12

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
      }

      # ── Icon page ──────────────────────────────────────────
      component "page_icon" {
        type = "panel"
        visible = false

        component "page_icon_inner" {
          type = "stack"
          direction = "vertical"
          spacing = 12

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
      }

      # ── Checkbox page ──────────────────────────────────────
      component "page_checkbox" {
        type = "panel"
        visible = false

        component "page_checkbox_inner" {
          type = "stack"
          direction = "vertical"
          spacing = 12

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
      }

      # ── Input page ─────────────────────────────────────────
      component "page_input" {
        type = "panel"
        visible = false

        component "page_input_inner" {
          type = "stack"
          direction = "vertical"
          spacing = 12

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
      }

      # ── Select page ────────────────────────────────────────
      component "page_select" {
        type = "panel"
        visible = false

        component "page_select_inner" {
          type = "stack"
          direction = "vertical"
          spacing = 12

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
      }

      # ── Progress page ──────────────────────────────────────
      component "page_progress" {
        type = "panel"
        visible = false

        component "page_progress_inner" {
          type = "stack"
          direction = "vertical"
          spacing = 12

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
      }

      # ── Notification page ──────────────────────────────────
      component "page_notification" {
        type = "panel"
        visible = false

        component "page_notif_inner" {
          type = "stack"
          direction = "vertical"
          spacing = 12

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
      }

      # ── Modal page ─────────────────────────────────────────
      component "page_modal" {
        type = "panel"
        visible = false

        component "page_modal_inner" {
          type = "stack"
          direction = "vertical"
          spacing = 12

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
      }

      # ── Text page ──────────────────────────────────────────
      component "page_text" {
        type = "panel"
        visible = false

        component "page_text_inner" {
          type = "stack"
          direction = "vertical"
          spacing = 12

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
      }

      # ── List page ──────────────────────────────────────────
      component "page_list" {
        type = "panel"
        visible = false

        component "page_list_inner" {
          type = "stack"
          direction = "vertical"
          spacing = 12

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
      }

      # ── Panel & Stack page ─────────────────────────────────
      component "page_panel" {
        type = "panel"
        visible = false

        component "page_panel_inner" {
          type = "stack"
          direction = "vertical"
          spacing = 12

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
      }

      # ── Table page ─────────────────────────────────────────
      component "page_table" {
        type = "panel"
        visible = false

        component "page_table_inner" {
          type = "stack"
          direction = "vertical"
          spacing = 12

          component "table_title" {
            type = "label"
            text = "Table"
            size = "lg"
          }

          component "table_desc" {
            type = "text"
            content = "Table displays tabular data. Currently a placeholder awaiting full implementation."
          }

          component "table_demo" {
            type = "table"
          }
        }
      }

      # ── Tree page ──────────────────────────────────────────
      component "page_tree" {
        type = "panel"
        visible = false

        component "page_tree_inner" {
          type = "stack"
          direction = "vertical"
          spacing = 12

          component "tree_title" {
            type = "label"
            text = "Tree"
            size = "lg"
          }

          component "tree_desc" {
            type = "text"
            content = "Tree displays hierarchical data. Currently a placeholder awaiting full implementation."
          }

          component "tree_demo" {
            type = "tree"
          }
        }
      }

      # ── Image page ─────────────────────────────────────────
      component "page_image" {
        type = "panel"
        visible = false

        component "page_image_inner" {
          type = "stack"
          direction = "vertical"
          spacing = 12

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
      }
    }
  }
}
