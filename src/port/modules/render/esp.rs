//! `systems.modules.render.ESP` - renders entities through walls.
//!
//! Ported from `render/ESP.java`. The Java module has five render modes; the port
//! keeps the `box` and `wireframe` geometry (expressible with
//! `box_outline`/`filled_box`) and reuses it for `2d`, `shader` and `glow`, which
//! need engine passes the renderer does not expose.

use std::sync::Arc;

use crate::meteor_core::category::Category;
use crate::meteor_core::color::{Color, SettingColor};
use crate::meteor_core::env::{EntityInfo, Env, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::Vec3d;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, ColorSet, DoubleSet, EnumSet, ListSet};

/// `ShapeMode` - which parts of a box a module draws.
pub(crate) const SHAPE_MODES: &[&str] = &["lines", "sides", "both"];

pub(crate) fn draw_sides(mode: &EnumSet) -> bool {
    !mode.is("lines")
}

pub(crate) fn draw_lines(mode: &EnumSet) -> bool {
    !mode.is("sides")
}

pub struct ESP {
    base: ModuleBase,
    // General
    mode: EnumSet,
    highlight_target: BoolSet,
    target_hitbox: BoolSet,
    outline_width: DoubleSet,
    glow_multiplier: DoubleSet,
    ignore_self: BoolSet,
    shape_mode: EnumSet,
    fill_opacity: DoubleSet,
    fade_distance: DoubleSet,
    entities: ListSet,
    // Colors
    color_mode: EnumSet,
    friend_override: BoolSet,
    non_living_entity_color: ColorSet,
    players_color: ColorSet,
    animals_color: ColorSet,
    water_animals_color: ColorSet,
    monsters_color: ColorSet,
    ambient_color: ColorSet,
    misc_color: ColorSet,
    target_color: ColorSet,
    target_hitbox_color: ColorSet,
    // Runtime
    /// `count`, the `getInfoString()` counter.
    count: usize,
}

impl ESP {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(Category::Render, "esp", "Renders entities through walls.");

        let s = base.settings_mut();
        // General
        let mode = s.default_group().enum_setting(
            "mode",
            &["box", "wireframe", "2d", "shader", "glow"],
            3,
        );
        s.describe_last("Rendering mode.");

        let highlight_target = s.default_group().bool_setting("highlight-target", false);
        s.describe_last("highlights the currently targeted entity differently");

        let target_hitbox = s.default_group().bool_setting("target-hitbox", true);
        s.describe_last("draw the hitbox of the target entity");
        let h = highlight_target.clone();
        s.set_visible_last(Arc::new(move || h.get()));

        let outline_width = s
            .default_group()
            .double_setting("outline-width", 2.0, 1.0, 10.0, 0);
        s.describe_last("The width of the shader outline.");
        let m = mode.clone();
        s.set_visible_last(Arc::new(move || m.is("shader")));

        let glow_multiplier =
            s.default_group()
                .double_setting("glow-multiplier", 3.5, 0.0, 10.0, 3);
        s.describe_last("Multiplier for glow effect");
        let m = mode.clone();
        s.set_visible_last(Arc::new(move || m.is("shader")));

        let ignore_self = s.default_group().bool_setting("ignore-self", true);
        s.describe_last("Ignores yourself drawing the shader.");

        let shape_mode = s.default_group().enum_setting("shape-mode", SHAPE_MODES, 2);
        s.describe_last("How the shapes are rendered.");
        let m = mode.clone();
        s.set_visible_last(Arc::new(move || !m.is("glow")));

        let fill_opacity = s
            .default_group()
            .double_setting("fill-opacity", 0.3, 0.0, 1.0, 2);
        s.describe_last("The opacity of the shape fill.");
        let sm = shape_mode.clone();
        let m = mode.clone();
        s.set_visible_last(Arc::new(move || !sm.is("lines") && !m.is("glow")));

