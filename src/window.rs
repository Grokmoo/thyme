use crate::{Frame, widget::WidgetBuilder, WidgetState, Point};

/// A [`WidgetBuilder`](struct.WidgetBuilder.html) specifically for creating windows.
///
/// Windows can have a titlebar, close button, move, and resizing capabilities.  Each window
/// is automatically part of its own [`render group`](struct.WidgetBuilder.html#method.new_render_group)
/// and will automatically come on top of other widgets when clicked on.  You can create a `WindowBuilder`
/// from a [`WidgetBuilder`](struct.WidgetBuilder.html) by calling [`window`](struct.WidgetBuilder.html#method.window)
/// after any calls to general purpose widget layout.
///
/// There is also a [`window method on Frame`](struct.Frame.html#method.window) as a convenience for simple cases.
///
/// Once you are finished setting up the window, you call [`children`](#method.children) to add children and add the widget
/// to the frame

// TODO add theme yaml sample here
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

    /// Specifies whether the created window should show a titlebar.
    #[must_use]
    pub fn with_titlebar(mut self, with_titlebar: bool) -> WindowBuilder<'a> {
        self.state.with_titlebar = with_titlebar;
        self
    }

    /// Specifies whether the created window should have a close button.
    #[must_use]
    pub fn with_close_button(mut self, with_close_button: bool) -> WindowBuilder<'a> {
        self.state.with_close_button = with_close_button;
        self
    }

    /// Specifies whether the user should be able to move the created window
    /// by dragging the mouse.  Note that if the [`titlebar`](#method.with_titlebar) is not shown, there
    /// will be no way to move the window regardless of this setting.
    #[must_use]
    pub fn moveable(mut self, moveable: bool) -> WindowBuilder<'a> {
        self.state.moveable = moveable;
        self
    }

    /// Specifies whether the user should be able to resize the created window.
    /// If false, the resize handle will not be shown.
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