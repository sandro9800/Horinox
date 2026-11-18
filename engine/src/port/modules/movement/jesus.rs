//! `systems.modules.movement.Jesus` - walk on liquids and powder snow like Jesus.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Action, Env, GameMode, PacketInfo};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, EnumSet, IntSet};

use super::{
    add_group, describe_in, fall_distance, is_move_player_packet, jumping, set_packet_field,
    shift_held, visible_in,
};

pub struct Jesus {
    base: ModuleBase,
    // General
    /// `powder-snow`
    powder_snow: BoolSet,
    /// `ncp-bypass`
    ncp_bypass: BoolSet,
    /// `slow-down`
    slow_down: BoolSet,
    // Water
    /// `mode` (the Water page's)
    water_mode: EnumSet,
    /// `dip-if-burning`
    dip_if_burning: BoolSet,
    /// `dip-on-sneak`
    dip_on_sneak_water: BoolSet,
    /// `dip-on-fall`
    dip_on_fall_water: BoolSet,
    /// `dip-fall-height`
    dip_fall_height_water: IntSet,
    // Lava
    /// `mode` (the Lava page's)
    lava_mode: EnumSet,
    /// `dip-if-resistant`
    dip_if_fire_resistant: BoolSet,
    /// `dip-on-sneak`
    dip_on_sneak_lava: BoolSet,
    /// `dip-on-fall`
    dip_on_fall_lava: BoolSet,
    /// `dip-fall-height`
    dip_fall_height_lava: IntSet,

    ascending: i32,
    swimming_ticks: i32,
    /// `isInBubbleColumn`
    pub is_in_bubble_column: bool,
}

