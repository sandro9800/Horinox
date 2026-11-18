//! `systems.modules.world.LiquidFiller` - "places blocks inside of liquid source
//! blocks within range of you".

use crate::meteor_core::category::Category;
use crate::meteor_core::env::Env;
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet, EnumSet, IntSet, ListSet};

use super::{
    can_place, describe, hotbar_block_slot, liquid_source, place_block, stack_in, visible,
};

pub struct LiquidFiller {
    base: ModuleBase,
    place_in: EnumSet,
    shape: EnumSet,
    place_range: DoubleSet,
    walls_range: DoubleSet,
    delay: IntSet,
    max_blocks_per_tick: IntSet,
    sort_mode: EnumSet,
    rotate: BoolSet,
    // Whitelist
    list_mode: EnumSet,
    whitelist: ListSet,
    blacklist: ListSet,
    // Runtime
    candidates: Vec<BlockPos>,
    timer: i32,
}

impl LiquidFiller {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::World,
            "liquid-filler",
            "Places blocks inside of liquid source blocks within range of you.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();

        let place_in = g.enum_setting("place-in", &["both", "water", "lava"], 0);
        describe(g, "What type of liquids to place in.");

        let shape = g.enum_setting("shape", &["sphere", "uniform-cube"], 0);
        describe(g, "The shape of placing algorithm.");

        let place_range = g.double_setting("place-range", 4.5, 0.0, 6.0, 2);
        describe(g, "The range at which blocks can be placed.");

        let walls_range = g.double_setting("walls-range", 4.5, 0.0, 6.0, 2);
        describe(g, "Range in which to place when behind blocks.");

        let delay = g.int_setting("delay", 0, 0, 20);
        describe(g, "Delay between actions in ticks.");

        let max_blocks_per_tick = g.int_setting("max-blocks-per-tick", 1, 1, 10);
        describe(g, "Maximum blocks to try to place per tick.");

        let sort_mode = g.enum_setting(
            "sort-mode",
            &["none", "closest", "furthest", "top-down", "bottom-up"],
            2,
        );
        describe(g, "The blocks you want to place first.");

        let rotate = g.bool_setting("rotate", true);
        describe(
            g,
            "Automatically rotates towards the space targeted for filling.",
        );

        let w = s.create_group("Whitelist");

        let list_mode = w.enum_setting("list-mode", &["whitelist", "blacklist"], 0);
        describe(w, "Selection mode.");

        let whitelist = w.list_setting(
            "whitelist",
            &[
                "minecraft:dirt",
                "minecraft:cobblestone",
                "minecraft:stone",
                "minecraft:netherrack",
                "minecraft:diorite",
                "minecraft:granite",
                "minecraft:andesite",
            ],
        );
        describe(
            w,
            "The allowed blocks that it will use to fill up the liquid.",
        );
        let mode = list_mode.clone();
        visible(w, move || mode.is("whitelist"));

        let blacklist = w.list_setting("blacklist", &[]);
        describe(
            w,
            "The denied blocks that it not will use to fill up the liquid.",
        );
        let mode = list_mode.clone();
        visible(w, move || mode.is("blacklist"));

