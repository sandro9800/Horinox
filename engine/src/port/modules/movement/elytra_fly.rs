//! `systems.modules.movement.ElytraFly` (stub, replaced below).

use crate::meteor_core::category::Category;
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, DoubleSet, EnumSet};

pub struct ElytraFly {
    base: ModuleBase,
    mode: EnumSet,
    speed: DoubleSet,
    auto_start: BoolSet,
}

impl ElytraFly {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Movement,
            "elytra-fly",
            "Gives you more control over your elytra.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        let mode = g.enum_setting("mode", &["vanilla", "control", "bounce", "packet"], 0);
        let speed = g.double_setting("speed", 1.0, 0.1, 10.0, 2);
        let auto_start = g.bool_setting("auto-start", false);
        Self {
            base,
            mode,
            speed,
            auto_start,
        }
    }
}

impl Default for ElytraFly {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for ElytraFly {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(ElytraFly::new()));
}
