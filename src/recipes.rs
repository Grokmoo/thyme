use std::fmt::Display;

use crate::{Align, Frame, Point, Rect, WidgetState};

/// Specific widget builders and convenience methods
impl Frame {
    /// The simplest way to construct a child widget.  The widget has no special behavior in code at all.
    /// It is defined entirely based on its `theme`.
    pub fn child(&mut self, theme: &str) -> WidgetState {
        self.start(theme).finish()
    }

    /**
    A simple label displaying the specified `text`, with no user interactivity.

    An example theme definition:
    ```yaml
    label:
      font: small
      border: { width: 5 }
      text_align: Center
      height_from: FontLine
    ```
    **/
    pub fn label<T: Into<String>>(&mut self, theme: &str, text: T) {
        self.start(theme).text(text).finish();
    }

    /**
    A simple button with a text `label`.

    An example theme definition:
    ```yaml
    button:
      font: small
      wants_mouse: true
      background: gui/small_button
      text_align: Center
      size: [150, 24]
      border: { all: 5 }
    ```
    **/
    pub fn button<T: Into<String>>(&mut self, theme: &str, label: T) -> WidgetState {
        self.start(theme).text(label).wants_mouse(true).finish()
    }

    /**
    A simple vertical slider.  The slider button can be dragged by the user.  The position
    of the button is based on the relative distance of `value` from `min` and `max`.
    Returns the new value if the user moved the slider on this frame, None, otherwise.  Will
    always return a value within [`min`, `max`] inclusive.  `max` must be greater than `min`.

    An example theme definition:
    ```yaml
    vertical_slider:
      size: [15, 0]
      height_from: Parent
      border: { top: 6, bot: 5, left: 5, right: 5 }
      children:
        slider_bar:
          align: TopLeft
          width_from: Parent
          height_from: Parent
          background: gui/slider_vertical
        slider_button:
          from: button
          size: [15, 15]
    ```
    **/
    pub fn vertical_slider(&mut self, theme: &str, min: f32, max: f32, value: f32) -> Option<f32> {
        let mut inner = Rect::default();
        let mut new_value = None;

        self.start(theme)
        .wants_mouse(true)
        .trigger_layout_inner(&mut inner)
        .children(|ui| {
            ui.child("slider_bar");

            let mut button_rect = Rect::default();
            let builder = ui.start("slider_button").wants_mouse(true).align(Align::Left).trigger_layout(&mut button_rect);

            let total_height = inner.size.y - button_rect.size.y;
            let pos = total_height * (value - min) / (max - min);

            let result = builder.pos(0.0, pos).finish();

            if result.moved.y != 0.0 {
                let delta_y = result.moved.y;

                let next_pos = pos + delta_y;
                let new_val = (max - min) * next_pos / total_height + min;

                new_value = Some(new_val.min(max).max(min));
            }
        });

        new_value
    }

    /**
    A simple horizontal slider.  The slider button can be dragged by the user.  The position
    of the button is based on the relative distance of `value` from `min` and `max`.
    Returns the new value if the user moved the slider on this frame, None, otherwise.  Will
    always return a value within [`min`, `max`] inclusive.  `max` must be greater than `min`.

    An example theme definition:
    ```yaml
    horizontal_slider:
      size: [0, 15]
      width_from: Parent
      border: { top: 6, bot: 5, left: 5, right: 5 }
      children:
        slider_bar:
          align: TopLeft
          width_from: Parent
          height_from: Parent
          background: gui/slider_horizontal
        slider_button:
          from: button
          size: [15, 15]
    ```
    **/
    pub fn horizontal_slider(&mut self, theme: &str, min: f32, max: f32, value: f32) -> Option<f32> {
        let mut inner = Rect::default();
        let mut new_value = None;

        self.start(theme)
        .wants_mouse(true)
        .trigger_layout_inner(&mut inner)
        .children(|ui| {
            ui.child("slider_bar");

            let mut button_rect = Rect::default();
            let builder = ui.start("slider_button").wants_mouse(true).align(Align::Left).trigger_layout(&mut button_rect);

            let total_width = inner.size.x - button_rect.size.x;
            let pos = total_width * (value - min) / (max - min);

            let result = builder.pos(pos, 0.0).finish();

            if result.moved.x != 0.0 {
                let delta_x = result.moved.x;

                let next_pos = pos + delta_x;
                let new_val = (max - min) * next_pos / total_width + min;

                new_value = Some(new_val.min(max).max(min));
            }
        });

        new_value
    }

