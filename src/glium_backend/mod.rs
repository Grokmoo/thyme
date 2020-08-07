use std::rc::Rc;
use std::fmt::Display;
use std::error::Error;
use std::borrow::Cow;

use glium::{implement_vertex, uniform, DrawParameters, program::{ProgramCreationError}, Program, Surface};
use glium::backend::{Context, Facade};
use glium::uniforms::{MagnifySamplerFilter, MinifySamplerFilter, Sampler, SamplerBehavior, SamplerWrapFunction};
use glium::texture::{Texture2d, RawImage2d, SrgbTexture2d};
use glium::index::PrimitiveType;

use crate::{DrawData, DrawMode, Renderer, TextureHandle, TextureData, TexCoord, Vertex, Font, FontSource, FontHandle, FontChar};

const FONT_TEX_SIZE: u32 = 512;

pub struct GliumRenderer {
    context: Rc<Context>,
    base_program: Program,
    font_program: Program,
    textures: Vec<GliumTexture>,
    fonts: Vec<GliumFont>,
}

impl GliumRenderer {
    pub fn new<F: Facade>(facade: &F) -> Result<GliumRenderer, GliumError> {
        let context = Rc::clone(facade.get_context());

        let base_program = Program::from_source(
            facade,
            VERTEX_SHADER_SRC,
            FRAGMENT_SHADER_SRC,
            None,
        )?;

        let font_program = Program::from_source(
            facade,
            VERTEX_SHADER_SRC,
            FONT_FRAGMENT_SHADER_SRC,
            None
        )?;

        Ok(GliumRenderer {
            context,
            base_program,
            font_program,
            fonts: Vec::new(),
            textures: Vec::new(),
        })
    }

    fn font(&self, font: FontHandle) -> &GliumFont {
        &self.fonts[font.id()]
    }

    fn texture(&self, texture: TextureHandle) -> &GliumTexture {
        &self.textures[texture.id]
    }

    pub fn draw<T: Surface>(&mut self, target: &mut T, data: &DrawData) -> Result<(), GliumError> {
        let left = data.display_pos[0];
        let right = data.display_pos[0] + data.display_size[0];
        let top = data.display_pos[1];
        let bottom = data.display_pos[1] + data.display_size[1];

        let matrix = [
            [         (2.0 / (right - left)),                             0.0,  0.0, 0.0],
            [                            0.0,          (2.0 / (top - bottom)),  0.0, 0.0],
            [                            0.0,                             0.0, -1.0, 0.0],
            [(right + left) / (left - right), (top + bottom) / (bottom - top),  0.0, 1.0],
        ];

        for list in &data.draw_lists {
            let vertices = glium::VertexBuffer::immutable(&self.context, &list.vertices)?;
            let indices = glium::IndexBuffer::immutable(&self.context, PrimitiveType::TrianglesList, &list.indices)?;

            let params = DrawParameters {
                blend: glium::Blend::alpha_blending(),
                clip_planes_bitmask: 0b1111, //enable the first 4 clip planes
                ..DrawParameters::default()
            };

            match list.mode {
                DrawMode::Base(texture) => {
                    let texture = self.texture(texture);
                    let uniforms = uniform! {
                        tex: Sampler(&texture.texture, texture.sampler),
                        matrix: matrix,
                    };
                    target.draw(&vertices, &indices, &self.base_program, &uniforms, &params)?;
                }, DrawMode::Font(font) => {
                    let font = self.font(font);
                    let tex = Sampler(&font.texture, font.sampler);
                    let uniforms = uniform! {
                        tex: tex,
                        matrix: matrix,
                    };
                    target.draw(&vertices, &indices, &self.font_program, &uniforms, &params)?;
                }
            }            
        }

        Ok(())
    }
}

