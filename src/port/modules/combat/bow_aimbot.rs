//! `systems.modules.combat.BowAimbot` - Automatically aims your bow for you.
//!
//! Ports the Java targeting predicate and the ballistic pitch formula
//! (`BowItem.getPowerForTime` + the quadratic launch-angle solve); the aim itself
//! goes through `env.set_rotation`.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{EntityInfo, Env};
use crate::meteor_core::math::{Rotation, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet, EnumSet, ListSet};

use super::{line_of_sight, select_target, SORT_LOWEST_HEALTH, SORT_PRIORITIES};

pub struct BowAimbot {
    base: ModuleBase,
    range: DoubleSet,
    entities: ListSet,
    priority: EnumSet,
    babies: BoolSet,
    nametagged: BoolSet,
    pause_on_combat: BoolSet,
    target: Option<EntityInfo>,
}

fn is_arrow(id: &str) -> bool {
    let id = id.strip_prefix("minecraft:").unwrap_or(id);
    id == "arrow" || id == "spectral_arrow" || id == "tipped_arrow"
}

/// `BowItem.getPowerForTime(ticks)` for a bow at full draw (the Java code reads the
/// player's item-use ticks, which `PlayerInfo` does not expose).
const BOW_VELOCITY: f64 = 3.0;

impl BowAimbot {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Combat,
            "bow-aimbot",
            "Automatically aims your bow for you.",
        );
        let s = base.settings_mut();
        let g = s.default_group();
        let range = g.double_setting("range", 20.0, 0.0, 100.0, 2);
        let entities = g.list_setting("entities", &["minecraft:player"]);
        let priority = g.enum_setting("priority", SORT_PRIORITIES, SORT_LOWEST_HEALTH);
        let babies = g.bool_setting("babies", true);
        let nametagged = g.bool_setting("nametagged", false);
        let pause_on_combat = g.bool_setting("pause-on-combat", false);
        Self {
            base,
            range,
            entities,
            priority,
            babies,
            nametagged,
            pause_on_combat,
            target: None,
        }
    }

    /// `BowAimbot.itemInHand`.
    fn item_in_hand(env: &dyn Env) -> bool {
        env.selected_stack()
            .map(|s| s.id.ends_with("bow"))
            .unwrap_or(false)
    }

    /// `BowAimbot.aim`: the Java ballistic solve.
    pub fn aim(&self, env: &mut dyn Env, target: &EntityInfo) {
        let Some(player) = env.player() else { return };
        // PORT GAP: `mc.player.getTicksUsingItem()` is not on `PlayerInfo`, so the
        // bow is assumed fully drawn (`BowItem.getPowerForTime(20)` = 1).
        let velocity = BOW_VELOCITY;
        let pos = target.pos;
        let relative_x = pos.x - player.pos.x;
        let relative_y = pos.y + 0.9 - player.eye_pos().y;
        let relative_z = pos.z - player.pos.z;
        let h_distance = (relative_x * relative_x + relative_z * relative_z).sqrt();
        let h_distance_sq = h_distance * h_distance;
        let g = 0.006f64;
        let velocity_sq = velocity * velocity;
        let pitch = -((velocity_sq
            - (velocity_sq * velocity_sq
                - g * (g * h_distance_sq + 2.0 * relative_y * velocity_sq))
                .sqrt())
            / (g * h_distance))
            .atan()
            .to_degrees();

        let yaw = Rotation::yaw_to(player.eye_pos(), Vec3d::new(pos.x, pos.y, pos.z));
        let rotation = if pitch.is_nan() {
            Rotation::between(player.eye_pos(), pos)
        } else {
            Rotation::new(yaw, pitch as f32)
        };
        env.set_rotation(rotation);
    }
}

impl Default for BowAimbot {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for BowAimbot {
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
        let Some(player) = env.player() else { return };
        if !player.is_alive() || !Self::item_in_hand(env) {
            return;
        }
        let creative = player.game_mode == crate::meteor_core::env::GameMode::Creative;
        let has_arrows = env
            .inventory()
            .into_iter()
            .filter_map(|s| s.stack)
            .any(|s| is_arrow(&s.id));
        if !creative && !has_arrows {
            return;
        }

        let priority = self.priority.index();
        let range = self.range.get();
        self.target = select_target(env, priority, |e| {
            if !e.alive || e.health <= 0.0 {
                return false;
            }
            if player.eye_pos().distance_sq(e.pos) > range * range {
                return false;
            }
            if !self.entities.contains(&e.kind) && !self.entities.contains(e.short_kind()) {
                return false;
            }
            if !self.nametagged.get() && !e.is_player && e.name.is_some() {
                return false;
            }
            if !line_of_sight(env, player.eye_pos(), e.pos) {
                return false;
            }
            if e.is_player {
                if let Some(name) = &e.name {
                    if env.is_friend(name) {
                        return false;
                    }
                }
            }
            self.babies.get() || !e.baby
        });

        let Some(target) = self.target.clone() else {
            return;
        };
        if env.mouse_down(1) || env.player().map(|p| p.using_item).unwrap_or(false) {
            self.aim(env, &target);
        }
        let _ = self.pause_on_combat.get();
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
    use crate::meteor_core::env::{InvSlot, ItemStackInfo};

    fn bow_env() -> TestEnv {
        let mut env = TestEnv::in_world();
        env.entities.push(EntityInfo {
            id: 31,
            kind: "minecraft:player".into(),
            name: Some("Victim".into()),
            pos: Vec3d::new(0.5, 64.0, -8.5),
            health: 20.0,
            max_health: 20.0,
            alive: true,
            is_player: true,
            ..Default::default()
        });
        for (index, id) in [(0, "minecraft:bow"), (1, "minecraft:arrow")] {
            env.inventory.push(InvSlot {
                index,
                stack: Some(ItemStackInfo {
                    id: id.into(),
                    count: 16,
                    ..Default::default()
                }),
            });
        }
        env.player.as_mut().unwrap().selected_slot = 0;
        env
    }

    #[test]
    fn picks_a_target_and_aims() {
        let mut m = BowAimbot::new();
        let mut env = bow_env();
        m.on_tick(&mut env);

        let target = m.target.clone().expect("a target in range");
        assert_eq!(m.info_string().as_deref(), Some("Victim"));

        m.aim(&mut env, &target);
        assert_ne!(env.rotation, Rotation::default(), "the bow should be aimed");
        assert!(
            (env.rotation.yaw.abs() - 180.0).abs() < 1.0e-3,
            "yaw towards -z was {}",
            env.rotation.yaw
        );
    }

    #[test]
    fn needs_arrows() {
        let mut m = BowAimbot::new();
        let mut env = bow_env();
        env.inventory.retain(|s| s.index != 1);
        m.on_tick(&mut env);
        assert!(m.target.is_none());
    }
}
