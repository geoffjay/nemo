use gpui::*;
use gpui_component::ActiveTheme;
use nemo_layout::BuiltComponent;

/// A list display component.
///
/// # XML Configuration
///
/// ```xml
/// <list id="fruits">
///   <list-item><text content="Apple" /></list-item>
///   <list-item><text content="Banana" /></list-item>
/// </list>
/// ```
///
/// Each `<list-item>` child is one row; the item's own children form the row
/// content, so rows can hold arbitrary components.
#[derive(IntoElement)]
#[allow(dead_code)]
pub struct List {
    source: BuiltComponent,
    /// Rendered content of each `<list-item>` child (one entry per row).
    items: Vec<Vec<AnyElement>>,
}

impl List {
    pub fn new(source: BuiltComponent) -> Self {
        Self {
            source,
            items: Vec::new(),
        }
    }

    pub fn items(mut self, items: Vec<Vec<AnyElement>>) -> Self {
        self.items = items;
        self
    }
}

impl RenderOnce for List {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let list_hover = cx.theme().colors.list_hover;
        let mut el = div().flex().flex_col().gap_1();

        for body in self.items {
            el = el.child(
                div()
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .hover(move |s| s.bg(list_hover))
                    .children(body),
            );
        }

        el
    }
}
