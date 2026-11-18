//! `systems.modules.world.AutoNametag` - "automatically uses nametags on
//! entities without a nametag. WILL nametag ALL entities in the specified
//! distance."

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{ClickMode, EntityInfo, Env};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet, EnumSet, ListSet};

use super::{
    describe, entity_center, entity_within, hotbar_slot, interact_entity, listed, stack_in,
};

pub struct AutoNametag {
    base: ModuleBase,
    entities: ListSet,
    range: DoubleSet,
    priority: EnumSet,
    renametag: BoolSet,
    rotate: BoolSet,
    /// `Object2IntMap<Entity> entityCooldowns`, as `(entity id, ticks)`.
    cooldowns: Vec<(i32, i32)>,
    target: Option<i32>,
    off_hand: bool,
}

impl AutoNametag {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::World,
            "auto-nametag",
            "Automatically uses nametags on entities without a nametag. WILL nametag ALL entities in the specified distance.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();

        // PORT GAP: `EntityTypeListSetting` - Horizon has no entity-type registry
        // to filter against, so the list holds ids as text.
        let entities = g.list_setting("entities", &[]);
        describe(g, "Which entities to nametag.");

        let range = g.double_setting("range", 5.0, 0.0, 6.0, 2);
        describe(g, "The maximum range an entity can be to be nametagged.");

        let priority = g.enum_setting(
            "priority",
            &[
                "lowest-distance",
                "highest-distance",
                "lowest-health",
                "highest-health",
            ],
            0,
        );
        describe(g, "Priority sort");

        let renametag = g.bool_setting("renametag", true);
        describe(g, "Allows already nametagged entities to be renamed.");

        let rotate = g.bool_setting("rotate", true);
        describe(g, "Automatically faces towards the mob being nametagged.");

