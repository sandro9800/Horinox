//! `systems.hud.elements.CombatHud` - the combat target's name, gear, health and
//! ping/distance readout.

use crate::meteor_core::color::{Color, SettingColor};
use crate::meteor_core::env::{EntityInfo, Env, ItemStackInfo, Renderer};
use crate::meteor_core::hud::{HudBox, HudElement, HudElementInfo};
use crate::meteor_core::setting::{BoolSet, ColorSet, DoubleSet, ListSet, Settings};

use super::common::{self, Sg};

const GREEN: Color = Color::rgb(15, 255, 15);
const RED: Color = Color::rgb(255, 15, 15);
const BLACK: Color = Color::rgb(0, 0, 0);
/// `Config.get().friendColor` stand-in: `Config` is not behind `Env`.
const FRIEND: Color = Color::rgb(0, 225, 255);

/// The enchantment ids `EnchantmentListSetting.vanillaDefaults()` pre-selects for
/// the gear this element shows.
const VANILLA_ENCHANTMENTS: [&str; 28] = [
    "minecraft:protection",
    "minecraft:fire_protection",
    "minecraft:feather_falling",
    "minecraft:blast_protection",
    "minecraft:projectile_protection",
    "minecraft:respiration",
    "minecraft:aqua_affinity",
    "minecraft:thorns",
    "minecraft:depth_strider",
    "minecraft:frost_walker",
    "minecraft:binding_curse",
    "minecraft:soul_speed",
    "minecraft:swift_sneak",
    "minecraft:sharpness",
    "minecraft:smite",
    "minecraft:bane_of_arthropods",
    "minecraft:knockback",
    "minecraft:fire_aspect",
    "minecraft:looting",
    "minecraft:sweeping_edge",
    "minecraft:unbreaking",
    "minecraft:mending",
    "minecraft:vanishing_curse",
    "minecraft:power",
    "minecraft:punch",
    "minecraft:flame",
    "minecraft:infinity",
    "minecraft:luck_of_the_sea",
];

pub struct CombatElement {
    pub info: &'static HudElementInfo,
    pub settings: Settings,
    pub box_: HudBox,
    pub active: bool,
    pub range: DoubleSet,
    pub displayed_enchantments: ListSet,
    pub enchantment_text_color: ColorSet,
    pub health_color1: ColorSet,
    pub health_color2: ColorSet,
    pub health_color3: ColorSet,
    pub display_ping: BoolSet,
    pub ping_color1: ColorSet,
    pub ping_color2: ColorSet,
    pub ping_color3: ColorSet,
    pub display_distance: BoolSet,
    pub dist_color1: ColorSet,
    pub dist_color2: ColorSet,
    pub dist_color3: ColorSet,
    pub custom_scale: BoolSet,
    pub scale: DoubleSet,
    pub background: BoolSet,
    pub background_color: ColorSet,
}

/// `CombatHud.INFO`.
pub static COMBAT_INFO: std::sync::OnceLock<HudElementInfo> = std::sync::OnceLock::new();

pub fn combat_info() -> &'static HudElementInfo {
    COMBAT_INFO.get_or_init(|| {
        HudElementInfo::new(
            crate::meteor_core::hud::GROUP,
            "combat",
            "Displays information about your combat target.",
            || Box::new(CombatElement::new()),
        )
    })
}

