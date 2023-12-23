use rustc_hash::FxHashMap;

use crate::theme_definition::CharacterRange;
use crate::render::{TexCoord, DrawList, FontHandle, DummyDrawList};
use crate::{Point, Rect, Align, Color};

pub struct FontSource {
    pub(crate) font: rusttype::Font<'static>,
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
            tex_coords: [TexCoord::new(0.0, 0.0), TexCoord::new(0.0, 0.0)],
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
    characters: FxHashMap<char, FontChar>,
    line_height: f32,
    ascent: f32,
}

impl Font {
    pub(crate) fn new(handle: FontHandle, characters: FxHashMap<char, FontChar>, line_height: f32, ascent: f32) -> Font {
        Font {
            handle,
            characters,
            line_height,
            ascent,
        }
    }

    fn char(&self, c: char) -> Option<&FontChar> {
        self.characters.get(&c)
    }

    pub fn line_height(&self) -> f32 { self.line_height }

    pub fn ascent(&self) -> f32 { self.ascent }

    pub fn handle(&self) -> FontHandle { self.handle }

    pub(crate) fn layout(
        &self,
        params: FontDrawParams,
        text: &str,
        cursor: &mut Point,
    ) {
        let mut draw_list = DummyDrawList::new();
        let mut renderer = FontRenderer::new(
            self,
            &mut draw_list,
            params,
            Rect::default(),
        );
        renderer.render(text);

        if text.is_empty() {
            // compute the cursor position for empty text
            renderer.adjust_line_x();
            renderer.size.y += 2.0 * renderer.font.line_height;
            renderer.adjust_all_y();
        }

        *cursor = renderer.pos;
    }

    pub(crate) fn draw<D: DrawList>(
        &self,
        draw_list: &mut D,
        params: FontDrawParams,
        text: &str,
        clip: Rect,
    ) {
        let mut renderer = FontRenderer::new(
            self,
            draw_list,
            params,
            clip
        );
        renderer.render(text);
    }
}

struct FontRenderer<'a,  D> {
    font: &'a Font,
    draw_list: &'a mut D,
    initial_index: usize,

    scale_factor: f32,
    clip: Rect,
    align: Align,
    color: Color,

    area_size: Point,
    initial_pos: Point,

    pos: Point,
    size: Point,
    cur_line_index: usize,

    cur_word: Vec<&'a FontChar>,
    cur_word_width: f32,

    is_first_line_with_indent: bool,
}

impl<'a, D: DrawList> FontRenderer<'a, D> {
    fn new(
        font: &'a Font,
        draw_list: &'a mut D,
        params: FontDrawParams,
        clip: Rect,
    ) -> FontRenderer<'a, D> {
        let initial_index = draw_list.len();

