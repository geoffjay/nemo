//! Integration tests for TUI component rendering.

use nemo_config::Value;
use nemo_layout::BuiltComponent;
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use std::collections::HashMap;

fn make_component(id: &str, component_type: &str) -> BuiltComponent {
    BuiltComponent {
        id: id.to_string(),
        component_type: component_type.to_string(),
        properties: HashMap::new(),
        handlers: HashMap::new(),
        children: Vec::new(),
        parent: None,
    }
}

fn render_to_string(width: u16, height: u16, f: impl FnOnce(&mut ratatui::Frame)) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            f(frame);
        })
        .unwrap();
    // Extract buffer content
    let buf = terminal.backend().buffer().clone();
    let mut lines = Vec::new();
    for y in 0..height {
        let mut line = String::new();
        for x in 0..width {
            line.push_str(buf.cell((x, y)).unwrap().symbol());
        }
        lines.push(line.trim_end().to_string());
    }
    lines.join("\n")
}

// ── Label ───────────────────────────────────────────────────────────

#[test]
fn test_label_renders_text() {
    let mut comp = make_component("lbl", "label");
    comp.properties
        .insert("text".into(), Value::String("Hello World".into()));

    let output = render_to_string(20, 1, |frame| {
        nemo_tui::components::label::render(frame, frame.area(), &comp);
    });

    assert!(output.contains("Hello World"));
}

#[test]
fn test_label_empty_text() {
    let comp = make_component("lbl", "label");

    let output = render_to_string(20, 1, |frame| {
        nemo_tui::components::label::render(frame, frame.area(), &comp);
    });

    // Should render without panic, just whitespace
    assert_eq!(output.trim(), "");
}

// ── Text ────────────────────────────────────────────────────────────

#[test]
fn test_text_renders_content() {
    let mut comp = make_component("txt", "text");
    comp.properties.insert(
        "content".into(),
        Value::String("Multi-line text content".into()),
    );

    let output = render_to_string(30, 2, |frame| {
        nemo_tui::components::text::render(frame, frame.area(), &comp);
    });

    assert!(output.contains("Multi-line text content"));
}

#[test]
fn test_text_falls_back_to_text_property() {
    let mut comp = make_component("txt", "text");
    comp.properties
        .insert("text".into(), Value::String("Fallback text".into()));

    let output = render_to_string(30, 1, |frame| {
        nemo_tui::components::text::render(frame, frame.area(), &comp);
    });

    assert!(output.contains("Fallback text"));
}

// ── Progress ────────────────────────────────────────────────────────

#[test]
fn test_progress_renders_gauge() {
    let mut comp = make_component("prog", "progress");
    comp.properties
        .insert("value".into(), Value::Integer(50));
    comp.properties
        .insert("max".into(), Value::Integer(100));

    let output = render_to_string(30, 1, |frame| {
        nemo_tui::components::progress::render(frame, frame.area(), &comp);
    });

    // Gauge should show percentage
    assert!(output.contains("50%"));
}

#[test]
fn test_progress_custom_label() {
    let mut comp = make_component("prog", "progress");
    comp.properties
        .insert("value".into(), Value::Integer(75));
    comp.properties
        .insert("max".into(), Value::Integer(100));
    comp.properties
        .insert("label".into(), Value::String("CPU: 75%".into()));

    let output = render_to_string(30, 1, |frame| {
        nemo_tui::components::progress::render(frame, frame.area(), &comp);
    });

    assert!(output.contains("CPU: 75%"));
}

// ── Panel ───────────────────────────────────────────────────────────

#[test]
fn test_panel_renders_with_title() {
    let mut comp = make_component("pnl", "panel");
    comp.properties
        .insert("title".into(), Value::String("My Panel".into()));

    let components: HashMap<String, BuiltComponent> = HashMap::new();

    let output = render_to_string(30, 5, |frame| {
        nemo_tui::components::panel::render(frame, frame.area(), &comp, &components);
    });

    assert!(output.contains("My Panel"));
}

#[test]
fn test_panel_renders_children() {
    let mut panel = make_component("pnl", "panel");
    panel
        .properties
        .insert("title".into(), Value::String("Container".into()));
    panel.children = vec!["child_lbl".into()];

    let mut child = make_component("child_lbl", "label");
    child
        .properties
        .insert("text".into(), Value::String("Inside".into()));
    child.parent = Some("pnl".into());

    let mut components: HashMap<String, BuiltComponent> = HashMap::new();
    components.insert("pnl".into(), panel.clone());
    components.insert("child_lbl".into(), child);

    let output = render_to_string(30, 5, |frame| {
        nemo_tui::components::panel::render(frame, frame.area(), &panel, &components);
    });

    assert!(output.contains("Container"));
    assert!(output.contains("Inside"));
}

