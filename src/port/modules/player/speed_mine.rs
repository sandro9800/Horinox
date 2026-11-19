//! `systems.modules.player.SpeedMine` - allows you to quickly mine blocks.
//!
//! PORT GAP: `MultiPlayerGameModeAccessor`'s breaking progress (the `Damage` and
//! `Normal` modes) and `player.addEffect(HASTE)` (the `Haste` mode) are not part
//! of `Env`. What the port can express is the wire behaviour: in `Damage` mode the
//! break is finished as soon as a filtered block is mined, and `grim-bypass`
//! sends the extra `ABORT_DESTROY_BLOCK` Grim 2.3.58 expects.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Action, Env, PacketInfo};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet, EnumSet, IntSet, ListSet};

use super::{bare, describe, field, format_pos, list_allows, parse_pos, visible};

pub struct SpeedMine {
    base: ModuleBase,
    mode: EnumSet,
    blocks: ListSet,
    blocks_filter: EnumSet,
    modifier: DoubleSet,
    haste_amplifier: IntSet,
    instamine: BoolSet,
    grim_bypass: BoolSet,
    mining: bool,
}

impl SpeedMine {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Player,
            "speed-mine",
            "Allows you to quickly mine blocks.",
        );
        let (mode, blocks, blocks_filter, modifier, haste_amplifier, instamine, grim_bypass);
        {
            let mut s = base.settings_mut();
            let g = s.default_group();
            mode = g.enum_setting("mode", &["normal", "haste", "damage"], 2);
            blocks = g.list_setting("blocks", &[]);
            blocks_filter = g.enum_setting("blocks-filter", &["whitelist", "blacklist"], 1);
            modifier = g.double_setting("modifier", 1.4, 0.0, 3.0, 2);
            haste_amplifier = g.int_setting("haste-amplifier", 2, 1, 10);
            instamine = g.bool_setting("instamine", true);
            grim_bypass = g.bool_setting("grim-bypass", false);
        }
        let mut s = base.settings_mut();
        describe(&mut s, "blocks", "Selected blocks.");
        describe(&mut s, "blocks-filter", "How to use the blocks setting.");
        describe(
            &mut s,
            "modifier",
            "Mining speed modifier. An additional value of 0.2 is equivalent to one haste level (1.2 = haste 1).",
        );
        describe(
            &mut s,
            "haste-amplifier",
            "What value of haste to give you. Above 2 not recommended.",
        );
        describe(
            &mut s,
            "instamine",
            "Whether or not to instantly mine blocks under certain conditions.",
        );
        describe(
            &mut s,
            "grim-bypass",
            "Bypasses Grim's fastbreak check, working as of 2.3.58",
        );
        visible(&mut s, "blocks", {
            let c = mode.clone();
            move || !c.is("haste")
        });
        visible(&mut s, "blocks-filter", {
            let c = mode.clone();
            move || !c.is("haste")
        });
        visible(&mut s, "modifier", {
            let c = mode.clone();
            move || c.is("normal")
        });
        visible(&mut s, "haste-amplifier", {
            let c = mode.clone();
            move || c.is("haste")
        });
        visible(&mut s, "instamine", {
            let c = mode.clone();
            move || c.is("damage")
        });
        visible(&mut s, "grim-bypass", {
            let c = mode.clone();
            move || c.is("damage")
        });

        Self {
            base,
            mode,
            blocks,
            blocks_filter,
            modifier,
            haste_amplifier,
            instamine,
            grim_bypass,
            mining: false,
        }
    }

    /// `SpeedMine.filter(block)`.
    pub fn filter(&self, block_id: &str) -> bool {
        let contains = self.blocks.contains(block_id) || self.blocks.contains(bare(block_id));
        list_allows(self.blocks_filter.is("whitelist"), contains)
    }

    /// `SpeedMine.instamine()`.
    pub fn instamine(&self) -> bool {
        self.mode.is("damage") && self.instamine.get()
    }

    fn send_stop_destroy(&self, env: &mut dyn Env, pos: crate::meteor_core::math::BlockPos) {
        env.send_packet(PacketInfo {
            name: "player_action".to_string(),
            clientbound: false,
            fields: vec![
                ("action".to_string(), "stop_destroy_block".to_string()),
                ("pos".to_string(), format_pos(pos)),
                ("face".to_string(), "up".to_string()),
                ("sequence".to_string(), "0".to_string()),
            ],
        });
    }
}

