//! `systems.modules.combat.KillAura` -?Attacks specified entities around you.
//!
//! The Java class aims through `Rotations.rotate` and hits with
//! `mc.gameMode.attack`; the port keeps the same target selection
//! (`TargetUtils.getList` + `SortPriority`), the same pause gates and the same
//! `hitTimer` / attack-cooldown gate, routed through `Env`.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Action, ClickMode, EntityInfo, Env, PlayerInfo};
use crate::meteor_core::math::{Rotation, Timer, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet, EnumSet, IntSet, ItemListSet, ListSet};

use super::{body_pos, is_tag, line_of_sight, sort_targets, SORT_CLOSEST_ANGLE, SORT_PRIORITIES};

pub struct KillAura {
    base: ModuleBase,
    // General
    attack_when_holding: EnumSet,
    weapons: ItemListSet,
    rotation: EnumSet,
    auto_switch: BoolSet,
    swap_back: BoolSet,
    shield_mode: EnumSet,
    only_on_click: BoolSet,
    only_on_look: BoolSet,
    pause_baritone: BoolSet,
    // Targeting
    entities: ListSet,
    priority: EnumSet,
    max_targets: IntSet,
    range: DoubleSet,
    walls_range: DoubleSet,
    passive_mob_age_filter: EnumSet,
    hostile_mob_age_filter: EnumSet,
    ignore_named: BoolSet,
    ignore_passive: BoolSet,
    ignore_tamed: BoolSet,
    // Timing
    pause_on_lag: BoolSet,
    pause_on_use: BoolSet,
    pause_on_ca: BoolSet,
    tps_sync: BoolSet,
    custom_delay: BoolSet,
    hit_delay: IntSet,
    switch_delay: IntSet,
    // Runtime
    targets: Vec<EntityInfo>,
    switch_timer: i32,
    hit_timer: i32,
    attacking: bool,
    swapped: bool,
    previous_slot: usize,
    cooldown: Timer,
    wanted: Option<Rotation>,
}

/// `EntityAgeTest`'s constants, in declaration order.
const AGE_TESTS: &[&str] = &["baby", "adult", "both"];

