use crate::{
    AnimState, AnimStateKey, Color, Frame, Point, Border, Align, 
    Layout, WidthRelative, HeightRelative, Rect,
};
use crate::{frame::{RendGroup}, font::FontSummary, image::ImageHandle};
use crate::theme::{WidgetTheme};
use crate::window::WindowBuilder;

pub struct Widget {
    // identifier for persistent state
    id: String,
    rend_group: RendGroup,

    // TODO potentially move these out and store current parent data
    // in the frame for a small perf boost
    // stored in the widget for parent ref purposes
    scroll: Point,
    cursor: Point,
    theme_id: String,
    child_align: Align,
    layout: Layout,
    layout_spacing: Point,

    // stored in the widget for drawing purposes
    clip: Rect,
    text: Option<String>,
    text_color: Color,
    text_align: Align,
    font: Option<FontSummary>,
    background: Option<ImageHandle>,
    foreground: Option<ImageHandle>,
    pos: Point,
    size: Point,
    border: Border,
    anim_state: AnimState,
    visible: bool,
}

impl Widget {
    pub(crate) fn root(size: Point) -> Widget {
        Widget {
            theme_id: String::new(),
            text: None,
            text_align: Align::default(),
            text_color: Color::default(),
            font: None,
            background: None,
            foreground: None,
            layout: Layout::default(),
            layout_spacing: Point::default(),
            child_align: Align::default(),
            pos: Point::default(),
            scroll: Point::default(),
            cursor: Point::default(),
            border: Border::default(),
            size,
            id: String::new(),
            rend_group: RendGroup::default(),
            anim_state: AnimState::normal(),
            visible: true,
            clip: Rect { pos: Point::default(), size },
        }
    }

    fn create(parent: &Widget, theme: &WidgetTheme, id: String) -> (WidgetData, Widget) {
        let font = theme.font;
        let border = theme.border.unwrap_or_default();
        let raw_size = theme.size.unwrap_or_default();
        let width_from = theme.width_from.unwrap_or_default();
        let height_from = theme.height_from.unwrap_or_default();
        let size = size(parent, raw_size, border, font, width_from, height_from);

        let align = theme.align.unwrap_or(parent.child_align);
        let manual_pos = theme.pos.is_some() || align != parent.child_align;
        let cursor_pos = if align == parent.child_align {
            parent.cursor + parent.scroll
        } else {
            parent.scroll
        };
        let raw_pos = theme.pos.unwrap_or(cursor_pos) + parent.scroll;
        let pos = pos(parent, raw_pos, size, align);

        let data = WidgetData {
            manual_pos,
            wants_mouse: theme.wants_mouse.unwrap_or_default(),
            raw_size,
            raw_pos,
            width_from,
            height_from,
            align,
            enabled: true,
            active: false,
            recalc_pos_size: true,
            next_render_group: false,
        };

        let widget = Widget {
            layout: theme.layout.unwrap_or_default(),
            layout_spacing: theme.layout_spacing.unwrap_or_default(),
            child_align: theme.child_align.unwrap_or_default(),
            theme_id: theme.full_id.to_string(),
            text: theme.text.clone(),
            text_color: theme.text_color.unwrap_or_default(),
            text_align: theme.text_align.unwrap_or_default(),
            font,
            background: theme.background,
            foreground: theme.foreground,
            pos,
            scroll: Point::default(),
            cursor: Point::default(),
            border,
            size,
            id,
            rend_group: RendGroup::default(),
            anim_state: AnimState::normal(),
            visible: true,
            clip: parent.clip,
        };

        (data, widget)
    }