impl Jesus {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Movement,
            "jesus",
            "Walk on liquids and powder snow like Jesus.",
        );
        let mut s = base.settings_mut();

        // General
        let powder_snow = s.default_group().bool_setting("powder-snow", true);
        let ncp_bypass = s.default_group().bool_setting("ncp-bypass", false);
        let slow_down = s.default_group().bool_setting("slow-down", false);

        // Water
        let water = add_group(&mut s, "Water");
        let water_mode = s.groups[water].enum_setting("mode", &["Solid", "Ignore"], 0);
        let dip_if_burning = s.groups[water].bool_setting("dip-if-burning", true);
        let dip_on_sneak_water = s.groups[water].bool_setting("dip-on-sneak", true);
        let dip_on_fall_water = s.groups[water].bool_setting("dip-on-fall", true);
        let dip_fall_height_water = s.groups[water].int_setting("dip-fall-height", 4, 1, 255);

        // Lava
        let lava = add_group(&mut s, "Lava");
        let lava_mode = s.groups[lava].enum_setting("mode", &["Solid", "Ignore"], 0);
        let dip_if_fire_resistant = s.groups[lava].bool_setting("dip-if-resistant", true);
        let dip_on_sneak_lava = s.groups[lava].bool_setting("dip-on-sneak", true);
        let dip_on_fall_lava = s.groups[lava].bool_setting("dip-on-fall", true);
        let dip_fall_height_lava = s.groups[lava].int_setting("dip-fall-height", 4, 1, 255);

        describe_in(
            &mut s,
            "General",
            &[
                ("powder-snow", "Walk on powder snow."),
                ("ncp-bypass", "Whether to apply a bypass for NCP."),
                ("slow-down", "Further movement option to try bypassing NCP"),
            ],
        );
        describe_in(
            &mut s,
            "Water",
            &[
                ("mode", "How to treat the water."),
                (
                    "dip-if-burning",
                    "Lets you go into the water when you are burning.",
                ),
                (
                    "dip-on-sneak",
                    "Lets you go into the water when your sneak key is held.",
                ),
                (
                    "dip-on-fall",
                    "Lets you go into the water when you fall over a certain height.",
                ),
                (
                    "dip-fall-height",
                    "The fall height at which you will go into the water.",
                ),
            ],
        );
        describe_in(
            &mut s,
            "Lava",
            &[
                ("mode", "How to treat the lava."),
                (
                    "dip-if-resistant",
                    "Lets you go into the lava if you have Fire Resistance effect.",
                ),
                (
                    "dip-on-sneak",
                    "Lets you go into the lava when your sneak key is held.",
                ),
                (
                    "dip-on-fall",
                    "Lets you go into the lava when you fall over a certain height.",
                ),
                (
                    "dip-fall-height",
                    "The fall height at which you will go into the lava.",
                ),
            ],
        );

        {
            let ncp = ncp_bypass.clone();
            visible_in(&mut s, "General", "slow-down", move || ncp.get());
        }
        {
            let mode = water_mode.clone();
            visible_in(&mut s, "Water", "dip-if-burning", move || mode.is("Solid"));
            let mode = water_mode.clone();
            visible_in(&mut s, "Water", "dip-on-sneak", move || mode.is("Solid"));
            let mode = water_mode.clone();
            visible_in(&mut s, "Water", "dip-on-fall", move || mode.is("Solid"));
            let mode = water_mode.clone();
            let fall = dip_on_fall_water.clone();
            visible_in(&mut s, "Water", "dip-fall-height", move || {
                mode.is("Solid") && fall.get()
            });
        }
        {
            let mode = lava_mode.clone();
            visible_in(&mut s, "Lava", "dip-if-resistant", move || mode.is("Solid"));
            let mode = lava_mode.clone();
            visible_in(&mut s, "Lava", "dip-on-sneak", move || mode.is("Solid"));
            let mode = lava_mode.clone();
            visible_in(&mut s, "Lava", "dip-on-fall", move || mode.is("Solid"));
            let mode = lava_mode.clone();
            let fall = dip_on_fall_lava.clone();
            visible_in(&mut s, "Lava", "dip-fall-height", move || {
                mode.is("Solid") && fall.get()
            });
        }

        Self {
            base,
            powder_snow,
            ncp_bypass,
            slow_down,
            water_mode,
            dip_if_burning,
            dip_on_sneak_water,
            dip_on_fall_water,
            dip_fall_height_water,
            lava_mode,
            dip_if_fire_resistant,
            dip_on_sneak_lava,
            dip_on_fall_lava,
            dip_fall_height_lava,
            ascending: 10,
            swimming_ticks: 0,
            is_in_bubble_column: false,
        }
    }

    /// `canWalkOnPowderSnow()`.
    pub fn can_walk_on_powder_snow(&self) -> bool {
        self.powder_snow.get()
    }

    /// `isLavaDangerous()`.
    fn is_lava_dangerous(&self) -> bool {
        if !self.dip_if_fire_resistant.get() {
            return true;
        }
        // PORT GAP: fire resistance and `Attributes.BURNING_TIME` are not read by
        // `PlayerInfo`, so the player is treated as unprotected.
        true
    }

    /// `waterShouldBeSolid()`.
    pub fn water_should_be_solid(&self, env: &dyn Env) -> bool {
        let Some(player) = env.player() else {
            return false;
        };
        if player.game_mode == GameMode::Spectator {
            return false;
        }
        // PORT GAP: `abilities.flying`, riding a boat and `player.isOnFire()` have
        // no `Env` field, so those dips cannot be detected.
        if env.module_active("flight") {
            return false;
        }
        if self.dip_on_sneak_water.get() && shift_held(env) {
            return false;
        }
        // PORT GAP: `dip-if-burning` needs `player.isOnFire()`.
        let _ = self.dip_if_burning.get();
        if self.dip_on_fall_water.get()
            && fall_distance(env) > self.dip_fall_height_water.get() as f64
        {
            return false;
        }
        self.water_mode.is("Solid")
    }

    /// `lavaShouldBeSolid()`.
    pub fn lava_should_be_solid(&self, env: &dyn Env) -> bool {
        let Some(player) = env.player() else {
            return false;
        };
        if player.game_mode == GameMode::Spectator {
            return false;
        }
        if self.is_lava_dangerous() && self.lava_mode.is("Solid") {
            return true;
        }
        if self.dip_on_sneak_lava.get() && shift_held(env) {
            return false;
        }
        if self.dip_on_fall_lava.get()
            && fall_distance(env) > self.dip_fall_height_lava.get() as f64
        {
            return false;
        }
        self.lava_mode.is("Solid")
    }

    /// `isOverLiquid()` - `(water, lava)` for the block under the player.
    fn is_over_liquid(&self, env: &dyn Env) -> (bool, bool) {
        let Some(player) = env.player() else {
            return (false, false);
        };
        let below = player.pos.to_block_pos().offset(0, -1, 0);
        match env.block(below) {
            Some(state) => (state.is("minecraft:water"), state.is("minecraft:lava")),
            None => (false, false),
        }
    }
}

impl Default for Jesus {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Jesus {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        // PORT GAP: `PathManagers.getSettings().getWalkOnWater()/getWalkOnLava()`
        // are saved and forced to `mode == Solid` in Java; the port has no path
        // manager settings to carry that.
        self.ascending = 10;
        self.swimming_ticks = 0;
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        let Some(player) = env.player() else { return };

        let bubble_column = self.is_in_bubble_column;
        self.is_in_bubble_column = false;

        // PORT GAP: `player.isVisuallySwimming()` and `isPassenger()` are not in
        // `PlayerInfo`; the water/lava checks below cover the common case.
        if player.in_water && !self.water_should_be_solid(env) {
            return;
        }
        if player.in_lava && !self.lava_should_be_solid(env) {
            return;
        }

        // Move up in bubble columns.
        if bubble_column {
            if jumping(env) && player.velocity.y < 0.11 {
                env.action(Action::Jump);
            }
            return;
        }

        // Move up while inside a liquid.
        if player.in_water || player.in_lava {
            // PORT GAP: `setDeltaMovement(y = 0.11)` - the `Env` has no velocity
            // write, so a jump is the only "rise" it can ask for.
            env.action(Action::Jump);
            self.ascending = 0;
            return;
        }

        let below = player.pos.to_block_pos().offset(0, -1, 0);
        let below_state = env
            .block(below)
            .unwrap_or_else(|| crate::meteor_core::env::BlockInfo::new("minecraft:air"));
        let water_logged = below_state
            .prop("waterlogged")
            .map(|v| v == "true")
            .unwrap_or(false);
        let is_liquid =
            below_state.is("minecraft:water") || below_state.is("minecraft:lava") || water_logged;

