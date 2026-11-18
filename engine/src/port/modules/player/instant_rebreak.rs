//! `systems.modules.player.InstantRebreak` - instantly re-breaks blocks in the
//! same position.
//!
//! The Java module listens to `StartBreakingBlockEvent` (a hook Horizon does not
//! have) and sends `ServerboundPlayerActionPacket(STOP_DESTROY_BLOCK)` plus a
//! punch packet; the port starts from the left mouse press and sends the same
//! `player_action` through `send_packet`, with `Action::Attack` for the punch.

use crate::meteor_core::category::Category;
use crate::meteor_core::color::SettingColor;
use crate::meteor_core::env::{Action, Env, PacketInfo, Renderer};
use crate::meteor_core::math::{BlockPos, Dir, Rotation, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, ColorSet, EnumSet, IntSet};

use super::{describe, is_pickaxe, visible};

pub struct InstantRebreak {
    base: ModuleBase,
    // General
    delay: IntSet,
    only_pick: BoolSet,
    rotate: BoolSet,
    // Render
    render: BoolSet,
    shape_mode: EnumSet,
    side_color: ColorSet,
    line_color: ColorSet,
    /// `blockPos` - `BlockPos.MutableBlockPos(0, Integer.MIN_VALUE, 0)` in Java.
    pub block_pos: BlockPos,
    direction: Option<Dir>,
    ticks: i32,
}

impl InstantRebreak {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Player,
            "instant-rebreak",
            "Instantly re-breaks blocks in the same position.",
        );
        let (delay, only_pick, rotate, render, shape_mode, side_color, line_color);
        {
            let mut s = base.settings_mut();
            let g = s.default_group();
            delay = g.int_setting("delay", 0, 0, 20);
            only_pick = g.bool_setting("only-pick", true);
            rotate = g.bool_setting("rotate", true);
            let r = s.create_group("Render");
            render = r.bool_setting("render", true);
            shape_mode = r.enum_setting("shape-mode", &["lines", "sides", "both", "none"], 2);
            side_color = r.color_setting("side-color", SettingColor::new(204, 0, 0, 10));
            line_color = r.color_setting("line-color", SettingColor::new(204, 0, 0, 255));
        }
        let mut s = base.settings_mut();
        describe(&mut s, "delay", "The delay between break attempts.");
        describe(
            &mut s,
            "only-pick",
            "Only tries to mine the block if you are holding a pickaxe.",
        );
        describe(&mut s, "rotate", "Faces the block being mined server side.");
        describe(
            &mut s,
            "render",
            "Renders an overlay on the block being broken.",
        );
        describe(&mut s, "shape-mode", "How the shapes are rendered.");
        describe(
            &mut s,
            "side-color",
            "The color of the sides of the blocks being rendered.",
        );
        describe(
            &mut s,
            "line-color",
            "The color of the lines of the blocks being rendered.",
        );

        Self {
            base,
            delay,
            only_pick,
            rotate,
            render,
            shape_mode,
            side_color,
            line_color,
            block_pos: BlockPos::new(0, i32::MIN, 0),
            direction: None,
            ticks: 0,
        }
    }

    /// `InstantRebreak.shouldMine`.
    pub fn should_mine(&self, env: &dyn Env) -> bool {
        let pos = self.block_pos;
        let Some(world) = env.world() else {
            return false;
        };
        if pos.y < world.min_y || pos.y >= world.min_y + world.height {
            return false;
        }
        let Some(block) = env.block(pos) else {
            return false;
        };
        if block.is_air() {
            return false;
        }
        if !self.only_pick.get() {
            return true;
        }
        env.selected_stack()
            .map(|s| is_pickaxe(&s.id))
            .unwrap_or(false)
    }

    /// `InstantRebreak.sendPacket`: `STOP_DESTROY_BLOCK` at the remembered pos.
    fn send_packet(&self, env: &mut dyn Env) {
        let pos = self.block_pos;
        let face = self.direction.unwrap_or(Dir::Up);
        env.send_packet(PacketInfo {
            name: "player_action".to_string(),
            clientbound: false,
            fields: vec![
                ("action".to_string(), "stop_destroy_block".to_string()),
                ("pos".to_string(), format!("{},{},{}", pos.x, pos.y, pos.z)),
                ("face".to_string(), face.name().to_string()),
                ("sequence".to_string(), "0".to_string()),
            ],
        });
    }
}

