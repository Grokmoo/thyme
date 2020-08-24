use crate::{Frame, widget::WidgetBuilder, WidgetState, Point};

pub struct WindowBuilder<'a> {
    builder: WidgetBuilder<'a>,
    state: WindowState,
}

impl<'a> WindowBuilder<'a> {
    pub(crate) fn new(builder: WidgetBuilder<'a>) -> WindowBuilder<'a> {
        WindowBuilder {
            builder,
            state: WindowState::default(),
        }
    }

    #[must_use]
    pub fn with_titlebar(mut self, with_titlebar: bool) -> WindowBuilder<'a> {
        self.state.with_titlebar = with_titlebar;
        self
    }

    #[must_use]
    pub fn with_close_button(mut self, with_close_button: bool) -> WindowBuilder<'a> {
        self.state.with_close_button = with_close_button;
        self
    }

    #[must_use]
    pub fn moveable(mut self, moveable: bool) -> WindowBuilder<'a> {
        self.state.moveable = moveable;
        self
    }

    #[must_use]
    pub fn resizable(mut self, resizable: bool) -> WindowBuilder<'a> {
        self.state.resizable = resizable;
        self
    }

    /// Consumes the builder and adds a widget to the current frame.  The
    /// returned data includes information about the animation state and
    /// mouse interactions of the created element.
    /// The provided closure is called to enable adding children to this window.
    pub fn children<F: FnOnce(&mut Frame)>(self, children: F) -> WidgetState {
        let builder = self.builder;
        let state = self.state;
        let id = builder.widget.id().to_string();

        builder.children(|ui| {
            let drag_move = if state.with_titlebar {
                let result = ui.start("titlebar")
                .children(|ui| {
                    ui.start("title").finish();

                    if state.with_close_button {
                        let clicked = ui.button("close", "").clicked;

                        if clicked {
                            ui.close(&id);
                        }
                    }
                });

                if state.moveable && result.pressed {
                    result.dragged
                } else {
                    Point::default()
                }
            } else {
                Point::default()
            };

            if drag_move != Point::default() {
                ui.modify(&id, |state| {
                    state.moved = state.moved + drag_move;
                });
            }

            if state.resizable {
                let result = ui.button("handle", "");
                if result.pressed {
                    ui.modify(&id, |state| {
                        state.resize = state.resize + result.dragged;
                    });
                }
            }

            (children)(ui);
        })
    }
}

struct WindowState {
    with_titlebar: bool,
    with_close_button: bool,
    moveable: bool,
    resizable: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            with_titlebar: true,
            with_close_button: true,
            moveable: true,
            resizable: true,
        }
    }
}