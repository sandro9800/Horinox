//! `systems.modules.player.NoInteract` - blocks interactions with certain types
//! of inputs.
//!
//! Java listens to `StartBreakingBlockEvent`, `InteractBlockEvent`,
//! `AttackEntityEvent` and `InteractEntityEvent`; the port answers the same
//! questions from the `player_action` packet and the mouse presses.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{EntityInfo, Env, Hand, PacketInfo};
use crate::meteor_core::math::BlockPos;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{EnumSet, ListSet};

use super::{bare, describe, field, list_allows, parse_pos};

pub struct NoInteract {
    base: ModuleBase,
    // Blocks
    block_mine: ListSet,
    block_mine_mode: EnumSet,
    block_interact: ListSet,
    block_interact_mode: EnumSet,
    block_interact_hand: EnumSet,
    // Entities
    entity_hit: ListSet,
    entity_hit_mode: EnumSet,
    entity_interact: ListSet,
    entity_interact_mode: EnumSet,
    entity_interact_hand: EnumSet,
    friends: EnumSet,
    babies: EnumSet,
    nametagged: EnumSet,
}

impl NoInteract {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Player,
            "no-interact",
            "Blocks interactions with certain types of inputs.",
        );
        let (block_mine, block_mine_mode, block_interact, block_interact_mode, block_interact_hand);
        let (entity_hit, entity_hit_mode, entity_interact, entity_interact_mode);
        let (entity_interact_hand, friends, babies, nametagged);
        {
            let mut s = base.settings_mut();
            let b = s.create_group("Blocks");
            block_mine = b.list_setting("block-mine", &[]);
            block_mine_mode = b.enum_setting("block-mine-mode", &["whitelist", "blacklist"], 1);
            block_interact = b.list_setting("block-interact", &[]);
            block_interact_mode =
                b.enum_setting("block-interact-mode", &["whitelist", "blacklist"], 1);
            block_interact_hand = b.enum_setting(
                "block-interact-hand",
                &["mainhand", "offhand", "both", "none"],
                3,
            );
            let e = s.create_group("Entities");
            entity_hit = e.list_setting("entity-hit", &[]);
            entity_hit_mode = e.enum_setting("entity-hit-mode", &["whitelist", "blacklist"], 1);
            entity_interact = e.list_setting("entity-interact", &[]);
            entity_interact_mode =
                e.enum_setting("entity-interact-mode", &["whitelist", "blacklist"], 1);
            entity_interact_hand = e.enum_setting(
                "entity-interact-hand",
                &["mainhand", "offhand", "both", "none"],
                3,
            );
            friends = e.enum_setting("friends", &["hit", "interact", "both", "none"], 3);
            babies = e.enum_setting("babies", &["hit", "interact", "both", "none"], 3);
            nametagged = e.enum_setting("nametagged", &["hit", "interact", "both", "none"], 3);
        }
        let mut s = base.settings_mut();
        describe(&mut s, "block-mine", "Cancels block mining.");
        describe(
            &mut s,
            "block-mine-mode",
            "List mode to use for block mine.",
        );
        describe(&mut s, "block-interact", "Cancels block interaction.");
        describe(
            &mut s,
            "block-interact-mode",
            "List mode to use for block interact.",
        );
        describe(
            &mut s,
            "block-interact-hand",
            "Cancels block interaction if performed by this hand.",
        );
        describe(&mut s, "entity-hit", "Cancel entity hitting.");
        describe(
            &mut s,
            "entity-hit-mode",
            "List mode to use for entity hit.",
        );
        describe(&mut s, "entity-interact", "Cancel entity interaction.");
        describe(
            &mut s,
            "entity-interact-mode",
            "List mode to use for entity interact.",
        );
        describe(
            &mut s,
            "entity-interact-hand",
            "Cancels entity interaction if performed by this hand.",
        );
        describe(&mut s, "friends", "Friends cancel mode.");
        describe(&mut s, "babies", "Baby entity cancel mode.");
        describe(&mut s, "nametagged", "Nametagged entity cancel mode.");

        Self {
            base,
            block_mine,
            block_mine_mode,
            block_interact,
            block_interact_mode,
            block_interact_hand,
            entity_hit,
            entity_hit_mode,
            entity_interact,
            entity_interact_mode,
            entity_interact_hand,
            friends,
            babies,
            nametagged,
        }
    }

    fn in_list(list: &ListSet, id: &str) -> bool {
        list.contains(id) || list.contains(bare(id))
    }

    fn hand_blocked(mode: &EnumSet, hand: Hand) -> bool {
        match mode.get() {
            "both" => true,
            "mainhand" => hand == Hand::Main,
            "offhand" => hand == Hand::Off,
            _ => false,
        }
    }

    fn mode_includes(mode: &EnumSet, name: &str) -> bool {
        mode.is("both") || mode.is(name)
    }

    /// `NoInteract.shouldAttackBlock`.
    pub fn should_attack_block(&self, env: &dyn Env, pos: BlockPos) -> bool {
        let contains = env
            .block(pos)
            .map(|b| Self::in_list(&self.block_mine, &b.id))
            .unwrap_or(false);
        list_allows(self.block_mine_mode.is("whitelist"), contains)
    }

    /// `NoInteract.shouldInteractBlock`.
    pub fn should_interact_block(&self, env: &dyn Env, pos: BlockPos, hand: Hand) -> bool {
        if Self::hand_blocked(&self.block_interact_hand, hand) {
            return false;
        }
        let contains = env
            .block(pos)
            .map(|b| Self::in_list(&self.block_interact, &b.id))
            .unwrap_or(false);
        list_allows(self.block_interact_mode.is("whitelist"), contains)
    }

    /// `NoInteract.shouldAttackEntity`.
    pub fn should_attack_entity(&self, env: &dyn Env, entity: &EntityInfo) -> bool {
        if Self::mode_includes(&self.friends, "hit") && entity.is_player {
            if let Some(name) = &entity.name {
                if env.is_friend(name) {
                    return false;
                }
            }
        }
        if Self::mode_includes(&self.babies, "hit") && entity.baby {
            return false;
        }
        if Self::mode_includes(&self.nametagged, "hit") && entity.name.is_some() {
            return false;
        }
        let contains = Self::in_list(&self.entity_hit, &entity.kind);
        list_allows(self.entity_hit_mode.is("whitelist"), contains)
    }

    /// `NoInteract.shouldInteractEntity`.
    pub fn should_interact_entity(&self, env: &dyn Env, entity: &EntityInfo, hand: Hand) -> bool {
        if Self::hand_blocked(&self.entity_interact_hand, hand) {
            return false;
        }
        if Self::mode_includes(&self.friends, "interact") && entity.is_player {
            if let Some(name) = &entity.name {
                if env.is_friend(name) {
                    return false;
                }
            }
        }
        if Self::mode_includes(&self.babies, "interact") && entity.baby {
            return false;
        }
        if Self::mode_includes(&self.nametagged, "interact") && entity.name.is_some() {
            return false;
        }
        let contains = Self::in_list(&self.entity_interact, &entity.kind);
        list_allows(self.entity_interact_mode.is("whitelist"), contains)
    }
}

