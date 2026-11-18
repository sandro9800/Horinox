//! `systems.modules.world.AutoBreed` - "automatically breeds specified animals".

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{EntityInfo, Env, Hand, ItemStackInfo};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet, EnumSet, IntSet, ListSet};

use super::{describe, entity_center, entity_within, interact_entity, is_item, listed, visible};

/// `Animal.isFood(ItemStack)` for the animals the default entity list covers.
///
/// PORT GAP: `Animal.isFood` is implemented per entity in vanilla; the port keeps
/// the same table of breeding items, keyed by entity id.
fn breed_food(kind: &str) -> &'static [&'static str] {
    match kind.trim_start_matches("minecraft:") {
        "cow" | "mooshroom" | "sheep" | "goat" => &["minecraft:wheat"],
        "pig" => &["minecraft:carrot", "minecraft:potato", "minecraft:beetroot"],
        "chicken" => &[
            "minecraft:wheat_seeds",
            "minecraft:melon_seeds",
            "minecraft:pumpkin_seeds",
            "minecraft:beetroot_seeds",
            "minecraft:torchflower_seeds",
        ],
        "horse" | "donkey" | "mule" | "llama" | "trader_llama" => &[
            "minecraft:golden_apple",
            "minecraft:enchanted_golden_apple",
            "minecraft:golden_carrot",
            "minecraft:hay_block",
        ],
        "rabbit" => &[
            "minecraft:carrot",
            "minecraft:golden_carrot",
            "minecraft:dandelion",
        ],
        "wolf" => &[
            "minecraft:beef",
            "minecraft:chicken",
            "minecraft:porkchop",
            "minecraft:mutton",
            "minecraft:rabbit",
            "minecraft:cod",
            "minecraft:salmon",
            "minecraft:rotten_flesh",
        ],
        "cat" | "ocelot" => &["minecraft:cod", "minecraft:salmon"],
        "turtle" => &["minecraft:seagrass"],
        "panda" => &["minecraft:bamboo"],
        "fox" => &["minecraft:sweet_berries", "minecraft:glow_berries"],
        "strider" => &["minecraft:warped_fungus"],
        "hoglin" => &["minecraft:crimson_fungus"],
        "bee" => &[
            "minecraft:poppy",
            "minecraft:dandelion",
            "minecraft:cornflower",
            "minecraft:azure_bluet",
            "minecraft:oxeye_daisy",
            "minecraft:blue_orchid",
            "minecraft:allium",
            "minecraft:lily_of_the_valley",
            "minecraft:sunflower",
        ],
        _ => &[],
    }
}

/// `EntityAgeTest.test(entity)`.
fn age_matches(filter: &str, baby: bool) -> bool {
    match filter {
        "baby" => baby,
        "adult" => !baby,
        _ => true,
    }
}

pub struct AutoBreed {
    base: ModuleBase,
    entities: ListSet,
    range: DoubleSet,
    hand: EnumSet,
    mob_age_filter: EnumSet,
    continuous_breeding: BoolSet,
    breeding_interval: IntSet,
    /// `LinkedHashMap<Entity, Integer> animalsFed`.
    animals_fed: Vec<(i32, i64)>,
    tick_counter: i64,
}

impl AutoBreed {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::World,
            "auto-breed",
            "Automatically breeds specified animals.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();

        let entities = g.list_setting(
            "entities",
            &[
                "minecraft:horse",
                "minecraft:donkey",
                "minecraft:cow",
                "minecraft:mooshroom",
                "minecraft:sheep",
                "minecraft:pig",
                "minecraft:chicken",
                "minecraft:wolf",
                "minecraft:cat",
                "minecraft:ocelot",
                "minecraft:rabbit",
                "minecraft:llama",
                "minecraft:turtle",
                "minecraft:panda",
                "minecraft:fox",
                "minecraft:bee",
                "minecraft:strider",
                "minecraft:hoglin",
            ],
        );
        describe(g, "Entities to breed.");

