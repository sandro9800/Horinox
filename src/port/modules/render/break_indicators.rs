//! `systems.modules.render.BreakIndicators` -- STUB: the port lands in this file.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::Env;
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::BoolSet;

pub struct BreakIndicators {
    base: ModuleBase,
    render: BoolSet,
}

impl BreakIndicators {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Render,
            "break-indicators",
            "Renders the progress of a block being broken.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        let render = g.bool_setting("render", true);
        Self { base, render }
    }
}

impl Default for BreakIndicators {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for BreakIndicators {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_tick(&mut self, _env: &mut dyn Env) {}
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(BreakIndicators::new()));
}