        if self.ascending == 0 {
            env.action(Action::Jump);
        } else if self.ascending == 1 && is_liquid {
            // PORT GAP: `setDeltaMovement(y = 0)` - nothing the `Env` can write.
        }

        self.ascending += 1;
    }

    fn on_send_packet(&mut self, env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        if !is_move_player_packet(&packet.name) {
            return true;
        }
        let Some(player) = env.player() else {
            return true;
        };
        if player.in_water && !self.water_should_be_solid(env) {
            return true;
        }
        if player.in_lava && !self.lava_should_be_solid(env) {
            return true;
        }
        if !self.ncp_bypass.get() {
            return true;
        }

        let over = self.is_over_liquid(env);
        let should_work = (over.0 && self.water_should_be_solid(env))
            || (over.1 && self.lava_should_be_solid(env));

        if player.in_water || player.in_lava || fall_distance(env) > 3.0 || !should_work {
            return true;
        }

        set_packet_field(packet, "onGround", "false");
        if !player.on_ground {
            return true;
        }

        if let Some(y) = super::packet_f64(packet, "y") {
            let y = y - (0.02 + 0.0001 * self.swimming_ticks as f64);
            set_packet_field(packet, "y", &format!("{y:.5}"));
        }
        true
    }

    fn on_player_tick(&mut self, env: &mut dyn Env) {
        if !self.ncp_bypass.get() {
            return;
        }

        let over = self.is_over_liquid(env);
        let water = over.0 && self.water_should_be_solid(env);
        let lava = over.1 && self.lava_should_be_solid(env);

        if !water && !lava {
            self.swimming_ticks = 0;
            return;
        }

        self.swimming_ticks += 1;
        if self.swimming_ticks < 15 {
            // PORT GAP: `Strafe.transformStrafe(...)` sets the movement vector;
            // the `Env` has no movement write.
            return;
        }

        self.swimming_ticks = 0;
        // PORT GAP: `setXZ(0, 0)` and `setDeltaMovement(y = 0.08)` - no velocity
        // write either; `slow-down` only gates that write.
        let _ = self.slow_down.get();
    }

    fn info_string(&self) -> Option<String> {
        Some(self.water_mode.get().to_string())
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(Jesus::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::math::{BlockPos, Vec3d};

    #[test]
    fn walking_on_water_rises() {
        let mut m = Jesus::new();
        let mut env = TestEnv::in_world();
        env.player.as_mut().unwrap().in_water = true;
        env.player.as_mut().unwrap().on_ground = false;

        assert!(m.water_should_be_solid(&env));
        m.on_tick(&mut env);
        assert_eq!(env.actions, vec![Action::Jump]);
    }

    #[test]
    fn sneaking_dips_into_the_water() {
        let mut m = Jesus::new();
        let mut env = TestEnv::in_world();
        env.player.as_mut().unwrap().in_water = true;
        env.keys = vec![super::super::keys::LEFT_SHIFT];

        assert!(!m.water_should_be_solid(&env));
        m.on_tick(&mut env);
        assert!(env.actions.is_empty(), "dipping means no lift");
    }

    #[test]
    fn a_long_fall_lets_you_dive_and_spectators_never_walk_on_water() {
        let mut m = Jesus::new();
        let mut env = TestEnv::in_world();
        env.player.as_mut().unwrap().prev_pos = Vec3d::new(0.5, 65.0, 0.5);
        assert!(m.water_should_be_solid(&env));

        env.player.as_mut().unwrap().prev_pos = Vec3d::new(0.5, 70.0, 0.5);
        assert!(!m.water_should_be_solid(&env), "a long fall lets you dive");

        env.player.as_mut().unwrap().game_mode = GameMode::Spectator;
        assert!(!m.water_should_be_solid(&env));
    }

    #[test]
    fn ncp_bypass_lowers_the_packet() {
        let mut m = Jesus::new();
        let mut env = TestEnv::in_world();
        m.ncp_bypass.set(true);
        env.push_block(BlockPos::new(0, 63, 0), "minecraft:water");

        let mut packet = PacketInfo {
            name: "ServerboundMovePlayerPacket.Pos".to_string(),
            clientbound: false,
            fields: vec![
                ("y".to_string(), "64.00000".to_string()),
                ("onGround".to_string(), "true".to_string()),
            ],
        };

        assert!(m.on_send_packet(&mut env, &mut packet));
        assert_eq!(super::super::packet_bool(&packet, "onGround"), Some(false));
        let y = super::super::packet_f64(&packet, "y").unwrap();
        assert!(y < 64.0, "y was {y}");
    }

    #[test]
    fn the_three_pages_match_the_java_module() {
        let m = Jesus::new();
        let groups: Vec<&str> = m
            .base
            .settings
            .groups
            .iter()
            .map(|g| g.name.as_str())
            .collect();
        assert_eq!(groups, vec!["General", "Water", "Lava"]);
        assert!(m.can_walk_on_powder_snow());
    }
}
