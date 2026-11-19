//! `systems.modules.movement.Slippy` - changes the base friction level of
//! blocks.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::Env;
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{DoubleSet, EnumSet, ListSet};

pub struct Slippy {
    base: ModuleBase,
    // General
    /// `friction`
    pub friction: DoubleSet,
    /// `list-mode`
    pub list_mode: EnumSet,
    /// `ignored-blocks`
    pub ignored_blocks: ListSet,
    /// `allowed-blocks`
    pub allowed_blocks: ListSet,
}

impl Slippy {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Movement,
            "slippy",
            "Changes the base friction level of blocks.",
        );
        let mut s = base.settings_mut();
        let friction = s
            .default_group()
            .double_setting("friction", 1.0, 0.01, 1.10, 3);
        s.describe_last("The base friction level.");
        let list_mode = s
            .default_group()
            .enum_setting("list-mode", &["Whitelist", "Blacklist"], 1);
        s.describe_last("The mode to select blocks.");
        let ignored_blocks = s.default_group().list_setting("ignored-blocks", &[]);
        s.describe_last("Decide which blocks not to slip on");
        {
            let c = list_mode.clone();
            s.set_visible_last(std::sync::Arc::new(move || c.is("Blacklist")));
        }
        let allowed_blocks = s.default_group().list_setting("allowed-blocks", &[]);
        s.describe_last("Decide which blocks to slip on");
        {
            let c = list_mode.clone();
            s.set_visible_last(std::sync::Arc::new(move || c.is("Whitelist")));
        }

        Self {
            base,
            friction,
            list_mode,
            ignored_blocks,
            allowed_blocks,
        }
    }

    /// The friction the module reports for a block, `None` when it leaves the
    /// block alone.
    ///
    /// PORT GAP: the Java module is only a set of settings - the friction is read
    /// by `BlockBehaviour`/`Slipperiness` mixins, which `Env` has no hook for, so
    /// the lookup is a method for the engine side to call.
    pub fn friction_for(&self, block_id: &str) -> Option<f64> {
        let listed = if self.list_mode.is("Whitelist") {
            self.allowed_blocks.contains(block_id)
        } else {
            self.ignored_blocks.contains(block_id)
        };

        let slip = if self.list_mode.is("Whitelist") {
            listed
        } else {
            !listed
        };

        if slip {
            Some(self.friction.get())
        } else {
            None
        }
    }

    /// The friction for the block the player stands on.
    pub fn friction_under(&self, env: &dyn Env) -> Option<f64> {
        let below = super::block_below(env)?;
        let block = env.block(below)?;
        self.friction_for(&block.id)
    }
}

impl Default for Slippy {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Slippy {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn info_string(&self) -> Option<String> {
        Some(format!("{:.2}", self.friction.get()))
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(Slippy::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::math::BlockPos;

    #[test]
    fn blacklist_slips_on_everything_else() {
        let m = Slippy::new();
        assert_eq!(m.friction_for("minecraft:stone"), Some(1.0));

        m.ignored_blocks.set(vec!["minecraft:stone".to_string()]);
        assert_eq!(m.friction_for("minecraft:stone"), None);
        assert_eq!(m.friction_for("minecraft:dirt"), Some(1.0));
    }

    #[test]
    fn whitelist_only_slips_on_the_list() {
        let m = Slippy::new();
        m.list_mode.set_index(0); // Whitelist
        assert_eq!(m.friction_for("minecraft:stone"), None);

        m.allowed_blocks.set(vec!["minecraft:stone".to_string()]);
        assert_eq!(m.friction_for("minecraft:stone"), Some(1.0));
    }

    #[test]
    fn the_block_under_the_player_is_used() {
        let mut m = Slippy::new();
        let mut env = TestEnv::in_world();
        env.push_block(BlockPos::new(0, 63, 0), "minecraft:ice");

        assert_eq!(m.friction_under(&env), Some(1.0));

        m.friction.set(0.4);
        assert_eq!(m.friction_under(&env), Some(0.4));
        assert_eq!(m.info_string().as_deref(), Some("0.40"));
    }
}
