use std::collections::{HashMap, HashSet};

use crate::theme_definition::{
    ThemeDefinition, ImageDefinition, ImageDefinitionKind, WidgetThemeDefinition, AnimState,
};
use crate::{
    Error, TextureData, TextureHandle, FontHandle, FontSource, TexCoord, DrawList,
    Vertex, Point, Border, Align, Layout, WidthRelative, HeightRelative, Renderer,
};

pub struct ThemeSet {
    fonts: HashMap<FontHandle, Font>,
    font_handles: HashMap<String, FontSummary>,
    images: HashMap<String, Image>,
    theme_handles: HashMap<String, WidgetThemeHandle>,
    themes: Vec<WidgetTheme>,
}

impl ThemeSet {
    pub(crate) fn new<R: Renderer>(
        definition: ThemeDefinition,
        textures: HashMap<String, TextureData>,
        font_sources: HashMap<String, FontSource>,
        renderer: &mut R,
    ) -> Result<ThemeSet, Error> {
        let mut font_handles = HashMap::new();
        let mut font_handle = FontHandle::default();
        let mut fonts = HashMap::new();
        for (font_id, font) in definition.fonts {
            let source = font_sources.get(&font.source).ok_or_else(||
                Error::Theme(format!("Unable to locate font handle {}", font.source))
            )?;

            let font = renderer.register_font(font_handle, source, font.size)?;
            font_handle.id += 1;

            let line_height = font.line_height;
            let handle = font.handle;
            fonts.insert(handle, font);
            font_handles.insert(font_id, FontSummary { handle, line_height });
        }

        let mut images = HashMap::new();
        for (set_id, set) in definition.image_sets {
            let mut images_in_set = HashMap::new();

            let texture = textures.get(&set.source).ok_or_else(||
                Error::Theme(format!("Unable to locate texture {}", set.source))
            )?;

            let mut animated_images:Vec<(String, ImageDefinition)> = Vec::new();

            // first parse all images without dependencies
            for (image_id, image_def) in set.images {
                match image_def.kind {
                    ImageDefinitionKind::Animated { .. } => animated_images.push((image_id, image_def)),
                    _ => {
                        let image = Image::new(&image_id, image_def, texture, &images_in_set)?;
                        images_in_set.insert(image_id, image);
                    }
                }
            }

            // now parse animated images
            for (id, image_def) in animated_images {
                let image = Image::new(&id, image_def, texture, &images_in_set)?;
                images_in_set.insert(id, image);
            }

            for (id, image) in images_in_set {
                images.insert(format!("{}/{}", set_id, id), image);
            }
        }

        // build the set of themes
        let mut theme_handles = HashMap::new();
        let mut themes = Vec::new();
        let mut handle_index = 0;
        for (theme_id, theme) in definition.widgets {
            WidgetTheme::new(
                "",
                None,
                theme_id, 
                &mut handle_index, 
                &mut theme_handles, 
                &mut themes, 
                theme, 
                &images,
                &font_handles,
            )?;
        }

        // recursively resolve all "from" theme references

        // we may need to loop several times in order to resolve nested references
        const MAX_ITERATIONS: i32 = 20;
        let mut iteration = 0;
        loop {
            if iteration == MAX_ITERATIONS {
                return Err(
                    Error::Theme(format!("Unable to resolve all from references after {} iterations.  \
                        This is most likely caused by a circular reference.", iteration))
                );
            }

            let to_ids: Vec<WidgetThemeHandle> = theme_handles.values().copied().collect();
            let mut found_new = false;

            for to_id in to_ids.iter() {
                let from_str = match &themes[to_id.id as usize].from {
                    None => continue,
                    Some(from_id) => from_id,
                };

                found_new = true;

                let from_id = *theme_handles.get(from_str).ok_or_else(|| {
                    Error::Theme(format!("Invalid from theme '{}' in '{}'", from_str, themes[to_id.id as usize].id))
                })?;

                // if the 'from' field has its own 'from', don't resolve
                // it yet.  we need the nested froms to resolve first
                // in order to populate all fields correctly
                if themes[from_id.id as usize].from.is_some() { continue; }

                // we are definitely going to resolve the from, so now remove it
                themes[to_id.id as usize].from.take();

                merge_from(
                    from_id,
                    *to_id,
                    &mut themes,
                    &mut handle_index,
                    &mut theme_handles,
                )
            }

            if !found_new { break; }
            iteration += 1;
        }

        Ok(ThemeSet {
            fonts,
            font_handles,
            images,
            theme_handles,
            themes,
        })
    }