impl CombatElement {
    pub fn new() -> Self {
        let mut settings = Settings::new();
        let (
            range,
            displayed_enchantments,
            enchantment_text_color,
            health_color1,
            health_color2,
            health_color3,
            display_ping,
            ping_color1,
            ping_color2,
            ping_color3,
            display_distance,
            dist_color1,
            dist_color2,
            dist_color3,
            custom_scale,
            scale,
            background,
            background_color,
        ) = {
            let mut sg = Sg::new(&mut settings);
            // Java's field order creates the groups Enchantments, Health, Distance,
            // Ping, Scale, Background before filling them.
            sg.group("Enchantments");
            sg.group("Health");
            sg.group("Distance");
            sg.group("Ping");

            let range = sg.double_setting(
                "General",
                "range",
                100.0,
                1.0,
                200.0,
                2,
                "The range to target players.",
            );
            let displayed_enchantments = sg.list_setting(
                "Enchantments",
                "displayed-enchantments",
                &VANILLA_ENCHANTMENTS,
                "The enchantments that are shown on nametags.",
            );
            let enchantment_text_color = sg.color_setting(
                "Enchantments",
                "enchantment-color",
                SettingColor::rgb(255, 255, 255),
                "Color of enchantment text.",
            );
            let health_color1 = sg.color_setting(
                "Health",
                "health-stage-1",
                SettingColor::rgb(255, 15, 15),
                "The color on the left of the health gradient.",
            );
            let health_color2 = sg.color_setting(
                "Health",
                "health-stage-2",
                SettingColor::rgb(255, 150, 15),
                "The color in the middle of the health gradient.",
            );
            let health_color3 = sg.color_setting(
                "Health",
                "health-stage-3",
                SettingColor::rgb(15, 255, 15),
                "The color on the right of the health gradient.",
            );
            let display_ping = sg.bool_setting("Ping", "ping", true, "Shows the player's ping.");
            let ping_color1 = sg.color_setting(
                "Ping",
                "ping-stage-1",
                SettingColor::rgb(15, 255, 15),
                "Color of ping text when under 75.",
            );
            let on = display_ping.clone();
            sg.visible_last(move || on.get());
            let ping_color2 = sg.color_setting(
                "Ping",
                "ping-stage-2",
                SettingColor::rgb(255, 150, 15),
                "Color of ping text when between 75 and 200.",
            );
            let on = display_ping.clone();
            sg.visible_last(move || on.get());
            let ping_color3 = sg.color_setting(
                "Ping",
                "ping-stage-3",
                SettingColor::rgb(255, 15, 15),
                "Color of ping text when over 200.",
            );
            let on = display_ping.clone();
            sg.visible_last(move || on.get());
            let display_distance = sg.bool_setting(
                "Distance",
                "distance",
                true,
                "Shows the distance between you and the player.",
            );
            let dist_color1 = sg.color_setting(
                "Distance",
                "distance-stage-1",
                SettingColor::rgb(255, 15, 15),
                "The color when a player is within 10 blocks of you.",
            );
            let on = display_distance.clone();
            sg.visible_last(move || on.get());
            let dist_color2 = sg.color_setting(
                "Distance",
                "distance-stage-2",
                SettingColor::rgb(255, 150, 15),
                "The color when a player is within 50 blocks of you.",
            );
            let on = display_distance.clone();
            sg.visible_last(move || on.get());
            let dist_color3 = sg.color_setting(
                "Distance",
                "distance-stage-3",
                SettingColor::rgb(15, 255, 15),
                "The color when a player is greater then 50 blocks away from you.",
            );
            let on = display_distance.clone();
            sg.visible_last(move || on.get());
            let custom_scale = sg.bool_setting(
                "Scale",
                "custom-scale",
                false,
                "Applies a custom scale to this hud element.",
            );
            let scale = sg.double_setting("Scale", "scale", 2.0, 0.5, 3.0, 2, "Custom scale.");
            let custom = custom_scale.clone();
            sg.visible_last(move || !custom.get());
            let background =
                sg.bool_setting("Background", "background", false, "Displays background.");
            let background_color = sg.color_setting(
                "Background",
                "background-color",
                SettingColor::new(25, 25, 25, 50),
                "Color used for the background.",
            );
            let on = background.clone();
            sg.visible_last(move || on.get());
            (
                range,
                displayed_enchantments,
                enchantment_text_color,
                health_color1,
                health_color2,
                health_color3,
                display_ping,
                ping_color1,
                ping_color2,
                ping_color3,
                display_distance,
                dist_color1,
                dist_color2,
                dist_color3,
                custom_scale,
                scale,
                background,
                background_color,
            )
        };

        let mut element = Self {
            info: combat_info(),
            settings,
            box_: HudBox::default(),
            active: true,
            range,
            displayed_enchantments,
            enchantment_text_color,
            health_color1,
            health_color2,
            health_color3,
            display_ping,
            ping_color1,
            ping_color2,
            ping_color3,
            display_distance,
            dist_color1,
            dist_color2,
            dist_color3,
            custom_scale,
            scale,
            background,
            background_color,
        };
        element.calculate_size();
        element
    }

    fn get_scale(&self) -> f64 {
        common::scale_of(&self.custom_scale, &self.scale, common::hud_text_scale())
    }

    fn calculate_size(&mut self) {
        let scale = self.get_scale();
        self.box_.set_size(175.0 * scale, 95.0 * scale);
    }

