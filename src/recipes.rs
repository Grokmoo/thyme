use crate::{Align, Frame, WidgetState};

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

    pub fn progress_bar(&mut self, theme: &str, frac: f32) {
        self.start(theme)
        .children(|ui| {
            let mut builder = ui.start("bar");

            let mut rect = builder.trigger_layout();
            rect.size.x *= frac;

            builder.clip(rect).finish();
        });
    }

    pub fn scrollpane<F: Fn(&mut Frame)>(&mut self, theme: &str, content_id: &str, children: F) {
        self.start(theme)
        .children(|ui| {
            let mut content = ui.start("content").id(content_id);
            let rect = content.trigger_layout();
            content.clip(rect)
            .children(children);

            let pane_total_size = ui.parent_max_child_pos() - rect.pos;

            // check whether to show horizontal scrollbar
            if pane_total_size.x > rect.size.x {
                ui.start("scrollbar_horizontal")
                .align(Align::BotLeft)
                .pos(0.0, 0.0)
                .children(|ui| {
                    if ui.button("right", "").clicked {
                        ui.change_scroll(content_id, -10.0, 0.0);
                    }
                    if ui.button("left", "").clicked {
                        ui.change_scroll(content_id, 10.0, 0.0);
                    }
                });
            }

            // check whether to show vertical scrollbar
            if pane_total_size.y > rect.size.y {
                ui.start("scrollbar_vertical")
                .align(Align::TopRight)
                .pos(0.0, 0.0)
                .children(|ui| {
                    if ui.button("up", "").clicked {
                        ui.change_scroll(content_id, 0.0, 10.0);
                    }
                    if ui.button("down", "").clicked {
                        ui.change_scroll(content_id, 0.0, -10.0);
                    }
                });
            }
        });
    }

    pub fn window<F: Fn(&mut Frame)>(&mut self, theme: &str, id: &str, children: F) {
        self
        .start(theme)
        .id(id)
        .children(|ui| {
            let result = ui.start("titlebar")
            .children(|ui| {
                ui.start("title").finish();

                if ui.button("close", "").clicked {
                    ui.set_open(id, false);
                }
            });

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
        });
    }
}