impl Renderer for GliumRenderer {
    fn register_texture(
        &mut self,
        handle: TextureHandle,
        image_data: &[u8],
        dimensions: (u32, u32),
    ) -> Result<TextureData, crate::Error> {
        let image = RawImage2d::from_raw_rgba_reversed(image_data, dimensions);
        let texture = SrgbTexture2d::new(&self.context, image).unwrap();

        let sampler = SamplerBehavior {
            minify_filter: MinifySamplerFilter::Linear,
            magnify_filter: MagnifySamplerFilter::Nearest,
            wrap_function: (
                SamplerWrapFunction::BorderClamp,
                SamplerWrapFunction::BorderClamp,
                SamplerWrapFunction::BorderClamp,
            ),
            ..Default::default()
        };

        assert!(handle.id == self.textures.len());
        self.textures.push(GliumTexture { texture, sampler });

        Ok(TextureData { handle, size: [dimensions.0, dimensions.1] })
    }

    fn register_font(
        &mut self,
        handle: FontHandle,
        source: &FontSource,
        size: f32,
    ) -> Result<Font, crate::Error> {
        let font = &source.font;

        // TODO size font texture appropriately
        let mut data = vec![0u8; (FONT_TEX_SIZE * FONT_TEX_SIZE) as usize];
        let font_scale = rusttype::Scale { x: size, y: size };

        let mut characters = Vec::new();
        let mut writer = FontTextureWriter {
            tex_x: 0,
            tex_y: 0,
            data: &mut data,
            font: &font,
            font_scale,
            max_row_height: 0,
        };

        for _ in 0..32 {
            characters.push(FontChar::default());
        }

        // write ASCII printable characters
        for i in 32..=126 {
            let font_char = writer.add_char(i);
            characters.push(font_char);
        }

        for _ in 127..161 {
            characters.push(FontChar::default());
        }

        for i in 161..=255 {
            let font_char = writer.add_char(i);
            characters.push(font_char);
        }

        let font_tex = Texture2d::with_format(
            &self.context,
            RawImage2d {
                data: Cow::Owned(data),
                width: FONT_TEX_SIZE,
                height: FONT_TEX_SIZE,
                format: glium::texture::ClientFormat::U8,
            },
            glium::texture::UncompressedFloatFormat::U8,
            glium::texture::MipmapsOption::NoMipmap,
        ).unwrap();

        let sampler = SamplerBehavior {
            minify_filter: MinifySamplerFilter::Nearest,
            magnify_filter: MagnifySamplerFilter::Nearest,
            wrap_function: (
                SamplerWrapFunction::BorderClamp,
                SamplerWrapFunction::BorderClamp,
                SamplerWrapFunction::BorderClamp,
            ),
            ..Default::default()
        };

        assert!(handle.id() == self.fonts.len());
        self.fonts.push(GliumFont {
            texture: font_tex,
            sampler,
        });

        let v_metrics = font.v_metrics(font_scale);

        let font_out = Font::new(
            handle,
            characters,
            v_metrics.ascent - v_metrics.descent + v_metrics.line_gap,
            v_metrics.ascent,
        );

        Ok(font_out)
    }
}

struct FontTextureWriter<'a> {
    tex_x: u32,
    tex_y: u32,
    data: &'a mut [u8],
    font: &'a rusttype::Font<'a>,
    font_scale: rusttype::Scale,
    max_row_height: u32,
}

