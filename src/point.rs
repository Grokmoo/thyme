use std::ops::*;
use std::fmt;

use serde::{Serialize, Deserialize, Deserializer, de::{self, Error, Visitor, MapAccess}};

/// A struct representing a rectangular border around a Widget.
/// In the theme file, border can be deserialzed as a standard mapping, or
/// using `all: {value}` to specify all four values are the same, or
/// `width` and `height` to specify `left` and `right` and `top` and `bot`,
/// respectively.
#[derive(Serialize, Copy, Clone, Default, Debug, PartialEq)]
pub struct Border {
    /// The upper edge border
    pub top: f32,

    /// The lower edge border
    pub bot: f32,

    /// The left edge border
    pub left: f32,

    /// The right edge border
    pub right: f32,
}

impl Border {
    /// The vertical border, top plus bottom
    pub fn vertical(&self) -> f32 {
        self.top + self.bot
    }

    /// The horizontal border, left plus right
    pub fn horizontal(&self) -> f32 {
        self.left + self.right
    }
    
    /// The border on the top right corner
    pub fn tr(&self) -> Point {
        Point { x: self.right, y: self.top }
    }

    /// The border on the top left corner
    pub fn tl(&self) -> Point {
        Point { x: self.left, y: self.top }
    }

    /// The border on the bottom left corner
    pub fn bl(&self) -> Point {
        Point { x: self.left, y: self.bot }
    }

    /// The border on the bottom right corner
    pub fn br(&self) -> Point {
        Point { x: self.right, y: self.bot }
    }
}

struct BorderVisitor;

impl<'de> Visitor<'de> for BorderVisitor {
    type Value = Border;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("Map")
    }

    fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
        const ERROR_MSG: &str =
            "Unable to parse border from map. Must specify values for: \
            all OR width, height, OR top, bot, left, right \
            Unspecified values are set to 0";

        let mut data = [f32::MIN; 4];
        #[derive(Copy, Clone, PartialEq)]
        enum Mode {
            One,
            Two,
            Four,
        };
        let mut mode: Option<Mode> = None;
        fn check_mode<E: de::Error>(mode: &mut Option<Mode>, must_eq: Mode) -> Result<(), E> {
            match mode {
                None => {
                    *mode = Some(must_eq);
                    Ok(())
                },
                Some(mode) => if *mode == must_eq {
                    Ok(())
                } else {
                    Err(E::custom(ERROR_MSG))
                }
            }
        }

        loop {
            let (kind, value) = match map.next_entry::<String, f32>()? {
                None => break,
                Some(data) => data,
            };

            match &*kind {
                "all" => {
                    check_mode(&mut mode, Mode::One)?;
                    data[0] = value;
                },
                "width" => {
                    check_mode(&mut mode, Mode::Two)?;
                    data[0] = value;
                },
                "height" => {
                    check_mode(&mut mode, Mode::Two)?;
                    data[1] = value;
                },
                "top" => {
                    check_mode(&mut mode, Mode::Four)?;
                    data[0] = value;
                },
                "bot" => {
                    check_mode(&mut mode, Mode::Four)?;
                    data[1] = value;
                },
                "left" => {
                    check_mode(&mut mode, Mode::Four)?;
                    data[2] = value;
                },
                "right" => {
                    check_mode(&mut mode, Mode::Four)?;
                    data[3] = value;
                },
                _ => return Err(M::Error::custom(ERROR_MSG))
            }
        }

        // fill in the default values at this point if needed
        for val in &mut data {
            if *val == f32::MIN {
                *val = 0.0;
            }
        }

        match mode {
            Some(Mode::One) =>
                Ok(Border { top: data[0], bot: data[0], left: data[0], right: data[0] }),
            Some(Mode::Two) =>
                Ok(Border { top: data[1], bot: data[1], left: data[0], right: data[0] }),
            Some(Mode::Four) =>
                Ok(Border { top: data[0], bot: data[1], left: data[2], right: data[3] }),
            None =>
                Err(M::Error::custom(ERROR_MSG)),
        }
    }
}

impl<'de> Deserialize<'de> for Border {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Border, D::Error> {
        deserializer.deserialize_map(BorderVisitor)
    }
}

/// A rectangular area, represented by a position and a size
#[derive(Serialize, Deserialize, Copy, Clone, Default, Debug, PartialEq)]
pub struct Rect {
    /// The position of the rectangle
    pub pos: Point,

    /// The size of the rectangle
    pub size: Point
}

impl Rect {
    /// Construct a new `Rect` with the specified position and size.
    pub fn new(pos: Point, size: Point) -> Rect {
        Rect {
            pos,
            size,
        }
    }

    /// Returns the left edge of this Rect.
    pub fn left(&self) -> f32 {
        self.pos.x
    }

    /// Returns the right edge of this Rect.
    pub fn right(&self) -> f32 {
        self.pos.x + self.size.x
    }

    /// Returns the top edge of this Rect.
    pub fn top(&self) -> f32 {
        self.pos.y
    }

    /// Returns the bottom edge of this Rect.
    pub fn bot(&self) -> f32 {
        self.pos.y + self.size.y
    }

    /// Returns true if the specified point is inside (or on the edge of)
    /// this rectangle; false otherwise
    pub fn is_inside(&self, pos: Point) -> bool {
        pos.x >= self.pos.x && pos.y >= self.pos.y &&
            pos.x <= self.pos.x + self.size.x && pos.y <= self.pos.y + self.size.y
    }

