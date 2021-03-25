use pulldown_cmark::{Event, Parser};

use crate::{Frame, Rect, Point};

impl Frame {
    /**
    A text area widget that parses markdown text.

    An example theme definition:
    ```yaml
    text_area:
      # the actual text is rendered on the children so you don't want to specify any font or
      # layout options here
      background: gui/window_bg_base
      border: { all: 5 }
      size: [0, 150]
      width_from: Parent
      custom_floats:
        line_height: 10.0
      children:
        item:
          from: label
      text: |
        This is multiline text in *YAML* with some
        basic markdown formatting.
    ```

    # Example
    ```
    fn create_text_box(ui: &mut Frame) {
        ui.text_area("text_area");
    }
    ```
    **/
    pub fn text_area(&mut self, theme: &str) {
        let line_height = self.custom_float(theme, "line_height", 10.0);

        let builder = self.start(theme);

        // Need to clone the text here to avoid ownership issues.  It should be possible
        // to find a way around this as we don't actually modify the text later
        let text = builder.widget().text().unwrap_or_default().to_string();

        let mut bounds = Rect::default();
        let mut cursor = Point::default();

        builder.trigger_layout_inner(&mut bounds).children(|ui| {
            let parser = Parser::new(&text);
            for event in parser {
                match event {
                    Event::Start(tag) => {}
                    Event::End(tag) => {}
                    Event::Text(text) => {
                        item(ui, &mut cursor, text.to_string());
                        ui.set_cursor(cursor.x, cursor.y);
                    },
                    Event::SoftBreak => {
                        cursor.x = 0.0;
                        cursor.y += line_height;
                        ui.set_cursor(cursor.x, cursor.y);
                    },
                    Event::HardBreak => {
                        cursor.x = 0.0;
                        cursor.y += 2.0 * line_height;
                        ui.set_cursor(cursor.x, cursor.y);
                    },
                    Event::Rule | Event::Code(_) | Event::Html(_) | Event::FootnoteReference(_) | Event::TaskListMarker(_) => {
                        ui.log(log::Level::Warn, format!("Tag {:?} event is unsupported", event));
                    }
                }
            }
        });
    }
}

fn item(
    ui: &mut Frame,
    cursor: &mut Point,
    text: String
) {
    ui.start("item").text(text).trigger_text_layout(cursor).finish();
}