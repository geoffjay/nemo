use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    label::Label,
    IconName, Sizable as _, TitleBar,
};

pub struct HeaderBar {
    title: String,
}

impl HeaderBar {
    pub fn new(title: String, _window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self { title }
    }
}

impl Render for HeaderBar {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let github_button = Button::new("github")
            .icon(IconName::GitHub)
            .small()
            .ghost()
            .on_click(|_, _, cx| cx.open_url("https://github.com/geoffjay/nemo"));

        let title = self.title.clone();

        TitleBar::new().child(
            h_flex()
                .w_full()
                .h(px(32.))
                .pr_2()
                .justify_between()
                .child(Label::new(title).text_xs())
                .child(div().flex().items_center().child(github_button)),
        )
    }
}