impl Default for InstantRebreak {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for InstantRebreak {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.ticks = 0;
        self.block_pos = BlockPos::new(0, -1, 0);
        self.direction = None;
    }

    fn on_key(&mut self, env: &mut dyn Env, is_key: bool, value: i32, pressed: bool) -> bool {
        // `StartBreakingBlockEvent`: remember the block the left click starts on.
        if is_key || value != 0 || !pressed {
            return false;
        }
        if let Some(pos) = env.raycast(4.5).block_pos() {
            self.block_pos = pos;
            self.direction = None;
            self.ticks = 0;
        }
        false
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        if self.ticks >= self.delay.get() {
            self.ticks = 0;

            if self.should_mine(env) {
                if self.rotate.get() {
                    if let Some(player) = env.player() {
                        let target = self.block_pos.center();
                        let rotation = Rotation::between(player.eye_pos(), target);
                        env.set_rotation(rotation);
                    }
                }
                self.send_packet(env);
                // `mc.getConnection().send(new ServerboundPunchPacket())`
                env.action(Action::Attack);
            }
        } else {
            self.ticks += 1;
        }
    }

    fn on_render_world(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        if !self.render.get() || !self.should_mine(env) {
            return;
        }
        let min = self.block_pos.as_vec();
        let max = min.add(Vec3d::new(1.0, 1.0, 1.0));
        match self.shape_mode.index() {
            0 => r.box_outline(min, max, self.line_color.color().argb(), 1.0, false),
            1 => r.filled_box(min, max, self.side_color.color().argb(), false),
            2 => {
                r.filled_box(min, max, self.side_color.color().argb(), false);
                r.box_outline(min, max, self.line_color.color().argb(), 1.0, false);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{InvSlot, NullRenderer};
    use crate::port::modules::player::{field, HOTBAR_START};

    fn pickaxe() -> crate::meteor_core::env::ItemStackInfo {
        crate::meteor_core::env::ItemStackInfo {
            id: "minecraft:diamond_pickaxe".to_string(),
            count: 1,
            ..Default::default()
        }
    }

    #[test]
    fn the_remembered_block_is_re_broken_every_tick() {
        let mut m = InstantRebreak::new();
        let mut env = TestEnv::in_world();
        env.inventory.push(InvSlot {
            index: HOTBAR_START,
            stack: Some(pickaxe()),
        });
        env.push_block(BlockPos::new(0, 65, 3), "minecraft:stone");

        m.on_activate(&mut env);
        m.on_key(&mut env, false, 0, true);
        assert_eq!(m.block_pos, BlockPos::new(0, 65, 3));

        m.on_tick(&mut env);

        assert!(env.packets.iter().any(|p| {
            p.name == "player_action"
                && field(p, "action") == Some("stop_destroy_block")
                && field(p, "pos") == Some("0,65,3")
        }));
        assert!(env.actions.contains(&Action::Attack), "the punch is swung");
        assert_ne!(env.rotation, Rotation::default(), "the block is faced");

        let mut r = NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
        assert!(r.boxes > 0);
    }

    #[test]
    fn air_is_not_re_broken() {
        let mut m = InstantRebreak::new();
        let mut env = TestEnv::in_world();
        env.inventory.push(InvSlot {
            index: HOTBAR_START,
            stack: Some(pickaxe()),
        });
        m.on_activate(&mut env);
        assert!(!m.should_mine(&env));
        m.on_tick(&mut env);
        assert!(env.packets.is_empty());
    }
}
