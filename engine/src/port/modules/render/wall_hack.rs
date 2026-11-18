//! `systems.modules.render.WallHack` - makes blocks translucent.
//!
//! Ported from `render/WallHack.java`. The Java module rewrites the block render
//! layers; the port keeps the settings, the targeted block list and the on/off
//! state that drives it.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::Env;
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, IntSet, ListSet};

pub struct WallHack {
    base: ModuleBase,
    opacity: IntSet,
    blocks: ListSet,
    occlude_chunks: BoolSet,
    /// Whether the render layers are asked to be translucent right now.
    applied: bool,
}

impl WallHack {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(Category::Render, "wall-hack", "Makes blocks translucent.");
        let s = base.settings_mut();
        let opacity = s.default_group().int_setting("opacity", 0, 0, 255);
        s.describe_last("The opacity for rendered blocks.");
        let blocks = s.default_group().list_setting("blocks", &[]);
        s.describe_last("What blocks should be targeted for Wall Hack.");
        let occlude_chunks = s.default_group().bool_setting("occlude-chunks", false);
        s.describe_last("Whether caves should occlude underground (may look wonky when on).");

        Self {
            base,
            opacity,
            blocks,
            occlude_chunks,
            applied: false,
        }
    }

    /// `levelExtractor.allChanged()`'s on/off flag.
    pub fn applied(&self) -> bool {
        self.applied
    }

    /// The opacity the renderer should use (`Setting<Integer> opacity`).
    pub fn opacity(&self) -> u8 {
        self.opacity.get().clamp(0, 255) as u8
    }

    /// `blocks.get().isEmpty() || blocks.get().contains(block)`.
    pub fn targets(&self, block_id: &str) -> bool {
        self.blocks.get().is_empty() || self.blocks.contains(block_id)
    }

    /// `ChunkOcclusionEvent`: cancelling it is what `occlude-chunks` controls.
    pub fn cancel_occlusion(&self) -> bool {
        !self.occlude_chunks.get()
    }
}

impl Default for WallHack {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for WallHack {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        // PORT GAP: no `levelExtractor.allChanged()` / render-layer switch, so the
        // flag is all the module can hand the host.
        self.applied = true;
    }

    fn on_deactivate(&mut self, _env: &mut dyn Env) {
        self.applied = false;
    }

    fn on_tick(&mut self, _env: &mut dyn Env) {
        // The Java module has no tick; keeping `applied` in step with the settings
        // panel is all this does.
        self.applied = true;
    }

    fn info_string(&self) -> Option<String> {
        Some(self.opacity().to_string())
    }

    fn extra_widget(&self) -> Option<Vec<String>> {
        Some(vec![
            "Blocks: ".to_string()
                + if self.blocks.get().is_empty() {
                    "all"
                } else {
                    "listed"
                },
        ])
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(WallHack::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_match_the_java_table() {
        let m = WallHack::new();
        for name in ["opacity", "blocks", "occlude-chunks"] {
            assert!(
                m.base.settings.get(name).is_some(),
                "setting {name} missing"
            );
        }
        assert_eq!(m.base.settings.groups.len(), 1);
        assert_eq!(m.opacity.get(), 0);
        assert!(!m.occlude_chunks.get());
    }

    #[test]
    fn activation_flips_the_applied_flag() {
        let mut m = WallHack::new();
        let mut env = TestEnv::in_world();
        assert!(!m.applied());
        m.on_activate(&mut env);
        assert!(m.applied());
        m.on_deactivate(&mut env);
        assert!(!m.applied());
    }

    #[test]
    fn the_block_list_gates_targets() {
        let m = WallHack::new();
        assert!(
            m.targets("minecraft:stone"),
            "an empty list targets everything"
        );
        m.blocks.set(vec!["minecraft:stone".into()]);
        assert!(m.targets("minecraft:stone"));
        assert!(!m.targets("minecraft:dirt"));
        m.opacity.set(400);
        assert_eq!(m.opacity(), 255, "the range is 0..255");
        assert!(m.cancel_occlusion(), "occlude-chunks is off by default");
    }

    #[test]
    fn the_extra_widget_shows_the_list_state() {
        let mut m = WallHack::new();
        assert_eq!(m.extra_widget().unwrap_or_default(), vec!["Blocks: all"]);
        m.blocks.set(vec!["minecraft:stone".into()]);
        assert_eq!(m.extra_widget().unwrap_or_default(), vec!["Blocks: listed"]);
    }
}
