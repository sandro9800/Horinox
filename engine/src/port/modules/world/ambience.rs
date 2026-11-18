//! `systems.modules.world.Ambience` - change the colour of the sky, the clouds,
//! lightning, grass, foliage, water, lava and fog.
//!
//! Java reaches into `LevelExtractor.allChanged()` and the renderer's colour
//! providers; the `Env` exposes no renderer state, so the module keeps the whole
//! setting table plus the per-dimension lookup the shaders used, and records
//! every `reload()` for the host to act on.

use crate::meteor_core::category::Category;
use crate::meteor_core::color::{Color, SettingColor};
use crate::meteor_core::env::{Env, WorldInfo};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, ColorSet};

use super::{describe, visible};

/// `PlayerUtils.getDimension()`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Dimension {
    Overworld,
    Nether,
    End,
}

fn dimension_of(world: Option<&WorldInfo>) -> Dimension {
    match world.map(|w| w.dimension.as_str()) {
        Some(d) if d.contains("nether") => Dimension::Nether,
        Some(d) if d.contains("end") => Dimension::End,
        _ => Dimension::Overworld,
    }
}

pub struct Ambience {
    base: ModuleBase,
    // Sky
    pub end_sky: BoolSet,
    pub custom_sky_color: BoolSet,
    overworld_sky_color: ColorSet,
    nether_sky_color: ColorSet,
    end_sky_color: ColorSet,
    pub custom_cloud_color: BoolSet,
    cloud_color: ColorSet,
    pub change_lightning_color: BoolSet,
    lightning_color: ColorSet,
    // World
    pub custom_grass_color: BoolSet,
    grass_color: ColorSet,
    pub custom_foliage_color: BoolSet,
    foliage_color: ColorSet,
    pub custom_water_color: BoolSet,
    water_color: ColorSet,
    pub custom_lava_color: BoolSet,
    lava_color: ColorSet,
    pub custom_fog_color: BoolSet,
    fog_color: ColorSet,
    /// `reload()` calls. The port's stand-in for `LevelExtractor.allChanged()`.
    reloads: u32,
}