impl Default for SpeedMine {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for SpeedMine {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.mining = false;
    }

    fn on_deactivate(&mut self, _env: &mut dyn Env) {
        // `removeHaste()`: there is no client-side Haste effect to remove.
        self.mining = false;
    }

    fn on_key(&mut self, _env: &mut dyn Env, is_key: bool, value: i32, pressed: bool) -> bool {
        if !is_key && value == 0 {
            self.mining = pressed;
        }
        false
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        if !env.can_update() {
            return;
        }

        match self.mode.get() {
            // PORT GAP: `player.addEffect(new MobEffectInstance(HASTE, -1, ...))`.
            "haste" => {
                let _ = (self.modifier.get(), self.haste_amplifier.get());
            }
            "damage" => {
                // PORT GAP: the breaking-progress accessor the mixin writes; the
                // port finishes the filtered break on the wire instead.
                if !self.instamine() || !self.mining {
                    return;
                }
                let Some(pos) = env.raycast(4.5).block_pos() else {
                    return;
                };
                let Some(block) = env.block(pos) else { return };
                if block.is_air() || !self.filter(&block.id) {
                    return;
                }
                self.send_stop_destroy(env, pos);
                env.action(Action::Attack);
            }
            // `Normal` mode needs `modifier` inside `getDestroyProgress`.
            _ => {}
        }
    }

    fn on_send_packet(&mut self, env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        if !self.mode.is("damage") || !self.grim_bypass.get() {
            return true;
        }
        if !packet.name.eq_ignore_ascii_case("player_action") {
            return true;
        }
        if !field(packet, "action")
            .map(|a| a.eq_ignore_ascii_case("stop_destroy_block"))
            .unwrap_or(false)
        {
            return true;
        }
        let Some(pos) = field(packet, "pos").and_then(parse_pos) else {
            return true;
        };
        let face = field(packet, "face").unwrap_or("up").to_string();

        // https://github.com/GrimAnticheat/Grim/issues/1296
        env.send_packet(PacketInfo {
            name: "player_action".to_string(),
            clientbound: false,
            fields: vec![
                ("action".to_string(), "abort_destroy_block".to_string()),
                ("pos".to_string(), format_pos(pos.offset(0, 1, 0))),
                ("face".to_string(), face),
                ("sequence".to_string(), "0".to_string()),
            ],
        });

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{InvSlot, ItemStackInfo};
    use crate::meteor_core::math::BlockPos;
    use crate::port::modules::player::HOTBAR_START;

    #[test]
    fn instamine_finishes_the_filtered_block() {
        let mut m = SpeedMine::new();
        let mut env = TestEnv::in_world();
        env.inventory.push(InvSlot {
            index: HOTBAR_START,
            stack: Some(ItemStackInfo {
                id: "minecraft:diamond_pickaxe".to_string(),
                count: 1,
                ..Default::default()
            }),
        });
        env.push_block(BlockPos::new(0, 65, 3), "minecraft:stone");

        m.on_key(&mut env, false, 0, true);
        m.on_tick(&mut env);

        assert!(env.packets.iter().any(|p| {
            p.name == "player_action"
                && field(p, "action") == Some("stop_destroy_block")
                && field(p, "pos") == Some("0,65,3")
        }));
        assert!(env.actions.contains(&Action::Attack));
    }

    #[test]
    fn grim_bypass_sends_the_abort_above_the_block() {
        let mut m = SpeedMine::new();
        let mut env = TestEnv::in_world();
        m.grim_bypass.set(true);

        let mut packet = PacketInfo {
            name: "player_action".to_string(),
            clientbound: false,
            fields: vec![
                ("action".to_string(), "stop_destroy_block".to_string()),
                ("pos".to_string(), "0,65,3".to_string()),
                ("face".to_string(), "up".to_string()),
            ],
        };
        assert!(m.on_send_packet(&mut env, &mut packet));

        assert!(env.packets.iter().any(|p| {
            field(p, "action") == Some("abort_destroy_block") && field(p, "pos") == Some("0,66,3")
        }));
    }
}