        FontRenderer {
            font,
            draw_list,
            initial_index,
            align: params.align,
            color: params.color,
            scale_factor: params.scale_factor,
            clip,
            area_size: params.area_size,
            initial_pos: params.pos,
            pos: Point::new(params.pos.x + params.indent, params.pos.y),
            size: Point::new(params.indent, 0.0),
            cur_line_index: initial_index,
            cur_word: Vec::new(),
            cur_word_width: 0.0,
            is_first_line_with_indent: params.indent > 0.0,
        }
    }

    fn render(&mut self, text: &str) {
        for c in text.chars() {
            let font_char = match self.font.char(c) {
                None => continue, // TODO draw a special character here?
                Some(char) => char,
            };

            if c == '\n' {
                self.draw_cur_word();
                self.next_line();
            } else if c.is_whitespace() {
                self.draw_cur_word();

                // don't draw whitespace at the start of a line
                if self.cur_line_index != self.draw_list.len() || self.is_first_line_with_indent {
                    self.pos.x += font_char.x_advance;
                    self.size.x += font_char.x_advance;
                }

                continue;
            }

            self.cur_word_width += font_char.x_advance;
            self.cur_word.push(font_char);

            if self.size.x + self.cur_word_width > self.area_size.x {
                //if the word was so long that we drew nothing at all
                if self.cur_line_index == self.draw_list.len() && !self.is_first_line_with_indent {
                    self.draw_cur_word();
                    self.next_line();
                } else {
                    self.next_line();
                    self.draw_cur_word();
                }
            }
        }

        self.draw_cur_word();

        if self.cur_line_index < self.draw_list.len() {    
            // adjust characters on the last line
            self.adjust_line_x();
            self.size.y += self.font.line_height;
        }

        self.adjust_all_y();
    }

    fn draw_cur_word(&mut self) {
        for font_char in self.cur_word.drain(..) {
            let x = (self.pos.x * self.scale_factor).round() / self.scale_factor;
            let y = ((self.pos.y + font_char.y_offset + self.font.ascent) * self.scale_factor).round() / self.scale_factor;

            self.draw_list.push_rect(
                [x, y],
                [font_char.size.x, font_char.size.y],
                font_char.tex_coords,
                self.color,
                self.clip,
            );
            self.pos.x += font_char.x_advance;
            self.size.x += font_char.x_advance;
        }
        self.cur_word_width = 0.0;
    }

    fn next_line(&mut self) {
        self.is_first_line_with_indent = false;
        self.pos.y += self.font.line_height;
        self.size.y += self.font.line_height;

        self.adjust_line_x();
        self.pos.x = self.initial_pos.x;
        self.cur_line_index = self.draw_list.len();
        self.size.x = 0.0;
    }

    fn adjust_all_y(&mut self) {
        use Align::*;
        let y_offset = match self.align {
            TopLeft =>  0.0,
            TopRight => 0.0,
            BotLeft =>  self.area_size.y - self.size.y,
            BotRight => self.area_size.y - self.size.y,
            Left =>     (self.area_size.y - self.size.y) / 2.0,
            Right =>    (self.area_size.y - self.size.y) / 2.0,
            Bot =>      self.area_size.y - self.size.y,
            Top =>      0.0,
            Center =>   (self.area_size.y - self.size.y) / 2.0,
        };

        self.pos.y += y_offset;
        self.draw_list.back_adjust_positions(
            self.initial_index,
            Point { x: 0.0, y: y_offset }
        );
    }

    fn adjust_line_x(&mut self) {
        use Align::*;
        let x_offset = match self.align {
            TopLeft =>  0.0,
            TopRight => self.area_size.x - self.size.x,
            BotLeft =>  0.0,
            BotRight => self.area_size.x - self.size.x,
            Left =>     0.0,
            Right =>    self.area_size.x - self.size.x,
            Bot =>      (self.area_size.x - self.size.x) / 2.0,
            Top =>      (self.area_size.x - self.size.x) / 2.0,
            Center =>   (self.area_size.x - self.size.x) / 2.0,
        };
    
        self.pos.x += x_offset;

        let x = (x_offset * self.scale_factor).round() / self.scale_factor;

        self.draw_list.back_adjust_positions(
            self.cur_line_index,
            Point { x, y: 0.0 }
        );
    }
}

pub(crate) struct FontTextureOut {
    pub font: Font,
    pub data: Vec<u8>,
    pub tex_width: u32,
    pub tex_height: u32,
}

pub(crate) struct FontTextureWriter<'a> {
    // current state
    tex_x: u32,
    tex_y: u32,
    max_row_height: u32,

    //input
    tex_width: u32,
    tex_height: u32,
    font: &'a rusttype::Font<'a>,
    font_scale: rusttype::Scale,
    
    //output
    data: Vec<u8>,
    characters: FxHashMap<char, FontChar>,
}

impl<'a> FontTextureWriter<'a> {
    pub fn new(font: &'a rusttype::Font<'a>, ranges: &[CharacterRange], size: f32, scale: f32) -> FontTextureWriter<'a> {
        // TODO if the approximation here doesn't work in practice, may need to do 2 passes over the font.
        // first pass would just determine the texture bounds.