    pub fn clip(&self) -> Rect { self.clip }
    pub fn visible(&self) -> bool { self.visible }
    pub fn text_color(&self) -> Color { self.text_color }
    pub fn text_align(&self) -> Align { self.text_align }
    pub fn text(&self) -> Option<&str> { self.text.as_deref() }
    pub fn font(&self) -> Option<FontSummary> { self.font }
    pub fn foreground(&self) -> Option<ImageHandle> { self.foreground }
    pub fn background(&self) -> Option<ImageHandle> { self.background }
    pub fn border(&self) -> Border { self.border }
    pub fn id(&self) -> &str { &self.id }
    pub fn theme_id(&self) -> &str { &self.theme_id }
    pub fn anim_state(&self) -> AnimState { self.anim_state }
    pub fn size(&self) -> Point { self.size }
    pub fn pos(&self) -> Point { self.pos }

    pub fn inner_size(&self) -> Point {
        Point { x: self.size.x - self.border.horizontal(), y: self.size.y - self.border.vertical() }
    }

    pub fn set_cursor(&mut self, x: f32, y: f32) {
        self.cursor = Point { x, y };
    }

    pub fn cursor(&self) -> Point {
        self.cursor
    }

    pub fn gap(&mut self, gap: f32) {
        match self.layout {
            Layout::Horizontal => self.cursor.x += gap,
            Layout::Vertical => self.cursor.y += gap,
            Layout::Free => (),
        }
    }

    pub(crate) fn rend_group(&self) -> RendGroup { self.rend_group }

    pub(crate) fn set_rend_group(&mut self, group: RendGroup) {
        self.rend_group = group;
    }
}

pub struct WidgetState {
    pub visible: bool,
    pub hovered: bool,
    pub pressed: bool,
    pub clicked: bool,
    pub dragged: Point,
}

impl WidgetState {
    fn hidden() -> WidgetState {
        WidgetState {
            visible: false,
            hovered: false,
            pressed: false,
            clicked: false,
            dragged: Point::default(),
        }
    }

    fn new(anim_state: AnimState, clicked: bool, dragged: Point) -> WidgetState {
        let (hovered, pressed) = if anim_state.contains(AnimStateKey::Pressed) {
            (true, true)
        } else if anim_state.contains(AnimStateKey::Hover) {
            (true, false)
        } else {
            (false, false)
        };

        WidgetState {
            visible: true,
            hovered,
            pressed,
            clicked,
            dragged,
        }
    }
}

fn size(
    parent: &Widget,
    size: Point,
    border: Border,
    font: Option<FontSummary>,
    width_from: WidthRelative,
    height_from: HeightRelative,
) -> Point {
    let x = match width_from {
        WidthRelative::Normal => size.x,
        WidthRelative::Parent => size.x + parent.size.x - parent.border.horizontal(),
    };
    let y = match height_from {
        HeightRelative::Normal => size.y,
        HeightRelative::Parent => size.y + parent.size.y - parent.border.vertical(),
        HeightRelative::FontLine => size.y + font.map_or(0.0,
            |sum| sum.line_height) + border.vertical(),
    };
    Point { x, y }
}

fn pos(parent: &Widget, pos: Point, self_size: Point, align: Align) -> Point {
    let size = parent.size;
    let border = parent.border;

    let pos = parent.pos + match align {
        Align::Left => Point {
            x: border.left + pos.x,
            y: border.top + (size.y - border.vertical()) / 2.0 + pos.y
        },
        Align::Right => Point {
            x: size.x - border.right - pos.x,
            y: border.top + (size.y - border.vertical()) / 2.0 + pos.y
        },
        Align::Bot => Point {
            x: border.left + (size.x - border.horizontal()) / 2.0 + pos.x,
            y: size.y - border.bot - pos.y
        },
        Align::Top => Point {
            x: border.left + (size.x - border.horizontal()) / 2.0 + pos.x,
            y: border.top + pos.y
        },
        Align::Center => Point {
            x: border.left + (size.x - border.horizontal()) / 2.0 + pos.x,
            y: border.top + (size.y - border.vertical()) / 2.0 + pos.y
        },
        Align::BotLeft => Point {
            x: border.left + pos.x,
            y: size.y - border.bot - pos.y
        },
        Align::BotRight => Point {
            x: size.x - border.right - pos.x,
            y: size.y - border.bot - pos.y
        },
        Align::TopLeft => Point {
            x: border.left + pos.x,
            y: border.top + pos.y
        },
        Align::TopRight => Point {
            x: size.x - border.right - pos.x,
            y: border.top + pos.y
        },
    };

    pos - align.adjust_for(self_size).round()
}

