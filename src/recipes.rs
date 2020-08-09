use crate::{Frame, Point, Rect, WidgetState};

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
            let mut rect = Rect::default();

            ui.start("bar")
            .trigger_layout(&mut rect)
            .clip(Rect::new(rect.pos, Point::new(rect.size.x * frac, rect.size.y)))
            .finish();
        });
    }

    pub fn scrollpane<F: Fn(&mut Frame)>(&mut self, theme: &str, content_id: &str, children: F) {
        self.start(theme)
        .children(|ui| {
            let mut content_bounds = Rect::default();

            ui.start("content")
            .id(content_id)
            .trigger_layout(&mut content_bounds)
            .clip(content_bounds)
            .children(children);

            let content_min = content_bounds.pos;
            let content_max = content_bounds.pos + content_bounds.size;

            let pane_bounds = ui.parent_max_child_bounds();
            let pane_min = pane_bounds.pos;
            let pane_max = pane_bounds.pos + pane_bounds.size;

            // check whether to show horizontal scrollbar
            if pane_min.x < content_min.x || pane_max.x > content_max.x {
                ui.start("scrollbar_horizontal")
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
            if pane_min.y < content_min.y || pane_max.y > content_max.y {
                ui.start("scrollbar_vertical")
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