impl KillAura {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Combat,
            "kill-aura",
            "Attacks specified entities around you.",
        );
        let s = base.settings_mut();
        let g = s.default_group();
        let attack_when_holding = g.enum_setting("attack-when-holding", &["weapons", "all"], 0);
        let weapons = g.item_list_setting(
            "selected-weapon-types",
            &[
                "minecraft:diamond_sword",
                "minecraft:diamond_axe",
                "minecraft:trident",
            ],
        );
        let rotation = g.enum_setting("rotate", &["always", "on-hit", "none"], 0);
        let auto_switch = g.bool_setting("auto-switch", false);
        let swap_back = g.bool_setting("swap-back", false);
        let shield_mode = g.enum_setting("shield-mode", &["ignore", "break", "none"], 2);
        let only_on_click = g.bool_setting("only-on-click", false);
        let only_on_look = g.bool_setting("only-on-look", false);
        let pause_baritone = g.bool_setting("pause-baritone", true);

        let t = s.create_group("Targeting");
        let entities = t.list_setting("entities", &["minecraft:player"]);
        let priority = t.enum_setting("priority", SORT_PRIORITIES, SORT_CLOSEST_ANGLE);
        let max_targets = t.int_setting("max-targets", 1, 1, 5);
        let range = t.double_setting("range", 4.5, 0.0, 6.0, 2);
        let walls_range = t.double_setting("walls-range", 3.5, 0.0, 6.0, 2);
        let passive_mob_age_filter = t.enum_setting("passive-mob-age-filter", AGE_TESTS, 1);
        let hostile_mob_age_filter = t.enum_setting("hostile-mob-age-filter", AGE_TESTS, 2);
        let ignore_named = t.bool_setting("ignore-named", false);
        let ignore_passive = t.bool_setting("ignore-passive", true);
        let ignore_tamed = t.bool_setting("ignore-tamed", false);

        let tim = s.create_group("Timing");
        let pause_on_lag = tim.bool_setting("pause-on-lag", true);
        let pause_on_use = tim.bool_setting("pause-on-use", false);
        let pause_on_ca = tim.bool_setting("pause-on-CA", true);
        let tps_sync = tim.bool_setting("TPS-sync", true);
        let custom_delay = tim.bool_setting("custom-delay", false);
        let hit_delay = tim.int_setting("hit-delay", 11, 0, 60);
        let switch_delay = tim.int_setting("switch-delay", 0, 0, 10);

        Self {
            base,
            attack_when_holding,
            weapons,
            rotation,
            auto_switch,
            swap_back,
            shield_mode,
            only_on_click,
            only_on_look,
            pause_baritone,
            entities,
            priority,
            max_targets,
            range,
            walls_range,
            passive_mob_age_filter,
            hostile_mob_age_filter,
            ignore_named,
            ignore_passive,
            ignore_tamed,
            pause_on_lag,
            pause_on_use,
            pause_on_ca,
            tps_sync,
            custom_delay,
            hit_delay,
            switch_delay,
            targets: Vec::new(),
            switch_timer: 0,
            hit_timer: 0,
            attacking: false,
            swapped: false,
            previous_slot: 0,
            cooldown: Timer::new(),
            wanted: None,
        }
    }

    /// `KillAura.entityCheck`.
    fn entity_check(&self, env: &dyn Env, e: &EntityInfo, player: &PlayerInfo) -> bool {
        if !e.alive || e.health <= 0.0 {
            return false;
        }
        let (min, max) = e.bounding_box();
        let clamped = Vec3d::new(
            player.pos.x.clamp(min.x, max.x),
            player.pos.y.clamp(min.y, max.y),
            player.pos.z.clamp(min.z, max.z),
        );
        let eye = player.eye_pos();
        if eye.distance_sq(clamped) > self.range.get() * self.range.get() {
            return false;
        }
        if !self.entities.contains(&e.kind) && !self.entities.contains(e.short_kind()) {
            return false;
        }
        // `hasCustomName()`: EntityInfo.name is a custom name for mobs and the
        // profile name for players, so players are exempt like in Java.
        if self.ignore_named.get() && !e.is_player && e.name.is_some() {
            return false;
        }
        if !line_of_sight(env, eye, e.pos) && !line_of_sight(env, eye, e.eye_pos()) {
            if eye.distance_sq(e.pos) > self.walls_range.get() * self.walls_range.get() {
                return false;
            }
        }
        if self.ignore_tamed.get() {
            // PORT GAP: taming/ownership (`OwnableEntity.getOwner`) is not part of
            // `EntityInfo`, so a tamed mob cannot be told from a wild one.
        }
        if self.ignore_passive.get() {
            let kind = e.short_kind();
            // `isCreepy()` / `Mob.isAggressive()` have no `EntityInfo` equivalent,
            // so a passive-targeting mob is treated as not aggressive.
            if matches!(kind, "enderman" | "piglin" | "zombified_piglin" | "wolf") {
                return false;
            }
        }
        if e.is_player {
            // PORT GAP: `EntityInfo` carries no game mode, so creative players are
            // not filtered (Java: `player.isCreative()`).
            if let Some(name) = &e.name {
                if env.is_friend(name) {
                    return false;
                }
            }
            if self.shield_mode.is("ignore") && e.blocking {
                return false;
            }
        }
        let age_ok = |setting: &EnumSet| -> bool {
            match setting.index() {
                0 => e.baby,
                1 => !e.baby,
                _ => true,
            }
        };
        if matches!(e.short_kind(), "zombie" | "piglin" | "hoglin" | "zoglin") {
            return age_ok(&self.hostile_mob_age_filter);
        }
        if e.is_mob {
            return age_ok(&self.passive_mob_age_filter);
        }
        true
    }

    /// `KillAura.acceptableWeapon`.
    fn acceptable_weapon(&self, stack: &crate::meteor_core::env::ItemStackInfo) -> bool {
        if self.should_shield_break() {
            return is_tag(stack, "axes");
        }
        if self.attack_when_holding.is("all") {
            return true;
        }
        if self.weapons.contains("minecraft:diamond_sword") && is_tag(stack, "swords") {
            return true;
        }
        if self.weapons.contains("minecraft:diamond_axe") && is_tag(stack, "axes") {
            return true;
        }
        if self.weapons.contains("minecraft:diamond_pickaxe") && is_tag(stack, "pickaxes") {
            return true;
        }
        if self.weapons.contains("minecraft:diamond_shovel") && is_tag(stack, "shovels") {
            return true;
        }
        if self.weapons.contains("minecraft:diamond_hoe") && is_tag(stack, "hoes") {
            return true;
        }
        if self.weapons.contains("minecraft:mace") && is_tag(stack, "mace") {
            return true;
        }
        if self.weapons.contains("minecraft:diamond_spear") && is_tag(stack, "spears") {
            return true;
        }
        self.weapons.contains("minecraft:trident") && is_tag(stack, "trident")
    }

    /// `KillAura.shouldShieldBreak`.
    fn should_shield_break(&self) -> bool {
        self.shield_mode.is("break") && self.targets.iter().any(|t| t.is_player && t.blocking)
    }

    /// `KillAura.stopAttacking`.
    fn stop_attacking(&mut self, env: &mut dyn Env) {
        self.wanted = None;
        if !self.attacking {
            return;
        }
        self.attacking = false;
        // PORT GAP: Baritone pausing (`PathManagers.get().pause()`) has no `Env`
        // equivalent; the `pause-baritone` setting is kept for fidelity only.
        if self.swap_back.get() && self.swapped {
            let previous = self.previous_slot;
            if let Some(player) = env.player() {
                let selected = player.selected_slot;
                env.click_slot(previous, selected as i32, ClickMode::Swap);
            }
            self.swapped = false;
        }
    }

    /// `KillAura.delayCheck`.
    fn delay_check(&mut self, env: &dyn Env) -> bool {
        if self.switch_timer > 0 {
            self.switch_timer -= 1;
            return false;
        }

        let mut delay = if self.custom_delay.get() {
            self.hit_delay.get() as f32
        } else {
            0.5
        };
        let tps = env.recent_tps();
        if self.tps_sync.get() && tps > 0.0 {
            delay /= tps / 20.0;
        }

        if self.custom_delay.get() {
            // `hitTimer` counts ticks, exactly like the Java field.
            if self.hit_timer < delay as i32 {
                self.hit_timer += 1;
                false
            } else {
                true
            }
        } else {
            // `mc.player.getAttackStrengthScale(delay) >= 1`: the port measures
            // the same 0.5s cooldown with a `Timer`.
            self.cooldown.passed(delay as f64)
        }
    }

    /// The aim point Java's `Rotations.getYaw/getPitch(entity, Target.Body)` uses.
    fn aim_at(&self, env: &dyn Env, target: &EntityInfo) -> Option<Rotation> {
        let player = env.player()?;
        Some(Rotation::between(player.eye_pos(), body_pos(target)))
    }
}