pub(crate) struct WidgetData {
    manual_pos: bool,
    wants_mouse: bool,

    raw_pos: Point,
    raw_size: Point,
    width_from: WidthRelative,
    height_from: HeightRelative,
    align: Align,

    enabled: bool,
    active: bool,
    recalc_pos_size: bool,
    next_render_group: bool,
}

pub struct WidgetBuilder<'a> {
    pub frame: &'a mut Frame,
    pub parent: usize,
    pub widget: Widget,
    data: WidgetData,    
}

impl<'a> WidgetBuilder<'a> {
    #[must_use]
    pub fn new(frame: &'a mut Frame, parent: usize, theme_id: String, base_theme: &str) -> WidgetBuilder<'a> {
        let (data, widget) = {
            let context = std::rc::Rc::clone(&frame.context_internal());
            let context = context.borrow();
            let theme = match context.themes().theme(&theme_id) {
                None => {
                    match context.themes().theme(base_theme) {
                        None => {
                            // TODO remove unwrap
                            println!("Unable to locate theme either at {} or {}", theme_id, base_theme);
                            panic!();
                        }, Some(theme) => theme,
                    }
                }, Some(theme) => theme,
            };

            let id = {
                let parent_widget = frame.widget(parent);
                if parent_widget.id.is_empty() {
                    theme.id.to_string()
                } else {
                    format!("{}/{}", parent_widget.id, theme.id)
                }
            };

            let id = frame.generate_id(id);
            let parent_widget = frame.widget(parent);

            let (data, widget) = Widget::create(parent_widget, theme, id);

            (data, widget)
        };

        WidgetBuilder {
            frame,
            parent,
            widget,
            data,
        }
    }

    fn recalculate_pos_size(&mut self, state_moved: Point, state_resize: Point) {
        {
            let parent = self.frame.widget(self.parent);
            let widget = &self.widget;
            let size = size (
                parent,
                self.data.raw_size,
                widget.border,
                widget.font,
                self.data.width_from,
                self.data.height_from
            );

            self.widget.size = size;
        }

        {
            let parent = self.frame.widget(self.parent);
            let widget = &self.widget;
            let pos = pos(parent, self.data.raw_pos, widget.size, self.data.align);
            self.widget.pos = pos + state_moved;
        }

        self.widget.size = self.widget.size + state_resize;

        self.data.recalc_pos_size = false;
    }

    fn parent(&self) -> &Widget {
        self.frame.widget(self.parent)
    }
    
    #[must_use]
    pub fn new_render_group(mut self) -> WidgetBuilder<'a> {
        self.data.next_render_group = true;
        self
    }

