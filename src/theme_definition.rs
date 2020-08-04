use std::collections::HashMap;
use std::fmt;

use serde::{Serialize, Deserialize, Deserializer, Serializer, de::{self, Visitor}};

use crate::{Border, Point};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeDefinition {
    pub fonts: HashMap<String, FontDefinition>,
    pub image_sets: HashMap<String, ImageSet>,
    pub widgets: HashMap<String, WidgetThemeDefinition>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WidgetThemeDefinition {
    pub from: Option<String>,

    pub text: Option<String>,
    pub font: Option<String>,
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

    #[serde(default)]
    pub children: HashMap<String, WidgetThemeDefinition>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageSet {
    pub source: String,
    pub images: HashMap<String, ImageDefinition>,
}

#[derive(Serialize, Deserialize)]
pub struct ImageDefinition {
    #[serde(flatten)]
    pub kind: ImageDefinitionKind,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
#[serde(deny_unknown_fields)]
pub enum ImageDefinitionKind {
    Composed {
        position: [u32; 2],
        grid_size: [u32; 2],
    },
    Simple {
        position: [u32; 2],
        size: [u32; 2],

        #[serde(default)]
        stretch: bool,
    },
    Animated {
        states: HashMap<AnimState, String>,
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct AnimState {
    keys: [AnimStateKey; 4],
}

impl AnimState {
    pub const fn new(state: AnimStateKey) -> AnimState {
        AnimState { keys: [state, AnimStateKey::Normal, AnimStateKey::Normal, AnimStateKey::Normal] }
    }

    pub const fn normal() -> AnimState {
        AnimState { keys: [AnimStateKey::Normal; 4] }
    }

    pub const fn disabled() -> AnimState {
        let mut keys = [AnimStateKey::Normal; 4];
        keys[0] = AnimStateKey::Disabled;
        AnimState { keys }
    }

    pub fn contains(&self, key: AnimStateKey) -> bool {
        for self_key in self.keys.iter() {
            if *self_key == key { return true; }
        }
        false
    }

    pub fn add(&mut self, to_add: AnimStateKey) {
        for key in self.keys.iter_mut() {
            if *key == AnimStateKey::Normal {
                *key = to_add;
                break;
            }
        }

        self.keys.sort();
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
        let mut key_index = 0;
        let mut normal_found = false;

        for key_id in value.split('+') {
            if key_index >= keys.len() {
                return Err(E::custom(format!("Only a maximum of {} AnimStateKeys are allowed", keys.len())));
            }

            if normal_found {
                return Err(E::custom(format!("Normal may only be specified as the sole AnimStateKey")));
            }

            let key_id = key_id.trim();
            match key_id {
                "Normal" => {
                    if key_index != 0 {
                        return Err(E::custom(format!("Normal may only be specified as the sole AnimStateKey")));
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

            key_index += 1;
        }

        keys.sort();

        Ok(AnimState { keys })
    }
}

fn add_if_not_already_present<E: de::Error>(keys: &mut [AnimStateKey; 4], max_index: usize, key: AnimStateKey) -> Result<(), E> {
    for i in 0..max_index {
        if keys[i] == key {
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

#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
#[serde(deny_unknown_fields)]
pub enum AnimStateKey {
    Hover,
    Pressed,
    Disabled,
    Normal,
    Active,
}

#[derive(Serialize, Deserialize, Copy, Clone)]
#[serde(deny_unknown_fields)]
pub enum Layout {
    Horizontal,
    Vertical,
    Free,
}

impl Default for Layout {
    fn default() -> Self { Layout::Horizontal }
}

#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub enum Align {
    Left,
    Right,
    Bot,
    Top,
    Center,
    BotLeft,
    BotRight,
    TopLeft,
    TopRight,
}

impl Default for Align {
    fn default() -> Self { Align::TopLeft }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FontDefinition {
    pub source: String,
    pub size: f32,
}

#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub enum WidthRelative {
    Normal,
    Parent,
}

impl Default for WidthRelative {
    fn default() -> Self { WidthRelative::Normal }
}

#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub enum HeightRelative {
    Normal,
    Parent,
    FontLine,
}

impl Default for HeightRelative {
    fn default() -> Self { HeightRelative::Normal }
}