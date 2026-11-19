//! `systems.modules.world.Flamethrower` - "ignites every alive piece of food".

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{ClickMode, Env, Hand};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::Dir;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet, IntSet, ListSet};

use super::{
    break_block, describe, entity_within, hotbar_slot, interact_block, listed, rotate_to_pos,
};

pub struct Flamethrower {
    base: ModuleBase,
    distance: DoubleSet,
    anti_break: BoolSet,
    put_out_fire: BoolSet,
    target_babies: BoolSet,
    tick_interval: IntSet,
    rotate: BoolSet,
    entities: ListSet,
    entity: Option<i32>,
    ticks: i32,
    hand: Hand,
}

impl Flamethrower {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::World,
            "flamethrower",
            "Ignites every alive piece of food.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();

        let distance = g.double_setting("distance", 5.0, 0.0, 10.0, 2);
        describe(
            g,
            "The maximum distance the animal has to be to be roasted.",
        );

        let anti_break = g.bool_setting("anti-break", false);
        describe(g, "Prevents flint and steel from being broken.");

        let put_out_fire = g.bool_setting("put-out-fire", true);
        describe(
            g,
            "Tries to put out the fire when animal is low health, so the items don't burn.",
        );

        let target_babies = g.bool_setting("target-babies", false);
        describe(g, "If checked babies will also be killed.");

        let tick_interval = g.int_setting("tick-interval", 5, 0, 100);
        describe(g, "Ticks between ignition attempts.");

        let rotate = g.bool_setting("rotate", true);
        describe(g, "Automatically faces towards the animal roasted.");

        let entities = g.list_setting(
            "entities",
            &[
                "minecraft:pig",
                "minecraft:cow",
                "minecraft:sheep",
                "minecraft:chicken",
                "minecraft:rabbit",
            ],
        );
        describe(g, "Entities to cook.");

        Self {
            base,
            distance,
            anti_break,
            put_out_fire,
            target_babies,
            tick_interval,
            rotate,
            entities,
            entity: None,
            ticks: 0,
            hand: Hand::Main,
        }
    }

    /// The animal being roasted this tick.
    pub fn target(&self) -> Option<i32> {
        self.entity
    }

    /// The hand holding the flint and steel.
    pub fn hand(&self) -> Hand {
        self.hand
    }

    /// `InvUtils.findInHotbar(itemStack -> FLINT_AND_STEEL || FIRE_CHARGE && ...)`.
    fn find_igniter(&self, env: &dyn Env) -> Option<usize> {
        let slot = hotbar_slot(env, &["minecraft:flint_and_steel", "minecraft:fire_charge"])?;
        if !self.anti_break.get() {
            return Some(slot);
        }
        // PORT GAP: `ItemStack.getMaxDamage()` - flint and steel's 64 uses are
        // hard-coded so `anti-break` can leave the last one alone.
        let worn = super::stack_in(env, slot)
            .and_then(|stack| stack.damage)
            .is_some_and(|damage| damage >= 63);
        if worn {
            None
        } else {
            Some(slot)
        }
    }

    fn interact(&mut self, env: &mut dyn Env, entity: &crate::meteor_core::env::EntityInfo) {
        let pos = entity.pos.to_block_pos();
        let block = env.block(pos).unwrap_or_default();
        let bottom = env.block(pos.offset(0, -1, 0)).unwrap_or_default();
        if block.is("minecraft:water")
            || bottom.is("minecraft:water")
            || bottom.is("minecraft:dirt_path")
        {
            return;
        }
        // `mc.gameMode.startDestroyBlock(entity.blockPosition(), Direction.DOWN)`
        if block.is("minecraft:grass_block") {
            break_block(env, pos);
        }

        if self.put_out_fire.get() && entity.health < 2.0 {
            break_block(env, pos);
            for dir in [Dir::North, Dir::South, Dir::West, Dir::East] {
                break_block(env, pos.offset_dir(dir));
            }
        } else if self.ticks >= self.tick_interval.get() && !entity.on_fire {
            // `BlockUtils.interact(new BlockHitResult(entity.position() - (0, 1, 0),
            // Direction.UP, entity.blockPosition().below(), false), hand, false)`
            interact_block(env, pos.offset(0, -1, 0), Dir::Up);
            self.ticks = 0;
        }
    }
}

impl Default for Flamethrower {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Flamethrower {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_deactivate(&mut self, _env: &mut dyn Env) {
        self.entity = None;
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        self.entity = None;
        self.ticks += 1;

