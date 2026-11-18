//! Face-exclusion boxes shared by the world ESP modules.
//!
//! Meteor's `Renderer3D.box(..., excludeDir)` takes a bitmask of faces to leave
//! out, which is how the hole/tunnel/storage modules stitch neighbouring boxes
//! together. `Renderer` has no such parameter, so this module draws the faces and
//! edges itself with the same exclusion rules.

use crate::meteor_core::env::Renderer;
use crate::meteor_core::math::{Dir, Vec3d};

/// The six faces of a box, `true` meaning "do not draw it".
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Exclude {
    pub down: bool,
    pub up: bool,
    pub north: bool,
    pub south: bool,
    pub west: bool,
    pub east: bool,
}

impl Exclude {
    pub const NONE: Exclude = Exclude {
        down: false,
        up: false,
        north: false,
        south: false,
        west: false,
        east: false,
    };

    /// `Dir.get(direction)`.
    pub fn of(d: Dir) -> Self {
        let mut e = Self::NONE;
        e.set(d);
        e
    }

    pub fn set(&mut self, d: Dir) {
        match d {
            Dir::Down => self.down = true,
            Dir::Up => self.up = true,
            Dir::North => self.north = true,
            Dir::South => self.south = true,
            Dir::West => self.west = true,
            Dir::East => self.east = true,
        }
    }

    pub fn has(&self, d: Dir) -> bool {
        match d {
            Dir::Down => self.down,
            Dir::Up => self.up,
            Dir::North => self.north,
            Dir::South => self.south,
            Dir::West => self.west,
            Dir::East => self.east,
        }
    }

    /// `excludeDir |= Dir.get(side)`.
    pub fn merge(self, other: Exclude) -> Self {
        Self {
            down: self.down || other.down,
            up: self.up || other.up,
            north: self.north || other.north,
            south: self.south || other.south,
            west: self.west || other.west,
            east: self.east || other.east,
        }
    }
}

/// `Renderer3D.box` with `excludeDir`.
pub fn draw_box(
    r: &mut dyn Renderer,
    min: Vec3d,
    max: Vec3d,
    side: u32,
    line: u32,
    sides: bool,
    lines: bool,
    ex: Exclude,
) {
    if sides {
        // Zero-thickness boxes stand in for the six quads.
        if !ex.down {
            r.filled_box(
                Vec3d::new(min.x, min.y, min.z),
                Vec3d::new(max.x, min.y, max.z),
                side,
                true,
            );
        }
        if !ex.up {
            r.filled_box(
                Vec3d::new(min.x, max.y, min.z),
                Vec3d::new(max.x, max.y, max.z),
                side,
                true,
            );
        }
        if !ex.north {
            r.filled_box(
                Vec3d::new(min.x, min.y, min.z),
                Vec3d::new(max.x, max.y, min.z),
                side,
                true,
            );
        }
        if !ex.south {
            r.filled_box(
                Vec3d::new(min.x, min.y, max.z),
                Vec3d::new(max.x, max.y, max.z),
                side,
                true,
            );
        }
        if !ex.west {
            r.filled_box(
                Vec3d::new(min.x, min.y, min.z),
                Vec3d::new(min.x, max.y, max.z),
                side,
                true,
            );
        }
        if !ex.east {
            r.filled_box(
                Vec3d::new(max.x, min.y, min.z),
                Vec3d::new(max.x, max.y, max.z),
                side,
                true,
            );
        }
    }

    if lines {
        // The four vertical edges.
        if !ex.west && !ex.north {
            r.line(
                Vec3d::new(min.x, min.y, min.z),
                Vec3d::new(min.x, max.y, min.z),
                line,
                1.5,
                true,
            );
        }
        if !ex.west && !ex.south {
            r.line(
                Vec3d::new(min.x, min.y, max.z),
                Vec3d::new(min.x, max.y, max.z),
                line,
                1.5,
                true,
            );
        }
        if !ex.east && !ex.north {
            r.line(
                Vec3d::new(max.x, min.y, min.z),
                Vec3d::new(max.x, max.y, min.z),
                line,
                1.5,
                true,
            );
        }
        if !ex.east && !ex.south {
            r.line(
                Vec3d::new(max.x, min.y, max.z),
                Vec3d::new(max.x, max.y, max.z),
                line,
                1.5,
                true,
            );
        }
        // The bottom and top rings.
        if !ex.down {
            if !ex.north {
                r.line(
                    Vec3d::new(min.x, min.y, min.z),
                    Vec3d::new(max.x, min.y, min.z),
                    line,
                    1.5,
                    true,
                );
            }
            if !ex.south {
                r.line(
                    Vec3d::new(min.x, min.y, max.z),
                    Vec3d::new(max.x, min.y, max.z),
                    line,
                    1.5,
                    true,
                );
            }
            if !ex.west {
                r.line(
                    Vec3d::new(min.x, min.y, min.z),
                    Vec3d::new(min.x, min.y, max.z),
                    line,
                    1.5,
                    true,
                );
            }
            if !ex.east {
                r.line(
                    Vec3d::new(max.x, min.y, min.z),
                    Vec3d::new(max.x, min.y, max.z),
                    line,
                    1.5,
                    true,
                );
            }
        }
        if !ex.up {
            if !ex.north {
                r.line(
                    Vec3d::new(min.x, max.y, min.z),
                    Vec3d::new(max.x, max.y, min.z),
                    line,
                    1.5,
                    true,
                );
            }
            if !ex.south {
                r.line(
                    Vec3d::new(min.x, max.y, max.z),
                    Vec3d::new(max.x, max.y, max.z),
                    line,
                    1.5,
                    true,
                );
            }
            if !ex.west {
                r.line(
                    Vec3d::new(min.x, max.y, min.z),
                    Vec3d::new(min.x, max.y, max.z),
                    line,
                    1.5,
                    true,
                );
            }
            if !ex.east {
                r.line(
                    Vec3d::new(max.x, max.y, min.z),
                    Vec3d::new(max.x, max.y, max.z),
                    line,
                    1.5,
                    true,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::NullRenderer;

    #[test]
    fn a_full_box_emits_faces_and_edges() {
        let mut r = NullRenderer::default();
        draw_box(
            &mut r,
            Vec3d::ZERO,
            Vec3d::new(1.0, 1.0, 1.0),
            0xFFFF_FFFF,
            0xFFFF_FFFF,
            true,
            true,
            Exclude::NONE,
        );
        assert_eq!(r.boxes, 6, "six faces");
        assert_eq!(r.lines, 12, "twelve edges");
    }

    #[test]
    fn an_excluded_face_takes_its_edges_with_it() {
        let mut r = NullRenderer::default();
        draw_box(
            &mut r,
            Vec3d::ZERO,
            Vec3d::new(1.0, 1.0, 1.0),
            0xFFFF_FFFF,
            0xFFFF_FFFF,
            true,
            true,
            Exclude::of(Dir::North),
        );
        assert_eq!(r.boxes, 5);
        // North is the z = min.z face: its two vertical edges and the two
        // bottom/top edges along x go with it.
        assert_eq!(r.lines, 8);
    }
}
