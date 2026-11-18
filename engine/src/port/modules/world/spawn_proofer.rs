//! `systems.modules.world.SpawnProofer` - "automatically spawnproofs unlit
//! areas".

use crate::meteor_core::category::Category;
use crate::meteor_core::env::Env;
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::BlockPos;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet, EnumSet, IntSet, ListSet};

use super::{can_place, describe, hotbar_block_slot, place_block, replaceable, stack_in};

/// `BlockUtils.MobSpawn`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MobSpawn {
    /// A floor to stand on: the spawn attempt will succeed right away.
    Always,
    /// In the air above a spawnable block.
    Potential,
    NotValid,
}

pub struct SpawnProofer {
    base: ModuleBase,
    place_delay: IntSet,
    place_range: DoubleSet,
    walls_range: DoubleSet,
    blocks_per_tick: IntSet,
    light_level: IntSet,
    blocks: ListSet,
    mode: EnumSet,
    rotate: BoolSet,
    spawns: Vec<BlockPos>,
    timer: i32,
}

impl SpawnProofer {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::World,
            "spawn-proofer",
            "Automatically spawnproofs unlit areas.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();

        let place_delay = g.int_setting("place-delay", 1, 0, 10);
        describe(g, "The tick delay between placing blocks.");

        let place_range = g.double_setting("place-range", 4.5, 0.0, 6.0, 2);
        describe(g, "How far away from the player you can place a block.");

        let walls_range = g.double_setting("walls-range", 4.5, 0.0, 6.0, 2);
        describe(
            g,
            "How far away from the player you can place a block behind walls.",
        );

        let blocks_per_tick = g.int_setting("blocks-per-tick", 1, 1, 10);
        describe(g, "How many blocks to place in one tick.");

        let light_level = g.int_setting("light-level", 0, 0, 15);
        describe(g, "Light levels to spawn proof. Old spawning light: 7.");

        let blocks = g.list_setting(
            "blocks",
            &[
                "minecraft:torch",
                "minecraft:stone_button",
                "minecraft:stone_slab",
            ],
        );
        describe(g, "Block to use for spawn proofing.");

        let mode = g.enum_setting("mode", &["always", "potential", "both"], 2);
        describe(g, "Which spawn types should be spawn proofed.");

        let rotate = g.bool_setting("rotate", true);
        describe(g, "Rotates towards the blocks being placed.");

        Self {
            base,
            place_delay,
            place_range,
            walls_range,
            blocks_per_tick,
            light_level,
            blocks,
            mode,
            rotate,
            spawns: Vec::new(),
            timer: 0,
        }
    }

    /// `BlockUtils.isValidMobSpawn(pos, state, lightLevel)`.
    ///
    /// PORT GAP: the Java helper splits `Always` (on a solid floor) from
    /// `Potential` (in the air) using the light engine's spawn rules. The port
    /// reads `env.block_light` and treats any position with a solid floor as
    /// `Always`.
    fn valid_mob_spawn(&self, env: &dyn Env, pos: BlockPos) -> MobSpawn {
        let Some(block) = env.block(pos) else {
            return MobSpawn::NotValid;
        };
        if !block.is_air() {
            return MobSpawn::NotValid;
        }
        let Some(below) = env.block(pos.offset(0, -1, 0)) else {
            return MobSpawn::NotValid;
        };
        if below.is_air() || replaceable(&below) {
            return MobSpawn::NotValid;
        }
        let Some(above) = env.block(pos.offset(0, 1, 0)) else {
            return MobSpawn::NotValid;
        };
        if !above.is_air() {
            return MobSpawn::NotValid;
        }
        let (block_light, sky_light) = env.block_light(pos);
        let light = block_light.max(sky_light);
        if light > self.light_level.get() as u8 {
            return MobSpawn::NotValid;
        }
        MobSpawn::Always
    }

    /// `SpawnProofer.isOutOfRange(pos)`: range, then the wall raycast.
    fn out_of_range(&self, env: &dyn Env, pos: BlockPos) -> bool {
        let Some(player) = env.player() else {
            return true;
        };
        if player.pos.distance_to(pos.center()) > self.place_range.get() {
            return true;
        }
        let visible = matches!(
            env.raycast(self.place_range.get() as f32),
            crate::meteor_core::env::Raycast::Block { pos: hit, .. } if hit == pos
        );
        if visible {
            return false;
        }
        player.pos.distance_to(pos.center()) > self.walls_range.get()
    }

    /// The positions found this tick.
    pub fn spawns(&self) -> &[BlockPos] {
        &self.spawns
    }
}

impl Default for SpawnProofer {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for SpawnProofer {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        // `TickEvent.Pre` returns while the delay is counting, `TickEvent.Post`
        // raises it; one pass per tick is the same schedule.
        if self.timer < self.place_delay.get() {
            self.timer += 1;
            return;
        }
        self.timer = 0;

        // Find slot
        let Some(slot) = hotbar_block_slot(env, &self.blocks) else {
            self.base
                .error(env, "Found none of the chosen blocks in hotbar.");
            env.request_toggle("spawn-proofer");
            return;
        };
        let Some(id) = stack_in(env, slot).map(|stack| stack.id) else {
            return;
        };

        // Find spawn locations
        let Some(player) = env.player() else { return };
        let radius = self.place_range.get().ceil() as i32;
        let feet = player.pos.to_block_pos();
        self.spawns.clear();
        for dy in -radius..=radius {
            for dz in -radius..=radius {
                for dx in -radius..=radius {
                    let pos = feet.offset(dx, dy, dz);
                    let spawn = self.valid_mob_spawn(env, pos);
                    let wanted = (spawn == MobSpawn::Always
                        && (self.mode.is("always") || self.mode.is("both")))
                        || (spawn == MobSpawn::Potential
                            && (self.mode.is("potential") || self.mode.is("both")));
                    if !wanted {
                        continue;
                    }
                    if !can_place(env, pos) {
                        continue;
                    }
                    if self.out_of_range(env, pos) {
                        continue;
                    }
                    self.spawns.push(pos);
                }
            }
        }