        let fade_distance = s
            .default_group()
            .double_setting("fade-distance", 3.0, 0.0, 12.0, 1);
        s.describe_last("The distance from an entity where the color begins to fade.");

        let entities = s
            .default_group()
            .list_setting("entities", &["minecraft:player"]);
        s.describe_last("Select specific entities.");

        // Colors
        s.create_group("Colors");

        let color_mode = s.groups.last_mut().unwrap().enum_setting(
            "color-mode",
            &["entity-type", "distance", "health"],
            0,
        );
        s.describe_last("Determines the colors used for entities.");

        let friend_override = s
            .groups
            .last_mut()
            .unwrap()
            .bool_setting("show-friend-colors", true);
        s.describe_last(
            "Whether or not to override the distance/health color of friends with the friend color.",
        );
        let c = color_mode.clone();
        s.set_visible_last(Arc::new(move || c.is("distance") || c.is("health")));

        let non_living_entity_color = s
            .groups
            .last_mut()
            .unwrap()
            .color_setting("non-living-entity-color", SettingColor::rgb(25, 25, 25));
        s.describe_last("The color used for non living entities such as dropped items.");
        let c = color_mode.clone();
        s.set_visible_last(Arc::new(move || c.is("health")));

        let players_color = s
            .groups
            .last_mut()
            .unwrap()
            .color_setting("players-color", SettingColor::rgb(255, 255, 255));
        s.describe_last("The other player's color.");
        let c = color_mode.clone();
        s.set_visible_last(Arc::new(move || c.is("entity-type")));

        let animals_color = s
            .groups
            .last_mut()
            .unwrap()
            .color_setting("animals-color", SettingColor::rgb(25, 255, 25));
        s.describe_last("The animal's color.");
        let c = color_mode.clone();
        s.set_visible_last(Arc::new(move || c.is("entity-type")));

        let water_animals_color = s
            .groups
            .last_mut()
            .unwrap()
            .color_setting("water-animals-color", SettingColor::rgb(25, 25, 255));
        s.describe_last("The water animal's color.");
        let c = color_mode.clone();
        s.set_visible_last(Arc::new(move || c.is("entity-type")));

        let monsters_color = s
            .groups
            .last_mut()
            .unwrap()
            .color_setting("monsters-color", SettingColor::rgb(255, 25, 25));
        s.describe_last("The monster's color.");
        let c = color_mode.clone();
        s.set_visible_last(Arc::new(move || c.is("entity-type")));

        let ambient_color = s
            .groups
            .last_mut()
            .unwrap()
            .color_setting("ambient-color", SettingColor::rgb(25, 25, 25));
        s.describe_last("The ambient's color.");
        let c = color_mode.clone();
        s.set_visible_last(Arc::new(move || c.is("entity-type")));

        let misc_color = s
            .groups
            .last_mut()
            .unwrap()
            .color_setting("misc-color", SettingColor::rgb(175, 175, 175));
        s.describe_last("The misc color.");
        let c = color_mode.clone();
        s.set_visible_last(Arc::new(move || c.is("entity-type")));

        let target_color = s
            .groups
            .last_mut()
            .unwrap()
            .color_setting("target-color", SettingColor::rgb(200, 200, 200));
        s.describe_last("The target color.");
        let h = highlight_target.clone();
        s.set_visible_last(Arc::new(move || h.get()));

        let target_hitbox_color = s
            .groups
            .last_mut()
            .unwrap()
            .color_setting("target-hitbox-color", SettingColor::rgb(100, 200, 200));
        s.describe_last("The target hitbox color.");
        let h = highlight_target.clone();
        let t = target_hitbox.clone();
        s.set_visible_last(Arc::new(move || h.get() && t.get()));

