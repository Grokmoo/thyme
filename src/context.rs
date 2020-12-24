use std::collections::{HashMap, HashSet};
use std::cell::RefCell;
use std::rc::Rc;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::{Error, Point, Frame, Rect, frame::{RendGroup, RendGroupDef}};
use crate::{font::FontSummary, widget::Widget, image::ImageHandle, theme::ThemeSet, resource::ResourceSet};
use crate::theme_definition::{AnimState, AnimStateKey};
use crate::render::Renderer;

#[derive(Copy, Clone)]
pub(crate) struct PersistentStateData {
    pub is_open: bool,
    pub expanded: bool,
    pub resize: Point,
    pub moved: Point,
    pub scroll: Point,
}

/**
The internal state stored by Thyme for a given Widget that
persists between frames.

Note that Thyme will generally be able to automatically generate
unique IDs for many widgets such as buttons.  But, if you want to
access this data for a particular widget you will need to specify
a known ID for that widget.

# Example
```
fn reset_window_state(ui: &mut Frame, window_id: &str) {
    ui.modify(window_id, |state| {
        state.resize = Point::default();
        state.moved = Point::default();
        state.is_open = true;
    });
}
```
*/
#[derive(Debug)]
pub struct PersistentState {
    /// Whether the widget will be shown.  Defaults to true.
    pub is_open: bool,

    /// Whether a tree or similar widget is expanded, showing all children, or not
    pub expanded: bool,

    /// An amount, in logical pixels that the widget has been resized by.  Default to zero.
    pub resize: Point,

    /// An amount, in logical pizels that the widget has been moved by.  Defaults to zero.
    pub moved: Point,

    /// An amount, in logical pixels that the internal content has been
    /// scrolled by.  Defaults to zero.
    pub scroll: Point,

    /// The "zero" time for timed images associated with this widget.  Defaults to zero,
    /// which is the internal [`Context`](struct.Context.html) init time.
    pub base_time_millis: u32,

    /// Any characters that have been sent to this widget from the keyboard.  Defaults to
    /// empty.  Widgets should typically drain this list as they work with input.
    pub characters: Vec<char>,

    /// The text for this widget, overriding default text.  Defaults to `None`.
    pub text: Option<String>,
}

impl PersistentState {
    pub(crate) fn copy_data(&self) -> PersistentStateData {
        PersistentStateData {
            is_open: self.is_open,
            expanded: self.expanded,
            resize: self.resize,
            moved: self.moved,
            scroll: self.scroll,
        }
    }
}

impl Default for PersistentState {
    fn default() -> Self {
        PersistentState {
            is_open: true,
            expanded: true,
            resize: Point::default(),
            moved: Point::default(),
            scroll: Point::default(),
            base_time_millis: 0,
            characters: Vec::default(),
            text: None,
        }
    }
}

/// The current state of the various keyboard modifier keys - Shift, Control, and Alt
/// You can get this using [`Frame.input_modiifers`](struct.Frame.html#method.input_modifiers)
#[derive(Default, Copy, Clone)]
pub struct InputModifiers {
    /// whether the Shift key is pressed
    pub shift: bool,

    /// whether the Control key is pressed
    pub ctrl: bool,

    /// Whether the Alt key is pressed
    pub alt: bool,
}

pub struct ContextInternal {
    resources: ResourceSet,
    themes: ThemeSet,
    frame_active: bool,

    mouse_taken_last_frame: Option<(String, RendGroup)>,
    mouse_in_rend_group_last_frame: Option<RendGroup>,
    top_rend_group: RendGroup,
    check_set_top_rend_group: Option<String>,

    modal: Option<Modal>,

    mouse_pressed_outside: [bool; 3],

    keyboard_focus_widget: Option<String>,
    persistent_state: HashMap<String, PersistentState>,
    empty_persistent_state: PersistentState,

    input_modifiers: InputModifiers,
    last_mouse_pos: Point,
    mouse_pos: Point,
    mouse_pressed: [bool; 3],
    mouse_clicked: [bool; 3],
    mouse_wheel: Point,

    display_size: Point,
    scale_factor: f32,

    start_instant: Instant,
    time_millis: u32,

    errors: HashSet<String>,
}

impl ContextInternal {
    pub(crate) fn log(&mut self, level: log::Level, error: String) {
        if self.errors.contains(&error) { return; }

        log::log!(level, "{}", error);
        self.errors.insert(error);
    }

    pub(crate) fn mut_modal<F: FnOnce(&mut Modal)>(&mut self, f: F) {
        if let Some(modal) = self.modal.as_mut() {
            (f)(modal);
        }
    }

    pub(crate) fn modal_id(&self) -> Option<&str> {
        self.modal.as_ref().map(|modal| modal.id.as_ref())
    }

