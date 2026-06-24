use nalgebra::{Point3, Vector3};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Up,
    North,
    East,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    North,
    South,
    East,
    West,
}

impl Direction {
    pub const fn axis(self) -> Axis {
        match self {
            Self::Up | Self::Down => Axis::Up,
            Self::North | Self::South => Axis::North,
            Self::East | Self::West => Axis::East,
        }
    }

    pub const fn sign(self) -> f64 {
        match self {
            Self::Up | Self::North | Self::East => 1.0,
            Self::Down | Self::South | Self::West => -1.0,
        }
    }

    pub const fn opposite(self) -> Self {
        match self {
            Self::Up => Self::Down,
            Self::Down => Self::Up,
            Self::North => Self::South,
            Self::South => Self::North,
            Self::East => Self::West,
            Self::West => Self::East,
        }
    }

    pub fn unit_vector(self) -> Vector3<f64> {
        match self {
            Self::Up => Vector3::new(1.0, 0.0, 0.0),
            Self::Down => Vector3::new(-1.0, 0.0, 0.0),
            Self::North => Vector3::new(0.0, 1.0, 0.0),
            Self::South => Vector3::new(0.0, -1.0, 0.0),
            Self::East => Vector3::new(0.0, 0.0, 1.0),
            Self::West => Vector3::new(0.0, 0.0, -1.0),
        }
    }
}

pub const UP: Direction = Direction::Up;
pub const DOWN: Direction = Direction::Down;
pub const NORTH: Direction = Direction::North;
pub const SOUTH: Direction = Direction::South;
pub const EAST: Direction = Direction::East;
pub const WEST: Direction = Direction::West;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeoPoint {
    pub up: f64,
    pub north: f64,
    pub east: f64,
}

impl GeoPoint {
    pub const fn new(up: f64, north: f64, east: f64) -> Self {
        Self { up, north, east }
    }

    pub const fn origin() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }

    pub fn moved(self, dir: Direction, amount: f64) -> Self {
        let delta = dir.unit_vector() * amount;
        Self {
            up: self.up + delta.x,
            north: self.north + delta.y,
            east: self.east + delta.z,
        }
    }
}

impl From<GeoPoint> for Point3<f64> {
    fn from(p: GeoPoint) -> Self {
        Point3::new(p.up, p.north, p.east)
    }
}

impl From<Point3<f64>> for GeoPoint {
    fn from(p: Point3<f64>) -> Self {
        Self::new(p.x, p.y, p.z)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeoVector {
    pub up: f64,
    pub north: f64,
    pub east: f64,
}

impl GeoVector {
    pub const fn new(up: f64, north: f64, east: f64) -> Self {
        Self { up, north, east }
    }

    pub fn as_nalgebra(self) -> Vector3<f64> {
        Vector3::new(self.up, self.north, self.east)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeoSize {
    pub up: f64,
    pub north: f64,
    pub east: f64,
}

impl GeoSize {
    pub const fn new(up: f64, north: f64, east: f64) -> Self {
        Self { up, north, east }
    }
}
