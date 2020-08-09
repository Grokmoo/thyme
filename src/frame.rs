use std::cell::RefCell;
use std::rc::Rc;

use crate::context::{Context, ContextInternal};

use crate::{
    AnimState, AnimStateKey, Rect, Point, WidgetBuilder, PersistentState,
};
use crate::widget::Widget;

const MOUSE_NOT_TAKEN: (bool, AnimState, Point) =
    (false, AnimState::normal(), Point { x: 0.0, y: 0.0 });

pub struct Frame {
    mouse_taken: Option<String>,
    context: Context,
    widgets: Vec<Widget>,
    parent_index: usize,
    parent_max_child_pos: Point,
    max_child_pos: Point,
}

impl Frame {
    pub(crate) fn new(context: Context, root: Widget) -> Frame {
        Frame {
            mouse_taken: None,
            context,
            widgets: vec![root],
            parent_index: 0,
            parent_max_child_pos: Point::default(),
            max_child_pos: Point::default(),
        }
    }

    pub(crate) fn context_internal(&self) -> &Rc<RefCell<ContextInternal>> {
        &self.context.internal()
    }

    pub(crate) fn check_mouse_taken(&mut self, index: usize) -> (bool, AnimState, Point) {
        let widget = &self.widgets[index];

        let context = self.context.internal().borrow_mut();

        if context.mouse_pressed_outside() || self.mouse_taken.is_some() {
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

    pub(crate) fn state(&self, index: usize) -> PersistentState {
        let context = self.context.internal().borrow();
        let widget = &self.widgets[index];
        context.state(widget.id())
    }

    pub(crate) fn max_child_pos(&self) -> Point { self.max_child_pos }

    pub(crate) fn set_max_child_pos(&mut self, pos: Point) {
        self.max_child_pos = pos;
    }

    

    pub(crate) fn set_parent_max_child_pos(&mut self, pos: Point) {
        self.parent_max_child_pos = pos;
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

    pub fn parent_max_child_pos(&self) -> Point { self.parent_max_child_pos }

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

    pub fn base_time_millis(&self, id: &str) -> u32 {
        let context = self.context.internal().borrow();
        context.state(id).base_time_millis
    }

    /// Returns the current time in millis minus the base time millis for the
    /// widget with the specified ID.  If the base time millis has not been set,
    /// will return the current time millis
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

    pub fn toggle_open<T: Into<String>>(&mut self, id: T) {
        let mut context = self.context.internal().borrow_mut();
        let state = context.state_mut(id);
        state.is_open = !state.is_open;
    }

    pub fn is_open(&self, id: &str) -> bool {
        let context = self.context.internal().borrow();
        context.state(id).is_open
    }

    pub fn set_open<T: Into<String>>(&mut self, id: T, open: bool) {
        let mut context = self.context.internal().borrow_mut();
        context.state_mut(id).is_open = open;
    }

    pub fn set_parent_open(&mut self, open: bool) {
        let mut context = self.context.internal().borrow_mut();
        let id = self.widgets[self.parent_index].id();
        context.state_mut(id).is_open = open;
    }

    pub fn modify<T: Into<String>, F: Fn(&mut PersistentState)>(&mut self, id: T, f: F) {
        let mut context = self.context.internal().borrow_mut();
        (f)(context.state_mut(id));
    }

    pub(crate) fn finish_frame(self) -> (Context, Vec<Widget>) {
        self.context.internal().borrow_mut().next_frame(self.mouse_taken);

        (self.context, self.widgets)
    }
}