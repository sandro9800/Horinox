//! `systems.modules.world.EndermanLook` - "either looks at all Endermen or
//! prevents you from looking at Endermen".

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{EntityInfo, Env, GameMode, Raycast};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::Rotation;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, EnumSet};

use super::{describe, rotate_to_pos, visible};

/// `ItemTags.GAZE_DISGUISE_EQUIPMENT`: the headwear that stops an Enderman from
/// noticing your gaze.
const GAZE_DISGUISE: [&str; 8] = [
    "minecraft:carved_pumpkin",
    "minecraft:skeleton_skull",
    "minecraft:wither_skeleton_skull",
    "minecraft:creeper_head",
    "minecraft:dragon_head",
    "minecraft:piglin_head",
    "minecraft:player_head",
    "minecraft:zombie_head",
];

/// The armour slot the head lives in: `Env::inventory` is the hotbar, the main
/// inventory, then feet, legs, chest, head, off hand.
const HEAD_SLOT: usize = 39;

pub struct EndermanLook {
    base: ModuleBase,
    look_mode: EnumSet,
    stun: BoolSet,
}

impl EndermanLook {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::World,
            "enderman-look",
            "Either looks at all Endermen or prevents you from looking at Endermen.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();

        let look_mode = g.enum_setting("look-mode", &["at", "away"], 1);
        describe(g, "How this module behaves.");

        let stun = g.bool_setting("stun-hostiles", true);
        describe(
            g,
            "Automatically stares at hostile endermen to stun them in place.",
        );
        let mode = look_mode.clone();
        visible(g, move || mode.is("away"));

        Self {
            base,
            look_mode,
            stun,
        }
    }

    /// `mc.player.getItemBySlot(EquipmentSlot.HEAD).is(ItemTags.GAZE_DISGUISE_EQUIPMENT)`.
    fn wears_disguise(env: &dyn Env) -> bool {
        super::stack_in(env, HEAD_SLOT).is_some_and(|stack| super::is_item(&stack, &GAZE_DISGUISE))
    }

    /// `Enderman#isBeingStaredBy(Player)`.
    ///
    /// `vec3d.dot(vec3d2) > 1.0 - 0.025 / d`
    fn is_being_stared_at(
        &self,
        player: &crate::meteor_core::env::PlayerInfo,
        entity: &EntityInfo,
    ) -> bool {
        let view = player.rotation().direction().normalize();
        let mut to = crate::meteor_core::math::Vec3d::new(
            entity.pos.x - player.pos.x,
            entity.eye_pos().y - player.eye_pos().y,
            entity.pos.z - player.pos.z,
        );
        let d = to.length();
        if d < 1.0e-6 {
            return false;
        }
        to = to.normalize();
        let dot = view.x * to.x + view.y * to.y + view.z * to.z;
        dot > 1.0 - 0.025 / d
    }
}

impl Default for EndermanLook {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for EndermanLook {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        let Some(player) = env.player() else { return };
        // `if (... gaze disguise ... || mc.player.getAbilities().instabuild) return;`
        if Self::wears_disguise(env) || player.game_mode == GameMode::Creative {
            return;
        }

        let entities = env.entities();
        for entity in &entities {
            if entity.short_kind() != "enderman" || !entity.alive {
                continue;
            }
            if !has_line_of_sight(env, entity) {
                continue;
            }
            // PORT GAP: `Enderman.isCreepy()` - `EntityInfo` carries no anger flag,
            // so a hostile Enderman cannot be told apart from a passive one; the
            // `stun-hostiles` branch therefore only fires on the gaze check.
            let creepy = false;

            if self.look_mode.is("away") {
                if creepy && self.stun.get() {
                    rotate_to_pos(env, entity.eye_pos());
                } else if self.is_being_stared_at(&player, entity) {
                    // `Rotations.rotate(mc.player.getYRot(), 90, -75, null)`
                    env.set_rotation(Rotation::new(player.yaw, 90.0));
                }
            } else if !creepy {
                // `Rotations.getYaw/Pitch(entity, Target.Head)`
                rotate_to_pos(env, entity.eye_pos());
            }
        }
    }
}