impl Ambience {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::World,
            "ambience",
            "Change the color of various pieces of the environment.",
        );
        let mut s = base.settings_mut();

        // Sky
        let sky = s.create_group("Sky");

        let end_sky = sky.bool_setting("end-sky", false);
        describe(sky, "Makes the sky like the end.");

        let custom_sky_color = sky.bool_setting("custom-sky-color", false);
        describe(sky, "Whether the sky color should be changed.");

        let overworld_sky_color =
            sky.color_setting("overworld-sky-color", SettingColor::new(0, 125, 255, 255));
        describe(sky, "The color of the overworld sky.");
        let flag = custom_sky_color.clone();
        visible(sky, move || flag.get());

        let nether_sky_color =
            sky.color_setting("nether-sky-color", SettingColor::new(102, 0, 0, 255));
        describe(sky, "The color of the nether sky.");
        let flag = custom_sky_color.clone();
        visible(sky, move || flag.get());

        let end_sky_color = sky.color_setting("end-sky-color", SettingColor::new(65, 30, 90, 255));
        describe(sky, "The color of the end sky.");
        let flag = custom_sky_color.clone();
        visible(sky, move || flag.get());

        let custom_cloud_color = sky.bool_setting("custom-cloud-color", false);
        describe(sky, "Whether the clouds color should be changed.");

        let cloud_color = sky.color_setting("cloud-color", SettingColor::new(102, 0, 0, 255));
        describe(sky, "The color of the clouds.");
        let flag = custom_cloud_color.clone();
        visible(sky, move || flag.get());

        let change_lightning_color = sky.bool_setting("custom-lightning-color", false);
        describe(sky, "Whether the lightning color should be changed.");

        let lightning_color =
            sky.color_setting("lightning-color", SettingColor::new(102, 0, 0, 255));
        describe(sky, "The color of the lightning.");
        let flag = change_lightning_color.clone();
        visible(sky, move || flag.get());

        // World
        let world = s.create_group("World");

        let custom_grass_color = world.bool_setting("custom-grass-color", false);
        describe(world, "Whether the grass color should be changed.");

        let grass_color = world.color_setting("grass-color", SettingColor::new(102, 0, 0, 255));
        describe(world, "The color of the grass.");
        let flag = custom_grass_color.clone();
        visible(world, move || flag.get());

        let custom_foliage_color = world.bool_setting("custom-foliage-color", false);
        describe(world, "Whether the foliage color should be changed.");

        let foliage_color = world.color_setting("foliage-color", SettingColor::new(102, 0, 0, 255));
        describe(world, "The color of the foliage.");
        let flag = custom_foliage_color.clone();
        visible(world, move || flag.get());

        let custom_water_color = world.bool_setting("custom-water-color", false);
        describe(world, "Whether the water color should be changed.");

        let water_color = world.color_setting("water-color", SettingColor::new(102, 0, 0, 255));
        describe(world, "The color of the water.");
        let flag = custom_water_color.clone();
        visible(world, move || flag.get());

        let custom_lava_color = world.bool_setting("custom-lava-color", false);
        describe(world, "Whether the lava color should be changed.");

        let lava_color = world.color_setting("lava-color", SettingColor::new(102, 0, 0, 255));
        describe(world, "The color of the lava.");
        let flag = custom_lava_color.clone();
        visible(world, move || flag.get());

        let custom_fog_color = world.bool_setting("custom-fog-color", false);
        describe(world, "Whether the fog color should be changed.");

        let fog_color = world.color_setting("fog-color", SettingColor::new(102, 0, 0, 255));
        describe(world, "The color of the fog.");
        let flag = custom_fog_color.clone();
        visible(world, move || flag.get());

        Self {
            base,
            end_sky,
            custom_sky_color,
            overworld_sky_color,
            nether_sky_color,
            end_sky_color,
            custom_cloud_color,
            cloud_color,
            change_lightning_color,
            lightning_color,
            custom_grass_color,
            grass_color,
            custom_foliage_color,
            foliage_color,
            custom_water_color,
            water_color,
            custom_lava_color,
            lava_color,
            custom_fog_color,
            fog_color,
            reloads: 0,
        }
    }

    /// `Ambience.reload()`: ask the renderer to re-extract its chunks.
    ///
    /// PORT GAP: `LevelExtractor.allChanged()` - Horizon's mod API does not hand
    /// a mod the level renderer, so the call is counted for the host instead of
    /// invalidating section meshes directly.
    fn reload(&mut self, _env: &mut dyn Env) {
        self.reloads += 1;
    }

    /// `Ambience.skyColor()`.
    pub fn sky_color(&self, world: Option<&WorldInfo>) -> Option<Color> {
        Some(match dimension_of(world) {
            Dimension::Overworld => self.overworld_sky_color.color(),
            Dimension::Nether => self.nether_sky_color.color(),
            Dimension::End => self.end_sky_color.color(),
        })
    }

    /// The colour the cloud layer uses when `custom-cloud-color` is on.
    pub fn cloud(&self) -> Option<Color> {
        self.custom_cloud_color
            .get()
            .then(|| self.cloud_color.color())
    }

    pub fn lightning(&self) -> Option<Color> {
        self.change_lightning_color
            .get()
            .then(|| self.lightning_color.color())
    }

    pub fn grass(&self) -> Option<Color> {
        self.custom_grass_color
            .get()
            .then(|| self.grass_color.color())
    }

    pub fn foliage(&self) -> Option<Color> {
        self.custom_foliage_color
            .get()
            .then(|| self.foliage_color.color())
    }

    pub fn water(&self) -> Option<Color> {
        self.custom_water_color
            .get()
            .then(|| self.water_color.color())
    }

    pub fn lava(&self) -> Option<Color> {
        self.custom_lava_color
            .get()
            .then(|| self.lava_color.color())
    }

    pub fn fog(&self) -> Option<Color> {
        self.custom_fog_color.get().then(|| self.fog_color.color())
    }

    /// How many times the module has asked for a re-extract.
    pub fn reloads(&self) -> u32 {
        self.reloads
    }
}

