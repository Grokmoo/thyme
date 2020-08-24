use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;

use crate::context::{Context, ContextInternal};
use crate::{
    AnimState, AnimStateKey, Rect, Point, WidgetBuilder, PersistentState, Align,
};
use crate::image::ImageHandle;
use crate::widget::Widget;

const MOUSE_NOT_TAKEN: (bool, AnimState, Point) =
    (false, AnimState::normal(), Point { x: 0.0, y: 0.0 });

pub struct Frame {
    mouse_taken: Option<String>,
    context: Context,
    widgets: Vec<Widget>,
    render_groups: Vec<RenderGroup>,
    parent_index: usize,
    pub(crate) in_modal_tree: bool,
    parent_max_child_bounds: Rect,
    max_child_bounds: Rect,

    generated_ids: HashMap<String, u32>,

    mouse_cursor: Option<(ImageHandle, Align)>,
    mouse_anim_state: AnimState,
}

impl Frame {
    pub(crate) fn new(context: Context, root: Widget, mouse_anim_state: AnimState) -> Frame {
        Frame {
            mouse_taken: None,
            context,
            widgets: vec![root],
            render_groups: vec![RenderGroup { start: 0, end: 0 }],
            parent_index: 0,
            in_modal_tree: false,
            parent_max_child_bounds: Rect::default(),
            max_child_bounds: Rect::default(),
            generated_ids: HashMap::default(),
            mouse_cursor: None,
            mouse_anim_state,
        }
    }

    pub(crate) fn mouse_cursor(&self) -> Option<(ImageHandle, Align, AnimState)> {
        self.mouse_cursor.map(|(image, align)| (image, align, self.mouse_anim_state))
    }

    pub(crate) fn generate_id(&mut self, id: String) -> String {
        let mut output = id.clone();
        let index = self.generated_ids.entry(id).or_insert(0);

        if *index > 0 {
            output.push_str(&index.to_string());
        }

        *index += 1;

        output
    }

    pub(crate) fn context_internal(&self) -> &Rc<RefCell<ContextInternal>> {
        &self.context.internal()
    }

    pub(crate) fn check_mouse_taken(&mut self, index: usize) -> (bool, AnimState, Point) {
        let widget = &self.widgets[index];

        let context = self.context.internal().borrow_mut();

        if context.has_modal() && !self.in_modal_tree {
            return MOUSE_NOT_TAKEN;
        }

        if context.mouse_pressed_outside() || self.mouse_taken.is_some() ||
            !widget.clip().is_inside(context.mouse_pos()) {
            return MOUSE_NOT_TAKEN;
        }

        let was_taken_last = context.mouse_taken_last_frame() == Some(widget.id());

        // check if we are dragging on this widget
        if context.mouse_pressed(0) {
            if was_taken_last {
                self.mouse_taken = Some(widget.id().to_string());
                let dragged = context.mouse_pos() - context.last_mouse_pos();
                return (
                    context.mouse_clicked(0),
                    AnimState::new(AnimStateKey::Pressed),
                    dragged
                );
            } else {
                return MOUSE_NOT_TAKEN;
            }
        }

        let bounds = Rect::new(widget.pos(), widget.size());
        if !bounds.is_inside(context.mouse_pos()) {
            return MOUSE_NOT_TAKEN;
        }

        self.mouse_taken = Some(widget.id().to_string());
        (
            was_taken_last && context.mouse_clicked(0),
            AnimState::new(AnimStateKey::Hover),
            Point::default()
        )
    }

    pub(crate) fn init_state(&mut self, index: usize, open: bool) {
        let mut context = self.context.internal().borrow_mut();
        let widget = &self.widgets[index];
        context.init_state(widget.id(), open);
    }

    pub(crate) fn max_child_bounds(&self) -> Rect { self.max_child_bounds }

    pub(crate) fn set_max_child_bounds(&mut self, bounds: Rect) {
        self.max_child_bounds = bounds;
    }

    pub(crate) fn set_parent_max_child_bounds(&mut self, bounds: Rect) {
        self.parent_max_child_bounds = bounds;
    }

    pub(crate) fn parent_index(&self) -> usize { self.parent_index }

    pub(crate) fn set_parent_index(&mut self, index: usize) {
        self.parent_index = index;
    }
    pub(crate) fn num_widgets(&self) -> usize { self.widgets.len() }

    pub(crate) fn push_widget(&mut self, widget: Widget) {
        self.widgets.push(widget);
    }

    pub(crate) fn widget(&self, index: usize) -> &Widget {
        &self.widgets[index]
    }

    pub(crate) fn widget_mut(&mut self, index: usize) -> &mut Widget {
        &mut self.widgets[index]
    }

    // ui builder methods
    pub fn gap(&mut self, gap: f32) {
        self.widgets[self.parent_index].gap(gap);
    }

    pub fn set_cursor(&mut self, x: f32, y: f32) {
        self.widgets[self.parent_index].set_cursor(x, y);
    }

    pub fn cursor(&self) -> Point { self.widgets[self.parent_index].cursor() }

    #[must_use]
    pub fn start(&mut self, theme: &str) -> WidgetBuilder {
        let parent = &self.widgets[self.parent_index];

        let theme_id = if parent.theme_id().is_empty() {
            theme.to_string()
        } else {
            format!("{}/{}", parent.theme_id(), theme)
        };

        WidgetBuilder::new(self, self.parent_index, theme_id, theme)
    }

    // internal state modifiers

