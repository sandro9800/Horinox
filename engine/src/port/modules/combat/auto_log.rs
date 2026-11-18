//! `systems.modules.combat.AutoLog` -?Automatically disconnects you when certain requirements are met.
//!
//! Ports the Java health / prediction / totem-pop / entity-quota checks and the
//! `disconnect(...)` path through `env.disconnect`.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, PacketInfo};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, IntSet, ListSet};

use super::{possible_health_reductions, within};

pub struct AutoLog {
    base: ModuleBase,
    // General
    health: IntSet,
    smart: BoolSet,
    totem_pops: IntSet,
    only_trusted: BoolSet,
    instant_death: BoolSet,
    smart_toggle: BoolSet,
    toggle_off: BoolSet,
    toggle_auto_reconnect: BoolSet,
    // Entities
    entities: ListSet,
    use_total_count: BoolSet,
    combined_entity_threshold: IntSet,
    individual_entity_threshold: IntSet,
    range: IntSet,
    // Runtime
    pops: i32,
}

impl AutoLog {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Combat,
            "auto-log",
            "Automatically disconnects you when certain requirements are met.",
        );
        let s = base.settings_mut();
        let g = s.default_group();
        let health = g.int_setting("health", 6, 0, 19);
        let smart = g.bool_setting("predict-incoming-damage", true);
        let totem_pops = g.int_setting("totem-pops", 0, 0, 10);
        let only_trusted = g.bool_setting("only-trusted", false);
        let instant_death = g.bool_setting("32K", false);
        let smart_toggle = g.bool_setting("smart-toggle", false);
        let toggle_off = g.bool_setting("toggle-off", true);
        let toggle_auto_reconnect = g.bool_setting("toggle-auto-reconnect", true);

        s.create_group("Entities");
        let entities = super::last_group(s).list_setting("entities", &["minecraft:end_crystal"]);
        let use_total_count = super::last_group(s).bool_setting("use-total-count", true);
        let combined_entity_threshold =
            super::last_group(s).int_setting("combined-entity-threshold", 10, 1, 32);
        let individual_entity_threshold =
            super::last_group(s).int_setting("individual-entity-threshold", 2, 1, 16);
        let range = super::last_group(s).int_setting("range", 5, 1, 16);

        Self {
            base,
            health,
            smart,
            totem_pops,
            only_trusted,
            instant_death,
            smart_toggle,
            toggle_off,
            toggle_auto_reconnect,
            entities,
            use_total_count,
            combined_entity_threshold,
            individual_entity_threshold,
            range,
            pops: 0,
        }
    }

    /// `AutoLog.disconnect(reason)`.
    fn disconnect(&self, env: &mut dyn Env, reason: &str) {
        let msg = format!("[AutoLog] {reason}");
        self.base.info(env, &msg);
        if self.toggle_auto_reconnect.get() && env.module_active("auto-reconnect") {
            env.request_toggle("auto-reconnect");
        }
        env.disconnect(&msg);
    }

    fn after_disconnect(&self, env: &mut dyn Env) {
        if self.smart_toggle.get() || self.toggle_off.get() {
            env.request_toggle("auto-log");
        }
    }
}

impl Default for AutoLog {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AutoLog {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.pops = 0;
    }