impl Default for Ambience {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Ambience {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, env: &mut dyn Env) {
        self.reload(env);
    }

    fn on_deactivate(&mut self, env: &mut dyn Env) {
        self.reload(env);
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(Ambience::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_are_the_java_table_in_two_groups() {
        let m = Ambience::new();
        let groups: Vec<&str> = m
            .base
            .settings
            .groups
            .iter()
            .map(|g| g.name.as_str())
            .collect();
        assert_eq!(groups, vec!["Sky", "World"]);
        let sky: Vec<&str> = m.base.settings.groups[0]
            .settings
            .iter()
            .map(|s| s.name.as_str())
            .collect();
        assert_eq!(
            sky,
            vec![
                "end-sky",
                "custom-sky-color",
                "overworld-sky-color",
                "nether-sky-color",
                "end-sky-color",
                "custom-cloud-color",
                "cloud-color",
                "custom-lightning-color",
                "lightning-color",
            ]
        );
        assert_eq!(m.base.title, "Ambience");
        assert_eq!(
            m.base.description,
            "Change the color of various pieces of the environment."
        );
    }

    #[test]
    fn activate_and_deactivate_reload_the_level() {
        let mut m = Ambience::new();
        let mut env = TestEnv::in_world();
        assert_eq!(m.reloads(), 0);
        m.on_activate(&mut env);
        assert_eq!(m.reloads(), 1, "onActivate() -> reload()");
        m.on_deactivate(&mut env);
        assert_eq!(m.reloads(), 2, "onDeactivate() -> reload()");
    }

    #[test]
    fn sky_colour_follows_the_dimension_and_visibility_follows_the_toggle() {
        let mut m = Ambience::new();
        let env = TestEnv::in_world();

        let overworld = env.world();
        assert_eq!(
            m.sky_color(overworld.as_ref()),
            Some(Color::new(0, 125, 255, 255))
        );

        let mut nether_env = TestEnv::in_world();
        nether_env.world.as_mut().unwrap().dimension = "minecraft:the_nether".into();
        let nether = nether_env.world();
        assert_eq!(
            m.sky_color(nether.as_ref()),
            Some(Color::new(102, 0, 0, 255))
        );

        let mut end_env = TestEnv::in_world();
        end_env.world.as_mut().unwrap().dimension = "minecraft:the_end".into();
        let end = end_env.world();
        assert_eq!(m.sky_color(end.as_ref()), Some(Color::new(65, 30, 90, 255)));

        // `visible(customSkyColor::get)` on the three sky colours.
        assert!(!m
            .base
            .settings
            .get("overworld-sky-color")
            .unwrap()
            .is_visible());
        m.custom_sky_color.set(true);
        assert!(m
            .base
            .settings
            .get("overworld-sky-color")
            .unwrap()
            .is_visible());
        assert!(m
            .base
            .settings
            .get("nether-sky-color")
            .unwrap()
            .is_visible());
        assert!(!m.base.settings.get("cloud-color").unwrap().is_visible());
        assert_eq!(m.cloud(), None, "custom-cloud-color is off by default");
        m.custom_cloud_color.set(true);
        assert_eq!(m.cloud(), Some(Color::new(102, 0, 0, 255)));
    }

    #[test]
    fn end_sky_setting_holds_its_default() {
        let m = Ambience::new();
        assert!(!m.end_sky.get());
        m.end_sky.set(true);
        assert!(m.end_sky.get());
        assert!(!m.custom_water_color.get());
        assert!(!m.custom_fog_color.get());
        assert!(!m.change_lightning_color.get());
    }
}
