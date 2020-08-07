use crate::{
    Align, Color, Point, Image, AnimState, Clip, TexCoord,
    ThemeSet, Font, FontHandle, TextureHandle, Widget
};

pub(crate) fn render(
    themes: &ThemeSet,
    widgets: Vec<Widget>,
    display_size: Point,
) -> DrawData {
    let mut draw_data = DrawData {
        display_size: display_size.into(),
        display_pos: [0.0, 0.0],
        draw_lists: Vec::new(),
    };

    let mut cur_draw: Option<DrawList> = None;

    // render backgrounds
    for widget in &widgets {
        if widget.hidden() { continue; }
        let image_handle = match widget.background() {
            None => continue,
            Some(handle) => handle,
        };

        render_image(
            themes.image(image_handle),
            &mut draw_data,
            &mut cur_draw,
            widget.pos(),
            widget.size(),
            widget.anim_state(),
            widget.clip(),
        );
    }

    for widget in &widgets {
        if widget.hidden() { continue; }
        render_widget_foreground(themes, &mut draw_data, &mut cur_draw, widget);
    }
    
    if let Some(draw) = cur_draw {
        draw_data.draw_lists.push(draw);
    }

    draw_data
}

fn render_widget_foreground(
    themes: &ThemeSet,
    draw_data: &mut DrawData,
    cur_draw: &mut Option<DrawList>,
    widget: &Widget,
) {
    let border = widget.border();
    let fg_pos = widget.pos() + border.tl();
    let fg_size = widget.inner_size();

    if let Some(handle) = widget.foreground() {
        render_image(
            themes.image(handle),
            draw_data,
            cur_draw,
            fg_pos,
            fg_size,
            widget.anim_state(),
            widget.clip(),
        );
    }

    if let Some(text) = widget.text() {
        if let Some(font_summary) = widget.font() {
            render_text(
                themes.font(font_summary.handle),
                widget.text_align(),
                widget.text_color(),
                fg_size,
                draw_data,
                cur_draw,
                fg_pos,
                text,
                widget.clip(),
            )
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_text(
    font: &Font,
    align: Align,
    color: Color,
    area_size: Point,
    draw_data: &mut DrawData,
    cur_draw: &mut Option<DrawList>,
    pos: Point,
    text: &str,
    clip: Clip,
) {
    let create_draw = match cur_draw {
        None => true,
        Some(draw) => draw.mode != DrawMode::Font(font.handle()),
    };

    if create_draw {
        if let Some(draw) = cur_draw.take() {
            draw_data.draw_lists.push(draw);
        }

        *cur_draw = Some(DrawList::font(font.handle()));
    }

    font.draw(
        cur_draw.as_mut().unwrap(),
        area_size,
        pos.into(),
        text,
        align,
        color,
        clip,
    )
}

fn render_image(
    image: &Image,
    draw_data: &mut DrawData,
    cur_draw: &mut Option<DrawList>,
    pos: Point,
    size: Point,
    anim_state: AnimState,
    clip: Clip,
) {
    let create_draw = match cur_draw {
        None => true,
        Some(draw) => draw.mode != DrawMode::Base(image.texture()),
    };

    if create_draw {
        if let Some(draw) = cur_draw.take() {
            draw_data.draw_lists.push(draw);
        }

        *cur_draw = Some(DrawList::new(image.texture()));
    }

    image.draw(
        cur_draw.as_mut().unwrap(),
        pos.into(),
        size.into(),
        anim_state,
        clip,
    );
}

#[derive(Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 2],
    pub tex_coords: [f32; 2],
    pub clip_pos: [f32; 2],
    pub clip_size: [f32; 2],
    pub color: [f32; 3],
}

impl Vertex {
    pub fn new(position: [f32; 2], tex_coords: [f32; 2], color: Color, clip: Clip) -> Vertex {
        Vertex {
            position,
            tex_coords,
            color: color.into(),
            clip_pos: clip.pos.into(),
            clip_size: clip.size.into(),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum DrawMode {
    Base(TextureHandle),
    Font(FontHandle),
}

pub struct DrawList {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub mode: DrawMode,
}

impl DrawList {
    pub fn new(texture: TextureHandle) -> DrawList {
        DrawList {
            vertices: Vec::new(),
            indices: Vec::new(),
            mode: DrawMode::Base(texture),
        }
    }

    pub fn font(font: FontHandle) -> DrawList {
        DrawList {
            vertices: Vec::new(),
            indices: Vec::new(),
            mode: DrawMode::Font(font),
        }
    }

    pub(crate) fn push_quad_components(
        &mut self,
        p0: [f32; 2],
        p1: [f32; 2],
        tex: [TexCoord; 2],
        color: Color,
        clip: Clip,
    ) {
        let ul = Vertex::new(p0, tex[0].into(), color, clip);
        let lr = Vertex::new(p1, tex[1].into(), color, clip);
        self.push_quad(ul, lr);
    }

    fn push_quad(&mut self, ul: Vertex, lr: Vertex) {
        let idx = self.vertices.len() as u32;
        self.indices.extend_from_slice(&[idx, idx + 1, idx + 2, idx, idx + 2, idx + 3]);

        self.vertices.push(ul);
        self.vertices.push(Vertex {
            position: [ul.position[0], lr.position[1]],
            tex_coords: [ul.tex_coords[0], lr.tex_coords[1]],
            clip_pos: ul.clip_pos,
            clip_size: ul.clip_size,
            color: ul.color,
        });
        self.vertices.push(lr);
        self.vertices.push(Vertex {
            position: [lr.position[0], ul.position[1]],
            tex_coords: [lr.tex_coords[0], ul.tex_coords[1]],
            clip_pos: lr.clip_pos,
            clip_size: lr.clip_size,
            color: lr.color,
        });
    }
}

pub struct DrawData {
    pub display_size: [f32; 2],
    pub display_pos: [f32; 2],
    pub draw_lists: Vec<DrawList>,
}