    fn on_receive_packet(&mut self, env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        if !packet.name.contains("entity_event") {
            return true;
        }
        let event = packet
            .fields
            .iter()
            .find(|(k, _)| k == "event_id" || k == "event" || k == "id")
            .map(|(_, v)| v.as_str());
        if event != Some("35") {
            return true;
        }
        self.pops += 1;
        if self.totem_pops.get() > 0 && self.pops >= self.totem_pops.get() {
            let reason = format!("Popped {} totems.", self.pops);
            self.disconnect(env, &reason);
            self.after_disconnect(env);
        }
        true
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        let Some(player) = env.player() else { return };
        let player_health = player.health;
        if player_health <= 0.0 {
            env.request_toggle("auto-log");
            return;
        }

        if self.health.get() > 0 {
            if player_health <= self.health.get() as f32 {
                let reason = format!("Health was lower than {}.", self.health.get());
                self.disconnect(env, &reason);
                self.after_disconnect(env);
                return;
            }
            if self.smart.get() {
                let reductions = possible_health_reductions(env, true, true);
                if player_health + player.absorption - reductions < self.health.get() as f32 {
                    let reason =
                        format!("Health was going to be lower than {}.", self.health.get());
                    self.disconnect(env, &reason);
                    self.after_disconnect(env);
                    return;
                }
            }
        }

        let entity_list = self.entities.get();
        if !self.only_trusted.get() && !self.instant_death.get() && entity_list.is_empty() {
            return;
        }

        let eye = player.eye_pos();
        for e in env.entities() {
            if !e.is_player {
                continue;
            }
            let name = e.name.clone().unwrap_or_default();
            if self.only_trusted.get() && !env.is_friend(&name) {
                let reason = format!(
                    "Non-trusted player '{}' appeared in your render distance.",
                    name
                );
                self.disconnect(env, &reason);
                self.after_disconnect(env);
                return;
            }
            if self.instant_death.get() && within(eye, e.pos, 8.0) {
                // `DamageUtils.getAttackDamage(player, mc.player) > health + absorption`
                // PORT GAP: attribute modifiers are not on `EntityInfo`; the 32k
                // check uses the "holding a weapon" test with the vanilla sword
                // damage as the stand-in.
                if e.equipment[4].is_some() && 7.0 > player_health + player.absorption {
                    self.disconnect(env, "Anti-32k measures.");
                    self.after_disconnect(env);
                    return;
                }
            }
        }

        if entity_list.is_empty() {
            return;
        }
        let range = self.range.get() as f64;
        let mut total = 0;
        let mut per_type: Vec<(String, i32)> = Vec::new();
        for e in env.entities() {
            if !within(eye, e.pos, range) {
                continue;
            }
            if !entity_list
                .iter()
                .any(|id| *id == e.kind || e.kind.ends_with(id.as_str()))
            {
                continue;
            }
            total += 1;
            if !self.use_total_count.get() {
                match per_type.iter_mut().find(|(k, _)| *k == e.kind) {
                    Some((_, count)) => *count += 1,
                    None => per_type.push((e.kind.clone(), 1)),
                }
            }
        }

        if self.use_total_count.get() && total >= self.combined_entity_threshold.get() {
            self.disconnect(
                env,
                "Total number of selected entities within range exceeded the limit.",
            );
            self.after_disconnect(env);
        } else if !self.use_total_count.get() {
            for (kind, count) in per_type {
                if count >= self.individual_entity_threshold.get() {
                    let reason = format!("Number of {} within range exceeded the limit.", kind);
                    self.disconnect(env, &reason);
                    self.after_disconnect(env);
                    return;
                }
            }
        }
    }

    fn info_string(&self) -> Option<String> {
        Some(self.pops.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::EntityInfo;
    use crate::meteor_core::math::Vec3d;

    #[test]
    fn disconnects_below_the_health_threshold() {
        let mut m = AutoLog::new();
        let mut env = TestEnv::in_world();
        env.player.as_mut().unwrap().health = 4.0;
        m.on_tick(&mut env);
        assert!(
            env.chats
                .iter()
                .any(|c| c.contains("Health was lower than 6")),
            "expected a logout line, got {:?}",
            env.chats
        );
    }

    #[test]
    fn disconnects_on_totem_pops() {
        let mut m = AutoLog::new();
        let mut env = TestEnv::in_world();
        m.totem_pops.set(1);
        let mut pop = PacketInfo {
            name: "minecraft:entity_event".into(),
            clientbound: true,
            fields: vec![("event_id".into(), "35".into())],
        };
        m.on_receive_packet(&mut env, &mut pop);
        assert!(env.chats.iter().any(|c| c.contains("Popped 1 totems")));
        assert_eq!(m.info_string().as_deref(), Some("1"));
    }

    #[test]
    fn disconnects_on_too_many_crystals() {
        let mut m = AutoLog::new();
        let mut env = TestEnv::in_world();
        m.combined_entity_threshold.set(2);
        for i in 0..2 {
            env.entities.push(EntityInfo {
                id: i,
                kind: "minecraft:end_crystal".into(),
                pos: Vec3d::new(0.5 + i as f64, 64.0, 0.5),
                alive: true,
                ..Default::default()
            });
        }
        m.on_tick(&mut env);
        assert!(env.chats.iter().any(|c| c.contains("Total number")));
    }
}