        Self {
            base,
            place_in,
            shape,
            place_range,
            walls_range,
            delay,
            max_blocks_per_tick,
            sort_mode,
            rotate,
            list_mode,
            whitelist,
            blacklist,
            candidates: Vec::new(),
            timer: 0,
        }
    }

    /// Whether a fluid kind passes the `place-in` setting.
    fn wants(&self, kind: &str) -> bool {
        match self.place_in.get() {
            "water" => kind == "water",
            "lava" => kind == "lava",
            _ => true,
        }
    }

    /// `LiquidFiller.isWithinShape(pos, range)`.
    fn within_shape(&self, player: Vec3d, pos: BlockPos, range: f64) -> bool {
        if self.shape.is("uniform-cube") {
            let feet = player.to_block_pos();
            let dx = (pos.x - feet.x).abs();
            let dy = (pos.y - feet.y).abs();
            let dz = (pos.z - feet.z).abs();
            return dx.max(dy).max(dz) as f64 <= range.floor();
        }
        player.distance_to(pos.center()) <= range
    }

    /// `LiquidFiller.isOutOfRange(pos)`: shape, then the wall raycast.
    fn out_of_range(&self, env: &dyn Env, player: Vec3d, pos: BlockPos) -> bool {
        if !self.within_shape(player, pos, self.place_range.get()) {
            return true;
        }
        let visible = matches!(
            env.raycast(self.place_range.get() as f32),
            crate::meteor_core::env::Raycast::Block { pos: hit, .. } if hit == pos
        );
        if visible {
            return false;
        }
        !self.within_shape(player, pos, self.walls_range.get())
    }

    /// The stack the module fills with: whitelisted block, or any block that is
    /// not blacklisted.
    fn fill_stack(&self, env: &dyn Env) -> Option<(usize, String)> {
        if self.list_mode.is("whitelist") {
            let slot = hotbar_block_slot(env, &self.whitelist)?;
            return stack_in(env, slot).map(|stack| (slot, stack.id));
        }
        env.inventory()
            .into_iter()
            .filter(|slot| slot.index < 9)
            .find(|slot| {
                slot.stack
                    .as_ref()
                    .is_some_and(|stack| !super::listed(&self.blacklist, &stack.id))
            })
            .and_then(|slot| slot.stack.map(|stack| (slot.index, stack.id)))
    }

    /// The positions the module wants to fill this tick.
    pub fn candidates(&self) -> &[BlockPos] {
        &self.candidates
    }
}

impl Default for LiquidFiller {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for LiquidFiller {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.timer = 0;
        self.candidates.clear();
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        // Update timer according to delay
        if self.timer < self.delay.get() {
            self.timer += 1;
            return;
        }
        self.timer = 0;

        let Some(player) = env.player() else { return };
        let Some((_slot, id)) = self.fill_stack(env) else {
            return;
        };

        // BlockIterator.register(ceil(range + 1), ceil(range), ...)
        let radius = self.place_range.get().ceil() as i32 + 1;
        let feet = player.pos.to_block_pos();
        self.candidates.clear();
        for dy in -radius..=radius {
            for dz in -radius..=radius {
                for dx in -radius..=radius {
                    let pos = feet.offset(dx, dy, dz);
                    if self.out_of_range(env, player.pos, pos) {
                        continue;
                    }
                    let Some(block) = env.block(pos) else {
                        continue;
                    };
                    let Some(kind) = liquid_source(&block) else {
                        continue;
                    };
                    if !self.wants(kind) {
                        continue;
                    }
                    if !can_place(env, pos) {
                        continue;
                    }
                    self.candidates.push(pos);
                }
            }
        }

        // Sort blocks
        match self.sort_mode.get() {
            "top-down" => self.candidates.sort_by_key(|pos| -pos.y),
            "bottom-up" => self.candidates.sort_by_key(|pos| pos.y),
            "none" => {}
            "closest" => self.candidates.sort_by(|a, b| {
                let da = player.pos.distance_to(a.center());
                let db = player.pos.distance_to(b.center());
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            }),
            _ => self.candidates.sort_by(|a, b| {
                let da = player.pos.distance_to(a.center());
                let db = player.pos.distance_to(b.center());
                db.partial_cmp(&da).unwrap_or(std::cmp::Ordering::Equal)
            }),
        }

        // Place
        let max = self.max_blocks_per_tick.get() as usize;
        for pos in self.candidates.iter().take(max) {
            if self.rotate.get() {
                super::rotate_to(env, *pos);
            }
            place_block(env, *pos, &id);
        }
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(LiquidFiller::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{BlockInfo, InvSlot, ItemStackInfo};
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

    #[test]
    fn settings_match_the_java_module() {
        let m = LiquidFiller::new();
        let groups: Vec<&str> = m
            .base
            .settings
            .groups
            .iter()
            .map(|g| g.name.as_str())
            .collect();
        assert_eq!(groups, vec!["General", "Whitelist"]);
        let names: Vec<&str> = m.base.settings.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "place-in",
                "shape",
                "place-range",
                "walls-range",
                "delay",
                "max-blocks-per-tick",
                "sort-mode",
                "rotate",
                "list-mode",
                "whitelist",
                "blacklist",
            ]
        );
        assert_eq!(m.base.title, "Liquid Filler");
        assert_eq!(
            m.base.description,
            "Places blocks inside of liquid source blocks within range of you."
        );
        assert_eq!(
            m.base.settings.get("sort-mode").unwrap().display(),
            "furthest"
        );
    }

