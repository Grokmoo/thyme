use pulldown_cmark::{Alignment, Event, Options, Parser, Tag};

use crate::{Frame, Rect, Point, Align, WidthRelative};

impl Frame {
    /**
    A text area widget that parses markdown text.  Child themes need to be defined for each font / size
    combination that you want to be able to render.  This normally includes at least normal text, strong text,
    emphasis text, strong emphasis text, and a few heading levels.  If, in your markdown, you make use of a
    combination that is not defined, the widget will log an error.

    The widget can currently handle a subset of common Markdown, including headings, strong / emphasis text, unordered
    and ordered lists, and tables with column alignments.

    Several parameters need to be specified for the widget to function properly, including `tab_width`, `column_width`, and
    a `list_bullet` character.  See the example below.  Note that the widget does not perform look-ahead to determine
    appropriate column widths - these are specified with the `column_width` parameter instead.

    An example theme definition:
    ```yaml
    text_area_item:
      font: small
      border: { width: 5 }
      text_align: TopLeft
      size_from: [Parent, FontLine]
    text_area:
      border: { all: 5 }
      size_from: [Parent, Children]
      custom:
        tab_width: 6.0
        column_width: 70.0
        list_bullet: "* "
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
        let builder = self.start(theme);

        let mut state = MarkdownState {
            line_height: 0.0,
            tab_width: builder.custom_float("tab_width", 4.0),
            list_bullet: builder.custom_string("list_bullet", "*".to_string()),
            column_width: builder.custom_float("column_width", 25.0),
            text_indent: 0.0,
            indent_level: 0.0,
            list_stack: Vec::new(),
            cursor: Point::default(),
            table_column: None,
            table_header: false,
            table: Vec::new(),
            font: FontMode::Normal,
            size: SizeMode::Paragraph,
            cur_theme: "paragraph_normal".to_string(),
            currently_at_new_line: true,
        };

        // Need to clone the text here to avoid ownership issues.  It should be possible
        // to find a way around this as we don't actually modify the text later
        let text = builder.widget().text().unwrap_or_default().to_string();

        let mut bounds = Rect::default();

        builder.trigger_layout_inner(&mut bounds).children(|ui| {
            let mut options = Options::empty();
            options.insert(Options::ENABLE_TABLES);
            let parser = Parser::new_ext(&text, options);

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
                        state.new_line(ui, 1.5);
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

    let mut builder = ui.start(state.cur_theme());

    if let Some(col) = state.table_column {
        let align = if state.table_header {
            Align::Top
        } else {
                match state.table.get(col as usize) {
                Some(Alignment::None) => Align::TopLeft,
                Some(Alignment::Left) => Align::TopLeft,
                Some(Alignment::Center) => Align::Top,
                Some(Alignment::Right) => Align::TopRight,
                None => Align::TopLeft,
            }
        };

        builder = builder
            .width_from(WidthRelative::Normal)
            .size(state.column_width, 0.0)
            .text_align(align);
    }

    let mut size = Rect::default();

    builder
        .text(text)
        .text_indent(state.text_indent)
        .trigger_layout(&mut size)
        .trigger_text_layout(&mut state.cursor)
        .finish();
    
    if state.currently_at_new_line {
        // if this is the first element in a new line, reset the line height
        state.line_height = size.size.y;
    } else {
        state.line_height = state.line_height.max(size.size.y);
    }
    
    state.cursor.y += original_y;
    state.update_cursor(ui);
    state.currently_at_new_line = false;
}

struct MarkdownState {
    // params read in at start
    line_height: f32,
    tab_width: f32,
    list_bullet: String,
    column_width: f32,

    // current state

    // cursor position where child widgets will be placed
    currently_at_new_line: bool,
    cursor: Point,
    table: Vec<Alignment>,
    table_header: bool,
    table_column: Option<u16>,

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
            Tag::Emphasis => self.set_font(self.font.push(FontMode::Emphasis)),
            Tag::Strong => self.set_font(self.font.push(FontMode::Strong)),
            Tag::Table(table) => {
                self.table = table;
            }
            Tag::TableHead => {
                self.table_column = Some(0);
                self.table_header = true;
                self.set_font(self.font.push(FontMode::Strong));
            }
            Tag::TableRow => {
                self.table_column = Some(0);
            },
            Tag::TableCell => {
                self.update_cursor(ui);
            },
            Tag::BlockQuote | Tag::CodeBlock(_) | Tag::FootnoteDefinition(_) | Tag::Strikethrough | Tag::Link(_, _, _) | Tag::Image(_, _, _) => {
                ui.log(log::Level::Warn, format!("Tag {:?} is unsupported", tag));
            }
        }
    }

    fn end_tag(&mut self, ui: &mut Frame, tag: Tag) {
        match tag {
            Tag::Paragraph => {
                self.new_line(ui, 1.5);
            },
            Tag::Heading(_) => {
                self.set_size(SizeMode::Paragraph);
                self.new_line(ui, 1.5);
            },
            
            Tag::List(_) => {
                self.indent_level -= 1.0;
                self.list_stack.pop();
                if self.list_stack.is_empty() {
                    // if we just did the end of the top level list
                    self.new_line(ui, 1.0);
                }
            },
            Tag::Item => {
                if !self.currently_at_new_line {
                    self.new_line(ui, 1.0);
                } else {
                    self.update_cursor(ui);
                }
            },
            Tag::Emphasis => self.set_font(self.font.remove(FontMode::Emphasis)),
            Tag::Strong => self.set_font(self.font.remove(FontMode::Strong)),
            Tag::Table(_) => {
                self.table.clear();
            }
            Tag::TableHead => {
                self.table_column = None;
                self.table_header = false;
                self.new_line(ui, 1.0);
                self.set_font(self.font.remove(FontMode::Strong));
            }
            Tag::TableRow => {
                self.table_column = None;
                self.new_line(ui, 1.0);
            }
            Tag::TableCell => {
                let col = self.table_column.get_or_insert(0);
                *col += 1;
            }
            Tag::BlockQuote | Tag::CodeBlock(_) | Tag::FootnoteDefinition(_) | Tag::Strikethrough | Tag::Link(_, _, _) | Tag::Image(_, _, _) => {
                ui.log(log::Level::Warn, format!("Tag {:?} is unsupported", tag));
            }
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
        if let Some(col) = self.table_column {
            self.text_indent = 0.0;
            ui.set_cursor(col as f32 * self.column_width, self.cursor.y);
        } else {
            self.text_indent = self.cursor.x;
            ui.set_cursor(self.indent_level * self.tab_width, self.cursor.y);
        }
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