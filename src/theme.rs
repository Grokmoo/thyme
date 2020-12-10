use std::collections::{HashMap};

use crate::theme_definition::{
    ThemeDefinition, ImageDefinition, ImageDefinitionKind, WidgetThemeDefinition,
};
use crate::font::{Font, FontSummary, FontSource};
use crate::image::{Image, ImageHandle};
use crate::render::{TextureData, Renderer, FontHandle};
use crate::theme_definition::CharacterRange;
use crate::{Color, Error, Point, Border, Align, Layout, WidthRelative, HeightRelative};

pub struct ThemeSet {
    fonts: Vec<Font>,
    font_handles: HashMap<String, FontSummary>,

    images: Vec<Image>,
    image_handles: HashMap<String, ImageHandle>,

    theme_handles: HashMap<String, WidgetThemeHandle>,
    themes: Vec<WidgetTheme>,
}

impl ThemeSet {
    pub(crate) fn new<R: Renderer>(
        definition: &ThemeDefinition,
        textures: HashMap<String, TextureData>,
        font_sources: HashMap<String, FontSource>,
        renderer: &mut R,
        display_scale: f32,
    ) -> Result<ThemeSet, Error> {
        let default_font_ranges = vec![
            CharacterRange { lower: 32, upper: 126 },
            CharacterRange { lower: 161, upper: 255 },
        ];

        // TODO need to be able to rebuild fonts when scale factor changes
        // FontSummary size will stay the same for this
        let mut font_handles = HashMap::new();
        let mut font_handle = FontHandle::default();
        let mut fonts = Vec::new();
        for (font_id, font) in &definition.fonts {
            let source = font_sources.get(&font.source).ok_or_else(||
                Error::Theme(format!("Unable to locate font handle {}", font.source))
            )?;

            let ranges = if font.characters.is_empty() {
                &default_font_ranges
            } else {
                &font.characters
            };

            let font = renderer.register_font(
                font_handle,
                source,
                ranges,
                font.size,
                display_scale
            )?;

            font_handle = font_handle.next();

            let line_height = font.line_height() / display_scale;
            let handle = font.handle();
            assert!(handle.id() == fonts.len());
            fonts.push(font);
            font_handles.insert(font_id.to_string(), FontSummary { handle, line_height });
        }

        let mut aliases: Vec<(String, String)> = Vec::new();

        let mut images = HashMap::new();
        for (set_id, set) in &definition.image_sets {
            let mut images_in_set = HashMap::new();

            let texture = if let Some(source) = set.source.as_ref() {
                textures.get(source).ok_or_else(||
                    Error::Theme(format!("Unable to locate texture {}", source))
                )?
            } else {
                &textures[crate::resource::INTERNAL_SINGLE_PIX_IMAGE_ID]
            };

            let mut collected_images: Vec<(&str, &ImageDefinition)> = Vec::new();
            let mut timed_images: Vec<(&str, &ImageDefinition)> = Vec::new();
            let mut animated_images: Vec<(&str, &ImageDefinition)> = Vec::new();

            // first parse all images without dependencies
            for (image_id, image_def) in &set.images {
                match &image_def.kind {
                    ImageDefinitionKind::Animated { .. } => animated_images.push((image_id, image_def)),
                    ImageDefinitionKind::Timed { .. } => timed_images.push((image_id, image_def)),
                    ImageDefinitionKind::Collected { .. } => collected_images.push((image_id, image_def)),
                    ImageDefinitionKind::Alias { from } => {
                        let to = format!("{}/{}", set_id, image_id);
                        let from = format!("{}/{}", set_id, from);
                        aliases.push((to, from));
                    },
                    ImageDefinitionKind::Group { group_scale, fill, images } => {
                        for (generated_id, xywh) in images {
                            let generated_def = ImageDefinition {
                                color: image_def.color,
                                kind: ImageDefinitionKind::Simple {
                                    position: [xywh[0] * group_scale[0], xywh[1] * group_scale[1]],
                                    size: [xywh[2] * group_scale[0], xywh[3] * group_scale[1]],
                                    fill: *fill,
                                }
                            };
                            let image = Image::new(generated_id, &generated_def, texture, &images_in_set, set.scale)?;
                            images_in_set.insert(generated_id.to_string(), image);
                        }
                    },
                    _ => {
                        let image = Image::new(&image_id, image_def, texture, &images_in_set, set.scale)?;
                        images_in_set.insert(image_id.to_string(), image);
                    }
                }
            }

            // now parse collected images
            for (id, image_def) in collected_images {
                let image = Image::new(id, image_def, texture, &images_in_set, set.scale)?;
                images_in_set.insert(id.to_string(), image);
            }

            // now parse timed images
            for (id, image_def) in timed_images {
                let image = Image::new(id, image_def, texture, &images_in_set, set.scale)?;
                images_in_set.insert(id.to_string(), image);
            }

            // now parse animated images
            for (id, image_def) in animated_images {
                let image = Image::new(id, image_def, texture, &images_in_set, set.scale)?;
                images_in_set.insert(id.to_string(), image);
            }

            // create the full hashmap with all images
            for (id, image) in images_in_set {
                images.insert(format!("{}/{}", set_id, id), image);
            }
        }

        let mut images_out = Vec::new();
        let mut image_handles = HashMap::new();
        for (index, (id, image)) in images.into_iter().enumerate() {
            let handle = ImageHandle { id: index };
            images_out.push(image);
            image_handles.insert(id, handle);
        }

        // insert empty image references for just "empty" and all sets as well
        image_handles.insert("empty".to_string(), ImageHandle { id: images_out.len() });
        images_out.push(Image::create_empty());

        for set_id in definition.image_sets.keys() {
            image_handles.insert(format!("{}/{}", set_id, "empty"), ImageHandle { id: images_out.len() });
            images_out.push(Image::create_empty());
        }

        // add in aliases
        for (to, from) in aliases {
            let handle = *image_handles.get(&from).ok_or_else(||
                Error::Theme(format!("Unable to locate image alias from '{}'", from))
            )?;

            image_handles.insert(to, handle);
        }

        // build the set of themes
        let mut theme_handles = HashMap::new();
        let mut themes = Vec::new();

        // create the default theme
        let default_handle = WidgetThemeHandle { id: 0 };
        let default_id = "default";
        themes.push(WidgetTheme::create_default(default_id, default_handle));
        theme_handles.insert(default_id.to_string(), default_handle);

        let mut handle_index = 1;
        for (theme_id, theme) in &definition.widgets {
            WidgetTheme::create(
                "",
                None,
                theme_id.to_string(), 
                &mut handle_index, 
                &mut theme_handles, 
                &mut themes, 
                theme, 
                &image_handles,
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

                let from_id = resolve_from(&themes, &theme_handles, from_str, *to_id).ok_or_else(|| {
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
            font_handles,
            fonts,
            image_handles,
            images: images_out,
            theme_handles,
            themes,
        })
    }

    pub(crate) fn default_theme(&self) -> &WidgetTheme {
        // This is always manually created
        &self.themes[0]
    }

    pub fn theme(&self, id: &str) -> Option<&WidgetTheme> {
        self.handle(id).map(|handle| &self.themes[handle.id as usize])
    }

    pub fn font(&self, handle: FontHandle) -> &Font {
        &self.fonts[handle.id()]
    }

    pub fn find_font(&self, id: Option<&str>) -> Option<FontSummary> {
        match id {
            None => None,
            Some(id) => {
                match self.font_handles.get(id) {
                    None => {
                        None
                    }, Some(font_sum) => {
                        Some(*font_sum)
                    }
                }
            }
        }
    }

    pub fn image(&self, handle: ImageHandle) -> &Image {
        &self.images[handle.id]
    }

    pub fn find_image(&self, id: Option<&str>) -> Option<ImageHandle> {
        match id {
            None => None,
            Some(id) => {
                match self.image_handles.get(id) {
                    None => {
                        None
                    }, Some(image) => Some(*image),
                }
            }
        }
    }

    pub fn handle(&self, id: &str) -> Option<WidgetThemeHandle> {
        self.theme_handles.get(id).cloned()
    }
}

fn resolve_from(
    themes: &[WidgetTheme],
    handles: &HashMap<String, WidgetThemeHandle>,
    from_str: &str,
    to_id: WidgetThemeHandle
) -> Option<WidgetThemeHandle> {
    // first, look for theme with the absolute path specified by from_str
    if let Some(handle) = handles.get(from_str) {
        return Some(*handle);
    }

    // now look for a theme relative to the current theme with from_str
    if let Some(parent_handle) = themes[to_id.id as usize].parent_handle {
        let parent_id = &themes[parent_handle.id as usize].full_id;
        let from_full_id = format!("{}/{}", parent_id, from_str);

        return handles.get(&from_full_id).copied();
    }

    None
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
    pub text_color: Option<Color>,
    pub font: Option<FontSummary>,
    pub background: Option<ImageHandle>,
    pub foreground: Option<ImageHandle>,
    pub tooltip: Option<String>,

    // all fields are options instead of using default so
    // we can detect when to override them
    pub wants_mouse: Option<bool>,
    pub wants_scroll: Option<bool>,
    pub text_align: Option<Align>,
    pub pos: Option<Point>,
    pub screen_pos: Option<Point>,
    pub size: Option<Point>,
    pub width_from: Option<WidthRelative>,
    pub height_from: Option<HeightRelative>,
    pub border: Option<Border>,
    pub align: Option<Align>,
    pub child_align: Option<Align>,
    pub layout: Option<Layout>,
    pub layout_spacing: Option<Point>,
    pub children: Vec<WidgetThemeHandle>,

    pub custom_floats: HashMap<String, f32>,
}

impl WidgetTheme {
    fn create_default(id: &'static str, handle: WidgetThemeHandle) -> WidgetTheme {
        WidgetTheme {
            from: None,
            full_id: id.to_string(),
            id: id.to_string(),
            parent_handle: None,
            handle,
            text: None,
            text_color: None,
            font: None,
            background: None,
            foreground: None,
            tooltip: None,
            wants_mouse: None,
            wants_scroll: None,
            text_align: None,
            pos: None,
            screen_pos: None,
            size: None,
            width_from: None,
            height_from: None,
            border: None,
            align: None,
            child_align: None,
            layout: None,
            layout_spacing: None,
            children: Vec::new(),
            custom_floats: HashMap::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn create(
        parent_id: &str,
        parent_handle: Option<WidgetThemeHandle>,
        id: String,
        handle_index: &mut u64,
        handles: &mut HashMap<String, WidgetThemeHandle>,
        themes: &mut Vec<WidgetTheme>,
        def: &WidgetThemeDefinition,
        images: &HashMap<String, ImageHandle>,
        fonts: &HashMap<String, FontSummary>,
    ) -> Result<WidgetThemeHandle, Error> {
        if id.contains('/') {
            return Err(
                Error::Theme(format!("'{}' theme name invalid.  the '/' character is not allowed", id))
            );
        }

        // handle top level as a special case
        let parent_id = if parent_id.is_empty() {
            id.to_string()
        } else {
            format!("{}/{}", parent_id, id)
        };

        let background = if let Some(bg) = def.background.as_ref() {
            Some(*images.get(bg).ok_or_else(||
                Error::Theme(format!("Unable to locate image '{}' as background for widget '{}'", bg, parent_id))
            )?)
        } else {
            None
        };

        let foreground = if let Some(fg) = def.foreground.as_ref() {
            Some(*images.get(fg).ok_or_else(||
                Error::Theme(format!("Unable to locate image '{}' as foreground for widget '{}'", fg, parent_id))
            )?)
        } else {
            None
        };

        let font = if let Some(font) = def.font.as_ref() {
            let font_handle = fonts.get(font).ok_or_else(||
                Error::Theme(format!("Unable to locate font '{}' for widget '{}'", font, parent_id))
            )?;
            Some(*font_handle)
        } else {
            None
        };

        let (width_from, height_from) = if let Some((width_from, height_from)) = def.size_from {
            (Some(width_from), Some(height_from))
        } else {
            (def.width_from, def.height_from)
        };

        let handle = WidgetThemeHandle { id: *handle_index };
        *handle_index += 1;
        let theme = WidgetTheme {
            from: def.from.clone(),
            parent_handle,
            handle,
            id,
            full_id: parent_id.to_string(),
            text: def.text.clone(),
            text_color: def.text_color,
            font,
            background,
            foreground,
            tooltip: def.tooltip.clone(),
            wants_mouse: def.wants_mouse,
            wants_scroll: def.wants_scroll,
            text_align: def.text_align,
            pos: def.pos,
            screen_pos: def.screen_pos,
            size: def.size,
            width_from,
            height_from,
            align: def.align,
            child_align: def.child_align,
            border: def.border,
            layout: def.layout,
            layout_spacing: def.layout_spacing,
            children: Vec::new(),
            custom_floats: def.custom_floats.clone(),
        };

        themes.push(theme);

        let mut children = Vec::new();
        for (child_id, child_def) in &def.children {
            let child = WidgetTheme::create(
                &parent_id,
                Some(handle),
                child_id.to_string(),
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
    if to.wants_scroll.is_none() { to.wants_scroll = from.wants_scroll; }
    if to.font.is_none() { to.font = from.font; }
    if to.background.is_none() { to.background = from.background; }
    if to.foreground.is_none() { to.foreground = from.foreground; }
    if to.text_align.is_none() { to.text_align = from.text_align; }
    if to.pos.is_none() { to.pos = from.pos; }
    if to.screen_pos.is_none() { to.screen_pos = from.screen_pos; }
    if to.size.is_none() { to.size = from.size; }
    if to.width_from.is_none() { to.width_from = from.width_from; }
    if to.height_from.is_none() { to.height_from = from.height_from; }
    if to.border.is_none() { to.border = from.border; }
    if to.align.is_none() { to.align = from.align; }
    if to.child_align.is_none() { to.child_align = from.child_align; }
    if to.layout.is_none() { to.layout = from.layout; }
    if to.layout_spacing.is_none() { to.layout_spacing = from.layout_spacing; }
    if to.text.is_none() { to.text = from.text.clone(); }
    if to.tooltip.is_none() { to.tooltip = from.tooltip.clone(); }

    for (id, value) in from.custom_floats.iter() {
        match to.custom_floats.entry(id.to_string()) {
            std::collections::hash_map::Entry::Occupied(_) => (),
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(*value);
            }
        }
    }

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