        if self.spawns.is_empty() {
            return;
        }

        // A light source lights up its neighbours, so one is enough per tick.
        let light_source = stack_in(env, slot).is_some_and(|stack| self.is_light_source(&stack.id));
        let per_tick = if light_source {
            1
        } else {
            self.blocks_per_tick.get()
        };

        let mut placed = 0;
        for pos in self.spawns.clone() {
            if placed >= per_tick {
                break;
            }
            if self.rotate.get() {
                super::rotate_to(env, pos);
            }
            if place_block(env, pos, &id) {
                placed += 1;
            }
        }
    }
}

impl SpawnProofer {
    /// `isLightSource(block)`: `getLightEmission() > 0` for the blocks this
    /// module places.
    fn is_light_source(&self, id: &str) -> bool {
        const LIGHT_SOURCES: [&str; 10] = [
            "minecraft:torch",
            "minecraft:soul_torch",
            "minecraft:lantern",
            "minecraft:soul_lantern",
            "minecraft:glowstone",
            "minecraft:sea_lantern",
            "minecraft:shroomlight",
            "minecraft:campfire",
            "minecraft:jack_o_lantern",
            "minecraft:end_rod",
        ];
        LIGHT_SOURCES
            .iter()
            .any(|known| id.eq_ignore_ascii_case(known))
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(SpawnProofer::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{InvSlot, ItemStackInfo};
    use crate::meteor_core::math::BlockPos;

    fn slot(index: usize, id: &str) -> InvSlot {
        InvSlot {
            index,
            stack: Some(ItemStackInfo {
                id: id.into(),
                count: 64,
                ..Default::default()
            }),
        }
    }

    fn floor(env: &mut TestEnv, size: i32) {
        for x in -size..=size {
            for z in -size..=size {
                env.push_block(BlockPos::new(x, 63, z), "minecraft:stone");
            }
        }
    }

    #[test]
    fn settings_match_the_java_module() {
        let m = SpawnProofer::new();
        let names: Vec<&str> = m.base.settings.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "place-delay",
                "place-range",
                "walls-range",
                "blocks-per-tick",
                "light-level",
                "blocks",
                "mode",
                "rotate",
            ]
        );
        assert_eq!(m.base.title, "Spawn Proofer");
        assert_eq!(m.base.description, "Automatically spawnproofs unlit areas.");
        assert_eq!(m.base.settings.get("mode").unwrap().display(), "both");
    }

    #[test]
    fn a_dark_floor_gets_a_torch() {
        let mut m = SpawnProofer::new();
        let mut env = TestEnv::in_world();
        floor(&mut env, 3);
        env.inventory = vec![slot(0, "minecraft:torch")];
        m.place_delay.set(0);
        // TestEnv's light is 15/15, so ask for a fully lit proofing threshold.
        m.light_level.set(15);

        m.on_tick(&mut env);
        assert!(!m.spawns().is_empty(), "dark floor positions were found");
        assert_eq!(env.placed.len(), 1, "one light source per tick");
        assert_eq!(env.placed[0].1, "minecraft:torch");
        assert_eq!(
            env.block(env.placed[0].0.offset(0, -1, 0)).unwrap().id,
            "minecraft:stone",
            "the torch goes on the floor"
        );
    }

    #[test]
    fn a_lit_area_and_a_missing_block_are_skipped() {
        let mut m = SpawnProofer::new();
        let mut env = TestEnv::in_world();
        floor(&mut env, 2);
        env.inventory = vec![slot(0, "minecraft:torch")];
        m.place_delay.set(0);
        // light-level 0 with TestEnv's 15/15 means no position qualifies.
        m.on_tick(&mut env);
        assert!(m.spawns().is_empty());
        assert!(env.placed.is_empty());

        // Wrong block in the hotbar: the module complains and toggles off.
        let mut empty = TestEnv::in_world();
        floor(&mut empty, 2);
        empty.inventory = vec![slot(0, "minecraft:dirt")];
        m.on_tick(&mut empty);
        assert!(empty.placed.is_empty());
        assert!(empty.chats[0].contains("Found none of the chosen blocks in hotbar."));
    }

    #[test]
    fn the_mode_filters_the_spawn_kind() {
        let mut m = SpawnProofer::new();
        let mut env = TestEnv::in_world();
        floor(&mut env, 2);
        env.inventory = vec![slot(0, "minecraft:stone_button")];
        m.place_delay.set(0);
        m.light_level.set(15);

        // Every found position in this world is `Always`, so `potential` finds none.
        m.mode.set_index(1);
        m.on_tick(&mut env);
        assert!(env.placed.is_empty(), "no potential spawns here");

        m.mode.set_index(0);
        m.on_tick(&mut env);
        assert_eq!(env.placed.len(), 1);
        assert_eq!(env.placed[0].1, "minecraft:stone_button");
    }

    #[test]
    fn the_place_delay_spreads_the_placements_out() {
        let mut m = SpawnProofer::new();
        let mut env = TestEnv::in_world();
        floor(&mut env, 2);
        env.inventory = vec![slot(0, "minecraft:torch")];
        m.light_level.set(15);
        // place-delay defaults to 1: the first tick only counts.
        m.on_tick(&mut env);
        assert!(env.placed.is_empty());
        m.on_tick(&mut env);
        assert_eq!(env.placed.len(), 1);
    }
}