    pub(crate) fn has_modal(&self) -> bool {
        self.modal.is_some()
    }

    pub(crate) fn clear_modal_if_match(&mut self, id: &str) {
        if self.modal_id() == Some(id) {
            self.modal.take();
        }
    }

    pub(crate) fn set_modal(&mut self, id: String) {
        self.modal = Some(Modal::new(id));
    }

    pub(crate) fn mouse_in_rend_group_last_frame(&self) -> Option<RendGroup> {
        self.mouse_in_rend_group_last_frame
    }

    pub(crate) fn set_top_rend_group(&mut self, group: RendGroup) {
        self.top_rend_group = group;
    }

    pub(crate) fn top_rend_group(&self) -> RendGroup { self.top_rend_group }

    pub(crate) fn set_top_rend_group_id(&mut self, id: &str) {
        self.check_set_top_rend_group = Some(id.to_string());
    }

    pub(crate) fn check_set_rend_group_top(&mut self, groups: &[RendGroupDef]) {
        let id = match &self.check_set_top_rend_group {
            None => return,
            Some(id) => id,
        };

        for group in groups {
            if group.id() == id {
                self.top_rend_group = group.group();
                self.check_set_top_rend_group = None;
                break;
            }
        }
    }

    pub(crate) fn base_time_millis_for(&self, id: &str) -> u32 {
        self.persistent_state.get(id).map_or(0, |state| state.base_time_millis)
    }

    pub(crate) fn time_millis(&self) -> u32 { self.time_millis }
    pub(crate) fn mouse_pos(&self) -> Point { self.mouse_pos }
    pub(crate) fn last_mouse_pos(&self) -> Point { self.last_mouse_pos }
    pub(crate) fn mouse_pressed(&self, index: usize) -> bool { self.mouse_pressed[index] }
    pub(crate) fn mouse_clicked(&self, index: usize) -> bool { self.mouse_clicked[index] }

    pub (crate) fn set_focus_keyboard(&mut self, id: String) {
        self.keyboard_focus_widget = Some(id);
    }

    pub (crate) fn is_focus_keyboard(&self, id: &str) -> bool {
        self.keyboard_focus_widget.as_deref() == Some(id)
    }

    pub(crate) fn take_mouse_wheel(&mut self) -> Point {
        let result = self.mouse_wheel;
        self.mouse_wheel = Point::default();
        result
    }

    pub(crate) fn mouse_taken_last_frame_id(&self) -> Option<&str> {
        self.mouse_taken_last_frame.as_ref().map(|(id, _)| id.as_ref())
    }

    pub(crate) fn scale_factor(&self) -> f32 { self.scale_factor }
    pub(crate) fn display_size(&self) -> Point { self.display_size }

    pub(crate) fn themes(&self) -> &ThemeSet { &self.themes }

    pub(crate) fn init_state<T: Into<String>>(&mut self, id: T, open: bool, expanded: bool) {
        self.persistent_state.entry(id.into()).or_insert(
            PersistentState {
                is_open: open,
                expanded,
                ..Default::default()
            }
        );
    }

    pub(crate) fn clear_state(&mut self, id: &str) {
        self.persistent_state.remove(id);
    }

    pub(crate) fn state(&self, id: &str) -> &PersistentState {
        match self.persistent_state.get(id) {
            None => &self.empty_persistent_state,
            Some(state) => state,
        }
    }

    pub(crate) fn state_mut<T: Into<String>>(&mut self, id: T) -> &mut PersistentState {
        self.persistent_state.entry(id.into()).or_default()
    }

    pub(crate) fn mouse_pressed_outside(&self) -> bool {
        for pressed in self.mouse_pressed_outside.iter() {
            if *pressed { return true; }
        }
        false
    }

    pub(crate) fn input_modifiers(&self) -> InputModifiers {
        self.input_modifiers
    }

    pub(crate) fn next_frame(&mut self, mouse_taken: Option<(String, RendGroup)>, mouse_in_rend_group: Option<RendGroup>) {
        let mut clear_modal = false;
        if let Some(modal) = self.modal.as_mut() {
            if modal.prevent_close {
                modal.prevent_close = false;
            } else if modal.close_on_click_outside && self.mouse_clicked[0] && !modal.bounds.is_inside(self.mouse_pos) {
                clear_modal = true;
            }
        }

        if clear_modal {
            let modal = self.modal.take().unwrap();
            self.state_mut(modal.id).is_open = false;
        }

        self.mouse_wheel = Point::default();
        self.mouse_clicked = [false; 3];
        self.mouse_taken_last_frame = mouse_taken;
        self.last_mouse_pos = self.mouse_pos;
        self.mouse_in_rend_group_last_frame = mouse_in_rend_group;
        self.frame_active = false;
    }
}

