use crate::{Frame, InputFieldKeyboard, Point, WidgetBuilder, WidgetState};

/**
A [`WidgetBuilder`](struct.WidgetBuilder.html) specifically for creating input fields.  See
[`simple_input_field`](struct.Frame.html#method.simple_input_field) for a basic overview.

This allows you to customize the behavior of the input field, such as whether or not you wish to
allow newlines to be input.  You can create a basic input field using
[`simple_input_field`](struct.Frame.html#method.simple_input_field) as a convenience.

# Example
```
fn create_input_field(ui: &mut Frame, unique_id: &str, out: &mut String) {
    let result = ui.start("input_field")
        .input_field(unique_id)
        .allow_newline(true)
        .finish();

    if let Some(result) = result {
        *out = ui.text_for(unique_id);
    }
}
```

# Theme definition
An example of a theme definition for an input field:

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
      size: [4, 16]
      background: gui/caret
```
*/
pub struct InputFieldBuilder<'a> {
    builder: WidgetBuilder<'a>,
    state: InputFieldBuilderState,
}

impl<'a> InputFieldBuilder<'a> {
    pub(crate) fn new(builder: WidgetBuilder<'a>) -> InputFieldBuilder<'a> {
        InputFieldBuilder {
            builder,
            state: InputFieldBuilderState::default(),
        }
    }

    /// Specifies whether the created input field should allow newlines to be entered
    #[must_use]
    pub fn with_allow_newlines(mut self, allow_newlines: bool) -> InputFieldBuilder<'a> {
        self.state.allow_newline = allow_newlines;
        self
    }

    /// Specifies the initial input field value
    #[must_use]
    pub fn with_initial_value(mut self, initial_value: Option<String>) -> InputFieldBuilder<'a> {
        self.state.initial_value = initial_value;
        self
    }

    /// Consumes the builder and adds a widget to the current frame.  The
    /// returned data includes the standard WidgetState information
    /// as well as the input field specifics.  See
    /// [`finish`](struct.WidgetBuilder.html#method.finish)
    pub fn finish(self) -> InputFieldResult {
        let mut output = InputFieldResult {
            cursor: Point::default(),
            keyboard: None,
            state: WidgetState::hidden(),
        };

        let (field_state, builder) = (self.state, self.builder);
        let id = builder.widget().id().to_string();

        builder.frame.modify(&id, |state| {
            let text = match state.text.as_mut() {
                Some(text) => text,
                None => {
                    state.text = Some(field_state.initial_value.unwrap_or_default());
                    state.text.as_mut().unwrap()
                }
            };

            if let Some(c) = state.characters.pop() {
                match c {
                    '\x08' => { text.pop(); }, // backspace
                    '\r' => {
                        if field_state.allow_newline {
                            output.keyboard = Some(InputFieldKeyboard::Char('\n'));
                            text.push('\n');
                        } else {
                            // do nothing on enter, user will receive this as a key event as well
                        }
                    },
                    _ => {
                        output.keyboard = Some(InputFieldKeyboard::Char(c));
                        text.push(c);
                    },
                }
            }

            if output.keyboard.is_none() && let Some(e) = state.key_events.pop() {
                output.keyboard = Some(InputFieldKeyboard::KeyEvent(e));
            }
        });
        let mut text_pos = Point::default();
        let mut text_lines = 0;

        let (ui, result) = builder
        .trigger_text_layout(&mut text_pos, &mut text_lines)
        .finish_with(Some(|ui: &mut Frame| {
            if ui.is_focus_keyboard(&id) {
                ui.start("caret").pos(text_pos.x, text_pos.y).finish();
            }
        }));

        output.cursor = text_pos;

        if result.clicked {
            ui.focus_keyboard(id);
        }

        output.state = result;
        output
    }
}

#[derive(Default)]
struct InputFieldBuilderState {
    allow_newline: bool,
    initial_value: Option<String>,
}

impl Frame {
    /**
    Creates a simple text input field.  The `id` that is passed in must be unique.
    The text input will grab keyboard focus when the user clicks on it, allowing
    the user to type text.  The return value will be `None` if no event occurred
    this frame, or will contain the character added or key event if an event did occur.
    Optionally, pass an initial_value which will set the field's text if it
    is not already set.  See [`input_field`](struct.Frame.html#method.input_field)
    if you wish to further customize the input field behavior.

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

    # Example
    ```
    fn select_name(ui: &mut Frame, name: &mut String) {
        if let Some(result) = ui.simple_input_field("input_field", "unique_id", None) {
            *name = ui.text_for("unique_id");
        }
    }
    ```
    */
    pub fn simple_input_field(&mut self, theme: &str, id: &str, initial_value: Option<String>) -> InputFieldResult {
        self.start(theme).input_field(id).with_initial_value(initial_value).finish()
    }
}

/// Result struct returned from the creation of an input field
#[derive(Debug)]
pub struct InputFieldResult {
    /// The current text cursor position for this input field
    pub cursor: Point,

    /// Any keyboard input for this input field this frame
    pub keyboard: Option<InputFieldKeyboard>,

    /// The widget state for this input field
    pub state: WidgetState,
}