    #[test]
    fn a_water_source_next_to_the_player_is_filled() {
        let mut m = LiquidFiller::new();
        let mut env = TestEnv::in_world();
        let water = BlockPos::new(1, 64, 0);
        env.push_block(water, "minecraft:water[level=0]");
        env.push_block(BlockPos::new(1, 63, 0), "minecraft:stone");
        env.inventory = vec![slot(0, "minecraft:dirt")];

        m.on_tick(&mut env);
        assert_eq!(env.placed, vec![(water, "minecraft:dirt".to_string())]);
        assert_eq!(m.candidates(), &[water]);
    }

    #[test]
    fn place_in_and_the_lists_choose_the_liquid_and_the_block() {
        let mut m = LiquidFiller::new();
        let mut env = TestEnv::in_world();
        let lava = BlockPos::new(1, 64, 0);
        env.push_block(lava, "minecraft:lava[level=0]");
        env.push_block(BlockPos::new(1, 63, 0), "minecraft:stone");
        env.inventory = vec![slot(0, "minecraft:dirt")];

        m.place_in.set_index(1); // water only
        m.on_tick(&mut env);
        assert!(env.placed.is_empty(), "lava is not on the menu");

        m.place_in.set_index(0); // both
        m.on_tick(&mut env);
        assert_eq!(env.placed.len(), 1);

        // Blacklist the dirt: nothing left to place with.
        env.placed.clear();
        m.list_mode.set_index(1);
        m.blacklist.set(vec!["minecraft:dirt".to_string()]);
        m.on_tick(&mut env);
        assert!(env.placed.is_empty(), "the only block is blacklisted");
    }

    #[test]
    fn flowing_liquid_blocks_are_left_alone() {
        let mut m = LiquidFiller::new();
        let mut env = TestEnv::in_world();
        env.push_block(BlockPos::new(1, 64, 0), "minecraft:water[level=4]");
        env.push_block(BlockPos::new(1, 63, 0), "minecraft:stone");
        env.inventory = vec![slot(0, "minecraft:dirt")];
        m.on_tick(&mut env);
        assert!(env.placed.is_empty(), "only source blocks are filled");
        assert_eq!(
            liquid_source(&BlockInfo::new("minecraft:water[level=4]")),
            None
        );
    }

    #[test]
    fn the_shape_and_range_limit_the_search() {
        let mut m = LiquidFiller::new();
        let mut env = TestEnv::in_world();
        let far = BlockPos::new(9, 64, 0);
        env.push_block(far, "minecraft:water[level=0]");
        env.push_block(BlockPos::new(9, 63, 0), "minecraft:stone");
        env.inventory = vec![slot(0, "minecraft:dirt")];
        m.on_tick(&mut env);
        assert!(env.placed.is_empty(), "9 blocks away is out of range");

        // `uniform-cube` uses the chebyshev distance against the player's block.
        let mut cube = LiquidFiller::new();
        let mut env = TestEnv::in_world();
        let pos = BlockPos::new(2, 65, 2);
        env.push_block(pos, "minecraft:water[level=0]");
        env.push_block(BlockPos::new(2, 64, 2), "minecraft:stone");
        env.inventory = vec![slot(0, "minecraft:dirt")];
        cube.shape.set_index(1);
        cube.place_range.set(4.0);
        cube.on_tick(&mut env);
        assert_eq!(env.placed.len(), 1, "chebyshev distance 2 within 4");
    }

    #[test]
    fn the_delay_and_the_per_tick_cap_hold_placements_back() {
        let mut m = LiquidFiller::new();
        let mut env = TestEnv::in_world();
        for x in 1..=3 {
            env.push_block(BlockPos::new(x, 64, 0), "minecraft:water[level=0]");
            env.push_block(BlockPos::new(x, 63, 0), "minecraft:stone");
        }
        env.inventory = vec![slot(0, "minecraft:stone")];
        m.max_blocks_per_tick.set(2);

        m.on_tick(&mut env);
        assert_eq!(env.placed.len(), 2, "max-blocks-per-tick caps the work");

        m.delay.set(3);
        env.placed.clear();
        m.on_tick(&mut env);
        m.on_tick(&mut env);
        assert!(env.placed.is_empty(), "still counting the delay down");
        m.on_tick(&mut env);
        m.on_tick(&mut env);
        assert!(!env.placed.is_empty(), "the delay elapsed");
    }
}
