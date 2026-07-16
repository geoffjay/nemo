//! The XML configuration surface that lives *outside* per-component
//! `ConfigSchema`s.
//!
//! Two kinds of surface aren't captured by the component registry:
//!
//! * **Universal style attributes** — applied to *every* component wrapper by
//!   `apply_layout_styles` (in the `nemo` binary), so they're valid on almost any
//!   element but appear in no per-component schema.
//! * **Structural elements** — the top-level XML elements (`<app>`, `<window>`,
//!   `<layout>`, …) that the parser special-cases; they are not components.
//!
//! This module is the single source of truth for both, consumed by the
//! `nemo validate` linter (so it doesn't flag universal attributes as unknown)
//! and by the `nemo schema` exporter (so the published schema is complete). Keep
//! [`universal_style_attributes`] in sync with `apply_layout_styles`.
//!
//! Names use the internal snake_case form (attributes are `kebab_to_snake`'d at
//! parse time, so XML `max-width` / `on-load` arrive as `max_width` / `on_load`).

/// A named attribute with a coarse type and a human description.
#[derive(Debug, Clone, Copy)]
pub struct AttrDef {
    pub name: &'static str,
    pub value_type: &'static str,
    pub description: &'static str,
}

/// A structural (non-component) top-level XML element.
#[derive(Debug, Clone, Copy)]
pub struct StructuralElement {
    pub element: &'static str,
    pub description: &'static str,
    pub attributes: &'static [AttrDef],
    pub child_elements: &'static [&'static str],
}

/// An open-ended attribute family matched by prefix (e.g. `on-*`, `bind-*`).
#[derive(Debug, Clone, Copy)]
pub struct AttrFamily {
    pub prefix: &'static str,
    pub description: &'static str,
}

/// Universal styling attributes applied by `apply_layout_styles` to every
/// component wrapper, regardless of component type. Must mirror the property
/// names that function reads.
pub fn universal_style_attributes() -> &'static [AttrDef] {
    &[
        // Sizing
        AttrDef {
            name: "width",
            value_type: "integer",
            description: "Fixed width in pixels.",
        },
        AttrDef {
            name: "height",
            value_type: "integer",
            description: "Fixed height in pixels.",
        },
        AttrDef {
            name: "min_width",
            value_type: "integer",
            description: "Minimum width in pixels.",
        },
        AttrDef {
            name: "min_height",
            value_type: "integer",
            description: "Minimum height in pixels.",
        },
        AttrDef {
            name: "max_width",
            value_type: "integer",
            description: "Maximum width in pixels.",
        },
        AttrDef {
            name: "max_height",
            value_type: "integer",
            description: "Maximum height in pixels.",
        },
        AttrDef {
            name: "flex",
            value_type: "float",
            description: "Grow to fill the main axis when truthy (\"1\"/\"true\").",
        },
        AttrDef {
            name: "scroll",
            value_type: "boolean",
            description: "Scroll along the main axis (stacks); also grows.",
        },
        // Margin
        AttrDef {
            name: "margin",
            value_type: "integer",
            description: "Margin on all sides, in pixels.",
        },
        AttrDef {
            name: "margin_x",
            value_type: "integer",
            description: "Horizontal (left+right) margin, in pixels.",
        },
        AttrDef {
            name: "margin_y",
            value_type: "integer",
            description: "Vertical (top+bottom) margin, in pixels.",
        },
        AttrDef {
            name: "margin_left",
            value_type: "integer",
            description: "Left margin, in pixels.",
        },
        AttrDef {
            name: "margin_right",
            value_type: "integer",
            description: "Right margin, in pixels.",
        },
        AttrDef {
            name: "margin_top",
            value_type: "integer",
            description: "Top margin, in pixels.",
        },
        AttrDef {
            name: "margin_bottom",
            value_type: "integer",
            description: "Bottom margin, in pixels.",
        },
        // Padding
        AttrDef {
            name: "padding",
            value_type: "integer",
            description: "Padding on all sides, in pixels.",
        },
        AttrDef {
            name: "padding_x",
            value_type: "integer",
            description: "Horizontal (left+right) padding, in pixels.",
        },
        AttrDef {
            name: "padding_y",
            value_type: "integer",
            description: "Vertical (top+bottom) padding, in pixels.",
        },
        AttrDef {
            name: "padding_left",
            value_type: "integer",
            description: "Left padding, in pixels.",
        },
        AttrDef {
            name: "padding_right",
            value_type: "integer",
            description: "Right padding, in pixels.",
        },
        AttrDef {
            name: "padding_top",
            value_type: "integer",
            description: "Top padding, in pixels.",
        },
        AttrDef {
            name: "padding_bottom",
            value_type: "integer",
            description: "Bottom padding, in pixels.",
        },
        // Border
        AttrDef {
            name: "border",
            value_type: "integer",
            description: "Border width on all sides, in pixels.",
        },
        AttrDef {
            name: "border_x",
            value_type: "integer",
            description: "Left+right border width, in pixels.",
        },
        AttrDef {
            name: "border_y",
            value_type: "integer",
            description: "Top+bottom border width, in pixels.",
        },
        AttrDef {
            name: "border_left",
            value_type: "integer",
            description: "Left border width, in pixels.",
        },
        AttrDef {
            name: "border_right",
            value_type: "integer",
            description: "Right border width, in pixels.",
        },
        AttrDef {
            name: "border_top",
            value_type: "integer",
            description: "Top border width, in pixels.",
        },
        AttrDef {
            name: "border_bottom",
            value_type: "integer",
            description: "Bottom border width, in pixels.",
        },
        AttrDef {
            name: "border_color",
            value_type: "string",
            description: "Border color: a hex value or a `theme.<name>` reference.",
        },
        // Decoration
        AttrDef {
            name: "shadow",
            value_type: "string",
            description: "Shadow preset: sm | md | lg | xl | 2xl.",
        },
        AttrDef {
            name: "rounded",
            value_type: "string",
            description: "Corner rounding preset: sm | md | lg | xl | full.",
        },
        AttrDef {
            name: "background",
            value_type: "string",
            description: "Background color: a hex value or a `theme.<name>` reference.",
        },
    ]
}

