use gpui::*;
use nemo_macros::NemoComponent;

#[derive(IntoElement, NemoComponent)]
pub struct List {
    #[source]
    source: nemo_layout::BuiltComponent,
}

impl RenderOnce for List {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let items: Vec<String> = self
            .source
            .properties
            .get("items")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|v| {
                        v.as_str()
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| format!("{:?}", v))
                    })
                    .collect()
            })
            .unwrap_or_default();

        let mut el = div().flex().flex_col().gap_1();

        for item in items {
            el = el.child(
                div()
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .hover(|s| s.bg(rgb(0x313244)))
                    .child(item),
            );
        }

        el
    }
}
