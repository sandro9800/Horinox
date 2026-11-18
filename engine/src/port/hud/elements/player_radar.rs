//! `systems.hud.elements.PlayerRadarHud` - the list of players in range.

use crate::meteor_core::color::SettingColor;
use crate::meteor_core::env::{EntityInfo, Env, Renderer};
use crate::meteor_core::hud::{Alignment, HudBox, HudElement, HudElementInfo};
use crate::meteor_core::setting::{BoolSet, ColorSet, DoubleSet, EnumSet, IntSet, Settings};

use super::common::{self, Sg};

/// `entity.equals(mc.player)`: the UUID when the env fills one in, the name
/// otherwise.
pub fn same_entity(me: &crate::meteor_core::env::PlayerInfo, other: &EntityInfo) -> bool {
    let same_uuid = me.uuid != [0; 16] && other.uuid == me.uuid;
    let same_name = other.name.as_deref() == Some(me.name.as_str());
    same_uuid || same_name
}

pub struct PlayerRadarElement {
    pub info: &'static HudElementInfo,
    pub settings: Settings,
    pub box_: HudBox,
    pub active: bool,
    pub limit: IntSet,
    pub distance: BoolSet,
    pub friends: BoolSet,
    pub shadow: BoolSet,
    pub primary_color: ColorSet,
    pub secondary_color: ColorSet,
    pub alignment: EnumSet,
    pub border: IntSet,
    pub custom_scale: BoolSet,
    pub scale: DoubleSet,
    pub background: BoolSet,
    pub background_color: ColorSet,
}

/// `PlayerRadarHud.INFO`.
pub static PLAYER_RADAR_INFO: std::sync::OnceLock<HudElementInfo> = std::sync::OnceLock::new();

pub fn player_radar_info() -> &'static HudElementInfo {
    PLAYER_RADAR_INFO.get_or_init(|| {
        HudElementInfo::new(
            crate::meteor_core::hud::GROUP,
            "player-radar",
            "Displays players in your visual range.",
            || Box::new(PlayerRadarElement::new()),
        )
    })
}

impl PlayerRadarElement {
    pub fn new() -> Self {
        let mut settings = Settings::new();
        let (
            limit,
            distance,
            friends,
            shadow,
            primary_color,
            secondary_color,
            alignment,
            border,
            custom_scale,
            scale,
            background,
            background_color,
        ) = {
            let mut sg = Sg::new(&mut settings);
            let limit = sg.int_setting(
                "General",
                "limit",
                10,
                1,
                20,
                "The max number of players to show.",
            );
            let distance = sg.bool_setting(
                "General",
                "distance",
                false,
                "Shows the distance to the player next to their name.",
            );
            let friends = sg.bool_setting(
                "General",
                "display-friends",
                true,
                "Whether to show friends or not.",
            );
            let shadow = sg.bool_setting("General", "shadow", true, "Renders shadow behind text.");
            let primary_color = sg.color_setting(
                "General",
                "primary-color",
                SettingColor::rgb(255, 255, 255),
                "Primary color.",
            );
            let secondary_color = sg.color_setting(
                "General",
                "secondary-color",
                SettingColor::rgb(175, 175, 175),
                "Secondary color.",
            );
            let alignment = sg.enum_setting(
                "General",
                "alignment",
                &Alignment::NAMES,
                0,
                "Horizontal alignment.",
            );
            let border = sg.int_setting(
                "General",
                "border",
                0,
                0,
                20,
                "How much space to add around the element.",
            );
            let custom_scale = sg.bool_setting(
                "Scale",
                "custom-scale",
                false,
                "Applies a custom scale to this hud element.",
            );
            let scale = sg.double_setting("Scale", "scale", 1.0, 0.5, 3.0, 2, "Custom scale.");
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
                limit,
                distance,
                friends,
                shadow,
                primary_color,
                secondary_color,
                alignment,
                border,
                custom_scale,
                scale,
                background,
                background_color,
            )
        };

