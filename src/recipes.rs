use crate::{Frame, Point};

impl Frame {
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