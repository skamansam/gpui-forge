use gpui_kit::*;

struct Studio;

impl Render for Studio {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .items_center()
            .justify_center()
            .child(format!(
                "Forge Studio (base: {})",
                gpui_forge_runtime::base::BASE_ID
            ))
    }
}

fn main() {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(|cx| {
            gpui_kit::init(cx);
            gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| cx.new(|_| Studio))
                .expect("failed to open window");
        });
}