/**
The main Thyme Context that holds internal [`PersistentState`](struct.PersistentState.html)
and is responsible for creating [`Frames`](struct.Frame.html).

This is created by [`build`](struct.ContextBuilder.html#method.build) on
[`ContextBuilder`](struct.ContextBuilder.html) after resource registration is complete.
**/
pub struct Context {
    internal: Rc<RefCell<ContextInternal>>,
}

impl Context {
    pub(crate) fn new(
        resources: ResourceSet,
        themes: ThemeSet,
        display_size: Point,
        scale_factor: f32
    ) -> Context {
        let internal = ContextInternal {
            resources,
            display_size,
            scale_factor,
            themes,
            persistent_state: HashMap::new(),
            empty_persistent_state: PersistentState::default(),
            mouse_pos: Point::default(),
            last_mouse_pos: Point::default(),
            input_modifiers: InputModifiers::default(),
            mouse_pressed: [false; 3],
            mouse_clicked: [false; 3],
            mouse_wheel: Point::default(),
            mouse_taken_last_frame: None,
            mouse_in_rend_group_last_frame: None,
            top_rend_group: RendGroup::default(),
            check_set_top_rend_group: None,
            mouse_pressed_outside: [false; 3],
            modal: None,
            time_millis: 0,
            start_instant: Instant::now(),
            keyboard_focus_widget: None,
            errors: HashSet::new(),
            frame_active: false,
        };

        Context {
            internal: Rc::new(RefCell::new(internal))
        }
    }

    // Finds the specified font and appropriately logs any error in this context.
    pub(crate) fn find_font(&self, id: &str) -> Option<FontSummary> {
        let mut internal = self.internal.borrow_mut();
        match internal.themes().find_font(Some(id)) {
            None => {
                internal.log(log::Level::Error, format!("Unable to find font '{}' for widget", id));
                None
            }, Some(handle) => Some(handle)
        }
    }

    // Finds the specified image and appropriately logs any error in this context.
    pub(crate) fn find_image(&self, id: &str) -> Option<ImageHandle> {
        let mut internal = self.internal.borrow_mut();
        match internal.themes().find_image(Some(id)) {
            None => {
                internal.log(log::Level::Error, format!("Unable to find image '{}' for widget", id));
                None
            }, Some(handle) => Some(handle),
        }
    }

    /// Returns true if thyme wants to use the mouse in the current frame, generally
    /// because the mouse is over a Thyme widget.  If this returns true, you probably
    /// want Thyme to handle input this frame, while if it returns false, your application
    /// or game logic should handle input.
    pub fn wants_mouse(&self) -> bool {
        let internal = self.internal.borrow();
        internal.mouse_taken_last_frame.is_some() || internal.modal.is_some()
    }

    /// Returns true if thyme wants to use keyboard input in the current frame, generally
    /// because a widget that accepts text input is keyboard focused.  If this returns true,
    /// you probably don't want to handle keyboard events in your own application code.
    pub fn wants_keyboard(&self) -> bool {
        let internal = self.internal.borrow();
        internal.modal.is_some() || internal.keyboard_focus_widget.is_some()
    }

    pub(crate) fn internal(&self) -> &Rc<RefCell<ContextInternal>> {
        &self.internal
    }

    /// Change the scale factor
    pub fn set_scale_factor(&mut self, scale: f32) {
        let mut internal = self.internal.borrow_mut();
        internal.scale_factor = scale;
    }

    /// Set the display size
    pub fn set_display_size(&mut self, size: Point) {
        let mut internal = self.internal.borrow_mut();
        internal.display_size = size;
    }

    /// Add mouse wheel event.
    pub fn add_mouse_wheel(&mut self, delta: Point) {
        let mut internal = self.internal.borrow_mut();

        internal.mouse_wheel = internal.mouse_wheel + delta;
    }

    /// Set the input modifiers. You should call this per frame.
    pub fn set_input_modifiers(&mut self, input_modifiers: InputModifiers) {
        let mut internal = self.internal.borrow_mut();
        internal.input_modifiers = input_modifiers;
    }

    /// Set the mouse pressed state.
    /// # Inputs:
    /// - button `pressed` state
    /// - index: 0 = LeftClick, 1 = Right Click, 2 = Middle Click
    pub fn set_mouse_pressed(&mut self, pressed: bool, index: usize) {
        let mut internal = self.internal.borrow_mut();

        if index >= internal.mouse_pressed.len() {
            return;
        }

        // don't take a mouse press that started outside the GUI elements
        if pressed && internal.mouse_taken_last_frame.is_none() {
            internal.mouse_pressed_outside[index] = true;
        }

        if !pressed && internal.mouse_pressed_outside[index] {
            internal.mouse_pressed_outside[index] = false;
        }

        if internal.mouse_pressed[index] && !pressed {
            internal.mouse_clicked[index] = true;
            internal.keyboard_focus_widget = None;
        }

        internal.mouse_pressed[index] = pressed;
    }