impl Default for NoInteract {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for NoInteract {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_send_packet(&mut self, env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        if !packet.name.eq_ignore_ascii_case("player_action") {
            return true;
        }
        let action = field(packet, "action").unwrap_or("").to_ascii_lowercase();
        if action != "start_destroy_block" {
            return true;
        }
        let Some(pos) = field(packet, "pos").and_then(parse_pos) else {
            return true;
        };
        self.should_attack_block(env, pos)
    }

    fn on_key(&mut self, env: &mut dyn Env, is_key: bool, value: i32, pressed: bool) -> bool {
        if is_key || !pressed {
            return false;
        }
        match value {
            // Left click: the entity attack.
            0 => {
                let Some(entity) = env.crosshair_entity() else {
                    return false;
                };
                !self.should_attack_entity(env, &entity)
            }
            // Right click: block or entity interaction with the main hand.
            1 => {
                if let Some(entity) = env.crosshair_entity() {
                    return !self.should_interact_entity(env, &entity, Hand::Main);
                }
                if let Some(pos) = env.raycast(4.5).block_pos() {
                    return !self.should_interact_block(env, pos, Hand::Main);
                }
                false
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::EntityInfo;
    use crate::port::modules::player::format_pos;

    fn start_break(pos: BlockPos) -> PacketInfo {
        PacketInfo {
            name: "player_action".to_string(),
            clientbound: false,
            fields: vec![
                ("action".to_string(), "start_destroy_block".to_string()),
                ("pos".to_string(), format_pos(pos)),
            ],
        }
    }

    #[test]
    fn a_blacklisted_block_is_not_mined() {
        let mut m = NoInteract::new();
        let mut env = TestEnv::in_world();
        m.block_mine.set(vec!["minecraft:stone".to_string()]);
        env.push_block(BlockPos::new(0, 65, 0), "minecraft:stone");
        env.push_block(BlockPos::new(1, 65, 0), "minecraft:dirt");

        let mut stone = start_break(BlockPos::new(0, 65, 0));
        assert!(
            !m.on_send_packet(&mut env, &mut stone),
            "stone is cancelled"
        );

        let mut dirt = start_break(BlockPos::new(1, 65, 0));
        assert!(m.on_send_packet(&mut env, &mut dirt), "dirt still mines");
    }

    #[test]
    fn nametagged_entities_are_not_attacked() {
        let mut m = NoInteract::new();
        let mut env = TestEnv::in_world();
        m.nametagged.set_index(0); // Hit
        let entity = EntityInfo {
            id: 9,
            kind: "minecraft:zombie".to_string(),
            name: Some("Bob".to_string()),
            is_mob: true,
            ..Default::default()
        };
        assert!(!m.should_attack_entity(&env, &entity));

        let plain = EntityInfo {
            id: 10,
            kind: "minecraft:zombie".to_string(),
            is_mob: true,
            ..Default::default()
        };
        assert!(m.should_attack_entity(&env, &plain));
    }
}
