use std::collections::{HashMap, hash_map::Entry};
use std::fmt;

use serde::{Serialize, Deserialize, Deserializer, Serializer, de::{self, Visitor}};

use crate::{Border, Point};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeDefinition {
    #[serde(default)]
    pub fonts: HashMap<String, FontDefinition>,

    #[serde(default)]
    pub image_sets: HashMap<String, ImageSet>,

    #[serde(default)]
    pub widgets: HashMap<String, WidgetThemeDefinition>,
}

impl ThemeDefinition {
    /// Merges the specified `other` theme definition into this one
    pub fn merge(&mut self, other: ThemeDefinition) {
        use Entry::*;

        for (id, font) in other.fonts {
            match self.fonts.entry(id) {
                Occupied(mut entry) => {
                    log::warn!("Overwriting font id '{}'", entry.key());
                    entry.insert(font);
                },
                Vacant(entry) => { entry.insert(font); }
            }
        }

        for (id, image) in other.image_sets {
            match self.image_sets.entry(id) {
                Occupied(mut entry) => {
                    log::warn!("Overwriting image set id '{}'", entry.key());
                    entry.insert(image);
                }, Vacant(entry) => { entry.insert(image); }
            }
        }

        for (id, widget) in other.widgets {
            match self.widgets.entry(id) {
                Occupied(mut entry) => {
                    log::warn!("Overwriting widget theme id '{}'", entry.key());
                    entry.insert(widget);
                }, Vacant(entry) => { entry.insert(widget); }
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WidgetThemeDefinition {
    pub from: Option<String>,

    pub text: Option<String>,
    pub font: Option<String>,
    pub background: Option<String>,
    pub foreground: Option<String>,
    pub tooltip: Option<String>,

    // all fields are options instead of using default so
    // we can detect when to override them
    pub text_color: Option<Color>,
    pub wants_mouse: Option<bool>,
    pub wants_scroll: Option<bool>,
    pub text_align: Option<Align>,
    pub pos: Option<Point>,
    pub screen_pos: Option<Point>,
    pub size: Option<Point>,
    pub size_from: Option<(WidthRelative, HeightRelative)>,
    pub width_from: Option<WidthRelative>,
    pub height_from: Option<HeightRelative>,
    pub border: Option<Border>,
    pub align: Option<Align>,
    pub child_align: Option<Align>,
    pub layout: Option<Layout>,
    pub layout_spacing: Option<Point>,

    #[serde(default)]
    pub custom: HashMap<String, CustomData>,

    #[serde(default)]
    pub children: HashMap<String, WidgetThemeDefinition>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields, untagged)]
pub enum CustomData {
    Float(f32),
    Int(i32),
    String(String),
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageSet {
    pub source: Option<String>,

    #[serde(default="f32_one")]
    pub scale: f32,

    pub images: HashMap<String, ImageDefinition>,
}

fn f32_one() -> f32 { 1.0 }

#[derive(Serialize, Deserialize, Clone)]
pub struct ImageDefinition {
    #[serde(default)]
    pub color: Color,

    #[serde(flatten)]
    pub kind: ImageDefinitionKind,
}

#[derive(Serialize, Deserialize, Copy, Clone)]
pub enum ImageFill {
    None,
    Stretch,
    Repeat,
}

impl Default for ImageFill {
    fn default() -> Self {
        ImageFill::None
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct CollectedSubImage {
    pub position: [i32; 2],
    pub size: [i32; 2],
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
#[serde(deny_unknown_fields)]
pub enum ImageDefinitionKind {
    Empty,
    Solid {
        solid: bool,
    },
    Group {
        group_scale: [u32; 2],
        fill: ImageFill,
        images: HashMap<String, [u32; 4]>,
    },
    Alias {
        from: String,
    },
    Collected {
        sub_images: HashMap<String, CollectedSubImage>,
    },
    Composed {
        position: [u32; 2],
        grid_size: [u32; 2],
    },
    ComposedVertical {
        position: [u32; 2],
        grid_size_vert: [u32; 2],
    },
    ComposedHorizontal {
        position: [u32; 2],
        grid_size_horiz: [u32; 2],
    },
    Simple {
        position: [u32; 2],
        size: [u32; 2],

        #[serde(default)]
        fill: ImageFill,
    },
    Timed {
        frame_time_millis: u32,
        frames: Vec<String>,

        #[serde(default)]
        once: bool,
    },
    Animated {
        states: HashMap<AnimState, String>,
    }
}

/// An `AnimState` consists of one or more (currently up to four) state keys,
/// with each key representing a different state.
/// 
/// For example, a state
/// could be [`Active`](enum.AnimStateKey.html#active) + [`Pressed`](enum.AnimStateKey.html#pressed)
/// or [`Hover`](enum.AnimStateKey#hover).
/// `AnimState`s are parsed from the theme file as strings in this format, i.e.
/// `Active + Pressed`, `Normal`, `Hover`, are all valid.  The `+` character is used
/// to concatenate multiple states, and whitespace is ignored.  The [`Normal`](enum.AnimStateKey.html#normal)
/// key is special and can only be present by itself.
/// `AnimState`s are used in Animated images in order to pick a particular image from a set.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct AnimState {
    keys: [AnimStateKey; 4],
}

impl AnimState {
    /// Creates an AnimState with the two specified state keys.
    pub const fn with_two(state1: AnimStateKey, state2: AnimStateKey) -> AnimState {
        AnimState { keys: [
            state1, state2, AnimStateKey::Normal, AnimStateKey::Normal
        ]}
    }

    /// Creates an AnimState with the three specified state keys.
    pub const fn with_three(state1: AnimStateKey, state2: AnimStateKey, state3: AnimStateKey) -> AnimState {
        AnimState { keys: [
            state1, state2, state3, AnimStateKey::Normal
        ]}
    }

    /// Creates an AnimState with the four specified state keys.
    pub const fn with_four(state1: AnimStateKey, state2: AnimStateKey, state3: AnimStateKey, state4: AnimStateKey) -> AnimState {
        AnimState { keys: [
            state1, state2, state3, state4
        ]}
    }

    /// Creates an AnimState consisting of the single specified `state`.
    pub const fn new(state: AnimStateKey) -> AnimState {
        AnimState { keys: [state, AnimStateKey::Normal, AnimStateKey::Normal, AnimStateKey::Normal] }
    }

    /// Creates an AnimState corresponding to the Normal state with no changes
    pub const fn normal() -> AnimState {
        AnimState { keys: [AnimStateKey::Normal; 4] }
    }

    /// Creates an AnimState consisting of only the Pressed state.
    pub const fn pressed() -> AnimState {
        let mut keys = [AnimStateKey::Normal; 4];
        keys[0] = AnimStateKey::Pressed;
        AnimState { keys }
    }

    /// Creates an AnimState consisting of the Hover state.
    pub fn hover() -> AnimState {
        let mut keys = [AnimStateKey::Normal; 4];
        keys[0] = AnimStateKey::Hover;
        AnimState { keys }
    }

    /// Creates an AnimState consisting of only the Distabled state.
    pub const fn disabled() -> AnimState {
        let mut keys = [AnimStateKey::Normal; 4];
        keys[0] = AnimStateKey::Disabled;
        AnimState { keys }
    }

    /// Returns whether or not this `AnimState` contains the specified key.
    pub fn contains(&self, key: AnimStateKey) -> bool {
        for self_key in self.keys.iter() {
            if *self_key == key { return true; }
        }
        false
    }

    /// Adds the given state key to this `AnimState`.  Note that
    /// adding `Normal` will have no effect.
    pub fn add(&mut self, to_add: AnimStateKey) {
        for key in self.keys.iter_mut() {
            if *key == AnimStateKey::Normal {
                *key = to_add;
                break;
            }
        }

        self.keys.sort();
    }

    /// Removes the given state key from this `AnimState`, if it
    /// is present.  Otherwise does nothing.  Removing `Normal`
    /// will have no effect
    pub fn remove(&mut self, to_remove: AnimStateKey) {
        for key in self.keys.iter_mut() {
            if *key == to_remove {
                *key = AnimStateKey::Normal;
                break;
            }
        }
        self.keys.sort();
    }

    /// Sets the specified `key` as either present in this `AnimState`,
    /// or not, depending on `value`.
    pub fn set(&mut self, key: AnimStateKey, value: bool) {
        if value {
            if self.contains(key) { return; }
            self.add(key);
        } else {
            self.remove(key);
        }
    }
}

struct AnimStateVisitor;

impl<'de> Visitor<'de> for AnimStateVisitor {
    type Value = AnimState;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("A valid list of AnimStateKeys separated by '+'.  Whitespace is ignored.")
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        let mut keys = [AnimStateKey::Normal; 4];
        let mut normal_found = false;

        for (key_index, key_id) in value.split('+').enumerate() {
            if key_index >= keys.len() {
                return Err(E::custom(format!("Only a maximum of {} AnimStateKeys are allowed", keys.len())));
            }

            if normal_found {
                return Err(E::custom("Normal may only be specified as the sole AnimStateKey"));
            }

            let key_id = key_id.trim();
            match key_id {
                "Normal" => {
                    if key_index != 0 {
                        return Err(E::custom("Normal may only be specified as the sole AnimStateKey"));
                    }
                    normal_found = true;
                },
                "Hover" => {
                    add_if_not_already_present(&mut keys, key_index, AnimStateKey::Hover)?;
                },
                "Pressed" => {
                    add_if_not_already_present(&mut keys, key_index, AnimStateKey::Pressed)?;
                }
                "Disabled" => {
                    add_if_not_already_present(&mut keys, key_index, AnimStateKey::Disabled)?;
                },
                "Active" => {
                    add_if_not_already_present(&mut keys, key_index, AnimStateKey::Active)?;
                }
                _ => {
                    return Err(E::custom(format!("Unable to parse AnimStateKey from {}", key_id)));
                }
            }
        }

        keys.sort();

        Ok(AnimState { keys })
    }
}

fn add_if_not_already_present<E: de::Error>(keys: &mut [AnimStateKey; 4], max_index: usize, key: AnimStateKey) -> Result<(), E> {
    for other in keys.iter().copied().take(max_index) {
        if other == key {
            return Err(E::custom(format!("Duplicate AnimStateKey {:?}", key)));
        }
    }
    keys[max_index] = key;
    Ok(())
}

impl<'de> Deserialize<'de> for AnimState {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<AnimState, D::Error> {
        deserializer.deserialize_str(AnimStateVisitor)
    }
}

impl Serialize for AnimState {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut first = true;
        let mut val = String::new();
        for key in self.keys.iter() {
            if !first {
                val.push('+');
            }

            use AnimStateKey::*;
            match key {
                Normal => (),
                Hover => val.push_str("Hover"),
                Pressed => val.push_str("Pressed"),
                Disabled => val.push_str("Disabled"),
                Active => val.push_str("Active"),
            }

            first = false;
        }

        serializer.serialize_str(&val)
    }
}

/// One component of an [`AnimState`](struct.AnimState.html)
///
/// This represents the animation state of a widget.  Animated images
/// use this state to determine which image is used from a set of
/// available images.
#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
#[serde(deny_unknown_fields)]
pub enum AnimStateKey {
    /// The mouse is hovering over the widget
    Hover,

    /// The mouse is pressed on a widget
    Pressed,

    /// The widget is disabled
    Disabled,

    /// The widget has no special animation state.
    Normal,

    /// The widget is activated.
    Active,
}

/// The Layout direction for a widget's children.
///
/// This only has effect if the child widget does not manually specify an alignment.
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub enum Layout {
    /// Layout children horizontally, from left to right
    Horizontal,

    /// Layout children vertically, from top to bottom
    Vertical,

    /// Don't layout children in any order.  Children must specify manual alignments to
    /// avoid overlap.
    Free,
}

impl Default for Layout {
    fn default() -> Self { Layout::Horizontal }
}

/// Widget or text horizontal and vertical alignment.
///
/// `Left`, `Right`, and `Center` variants will center the element
/// vertically, while `Bot`, `Top`, and `Center` variants will
/// center the element horizontally.  The final position of a widget
/// is calculated based on the parent position and size, this alignment
/// and the child [`pos`](struct.WidgetBuilder.html#method.pos)
#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub enum Align {
    /// Center Left alignment
    Left,

    /// Center Right alignment
    Right,

    /// Bottom Center alignment
    Bot,

    /// Top Center alignment
    Top,

    /// Centered alignment on both axes
    Center,

    /// Bottom Left alignment
    BotLeft,

    /// Bottom Right alignment
    BotRight,

    /// Top Left alignment
    TopLeft,

    /// Top Right alignment
    TopRight,
}

impl Default for Align {
    fn default() -> Self { Align::TopLeft }
}

impl Align {
    /// Computes the position adjustment for this aligment within the given `size`.
    /// For example, [`TopLeft`](#variant.TopLeft) will return (0, 0), while
    /// [`BotRight`](#variant.BotRight) will return `size`.
    pub fn adjust_for(self, size: Point) -> Point {
        use Align::*;
        match self {
            Left => Point { x: 0.0, y: size.y / 2.0 },
            Right => Point { x: size.x, y: size.y / 2.0 },
            Bot => Point { x: size.x / 2.0, y: size.y },
            Top => Point { x: size.x / 2.0, y: 0.0 },
            Center => Point { x: size.x / 2.0, y: size.y / 2.0 },
            BotLeft => Point { x: 0.0, y: size.y },
            BotRight => Point { x: size.x, y: size.y },
            TopLeft => Point { x: 0.0, y: 0.0 },
            TopRight => Point { x: size.x, y: 0.0 },
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct FontDefinition {
    pub source: String,
    pub size: f32,

    #[serde(default)]
    pub characters: Vec<CharacterRange>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct CharacterRange {
    pub lower: u32,
    pub upper: u32,
}

/// What to compute the width of a widget relative to.
#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub enum WidthRelative {
    /// Width is equal to the `x` field of the widget's `size`.
    Normal,

    /// Width is sized so that the widget's inner width just encompasses all child widgets, plus the `x` field
    /// of the widget's `size`.  Note that any children of this Widget should not use `Parent` width if this
    /// is used.
    Children,

    /// Width is equal to the parent widget's inner width plus the `x` field of the widget's `size`.
    Parent,
}

impl Default for WidthRelative {
    fn default() -> Self { WidthRelative::Normal }
}

/// What to compute the height of widget relative to.
#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub enum HeightRelative {
    /// Height is equal to the `y` field of the widget's `size`.
    Normal,

    /// Height is equal to the parent widget's inner height plus the `y` field of the widget's `size`.
    Parent,

    /// Height is sized so that the widget's inner height just encompasses all child widgets, plus the `y` field
    /// of the widget's `size`.  Note that any children of this Widget should not use `Parent` height if this
    /// is used.
    Children,

    /// Height is equal to the line height of the widget's font plus the `y` field of the widget's `size`.
    FontLine,
}

impl Default for HeightRelative {
    fn default() -> Self { HeightRelative::Normal }
}

/// A Color with red, green, blue, and alpha components, with each component stored as a `u8`.
///
/// Colors can be deserialized from strings consisting of either
/// one of the predefined names: `white`, `black`, `red`, `green`,
/// `blue`, `cyan`, `yellow`, or `magenta`.
/// Or, the `#` character followed by a hex color code.  The hex code can either
/// be 8, 6, 4 or 3 digits long.
/// * 8 digits - each set of 2 digits specifies one color component - red, green, blue, then alpha
/// * 6 digits - each set of 2 digits specifies one color component - red, green, and blue.  Alpha is assumed to be the maximum value of FF.
/// * 4 digits - each single digit specifies one color component - red, green, blue, then alpha, with half precision.
/// * 3 digits - each single digit specifies one color component - red, green, then blue.  Alpha ia assumed to be the maximum value of F.
/// For the 4 and 3 digit variants - each component has one of 16 possible values.  The value is multiplied by 17 to determine the
/// corresponding full precision value.  For example, `0` maps to `00`, `F` maps to `FF`, and `8` maps to `88`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Color {
    /// The red component
    pub r: u8,

    /// The green component
    pub g: u8,

    /// The blue component
    pub b: u8,

    /// The alpha or transparency component
    pub a: u8,
}

impl Color {
    /// The color white.  `#FFFFFF` or `#FFF` in the theme
    pub fn white() -> Self { Color { r: 255, g: 255, b: 255, a: 255 }}

    /// The color black.  `#000000` or `#000` in the theme
    pub fn black() -> Self { Color { r: 0, g: 0, b: 0, a: 255 }}

    /// The color red.  `#FF0000` or `#F00` in the theme
    pub fn red() -> Self { Color { r: 255, g: 0, b: 0, a: 255 }}

    /// The color green.  `#00FF00` or `#0F0` in the theme
    pub fn green() -> Self { Color { r: 0, g: 255, b: 255, a: 255 }}

    /// The color blue.  `#0000FF` or `#00F` in the theme
    pub fn blue() -> Self { Color { r: 0, g: 0, b: 255, a: 255 }}

    /// The color cyan.  `#00FFFF` or `#0FF` in the theme
    pub fn cyan() -> Self { Color { r: 0, g: 255, b: 255, a: 255 }}

    /// The color yellow.  `#FFFF00` or `#FF0` in the theme
    pub fn yellow() -> Self { Color { r: 255, g: 255, b: 0, a: 255 }}

    /// The color magenta or purple.  `#FF00FF` or `#F0F` in the theme
    pub fn magenta() -> Self { Color { r: 255, g: 0, b: 255, a: 255 }}

    /// Attempts to parse a Color from the specified `value` string.  Returns None if
    /// it cannot be parsed.  See the main [`Color`](struct.Color.html) serialization rules.
    pub fn parse_str(value: &str) -> Option<Color> {
        let visitor = ColorVisitor {};
        visitor.visit_str::<serde::de::value::Error>(value).ok()
    }
}

impl Default for Color {
    fn default() -> Self { Color::white() }
}

impl From<Color> for [f32; 4] {
    fn from(c: Color) -> [f32; 4] {
        [c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0, c.a as f32 / 255.0]
    }
}

struct ColorVisitor;

impl<'de> Visitor<'de> for ColorVisitor {
    type Value = Color;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("A valid color name or # followed by a 6 or 8 character \
            (2 digits per color) or 3 or 4 character (1 digit per color) hex string")
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        if value.starts_with('#') {
            let count = value.chars().count();
            if value.len() != count {
                // non single byte characters which cannot be parsed
                return Err(E::custom(format!("{} is not a valid 3, 4, 6, or 8 character hex code", value)));
            }
            match count {
                4 => {
                    let r = hex_str_to_color_component(&value[1..2])? * 17;
                    let g = hex_str_to_color_component(&value[2..3])? * 17;
                    let b = hex_str_to_color_component(&value[3..4])? * 17;
                    Ok(Color { r, g, b, a: 255 })
                },
                5 => {
                    let r = hex_str_to_color_component(&value[1..2])? * 17;
                    let g = hex_str_to_color_component(&value[2..3])? * 17;
                    let b = hex_str_to_color_component(&value[3..4])? * 17;
                    let a = hex_str_to_color_component(&value[4..5])? * 17;
                    Ok(Color { r, g, b, a })
                },
                7 => {
                    let r = hex_str_to_color_component(&value[1..3])?;
                    let g = hex_str_to_color_component(&value[3..5])?;
                    let b = hex_str_to_color_component(&value[5..7])?;
                    Ok(Color { r, g, b, a: 255 })
                },
                9 => {
                    let r = hex_str_to_color_component(&value[1..3])?;
                    let g = hex_str_to_color_component(&value[3..5])?;
                    let b = hex_str_to_color_component(&value[5..7])?;
                    let a = hex_str_to_color_component(&value[7..9])?;
                    Ok(Color { r, g, b, a })
                },
                _ => Err(E::custom(format!("{} is not a valid 3, 4, 6, or 8 character hex code", value)))
            }
        } else {
            Ok(match value {
                "white" => Color::white(),
                "black" => Color::black(),
                "red" => Color::red(),
                "green" => Color::green(),
                "blue" => Color::blue(),
                "cyan" => Color::cyan(),
                "yellow" => Color::yellow(),
                "magenta" => Color::magenta(),
                _ => {
                    return Err(E::custom(format!("Unable to parse color from {}.  Hex codes must start with #", value)));
                }
            })
        }
    }
}

fn hex_str_to_color_component<E: de::Error>(input: &str) -> Result<u8, E> {
    let c = u8::from_str_radix(input, 16).map_err(|_| {
        E::custom(format!("Unable to parse color component from {}", input))
    })?;

    Ok(c)
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Color, D::Error> {
        deserializer.deserialize_str(ColorVisitor)
    }
}

impl Serialize for Color {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("#{:x?}{:x?}{:x?}", self.r, self.g, self.b))
    }
}