    /// Push a character
    pub fn push_character(&mut self, c: char) {
        let mut internal = self.internal.borrow_mut();

        let id = match &internal.keyboard_focus_widget {
            Some(id) => id.to_string(),
            None => return,
        };

        let state = internal.state_mut(id);
        state.characters.push(c);
    }

    /// Set mouse position. 
    /// You need to take into account the scale factor when setting this. (see `demo_glium.rs`).
    pub fn set_mouse_pos(&mut self, pos: Point) {
        let mut internal = self.internal.borrow_mut();
        internal.mouse_pos = pos;
    }

    /// Adds the specified path as a source file for the resources being used
    /// by the theme for this context.  This will only work if the theme was
    /// set up to read source data from files, i.e. using
    /// [`ContextBuilder#register_theme_from_files`](struct.ContextBuilder.html#method.register_theme_from_files)
    /// This does not rebuild the theme; you will
    /// need to call [`rebuild_all`](#method.rebuild_all) for that.
    pub fn add_theme_file<P: Into<PathBuf>>(&mut self, path: P) {
        let path = path.into();
        let mut internal = self.internal.borrow_mut();
        internal.resources.add_theme_file(path);
    }

    /// Removes the theme source file with the specified path from the resources
    /// being used by the theme for this context, if it is present.  If it is not
    /// present, does nothing.  This does not rebuild the theme; you will
    /// need to call [`rebuild_all`](#method.rebuild_all) for that.
    pub fn remove_theme_file<P: Into<PathBuf>>(&mut self, path: P) {
        let path: &Path = &path.into();
        let mut internal = self.internal.borrow_mut();
        internal.resources.remove_theme_file(path);
    }

    /// Rebuilds this context, reloading all asset data.  Notably, files on disk
    /// that were used in [`building`](struct.ContextBuilder.html) the context
    /// are re-read.  If any errors are encountered in reading or parsing files, this
    /// will return `Err` and no  changes are made to the context.
    pub fn rebuild_all<R: Renderer>(&mut self, renderer: &mut R) -> Result<(), Error> {
        let mut internal = self.internal.borrow_mut();
        internal.resources.clear_data_cache();
        internal.resources.cache_data()?;

        let scale_factor = internal.scale_factor;
        let themes = internal.resources.build_assets(renderer, scale_factor)?;
        internal.themes = themes;
        Ok(())
    }

    /// Checks the internal live reload thread to see if any file notifications have occurred
    /// since the last check.  If so, will fully rebuild the theme.  If any errors are encountered
    /// in the process of rebuilding the theme, will return the `Err` and no changes are made to
    /// the current theme.  Note that if you built the context with live reload disabled
    /// (see [`BuildOptions`](struct.BuildOptions.html)), this function will do nothing.
    pub fn check_live_reload<R: Renderer>(&mut self, renderer: &mut R) -> Result<(), Error> {
        let mut internal = self.internal.borrow_mut();
        let scale_factor = internal.scale_factor;

        let themes = internal.resources.check_live_reload(renderer, scale_factor)?;

        if let Some(themes) = themes {
            internal.themes = themes;
        }

        Ok(())
    }

    /// Creates a [`Frame`](struct.Frame.html), the main object that should pass through
    /// your UI building functions and is responsible for constructing the widget tree.
    /// This method should be called each frame you want to draw / interact with the UI.
    pub fn create_frame(&mut self) -> Frame {
        let now = Instant::now();

        let anim_state;
        let display_size = {
            let mut context = self.internal.borrow_mut();

            if context.frame_active {
                panic!("A Thyme Frame is already active but a new one has been requested.");
            }

            context.frame_active = true;

            let elapsed = (now - context.start_instant).as_millis() as u32;
            context.time_millis = elapsed;

            if context.mouse_pressed[0] {
                anim_state = AnimState::new(AnimStateKey::Pressed);
            } else {
                anim_state = AnimState::normal();
            }

            context.display_size() / context.scale_factor()
        };

        let context = Context { internal: Rc::clone(&self.internal) };

        let root = Widget::root(display_size);
        Frame::new(context, root, anim_state)
    }
}

pub(crate) struct Modal {
    pub(crate) id: String,
    pub(crate) close_on_click_outside: bool,
    pub(crate) bounds: Rect,
    pub(crate) prevent_close: bool,
}

impl Modal {
    fn new(id: String) -> Modal {
        Modal {
            id,
            close_on_click_outside: false,
            bounds: Rect::default(),
            prevent_close: true,
        }
    }
}