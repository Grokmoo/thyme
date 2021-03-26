use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;

use crate::theme_definition::CustomData;
use crate::context::{Context, ContextInternal, InputModifiers};
use crate::{
    AnimState, AnimStateKey, Rect, Point, WidgetBuilder, PersistentState, Align,
};
use crate::image::ImageHandle;
use crate::widget::Widget;

const MOUSE_NOT_TAKEN: MouseState =
    MouseState { clicked: false, anim: AnimState::normal(), dragged: Point { x: 0.0, y: 0.0 } };

/// A Frame, holding the widget tree to be drawn on a given frame, and a reference to the
/// Thyme [`Context`](struct.Context.html)
///
/// The frame is the main object that you pass along through your UI builder functions.  It allows
/// you to construct [`WidgetBuilders`](struct.WidgetBuilder.html) both with full control and
/// convenient helper methods.
///
/// Frame also contains a number of methods for manipulating the internal [`PersistentState`](struct.PersistentState.html)
/// associated with a particular widget, with [`modify`](#method.modify) providing full control.
///
/// When building your UI, there will always be a current parent widget that widgets are currently being added to.  This
/// starts at the root widget which has defaults for all parameters.  Each [`children`](struct.WidgetBuilder.html#method.children)
/// closure you enter changes the associated parent widget.
pub struct Frame {
    mouse_taken: Option<(String, RendGroup)>,
    context: Context,
    widgets: Vec<Widget>,
    render_groups: Vec<RendGroupDef>,
    cur_rend_group: RendGroup,

    parent_index: usize,
    pub(crate) in_modal_tree: bool,
    parent_max_child_bounds: Rect,
    max_child_bounds: Rect,

    generated_ids: HashMap<String, u32>,

    mouse_cursor: Option<(ImageHandle, Align)>,
    mouse_anim_state: AnimState,
}

pub(crate) struct MouseState {
    pub clicked: bool,
    pub anim: AnimState,
    pub dragged: Point,
}