impl<'a> FontTextureWriter<'a> {
    fn add_char(
        &mut self,
        i: usize,
    ) -> FontChar {
        let c: char = i as u8 as char;

        let glyph = self.font.glyph(c)
            .scaled(self.font_scale)
            .positioned(rusttype::Point { x: 0.0, y: 0.0 });

        // compute the glyph size.  use a minimum size of (1,1) for spaces
        let y_offset = glyph.pixel_bounding_box().map_or(0.0, |bb| bb.min.y as f32);
        let bounding_box = glyph.pixel_bounding_box()
            .map_or((1, 1), |bb| (bb.width() as u32, bb.height() as u32));
        
        if self.tex_x + bounding_box.0 >= FONT_TEX_SIZE {
            // move to next row
            self.tex_x = 0;
            self.tex_y = self.tex_y + self.max_row_height + 1;
            self.max_row_height = 0;
        }

        self.max_row_height = self.max_row_height.max(bounding_box.1);

        glyph.draw(|x, y, val| {
            let index = (self.tex_x + x) + (self.tex_y + y) * FONT_TEX_SIZE;
            let value = (val * 255.0).round() as u8;
            self.data[index as usize] = value;
        });

        let tex_coords = [
            TexCoord([
                self.tex_x as f32 / FONT_TEX_SIZE as f32,
                self.tex_y as f32 / FONT_TEX_SIZE as f32
            ]),
            TexCoord([
                (self.tex_x + bounding_box.0) as f32 / FONT_TEX_SIZE as f32,
                (self.tex_y + bounding_box.1) as f32 / FONT_TEX_SIZE as f32
            ]),
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
implement_vertex!(Vertex, position, tex_coords, color, clip_pos, clip_size);

struct GliumFont {
    texture: Texture2d,
    sampler: SamplerBehavior,
}

struct GliumTexture {
    texture: SrgbTexture2d,
    sampler: SamplerBehavior,
}

#[derive(Debug)]
pub enum GliumError {
    Draw(glium::DrawError),
    Index(glium::index::BufferCreationError),
    Font(String),
    InvalidTexture(TextureHandle),
    InvalidFont(FontHandle),
    Program(ProgramCreationError),
    Vertex(glium::vertex::BufferCreationError),
}

impl Display for GliumError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use self::GliumError::*;
        match self {
            Draw(e) => write!(f, "Error drawing to target: {}", e),
            Index(e) => write!(f, "Index buffer creation failed: {}", e),
            Font(e) => write!(f, "{}", e),
            InvalidTexture(handle) => write!(f, "Invalid texture: {:?}", handle),
            InvalidFont(handle) => write!(f, "Invalid font: {:?}", handle),
            Program(e) => write!(f, "Shader program creation failed: {}", e),
            Vertex(e) => write!(f, "Vertex buffer creation failed: {}", e),

        }
    }
}

impl Error for GliumError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        use self::GliumError::*;
        match self {
            Draw(e) => Some(e),
            InvalidTexture(_) => None,
            InvalidFont(_) => None,
            Font(_) => None,
            Index(e) => Some(e),
            Program(e) => Some(e),
            Vertex(e) => Some(e),
        }
    }
}

impl From<glium::index::BufferCreationError> for GliumError {
    fn from(e: glium::index::BufferCreationError) -> GliumError {
        GliumError::Index(e)
    }
}

impl From<glium::vertex::BufferCreationError> for GliumError {
    fn from(e: glium::vertex::BufferCreationError) -> GliumError {
        GliumError::Vertex(e)
    }
}

impl From<glium::DrawError> for GliumError {
    fn from(e: glium::DrawError) -> GliumError {
        GliumError::Draw(e)
    }
}

impl From<ProgramCreationError> for GliumError {
    fn from(e: ProgramCreationError) -> GliumError {
        GliumError::Program(e)
    }
}

const VERTEX_SHADER_SRC: &str = r#"
  #version 140

  in vec2 position;
  in vec2 tex_coords;
  in vec3 color;
  in vec2 clip_pos;
  in vec2 clip_size;

  out vec2 v_tex_coords;
  out vec3 v_color;

  uniform mat4 matrix;

  void main() {
    v_tex_coords = tex_coords;
    v_color = color;

    gl_ClipDistance[0] = position.x - clip_pos.x;
    gl_ClipDistance[1] = clip_pos.x + clip_size.x - position.x;
    gl_ClipDistance[2] = position.y - clip_pos.y;
    gl_ClipDistance[3] = clip_pos.y + clip_size.y - position.y;

    gl_Position = matrix * vec4(position, 0.0, 1.0);
  }
"#;

const FRAGMENT_SHADER_SRC: &str = r#"
  #version 140

  in vec2 v_tex_coords;
  in vec3 v_color;

  out vec4 color;

  uniform sampler2D tex;

  void main() {
    color = vec4(v_color, 1.0) * texture(tex, v_tex_coords);
  }
"#;

const FONT_FRAGMENT_SHADER_SRC: &str = r#"
    #version 140

    in vec2 v_tex_coords;
    in vec3 v_color;

    out vec4 color;

    uniform sampler2D tex;
    
    void main() {
        color = vec4(v_color, texture(tex, v_tex_coords).r);
    }
"#;