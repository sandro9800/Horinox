//! `systems.modules.render.Marker` -- STUB: the port lands in this file.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::Env;
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::IntSet;

pub struct Marker {
    base: ModuleBase,
    range: IntSet,
}

impl Marker {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Render,
            "marker",
            "Renders shapes. Useful for large scale projects",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        let range = g.int_setting("range", 128, 1, 512);
        Self { base, range }
    }
}

impl Default for Marker {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Marker {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_tick(&mut self, _env: &mut dyn Env) {}
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(Marker::new()));
}