impl Frame {
    pub(crate) fn new(context: Context, root: Widget, mouse_anim_state: AnimState) -> Frame {
        let cur_rend_group = RendGroup::default();
        Frame {
            mouse_taken: None,
            context,
            widgets: vec![root],
            cur_rend_group,
            render_groups: vec![RendGroupDef {
                rect: Rect::default(),
                id: String::new(),
                group: cur_rend_group,
                start: 0,
                num: 0,
                always_top: false,
            }],
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

    pub(crate) fn context(&self) -> &Context {
        &self.context
    }

    pub(crate) fn context_internal(&self) -> &Rc<RefCell<ContextInternal>> {
        &self.context.internal()
    }

    pub(crate) fn check_mouse_wheel(&mut self, index: usize) -> Option<Point> {
        let widget = &self.widgets[index];

        let mut context = self.context.internal().borrow_mut();

        if context.has_modal() && !self.in_modal_tree {
            return None;
        }

        if let Some(group) = context.mouse_in_rend_group_last_frame() {
            if widget.rend_group() != group {
                return None;
            }
        }

        let bounds = Rect::new(widget.pos(), widget.size());
        if !bounds.is_inside(context.mouse_pos()) {
            return None;
        }

        Some(context.take_mouse_wheel())
    }

    pub(crate) fn check_mouse_state(&mut self, index: usize) -> MouseState {
        let widget = &self.widgets[index];

        let mut context = self.context.internal().borrow_mut();

        if context.has_modal() && !self.in_modal_tree {
            return MOUSE_NOT_TAKEN;
        }

        if let Some(group) = context.mouse_in_rend_group_last_frame() {
            if widget.rend_group() != group {
                return MOUSE_NOT_TAKEN;
            }
        }

        if context.mouse_pressed_outside() || self.mouse_taken.is_some() ||
            !widget.clip().is_inside(context.mouse_pos()) {
            return MOUSE_NOT_TAKEN;
        }

        let was_taken_last = context.mouse_taken_last_frame_id() == Some(widget.id());

        // check if we are dragging on this widget
        if context.mouse_pressed(0) {
            if was_taken_last {
                self.mouse_taken = Some((widget.id().to_string(), widget.rend_group()));
                let dragged = context.mouse_pos() - context.last_mouse_pos();

                if context.mouse_pressed(0) {
                    context.set_top_rend_group(widget.rend_group());
                }
                return MouseState {
                    clicked: context.mouse_clicked(0),
                    anim: AnimState::new(AnimStateKey::Pressed),
                    dragged
                };
            } else {
                return MOUSE_NOT_TAKEN;
            }
        }

        let bounds = Rect::new(widget.pos(), widget.size());
        if !bounds.is_inside(context.mouse_pos()) {
            return MOUSE_NOT_TAKEN;
        }

        if context.mouse_pressed(0) {
            context.set_top_rend_group(widget.rend_group());
        }

        self.mouse_taken = Some((widget.id().to_string(), widget.rend_group()));
        MouseState {
            clicked: was_taken_last && context.mouse_clicked(0),
            anim: AnimState::new(AnimStateKey::Hover),
            dragged: Point::default()
        }
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

    pub(crate) fn widget(&self, index: usize) -> &Widget {
        &self.widgets[index]
    }

    pub(crate) fn widget_mut(&mut self, index: usize) -> &mut Widget {
        &mut self.widgets[index]
    }

    /**
    Starts creating a new child widget within the current parent, using the specified `theme`.
    See [`the crate root`](index.html) for a discussion of the theme format.  This method
    returns a [`WidgetBuilder`](struct.WidgetBuilder.html) which can be used for fully
    customizing the new widget.

    # Example
    ```
    fn create_ui(ui: &mut Frame) {
        ui.start("cancel_button").finish();
    }
    ```

    */
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

    // ui builder methods

    /// Returns the current window display size, in logical pixels.
    pub fn display_size(&self) -> Point {
        let context = self.context_internal().borrow();
        context.display_size() / context.scale_factor()
    }

    /// Returns the current state of the keyboard modifier keys
    pub fn input_modifiers(&self) -> InputModifiers {
        let context = self.context_internal().borrow();
        context.input_modifiers()
    }

    /// Returns the current mouse position and size, in logical pixels
    pub fn mouse_rect(&self) -> Rect {
        let context = self.context_internal().borrow();

        let (align, size) = if let Some((handle, align)) = self.mouse_cursor {
            (align, context.themes().image(handle).base_size())
        } else {
            // TODO how to get platform mouse cursor size?
            (Align::TopLeft, Point::new(24.0, 24.0))
        };

        let pos = context.mouse_pos() + align.adjust_for(size);

        Rect::new(pos, size)
    }

    /// Returns whether or not the Thyme UI wants the mouse this frame.
    /// See [`Context.wants_mouse`](struct.Context.html#wants_mouse)
    pub fn wants_mouse(&self) -> bool {
        self.context.wants_mouse()
    }

    /// Returns whether or not the Thyme UI wants the keyboard this frame.
    /// See [`Context.wants_keyboard`](struct.Context.html#wants_keyboard)
    pub fn wants_keyboard(&self) -> bool {
        self.context.wants_keyboard()
    }

    /// Sets the mouse cursor to the specified image with alignment.  If you are hiding the default
    /// OS cursor, this should be called at least once every frame you want to show a cursor.  If it
    /// is called multiple times, the last call will take effect.  The image will automatically inherit
    /// `Normal` and `Pressed` animation states.  See `set_mouse_state` to override this behavior.
    pub fn set_mouse_cursor(&mut self, image: &str, align: Align) {
        let image = self.context.find_image(image);
        self.mouse_cursor = image.map(|image| (image, align));
    }

    /// Manually set the Mouse cursor to the specified `state`.  This is used when
    /// drawing the specified mouse cursor image.  The mouse will automatically inherit
    /// `Normal` and `Pressed` states by default.  This overrides that behavior.
    pub fn set_mouse_state(&mut self, state: AnimState) {
        self.mouse_anim_state = state;
    }

    /// Adds a gap between the previous widget and the next to be specified, subject
    /// to the current parent's layout requirement.
    pub fn gap(&mut self, gap: f32) {
        self.widgets[self.parent_index].gap(gap);
    }

    /// Sets the current cursor position of the current parent widget to the specified value.
    /// Normally, the cursor widget moves after each widget is placed based on the parent's
    /// [`layout`](struct.WidgetBuilder.html#method.layout).
    /// This has nothing to do with the mouse cursor.
    pub fn set_cursor(&mut self, x: f32, y: f32) {
        self.widgets[self.parent_index].set_cursor(x, y);
    }

    /// Returns the current cursor position of the parent widget.  You can use this as a basis
    /// for relative changes with [`set_cursor`](#method.set_cursor).
    /// This has nothing to do with the mouse cursor.
    pub fn cursor(&self) -> Point { self.widgets[self.parent_index].cursor() }

    /// Causes Thyme to focus the keyboard on the widget with the specified `id`.  Keyboard
    /// events will subsequently be sent to this widget, if it exists.  Only
    /// one widget may have keyboard focus at a time.
    /// # Example
    /// ```
    /// fn open_query_popup(ui: &mut Frame) {
    ///     ui.open("query_popup");
    ///     ui.focus_keyboard("query_popup_input_field");  
    /// }
    /// ```
    pub fn focus_keyboard<T: Into<String>>(&mut self, id: T) {
        let mut context = self.context.internal().borrow_mut();
        context.set_focus_keyboard(id.into());
    }

    /// Returns whether or not the widget with the specified `id` currently has keyboard focus.
    /// See [`focus_keyboard`](#method.focus_keyboard).
    pub fn is_focus_keyboard(&self, id: &str) -> bool {
        let context = self.context.internal().borrow();
        context.is_focus_keyboard(id)
    }

    /// Returns a [`Rect](struct.Rect.html) with the current size and position of the
    /// current parent widget.  (This is the widget that any currently created
    /// widgets will be added as a child of).  Note that the size of the parent
    /// might change later depending on the layout choice.
    pub fn parent_bounds(&self) -> Rect { self.max_child_bounds }

    /// Returns a [`Rect`](struct.Rect.html) encompassing all children that have currently
    /// been added to the parent widget, recursively.  This includes each widget's actual
    /// final position and size.
    pub fn parent_max_child_bounds(&self) -> Rect { self.parent_max_child_bounds }

    /**
    Returns the current internal time being used by Thyme.  This is useful
    if you want to set a timer to start running based on the current frame,
    using [`set_base_time_millis`](#method.set_base_time_millis).

    # Example
    ```
    fn set_timer(ui: &mut Frame) {
        // widget will reach its zero animation time in 10 seconds
        let time = ui.cur_time_millis();
        ui.set_base_time_millis("my_timer_widget", time + 10_000);
    }
    ```

    */
    
    pub fn cur_time_millis(&self) -> u32 {
        let context = self.context.internal().borrow();
        context.time_millis()
    }

    /// Sets the base time of the [`PersistentState`](struct.PersistentState.html) for the widget with the
    /// specified `id` to the specified `time`.
    /// This time should probably be based on something obtained from [`cur_time_millis`](#method.cur_time_millis)
    /// or [`base_time_millis`](#method.base_time_millis).  The base time of a widget is used to specify the
    /// zero time of an Timed images associated with that widget.
    pub fn set_base_time_millis<T: Into<String>>(&mut self, id: T, time: u32) {
        let mut context = self.context.internal().borrow_mut();
        let state = context.state_mut(id);
        state.base_time_millis = time;
    }

    /// Sets the base time of the [`PersistentState`](struct.PersistentState.html) for the widget with the
    /// specified `id` to the current internal time.
    /// See [`set_base_time_millis`](#method.set_base_time_millis).
    pub fn set_base_time_now<T: Into<String>>(&mut self, id: T) {
        let mut context = self.context.internal().borrow_mut();
        let cur_time = context.time_millis();
        let state = context.state_mut(id);
        state.base_time_millis = cur_time;
    }

    /// Returns the current base time in millis of the [`PersistentState`](struct.PersistentState.html) for the
    /// widget with the current `id`.
    pub fn base_time_millis(&self, id: &str) -> u32 {
        let context = self.context.internal().borrow();
        context.state(id).base_time_millis
    }
    
    /// Sets the internal `scroll` of the [`PersistentState`](struct.PersistentState.html) for
    /// the widget with the specified `id`.  Useful for [`Scrollpanes`](struct.WidgetBuilder.html#method.scrollpane).
    pub fn scroll(&self, id: &str) -> Point {
        let context = self.context.internal().borrow();
        context.state(id).scroll
    }

    /// Modifies the internal `scroll` of the widget with the specified `id` by the specified `x` and `y` amounts.
    /// See [`scroll`](#method.scroll)
    pub fn change_scroll<T: Into<String>>(&mut self, id: T, x: f32, y: f32) {
        let mut context = self.context.internal().borrow_mut();
        let state = context.state_mut(id);
        state.scroll = state.scroll + Point { x, y }
    }

    /// Returns the current `text` associated with the [`PersistentState`](struct.PersistentState.html) of
    /// the widget with the specified `id`.  Useful for [`input fields`](#method.input_field).
    pub fn text_for(&self, id: &str) -> Option<String> {
        let context = self.context.internal().borrow();
        context.state(id).text.clone()
    }

    /// Returns whether the widget with the specified `id` is expanded in its [`PersistentState`](struct.PersistentState.html).
    /// Trees and similar widgets will not show their entire content if not expanded
    pub fn is_expanded(&self, id: &str) -> bool {
        let context = self.context.internal().borrow();
        context.state(id).expanded
    }

    /// Sets the expanded value for the given widget to `expanded`.  See [`is_expanded`](#method.is_expanded)
    pub fn set_expanded<T: Into<String>>(&mut self, id: T, expanded: bool) {
        let mut context = self.context.internal().borrow_mut();
        context.state_mut(id).expanded = expanded;
    }

    /// Returns whether the widget with the specified `id` is open in its [`PersistentState`](struct.PersistentState.html).
    /// If not open, widgets are not visible.
    pub fn is_open(&self, id: &str) -> bool {
        let context = self.context.internal().borrow();
        context.state(id).is_open
    }

    /// Opens the widget with the specified `id` as a modal.  This modifies the [`PersistentState`](struct.PersistentState.html)
    /// associated with that widget, as well as setting the overall Thyme modal to the specified widget.
    /// When a modal is open, only the modal and its children may receive input.  There may be only one modal open at a time.
    /// If the specified `id` is closed, i.e. via [`close`](#method.close), the modal state ends.
    pub fn open_modal<T: Into<String>>(&mut self, id: T) {
        let id = id.into();

        let mut context = self.context.internal().borrow_mut();
        context.set_top_rend_group_id(&id);
        context.state_mut(id.clone()).is_open = true;
        context.set_modal(id);
    }

    /// Sets the currently open modal, if there is one, to close if the mouse is clicked outside of the modal's area.
    pub fn close_modal_on_click_outside(&mut self) {
        let mut context = self.context.internal().borrow_mut();
        context.mut_modal(|modal| {
            modal.close_on_click_outside = true;
        });
    }

    /// Opens the widget with the specified `id`.  This modifies the [`PersistentState`](struct.PersistentState.html).
    /// See [`is_open`](#method.is_open)
    pub fn open<T: Into<String>>(&mut self, id: T) {
        let id = id.into();
        let mut context = self.context.internal().borrow_mut();
        context.set_top_rend_group_id(&id);
        context.state_mut(id).is_open = true;
    }

    /// Closes the widget with the specified `id`.  This modifies the [`PersistentState`](struct.PersistentState.html).
    /// See [`is_open`](#method.is_open).  If the widget was the current modal, resets Thyme so there is no longer a modal.
    pub fn close<T: Into<String>>(&mut self, id: T) {
        let id = id.into();

        let mut context = self.context.internal().borrow_mut();
        context.clear_modal_if_match(&id);
        context.state_mut(id).is_open = false;
    }

    /// Opens the current parent widget.  See [`open`](#method.open).
    pub fn open_parent(&mut self) {
        let mut context = self.context.internal().borrow_mut();
        let id = self.widgets[self.parent_index].id();
        context.set_top_rend_group_id(&id);
        context.state_mut(id).is_open = true;
    }

    /// Closes the current parent widget.  See [`close`](#method.close).
    pub fn close_parent(&mut self) {
        let mut context = self.context.internal().borrow_mut();
        let id = self.widgets[self.parent_index].id();
        context.clear_modal_if_match(id);
        context.state_mut(id).is_open = false;
    }

    /// Completely clears all [`PersistentState`](struct.PersistentState.html) associated with the 
    /// specified `id`, resetting it to its default state.
    /// This includies clearing the modal state if the `id` is the current modal.
    pub fn clear(&mut self, id: &str) {
        let mut context = self.context.internal().borrow_mut();
        context.clear_modal_if_match(id);
        context.clear_state(id);
    }

    /// Gets a mutable reference to the [`PersistentState`](struct.PersistentState.html) associated with
    /// the `id`, and calls the passed in closure, `f`, allowing you to modify it in arbitrary ways.  This
    /// is more efficient than calling several individual methods in a row, such as [`open`](#method.open),
    /// [`scroll`](#method.scroll), etc.  The return value of the passed in function is passed through
    /// this method, allowing you to use it for queries as well.
    pub fn modify<T: Into<String>, Ret, F: FnOnce(&mut PersistentState) -> Ret>(&mut self, id: T, f: F) -> Ret{
        let mut context = self.context.internal().borrow_mut();
        (f)(context.state_mut(id))
    }

    /// Queries the theme for the specified custom int, in the `custom` field for the
    /// theme with the specified `key`.  Returns the `default_value` if the theme or key cannot
    /// be found, or if the key is specified but is not a float
    pub fn custom_int(&self, theme_id: &str, key: &str, default_value: i32) -> i32 {
        let context = self.context_internal().borrow();

        let value = match context.themes().theme(theme_id) {
            None => return default_value,
            Some(theme) => theme.custom.get(key),
        };

        if let Some(CustomData::Int(value)) = value {
            *value
        } else {
            default_value
        }
    }

    /// Queries the theme for the specified custom float, in the `custom` field for the
    /// theme with the specified `key`.  Returns the `default_value` if the theme or key cannot
    /// be found, or if the key is specified but is not a float
    pub fn custom_float(&self, theme_id: &str, key: &str, default_value: f32) -> f32 {
        let context = self.context_internal().borrow();

        let value = match context.themes().theme(theme_id) {
            None => return default_value,
            Some(theme) => theme.custom.get(key),
        };

        if let Some(CustomData::Float(value)) = value {
            *value
        } else {
            default_value
        }
    }

    /// Queries the theme for the specified custom String, in the `custom` field for the
    /// theme with the specified `key`.  Returns the `default_value` if the theme or key
    /// cannot be found, or if the key is specified but is not a String
    pub fn custom_string(&self, theme_id: &str, key: &str, default_value: String) -> String {
        let context = self.context_internal().borrow();

        let value = match context.themes().theme(theme_id) {
            None => return default_value,
            Some(theme) => theme.custom.get(key),
        };

        if let Some(CustomData::String(value)) = value {
            value.clone()
        } else {
            default_value
        }
    }

    /// Logs a message using the Thyme internal logger.  Prevents a flood of the same message
    /// from appearing on each frame - the message will only appear once in the log output.
    pub fn log<T: Into<String>>(&self, level: log::Level, message: T) {
        let mut context = self.context_internal().borrow_mut();
        context.log(level, message.into());
    }

    pub(crate) fn push_widget(&mut self, mut widget: Widget) {
        widget.set_rend_group(self.cur_rend_group);
        self.render_groups[self.cur_rend_group.index as usize].num += 1;
        self.widgets.push(widget);
    }

    pub(crate) fn cur_render_group(&self) -> RendGroup { self.cur_rend_group }

    pub(crate) fn prev_render_group(&mut self, group: RendGroup) {
        self.cur_rend_group = group;
    }

    pub(crate) fn next_render_group(&mut self, rect: Rect, id: String, always_top: bool) {
        let widgets_len = self.widgets.len();
        let index = self.render_groups.len() as u16;
        let cur_rend_group = RendGroup { index };

        self.render_groups.push(RendGroupDef {
            rect,
            id,
            group: cur_rend_group,
            start: widgets_len,
            num: 0,
            always_top,
        });
        self.cur_rend_group = cur_rend_group;
    }

    pub(crate) fn rebound_cur_render_group(&mut self, bounds: Rect) {
        self.render_groups[self.cur_rend_group.index as usize].rect = bounds;
    }

    pub(crate) fn finish_frame(self) -> (Context, Vec<Widget>, Vec<RendGroupDef>) {
        let (top_rend_group, mouse_pos) = {
            let mut context = self.context.internal().borrow_mut();

            context.check_set_rend_group_top(&self.render_groups);

            (context.top_rend_group(), context.mouse_pos())
        };

        let mut render_groups = self.render_groups;
        render_groups.sort_by_key(|group| {
            if group.always_top {
                -1
            } else if group.group == top_rend_group {
                0
            } else {
                1
            }
        });

        let mut mouse_in_rend_group = None;
        for rend_group in render_groups.iter() {
            if rend_group.rect.is_inside(mouse_pos) {
                mouse_in_rend_group = Some(rend_group.group);
                break;
            }
        }

        self.context.internal().borrow_mut().next_frame(self.mouse_taken, mouse_in_rend_group);

        (self.context, self.widgets, render_groups)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub(crate) struct RendGroup {
    index: u16,
}

#[derive(Debug)]
pub(crate) struct RendGroupDef {
    rect: Rect,
    id: String,
    group: RendGroup,
    start: usize,
    num: usize,
    always_top: bool,
}

impl RendGroupDef {
    pub(crate) fn iter<'a, 'b>(&'a self, widgets: &'b [Widget]) -> impl Iterator<Item=&'b Widget> {
        let group = self.group;
        widgets.iter().skip(self.start).filter(move |widget| widget.rend_group() == group).take(self.num + 1)
    }

    pub(crate) fn id(&self) -> &str { &self.id }
    pub(crate) fn group(&self) -> RendGroup { self.group }
}