        // count number of characters and size texture conservatively based on how much space the characters should need
        let count = ranges.iter().fold(0, |accum, range| accum + (range.upper - range.lower + 1));
        let rows = (count as f32).sqrt().ceil();
        const FUDGE_FACTOR: f32 = 1.2; // factor for characters with tails and wider than usual characters
        let tex_size = (rows * size * FUDGE_FACTOR * scale).ceil() as u32;
        log::info!("Using texture of size {} for {} characters in font of size {}.", tex_size, count, size * scale);

        let tex_width = tex_size;
        let tex_height = tex_size;

        let data = vec![0u8; (tex_width * tex_height) as usize];
        let font_scale = rusttype::Scale { x: size * scale, y: size * scale };

        FontTextureWriter {
            tex_x: 0,
            tex_y: 0,
            max_row_height: 0,
            tex_width,
            tex_height,
            font,
            font_scale,
            data,
            characters: FxHashMap::default(),
        }
    }

    pub fn write(mut self, handle: FontHandle, ranges: &[CharacterRange]) -> Result<FontTextureOut, crate::Error> {
        self.characters.insert('\n', FontChar::default());

        for range in ranges {
            for codepoint in range.lower..=range.upper {
                let c = match std::char::from_u32(codepoint) {
                    None => {
                        log::warn!("Character range {:?} contains invalid codepoint {}", range, codepoint);
                        break;
                    }, Some(c) => c,
                };

                let font_char = self.add_char(c);
                self.characters.insert(c, font_char);
            }
        }

        let v_metrics = self.font.v_metrics(self.font_scale);

        let font_out = Font::new(
            handle,
            self.characters,
            v_metrics.ascent - v_metrics.descent + v_metrics.line_gap,
            v_metrics.ascent,
        );

        Ok(FontTextureOut {
            font: font_out,
            data: self.data,
            tex_width: self.tex_width,
            tex_height: self.tex_height,
        })
    }
    
    fn add_char(
        &mut self,
        c: char,
    ) -> FontChar {
        let glyph = self.font.glyph(c)
            .scaled(self.font_scale)
            .positioned(rusttype::Point { x: 0.0, y: 0.0 });

        // compute the glyph size.  use a minimum size of (1,1) for spaces
        let y_offset = glyph.pixel_bounding_box().map_or(0.0, |bb| bb.min.y as f32);
        let bounding_box = glyph.pixel_bounding_box()
            .map_or((1, 1), |bb| (bb.width() as u32, bb.height() as u32));
        
        if self.tex_x + bounding_box.0 >= self.tex_width {
            // move to next row
            self.tex_x = 0;
            self.tex_y = self.tex_y + self.max_row_height + 1;
            self.max_row_height = 0;
        }

        assert!(bounding_box.0 + self.tex_x < self.tex_width);
        assert!(bounding_box.1 + self.tex_y < self.tex_height);

        self.max_row_height = self.max_row_height.max(bounding_box.1);

        glyph.draw(|x, y, val| {
            let index = (self.tex_x + x) + (self.tex_y + y) * self.tex_width;
            let value = (val * 255.0).round() as u8;
            self.data[index as usize] = value;
        });

        let tex_coords = [
            TexCoord::new(
                self.tex_x as f32 / self.tex_width as f32,
                self.tex_y as f32 / self.tex_height as f32
            ),
            TexCoord::new(
                (self.tex_x + bounding_box.0) as f32 / self.tex_width as f32,
                (self.tex_y + bounding_box.1) as f32 / self.tex_height as f32
            ),
        ];

        self.tex_x += bounding_box.0 + 1;

        FontChar {
            size: (bounding_box.0 as f32, bounding_box.1 as f32).into(),
            tex_coords,
            x_advance: glyph.unpositioned().h_metrics().advance_width,
            y_offset,
        }
    }
}

pub struct FontDrawParams {
    pub area_size: Point,
    pub pos: Point,
    pub indent: f32,
    pub align: Align,
    pub color: Color,
    pub scale_factor: f32,
}