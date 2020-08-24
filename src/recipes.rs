use std::fmt::Display;

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

    pub fn combo_box<'a, T: Display>(&mut self, theme: &str, id: &str, current: T, values: &'a [T]) -> Option<&'a T> {
        let popup_id = format!("{}_popup", id);
        
        let mut rect = Rect::default();

        if self.start(theme)
        .text(current.to_string())
        .wants_mouse(true)
        .trigger_layout(&mut rect)
        .finish().clicked {
            self.open_modal(&popup_id);
            self.close_modal_on_click_outside();
        }

        let mut result = None;

        // TODO combo box popup to show up on top
        self.start(&format!("{}_popup", id))
        .id(&popup_id)
        .screen_pos(rect.pos.x, rect.pos.y + rect.size.y)
        .initially_open(false)
        .scrollpane("cb_popup_content", |ui| {
            for value in values {
                if ui.button("entry", value.to_string()).clicked {
                    result = Some(value);
                    ui.close(&popup_id);
                }
            }
        });

        result
    }

    pub fn toggle_button<T: Into<String>>(&mut self, theme: &str, label: T, active: bool) -> WidgetState {
        self.start(theme).text(label).active(active).wants_mouse(true).finish()
    }

    pub fn input_field(&mut self, theme: &str, id: &str) -> Option<String> {
        let mut text_out = None;

        self.modify(id, |state| {
            if state.text.is_none() {
                state.text = Some(String::new());
            }

            let mut text_changed = false;
            for c in state.characters.drain(..) {
                if c as u32 == 8 { //backspace
                    state.text.as_mut().unwrap().pop();
                    text_changed = true;
                } else {
                    state.text.as_mut().unwrap().push(c);
                    text_changed = true;
                }
            }

            if text_changed {
                text_out = state.text.clone();
            }
        });
        let mut text_pos = Point::default();

        let result = self.start(theme)
        .id(id)
        .trigger_text_layout(&mut text_pos)
        .children(|ui| {
            if ui.is_focus_keyboard(id) {
                ui.start("caret").pos(text_pos.x, text_pos.y).finish();
            }
        });

        if result.clicked {
            self.focus_keyboard(id);
        }

        text_out
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

    pub fn window<F: FnOnce(&mut Frame)>(&mut self, theme: &str, id: &str, children: F) {
        self
        .start(theme)
        .window(id)
        .children(|ui| {
            (children)(ui);
        });
    }

    pub fn scrollpane<F: FnOnce(&mut Frame)>(&mut self, theme_id: &str, content_id: &str, children: F) {
        self.start(theme_id)
        .scrollpane(content_id, children);
    }
}

pub fn scrollpane_content<'a, F: FnOnce(&mut Frame) + 'a>(content_id: &'a str, children: F) -> impl FnOnce(&mut Frame) + 'a {
    move |ui| {
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
    }
}