use crate::{Frame, widget::WidgetBuilder, WidgetState, Point};

/**
A [`WidgetBuilder`](struct.WidgetBuilder.html) specifically for creating windows.

Windows can have a titlebar, close button, move, and resizing capabilities.  Each window
is automatically part of its own [`render group`](struct.WidgetBuilder.html#method.new_render_group)
and will automatically come on top of other widgets when clicked on.  You can create a `WindowBuilder`
from a [`WidgetBuilder`](struct.WidgetBuilder.html) by calling [`window`](struct.WidgetBuilder.html#method.window)
after any calls to general purpose widget layout.

There is also a [`window method on Frame`](struct.Frame.html#method.window) as a convenience for simple cases.

Once you are finished setting up the window, you call [`children`](#method.children) to add children and add the widget
to the frame.

# Example
```
fn create_window(ui: &mut Frame, unique_id: &str) {
    ui.start("window")
    .window(unique_id)
    .title("My Window")
    .resizable(false)
    .children(|ui| {
        // window content here
    });
}
```

# Theme definition
An example of a theme definition for a window:

```yaml
  window:
    background: gui/window_bg
    wants_mouse: true
    layout: Vertical
    layout_spacing: [5, 5]
    border: { left: 5, right: 5, top: 35, bot: 5 }
    size: [300, 400]
    child_align: Top
    children:
      titlebar:
        wants_mouse: true
        background: gui/small_button
        size: [10, 30]
        pos: [-6, -36]
        border: { all: 5 }
        width_from: Parent
        child_align: Center
        align: TopLeft
        children:
          title:
            from: label
            text: "Main Window"
            font: medium
            width_from: Parent
          close:
            wants_mouse: true
            background: gui/small_button
            foreground: gui/close_icon
            size: [20, 20]
            border: { all: 4 }
            align: TopRight
      handle:
        wants_mouse: true
        background: gui/window_handle
        size: [12, 12]
        align: BotRight
        pos: [-1, -1]
```
*/
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

    /// Specifies that this window will not use a new render group.  This can
    /// be useful in some cases where you want to handle grouping yourself.
    /// See [`WidgetBuilder.new_render_group`](struct.WidgetBuilder.html#method.new_render_group)
    #[must_use]
    pub fn cancel_render_group(mut self) -> WindowBuilder<'a> {
        self.builder.set_next_render_group(crate::widget::NextRenderGroup::None);
        self
    }

    /// Specifies whether the created window should show a titlebar.
    #[must_use]
    pub fn with_titlebar(mut self, with_titlebar: bool) -> WindowBuilder<'a> {
        self.state.with_titlebar = with_titlebar;
        self
    }

    /// Specify a title to show in the window's titlebar, if it is present.  If the
    /// titlebar is not present, does nothing.  This will override any text set in the theme.
    #[must_use]
    pub fn title<T: Into<String>>(mut self, title: T) -> WindowBuilder<'a> {
        self.state.title = Some(title.into());
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
            (children)(ui);

            let drag_move = if state.with_titlebar {
                let result = ui.start("titlebar")
                .children(|ui| {
                    if let Some(title) = state.title.as_ref() {
                        ui.start("title").text(title).finish();
                    } else {
                        ui.start("title").finish();
                    }
                    
                    if state.with_close_button {
                        let clicked = ui.child("close").clicked;

                        if clicked {
                            ui.close(&id);
                        }
                    }
                });

                if state.moveable && result.pressed {
                    result.moved
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
                        state.resize = state.resize + result.moved;
                    });
                }
            }
        })
    }
}

struct WindowState {
    with_titlebar: bool,
    with_close_button: bool,
    moveable: bool,
    resizable: bool,
    title: Option<String>,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            with_titlebar: true,
            with_close_button: true,
            moveable: true,
            resizable: true,
            title: None,
        }
    }
}