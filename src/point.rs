use std::ops::*;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Copy, Clone, Default, Debug, PartialEq)]
pub struct Border {
    #[serde(default)]
    pub top: f32,

    #[serde(default)]
    pub bot: f32,

    #[serde(default)]
    pub left: f32,

    #[serde(default)]
    pub right: f32,
}

impl Border {
    pub fn vertical(&self) -> f32 {
        self.top + self.bot
    }

    pub fn horizontal(&self) -> f32 {
        self.left + self.right
    }
    
    pub fn tr(&self) -> Point {
        Point { x: self.right, y: self.top }
    }

    pub fn tl(&self) -> Point {
        Point { x: self.left, y: self.top }
    }

    pub fn bl(&self) -> Point {
        Point { x: self.left, y: self.bot }
    }

    pub fn br(&self) -> Point {
        Point { x: self.right, y: self.bot }
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, Default, Debug, PartialEq)]
pub struct Rect {
    pub ul: Point,
    pub lr: Point,
}

impl Rect {
    pub fn new(pos: Point, size: Point) -> Rect {
        Rect {
            ul: pos,
            lr: pos + size,
        }
    }

    pub fn inside(&self, pos: Point) -> bool {
        pos.x >= self.ul.x && pos.x <= self.lr.x && pos.y >= self.ul.y && pos.y <= self.lr.y
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, Default, Debug, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
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