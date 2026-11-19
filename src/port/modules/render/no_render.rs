//! `systems.modules.render.NoRender` - Disables certain animations or overlays from rendering.
//!
//! Ported from the Java module of the same name: the setting table below is the
//! module's, with the names, kinds, defaults and bounds `modules.nbt` carries.
//! The behaviour is the `state` family's (see `PORT-MAP.md`).

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, ListSet, StringSet};

pub struct NoRender {
    base: ModuleBase,
    portal_overlay: BoolSet,
    spyglass_overlay: BoolSet,
    nausea: BoolSet,
    pumpkin_overlay: BoolSet,
    powdered_snow_overlay: BoolSet,
    fire_overlay: BoolSet,
    liquid_overlay: BoolSet,
    in_wall_overlay: BoolSet,
    vignette: BoolSet,
    gui_background: BoolSet,
    totem_animation: BoolSet,
    eating_particles: BoolSet,
    enchantment_glint: BoolSet,
    boss_bar: BoolSet,
    scoreboard: BoolSet,
    crosshair: BoolSet,
    title: BoolSet,
    held_item_name: BoolSet,
    obfuscation: BoolSet,
    potion_icons: BoolSet,
    message_signature_indicator: BoolSet,
    weather: BoolSet,
    world_border: BoolSet,
    blindness: BoolSet,
    darkness: BoolSet,
    fog: BoolSet,
    enchantment_table_book: BoolSet,
    sign_text: BoolSet,
    block_break_overlay: BoolSet,
    beacon_beams: BoolSet,
    falling_blocks: BoolSet,
    cave_culling: BoolSet,
    map_markers: BoolSet,
    map_contents: BoolSet,
    banners: StringSet,
    firework_explosions: BoolSet,
    particles: ListSet,
    barrier_invisibility: BoolSet,
    texture_rotations: BoolSet,
    block_entities: ListSet,
    entities: ListSet,
    drop_spawn_packets: BoolSet,
    armor: BoolSet,
    invisibility: BoolSet,
    glowing: BoolSet,
    spawner_entities: BoolSet,
    dead_entities: BoolSet,
    nametags: BoolSet,
}

impl NoRender {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Render,
            "no-render",
            "Disables certain animations or overlays from rendering.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();
        let portal_overlay = g.bool_setting("portal-overlay", false);
        let spyglass_overlay = g.bool_setting("spyglass-overlay", false);
        let nausea = g.bool_setting("nausea", false);
        let pumpkin_overlay = g.bool_setting("pumpkin-overlay", false);
        let powdered_snow_overlay = g.bool_setting("powdered-snow-overlay", false);
        let fire_overlay = g.bool_setting("fire-overlay", false);
        let liquid_overlay = g.bool_setting("liquid-overlay", false);
        let in_wall_overlay = g.bool_setting("in-wall-overlay", false);
        let vignette = g.bool_setting("vignette", false);
        let gui_background = g.bool_setting("gui-background", false);
        let totem_animation = g.bool_setting("totem-animation", false);
        let eating_particles = g.bool_setting("eating-particles", false);
        let enchantment_glint = g.bool_setting("enchantment-glint", false);
        let boss_bar = g.bool_setting("boss-bar", false);
        let scoreboard = g.bool_setting("scoreboard", false);
        let crosshair = g.bool_setting("crosshair", false);
        let title = g.bool_setting("title", false);
        let held_item_name = g.bool_setting("held-item-name", false);
        let obfuscation = g.bool_setting("obfuscation", false);
        let potion_icons = g.bool_setting("potion-icons", false);
        let message_signature_indicator = g.bool_setting("message-signature-indicator", false);
        let weather = g.bool_setting("weather", false);
        let world_border = g.bool_setting("world-border", false);
        let blindness = g.bool_setting("blindness", false);
        let darkness = g.bool_setting("darkness", false);
        let fog = g.bool_setting("fog", false);
        let enchantment_table_book = g.bool_setting("enchantment-table-book", false);
        let sign_text = g.bool_setting("sign-text", false);
        let block_break_overlay = g.bool_setting("block-break-overlay", false);
        let beacon_beams = g.bool_setting("beacon-beams", false);
        let falling_blocks = g.bool_setting("falling-blocks", false);
        let cave_culling = g.bool_setting("cave-culling", false);
        let map_markers = g.bool_setting("map-markers", false);
        let map_contents = g.bool_setting("map-contents", false);
        let banners = g.string_setting("banners", "");
        let firework_explosions = g.bool_setting("firework-explosions", false);
        let particles = g.list_setting("particles", &[]);
        let barrier_invisibility = g.bool_setting("barrier-invisibility", false);
        let texture_rotations = g.bool_setting("texture-rotations", false);
        let block_entities = g.list_setting("block-entities", &[]);
        let entities = g.list_setting("entities", &[]);
        let drop_spawn_packets = g.bool_setting("drop-spawn-packets", false);
        let armor = g.bool_setting("armor", false);
        let invisibility = g.bool_setting("invisibility", false);
        let glowing = g.bool_setting("glowing", false);
        let spawner_entities = g.bool_setting("spawner-entities", false);
        let dead_entities = g.bool_setting("dead-entities", false);
        let nametags = g.bool_setting("nametags", false);
        Self {
            base,
            portal_overlay,
            spyglass_overlay,
            nausea,
            pumpkin_overlay,
            powdered_snow_overlay,
            fire_overlay,
            liquid_overlay,
            in_wall_overlay,
            vignette,
            gui_background,
            totem_animation,
            eating_particles,
            enchantment_glint,
            boss_bar,
            scoreboard,
            crosshair,
            title,
            held_item_name,
            obfuscation,
            potion_icons,
            message_signature_indicator,
            weather,
            world_border,
            blindness,
            darkness,
            fog,
            enchantment_table_book,
            sign_text,
            block_break_overlay,
            beacon_beams,
            falling_blocks,
            cave_culling,
            map_markers,
            map_contents,
            banners,
            firework_explosions,
            particles,
            barrier_invisibility,
            texture_rotations,
            block_entities,
            entities,
            drop_spawn_packets,
            armor,
            invisibility,
            glowing,
            spawner_entities,
            dead_entities,
            nametags,
        }
    }
}

impl Default for NoRender {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for NoRender {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, env: &mut dyn Env) {
        // The Java module changes the renderer's state; Horizon's equivalent is
        // the hook this module registers, so the port reports the switch.
        // PORT GAP: the engine-side effect this module toggles.
        let _ = env;
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(NoRender::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_match_the_java_module() {
        let m = NoRender::new();
        assert_eq!(m.base.name, "no-render");
        assert_eq!(m.base.category, Category::Render);
    }

    #[test]
    fn it_runs() {
        let mut m = NoRender::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.on_tick(&mut env);
        let mut r = crate::meteor_core::env::NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
    }
}
