use std::cell::RefCell;
use std::rc::Rc;

use crate::{
    AnimState, AnimStateKey, Context, DrawData,
    Rect, Point, Widget, WidgetBuilder,
    WidgetState, ContextInternal, PersistentState,
};

const MOUSE_NOT_TAKEN: (bool, AnimState, Point) =
    (false, AnimState::normal(), Point { x: 0.0, y: 0.0 });

pub struct Frame {
    mouse_taken: Option<String>,
    context: Context,
    widgets: Vec<Widget>,
    parent_index: usize,
}

impl Frame {
    pub(crate) fn new(context: Context, root: Widget) -> Frame {
        Frame {
            mouse_taken: None,
            context,
            widgets: vec![root],
            parent_index: 0,
        }
    }

    pub(crate) fn context_internal(&self) -> &Rc<RefCell<ContextInternal>> {
        &self.context.internal
    }

    pub(crate) fn check_mouse_taken(&mut self, index: usize) -> (bool, AnimState, Point) {
        let widget = &self.widgets[index];

        let context = self.context.internal.borrow_mut();

        if context.mouse_pressed_outside() || self.mouse_taken.is_some() {
            return MOUSE_NOT_TAKEN;
        }

        let was_taken_last = context.mouse_taken_last_frame.as_deref() == Some(widget.id());

        // check if we are dragging on this widget
        if context.mouse_pressed[0] {
            if was_taken_last {
                self.mouse_taken = Some(widget.id().to_string());
                let dragged = context.mouse_pos - context.last_mouse_pos;
                return (
                    context.mouse_clicked[0],
                    AnimState::new(AnimStateKey::Pressed),
                    dragged
                );
            } else {
                return MOUSE_NOT_TAKEN;
            }
        }

        let bounds = Rect::new(widget.pos(), widget.size());
        if !bounds.inside(context.mouse_pos) {
            return MOUSE_NOT_TAKEN;
        }

        self.mouse_taken = Some(widget.id().to_string());
        (
            was_taken_last && context.mouse_clicked[0],
            AnimState::new(AnimStateKey::Hover),
            Point::default()
        )
    }

    pub(crate) fn init_state(&mut self, index: usize, open: bool) {
        let mut context = self.context.internal.borrow_mut();
        let widget = &self.widgets[index];
        context.init_state(widget.id(), open);
    }

    pub(crate) fn state(&self, index: usize) -> PersistentState {
        let context = self.context.internal.borrow();
        let widget = &self.widgets[index];
        context.state(widget.id())
    }

    pub(crate) fn parent_index(&self) -> usize { self.parent_index }

    pub(crate) fn set_parent_index(&mut self, index: usize) {
        self.parent_index = index;
    }
    pub(crate) fn next_index(&self) -> usize { self.widgets.len() }

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

    // convenience builder methods
    pub fn child(&mut self, theme: &str) -> WidgetState {
        self.start(theme).finish()
    }

    pub fn label<T: Into<String>>(&mut self, theme: &str, text: T) {
        self.start(theme).text(text).finish();
    }

    pub fn button<T: Into<String>>(&mut self, theme: &str, label: T) -> WidgetState {
        self.start(theme).text(label).wants_mouse(true).finish()
    }

    pub fn toggle_button<T: Into<String>>(&mut self, theme: &str, label: T, active: bool) -> WidgetState {
        self.start(theme).text(label).active(active).wants_mouse(true).finish()
    }

    // internal state modifiers
    pub fn toggle_open<T: Into<String>>(&mut self, id: T) {
        let mut context = self.context.internal.borrow_mut();
        let state = context.state_mut(id);
        state.is_open = !state.is_open;
    }

    pub fn is_open(&self, id: &str) -> bool {
        let context = self.context.internal.borrow();
        context.state(id).is_open
    }

    pub fn set_open<T: Into<String>>(&mut self, id: T, open: bool) {
        let mut context = self.context.internal.borrow_mut();
        context.state_mut(id).is_open = open;
    }

    pub fn set_parent_open(&mut self, open: bool) {
        let mut context = self.context.internal.borrow_mut();
        let id = self.widgets[self.parent_index].id();
        context.state_mut(id).is_open = open;
    }

    pub fn modify<T: Into<String>, F: Fn(&mut PersistentState)>(&mut self, id: T, f: F) {
        let mut context = self.context.internal.borrow_mut();
        (f)(context.state_mut(id));
    }

    pub fn render(self) -> DrawData {
        let mut context = self.context.internal.borrow_mut();
        context.mouse_clicked = [false; 3];
        context.mouse_taken_last_frame = self.mouse_taken;
        context.last_mouse_pos = context.mouse_pos;

        let themes = &context.themes;
        crate::render::render(themes, self.widgets, context.display_size)
    }
}