    // TODO tree

    // TODO menubar

    /**
    A drop down box. It displays its currently active selection (`current`), and opens a modal popup to select a new
    choice from the list of `values` when the user clicks on it.  The specified `id` must be unique.
    The method will return a selected choice on the frame the user clicks on it, otherwise returning `None`.

    An example theme definition;  See [`ScrollpaneBuilder`](struct.ScrollpaneBuilder.html) for the scrollpane example.
    ```yaml
    combo_box:
      from: button
    combo_box_popup:
      from: scrollpane
      width_from: Parent
      height_from: Normal
      size: [0, 75]
      background: gui/small_button_normal
      children:
        content:
          size: [-15, 0]
          children:
            entry:
              from: button
              width_from: Parent
              size: [0, 25]
        scrollbar_vertical:
          size: [20, 30]
    ```
    **/
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

        self.start(&format!("{}_popup", id))
        .id(&popup_id)
        .screen_pos(rect.pos.x, rect.pos.y + rect.size.y)
        .initially_open(false)
        .unclip()
        .new_render_group()
        .scrollpane("cb_popup_content")
        .children(|ui| {
            for value in values {
                if ui.button("entry", value.to_string()).clicked {
                    result = Some(value);
                    ui.close(&popup_id);
                }
            }
        });

        result
    }

    /// A simple toggle button that can be toggle on or off, based on the passed in `active` state.
    ///
    /// See [`button`](#method.button) for a YAML example.
    pub fn toggle_button<T: Into<String>>(&mut self, theme: &str, label: T, active: bool) -> WidgetState {
        self.start(theme).text(label).active(active).wants_mouse(true).finish()
    }

    /**
    Creates a simple text input field.  The `id` that is passed in must be unique.
    The text input will grab keyboard focus when the user clicks on it, allowing
    the user to type text.  The return value will be `None` if the text didn't change
    this frame, or will contain the current text displayed by the textbox if it did
    change.

    An example YAML theme definition:
    ```yaml
    input_field:
      font: small
      border: { height: 4, width: 5 }
      background: gui/input_field
      text_align: Left
      wants_mouse: true
      size: [150, 24]
      child_align: TopLeft
      children:
        caret:
          size: [2, -2]
          height_from: Parent
          background: gui/caret
    ```
    **/
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

    /**
    Creates a simple progress bar.  The drawing will be clipped based on the size
    of the widget and the passed in `frac`.

    An example YAML theme definition:
    ```yaml
    progress_bar:
      size: [100, 24]
      background: gui/button
      border: { width: 27 }
      child_align: TopLeft
      children:
        bar:
          background: gui/progress_bar
          width_from: Parent
          height_from: Parent
    ```
    **/
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

    /**
    Creates a simple tooltip with the specified text.  The tooltip is placed based on the
    position of the mouse.

    An example YAML theme definition:
    ```yaml
    tooltip:
      background: gui/button
      font: small
      text_align: Center
    ```
    **/
    pub fn tooltip<T: Into<String>>(&mut self, theme: &str, label: T) {
        let mouse = self.mouse_rect();

        self.start(theme)
        .unclip()
        .unparent()
        .text(label)
        .screen_pos(mouse.right(), mouse.bot())
        .new_render_group()
        .finish();
    }

    /// A convenience method to create a window with the specified `theme`.  The `theme` is also
    /// used for the window ID, which must be unique in your application. If this is not the case,
    /// you should use the full [`WindowBuilder`](struct.WindowBuilder.html) form.
    /// The specified closure is called to add `children` to the window.
    /// The window will include a titlebar, close button, be moveable, and resizable.
    /// See [`WindowBuilder`](struct.WindowBuilder.html) for more details and more
    /// flexible window creation. 
    pub fn window<F: FnOnce(&mut Frame)>(&mut self, theme: &str, children: F) {
        self
        .start(theme)
        .window(theme)
        .children(|ui| {
            (children)(ui);
        });
    }

    /// A convenience method to create a scrollpane with the specified `theme` and `content_id`, which must
    /// be unique.  See [`ScrollpaneBuilder`](struct.ScrollpaneBuilder.html) for more details and more
    /// flexible scrollpane creation.
    pub fn scrollpane<F: FnOnce(&mut Frame)>(&mut self, theme: &str, content_id: &str, children: F) {
        self.start(theme).scrollpane(content_id).children(children);
    }
}