        let own_name = env.player().map(|p| p.name).unwrap_or_default();
        let entities = env.entities();
        for entity in &entities {
            if !listed(&self.entities, &entity.kind) {
                continue;
            }
            if !entity_within(env, entity, self.distance.get()) {
                continue;
            }
            // `if (entity == mc.player) continue;`
            if entity.is_player && entity.name.as_deref() == Some(own_name.as_str()) {
                continue;
            }
            // PORT GAP: `entity.isInPowderSnow()`, `isInWaterOrRain()` and
            // `fireImmune()` - `EntityInfo` carries none of them, so only death
            // and the baby filter are applied here.
            if !entity.alive {
                continue;
            }
            if !self.target_babies.get() && entity.baby {
                continue;
            }

            let Some(slot) = self.find_igniter(env) else {
                return;
            };
            let selected = env.player().map(|p| p.selected_slot).unwrap_or(0);
            env.click_slot(slot, selected as i32, ClickMode::Swap);

            self.hand = Hand::Main;
            self.entity = Some(entity.id);

            if self.rotate.get() {
                rotate_to_pos(env, entity.pos.center());
            }
            self.interact(env, entity);
            return;
        }
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(Flamethrower::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{EntityInfo, InvSlot, ItemStackInfo};
    use crate::meteor_core::math::{BlockPos, Vec3d};

    fn animal(id: i32, kind: &str, x: f64, health: f32) -> EntityInfo {
        EntityInfo {
            id,
            kind: kind.into(),
            pos: Vec3d::new(x, 64.0, 0.5),
            prev_pos: Vec3d::new(x, 64.0, 0.5),
            alive: true,
            is_mob: true,
            health,
            max_health: 10.0,
            ..Default::default()
        }
    }

    fn igniter(slot: usize) -> InvSlot {
        InvSlot {
            index: slot,
            stack: Some(ItemStackInfo {
                id: "minecraft:flint_and_steel".into(),
                count: 1,
                ..Default::default()
            }),
        }
    }

    #[test]
    fn settings_match_the_java_module() {
        let m = Flamethrower::new();
        let names: Vec<&str> = m.base.settings.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "distance",
                "anti-break",
                "put-out-fire",
                "target-babies",
                "tick-interval",
                "rotate",
                "entities",
            ]
        );
        assert_eq!(m.base.title, "Flamethrower");
        assert_eq!(m.base.description, "Ignites every alive piece of food.");
        assert_eq!(m.tick_interval.get(), 5);
    }

    #[test]
    fn a_cow_standing_on_stone_is_set_alight() {
        let mut m = Flamethrower::new();
        let mut env = TestEnv::in_world();
        env.entities = vec![animal(4, "minecraft:cow", 2.5, 10.0)];
        env.inventory = vec![igniter(0)];
        m.tick_interval.set(0);
        env.push_block(BlockPos::new(2, 64, 0), "minecraft:air");
        env.push_block(BlockPos::new(2, 63, 0), "minecraft:stone");

        m.on_tick(&mut env);
        assert_eq!(m.target(), Some(4));
        assert!(
            (env.rotation.yaw + 90.0).abs() < 1.0,
            "the cow is due east, got {}",
            env.rotation.yaw
        );
        assert_eq!(m.ticks, 0, "the ignition timer was reset");
    }

    #[test]
    fn the_tick_interval_delays_the_ignition() {
        let mut m = Flamethrower::new();
        let mut env = TestEnv::in_world();
        env.entities = vec![animal(4, "minecraft:chicken", 2.5, 10.0)];
        env.inventory = vec![igniter(0)];
        env.push_block(BlockPos::new(2, 63, 0), "minecraft:stone");

        for _ in 0..4 {
            m.on_tick(&mut env);
        }
        assert_eq!(m.ticks, 4, "4 < tick-interval 5, so no ignition yet");
        assert_eq!(m.target(), Some(4));

        m.on_tick(&mut env);
        assert_eq!(m.ticks, 0, "ignited once the interval elapsed");
    }

    #[test]
    fn low_health_puts_the_fire_out_instead() {
        let mut m = Flamethrower::new();
        let mut env = TestEnv::in_world();
        env.entities = vec![animal(4, "minecraft:pig", 2.5, 1.0)];
        env.inventory = vec![igniter(0)];
        m.tick_interval.set(0);
        env.push_block(BlockPos::new(2, 63, 0), "minecraft:stone");

        m.on_tick(&mut env);
        // The pig's own block plus the four horizontal neighbours are "broken" to
        // smother the fire; the port has one `Action::Attack` per position.
        assert_eq!(
            env.actions.len(),
            5,
            "put-out-fire smacks the ground around the pig"
        );
        assert_eq!(m.ticks, 1, "no ignition happened, the timer kept running");
    }

    #[test]
    fn babies_water_and_missing_igniters_are_skipped() {
        let mut m = Flamethrower::new();
        let mut env = TestEnv::in_world();
        let mut baby = animal(4, "minecraft:cow", 2.5, 10.0);
        baby.baby = true;
        env.entities = vec![baby];
        env.inventory = vec![igniter(0)];
        m.on_tick(&mut env);
        assert_eq!(m.target(), None, "target-babies is off");

        env.entities = vec![animal(5, "minecraft:cow", 2.5, 10.0)];
        env.inventory.clear();
        m.on_tick(&mut env);
        assert_eq!(m.target(), None, "no flint and steel");

        // Water under the animal stops the interaction altogether.
        env.inventory = vec![igniter(0)];
        env.push_block(BlockPos::new(2, 63, 0), "minecraft:water[level=0]");
        m.tick_interval.set(0);
        m.on_tick(&mut env);
        assert_eq!(m.ticks, 3, "the water check returns before igniting");
    }
}