// ── Stack ───────────────────────────────────────────────────────────

#[test]
fn test_stack_vertical_layout() {
    let mut stack = make_component("stk", "stack");
    stack
        .properties
        .insert("direction".into(), Value::String("vertical".into()));
    stack.children = vec!["lbl1".into(), "lbl2".into()];

    let mut lbl1 = make_component("lbl1", "label");
    lbl1.properties
        .insert("text".into(), Value::String("First".into()));
    lbl1.parent = Some("stk".into());

    let mut lbl2 = make_component("lbl2", "label");
    lbl2.properties
        .insert("text".into(), Value::String("Second".into()));
    lbl2.parent = Some("stk".into());

    let mut components: HashMap<String, BuiltComponent> = HashMap::new();
    components.insert("stk".into(), stack.clone());
    components.insert("lbl1".into(), lbl1);
    components.insert("lbl2".into(), lbl2);

    let output = render_to_string(20, 4, |frame| {
        nemo_tui::components::stack::render(frame, frame.area(), &stack, &components);
    });

    assert!(output.contains("First"));
    assert!(output.contains("Second"));
}

#[test]
fn test_stack_horizontal_layout() {
    let mut stack = make_component("stk", "stack");
    stack
        .properties
        .insert("direction".into(), Value::String("horizontal".into()));
    stack.children = vec!["lbl1".into(), "lbl2".into()];

    let mut lbl1 = make_component("lbl1", "label");
    lbl1.properties
        .insert("text".into(), Value::String("Left".into()));
    lbl1.parent = Some("stk".into());

    let mut lbl2 = make_component("lbl2", "label");
    lbl2.properties
        .insert("text".into(), Value::String("Right".into()));
    lbl2.parent = Some("stk".into());

    let mut components: HashMap<String, BuiltComponent> = HashMap::new();
    components.insert("stk".into(), stack.clone());
    components.insert("lbl1".into(), lbl1);
    components.insert("lbl2".into(), lbl2);

    let output = render_to_string(30, 2, |frame| {
        nemo_tui::components::stack::render(frame, frame.area(), &stack, &components);
    });

    assert!(output.contains("Left"));
    assert!(output.contains("Right"));
}

// ── Table ───────────────────────────────────────────────────────────

#[test]
fn test_table_renders_with_data() {
    let mut comp = make_component("tbl", "table");
    comp.properties
        .insert("title".into(), Value::String("Services".into()));

    let columns = Value::Array(vec![
        Value::Object(
            [
                ("key".into(), Value::String("name".into())),
                ("label".into(), Value::String("Name".into())),
            ]
            .into_iter()
            .collect(),
        ),
        Value::Object(
            [
                ("key".into(), Value::String("status".into())),
                ("label".into(), Value::String("Status".into())),
            ]
            .into_iter()
            .collect(),
        ),
    ]);
    comp.properties.insert("columns".into(), columns);

    let data = Value::Array(vec![Value::Object(
        [
            ("name".into(), Value::String("API".into())),
            ("status".into(), Value::String("Running".into())),
        ]
        .into_iter()
        .collect(),
    )]);
    comp.properties.insert("data".into(), data);

    let output = render_to_string(40, 6, |frame| {
        nemo_tui::components::table::render(frame, frame.area(), &comp);
    });

    assert!(output.contains("Services"));
    assert!(output.contains("Name"));
    assert!(output.contains("Status"));
    assert!(output.contains("API"));
    assert!(output.contains("Running"));
}

// ── Renderer dispatch ───────────────────────────────────────────────

#[test]
fn test_renderer_unsupported_type_renders_children() {
    // An unsupported component type should still render its children
    let mut comp = make_component("unk", "unknown_widget");
    comp.children = vec!["child".into()];

    let mut child = make_component("child", "label");
    child
        .properties
        .insert("text".into(), Value::String("Fallback".into()));
    child.parent = Some("unk".into());

    let mut components: HashMap<String, BuiltComponent> = HashMap::new();
    components.insert("unk".into(), comp.clone());
    components.insert("child".into(), child);

    let output = render_to_string(20, 3, |frame| {
        nemo_tui::renderer::render_component(frame, frame.area(), &comp, &components);
    });

    assert!(output.contains("Fallback"));
}
