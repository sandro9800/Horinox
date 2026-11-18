//! `systems.modules.combat.AutoWeb` -?Automatically places webs on other players.
//!
//! Ports the Java tick: pick a target, predict their position with the movement
//! delta, then place a cobweb into the block they are in (and above, with
//! `doubles`).

use crate::meteor_core::category::Category;
use crate::meteor_core::color::SettingColor;
use crate::meteor_core::env::{EntityInfo, Env, Renderer};
use crate::meteor_core::math::BlockPos;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, ColorSet, DoubleSet, EnumSet};

use super::{
    can_place_at, draw_shape, in_place_range, is_bad_target, last_group, place_block,
    player_target, SHAPE_BOTH, SHAPE_MODES, SORT_LOWEST_DISTANCE, SORT_PRIORITIES,
};

pub struct AutoWeb {
    base: ModuleBase,
    place_range: DoubleSet,
    place_walls_range: DoubleSet,
    priority: EnumSet,
    target_range: DoubleSet,
    predict_movement: BoolSet,
    ticks_to_predict: DoubleSet,
    doubles: BoolSet,
    rotate: BoolSet,
    // Render
    render: BoolSet,
    shape_mode: EnumSet,
    side_color: ColorSet,
    line_color: ColorSet,
    // Runtime
    target: Option<EntityInfo>,
    place_positions: Vec<BlockPos>,
}

impl AutoWeb {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Combat,
            "auto-web",
            "Automatically places webs on other players.",
        );
        let s = base.settings_mut();
        let g = s.default_group();
        let place_range = g.double_setting("place-range", 4.0, 0.0, 6.0, 2);
        let place_walls_range = g.double_setting("walls-range", 4.0, 0.0, 6.0, 2);
        let priority = g.enum_setting("target-priority", SORT_PRIORITIES, SORT_LOWEST_DISTANCE);
        let target_range = g.double_setting("target-range", 10.0, 0.0, 30.0, 2);
        let predict_movement = g.bool_setting("predict-movement", true);
        let ticks_to_predict = g.double_setting("ticks-to-predict", 10.0, 1.0, 30.0, 0);
        let doubles = g.bool_setting("doubles", false);
        let rotate = g.bool_setting("rotate", true);

        s.create_group("Render");
        let render = last_group(s).bool_setting("render", true);
        let shape_mode = last_group(s).enum_setting("shape-mode", SHAPE_MODES, SHAPE_BOTH);
        let side_color =
            last_group(s).color_setting("side-color", SettingColor::new(239, 231, 244, 31));
        let line_color =
            last_group(s).color_setting("line-color", SettingColor::new(255, 255, 255, 255));

        Self {
            base,
            place_range,
            place_walls_range,
            priority,
            target_range,
            predict_movement,
            ticks_to_predict,
            doubles,
            rotate,
            render,
            shape_mode,
            side_color,
            line_color,
            target: None,
            place_positions: Vec::new(),
        }
    }

    /// `AutoWeb.canPlaceWebAt`.
    fn can_place_web_at(&self, env: &dyn Env, pos: BlockPos) -> bool {
        if !can_place_at(env, pos) {
            return false;
        }
        let Some(player) = env.player() else {
            return false;
        };
        in_place_range(
            env,
            player.eye_pos(),
            pos,
            self.place_range.get(),
            self.place_walls_range.get(),
        )
    }
}

impl Default for AutoWeb {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AutoWeb {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.target = None;
    }

    fn on_deactivate(&mut self, _env: &mut dyn Env) {
        self.place_positions.clear();
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        self.place_positions.clear();

        if is_bad_target(env, self.target.as_ref(), self.target_range.get()) {
            self.target = player_target(env, self.target_range.get(), self.priority.index());
            if is_bad_target(env, self.target.as_ref(), self.target_range.get()) {
                return;
            }
        }
        let Some(target) = self.target.clone() else {
            return;
        };
        if super::find_hotbar_item(env, &["minecraft:cobweb"]).is_none() {
            return;
        }

        let mut pos = target.pos;
        if self.predict_movement.get() {
            pos = pos.add(target.velocity.mul(self.ticks_to_predict.get()));
        }
        let block_pos = pos.to_block_pos();
        let above = block_pos.offset(0, 1, 0);

        let rotate = self.rotate.get();
        if self.can_place_web_at(env, block_pos) {
            place_block(env, block_pos, "minecraft:cobweb", rotate, false);
            self.place_positions.push(block_pos);
        }
        if self.doubles.get() && self.can_place_web_at(env, above) {
            place_block(env, above, "minecraft:cobweb", rotate, false);
            self.place_positions.push(above);
        }
    }

    fn on_render_world(&mut self, _env: &mut dyn Env, r: &mut dyn Renderer) {
        if !self.render.get() || self.place_positions.is_empty() {
            return;
        }
        let mode = self.shape_mode.clone();
        for pos in self.place_positions.clone() {
            draw_shape(
                r,
                pos,
                &mode,
                self.side_color.color().argb(),
                self.line_color.color().argb(),
            );
        }
    }

    fn info_string(&self) -> Option<String> {
        self.target
            .as_ref()
            .map(|t| t.name.clone().unwrap_or_else(|| t.short_kind().to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{InvSlot, ItemStackInfo, NullRenderer};
    use crate::meteor_core::math::Vec3d;

    fn web_env() -> TestEnv {
        let mut env = TestEnv::in_world();
        env.entities.push(EntityInfo {
            id: 11,
            kind: "minecraft:player".into(),
            name: Some("Victim".into()),
            pos: Vec3d::new(0.5, 64.0, -2.5),
            health: 20.0,
            max_health: 20.0,
            alive: true,
            is_player: true,
            ..Default::default()
        });
        env.inventory.push(InvSlot {
            index: 0,
            stack: Some(ItemStackInfo {
                id: "minecraft:cobweb".into(),
                count: 8,
                ..Default::default()
            }),
        });
        env.player.as_mut().unwrap().selected_slot = 0;
        env.push_block(BlockPos::new(0, 63, -3), "minecraft:stone");
        env
    }

    #[test]
    fn webs_the_target() {
        let mut m = AutoWeb::new();
        let mut env = web_env();
        m.on_activate(&mut env);
        m.on_tick(&mut env);

        assert!(!env.placed.is_empty(), "a web should be placed");
        assert_eq!(env.placed[0].1, "minecraft:cobweb");
        assert_eq!(m.info_string().as_deref(), Some("Victim"));
    }

    #[test]
    fn renders_the_web_positions() {
        let mut m = AutoWeb::new();
        let mut env = web_env();
        m.on_tick(&mut env);
        let mut r = NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
        assert!(r.boxes > 0);
    }
}