    pub fn get(&self, handle: WidgetThemeHandle) -> &WidgetTheme {
        &self.themes[handle.id as usize]
    }

    pub fn child_set(&self, handle: WidgetThemeHandle) -> HashSet<WidgetThemeHandle> {
        self.themes[handle.id as usize].children.iter().copied().collect()
    }

    pub fn theme(&self, id: &str) -> Option<&WidgetTheme> {
        self.handle(id).and_then(|handle| Some(&self.themes[handle.id as usize]))
    }

    pub fn font(&self, handle: FontHandle) -> &Font {
        &self.fonts[&handle]
    }

    pub fn find_font(&self, id: Option<&str>) -> Option<FontSummary> {
        match id {
            None => None,
            Some(id) => {
                match self.font_handles.get(id) {
                    None => {
                        // TODO warn earlier and only once instead of on every frame
                        log::warn!("Invalid font when drawing: '{}'", id);
                        None
                    }, Some(font_sum) => {
                        Some(*font_sum)
                    }
                }
            }
        }
    }

    pub fn image(&self, id: Option<&str>) -> Option<&Image> {
        match id {
            None => None,
            Some(id) => {
                match self.images.get(id) {
                    None => {
                        // TODO warn earlier and only once instead of every frame like this will
                        log::warn!("Invalid image when drawing: '{}'", id);
                        None
                    }, Some(image) => Some(image),
                }
            }
        }
    }