/// `mc.player.hasLineOfSight(entity)`, to the reach the `Env` can express.
fn has_line_of_sight(env: &dyn Env, entity: &EntityInfo) -> bool {
    let Some(player) = env.player() else {
        return false;
    };
    let distance = player.pos.distance_to(entity.pos) as f32;
    match env.raycast(distance.max(0.1)) {
        Raycast::Block { distance: hit, .. } => hit >= distance,
        Raycast::Entity { id, .. } => id == entity.id,
        Raycast::Miss => true,
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(EndermanLook::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{GameMode, InvSlot, ItemStackInfo};
    use crate::meteor_core::math::Vec3d;

    fn enderman(id: i32, x: f64, z: f64) -> EntityInfo {
        EntityInfo {
            id,
            kind: "minecraft:enderman".into(),
            pos: Vec3d::new(x, 64.0, z),
            prev_pos: Vec3d::new(x, 64.0, z),
            alive: true,
            is_mob: true,
            health: 40.0,
            max_health: 40.0,
            ..Default::default()
        }
    }

    #[test]
    fn settings_match_the_java_module() {
        let m = EndermanLook::new();
        let names: Vec<&str> = m.base.settings.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["look-mode", "stun-hostiles"]);
        assert_eq!(m.base.title, "Enderman Look");
        assert_eq!(
            m.base.description,
            "Either looks at all Endermen or prevents you from looking at Endermen."
        );
        assert_eq!(m.base.settings.get("look-mode").unwrap().display(), "away");
        assert!(m.base.settings.get("stun-hostiles").unwrap().is_visible());
    }

    #[test]
    fn away_mode_drops_the_gaze_to_the_floor() {
        let mut m = EndermanLook::new();
        let mut env = TestEnv::in_world();
        // Straight ahead of the player (yaw 0 looks towards +z).
        env.entities = vec![enderman(20, 0.5, 3.5)];

        m.on_tick(&mut env);
        assert_eq!(env.rotation.pitch, 90.0, "look at your feet instead");
        assert_eq!(env.rotation.yaw, 0.0, "the yaw is kept");
    }

    #[test]
    fn at_mode_stares_at_the_enderman() {
        let mut m = EndermanLook::new();
        let mut env = TestEnv::in_world();
        m.look_mode.set_index(0); // at
        assert!(!m.base.settings.get("stun-hostiles").unwrap().is_visible());
        env.entities = vec![enderman(20, 0.5, 3.5)];

        m.on_tick(&mut env);
        assert!(env.rotation.yaw.abs() < 1.0, "straight ahead");
        assert!(
            env.rotation.pitch.abs() < 1.0,
            "the eye line is level, got {}",
            env.rotation.pitch
        );
    }

    #[test]
    fn a_carved_pumpkin_or_creative_mode_stops_the_module() {
        let mut m = EndermanLook::new();
        let mut disguised = TestEnv::in_world();
        disguised.entities = vec![enderman(20, 0.5, 3.5)];
        disguised.inventory = vec![InvSlot {
            index: HEAD_SLOT,
            stack: Some(ItemStackInfo {
                id: "minecraft:carved_pumpkin".into(),
                count: 1,
                ..Default::default()
            }),
        }];
        m.on_tick(&mut disguised);
        assert_eq!(disguised.rotation.pitch, 0.0, "the pumpkin hides the gaze");

        let mut creative = TestEnv::in_world();
        creative.entities = vec![enderman(20, 0.5, 3.5)];
        creative.player.as_mut().unwrap().game_mode = GameMode::Creative;
        m.on_tick(&mut creative);
        assert_eq!(creative.rotation.pitch, 0.0, "instabuild ignores Endermen");
    }

    #[test]
    fn looking_away_from_the_enderman_does_nothing() {
        let mut m = EndermanLook::new();
        let mut env = TestEnv::in_world();
        // Behind the player, who looks towards +z.
        env.entities = vec![enderman(20, 0.5, -3.5)];
        m.on_tick(&mut env);
        assert_eq!(env.rotation, Rotation::default(), "no gaze, no change");

        // A wall in the way breaks line of sight.
        let mut walled = TestEnv::in_world();
        walled.entities = vec![enderman(20, 0.5, 3.5)];
        walled.push_block(
            crate::meteor_core::math::BlockPos::new(0, 65, 2),
            "minecraft:stone",
        );
        m.on_tick(&mut walled);
        assert_eq!(walled.rotation, Rotation::default());
    }
}
