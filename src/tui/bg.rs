use cursive::{
    theme::{ColorStyle, Effect, PaletteColor},
    views::TextView,
    Printer, Vec2, View,
};

pub fn background_text(text: &str, subtext: &str) -> BackgroundTextView {
    BackgroundTextView::new(text, subtext)
}

pub struct BackgroundTextView {
    text_view: TextView,
    subtext_view: TextView,
}

impl BackgroundTextView {
    pub fn new(text: &str, subtext: &str) -> Self {
        Self {
            text_view: TextView::new(text),
            subtext_view: TextView::new(subtext)
                .h_align(cursive::align::HAlign::Right)
                .style(Effect::Dim),
        }
    }
}

impl View for BackgroundTextView {
    fn draw(&self, printer: &Printer) {
        let mut sub_printer = printer.offset((1, 1)).shrinked((1, 0));
        sub_printer.set_color(ColorStyle::new(
            PaletteColor::View,
            PaletteColor::Background,
        ));

        self.text_view.draw(&sub_printer);

        let mut sub_printer = printer
            .offset((1, printer.size.y.saturating_sub(2)))
            .shrinked((1, 0));
        sub_printer.set_color(ColorStyle::new(
            PaletteColor::View,
            PaletteColor::Background,
        ));

        self.subtext_view.draw(&sub_printer);
    }

    fn layout(&mut self, size: Vec2) {
        self.text_view
            .layout((size.x.saturating_sub(2), size.y.saturating_sub(1)).into());
        self.subtext_view
            .layout((size.x.saturating_sub(2), size.y.saturating_sub(1)).into());
    }

    fn needs_relayout(&self) -> bool {
        self.text_view.needs_relayout() || self.subtext_view.needs_relayout()
    }

    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        constraint
    }
}
