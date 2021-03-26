use pulldown_cmark::{Event, Parser, Tag};

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
        let normal_font = self.custom_string(theme, "normal_font", String::new());
        let strong_font = self.custom_string(theme, "strong_font", String::new());
        let emphasis_font = self.custom_string(theme, "emphasis_font", String::new());

        let mut state = MarkdownState {
            line_height,
            normal_font,
            strong_font,
            emphasis_font,
            cur_font: FontMode::Normal,
            indent: 0.0,
            cursor: Point::default(),
        };

        let builder = self.start(theme);

        // Need to clone the text here to avoid ownership issues.  It should be possible
        // to find a way around this as we don't actually modify the text later
        let text = builder.widget().text().unwrap_or_default().to_string();

        let mut bounds = Rect::default();

        builder.trigger_layout_inner(&mut bounds).children(|ui| {
            let parser = Parser::new(&text);
            for event in parser {
                match event {
                    Event::Start(tag) => {
                        state.start_tag(tag);
                    },
                    Event::End(tag) => {
                        state.end_tag(tag);
                    },
                    Event::Text(text) => {
                        item(ui, &mut state, text.to_string());
                        state.update_cursor(ui);
                    },
                    Event::SoftBreak => {
                        state.new_line(1.0);
                        state.update_cursor(ui);
                    },
                    Event::HardBreak => {
                        state.new_line(2.0);
                        state.update_cursor(ui);
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
    state: &mut MarkdownState,
    text: String
) {
    let original_y = state.cursor.y;

    ui.start("item")
        .text(text)
        .text_indent(state.indent)
        .font(state.cur_font())
        .trigger_text_layout(&mut state.cursor)
        .finish();
    
    state.cursor.y += original_y;
}

struct MarkdownState {
    line_height: f32,
    cursor: Point,
    indent: f32,
    normal_font: String,
    strong_font: String,
    emphasis_font: String,

    cur_font: FontMode,
}

impl MarkdownState {
    fn start_tag(&mut self, tag: Tag) {
        match tag {
            Tag::Paragraph => {}
            Tag::Heading(_) => {}
            Tag::BlockQuote => {}
            Tag::CodeBlock(_) => {}
            Tag::List(_) => {}
            Tag::Item => {}
            Tag::FootnoteDefinition(_) => {}
            Tag::Table(_) => {}
            Tag::TableHead => {}
            Tag::TableRow => {}
            Tag::TableCell => {}
            Tag::Emphasis => self.cur_font = FontMode::Emphasis,
            Tag::Strong => self.cur_font = FontMode::Strong,
            Tag::Strikethrough => {}
            Tag::Link(_, _, _) => {}
            Tag::Image(_, _, _) => {}
        }
    }

    fn end_tag(&mut self, tag: Tag) {
        match tag {
            Tag::Paragraph => {}
            Tag::Heading(_) => {}
            Tag::BlockQuote => {}
            Tag::CodeBlock(_) => {}
            Tag::List(_) => {}
            Tag::Item => {}
            Tag::FootnoteDefinition(_) => {}
            Tag::Table(_) => {}
            Tag::TableHead => {}
            Tag::TableRow => {}
            Tag::TableCell => {}
            Tag::Emphasis => self.cur_font = FontMode::Normal,
            Tag::Strong => self.cur_font = FontMode::Normal,
            Tag::Strikethrough => {}
            Tag::Link(_, _, _) => {}
            Tag::Image(_, _, _) => {}
        }
    }

    fn new_line(&mut self, lines: f32) {
        self.cursor.x = 0.0;
        self.cursor.y += lines * self.line_height;
    }

    fn update_cursor(&mut self, ui: &mut Frame) {
        self.indent = self.cursor.x;
        ui.set_cursor(0.0, self.cursor.y);
    }

    fn cur_font(&self) -> &str {
        match self.cur_font {
            FontMode::Normal => &self.normal_font,
            FontMode::Strong => &self.strong_font,
            FontMode::Emphasis => &self.emphasis_font,
        }
    }
}

enum FontMode {
    Normal,
    Strong,
    Emphasis,
}