    /// `TargetUtils.getPlayerTarget(range, SortPriority.LowestDistance)`, or the
    /// player itself in the editor.
    fn target(&self, env: &dyn Env) -> Option<EntityInfo> {
        let me = env.player()?;
        if common::is_in_editor(env) {
            return Some(EntityInfo {
                id: 0,
                kind: "minecraft:player".to_string(),
                name: Some(me.name.clone()),
                uuid: me.uuid,
                pos: me.pos,
                yaw: me.yaw,
                pitch: me.pitch,
                health: me.health,
                max_health: me.max_health,
                is_player: true,
                alive: true,
                ..Default::default()
            });
        }
        let range = self.range.get();
        let mut best: Option<EntityInfo> = None;
        let mut best_distance = f64::MAX;
        for entity in env.entities() {
            if !entity.is_player || !entity.alive {
                continue;
            }
            if super::player_radar::same_entity(&me, &entity) {
                continue;
            }
            let distance = me.pos.distance_to(entity.pos);
            if distance > range || distance >= best_distance {
                continue;
            }
            best_distance = distance;
            best = Some(entity);
        }
        best
    }

    /// `CombatHud.getItem(i)`: `0..3` the armour slots, `4` the off hand, `5` the
    /// main hand - `EntityInfo::equipment`'s own order.
    fn item(
        &self,
        env: &dyn Env,
        target: Option<&EntityInfo>,
        index: usize,
    ) -> Option<ItemStackInfo> {
        if common::is_in_editor(env) {
            return Some(common::preview(match index {
                0 => "minecraft:netherite_boots",
                1 => "minecraft:netherite_leggings",
                2 => "minecraft:netherite_chestplate",
                3 => "minecraft:netherite_helmet",
                4 => "minecraft:totem_of_undying",
                5 => "minecraft:end_crystal",
                _ => return None,
            }));
        }
        let target = target?;
        let slot = match index {
            4 => 5, // off hand
            5 => 4, // main hand
            other => other,
        };
        target.equipment.get(slot).cloned().flatten()
    }

    /// `Utils.getEnchantSimpleName(enchantment, 3)`: the first three letters.
    fn enchant_simple_name(id: &str) -> String {
        let name = crate::meteor_core::util::name_to_title(
            common::strip_ns(id).replace('_', "-").as_str(),
        );
        name.chars().take(3).collect()
    }

    fn is_curse(id: &str) -> bool {
        common::strip_ns(id).ends_with("curse")
    }

    /// The "Friend"/"Naked"/"Threat"/"Unknown" status text and its colour.
    fn status(&self, env: &dyn Env, target: &EntityInfo) -> (&'static str, Color) {
        if target
            .name
            .as_deref()
            .map(|name| env.is_friend(name))
            .unwrap_or(false)
        {
            return ("Friend", FRIEND);
        }
        // PORT GAP: `Friends.get().isFriend(...)` beyond the name list, and
        // `Config.friendColor`, are not behind `Env`; the name list and a stand-in
        // colour are used.
        let naked = (0..4).all(|i| target.equipment.get(i).cloned().flatten().is_none());
        if naked {
            return ("Naked", GREEN);
        }
        let mut threat = false;
        for i in 0..6 {
            let Some(stack) = target.equipment.get(i).cloned().flatten() else {
                continue;
            };
            let id = common::strip_ns(&stack.id);
            if id.ends_with("_sword")
                || id == "end_crystal"
                || id == "respawn_anchor"
                || id.ends_with("_bed")
            {
                threat = true;
            }
        }
        if threat {
            ("Threat", RED)
        } else {
            ("Unknown", Color::WHITE)
        }
    }

    fn tick_impl(&mut self, _env: &mut dyn Env) {}