        Self {
            info: player_radar_info(),
            settings,
            box_: HudBox::default(),
            active: true,
            limit,
            distance,
            friends,
            shadow,
            primary_color,
            secondary_color,
            alignment,
            border,
            custom_scale,
            scale,
            background,
            background_color,
        }
    }

    fn get_scale(&self) -> f64 {
        common::scale_of(&self.custom_scale, &self.scale, common::hud_text_scale())
    }

    /// `PlayerRadarHud.setSize`: the border is added on top of the content.
    fn set_content_size(&mut self, width: f64, height: f64) {
        let border = self.border.get() as f64;
        self.box_
            .set_size(width + border * 2.0, height + border * 2.0);
    }

    /// `PlayerRadarHud.alignX`: the alignment ignores the border columns.
    fn align_content_x(&self, width: f64) -> f64 {
        let border = self.border.get() as f64;
        self.box_.align_x(
            self.width() as f64 - border * 2.0,
            width,
            common::alignment_of(&self.alignment),
        )
    }

    /// `getPlayers()`: every player but you, friends filtered, nearest first,
    /// truncated to `limit`.
    fn players(&self, env: &dyn Env) -> Vec<EntityInfo> {
        let Some(me) = env.player() else {
            return Vec::new();
        };
        let mut list: Vec<EntityInfo> = env
            .entities()
            .into_iter()
            .filter(|e| e.is_player)
            // `entity.equals(mc.player)`: compare the UUID when the env fills it in,
            // and the name otherwise (a headless env leaves both zeroed).
            .filter(|e| !same_entity(&me, e))
            .filter(|e| {
                self.friends.get()
                    || e.name
                        .as_deref()
                        .map(|name| !env.is_friend(name))
                        .unwrap_or(true)
            })
            .collect();
        // `Comparator.comparingDouble(e -> e.distanceToSqr(cameraEntity))`.
        list.sort_by(|a, b| {
            a.pos
                .distance_sq(me.pos)
                .partial_cmp(&b.pos.distance_sq(me.pos))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        list.truncate(self.limit.get().max(1) as usize);
        list
    }

    /// The `(name, distance)` text of one row.
    fn row_text(&self, env: &dyn Env, player: &EntityInfo) -> (String, Option<String>) {
        let name = player.name.clone().unwrap_or_else(|| "Unknown".to_string());
        if !self.distance.get() {
            return (name, None);
        }
        let distance = env
            .player()
            .map(|me| me.pos.distance_to(player.pos))
            .unwrap_or(player.distance);
        (name, Some(format!("({}m)", distance.round() as i64)))
    }

    fn tick_impl(&mut self, env: &mut dyn Env) {
        // The players list is rebuilt every frame by `render_impl`, which is where the
        // text can be measured (`tick` gets no `Renderer`).
        let _ = env;
    }

    /// `PlayerRadarHud.tick`: the header plus one row per player, measured with the
    /// real font.
    fn measure(&mut self, env: &dyn Env, r: &dyn Renderer) {
        let scale = self.get_scale();
        let mut width = r.text_width("Players:", scale);
        let mut height = common::text_height(scale);

        for player in self.players(env) {
            let (name, distance) = self.row_text(env, &player);
            let text = match distance {
                Some(d) => format!("{name}{d}"),
                None => name,
            };
            width = width.max(r.text_width(&text, scale));
            height += common::line_step(scale);
        }
        self.set_content_size(width, height);
    }

    fn render_impl(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        let scale = self.get_scale();
        self.measure(env, r);
        let border = self.border.get() as f64;
        let width = self.width() as f64;
        let height = self.height() as f64;

        common::background(
            r,
            width,
            height,
            self.background.get(),
            self.background_color.color().argb(),
        );

        let secondary = self.secondary_color.color().argb();
        let primary = self.primary_color.color().argb();
        let head = "Players:";
        let head_width = r.text_width(head, scale);
        r.text2d(
            border + self.align_content_x(head_width),
            border,
            head,
            secondary,
            scale,
        );

        let mut y = border;
        let space = r.text_width(" ", scale);
        // PORT GAP: `PlayerUtils.getPlayerColor` picks the friend/enemy colours out
        // of `Config`, which `Env` does not expose, so every row uses the primary
        // colour.
        for player in self.players(env) {
            let (name, distance) = self.row_text(env, &player);
            let mut row_width = r.text_width(&name, scale);
            if let Some(distance) = &distance {
                row_width += space + r.text_width(distance, scale);
            }
            let x = border + self.align_content_x(row_width);
            y += common::line_step(scale);
            r.text2d(x, y, &name, primary, scale);
            if let Some(distance) = distance {
                r.text2d(
                    x + r.text_width(&name, scale) + space,
                    y,
                    &distance,
                    secondary,
                    scale,
                );
            }
        }
    }
}

impl Default for PlayerRadarElement {
    fn default() -> Self {
        Self::new()
    }
}

super::hud_element!(PlayerRadarElement);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::NullRenderer;
    use crate::meteor_core::math::Vec3d;

    fn env_with_players() -> TestEnv {
        let mut env = TestEnv::in_world();
        env.entities.push(EntityInfo {
            id: 1,
            kind: "minecraft:player".into(),
            name: Some("Far".into()),
            pos: Vec3d::new(30.0, 64.0, 0.0),
            is_player: true,
            ..Default::default()
        });
        env.entities.push(EntityInfo {
            id: 2,
            kind: "minecraft:player".into(),
            name: Some("Near".into()),
            pos: Vec3d::new(3.0, 64.0, 0.0),
            is_player: true,
            ..Default::default()
        });
        env.entities.push(EntityInfo {
            id: 3,
            kind: "minecraft:zombie".into(),
            pos: Vec3d::new(1.0, 64.0, 0.0),
            is_mob: true,
            ..Default::default()
        });
        env
    }

    #[test]
    fn lists_players_nearest_first() {
        let mut element = PlayerRadarElement::new();
        let mut env = env_with_players();
        let players = element.players(&env);
        assert_eq!(players.len(), 2, "the zombie is not a player");
        assert_eq!(players[0].name.as_deref(), Some("Near"));

        let mut r = NullRenderer::default();
        element.render(&mut env, &mut r);
        assert!(element.width() > 0);
        // Header plus two rows, each `textHeight + 2`.
        assert_eq!(element.height(), 9 + 11 + 11);
    }

    #[test]
    fn renders_the_header_and_rows() {
        let mut element = PlayerRadarElement::new();
        element.distance.set(true);
        let mut env = env_with_players();
        let mut r = NullRenderer::default();
        element.render(&mut env, &mut r);
        assert_eq!(r.texts, 5, "the header, two names and two distances");
    }

    #[test]
    fn limit_truncates_the_list() {
        let element = PlayerRadarElement::new();
        element.limit.set(1);
        let env = env_with_players();
        assert_eq!(element.players(&env).len(), 1);
    }
}