        let range = g.double_setting("range", 4.5, 0.0, 10.0, 2);
        describe(g, "How far away the animals can be to be bred.");

        let hand = g.enum_setting("hand-for-breeding", &["main-hand", "off-hand"], 0);
        describe(g, "The hand to use for breeding.");

        let mob_age_filter = g.enum_setting("mob-age-filter", &["baby", "adult", "both"], 1);
        describe(
            g,
            "Determines the age of the mobs to target (baby, adult, or both).",
        );

        let continuous_breeding = g.bool_setting("continuous-breeding", false);
        describe(
            g,
            "Whether to feed the same animal again after a certain time period.",
        );

        // 30s in love mode and the 5-minute breeding cooldown.
        let breeding_interval = g.int_setting("breeding-interval", 6600, 1, 24000);
        describe(g, "Determines how often the same animal is fed in ticks.");
        let flag = continuous_breeding.clone();
        visible(g, move || flag.get());

        Self {
            base,
            entities,
            range,
            hand,
            mob_age_filter,
            continuous_breeding,
            breeding_interval,
            animals_fed: Vec::new(),
            tick_counter: 0,
        }
    }

    /// The stack the `hand-for-breeding` setting selects.
    fn held_stack(&self, env: &dyn Env) -> Option<ItemStackInfo> {
        if self.hand.is("off-hand") {
            // The `Env`'s inventory layout puts the off hand after the armour
            // slots; slot 40 is vanilla's off hand.
            super::stack_in(env, 40)
        } else {
            env.selected_stack()
        }
    }

    fn already_fed(&self, id: i32) -> bool {
        self.animals_fed.iter().any(|(fed, _)| *fed == id)
    }

    /// The animals fed so far this session, `(entity id, tick)`.
    pub fn animals_fed(&self) -> &[(i32, i64)] {
        &self.animals_fed
    }

    fn feed(&mut self, env: &mut dyn Env, entity: &EntityInfo) {
        // `Rotations.rotate(..., () -> { interact; swing; animalsFed.putLast; })`
        interact_entity(env, entity_center(entity), true);
        self.animals_fed.push((entity.id, self.tick_counter));
    }
}

impl Default for AutoBreed {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for AutoBreed {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.animals_fed.clear();
        self.tick_counter = 0;
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        let held = self.held_stack(env);
        let food_hand = match self.hand.is("off-hand") {
            true => Hand::Off,
            false => Hand::Main,
        };
        let _ = food_hand;

        let candidates: Vec<EntityInfo> = env.entities();
        for entity in &candidates {
            // `if (!(entity instanceof Animal animal)) continue;`
            if !entity.is_mob || !entity.alive {
                continue;
            }
            if !listed(&self.entities, &entity.kind) {
                continue;
            }
            if !age_matches(self.mob_age_filter.get(), entity.baby) {
                continue;
            }
            if self.already_fed(entity.id) {
                continue;
            }
            if !entity_within(env, entity, self.range.get()) {
                continue;
            }
            // `animal.isFood(held item)`
            let food = breed_food(&entity.kind);
            let Some(stack) = &held else { continue };
            if !is_item(stack, food) {
                continue;
            }

            self.feed(env, entity);
            break;
        }