    pub fn handle(&self, id: &str) -> Option<WidgetThemeHandle> {
        self.theme_handles.get(id).cloned()
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct WidgetThemeHandle {
    id: u64,
}

#[derive(Clone)]
pub struct WidgetTheme {
    from: Option<String>,
    pub full_id: String,

    pub id: String,
    pub parent_handle: Option<WidgetThemeHandle>,
    pub handle: WidgetThemeHandle,

    pub text: Option<String>,
    pub font: Option<FontSummary>,
    pub background: Option<String>,
    pub foreground: Option<String>,

    // all fields are options instead of using default so
    // we can detect when to override them
    pub wants_mouse: Option<bool>,
    pub text_align: Option<Align>,
    pub pos: Option<Point>,
    pub size: Option<Point>,
    pub width_from: Option<WidthRelative>,
    pub height_from: Option<HeightRelative>,
    pub border: Option<Border>,
    pub align: Option<Align>,
    pub child_align: Option<Align>,
    pub layout: Option<Layout>,
    pub layout_spacing: Option<Point>,
    pub children: Vec<WidgetThemeHandle>,
}

impl WidgetTheme {
    fn new(
        parent_id: &str,
        parent_handle: Option<WidgetThemeHandle>,
        id: String,
        handle_index: &mut u64,
        handles: &mut HashMap<String, WidgetThemeHandle>,
        themes: &mut Vec<WidgetTheme>,
        def: WidgetThemeDefinition,
        images: &HashMap<String, Image>,
        fonts: &HashMap<String, FontSummary>,
    ) -> Result<WidgetThemeHandle, Error> {
        if id.contains("/") {
            return Err(
                Error::Theme(format!("'{}' theme name invalid.  the '/' character is not allowed", id))
            );
        }

        // handle top level as a special case
        let parent_id = if parent_id.len() == 0 {
            id.to_string()
        } else {
            format!("{}/{}", parent_id, id)
        };

        let background = if let Some(bg) = def.background {
            images.get(&bg).ok_or_else(||
                Error::Theme(format!("Unable to locate image '{}' as background for widget '{}'", bg, parent_id))
            )?;
            Some(bg)
        } else {
            None
        };

        let foreground = if let Some(fg) = def.foreground {
            images.get(&fg).ok_or_else(||
                Error::Theme(format!("Unable to locate image '{}' as foreground for widget '{}'", fg, parent_id))
            )?;
            Some(fg)
        } else {
            None
        };

        let font = if let Some(font) = def.font {
            let font_handle = fonts.get(&font).ok_or_else(||
                Error::Theme(format!("Unable to locate font '{}' for widget '{}'", font, parent_id))
            )?;
            Some(*font_handle)
        } else {
            None
        };

        let handle = WidgetThemeHandle { id: *handle_index };
        *handle_index += 1;
        let theme = WidgetTheme {
            from: def.from,
            parent_handle,
            handle,
            id,
            full_id: parent_id.to_string(),
            text: def.text,
            font,
            background,
            foreground,
            wants_mouse: def.wants_mouse,
            text_align: def.text_align,
            pos: def.pos,
            size: def.size,
            width_from: def.width_from,
            height_from: def.height_from,
            align: def.align,
            child_align: def.child_align,
            border: def.border,
            layout: def.layout,
            layout_spacing: def.layout_spacing,
            children: Vec::new(),
        };

        themes.push(theme);

        let mut children = Vec::new();
        for (child_id, child_def) in def.children {
            let child = WidgetTheme::new(
                &parent_id,
                Some(handle),
                child_id,
                handle_index,
                handles,
                themes,
                child_def,
                images,
                fonts
            )?;
            children.push(child);
        }

        themes[handle.id as usize].children = children;

        handles.insert(parent_id, handle);

        Ok(handle)
    }
}

fn merge_from(
    from_id: WidgetThemeHandle,
    to_id: WidgetThemeHandle,
    themes: &mut Vec<WidgetTheme>,
    handle_index: &mut u64,
    theme_handles: &mut HashMap<String, WidgetThemeHandle>,
) {
    let from = themes[from_id.id as usize].clone();
    let from_children = from.children.clone();

    let to = &mut themes[to_id.id as usize];
    let to_children = to.children.clone();

    // preserve any as-yet unresolve child from refs
    to.from = from.from;

    if to.wants_mouse.is_none() { to.wants_mouse = from.wants_mouse; }
    if to.font.is_none() { to.font = from.font.clone(); }
    if to.background.is_none() { to.background = from.background.clone(); }
    if to.foreground.is_none() { to.foreground = from.foreground.clone(); }
    if to.text_align.is_none() { to.text_align = from.text_align; }
    if to.pos.is_none() { to.pos = from.pos; }
    if to.size.is_none() { to.size = from.size; }
    if to.width_from.is_none() { to.width_from = from.width_from; }
    if to.height_from.is_none() { to.height_from = from.height_from; }
    if to.border.is_none() { to.border = from.border; }
    if to.align.is_none() { to.align = from.align; }
    if to.child_align.is_none() { to.child_align = from.child_align; }
    if to.layout.is_none() { to.layout = from.layout; }
    if to.layout_spacing.is_none() { to.layout_spacing = from.layout_spacing; }

    for child_id in to_children.iter() {
        let mut merge = None;

        {
            let child = &themes[child_id.id as usize];
            
            for from_child_id in from_children.iter() {
                let from_child = &themes[from_child_id.id as usize];
                if from_child.id == child.id {
                    merge = Some(from_child_id);
                    break;
                }
            }
        }

        if let Some(from_id) = merge {
            merge_from(
                *from_id,
                *child_id,
                themes,
                handle_index,
                theme_handles,
            )
        }
    }

    for from_child_id in from_children.iter() {
        let mut found = false;

        {
            let from_child = &themes[from_child_id.id as usize];

            for to_child_id in to_children.iter() {
                let child = &themes[to_child_id.id as usize];
                if from_child.id == child.id {
                    found = true;
                    break;
                }
            }
        }

        if !found {
            add_children_recursive(
                *from_child_id,
                to_id,
                themes,
                handle_index,
                theme_handles,
            );
        }
    }
}

fn add_children_recursive(
    from_id: WidgetThemeHandle,
    to_id: WidgetThemeHandle,
    themes: &mut Vec<WidgetTheme>,
    handle_index: &mut u64,
    theme_handles: &mut HashMap<String, WidgetThemeHandle>,
) {
    let mut from = themes[from_id.id as usize].clone();

    let to = &mut themes[to_id.id as usize];
    let handle = WidgetThemeHandle { id: *handle_index };
    *handle_index += 1;

    let full_id = format!("{}/{}", to.full_id, from.id);

    from.full_id = full_id.to_string();
    from.handle = handle;
    from.parent_handle = Some(to_id);

    // take all the children out of our new theme and add them recursively
    // as new themes, rather than just making a shallow copy
    let from_children: Vec<_> = from.children.drain(..).collect();

    to.children.push(handle);
    themes.push(from);
    theme_handles.insert(full_id.clone(), handle);

    for from_child in from_children {
        {
            let from = &mut themes[from_child.id as usize];
            from.full_id = format!("{}/{}", full_id, from.id);
        }
        add_children_recursive(from_child, handle, themes, handle_index, theme_handles);
    }
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

    pub fn handle(&self) -> FontHandle { self.handle }

    pub fn draw(&self, draw_list: &mut DrawList, area_size: Point, pos: [f32; 2], text: &str, align: Align) {
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
                }, Vertex {
                    position: [x_pos + font_char.size.x, y_pos + font_char.size.y + font_char.y_offset + self.ascent],
                    tex_coords: font_char.tex_coords[1].into(),
                }
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

#[derive(Clone)]
enum ImageKind {
    Composed {
        tex_coords: [[TexCoord; 4]; 4],
        grid_size: [f32; 2],
    },
    Simple {
        tex_coords: [TexCoord; 2],
        fixed_size: Option<[f32; 2]>,
    },
    Animated {
        states: Vec<(AnimState, Image)>
    }
}

#[derive(Clone)]
pub struct Image {
    texture: TextureHandle,
    kind: ImageKind
}

impl Image {
    pub fn texture(&self) -> TextureHandle { self.texture }

    pub fn draw(&self, draw_list: &mut DrawList, pos: [f32; 2], size: [f32; 2], anim_state: AnimState) {
        match &self.kind {
            ImageKind::Composed { tex_coords, grid_size } => {
                self.draw_composed(draw_list, tex_coords, *grid_size, pos, size);
            },
            ImageKind::Simple { tex_coords, fixed_size } => {
                if let Some(size) = fixed_size {
                    self.draw_simple(draw_list, tex_coords, pos, *size);
                } else {
                    self.draw_simple(draw_list, tex_coords, pos, size);
                }
            },
            ImageKind::Animated { states } => {
                self.draw_animated(draw_list, pos, size, anim_state, states);
            }
        }
    }

    pub(crate) fn new(
        image_id: &str,
        def: ImageDefinition,
        texture: &TextureData,
        others: &HashMap<String, Image>
    )-> Result<Image, Error> {
        let kind = match def.kind {
            ImageDefinitionKind::Composed { grid_size, position} => {
                let mut tex_coords = [[TexCoord::default(); 4]; 4];
                for y in 0..4 {
                    for x in 0..4 {
                        let x_val = position[0] + x as u32 * grid_size[0];
                        let y_val = position[1] + y as u32 * grid_size[1];
                        tex_coords[x][y] = texture.tex_coord(x_val, y_val);
                    }
                }

                let grid_size = [grid_size[0] as f32, grid_size[1] as f32];
                ImageKind::Composed { tex_coords, grid_size }
            },
            ImageDefinitionKind::Simple { size, position, stretch } => {
                let tex1 = texture.tex_coord(position[0], position[1]);
                let tex2 = texture.tex_coord(position[0] + size[0], position[1] + size[1]);
                let fixed_size = if !stretch { Some([size[0] as f32, size[1] as f32]) } else { None };
                ImageKind::Simple { tex_coords: [tex1, tex2], fixed_size }
            },
            ImageDefinitionKind::Animated { states } => {
                let mut states_out: Vec<(AnimState, Image)> = Vec::new();
                for (state, id) in states {
                    let image = find_image_in_set(image_id, others, &id)?;
                    states_out.push((state, image));
                }

                ImageKind::Animated { states: states_out }
            }
        };

        Ok(Image {
            texture: texture.handle,
            kind
        })
    }

    fn draw_animated(
        &self,
        draw_list: &mut DrawList,
        pos: [f32; 2],
        size: [f32; 2],
        to_find: AnimState,
        states: &[(AnimState, Image)],
    ) {
        for (state, image) in states {
            if state == &to_find {
                image.draw(draw_list, pos, size, to_find);
                break;
            }
        }
    }

    fn draw_simple(
        &self,
        draw_list: &mut DrawList,
        tex: &[TexCoord; 2],
        pos: [f32; 2],
        size: [f32; 2],
    ) {
        draw_list.push_quad(
            Vertex {
                position: [pos[0], pos[1]],
                tex_coords: tex[0].into(),
            },
            Vertex {
                position: [pos[0] + size[0], pos[1] + size[1]],
                tex_coords: tex[1].into(),
            }
        );
    }

    fn draw_composed(
        &self,
        draw_list: &mut DrawList,
        tex: &[[TexCoord; 4]; 4],
        grid_size: [f32; 2],
        pos: [f32; 2],
        size: [f32; 2]
    ) {
        draw_list.push_quad(
            Vertex {
                position: pos,
                tex_coords: tex[0][0].into(),
            },
            Vertex {
                position: [pos[0] + grid_size[0], pos[1] + grid_size[1]],
                tex_coords: tex[1][1].into(),
            },
        );

        if size[0] > 2.0 * grid_size[0] {
            draw_list.push_quad(
                Vertex {
                    position: [pos[0] + grid_size[0], pos[1]],
                    tex_coords: tex[1][0].into(),
                },
                Vertex {
                    position: [pos[0] + size[0] - grid_size[0], pos[1] + grid_size[1]],
                    tex_coords: tex[2][1].into(),
                }
            );
        }

        draw_list.push_quad(
            Vertex {
                position: [pos[0] + size[0] - grid_size[0], pos[1]],
                tex_coords: tex[2][0].into(),
            },
            Vertex {
                position: [pos[0] + size[0], pos[1] + grid_size[1]],
                tex_coords: tex[3][1].into(),
            },
        );

        if size[1] > 2.0 * grid_size[1] {
            draw_list.push_quad(
                Vertex {
                    position: [pos[0], pos[1] + grid_size[1]],
                    tex_coords: tex[0][1].into(),
                },
                Vertex {
                    position: [pos[0] + grid_size[0], pos[1] + size[1] - grid_size[1]],
                    tex_coords: tex[1][2].into(),
                },
            );

            if size[0] > 2.0 * grid_size[0] {
                draw_list.push_quad(
                    Vertex {
                        position: [pos[0] + grid_size[0], pos[1] + grid_size[1]],
                        tex_coords: tex[1][1].into(),
                    },
                    Vertex {
                        position: [pos[0] + size[0] - grid_size[0], pos[1] + size[1] - grid_size[1]],
                        tex_coords: tex[2][2].into(),
                    },
                );
            }

            draw_list.push_quad(
                Vertex {
                    position: [pos[0] + size[0] - grid_size[0], pos[1] + grid_size[1]],
                    tex_coords: tex[2][1].into(),
                },
                Vertex {
                    position: [pos[0] + size[0], pos[1] + size[1] - grid_size[1]],
                    tex_coords: tex[3][2].into(),
                }
            );
        }

        draw_list.push_quad(
            Vertex {
                position: [pos[0], pos[1] + size[1] - grid_size[1]],
                tex_coords: tex[0][2].into(),
            },
            Vertex {
                position: [pos[0] + grid_size[0], pos[1] + size[1]],
                tex_coords: tex[1][3].into(),
            }
        );

        if size[0] > 2.0 * grid_size[0] {
            draw_list.push_quad(
                Vertex {
                    position: [pos[0] + grid_size[0], pos[1] + size[1] - grid_size[1]],
                    tex_coords: tex[1][2].into(),
                },
                Vertex {
                    position: [pos[0] + size[0] - grid_size[0], pos[1] + size[1]],
                    tex_coords: tex[2][3].into(),
                }
            );
        }

        draw_list.push_quad(
            Vertex {
                position: [pos[0] + size[0] - grid_size[0], pos[1] + size[1] - grid_size[1]],
                tex_coords: tex[2][2].into(),
            },
            Vertex {
                position: [pos[0] + size[0], pos[1] + size[1]],
                tex_coords: tex[3][3].into(),
            }
        );
    }
}

fn find_image_in_set(parent_id: &str, set: &HashMap<String, Image>, id: &str) -> Result<Image, Error> {
    match set.get(id) {
        None => {
            Err(
                Error::Theme(format!("Unable to find image '{}' referenced as sub image of '{}'", id, parent_id))
            )
        }, Some(image) => Ok(image.clone())
    }
}