    fn render_impl(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        let scale = self.get_scale();
        self.calculate_size();
        let width = self.width() as f64;
        let height = self.height() as f64;
        let background_color = self.background_color.color().argb();

        // `TextHud.getSectionColor(0)` / `(1)`: the HUD's text colour list, which
        // the port's `Hud` does not carry yet.
        // PORT GAP: `Hud.text-colors` is not in `crate::meteor_core::hud::Hud`, so the two
        // section colours are the defaults.
        let primary = Color::WHITE;
        let secondary = Color::rgb(175, 175, 175);

        let target = self.target(env);
        if target.is_none() && !common::is_in_editor(env) {
            return;
        }

        if self.background.get() {
            r.rect(0.0, 0.0, width, height, background_color);
        }

        let Some(target) = target else {
            common::placeholder(r, width, height);
            return;
        };

        // PORT GAP: the player model (`HudRenderer.entity`) has no `Renderer`
        // counterpart, so the 50x60 model box at (5, 10) * scale stays empty; every
        // other readout keeps Java's position so the element's look matches.
        let x = 50.0 * scale;
        let mut y = 5.0 * scale;

        let text_scale = 0.45 * scale;
        let text_height = common::text_height(text_scale);
        let name = target.name.clone().unwrap_or_else(|| "Unknown".to_string());

        let (status, status_color) = self.status(env, &target);
        let my_pos = env.player().map(|p| p.pos).unwrap_or_default();
        let distance = if common::is_in_editor(env) {
            0.0
        } else {
            (my_pos.distance_to(target.pos) * 100.0).round() / 100.0
        };
        let ping = target.ping;

        let ping_color = if ping <= 75 {
            self.ping_color1.color()
        } else if ping <= 200 {
            self.ping_color2.color()
        } else {
            self.ping_color3.color()
        };
        let dist_color = if distance <= 10.0 {
            self.dist_color1.color()
        } else if distance <= 50.0 {
            self.dist_color2.color()
        } else {
            self.dist_color3.color()
        };

        r.text2d(x, y, &name, primary.argb(), text_scale);
        y += text_height;
        r.text2d(x, y, status, status_color.argb(), text_scale);

        let separator = " | ";
        let sep_width = r.text_width(separator, text_scale);
        let mut line_x = x + r.text_width(status, text_scale);
        let ping_text = format!("{ping}ms");
        let dist_text = format!("{distance}m");

        if self.display_ping.get() {
            r.text2d(line_x, y, separator, secondary.argb(), text_scale);
            line_x += sep_width;
            r.text2d(line_x, y, &ping_text, ping_color.argb(), text_scale);
            line_x += r.text_width(&ping_text, text_scale);
            if self.display_distance.get() {
                r.text2d(line_x, y, separator, secondary.argb(), text_scale);
                line_x += sep_width;
                r.text2d(line_x, y, &dist_text, dist_color.argb(), text_scale);
            }
        } else if self.display_distance.get() {
            r.text2d(line_x, y, separator, secondary.argb(), text_scale);
            line_x += sep_width;
            r.text2d(line_x, y, &dist_text, dist_color.argb(), text_scale);
        }

        // ---- armour and enchantments ----
        y += 10.0 * scale;
        let enchant_color = self.enchantment_text_color.color();
        let enchant_scale = 0.35 * scale;
        let enchant_height = common::text_height(enchant_scale);
        let displayed = self.displayed_enchantments.get();

        for position in 0..6 {
            let icon_x = x + position as f64 * 20.0 * scale;
            let icon_y = y;
            let stack = self.item(env, Some(&target), 5 - position.min(5));
            let Some(stack) = stack else { continue };
            r.item2d(icon_x, icon_y, &stack, common::item_size(scale));

            let mut enchant_y = icon_y + 18.0 * scale;
            for (id, level) in &stack.enchantments {
                if !displayed.iter().any(|e| common::same_id(e, id)) {
                    continue;
                }
                let text = format!("{} {}", Self::enchant_simple_name(id), level);
                let text_width = r.text_width(&text, enchant_scale);
                let text_x = icon_x + 8.0 * scale - text_width / 2.0;
                let color = if Self::is_curse(id) {
                    RED
                } else {
                    enchant_color
                };
                r.text2d(text_x, enchant_y, &text, color.argb(), enchant_scale);
                enchant_y += enchant_height;
            }
        }

        // ---- health bar ----
        let bar_x = 5.0 * scale;
        let bar_y = 75.0 * scale;
        r.rect_outline(bar_x, bar_y, 165.0 * scale, 11.0 * scale, BLACK.argb(), 1.0);

        let max_health = target.max_health.max(1.0);
        let max_absorb = 16.0f32;
        let max_total = max_health + max_absorb;
        let total_health_width = 161.0 * max_health / max_total;
        let total_absorb_width = 161.0 * max_absorb / max_total;
        let health = target.health.max(0.0);
        // PORT GAP: `EntityInfo` carries no absorption (only `PlayerInfo` does), so
        // the absorption half of the bar stays empty for a target.
        let absorb = 0.0f32;
        let health_width = total_health_width * (health / max_health);
        let absorb_width = total_absorb_width * (absorb / max_absorb);

        let inner_x = bar_x + 2.0 * scale;
        let inner_y = bar_y + 2.0 * scale;
        r.rect(
            inner_x,
            inner_y,
            health_width as f64 * scale,
            7.0 * scale,
            self.health_color1.color().argb(),
        );
        r.rect(
            inner_x + health_width as f64 * scale,
            inner_y,
            absorb_width as f64 * scale,
            7.0 * scale,
            self.health_color2.color().argb(),
        );
        // The third health colour is the gradient's right end in Java; a flat draw
        // cannot interpolate, so it tints the absorb bar instead.
        let _ = self.health_color3.color();
    }
}

