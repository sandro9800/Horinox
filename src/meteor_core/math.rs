//! Vectors, rotations, timers and the small maths helpers Meteor's modules share
//! (`utils.player.*`, `utils.world.*`, `utils.misc.MathUtils`).

use std::f64::consts::PI;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3d {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3d {
    pub const ZERO: Vec3d = Vec3d {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub const fn of_block(x: i32, y: i32, z: i32) -> Self {
        Self::new(x as f64, y as f64, z as f64)
    }

    pub fn add(self, o: Vec3d) -> Vec3d {
        Vec3d::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }

    pub fn sub(self, o: Vec3d) -> Vec3d {
        Vec3d::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }

    pub fn mul(self, s: f64) -> Vec3d {
        Vec3d::new(self.x * s, self.y * s, self.z * s)
    }

    pub fn length(self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn length_sq(self) -> f64 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    pub fn horizontal_length(self) -> f64 {
        (self.x * self.x + self.z * self.z).sqrt()
    }

    pub fn normalize(self) -> Vec3d {
        let l = self.length();
        if l < 1.0e-8 {
            Vec3d::ZERO
        } else {
            self.mul(1.0 / l)
        }
    }

    /// Alias of Vec3d::distance_to (the name the Java Vec3 uses).
    pub fn distance(self, o: Vec3d) -> f64 {
        self.distance_to(o)
    }

    pub fn distance_to(self, o: Vec3d) -> f64 {
        self.sub(o).length()
    }

    pub fn distance_sq(self, o: Vec3d) -> f64 {
        self.sub(o).length_sq()
    }

    /// `BlockPos.containing(Vec3d)`.
    pub fn to_block_pos(self) -> BlockPos {
        BlockPos::new(
            self.x.floor() as i32,
            self.y.floor() as i32,
            self.z.floor() as i32,
        )
    }

    pub fn center(self) -> Vec3d {
        Vec3d::new(self.x + 0.5, self.y + 0.5, self.z + 0.5)
    }

    pub fn lerp(self, o: Vec3d, t: f64) -> Vec3d {
        Vec3d::new(
            self.x + (o.x - self.x) * t,
            self.y + (o.y - self.y) * t,
            self.z + (o.z - self.z) * t,
        )
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockPos {
    pub const ORIGIN: BlockPos = BlockPos { x: 0, y: 0, z: 0 };

    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub fn offset(self, dx: i32, dy: i32, dz: i32) -> Self {
        Self::new(self.x + dx, self.y + dy, self.z + dz)
    }

    pub fn offset_dir(self, d: Dir) -> Self {
        let (dx, dy, dz) = d.delta();
        self.offset(dx, dy, dz)
    }

    pub fn center(self) -> Vec3d {
        Vec3d::new(
            self.x as f64 + 0.5,
            self.y as f64 + 0.5,
            self.z as f64 + 0.5,
        )
    }

    pub fn as_vec(self) -> Vec3d {
        Vec3d::new(self.x as f64, self.y as f64, self.z as f64)
    }

    pub fn distance(self, o: BlockPos) -> f64 {
        let dx = (self.x - o.x) as f64;
        let dy = (self.y - o.y) as f64;
        let dz = (self.z - o.z) as f64;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// The six neighbours plus diagonals Meteor's hole/surround code uses.
    pub const NEIGHBORS: [Dir; 6] = [
        Dir::Down,
        Dir::Up,
        Dir::North,
        Dir::South,
        Dir::West,
        Dir::East,
    ];

    pub fn neighbors(self) -> impl Iterator<Item = BlockPos> {
        Self::NEIGHBORS.into_iter().map(move |d| self.offset_dir(d))
    }
}

/// `net.minecraft.core.Direction`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Dir {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl Dir {
    pub const fn delta(self) -> (i32, i32, i32) {
        match self {
            Dir::Down => (0, -1, 0),
            Dir::Up => (0, 1, 0),
            Dir::North => (0, 0, -1),
            Dir::South => (0, 0, 1),
            Dir::West => (-1, 0, 0),
            Dir::East => (1, 0, 0),
        }
    }

    pub const fn opposite(self) -> Dir {
        match self {
            Dir::Down => Dir::Up,
            Dir::Up => Dir::Down,
            Dir::North => Dir::South,
            Dir::South => Dir::North,
            Dir::West => Dir::East,
            Dir::East => Dir::West,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Dir::Down => "down",
            Dir::Up => "up",
            Dir::North => "north",
            Dir::South => "south",
            Dir::West => "west",
            Dir::East => "east",
        }
    }
}

/// Yaw/pitch in degrees, Minecraft's convention (yaw 0 = south, 90 = west).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rotation {
    pub yaw: f32,
    pub pitch: f32,
}

impl Rotation {
    pub const fn new(yaw: f32, pitch: f32) -> Self {
        Self { yaw, pitch }
    }

    /// `RotationUtils.getYaw(from, to)`.
    pub fn yaw_to(from: Vec3d, to: Vec3d) -> f32 {
        let dx = to.x - from.x;
        let dz = to.z - from.z;
        (dz.atan2(dx).to_degrees() as f32) - 90.0
    }

    /// `RotationUtils.getPitch(from, to)`.
    pub fn pitch_to(from: Vec3d, to: Vec3d) -> f32 {
        let dx = to.x - from.x;
        let dy = to.y - from.y;
        let dz = to.z - from.z;
        -((dy.atan2((dx * dx + dz * dz).sqrt())).to_degrees() as f32)
    }

    /// `RotationUtils.getRotation(from, to)`.
    pub fn between(from: Vec3d, to: Vec3d) -> Self {
        Self::new(Self::yaw_to(from, to), Self::pitch_to(from, to))
    }

    pub fn normalize(self) -> Self {
        Self::new(normalize_yaw(self.yaw), self.pitch.clamp(-90.0, 90.0))
    }

    /// The unit vector this rotation looks along.
    pub fn direction(self) -> Vec3d {
        let yaw = (self.yaw as f64).to_radians();
        let pitch = (self.pitch as f64).to_radians();
        Vec3d::new(
            -yaw.sin() * pitch.cos(),
            -pitch.sin(),
            yaw.cos() * pitch.cos(),
        )
    }

    pub fn distance_to(self, o: Rotation) -> f64 {
        let dy = normalize_yaw(o.yaw - self.yaw) as f64;
        let dp = (o.pitch - self.pitch) as f64;
        (dy * dy + dp * dp).sqrt()
    }
}

pub fn normalize_yaw(yaw: f32) -> f32 {
    let mut y = yaw % 360.0;
    if y >= 180.0 {
        y -= 360.0;
    }
    if y < -180.0 {
        y += 360.0;
    }
    y
}

/// `utils.misc.Timer` - a stopwatch whose elapsed time is read in seconds.
#[derive(Clone, Debug)]
pub struct Timer {
    start: Instant,
    running: bool,
}

impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}

impl Timer {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            running: true,
        }
    }

    pub fn reset(&mut self) {
        self.start = Instant::now();
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    pub fn resume(&mut self) {
        self.running = true;
    }

    /// Seconds since the last reset (`Timer.getTime()`).
    pub fn elapsed(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    pub fn elapsed_ms(&self) -> f64 {
        self.elapsed() * 1000.0
    }

    /// `Timer.passed(seconds)`.
    pub fn passed(&self, seconds: f64) -> bool {
        self.running && self.elapsed() >= seconds
    }

    pub fn passed_ms(&self, ms: f64) -> bool {
        self.passed(ms / 1000.0)
    }

    /// `Timer.setElapsed(...)`-ish helper used by cooldown-based modules.
    pub fn shift(&mut self, seconds: f64) {
        if seconds >= 0.0 {
            self.start = Instant::now()
                .checked_sub(Duration::from_secs_f64(seconds))
                .unwrap_or_else(Instant::now);
        }
    }
}

pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

pub fn clamp(v: f64, min: f64, max: f64) -> f64 {
    v.clamp(min, max)
}

/// `MathUtils.degToRad`.
pub fn deg_to_rad(deg: f64) -> f64 {
    deg * PI / 180.0
}

/// Smallest absolute difference between two yaw angles.
pub fn yaw_diff(a: f32, b: f32) -> f32 {
    normalize_yaw(b - a).abs()
}

/// `utils.world.HoleUtils` hole shapes, shared by the hole modules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HoleType {
    Single,
    DoubleX,
    DoubleZ,
    Quad,
}

impl HoleType {
    pub const fn offsets(self) -> &'static [(i32, i32)] {
        match self {
            HoleType::Single => &[(0, 0)],
            HoleType::DoubleX => &[(0, 0), (1, 0)],
            HoleType::DoubleZ => &[(0, 0), (0, 1)],
            HoleType::Quad => &[(0, 0), (1, 0), (0, 1), (1, 1)],
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            HoleType::Single => "single",
            HoleType::DoubleX => "double-x",
            HoleType::DoubleZ => "double-z",
            HoleType::Quad => "quad",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaw_to_north_is_180_or_minus_180() {
        let yaw = Rotation::yaw_to(Vec3d::ZERO, Vec3d::new(0.0, 0.0, -1.0));
        assert!((yaw.abs() - 180.0).abs() < 1.0e-4, "yaw was {yaw}");
    }

    #[test]
    fn pitch_to_straight_down_is_plus_90() {
        // Minecraft's pitch is positive looking down, and Meteor's
        // `RotationUtils.getPitch` negates the atan2 result to get that.
        let pitch = Rotation::pitch_to(Vec3d::ZERO, Vec3d::new(0.0, -1.0, 0.0));
        assert!((pitch - 90.0).abs() < 1.0e-4, "pitch was {pitch}");
    }

    #[test]
    fn direction_matches_rotation() {
        let r = Rotation::new(0.0, 0.0);
        let d = r.direction();
        assert!((d.z - 1.0).abs() < 1.0e-6);
        let back = Rotation::between(Vec3d::ZERO, d);
        assert!(back.yaw.abs() < 1.0e-4);
    }

    #[test]
    fn normalize_yaw_wraps() {
        assert_eq!(normalize_yaw(190.0), -170.0);
        assert_eq!(normalize_yaw(-190.0), 170.0);
    }

    #[test]
    fn timer_reports_passage() {
        let mut t = Timer::new();
        assert!(!t.passed(10.0));
        t.shift(20.0);
        assert!(t.passed(10.0));
    }

    #[test]
    fn block_pos_distance() {
        assert_eq!(BlockPos::new(0, 0, 0).distance(BlockPos::new(3, 4, 0)), 5.0);
    }
}
