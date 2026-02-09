use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    label::Label,
    IconName, Sizable as _, TitleBar,
};

pub struct HeaderBar;

impl HeaderBar {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {}
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Render for HeaderBar {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let github_button = Button::new("github")
            .icon(IconName::GitHub)
            .small()
            .ghost()
            .on_click(|_, _, cx| cx.open_url("https://github.com/geoffjay/persona"));

        TitleBar::new().child(
            h_flex()
                .w_full()
                .h(px(32.))
                .pr_2()
                .justify_between()
                .child(Label::new("Persona").text_xs())
                .child(div().flex().items_center().child(github_button)),
        )
    }
}