        Self {
            base,
            entities,
            range,
            priority,
            renametag,
            rotate,
            cooldowns: Vec::new(),
            target: None,
            off_hand: false,
        }
    }

    /// `TargetUtils.get(...)`: the nearest/healthiest entity that still wants a
    /// nametag, under the `priority` sort.
    fn find_target(&self, env: &dyn Env, nametag_name: Option<&str>) -> Option<EntityInfo> {
        let mut candidates: Vec<EntityInfo> = env
            .entities()
            .into_iter()
            .filter(|entity| {
                entity_within(env, entity, self.range.get())
                    && listed(&self.entities, &entity.kind)
                    && self.cooldown(entity.id) <= 0
            })
            .filter(|entity| match &entity.name {
                // `entity.hasCustomName() && (!renametag || name == displayName)`
                Some(name) => {
                    if !self.renametag.get() {
                        return false;
                    }
                    match nametag_name {
                        Some(held) => name != held,
                        None => false,
                    }
                }
                None => true,
            })
            .collect();

        let order = self.priority.get();
        let at = env.player().map(|p| p.pos).unwrap_or_default();
        let key = |e: &EntityInfo| -> f64 {
            match order {
                "highest-distance" => -at.distance_to(e.pos),
                "lowest-health" => e.health as f64,
                "highest-health" => -(e.health as f64),
                _ => at.distance_to(e.pos),
            }
        };
        candidates.sort_by(|a, b| {
            key(a)
                .partial_cmp(&key(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        candidates.into_iter().next()
    }

    fn cooldown(&self, id: i32) -> i32 {
        self.cooldowns
            .iter()
            .find(|(entity, _)| *entity == id)
            .map(|(_, ticks)| *ticks)
            .unwrap_or(0)
    }

    fn cool(&mut self, id: i32, ticks: i32) {
        if let Some(entry) = self.cooldowns.iter_mut().find(|(entity, _)| *entity == id) {
            entry.1 = ticks;
        } else {
            self.cooldowns.push((id, ticks));
        }
    }

    /// The entity the module nametagged last, if any.
    pub fn target(&self) -> Option<i32> {
        self.target
    }

    /// Whether the nametag came from the off hand.
    pub fn used_off_hand(&self) -> bool {
        self.off_hand
    }
}

impl Default for AutoNametag {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AutoNametag {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_deactivate(&mut self, _env: &mut dyn Env) {
        self.cooldowns.clear();
        self.target = None;
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        // Find nametag in hotbar
        let Some(slot) = hotbar_slot(env, &["minecraft:name_tag"]) else {
            self.base.error(env, "No Nametag in Hotbar");
            env.request_toggle("auto-nametag");
            return;
        };

        let held_name = stack_in(env, slot).map(|s| s.display_name());
        self.target = self
            .find_target(env, held_name.as_deref())
            .map(|entity| entity.id);
        let Some(id) = self.target else { return };
        let Some(entity) = env.entities().into_iter().find(|e| e.id == id) else {
            return;
        };

        // Swapping slots (`InvUtils.swap(findNametag.slot(), true)`).
        // PORT GAP: `InvUtils.swap`/`swapBack` - the port asks the host for the
        // vanilla hotbar swap and never swaps back, since the crosshair use is
        // immediate.
        let selected = env.player().map(|p| p.selected_slot).unwrap_or(0);
        env.click_slot(slot, selected as i32, ClickMode::Swap);
        self.off_hand = slot >= 36;

        // Interaction
        interact_entity(env, entity_center(&entity), self.rotate.get());
        self.cool(id, 20);

        // `TickEvent.Post`: counting the cooldowns down.
        for (_, ticks) in self.cooldowns.iter_mut() {
            if *ticks > 0 {
                *ticks -= 1;
            }
        }
        self.cooldowns.retain(|(_, ticks)| *ticks > 0);
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(AutoNametag::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{Action, InvSlot, ItemStackInfo};
    use crate::meteor_core::math::Vec3d;

    fn mob(id: i32, name: Option<&str>, x: f64) -> EntityInfo {
        EntityInfo {
            id,
            kind: "minecraft:cow".into(),
            name: name.map(|n| n.to_string()),
            pos: Vec3d::new(x, 64.0, 0.5),
            prev_pos: Vec3d::new(x, 64.0, 0.5),
            alive: true,
            is_mob: true,
            distance: (x - 0.5).abs(),
            ..Default::default()
        }
    }

    fn nametag(slot: usize) -> InvSlot {
        InvSlot {
            index: slot,
            stack: Some(ItemStackInfo {
                id: "minecraft:name_tag".into(),
                count: 4,
                ..Default::default()
            }),
        }
    }

    #[test]
    fn settings_match_the_java_module() {
        let m = AutoNametag::new();
        let names: Vec<&str> = m.base.settings.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["entities", "range", "priority", "renametag", "rotate"]
        );
        assert_eq!(m.base.title, "Auto Nametag");
        assert_eq!(
            m.base.description,
            "Automatically uses nametags on entities without a nametag. WILL nametag ALL entities in the specified distance."
        );
        assert_eq!(
            m.base.settings.get("priority").unwrap().display(),
            "lowest-distance"
        );
    }

    #[test]
    fn a_listed_entity_gets_the_nametag() {
        let mut m = AutoNametag::new();
        let mut env = TestEnv::in_world();
        m.entities.set(vec!["minecraft:cow".to_string()]);
        env.entities = vec![mob(11, None, 2.5)];
        env.inventory = vec![nametag(3)];

        m.on_tick(&mut env);
        assert_eq!(m.target(), Some(11));
        assert_eq!(env.actions, vec![Action::Use], "interact with the cow");
        assert_eq!(
            env.clicks,
            vec![(3, 0, ClickMode::Swap)],
            "the nametag slot is swapped into the hand"
        );
        assert!(env.rotation.yaw.abs() > 1.0);
    }

    #[test]
    fn without_a_nametag_the_module_complains_and_toggles_off() {
        let mut m = AutoNametag::new();
        let mut env = TestEnv::in_world();
        env.entities = vec![mob(11, None, 2.5)];

        m.on_tick(&mut env);
        assert!(env.actions.is_empty());
        assert_eq!(env.chats.len(), 1);
        assert!(
            env.chats[0].contains("No Nametag in Hotbar"),
            "got {}",
            env.chats[0]
        );
    }

    #[test]
    fn renametag_and_the_cooldown_gate_the_target() {
        let mut m = AutoNametag::new();
        let mut env = TestEnv::in_world();
        m.entities.set(vec!["minecraft:cow".to_string()]);
        env.inventory = vec![nametag(0)];

        // The cow already carries a custom name; `renametag` is on and the name
        // differs from the held nametag's, so it is re-nametagged.
        env.entities = vec![mob(11, Some("Bessie"), 2.5)];
        m.on_tick(&mut env);
        assert_eq!(m.target(), Some(11));
        assert_eq!(env.actions.len(), 1);

        // Now the held nametag's display name equals the cow's: skipped.
        env.inventory[0].stack.as_mut().unwrap().name = Some("Bessie".to_string());
        m.on_tick(&mut env);
        assert_eq!(m.target(), None, "the name matches the held nametag");

        // `renametag` off keeps every named entity alone.
        m.renametag.set(false);
        env.entities = vec![mob(12, Some("Bessie"), 3.5)];
        m.on_tick(&mut env);
        assert_eq!(m.target(), None);

        // A fresh, unnamed animal is taken, and its 20-tick cooldown keeps it out
        // of the next ticks.
        env.entities = vec![mob(13, None, 2.0)];
        m.on_tick(&mut env);
        assert_eq!(m.target(), Some(13));
        assert_eq!(env.actions.len(), 2);
        m.on_tick(&mut env);
        assert_eq!(env.actions.len(), 2, "still cooling down");
    }

    #[test]
    fn out_of_range_entities_are_ignored() {
        let mut m = AutoNametag::new();
        let mut env = TestEnv::in_world();
        m.entities.set(vec!["minecraft:cow".to_string()]);
        env.inventory = vec![nametag(0)];
        env.entities = vec![mob(14, None, 12.0)];
        m.on_tick(&mut env);
        assert_eq!(m.target(), None);
        assert!(env.actions.is_empty());
    }
}