        Self {
            base,
            mode,
            highlight_target,
            target_hitbox,
            outline_width,
            glow_multiplier,
            ignore_self,
            shape_mode,
            fill_opacity,
            fade_distance,
            entities,
            color_mode,
            friend_override,
            non_living_entity_color,
            players_color,
            animals_color,
            water_animals_color,
            monsters_color,
            ambient_color,
            misc_color,
            target_color,
            target_hitbox_color,
            count: 0,
        }
    }

    /// `ESP.isShader()` / `isGlow()`.
    pub fn is_pass_mode(&self) -> bool {
        self.mode.is("shader") || self.mode.is("glow")
    }

    /// `ESP.drawAsTarget(Entity)`.
    pub fn draw_as_target(&self, env: &dyn Env, entity: &EntityInfo) -> bool {
        self.highlight_target.get()
            && env
                .crosshair_entity()
                .map(|e| e.id == entity.id)
                .unwrap_or(false)
    }

    /// `ESP.shouldSkip(Entity)`: the `entities` list holds ids; the local player is
    /// matched by name because the port has no entity reference to compare.
    pub fn should_skip(&self, env: &dyn Env, entity: &EntityInfo) -> bool {
        if self.draw_as_target(env, entity) {
            return false;
        }
        if !self.entity_enabled(entity) {
            return true;
        }
        if self.ignore_self.get() {
            if let Some(p) = env.player() {
                if entity.is_player && entity.name.as_deref() == Some(p.name.as_str()) {
                    return true;
                }
            }
        }
        false
    }

    /// `entities.get().contains(entity.getType())`.
    pub fn entity_enabled(&self, entity: &EntityInfo) -> bool {
        self.entities.contains(&entity.kind) || self.entities.contains(entity.short_kind())
    }

    /// `ESP.getColor(Entity)`: `None` when the entity fades to nothing.
    pub fn color_of(&self, env: &dyn Env, entity: &EntityInfo) -> Option<Color> {
        let color;
        let alpha;
        if self.draw_as_target(env, entity) {
            color = self.target_color.color();
            alpha = 1.0;
        } else {
            if !self.entity_enabled(entity) {
                return None;
            }
            alpha = self.fade_alpha(env, entity);
            if alpha == 0.0 {
                return None;
            }
            color = self.entity_type_color(env, entity);
        }
        Some(color.with_alpha((color.a as f64 * alpha) as u8))
    }

    /// `ESP.getFadeAlpha(Entity)`.
    fn fade_alpha(&self, env: &dyn Env, entity: &EntityInfo) -> f64 {
        let camera = env.player().map(|p| p.eye_pos()).unwrap_or(Vec3d::ZERO);
        let dist = camera.distance_sq(entity.pos.add(Vec3d::new(0.0, 1.62, 0.0)));
        let fade_dist = self.fade_distance.get().powi(2);
        let mut alpha = 1.0;
        if fade_dist > 0.0 && dist <= fade_dist * fade_dist {
            alpha = dist.sqrt() / fade_dist;
        }
        if alpha <= 0.075 {
            alpha = 0.0;
        }
        alpha
    }

    /// `ESP.getEntityTypeColor(Entity)`.
    pub fn entity_type_color(&self, env: &dyn Env, entity: &EntityInfo) -> Color {
        if self.color_mode.is("entity-type") {
            if entity.is_player {
                let name = entity.name.clone().unwrap_or_default();
                if env.is_friend(&name) {
                    return Color::GREEN;
                }
                return self.players_color.color();
            }
            let kind = entity.short_kind();
            if is_water_mob(kind) {
                return self.water_animals_color.color();
            }
            if is_monster(kind) {
                return self.monsters_color.color();
            }
            if is_animal(kind) {
                return self.animals_color.color();
            }
            if is_ambient(kind) {
                return self.ambient_color.color();
            }
            return self.misc_color.color();
        }

        if self.friend_override.get() && entity.is_player {
            let name = entity.name.clone().unwrap_or_default();
            if env.is_friend(&name) {
                return Color::GREEN;
            }
        }

        if self.color_mode.is("health") {
            if entity.max_health <= 0.0 {
                return self.non_living_entity_color.color();
            }
            let pct = (entity.health / entity.max_health).clamp(0.0, 1.0) as f64;
            return Color::RED.lerp(Color::GREEN, pct);
        }
        distance_color(entity.distance)
    }

    /// `ESP.onRender3D` + `drawBoundingBox`.
    fn render_boxes(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        self.count = 0;
        // PORT GAP: no screen-space projection (`mode 2d`), no outline shader or
        // glow pass (`mode shader`/`glow`) - those modes draw the box pass here.
        let _ = (
            &self.outline_width,
            &self.glow_multiplier,
            self.target_hitbox.get(),
        );
        for entity in env.entities() {
            let target = self.draw_as_target(env, &entity);
            if !target && self.should_skip(env, &entity) {
                continue;
            }
            self.count += 1;
            let Some(color) = self.color_of(env, &entity) else {
                continue;
            };
            let (mut min, mut max) = entity.bounding_box();
            // `Mth.lerp(tickDelta, xOld, getX()) - getX()`, as Java shifts the box.
            let x = entity.prev_pos.x - entity.pos.x;
            let y = entity.prev_pos.y - entity.pos.y;
            let z = entity.prev_pos.z - entity.pos.z;
            min = Vec3d::new(min.x + x, min.y + y, min.z + z);
            max = Vec3d::new(max.x + x, max.y + y, max.z + z);

            let line_color = if target {
                self.target_hitbox_color.color()
            } else {
                color
            };
            let side_color = color.with_alpha((color.a as f64 * self.fill_opacity.get()) as u8);

            if draw_sides(&self.shape_mode) {
                r.filled_box(min, max, side_color.argb(), true);
            }
            if draw_lines(&self.shape_mode) {
                r.box_outline(min, max, line_color.argb(), 1.5, true);
            }
        }
    }
}

