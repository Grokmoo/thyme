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

    pub fn scrollpane<F: FnOnce(&mut Frame)>(&mut self, theme: &str, content_id: &str, children: F) {
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

            let mut delta_scroll = Point::default();
            // check whether to show horizontal scrollbar
            if pane_min.x < content_min.x || pane_max.x > content_max.x {
                ui.start("scrollbar_horizontal")
                .children(|ui| {
                    let mut right_rect = Rect::default();
                    let result = ui.start("right")
                    .enabled(pane_max.x > content_max.x)
                    .trigger_layout(&mut right_rect).finish();
                    if result.clicked {
                        delta_scroll.x -= 10.0;
                    }

                    let mut left_rect = Rect::default();
                    let result = ui.start("left")
                    .enabled(pane_min.x < content_min.x)
                    .trigger_layout(&mut left_rect).finish();
                    if result.clicked {
                        delta_scroll.x += 10.0;
                    }

                    // compute size and position for main scroll button
                    let start_frac = ((content_min.x - pane_min.x) / pane_bounds.size.x).max(0.0);
                    let width_frac = content_bounds.size.x / pane_bounds.size.x;

                    // assume left button starts at 0,0 within the parent widget
                    let min_x = left_rect.size.x;
                    let max_x = right_rect.pos.x - left_rect.pos.x;
                    let pos_x = min_x + start_frac * (max_x - min_x);
                    let pos_y = 0.0;
                    let size_x = width_frac * (max_x - min_x);
                    let size_y = left_rect.size.y;

                    let result = ui.start("scroll")
                    .size(size_x, size_y)
                    .pos(pos_x, pos_y)
                    .finish();

                    if result.pressed {
                        delta_scroll.x -= result.dragged.x / width_frac;
                    }
                });
            }

            // check whether to show vertical scrollbar
            if pane_min.y < content_min.y || pane_max.y > content_max.y {
                ui.start("scrollbar_vertical")
                .children(|ui| {
                    let mut top_rect = Rect::default();
                    let result = ui.start("up")
                    .enabled(pane_min.y < content_min.y)
                    .trigger_layout(&mut top_rect).finish();
                    if result.clicked {
                        delta_scroll.y += 10.0;
                    }

                    let mut bot_rect = Rect::default();
                    let result = ui.start("down")
                    .enabled(pane_max.y > content_max.y)
                    .trigger_layout(&mut bot_rect).finish();
                    if result.clicked {
                        delta_scroll.y -= 10.0;
                    }

                    // compute size and position for main scroll button
                    let start_frac = ((content_min.y - pane_min.y) / pane_bounds.size.y).max(0.0);
                    let height_frac = content_bounds.size.y / pane_bounds.size.y;

                    // assume top button starts at 0,0 within the parent widget
                    let min_y = top_rect.size.y;
                    let max_y = bot_rect.pos.y - top_rect.pos.y;
                    let pos_y = min_y + start_frac * (max_y - min_y);
                    let pos_x = 0.0;
                    let size_y = height_frac * (max_y - min_y);
                    let size_x = top_rect.size.x;

                    let result = ui.start("scroll")
                    .size(size_x, size_y)
                    .pos(pos_x, pos_y)
                    .finish();

                    if result.pressed {
                        delta_scroll.y -= result.dragged.y / height_frac;
                    }
                });
            }

            if delta_scroll != Point::default() {
                let min_scroll = content_max - pane_max;
                let max_scroll = content_min - pane_min;
                let delta = delta_scroll.max(min_scroll).min(max_scroll);

                ui.modify(content_id, |state| {
                    state.scroll = state.scroll + delta;
                });
            }
        });
    }

    pub fn window<F: FnOnce(&mut Frame)>(&mut self, theme: &str, id: &str, children: F) {
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