    #[must_use]
    pub fn wants_mouse(mut self, wants_mouse: bool) -> WidgetBuilder<'a> {
        self.data.wants_mouse = wants_mouse;
        self
    }

    #[must_use]
    pub fn id<T: Into<String>>(mut self, id: T) -> WidgetBuilder<'a> {
        self.widget.id = id.into();
        self.data.recalc_pos_size = true;
        self
    }

    #[must_use]
    pub fn initially_open(self, open: bool) -> WidgetBuilder<'a> {
        {
            let mut context = self.frame.context_internal().borrow_mut();
            context.init_state(&self.widget.id, open);
        }
        self
    }

    #[must_use]
    pub fn text_color(mut self, color: Color) -> WidgetBuilder<'a> {
        self.widget.text_color = color;
        self
    }

    #[must_use]
    pub fn text_align(mut self, align: Align) -> WidgetBuilder<'a> {
        self.widget.text_align = align;
        self
    }

    #[must_use]
    pub fn text<T: Into<String>>(mut self, text: T) -> WidgetBuilder<'a> {
        self.widget.text = Some(text.into());
        self
    }

    #[must_use]
    pub fn font(mut self, font: &str) -> WidgetBuilder<'a> {
        let font = {
            let context = self.frame.context_internal();
            let context = context.borrow();
            context.themes().find_font(Some(font))
        };

        self.widget.font = font;
        self.data.recalc_pos_size = true;
        self
    }

    #[must_use]
    pub fn foreground(mut self, fg: &str) -> WidgetBuilder<'a> {
        let fg = {
            let context = self.frame.context_internal();
            let context = context.borrow();
            context.themes().find_image(Some(fg))
        };

        self.widget.foreground = fg;
        self
    }

    #[must_use]
    pub fn background(mut self, bg: &str) -> WidgetBuilder<'a> {
        let bg = {
            let context = self.frame.context_internal();
            let context = context.borrow();
            context.themes().find_image(Some(bg))
        };

        self.widget.background = bg;
        self
    }

    #[must_use]
    pub fn child_align(mut self, align: Align) -> WidgetBuilder<'a> {
        self.widget.child_align = align;
        self
    }

    #[must_use]
    pub fn layout_spacing(mut self, spacing: Point) -> WidgetBuilder<'a> {
        self.widget.layout_spacing = spacing;
        self
    }

    #[must_use]
    pub fn layout_horizontal(self) -> WidgetBuilder<'a> {
        self.layout(Layout::Horizontal)
    }

    #[must_use]
    pub fn layout_vertical(self) -> WidgetBuilder<'a> {
        self.layout(Layout::Vertical)
    }

    #[must_use]
    pub fn layout(mut self, layout: Layout) -> WidgetBuilder<'a> {
        self.widget.layout = layout;
        self
    }

    #[must_use]
    pub fn screen_pos(mut self, x: f32, y: f32) -> WidgetBuilder<'a> {
        self.data.raw_pos = Point { x, y };
        self.widget.pos = Point { x, y };
        self.data.align = Align::TopLeft;
        self.data.manual_pos = true;
        self.data.recalc_pos_size = false;
        self
    }

    #[must_use]
    pub fn pos(mut self, x: f32, y: f32) -> WidgetBuilder<'a> {
        self.data.raw_pos = Point { x, y } + self.parent().scroll;
        self.data.manual_pos = true;
        self.data.recalc_pos_size = true;
        self
    }

    #[must_use]
    pub fn align(mut self, align: Align) -> WidgetBuilder<'a> {
        self.data.align = align;
        self.data.manual_pos = true;
        self.data.recalc_pos_size = true;
        self
    }
    
    #[must_use]
    pub fn border(mut self, border: Border) -> WidgetBuilder<'a> {
        self.widget.border = border;
        self.data.recalc_pos_size = true;
        self
    }

    #[must_use]
    pub fn size(mut self, x: f32, y: f32) -> WidgetBuilder<'a> {
        self.data.raw_size = Point { x, y };
        self.data.recalc_pos_size = true;
        self
    }

    #[must_use]
    pub fn width_from(mut self, from: WidthRelative) -> WidgetBuilder<'a> {
        self.data.width_from = from;
        self.data.recalc_pos_size = true;
        self
    }

    #[must_use]
    pub fn height_from(mut self, from: HeightRelative) -> WidgetBuilder<'a> {
        self.data.height_from = from;
        self.data.recalc_pos_size = true;
        self
    }

    #[must_use]
    pub fn clip(mut self, clip: Rect) -> WidgetBuilder<'a> {
        let cur_clip = self.widget.clip;
        self.widget.clip = cur_clip.min(clip);
        self
    }

    #[must_use]
    pub fn active(mut self, active: bool) -> WidgetBuilder<'a> {
        self.data.active = active;
        self
    }

    #[must_use]
    pub fn visible(mut self, visible: bool) -> WidgetBuilder<'a> {
        self.widget.visible = visible;
        self
    }

    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> WidgetBuilder<'a> {
        self.data.enabled = enabled;
        self
    }

    
    /// Force the widget to layout its `size` and `position` immediately.
    /// Assuming these attributes are not changed after this method is
    /// called, these attributes will have their final values after this
    /// method returns.  The size and position are written to the passed
    /// in rect.
    #[must_use]
    pub fn trigger_layout(mut self, rect: &mut Rect) -> WidgetBuilder<'a> {
        let (state_moved, state_resize) = {
            let internal = self.frame.context_internal().borrow();
            let state = internal.state(&self.widget.id);
            (state.moved, state.resize)
        };
        if self.data.recalc_pos_size {
            self.recalculate_pos_size(state_moved, state_resize);
        }

        rect.pos = self.widget.pos;
        rect.size = self.widget.size;
        self
    }

    /// Causes this widget to layout its current text.  The final position of the text
    /// cursor is written into `pos`.  If this widget does not have a font or has no text,
    /// nothing is written into `pos`.
    #[must_use]
    pub fn trigger_text_layout(mut self, cursor: &mut Point) -> WidgetBuilder<'a> {
        // recalculate pos size and calculate text, if needed
        let (text, state_moved, state_resize) = {
            let internal = self.frame.context_internal().borrow();
            let state = internal.state(&self.widget.id);
            (
                state.text.as_ref().map(|t| t.to_string()),
                state.moved,
                state.resize,
            )
        };

        if self.data.recalc_pos_size {
            self.recalculate_pos_size(state_moved, state_resize);
        }

        if let Some(text) = text {
            self.widget.text = Some(text);
        }

        let text = match &self.widget.text {
            None => return self,
            Some(text) => text,
        };

        let font_def = match self.widget.font {
            None => return self,
            Some(def) => def,
        };

        {
            let widget = &self.widget;
            let fg_pos = Point::default();
            let fg_size = widget.inner_size();
            let align = widget.text_align();

            let internal = self.frame.context_internal().borrow();
            let scale = internal.scale_factor();
            let font = internal.themes().font(font_def.handle);

            let mut scaled_cursor = *cursor * scale;

            font.layout(fg_size * scale, fg_pos * scale, text, align, &mut scaled_cursor);

            *cursor = scaled_cursor / scale;
        }

        self
    }

    /// Turns this builder into a WindowBuilder.  You should use all `WidgetBuilder` methods
    /// before calling this method.  The window must still be completed with one of the
    /// `WindowBuilder` methods.  You must pass a unique `id` for each each window
    /// created by your application.
    #[must_use]
    pub fn window(self, id: &str) -> WindowBuilder<'a> {
        WindowBuilder::new(self.id(id).new_render_group())
    }

    /// Consumes this builder and adds a scrollpane widget to the current frame.
    /// The provided closure is called to add children to the scrollpane's content.
    pub fn scrollpane<F: FnOnce(&mut Frame)>(self, content_id: &str, f: F) -> WidgetState {
        self.finish_with(Some(crate::scrollpane_content(content_id, f)))
    }

    /// Consumes the builder and adds a widget to the current frame.  The
    /// returned data includes information about the animation state and
    /// mouse interactions of the created element.
    /// If you wish this widget to have one or more child widgets, you should
    /// call `children` instead.
    pub fn finish(self) -> WidgetState {
        self.finish_with(None::<fn(&mut Frame)>)
    }

    /// Consumes the builder and adds a widget to the current frame.  The
    /// returned data includes information about the animation state and
    /// mouse interactions of the created element.
    /// The provided closure is called to enable adding children to this widget.
    /// If you don't want to add children, you can just call `finish` instead.
    pub fn children<F: FnOnce(&mut Frame)>(self, f: F) -> WidgetState {
        self.finish_with(Some(f))
    }

    fn finish_with<F: FnOnce(&mut Frame)>(mut self, f: Option<F>) -> WidgetState {
        if !self.widget.visible { return WidgetState::hidden(); }

        let (state, text, in_modal_tree) = {
            let internal = self.frame.context_internal().borrow();
            let state = internal.state(&self.widget.id);

            let text = match &state.text {
                None => None,
                Some(text) => Some(text.to_string())
            };

            let in_modal_tree = Some(self.widget.id()) == internal.modal_id();

            (state.copy_data(), text, in_modal_tree)
        };

        if let Some(text) = text {
            self.widget.text = Some(text);
        }

        self.widget.scroll = state.scroll;
        self.widget.cursor = self.widget.cursor;

        if !state.is_open {
            self.widget.visible = false;
            return WidgetState::hidden();
        }

        if self.data.recalc_pos_size {
            self.recalculate_pos_size(state.moved, state.resize);
        }

        let self_pos = self.widget.pos;
        let self_size = self.widget.size;
        let self_bounds = Rect::new(self_pos, self_size);
        let old_max_child_bounds = self.frame.max_child_bounds();

        // set modal tree value only if a match is found
        if in_modal_tree {
            {
                let mut internal = self.frame.context_internal().borrow_mut();
                internal.mut_modal(|modal| {
                    modal.bounds = self_bounds;
                });
            }
            self.frame.in_modal_tree = true;
        }

        let prev_rend_group = self.frame.cur_render_group();

        if self.data.next_render_group {
            self.frame.next_render_group(self_bounds, self.widget.id.to_string());
        }

        let widget_index = self.frame.num_widgets();
        self.frame.push_widget(self.widget);

        // if there is a child function
        if let Some(f) = f {
            // push the max_child pos and parent index
            self.frame.set_max_child_bounds(self_bounds);
            let old_parent_index = self.frame.parent_index();
            self.frame.set_parent_index(widget_index);

            // build all children
            (f)(self.frame);

            self.frame.set_parent_index(old_parent_index);
            let this_children_max_bounds = self.frame.max_child_bounds();
            self.frame.set_parent_max_child_bounds(this_children_max_bounds);
        }

        self.frame.set_max_child_bounds(old_max_child_bounds.max(self_bounds));

        let (clicked, mut anim_state, dragged) = if self.data.enabled && self.data.wants_mouse {
            self.frame.check_mouse_taken(widget_index)
        } else {
            (false, AnimState::disabled(), Point::default())
        };

        if self.data.next_render_group {
            self.frame.prev_render_group(prev_rend_group);
        }

        // unset modal tree value only if this widget was the modal one
        if in_modal_tree {
            self.frame.in_modal_tree = false;
        }

        if self.data.active {
            anim_state.add(AnimStateKey::Active);
        }

        self.frame.widget_mut(widget_index).anim_state = anim_state;

        let state = WidgetState::new(anim_state, clicked, dragged);
        let size = self.frame.widget(widget_index).size;
        if !self.data.manual_pos {
            use Align::*;
            let (x, y) = match self.frame.widget(self.parent).child_align {
                Left => (size.x, 0.0),
                Right => (-size.x, 0.0),
                Bot => (0.0, -size.y),
                Top => (0.0, size.y),
                Center => (0.0, 0.0),
                BotLeft => (size.x, -size.y),
                BotRight => (-size.x, -size.y),
                TopLeft => (size.x, size.y),
                TopRight => (-size.x, size.y),
            };

            let parent = self.frame.widget_mut(self.parent);
            use Layout::*;
            match parent.layout {
                Horizontal => parent.cursor.x += x + parent.layout_spacing.x,
                Vertical => parent.cursor.y += y + parent.layout_spacing.y,
                Free => (),
            }
        }
        
        state
    }
}