impl Default for ESP {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for ESP {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_render_world(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        self.render_boxes(env, r);
    }

    fn info_string(&self) -> Option<String> {
        Some(self.count.to_string())
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(ESP::new()));
}

/// `EntityUtils.getColorFromDistance`-ish ramp used by `color-mode distance`.
pub(crate) fn distance_color(distance: f64) -> Color {
    let pct = (1.0 - distance / 32.0).clamp(0.0, 1.0);
    Color::RED.lerp(Color::GREEN, pct)
}

/// `EntityType.getCategory()` stand-in: the ids Meteor's five colour buckets use.
fn is_water_mob(kind: &str) -> bool {
    matches!(
        kind,
        "squid"
            | "glow_squid"
            | "cod"
            | "salmon"
            | "tropical_fish"
            | "pufferfish"
            | "dolphin"
            | "axolotl"
            | "tadpole"
            | "guardian"
            | "elder_guardian"
            | "turtle"
    )
}

fn is_monster(kind: &str) -> bool {
    matches!(
        kind,
        "zombie"
            | "husk"
            | "drowned"
            | "skeleton"
            | "stray"
            | "wither_skeleton"
            | "creeper"
            | "spider"
            | "cave_spider"
            | "enderman"
            | "witch"
            | "slime"
            | "magma_cube"
            | "blaze"
            | "ghast"
            | "zombified_piglin"
            | "piglin"
            | "piglin_brute"
            | "hoglin"
            | "zoglin"
            | "phantom"
            | "pillager"
            | "vindicator"
            | "evoker"
            | "ravager"
            | "vex"
            | "shulker"
            | "silverfish"
            | "endermite"
            | "warden"
            | "wither"
            | "ender_dragon"
            | "bogged"
            | "breeze"
    )
}

fn is_animal(kind: &str) -> bool {
    matches!(
        kind,
        "cow"
            | "mooshroom"
            | "pig"
            | "sheep"
            | "chicken"
            | "rabbit"
            | "horse"
            | "donkey"
            | "mule"
            | "llama"
            | "wolf"
            | "cat"
            | "ocelot"
            | "fox"
            | "panda"
            | "polar_bear"
            | "bee"
            | "goat"
            | "frog"
            | "camel"
            | "sniffer"
            | "armadillo"
            | "parrot"
            | "villager"
            | "wandering_trader"
            | "iron_golem"
            | "snow_golem"
    )
}

fn is_ambient(kind: &str) -> bool {
    matches!(kind, "bat" | "allay")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::NullRenderer;

    fn player_entity(id: i32, name: &str, x: f64) -> EntityInfo {
        EntityInfo {
            id,
            kind: "minecraft:player".into(),
            name: Some(name.into()),
            pos: Vec3d::new(x, 64.0, 0.5),
            prev_pos: Vec3d::new(x, 64.0, 0.5),
            health: 20.0,
            max_health: 20.0,
            distance: 4.0,
            alive: true,
            is_player: true,
            ..Default::default()
        }
    }

    #[test]
    fn settings_match_the_java_table() {
        let m = ESP::new();
        for name in [
            "mode",
            "highlight-target",
            "target-hitbox",
            "outline-width",
            "glow-multiplier",
            "ignore-self",
            "shape-mode",
            "fill-opacity",
            "fade-distance",
            "entities",
            "color-mode",
            "show-friend-colors",
            "non-living-entity-color",
            "players-color",
            "animals-color",
            "water-animals-color",
            "monsters-color",
            "ambient-color",
            "misc-color",
            "target-color",
            "target-hitbox-color",
        ] {
            assert!(
                m.base.settings.get(name).is_some(),
                "setting {name} missing"
            );
        }
        assert_eq!(m.base.settings.groups.len(), 2);
        assert_eq!(m.base.settings.groups[0].name, "General");
        assert_eq!(m.base.settings.groups[1].name, "Colors");
        assert!(m.mode.is("shader"), "Java default is Mode.Shader");
        assert_eq!(m.fill_opacity.get(), 0.3);
        assert!(m.entity_enabled(&player_entity(1, "Dev2", 1.0)));
    }

    #[test]
    fn render_draws_a_box_per_player_and_counts_them() {
        let mut m = ESP::new();
        m.mode.set_index(0); // Box
        let mut env = TestEnv::in_world();
        env.entities.push(player_entity(2, "Dev2", 2.5));
        env.entities.push(player_entity(3, "Dev3", 4.5));

        let mut r = NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
        assert_eq!(m.count, 2);
        assert_eq!(m.info_string().as_deref(), Some("2"));
        assert_eq!(r.boxes, 4, "two entities, a fill and an outline each");
    }

    #[test]
    fn the_local_player_is_ignored_when_asked() {
        let mut m = ESP::new();
        m.mode.set_index(0);
        let mut env = TestEnv::in_world();
        env.entities.push(player_entity(1, "Dev", 2.5));
        let mut r = NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
        assert_eq!(m.count, 0, "ignore-self is on by default");
        assert_eq!(r.boxes, 0);

        m.ignore_self.set(false);
        m.on_render_world(&mut env, &mut r);
        assert_eq!(m.count, 1);
        assert!(r.boxes > 0, "the player is drawn once self is allowed");
    }

    #[test]
    fn fade_distance_controls_the_alpha() {
        let mut m = ESP::new();
        m.mode.set_index(0);
        let mut env = TestEnv::in_world();
        // `getFadeAlpha`: squared distance 1 -> sqrt(1)/9 = 0.11 (drawn, faded);
        // squared distance 0.25 -> 0.5/9 = 0.055 -> clamped to 0 by the 0.075 cut.
        env.entities.push(player_entity(2, "Near", 0.5));
        env.entities.push(player_entity(3, "Farther", 3.5));
        let near = env.entities[0].clone();
        let farther = env.entities[1].clone();
        assert_eq!(m.fade_alpha(&env, &near), 0.0, "on top of the camera");
        assert!(m.fade_alpha(&env, &farther) > 0.0, "3 blocks away fades in");

        let mut r = NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
        assert_eq!(m.count, 2, "both are counted");
        assert_eq!(r.boxes, 2, "only the visible one is drawn");
    }
}
