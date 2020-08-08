use crate::{Frame, Point, WidgetState};

impl Frame {
    pub fn child(&mut self, theme: &str) -> WidgetState {
        self.start(theme).finish()
    }

    pub fn label<T: Into<String>>(&mut self, theme: &str, text: T) {
        self.start(theme).text(text).finish();
    }

    pub fn button<T: Into<String>>(&mut self, theme: &str, label: T) -> WidgetState {
        self.start(theme).text(label).wants_mouse(true).finish()
    }

    pub fn toggle_button<T: Into<String>>(&mut self, theme: &str, label: T, active: bool) -> WidgetState {
        self.start(theme).text(label).active(active).wants_mouse(true).finish()
    }

    pub fn window<F: Fn(&mut Frame)>(&mut self, theme: &str, id: &str, size: Point, children: F) {
        self
        .start(theme)
        .size(size.x, size.y)
        .pos(0.0, 0.0)
        .id(id)
        .children(|ui| {
            let result = ui.start("titlebar")
            .children(|ui| {
                ui.label("title", "Window Title");

                if ui.button("close", "").clicked {
                    ui.set_open(id, false);
                }
            }).finish();

            if result.pressed {
                ui.modify(id, |state| {
                    state.moved = state.moved + result.dragged;
                });
            }

            let result = ui.button("handle", "");
            if result.pressed {
                ui.modify(id, |state| {
                    state.resize = state.resize + result.dragged;
                });
            }

            (children)(ui);
        })
        .finish();
    }
}