        if self.continuous_breeding.get() {
            let interval = self.breeding_interval.get() as i64;
            while let Some((_, fed_at)) = self.animals_fed.first().copied() {
                if fed_at < self.tick_counter - interval {
                    self.animals_fed.remove(0);
                } else {
                    break;
                }
            }
            self.tick_counter += 1;
        }
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(AutoBreed::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{Action, InvSlot};
    use crate::meteor_core::math::Vec3d;

    fn cow(id: i32, x: f64) -> EntityInfo {
        EntityInfo {
            id,
            kind: "minecraft:cow".into(),
            pos: Vec3d::new(x, 64.0, 0.0),
            prev_pos: Vec3d::new(x, 64.0, 0.0),
            alive: true,
            is_mob: true,
            ..Default::default()
        }
    }

    fn with_wheat(env: &mut TestEnv) {
        env.inventory = vec![InvSlot {
            index: 0,
            stack: Some(ItemStackInfo {
                id: "minecraft:wheat".into(),
                count: 32,
                ..Default::default()
            }),
        }];
    }

    #[test]
    fn settings_match_the_java_module() {
        let m = AutoBreed::new();
        let names: Vec<&str> = m.base.settings.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "entities",
                "range",
                "hand-for-breeding",
                "mob-age-filter",
                "continuous-breeding",
                "breeding-interval",
            ]
        );
        assert_eq!(m.base.title, "Auto Breed");
        assert_eq!(
            m.base.description,
            "Automatically breeds specified animals."
        );
        assert!(!m
            .base
            .settings
            .get("breeding-interval")
            .unwrap()
            .is_visible());
        m.continuous_breeding.set(true);
        assert!(m
            .base
            .settings
            .get("breeding-interval")
            .unwrap()
            .is_visible());
        assert_eq!(
            m.base.settings.get("mob-age-filter").unwrap().display(),
            "adult"
        );
    }

    #[test]
    fn a_nearby_adult_cow_is_fed_once() {
        let mut m = AutoBreed::new();
        let mut env = TestEnv::in_world();
        with_wheat(&mut env);
        env.entities = vec![cow(7, 2.0)];

        m.on_activate(&mut env);
        m.on_tick(&mut env);
        assert_eq!(env.actions, vec![Action::Use], "interact with the cow");
        assert_eq!(m.animals_fed(), &[(7, 0)]);
        assert!(
            env.rotation.yaw.abs() > 1.0,
            "Rotations.rotate turned to it"
        );

        // `animalsFed.containsKey(animal)` stops a second helping.
        m.on_tick(&mut env);
        assert_eq!(env.actions.len(), 1);
    }

    #[test]
    fn babies_adults_and_missing_food_are_respected() {
        let mut m = AutoBreed::new();
        let mut env = TestEnv::in_world();
        with_wheat(&mut env);
        let mut baby = cow(9, 2.0);
        baby.baby = true;
        env.entities = vec![baby];

        m.on_tick(&mut env);
        assert!(env.actions.is_empty(), "mob-age-filter is adult by default");

        m.mob_age_filter.set_index(0); // baby
        m.on_tick(&mut env);
        assert_eq!(env.actions.len(), 1);

        // No food in hand, no feeding.
        let mut m = AutoBreed::new();
        let mut env = TestEnv::in_world();
        env.entities = vec![cow(7, 2.0)];
        m.on_tick(&mut env);
        assert!(env.actions.is_empty(), "animal.isFood(held) is false");

        // Out of range.
        let mut m = AutoBreed::new();
        let mut env = TestEnv::in_world();
        with_wheat(&mut env);
        env.entities = vec![cow(7, 30.0)];
        m.on_tick(&mut env);
        assert!(env.actions.is_empty(), "range is 4.5 blocks");
    }

    #[test]
    fn continuous_breeding_prunes_the_fed_list_after_the_interval() {
        let mut m = AutoBreed::new();
        let mut env = TestEnv::in_world();
        with_wheat(&mut env);
        env.entities = vec![cow(7, 2.0)];
        m.continuous_breeding.set(true);
        m.breeding_interval.set(1);

        m.on_tick(&mut env);
        assert_eq!(m.animals_fed().len(), 1);
        m.on_tick(&mut env);
        m.on_tick(&mut env);
        assert!(
            m.animals_fed().is_empty(),
            "fed at tick 0, pruned once 0 < tick - 1"
        );
        m.on_tick(&mut env);
        assert_eq!(env.actions.len(), 2, "the cow is fed again");
    }
}
