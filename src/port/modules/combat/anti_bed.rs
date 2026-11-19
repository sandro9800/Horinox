//! `systems.modules.combat.AntiBed` - Places string to prevent beds being placed on you.
//!
//! Ports the Java tick: mine a bed that appears in your head block, then fill the
//! configured body positions with string.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, PacketInfo};
use crate::meteor_core::math::{BlockPos, Dir};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::BoolSet;

use super::{place_block, replaceable};

pub struct AntiBed {
    base: ModuleBase,
    place_string_top: BoolSet,
    place_string_middle: BoolSet,
    place_string_bottom: BoolSet,
    only_in_hole: BoolSet,
    breaking: bool,
}

impl AntiBed {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Combat,
            "anti-bed",
            "Places string to prevent beds being placed on you.",
        );
        let s = base.settings_mut();
        let g = s.default_group();
        let place_string_top = g.bool_setting("place-string-top", false);
        let place_string_middle = g.bool_setting("place-string-middle", true);
        let place_string_bottom = g.bool_setting("place-string-bottom", false);
        let only_in_hole = g.bool_setting("only-in-hole", true);
        Self {
            base,
            place_string_top,
            place_string_middle,
            place_string_bottom,
            only_in_hole,
            breaking: false,
        }
    }

    /// `PlayerUtils.isInHole(true)`.
    fn in_hole(env: &dyn Env, pos: BlockPos) -> bool {
        let solid = |p: BlockPos| env.block(p).map(|b| !replaceable(&b)).unwrap_or(false);
        [Dir::North, Dir::South, Dir::West, Dir::East]
            .iter()
            .all(|d| solid(pos.offset_dir(*d)))
            && solid(pos.offset(0, -1, 0))
    }

    /// `AntiBed.place`.
    fn place(&self, env: &mut dyn Env, pos: BlockPos) {
        let is_string = env
            .block(pos)
            .map(|b| b.id == "minecraft:string")
            .unwrap_or(false);
        if !is_string {
            place_block(env, pos, "minecraft:string", false, true);
        }
    }

    fn action_packet(action: &str, pos: BlockPos) -> PacketInfo {
        PacketInfo {
            name: "minecraft:player_action".into(),
            clientbound: false,
            fields: vec![
                ("action".into(), action.into()),
                ("pos".into(), format!("{}, {}, {}", pos.x, pos.y, pos.z)),
                ("face".into(), Dir::Up.name().into()),
            ],
        }
    }
}

impl Default for AntiBed {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AntiBed {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        let Some(player) = env.player() else { return };
        let pos = player.pos.to_block_pos();
        if self.only_in_hole.get() && !Self::in_hole(env, pos) {
            return;
        }

        let head = pos.offset(0, 1, 0);
        let bed = env
            .block(head)
            .map(|b| b.id.ends_with("_bed"))
            .unwrap_or(false);
        if bed && !self.breaking {
            if let Some(p) = env.player() {
                env.set_rotation(crate::meteor_core::math::Rotation::between(
                    p.eye_pos(),
                    head.center(),
                ));
            }
            env.send_packet(Self::action_packet("start_destroy_block", head));
            env.send_packet(Self::action_packet("stop_destroy_block", head));
            self.breaking = true;
        } else if self.breaking {
            env.send_packet(Self::action_packet("abort_destroy_block", head));
            self.breaking = false;
        }

        if self.place_string_top.get() {
            self.place(env, pos.offset(0, 2, 0));
        }
        if self.place_string_middle.get() {
            self.place(env, pos.offset(0, 1, 0));
        }
        if self.place_string_bottom.get() {
            self.place(env, pos);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{InvSlot, ItemStackInfo};

    fn hole_env() -> TestEnv {
        let mut env = TestEnv::in_world();
        for (x, z) in [(0, 0), (-1, 0), (1, 0), (0, -1), (0, 1)] {
            env.push_block(BlockPos::new(x, 63, z), "minecraft:obsidian");
        }
        for (x, z) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            env.push_block(BlockPos::new(x, 64, z), "minecraft:obsidian");
        }
        env.inventory.push(InvSlot {
            index: 0,
            stack: Some(ItemStackInfo {
                id: "minecraft:string".into(),
                count: 16,
                ..Default::default()
            }),
        });
        env.player.as_mut().unwrap().selected_slot = 0;
        env
    }

    #[test]
    fn fills_the_body_with_string() {
        let mut m = AntiBed::new();
        let mut env = hole_env();
        m.on_tick(&mut env);
        assert!(
            env.placed
                .iter()
                .any(|(p, _)| *p == BlockPos::new(0, 65, 0)),
            "middle string missing: {:?}",
            env.placed
        );
    }

    #[test]
    fn mines_a_bed_in_the_head_block() {
        let mut m = AntiBed::new();
        let mut env = hole_env();
        env.push_block(BlockPos::new(0, 65, 0), "minecraft:red_bed");
        m.on_tick(&mut env);
        assert!(env
            .packets
            .iter()
            .any(|p| p.name == "minecraft:player_action"));
        assert!(m.breaking);
    }
}