    /// Returns a new `Rect` this is the minimum extent on a component-by-component
    /// basis between this and `other`.  The returned `Rect` will barely fit inside
    /// both this and `other` (if possible - if not it will have size 0)
    pub fn min(self, other: Rect) -> Rect {
        let min = self.pos.max(other.pos);
        let max: Point = (self.pos + self.size).min(other.pos + other.size);

        Rect {
            pos: min,
            size: (max - min).max(Point::default()),
        }
    }

    /// Returns a new `Rect` that is the maximum extent on a component-by-component
    /// basis between this and `other`.  The returned `Rect` will barely contain
    /// both this and `other`.
    pub fn max(self, other: Rect) -> Rect {
        let min = self.pos.min(other.pos);
        let max: Point = (self.pos + self.size).max(other.pos + other.size);

        Rect {
            pos: min,
            size: max - min,
        }
    }
}

impl Mul<Rect> for f32 {
    type Output = Rect;
    fn mul(self, rect: Rect) -> Rect {
        Rect {
            pos: rect.pos * self,
            size: rect.size * self,
        }
    }
}

impl Mul<f32> for Rect {
    type Output = Rect;
    fn mul(self, val: f32) -> Rect {
        Rect {
            pos: self.pos * val,
            size: self.size * val,
        }
    }
}

/// A two-dimensional point, with `x` and `y` coordinates.
#[derive(Serialize, Deserialize, Copy, Clone, Default, Debug, PartialEq)]
pub struct Point {
    /// The `x` cartesian coordinate
    pub x: f32,

    /// The `y` cartesian coordinate
    pub y: f32,
}

impl Point {
    /// Creates a new point from the specified components.
    pub fn new(x: f32, y: f32) -> Point {
        Point { x, y }
    }

    /// Returns a point with both components rounded to the nearest integer
    pub fn round(self) -> Point {
        Point {
            x: self.x.round(),
            y: self.y.round(),
        }
    }

    /// Returns a per-component maximum of this and `other`
    pub fn max(self, other: Point) -> Point {
        Point {
            x: self.x.max(other.x),
            y: self.y.max(other.y)
        }
    }

    /// Returns a per-component minimum of this and `other`
    pub fn min(self, other: Point) -> Point {
        Point {
            x: self.x.min(other.x),
            y: self.y.min(other.y),
        }
    }
}

impl From<[f32; 2]> for Point {
    fn from(t: [f32; 2]) -> Self {
        Point { x: t[0], y: t[1] }
    }
}

impl From<Point> for [f32; 2] {
    fn from(point: Point) -> Self {
        [point.x, point.y]
    }
}

impl From<(f32, f32)> for Point {
    fn from(t: (f32, f32)) -> Self {
        Point { x: t.0, y: t.1 }
    }
}

impl From<Point> for (f32, f32) {
    fn from(point: Point) -> Self {
        (point.x, point.y)
    }
}

impl Sub<Point> for Point {
    type Output = Point;
    fn sub(self, other: Point) -> Point {
        Point { x: self.x - other.x, y: self.y - other.y }
    }
}

impl Add<Point> for Point {
    type Output = Point;
    fn add(self, other: Point) -> Point{
        Point { x: self.x + other.x, y: self.y + other.y }
    }
}

impl Sub<(f32, f32)> for Point {
    type Output = Point;
    fn sub(self, other: (f32, f32)) -> Point {
        Point { x: self.x - other.0, y: self.y - other.0 }
    }
}

impl Sub<[f32; 2]> for Point {
    type Output = Point;
    fn sub(self, other: [f32; 2]) -> Point {
        Point { x: self.x - other[0], y: self.y - other[0] }
    }
}

impl Add<(f32, f32)> for Point {
    type Output = Point;
    fn add(self, other: (f32, f32)) -> Point {
        Point { x: self.x + other.0, y: self.y + other.1 }
    }
}

impl Add<[f32; 2]> for Point {
    type Output = Point;
    fn add(self, other: [f32; 2]) -> Point {
        Point { x: self.x + other[0], y: self.y + other[1] }
    }
}

impl Mul<f32> for Point {
    type Output = Point;
    fn mul(self, val: f32) -> Point {
        Point { x: self.x * val, y: self.y * val }
    }
}

impl Div<f32> for Point {
    type Output = Point;
    fn div(self, val: f32) -> Point {
        Point { x: self.x / val, y: self.y / val }
    }
}


impl Sub<Point> for (f32, f32) {
    type Output = Point;
    fn sub(self, other: Point) -> Point {
        Point { x: other.x - self.0, y: other.y - self.0 }
    }
}

impl Sub<Point> for [f32; 2] {
    type Output = Point;
    fn sub(self, other: Point) -> Point {
        Point { x: other.x - self[0], y: other.y - self[0] }
    }
}

impl Add<Point> for (f32, f32) {
    type Output = Point;
    fn add(self, other: Point) -> Point {
        Point { x: other.x + self.0, y: other.y + self.1 }
    }
}

impl Add<Point> for [f32; 2] {
    type Output = Point;
    fn add(self, other: Point) -> Point {
        Point { x: other.x + self[0], y: other.y + self[1] }
    }
}

impl Mul<Point> for f32 {
    type Output = Point;
    fn mul(self, val: Point) -> Point {
        Point { x: val.x * self, y: val.y * self }
    }
}

impl Div<Point> for f32 {
    type Output = Point;
    fn div(self, val: Point) -> Point {
        Point { x: self / val.x, y: self / val.y }
    }
}