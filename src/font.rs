use crate::{Point, Align, TexCoord, DrawList, Color, Vertex};

pub struct FontSource {
    pub(crate) font: rusttype::Font<'static>,
}

#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, Hash)]
pub struct FontHandle {
    id: usize,
}

impl FontHandle {
    pub(crate) fn id(self) -> usize { self.id }

    pub(crate) fn next(self) -> FontHandle { FontHandle { id: self.id + 1 } }
}

pub struct FontChar {
    pub size: Point,
    pub(crate) tex_coords: [TexCoord; 2],
    pub x_advance: f32,
    pub y_offset: f32,
}

impl Default for FontChar {
    fn default() -> Self {
        FontChar {
            size: Point::default(),
            tex_coords: [TexCoord([0.0, 0.0]), TexCoord([0.0, 0.0])],
            x_advance: 0.0,
            y_offset: 0.0,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct FontSummary {
    pub handle: FontHandle,
    pub line_height: f32,
}

pub struct Font {
    handle: FontHandle,
    characters: Vec<FontChar>,
    line_height: f32,
    ascent: f32,
}

impl Font {
    pub(crate) fn new(handle: FontHandle, characters: Vec<FontChar>, line_height: f32, ascent: f32) -> Font {
        Font {
            handle,
            characters,
            line_height,
            ascent,
        }
    }

    fn char(&self, c: char) -> Option<&FontChar> {
        self.characters.get(c as usize) // TODO smarter lookup
    }

    pub fn line_height(&self) -> f32 { self.line_height }

    pub fn ascent(&self) -> f32 { self.ascent }

    pub fn handle(&self) -> FontHandle { self.handle }

    pub fn draw(
        &self,
        draw_list: &mut DrawList,
        area_size: Point,
        pos: [f32; 2],
        text: &str,
        align: Align,
        color: Color,
    ) {
        let mut y_pos = pos[1];
        let mut x_pos = pos[0];
        let mut width = 0.0;
        let mut height = 0.0;

        let initial_index = draw_list.vertices.len();
        let mut line_index = draw_list.vertices.len();
        let mut had_char_on_cur_line = false;

        for c in text.chars() {
            if c == '\n' {
                y_pos += self.line_height;
                height += self.line_height;
                x_pos = pos[0];

                adjust_line_x(draw_list, line_index, area_size.x, width, align);
                line_index = draw_list.vertices.len();
                width = 0.0;
                had_char_on_cur_line = false;

                continue;
            }

            had_char_on_cur_line = true;

            let font_char = match self.char(c) {
                None => continue, // TODO draw a special character here?
                Some(char) => char,
            };

            draw_list.push_quad(
                Vertex {
                    position: [x_pos, y_pos + font_char.y_offset + self.ascent],
                    tex_coords: font_char.tex_coords[0].into(),
                    color: color.into()
                }, Vertex {
                    position: [x_pos + font_char.size.x, y_pos + font_char.size.y + font_char.y_offset + self.ascent],
                    tex_coords: font_char.tex_coords[1].into(),
                    color: color.into()
                },
            );

            x_pos += font_char.x_advance;
            width += font_char.x_advance;
        }

        if had_char_on_cur_line {
            // adjust characters on the last line
            adjust_line_x(draw_list, line_index, area_size.x, width, align);
            height += self.line_height;
        }

        // adjust y coordinate based on text alignment for all lines
        use Align::*;
        let y_offset = match align {
            TopLeft =>  0.0,
            TopRight => 0.0,
            BotLeft =>  area_size.y - height,
            BotRight => area_size.y - height,
            Left =>     (area_size.y - height) / 2.0,
            Right =>    (area_size.y - height) / 2.0,
            Bot =>      area_size.y - height,
            Top =>      0.0,
            Center =>   (area_size.y - height) / 2.0,
        };

        for vert in draw_list.vertices.iter_mut().skip(initial_index) {
            vert.position[1] += y_offset;
        }
    }
}

fn adjust_line_x(
    draw_list: &mut DrawList,
    initial_index: usize,
    area_width: f32,
    width: f32,
    align: Align
) {
    use Align::*;
    let x_offset = match align {
        TopLeft =>  0.0,
        TopRight => area_width - width,
        BotLeft =>  0.0,
        BotRight => area_width - width,
        Left =>     0.0,
        Right =>    area_width - width,
        Bot =>      (area_width - width) / 2.0,
        Top =>      (area_width - width) / 2.0,
        Center =>   (area_width - width) / 2.0,
    };

    for vert in draw_list.vertices.iter_mut().skip(initial_index) {
        vert.position[0] += x_offset;
    }
}