//! `systems.modules.render.BlockESP` -- STUB: the port lands in this file.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::Env;
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{ColorSet, IntSet};

pub struct BlockESP {
    base: ModuleBase,
    range: IntSet,
    color: ColorSet,
}

impl BlockESP {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Render,
            "block-esp",
            "Renders specified blocks through walls.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        let range = g.int_setting("range", 64, 1, 256);
        let color = g.color_setting(
            "color",
            crate::meteor_core::color::SettingColor::rgb(145, 61, 226),
        );
        Self { base, range, color }
    }
}

impl Default for BlockESP {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for BlockESP {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_tick(&mut self, _env: &mut dyn Env) {}
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(BlockESP::new()));
}