impl Default for KillAura {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for KillAura {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.previous_slot = 0;
        self.swapped = false;
        self.hit_timer = 0;
        self.switch_timer = 0;
        // The vanilla attack cooldown is full on activate, so the first hit may
        // land on the very next tick.
        self.cooldown = Timer::new();
        self.cooldown.shift(10.0);
    }

    fn on_deactivate(&mut self, env: &mut dyn Env) {
        self.targets.clear();
        self.stop_attacking(env);
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        let Some(player) = env.player() else { return };
        if !player.is_alive() || player.game_mode == crate::meteor_core::env::GameMode::Spectator {
            self.stop_attacking(env);
            return;
        }
        if self.pause_on_use.get() && player.using_item {
            self.stop_attacking(env);
            return;
        }
        if self.only_on_click.get() && !env.mouse_down(0) {
            self.stop_attacking(env);
            return;
        }
        if self.pause_on_lag.get() && 20.0 - env.recent_tps() >= 1.0 {
            self.stop_attacking(env);
            return;
        }
        if self.pause_on_ca.get() && env.module_active("crystal-aura") {
            // PORT GAP: Java also requires CrystalAura.kaTimer > 0, a field read
            // across modules that `Env` does not expose.
            self.stop_attacking(env);
            return;
        }

        if self.only_on_look.get() {
            let visible = env.crosshair_entity();
            match visible {
                Some(e) if self.entity_check(env, &e, &player) => {
                    self.targets.clear();
                    self.targets.push(e);
                }
                _ => {
                    self.stop_attacking(env);
                    return;
                }
            }
        } else {
            let priority = self.priority.index();
            let max = self.max_targets.get() as usize;
            let mut list: Vec<EntityInfo> = env
                .entities()
                .into_iter()
                .filter(|e| self.entity_check(env, e, &player))
                .collect();
            sort_targets(
                &mut list,
                priority,
                player.eye_pos(),
                player.yaw,
                player.pitch,
            );
            list.truncate(max.max(1));
            self.targets = list;
        }

        if self.targets.is_empty() {
            self.stop_attacking(env);
            return;
        }

        let primary = self.targets[0].clone();

        if self.auto_switch.get() {
            let mut weapon = env.player().map(|p| p.selected_slot).unwrap_or(0);
            if self.attack_when_holding.is("weapons") {
                let mut found = super::find_hotbar_item(
                    env,
                    &[
                        "minecraft:diamond_sword",
                        "minecraft:diamond_axe",
                        "minecraft:trident",
                        "minecraft:mace",
                        "minecraft:diamond_spear",
                    ],
                );
                if found.is_none() {
                    found = super::find_hotbar_item(
                        env,
                        &[
                            "minecraft:netherite_sword",
                            "minecraft:iron_sword",
                            "minecraft:stone_sword",
                            "minecraft:wooden_sword",
                            "minecraft:netherite_axe",
                            "minecraft:iron_axe",
                            "minecraft:diamond_pickaxe",
                        ],
                    );
                }
                if let Some(slot) = found {
                    weapon = slot;
                }
            }
            if self.should_shield_break() {
                if let Some(slot) = super::find_hotbar_item(
                    env,
                    &[
                        "minecraft:diamond_axe",
                        "minecraft:netherite_axe",
                        "minecraft:iron_axe",
                        "minecraft:stone_axe",
                    ],
                ) {
                    weapon = slot;
                }
            }
            if !self.swapped {
                self.previous_slot = env.player().map(|p| p.selected_slot).unwrap_or(0);
                self.swapped = true;
            }
            // PORT GAP: `InvUtils.swap` sends ServerboundSetCarriedItemPacket; `Env`
            // has no "select hotbar slot", so the hotbar swap click stands in.
            let selected = env.player().map(|p| p.selected_slot).unwrap_or(0);
            if weapon != selected {
                env.click_slot(weapon, selected as i32, ClickMode::Swap);
            }
        }

        let acceptable = env
            .selected_stack()
            .map(|s| self.acceptable_weapon(&s))
            .unwrap_or(false);
        if !acceptable {
            self.stop_attacking(env);
            return;
        }

        self.attacking = true;
        if self.rotation.is("always") {
            if let Some(r) = self.aim_at(env, &primary) {
                env.set_rotation(r);
                self.wanted = Some(r);
            }
        }

        if self.delay_check(env) {
            let on_hit = self.rotation.is("on-hit");
            for target in self.targets.clone() {
                if on_hit {
                    if let Some(r) = self.aim_at(env, &target) {
                        env.set_rotation(r);
                        self.wanted = Some(r);
                    }
                }
                env.attack_entity(target.id);
                env.action(Action::Attack);
                self.hit_timer = 0;
                self.cooldown.reset();
            }
        }
    }

