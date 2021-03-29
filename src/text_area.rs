use pulldown_cmark::{Event, Parser, Tag};

use crate::{Frame, Rect, Point};

impl Frame {
    /**
    A text area widget that parses markdown text.  Child themes need to be defined for each font / size
    combination that you want to be able to render.

    An example theme definition:
    ```yaml
    text_area:
      background: gui/window_bg_base
      border: { all: 5 }
      size: [0, 150]
      width_from: Parent
      custom:
        line_height: 14.0
      children:
        paragraph_normal:
          from: text_area_item
          font: small
        paragraph_strong:
          from: text_area_item
          font: small_bold
        paragraph_emphasis:
          from: text_area_item
          font: small_italic
        paragraph_strong_emphasis:
          from: text_area_item
          font: small_bold_italic
        heading1_normal:
          from: text_area_item
          font: heading1
        heading2_normal:
          from: text_area_item
          font: heading2
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
        let mut state = MarkdownState {
            line_height: self.custom_float(theme, "line_height", 10.0),
            tab_width: self.custom_float(theme, "tab_width", 4.0),
            list_bullet: self.custom_string(theme, "list_bullet", "*".to_string()),
            text_indent: 0.0,
            indent_level: 0.0,
            list_stack: Vec::new(),
            cursor: Point::default(),
            font: FontMode::Normal,
            size: SizeMode::Paragraph,
            cur_theme: "paragraph_normal".to_string(),
            currently_at_new_line: true,
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
                        state.start_tag(ui, tag);
                    },
                    Event::End(tag) => {
                        state.end_tag(ui, tag);
                    },
                    Event::Text(text) => {
                        item(ui, &mut state, text.to_string());
                    },
                    Event::SoftBreak => {
                        state.new_line(ui, 1.0);
                    },
                    Event::HardBreak => {
                        state.new_line(ui, 2.0);
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

    ui.start(state.cur_theme())
        .text(text)
        .text_indent(state.text_indent)
        .trigger_text_layout(&mut state.cursor)
        .finish();
    
    state.cursor.y += original_y;
    state.update_cursor(ui);
    state.currently_at_new_line = false;
}

struct MarkdownState {
    // params read in at start
    line_height: f32,
    tab_width: f32,
    list_bullet: String,

    // current state

    // cursor position where child widgets will be placed
    currently_at_new_line: bool,
    cursor: Point,

    // text indent - additional x indent within a child widget
    // beyond what is specified by the cursor position
    text_indent: f32,
    
    // number of tabs we are currently indented
    indent_level: f32,


    list_stack: Vec<ListMode>,

    size: SizeMode,
    font: FontMode,

    cur_theme: String, // computed based on size and font
}

impl MarkdownState {
    fn start_tag(&mut self, ui: &mut Frame, tag: Tag) {
        match tag {
            Tag::Paragraph => self.size = SizeMode::Paragraph,
            Tag::Heading(level) => {
                self.set_size(match level {
                    1 => SizeMode::Heading1,
                    2 => SizeMode::Heading2,
                    3 => SizeMode::Heading3,
                    _ => SizeMode::Paragraph,
                });
            },
            Tag::BlockQuote => {}
            Tag::CodeBlock(_) => {}
            Tag::List(kind) => {
                self.indent_level += 1.0;
                self.list_stack.push(match kind {
                    None => ListMode::Unordered,
                    Some(num) => ListMode::Ordered(num as u16),
                });
                if !self.currently_at_new_line {
                    self.new_line(ui, 1.0);
                } else {
                    self.update_cursor(ui);
                }
            },
            Tag::Item => {
                match self.list_stack.last_mut() {
                    Some(ListMode::Unordered) => {
                        item(ui, self, self.list_bullet.to_string());
                    },
                    Some(ListMode::Ordered(num)) => {
                        let cur_num = *num;
                        *num += 1;
                        item(ui, self, format!("{}. ", cur_num));
                    },
                    None => panic!("List item but not currently in a list!"),
                };
            },
            Tag::FootnoteDefinition(_) => {}
            Tag::Table(_) => {}
            Tag::TableHead => {}
            Tag::TableRow => {}
            Tag::TableCell => {}
            Tag::Emphasis => self.set_font(self.font.push(FontMode::Emphasis)),
            Tag::Strong => self.set_font(self.font.push(FontMode::Strong)),
            Tag::Strikethrough => {}
            Tag::Link(_, _, _) => {}
            Tag::Image(_, _, _) => {}
        }
    }

    fn end_tag(&mut self, ui: &mut Frame, tag: Tag) {
        match tag {
            Tag::Paragraph => {
                self.new_line(ui, 2.0);
            },
            Tag::Heading(_) => {
                self.set_size(SizeMode::Paragraph);
                self.new_line(ui, 2.0);
            },
            Tag::BlockQuote => {}
            Tag::CodeBlock(_) => {}
            Tag::List(_) => {
                self.indent_level -= 1.0;
                self.list_stack.pop();
            },
            Tag::Item => {
                if !self.currently_at_new_line {
                    self.new_line(ui, 1.0);
                } else {
                    self.update_cursor(ui);
                }
            },
            Tag::FootnoteDefinition(_) => {}
            Tag::Table(_) => {}
            Tag::TableHead => {}
            Tag::TableRow => {}
            Tag::TableCell => {}
            Tag::Emphasis => self.set_font(self.font.remove(FontMode::Emphasis)),
            Tag::Strong => self.set_font(self.font.remove(FontMode::Strong)),
            Tag::Strikethrough => {}
            Tag::Link(_, _, _) => {}
            Tag::Image(_, _, _) => {}
        }
    }

    fn set_font(&mut self, font: FontMode) {
        self.font = font;
        self.recompute_theme();
    }

    fn set_size(&mut self, size: SizeMode) {
        self.size = size;
        self.recompute_theme();
    }

    fn recompute_theme(&mut self) {
        self.cur_theme = format!("{}_{}", self.size.theme(), self.font.theme());
    }

    fn new_line(&mut self, ui: &mut Frame, lines: f32) {
        self.currently_at_new_line = true;
        self.cursor.x = 0.0;
        self.text_indent = 0.0;
        self.cursor.y += lines * self.line_height;
        self.update_cursor(ui);
    }

    fn update_cursor(&mut self, ui: &mut Frame) {
        self.text_indent = self.cursor.x;
        ui.set_cursor(self.indent_level * self.tab_width, self.cursor.y);
    }

    fn cur_theme(&self) -> &str {
        &self.cur_theme
    }
}

#[derive(Copy, Clone)]
enum ListMode {
    Unordered,
    Ordered(u16),
}

#[derive(Copy, Clone)]
enum SizeMode {
    Paragraph,
    Heading1,
    Heading2,
    Heading3,
}

impl SizeMode {
    fn theme(self) -> &'static str {
        use SizeMode::*;
        match self {
            Paragraph => "paragraph",
            Heading1 => "heading1",
            Heading2 => "heading2",
            Heading3 => "heading3",
        }
    }
}

#[derive(Copy, Clone)]
enum FontMode {
    Normal,
    Strong,
    Emphasis,
    StrongEmphasis,
}

impl FontMode {
    fn theme(self) -> &'static str {
        use FontMode::*;
        match self {
            Normal => "normal",
            Strong => "strong",
            Emphasis => "emphasis",
            StrongEmphasis => "strong_emphasis",
        }
    }

    fn push(self, other: FontMode) -> FontMode {
        use FontMode::*;
        match self {
            Normal => other,
            Strong => match other {
                Normal => Strong,
                Strong => Strong,
                Emphasis => StrongEmphasis,
                StrongEmphasis => StrongEmphasis,
            },
            Emphasis => match other {
                Normal => Emphasis,
                Strong => StrongEmphasis,
                Emphasis => Emphasis,
                StrongEmphasis => StrongEmphasis,
            },
            StrongEmphasis => StrongEmphasis,
        }
    }

    fn remove(self, other: FontMode) -> FontMode {
        use FontMode::*;
        match self {
            Normal => Normal,
            Strong => match other {
                Normal => Strong,
                Strong => Normal,
                Emphasis => Strong,
                StrongEmphasis => Normal,
            },
            Emphasis => match other {
                Normal => Emphasis,
                Strong => Emphasis,
                Emphasis => Normal,
                StrongEmphasis => Normal,
            },
            StrongEmphasis => match other {
                Normal => StrongEmphasis,
                Strong => Emphasis,
                Emphasis => Strong,
                StrongEmphasis => Normal,
            }
        }
    }
}