/// Open-ended attribute families matched by prefix in the parser/runtime.
pub fn attribute_families() -> &'static [AttrFamily] {
    &[
        AttrFamily {
            prefix: "on-",
            description: "Event handler. `on-<event>=\"fn\"` wires the event to a Rhai function (bare name, or `script_id::fn`).",
        },
        AttrFamily {
            prefix: "bind-",
            description: "One-way data binding. `bind-<property>=\"path\"` binds the property to a data-repository path.",
        },
    ]
}

/// Structural (non-component) top-level elements special-cased by the parser.
/// Hand-authored: these have no entry in the component registry.
pub fn structural_elements() -> &'static [StructuralElement] {
    &[
        StructuralElement {
            element: "nemo",
            description: "Document root; wraps the whole configuration.",
            attributes: &[],
            child_elements: &[
                "app",
                "script",
                "data",
                "templates",
                "template",
                "variable",
                "include",
                "layout",
            ],
        },
        StructuralElement {
            element: "app",
            description: "Application metadata and window/theme configuration.",
            attributes: &[AttrDef {
                name: "title",
                value_type: "string",
                description: "Application title.",
            }],
            child_elements: &["window", "theme", "plugins"],
        },
        StructuralElement {
            element: "window",
            description: "Window configuration.",
            attributes: &[AttrDef {
                name: "title",
                value_type: "string",
                description: "Window title.",
            }],
            child_elements: &["header-bar"],
        },
        StructuralElement {
            element: "header-bar",
            description: "Title-bar chrome shown at the top of the window.",
            attributes: &[
                AttrDef {
                    name: "github_url",
                    value_type: "string",
                    description: "Optional GitHub link shown in the header.",
                },
                AttrDef {
                    name: "theme_toggle",
                    value_type: "boolean",
                    description: "Show a light/dark theme toggle.",
                },
            ],
            child_elements: &[],
        },
        StructuralElement {
            element: "theme",
            description: "Theme selection.",
            attributes: &[
                AttrDef {
                    name: "name",
                    value_type: "string",
                    description: "Theme name (e.g. nord, kanagawa).",
                },
                AttrDef {
                    name: "mode",
                    value_type: "string",
                    description: "Theme mode: light | dark.",
                },
            ],
            child_elements: &[],
        },
        StructuralElement {
            element: "script",
            description: "Rhai script configuration and inline code.",
            attributes: &[
                AttrDef {
                    name: "src",
                    value_type: "string",
                    description: "Path to a script file or directory.",
                },
                AttrDef {
                    name: "features",
                    value_type: "string",
                    description: "Opt-in capability set (e.g. file-io, system, science).",
                },
                AttrDef {
                    name: "on_load",
                    value_type: "string",
                    description: "Handler run once after scripts load and the layout is built.",
                },
            ],
            child_elements: &[],
        },
        StructuralElement {
            element: "data",
            description: "Container for data sources and sinks.",
            attributes: &[],
            child_elements: &["source", "sink"],
        },
        StructuralElement {
            element: "source",
            description:
                "A data source. Type-specific attributes come from the dataSources catalog.",
            attributes: &[
                AttrDef {
                    name: "name",
                    value_type: "string",
                    description: "Unique source name (referenced by bindings).",
                },
                AttrDef {
                    name: "type",
                    value_type: "string",
                    description:
                        "Source type (see dataSources): http | websocket | timer | file | …",
                },
            ],
            child_elements: &[],
        },
        StructuralElement {
            element: "sink",
            description: "A data sink.",
            attributes: &[
                AttrDef {
                    name: "name",
                    value_type: "string",
                    description: "Unique sink name.",
                },
                AttrDef {
                    name: "type",
                    value_type: "string",
                    description: "Sink type.",
                },
            ],
            child_elements: &[],
        },
        StructuralElement {
            element: "layout",
            description: "Root layout wrapper; its children are the component tree.",
            attributes: &[AttrDef {
                name: "type",
                value_type: "string",
                description: "Layout type: stack | grid | dock | tiles (default stack).",
            }],
            child_elements: &[],
        },
        StructuralElement {
            element: "templates",
            description: "Container for reusable component templates.",
            attributes: &[],
            child_elements: &["template"],
        },
        StructuralElement {
            element: "template",
            description: "A reusable component subtree, referenced via `template=\"name\"`.",
            attributes: &[AttrDef {
                name: "name",
                value_type: "string",
                description: "Template name.",
            }],
            child_elements: &[],
        },
        StructuralElement {
            element: "variable",
            description: "A configuration variable, referenced via `${var.name}`.",
            attributes: &[
                AttrDef {
                    name: "name",
                    value_type: "string",
                    description: "Variable name.",
                },
                AttrDef {
                    name: "type",
                    value_type: "string",
                    description: "Value type.",
                },
                AttrDef {
                    name: "default",
                    value_type: "string",
                    description: "Default value.",
                },
                AttrDef {
                    name: "value",
                    value_type: "string",
                    description: "Value.",
                },
            ],
            child_elements: &[],
        },
        StructuralElement {
            element: "include",
            description: "Include another configuration file.",
            attributes: &[AttrDef {
                name: "src",
                value_type: "string",
                description: "Path to the file to include.",
            }],
            child_elements: &[],
        },
        StructuralElement {
            element: "slot",
            description: "A named slot inside a template, filled at expansion.",
            attributes: &[AttrDef {
                name: "name",
                value_type: "string",
                description: "Slot name.",
            }],
            child_elements: &[],
        },
        StructuralElement {
            element: "binding",
            description: "An explicit data binding as a child of a component.",
            attributes: &[
                AttrDef {
                    name: "source",
                    value_type: "string",
                    description: "Data-repository source path.",
                },
                AttrDef {
                    name: "target",
                    value_type: "string",
                    description: "Target property on the parent component.",
                },
                AttrDef {
                    name: "mode",
                    value_type: "string",
                    description: "Binding mode: one_way (default) | two_way.",
                },
                AttrDef {
                    name: "transform",
                    value_type: "string",
                    description: "Optional field-extraction/value template.",
                },
            ],
            child_elements: &[],
        },
    ]
}
