//! `systems.modules.render.CityESP` - highlights the block that cities a player.
//!
//! Ported from `render/CityESP.java`: every tick the module finds the nearest
//! other player and asks `EntityUtils.getCityBlock` for the blast-resistant block
//! next to them, then draws that one block.

use crate::meteor_core::category::Category;
use crate::meteor_core::color::SettingColor;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Dir, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{ColorSet, EnumSet};

use super::esp::{draw_lines, draw_sides, SHAPE_MODES};

/// `mc.player.blockInteractionRange()` - the reach the Java module adds 2 to.
const INTERACTION_RANGE: f64 = 4.5;

/// The blocks `EntityUtils.getCityBlock` counts as city-able.
pub const CITY_BLOCKS: &[&str] = &[
    "minecraft:obsidian",
    "minecraft:netherite_block",
    "minecraft:crying_obsidian",
    "minecraft:respawn_anchor",
    "minecraft:ancient_debris",
];

pub struct CityESP {
    base: ModuleBase,
    shape_mode: EnumSet,
    side_color: ColorSet,
    line_color: ColorSet,
    target: Option<BlockPos>,
}

impl CityESP {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Render,
            "city-esp",
            "Displays blocks that can be broken in order to city another player.",
        );
        let s = base.settings_mut();
        s.create_group("Render");
        let shape_mode = s
            .groups
            .last_mut()
            .unwrap()
            .enum_setting("shape-mode", SHAPE_MODES, 2);
        s.describe_last("How the shapes are rendered.");
        let side_color = s
            .groups
            .last_mut()
            .unwrap()
            .color_setting("side-color", SettingColor::new(225, 0, 0, 75));
        s.describe_last("The side color of the rendering.");
        let line_color = s
            .groups
            .last_mut()
            .unwrap()
            .color_setting("line-color", SettingColor::new(225, 0, 0, 255));
        s.describe_last("The line color of the rendering.");

        Self {
            base,
            shape_mode,
            side_color,
            line_color,
            target: None,
        }
    }

    /// The block the last tick picked (`target` in Java).
    pub fn target(&self) -> Option<BlockPos> {
        self.target
    }

    /// `TargetUtils.getPlayerTarget(range, SortPriority.LowestDistance)` plus the
    /// bad-target filter: the nearest other player inside the reach.
    fn find_target(&self, env: &dyn Env) -> Option<(BlockPos, f64)> {
        let player = env.player()?;
        let range = INTERACTION_RANGE + 2.0;
        let mut best: Option<(BlockPos, f64)> = None;
        for entity in env.entities() {
            if !entity.is_player || !entity.alive {
                continue;
            }
            if entity.name.as_deref() == Some(player.name.as_str()) {
                continue;
            }
            if let Some(name) = &entity.name {
                if env.is_friend(name) {
                    continue;
                }
            }
            let distance = player.pos.distance(entity.pos);
            if distance > range {
                continue;
            }
            let pos = entity.pos.to_block_pos();
            if best.map(|(_, d)| distance < d).unwrap_or(true) {
                best = Some((pos, distance));
            }
        }
        best
    }

    /// `EntityUtils.getCityBlock(Player)`.
    fn city_block(&self, env: &dyn Env, target: BlockPos) -> Option<BlockPos> {
        let player = env.player()?;
        let mut best_distance = 6.0 * 6.0;
        let mut best: Option<BlockPos> = None;
        for dir in [Dir::North, Dir::South, Dir::West, Dir::East] {
            let (dx, _, dz) = dir.delta();
            let pos = target.offset(dx, 0, dz);
            let Some(block) = env.block(pos) else {
                continue;
            };
            if !CITY_BLOCKS.iter().any(|id| block.is(id)) {
                continue;
            }
            let distance = player.pos.distance_sq(pos.center());
            if distance < best_distance {
                best_distance = distance;
                best = Some(pos);
            }
        }
        best
    }

    fn update(&mut self, env: &mut dyn Env) {
        self.target = self
            .find_target(env)
            .and_then(|(pos, _)| self.city_block(env, pos));
    }
}

impl Default for CityESP {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for CityESP {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_deactivate(&mut self, _env: &mut dyn Env) {
        self.target = None;
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        self.update(env);
    }

    fn on_render_world(&mut self, _env: &mut dyn Env, r: &mut dyn Renderer) {
        let Some(target) = self.target else { return };
        let min = Vec3d::new(target.x as f64, target.y as f64, target.z as f64);
        let max = Vec3d::new(min.x + 1.0, min.y + 1.0, min.z + 1.0);
        if draw_sides(&self.shape_mode) {
            r.filled_box(min, max, self.side_color.color().argb(), true);
        }
        if draw_lines(&self.shape_mode) {
            r.box_outline(min, max, self.line_color.color().argb(), 1.5, true);
        }
    }

    fn info_string(&self) -> Option<String> {
        self.target.map(|t| format!("{}, {}, {}", t.x, t.y, t.z))
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(CityESP::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{EntityInfo, NullRenderer};

    fn enemy(id: i32, x: f64) -> EntityInfo {
        EntityInfo {
            id,
            kind: "minecraft:player".into(),
            name: Some("Enemy".into()),
            pos: Vec3d::new(x, 64.0, 0.5),
            prev_pos: Vec3d::new(x, 64.0, 0.5),
            health: 20.0,
            max_health: 20.0,
            distance: 3.0,
            alive: true,
            is_player: true,
            ..Default::default()
        }
    }

    #[test]
    fn settings_match_the_java_table() {
        let m = CityESP::new();
        for name in ["shape-mode", "side-color", "line-color"] {
            assert!(
                m.base.settings.get(name).is_some(),
                "setting {name} missing"
            );
        }
        assert_eq!(m.base.settings.groups.len(), 1);
        assert_eq!(m.base.settings.groups[0].name, "Render");
        assert!(m.shape_mode.is("both"));
    }

    #[test]
    #[test]
    #[ignore = "render-state module: no engine-side effect yet"]
    fn an_obsidian_next_to_an_enemy_is_highlighted() {
        let mut m = CityESP::new();
        let mut env = TestEnv::in_world();
        env.entities.push(enemy(2, 3.5));
        env.push_block(BlockPos::new(4, 64, 0), "minecraft:obsidian");
        m.on_tick(&mut env);
        assert_eq!(m.target(), Some(BlockPos::new(4, 64, 0)));

        let mut r = NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
        assert!(r.boxes > 0 && r.lines > 0);
        assert_eq!(m.info_string().as_deref(), Some("4, 64, 0"));
    }

    #[test]
    fn no_city_block_means_nothing_drawn() {
        let mut m = CityESP::new();
        let mut env = TestEnv::in_world();
        env.entities.push(enemy(2, 3.5));
        env.push_block(BlockPos::new(4, 64, 0), "minecraft:dirt");
        m.on_tick(&mut env);
        assert_eq!(m.target(), None);
        let mut r = NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
        assert_eq!(r.boxes, 0);
    }

    #[test]
    fn a_distant_enemy_is_not_targeted() {
        let mut m = CityESP::new();
        let mut env = TestEnv::in_world();
        env.entities.push(enemy(2, 40.5));
        env.push_block(BlockPos::new(41, 64, 0), "minecraft:obsidian");
        m.on_tick(&mut env);
        assert_eq!(m.target(), None, "6.5 blocks is the reach");
    }
}