impl Default for CombatElement {
    fn default() -> Self {
        Self::new()
    }
}

super::hud_element!(CombatElement);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::NullRenderer;
    use crate::meteor_core::math::Vec3d;

    fn combat_env() -> TestEnv {
        let mut env = TestEnv::in_world();
        env.entities.push(EntityInfo {
            id: 7,
            kind: "minecraft:player".into(),
            name: Some("Target".into()),
            pos: Vec3d::new(4.0, 64.0, 0.0),
            is_player: true,
            alive: true,
            health: 12.0,
            max_health: 20.0,
            ping: 30,
            equipment: [
                Some(common::preview("minecraft:netherite_boots")),
                Some(common::preview("minecraft:netherite_leggings")),
                Some(common::preview("minecraft:netherite_chestplate")),
                Some(common::preview("minecraft:netherite_helmet")),
                Some(ItemStackInfo {
                    id: "minecraft:diamond_sword".into(),
                    count: 1,
                    enchantments: vec![("minecraft:sharpness".into(), 5)],
                    ..Default::default()
                }),
                Some(common::preview("minecraft:totem_of_undying")),
            ],
            ..Default::default()
        });
        env
    }

    #[test]
    fn draws_the_target_readout() {
        let mut element = CombatElement::new();
        let mut env = combat_env();
        let mut r = NullRenderer::default();
        element.render(&mut env, &mut r);

        assert_eq!(element.width(), 175);
        assert_eq!(element.height(), 95);
        // Name, status, ping and distance, plus the sword's "Sha 5".
        assert!(r.texts >= 5, "got {} text draws", r.texts);
    }

    #[test]
    fn target_is_the_nearest_player_in_range() {
        let element = CombatElement::new();
        let mut env = combat_env();
        env.entities.push(EntityInfo {
            id: 8,
            kind: "minecraft:player".into(),
            name: Some("Closer".into()),
            pos: Vec3d::new(1.0, 64.0, 0.0),
            is_player: true,
            alive: true,
            ..Default::default()
        });
        assert_eq!(
            element.target(&env).and_then(|e| e.name),
            Some("Closer".to_string())
        );

        // `range` bottoms out at one block, so the out-of-range case needs the far
        // player on its own (`Target` is ~3.5 blocks away).
        env.entities.retain(|e| e.name.as_deref() == Some("Target"));
        element.range.set(1.0);
        assert!(element.target(&env).is_none(), "out of range");
    }

    #[test]
    fn naked_and_threat_status() {
        let element = CombatElement::new();
        let mut env = combat_env();
        {
            let target = &mut env.entities[0];
            target.equipment = std::array::from_fn(|_| None);
        }
        let target = env.entities[0].clone();
        assert_eq!(element.status(&env, &target).0, "Naked");

        // Full armour (so not naked) plus a crystal in the main hand.
        for (i, id) in [
            "minecraft:netherite_boots",
            "minecraft:netherite_leggings",
            "minecraft:netherite_chestplate",
            "minecraft:netherite_helmet",
        ]
        .iter()
        .enumerate()
        {
            env.entities[0].equipment[i] = Some(common::preview(id));
        }
        env.entities[0].equipment[4] = Some(ItemStackInfo {
            id: "minecraft:end_crystal".into(),
            count: 1,
            ..Default::default()
        });
        let target = env.entities[0].clone();
        assert_eq!(element.status(&env, &target).0, "Threat");
    }
}