    /// Sets the mouse cursor to the specified image with alignment.  If you are hiding the default
    /// OS cursor, this should be called at least once every frame you want to show a cursor.  If it
    /// is called multiple times, the last call will take effect.  The image will automatically inherit
    /// `Normal` and `Pressed` animation states.  See `set_mouse_state` to override this behavior.
    pub fn set_mouse_cursor(&mut self, image: &str, align: Align) {
        let context = self.context.internal().borrow();
        let image = context.themes().find_image(Some(image));
        self.mouse_cursor = image.map(|image| (image, align));
    }

    /// Manually set the Mouse cursor to the specified `state`.  This is used when
    /// drawing the specified mouse cursor image.  The mouse will automatically inherit
    /// `Normal` and `Pressed` states by default.  This overrides that behavior.
    pub fn set_mouse_state(&mut self, state: AnimState) {
        self.mouse_anim_state = state;
    }

    pub fn focus_keyboard<T: Into<String>>(&mut self, id: T) {
        let mut context = self.context.internal().borrow_mut();
        context.set_focus_keyboard(id.into());
    }

    pub fn is_focus_keyboard(&self, id: &str) -> bool {
        let context = self.context.internal().borrow();
        context.is_focus_keyboard(id)
    }

    pub fn parent_max_child_bounds(&self) -> Rect { self.parent_max_child_bounds }

    pub fn cur_time_millis(&self) -> u32 {
        let context = self.context.internal().borrow();
        context.time_millis()
    }

    pub fn set_base_time_millis<T: Into<String>>(&mut self, id: T, time: u32) {
        let mut context = self.context.internal().borrow_mut();
        let state = context.state_mut(id);
        state.base_time_millis = time;
    }

    pub fn set_base_time_now<T: Into<String>>(&mut self, id: T) {
        let mut context = self.context.internal().borrow_mut();
        let cur_time = context.time_millis();
        let state = context.state_mut(id);
        state.base_time_millis = cur_time;
    }

    /// Returns the current time in millis minus the base time millis for the
    /// widget with the specified ID.  If the base time millis has not been set,
    /// will return the current time millis
    pub fn base_time_millis(&self, id: &str) -> u32 {
        let context = self.context.internal().borrow();
        context.state(id).base_time_millis
    }
    
    pub fn scroll(&self, id: &str) -> Point {
        let context = self.context.internal().borrow();
        context.state(id).scroll
    }

    pub fn change_scroll<T: Into<String>>(&mut self, id: T, x: f32, y: f32) {
        let mut context = self.context.internal().borrow_mut();
        let state = context.state_mut(id);
        state.scroll = state.scroll + Point { x, y }
    }

    pub fn offset_time_millis(&self, id: &str) -> i32 {
        let context = self.context.internal().borrow();
        context.time_millis() as i32 - context.state(id).base_time_millis as i32
    }

    pub fn text_for(&self, id: &str) -> Option<String> {
        let context = self.context.internal().borrow();
        context.state(id).text.clone()
    }

    pub fn is_open(&self, id: &str) -> bool {
        let context = self.context.internal().borrow();
        context.state(id).is_open
    }

    pub fn open_modal<T: Into<String>>(&mut self, id: T) {
        let id = id.into();

        let mut context = self.context.internal().borrow_mut();
        context.state_mut(id.clone()).is_open = true;
        context.set_modal(id);
    }

    pub fn close_modal_on_click_outside(&mut self) {
        let mut context = self.context.internal().borrow_mut();
        context.mut_modal(|modal| {
            modal.close_on_click_outside = true;
        });
    }

    pub fn open<T: Into<String>>(&mut self, id: T) {
        let mut context = self.context.internal().borrow_mut();
        context.state_mut(id).is_open = true;
    }

    pub fn close<T: Into<String>>(&mut self, id: T) {
        let id = id.into();

        let mut context = self.context.internal().borrow_mut();
        context.clear_modal_if_match(&id);
        context.state_mut(id).is_open = false;
    }

    pub fn open_parent(&mut self) {
        let mut context = self.context.internal().borrow_mut();
        let id = self.widgets[self.parent_index].id();
        context.state_mut(id).is_open = true;
    }

    pub fn close_parent(&mut self) {
        let mut context = self.context.internal().borrow_mut();
        let id = self.widgets[self.parent_index].id();
        context.clear_modal_if_match(id);
        context.state_mut(id).is_open = false;
    }

    pub fn clear(&mut self, id: &str) {
        let mut context = self.context.internal().borrow_mut();
        context.clear_modal_if_match(id);
        context.clear_state(id);
    }

    pub fn modify<T: Into<String>, F: FnOnce(&mut PersistentState)>(&mut self, id: T, f: F) {
        let mut context = self.context.internal().borrow_mut();
        (f)(context.state_mut(id));
    }

    pub(crate) fn next_render_group(&mut self) {
        let last_render_group = self.render_groups.len() - 1;
        let widgets_len = self.widgets.len();
        self.render_groups[last_render_group].end = widgets_len;
        self.render_groups.push(RenderGroup { start: widgets_len, end: widgets_len });
    }

    pub(crate) fn finish_frame(mut self) -> (Context, Vec<Widget>, Vec<RenderGroup>) {
        let last_render_group = self.render_groups.len() - 1;
        self.render_groups[last_render_group].end = self.widgets.len();

        self.context.internal().borrow_mut().next_frame(self.mouse_taken);

        (self.context, self.widgets, self.render_groups)
    }
}

pub(crate) struct RenderGroup {
    start: usize,
    end: usize,
}

impl RenderGroup {
    pub(crate) fn iter<'a, 'b>(&'a self, widgets: &'b [Widget]) -> impl Iterator<Item=&'b Widget> {
        widgets.iter().skip(self.start).take(self.end - self.start)
    }
}