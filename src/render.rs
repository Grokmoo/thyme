use crate::{
    Align, Color, Point, Image, AnimState,
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
        render_if_present(
            themes.image(widget.background()),
            &mut draw_data,
            &mut cur_draw,
            widget.pos(),
            widget.size(),
            widget.anim_state()
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

    render_if_present(
        themes.image(widget.foreground()),
        draw_data,
        cur_draw,
        fg_pos,
        fg_size,
        widget.anim_state()
    );

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
                text
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
    )
}

fn render_if_present(
    image: Option<&Image>,
    draw_data: &mut DrawData,
    cur_draw: &mut Option<DrawList>,
    pos: Point,
    size: Point,
    anim_state: AnimState,
) {
    let image = match image {
        None => return,
        Some(image) => image,
    };

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
    );
}

#[derive(Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 2],
    pub tex_coords: [f32; 2],
    pub color: [f32; 3],
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

    pub fn push_quad(&mut self, ul: Vertex, lr: Vertex) {
        let idx = self.vertices.len() as u32;
        self.indices.extend_from_slice(&[idx, idx + 1, idx + 2, idx, idx + 2, idx + 3]);

        self.vertices.push(ul);
        self.vertices.push(Vertex {
            position: [ul.position[0], lr.position[1]],
            tex_coords: [ul.tex_coords[0], lr.tex_coords[1]],
            color: ul.color,
        });
        self.vertices.push(lr);
        self.vertices.push(Vertex {
            position: [lr.position[0], ul.position[1]],
            tex_coords: [lr.tex_coords[0], ul.tex_coords[1]],
            color: lr.color,
        });
    }
}

pub struct DrawData {
    pub display_size: [f32; 2],
    pub display_pos: [f32; 2],
    pub draw_lists: Vec<DrawList>,
}