    fn on_send_packet(
        &mut self,
        _env: &mut dyn Env,
        packet: &mut crate::meteor_core::env::PacketInfo,
    ) -> bool {
        // `ServerboundSetCarriedItemPacket` resets the switch timer.
        if packet.name.contains("carried") || packet.name.contains("held_item") {
            self.switch_timer = self.switch_delay.get();
        }
        true
    }

    fn info_string(&self) -> Option<String> {
        if let Some(target) = self.targets.first() {
            return Some(
                target
                    .name
                    .clone()
                    .unwrap_or_else(|| target.short_kind().to_string()),
            );
        }
        None
    }

    fn wanted_rotation(&self) -> Option<Rotation> {
        self.wanted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{EntityInfo, InvSlot, ItemStackInfo};

    fn target_at(z: f64) -> EntityInfo {
        EntityInfo {
            id: 42,
            kind: "minecraft:player".into(),
            name: Some("Target".into()),
            pos: Vec3d::new(0.5, 64.0, z),
            health: 20.0,
            max_health: 20.0,
            alive: true,
            is_player: true,
            ..Default::default()
        }
    }

    fn armed_env() -> TestEnv {
        let mut env = TestEnv::in_world();
        env.inventory.push(InvSlot {
            index: 0,
            stack: Some(ItemStackInfo {
                id: "minecraft:diamond_sword".into(),
                count: 1,
                ..Default::default()
            }),
        });
        env.player.as_mut().unwrap().selected_slot = 0;
        env
    }

    #[test]
    fn attacks_the_target_in_range() {
        let mut m = KillAura::new();
        let mut env = armed_env();
        env.entities.push(target_at(-2.5));
        m.on_activate(&mut env);
        m.on_tick(&mut env);

        assert!(
            env.actions.contains(&Action::Attack),
            "the target in range should have been attacked"
        );
        assert_ne!(env.rotation, Rotation::default(), "aim should have moved");
        assert_eq!(m.info_string().as_deref(), Some("Target"));
    }

    #[test]
    fn ignores_targets_out_of_range() {
        let mut m = KillAura::new();
        let mut env = armed_env();
        env.entities.push(target_at(-30.0));
        m.on_activate(&mut env);
        m.on_tick(&mut env);

        assert!(env.actions.is_empty(), "out of range: no attack");
        assert_eq!(env.rotation, Rotation::default());
        assert_eq!(m.info_string(), None);
    }

    #[test]
    fn custom_delay_gates_the_hit() {
        let mut m = KillAura::new();
        let mut env = armed_env();
        env.entities.push(target_at(-2.5));
        m.custom_delay.set(true);
        m.hit_delay.set(2);
        m.on_activate(&mut env);

        // hitTimer starts at 0, so the first tick only arms the counter.
        m.on_tick(&mut env);
        assert!(env.actions.is_empty());
        m.on_tick(&mut env);
        assert!(env.actions.is_empty());
        m.on_tick(&mut env);
        assert!(env.actions.contains(&Action::Attack));
    }

    #[test]
    fn settings_match_the_java_module() {
        let m = KillAura::new();
        assert_eq!(m.base.name, "kill-aura");
        assert_eq!(m.base.category, Category::Combat);
        for name in [
            "attack-when-holding",
            "selected-weapon-types",
            "rotate",
            "auto-switch",
            "shield-mode",
            "pause-baritone",
            "entities",
            "priority",
            "max-targets",
            "walls-range",
            "TPS-sync",
            "hit-delay",
        ] {
            assert!(m.base.settings.get(name).is_some(), "missing {name}");
        }
        assert_eq!(m.base.settings.groups.len(), 3);
    }
}
