//! `systems.hud.elements` - the HUD elements.
//!
//! The model (box, anchors, settings, serialization) lives in
//! `crate::meteor_core::hud`; this module registers the elements Meteor ships and
//! implements them against `Env`/`Renderer`.

use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::hud::{
    bool_setting, double_setting, int_setting, Alignment, Hud, HudBox, HudElement, HudElementInfo,
    HudGroup,
};
use crate::meteor_core::setting::{
    BoolSet, ColorSet, DoubleSet, EnumSet, IntSet, ListSet, Settings,
};
use crate::meteor_core::util::name_to_title;

pub mod elements;

pub use elements::*;

/// `Hud.init()`'s element list, in the order Java registers them.
pub fn register(hud: &mut Hud) {
    elements::register(hud);
}

/// Shared plumbing: every element keeps `settings` + `box` + an active flag, and
/// the trait needs `&'static HudElementInfo`, which each element supplies.
macro_rules! element_base {
    ($ty:ty, $info:ident) => {
        impl $ty {
            pub fn settings_ref(&self) -> &Settings {
                &self.settings
            }
        }
    };
}
pub(crate) use element_base;

/// One line of text drawn by the HUD: the element every other one copies.
pub struct TextElement {
    pub info: &'static HudElementInfo,
    pub settings: Settings,
    pub box_: HudBox,
    pub active: bool,
    pub text: ListSet,
    pub scale: DoubleSet,
    pub alignment: EnumSet,
    pub shadow: crate::meteor_core::setting::BoolSet,
    pub color: ColorSet,
}

pub struct MeteorTextHud;

pub static METEOR_TEXT_INFO: std::sync::OnceLock<HudElementInfo> = std::sync::OnceLock::new();

pub fn meteor_text_info() -> &'static HudElementInfo {
    METEOR_TEXT_INFO.get_or_init(|| {
        HudElementInfo::new(
            HudGroup("HORIXON"),
            "horixon-text",
            "The Meteor Client watermark.",
            || Box::new(TextElement::meteor_text()),
        )
    })
}

impl TextElement {
    pub fn meteor_text() -> Self {
        let mut settings = Settings::new();
        let g = settings.default_group();
        let text = g.list_setting("text", &["Meteor Client on top"]);
        let scale = g.double_setting("scale", 1.0, 0.5, 3.0, 2);
        let alignment = g.enum_setting("alignment", &Alignment::NAMES, 0);
        let shadow = g.bool_setting("shadow", true);
        let color = g.color_setting(
            "color",
            crate::meteor_core::color::SettingColor::rgb(255, 255, 255),
        );
        Self {
            info: meteor_text_info(),
            settings,
            box_: HudBox::default(),
            active: true,
            text,
            scale,
            alignment,
            shadow,
            color,
        }
    }
}

impl HudElement for TextElement {
    fn info(&self) -> &'static HudElementInfo {
        self.info
    }
    fn settings(&self) -> &Settings {
        &self.settings
    }
    fn settings_mut(&mut self) -> &mut Settings {
        &mut self.settings
    }
    fn box_ref(&self) -> &HudBox {
        &self.box_
    }
    fn box_mut(&mut self) -> &mut HudBox {
        &mut self.box_
    }
    fn is_active(&self) -> bool {
        self.active
    }
    fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    fn render(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        let mut y = 0.0;
        let scale = self.scale.get();
        let color = self.color.color().argb();
        for line in self.text.get() {
            let w = r.text_width(&line, scale);
            let x = self
                .box_ref()
                .align_x(self.width() as f64, w, alignment_of(&self.alignment));
            r.text2d(x, y, &line, color, scale);
            y += r.text_width("A", scale).max(9.0 * scale) + 1.0;
        }
        let _ = env;
        let (w, h) = (self.width() as f64, y);
        self.set_size(w, h);
    }
}

fn alignment_of(e: &EnumSet) -> Alignment {
    match e.index() {
        1 => Alignment::Left,
        2 => Alignment::Center,
        3 => Alignment::Right,
        _ => Alignment::Auto,
    }
}

/// `ActiveModulesHud` - the list of enabled modules and their info strings.
pub struct ActiveModulesElement {
    pub info: &'static HudElementInfo,
    pub settings: Settings,
    pub box_: HudBox,
    pub active: bool,
    pub sort: EnumSet,
    pub show_info: crate::meteor_core::setting::BoolSet,
    pub color: ColorSet,
    pub accent: crate::meteor_core::setting::BoolSet,
    pub separator: crate::meteor_core::setting::BoolSet,
    pub lowercase: crate::meteor_core::setting::BoolSet,
}

pub static ACTIVE_MODULES_INFO: std::sync::OnceLock<HudElementInfo> = std::sync::OnceLock::new();

pub fn active_modules_info() -> &'static HudElementInfo {
    ACTIVE_MODULES_INFO.get_or_init(|| {
        HudElementInfo::new(
            HudGroup("HORIXON"),
            "active-modules",
            "The modules that are currently on.",
            || Box::new(ActiveModulesElement::new()),
        )
    })
}

impl ActiveModulesElement {
    pub fn new() -> Self {
        let mut settings = Settings::new();
        let g = settings.default_group();
        let sort = g.enum_setting("sort", &["length", "alphabetical"], 0);
        let show_info = g.bool_setting("show-info", true);
        let color = g.color_setting(
            "color",
            crate::meteor_core::color::SettingColor::rgb(255, 255, 255),
        );
        let accent = g.bool_setting("accent", true);
        let separator = g.bool_setting("separator", false);
        let lowercase = g.bool_setting("lowercase", false);
        Self {
            info: active_modules_info(),
            settings,
            box_: HudBox::default(),
            active: true,
            sort,
            show_info,
            color,
            accent,
            separator,
            lowercase,
        }
    }
}

impl HudElement for ActiveModulesElement {
    fn info(&self) -> &'static HudElementInfo {
        self.info
    }
    fn settings(&self) -> &Settings {
        &self.settings
    }
    fn settings_mut(&mut self) -> &mut Settings {
        &mut self.settings
    }
    fn box_ref(&self) -> &HudBox {
        &self.box_
    }
    fn box_mut(&mut self) -> &mut HudBox {
        &mut self.box_
    }
    fn is_active(&self) -> bool {
        self.active
    }
    fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    fn render(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        let mut lines: Vec<(String, String)> = env.module_titles();
        if self.sort.index() == 1 {
            lines.sort_by(|a, b| a.0.cmp(&b.0));
        }
        let color = self.color.color().argb();
        let mut y = 0.0;
        for (name, info) in lines {
            let mut text = name_to_title(&name);
            if self.lowercase.get() {
                text = text.to_lowercase();
            }
            let width = r.text_width(&text, 1.0);
            if self.separator.get() {
                r.rect(0.0, y + 4.0, width, 1.0, color);
            }
            r.text2d(0.0, y, &text, color, 1.0);
            if self.show_info.get() && !info.is_empty() {
                r.text2d(width + 4.0, y, &info, color, 1.0);
            }
            y += 10.0;
        }
        self.set_size(self.width() as f64, y);
    }
}

/// A tiny coordinate readout, for the "does the HUD draw at all" check.
pub struct CoordsElement {
    pub info: &'static HudElementInfo,
    pub settings: Settings,
    pub box_: HudBox,
    pub active: bool,
    pub show_dimension: crate::meteor_core::setting::BoolSet,
}

pub static COORDS_INFO: std::sync::OnceLock<HudElementInfo> = std::sync::OnceLock::new();

pub fn coords_info() -> &'static HudElementInfo {
    COORDS_INFO.get_or_init(|| {
        HudElementInfo::new(HudGroup("HORIXON"), "coords", "Your coordinates.", || {
            Box::new(CoordsElement::new())
        })
    })
}

impl CoordsElement {
    pub fn new() -> Self {
        let mut settings = Settings::new();
        let g = settings.default_group();
        let show_dimension = g.bool_setting("show-dimension", true);
        Self {
            info: coords_info(),
            settings,
            box_: HudBox::default(),
            active: true,
            show_dimension,
        }
    }
}

impl HudElement for CoordsElement {
    fn info(&self) -> &'static HudElementInfo {
        self.info
    }
    fn settings(&self) -> &Settings {
        &self.settings
    }
    fn settings_mut(&mut self) -> &mut Settings {
        &mut self.settings
    }
    fn box_ref(&self) -> &HudBox {
        &self.box_
    }
    fn box_mut(&mut self) -> &mut HudBox {
        &mut self.box_
    }
    fn is_active(&self) -> bool {
        self.active
    }
    fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    fn render(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        let Some(player) = env.player() else { return };
        let mut text = format!(
            "{}, {}, {}",
            player.pos.x.floor() as i32,
            player.pos.y.floor() as i32,
            player.pos.z.floor() as i32
        );
        if self.show_dimension.get() {
            let dim = player
                .dimension
                .strip_prefix("minecraft:")
                .unwrap_or(&player.dimension);
            text.push_str(&format!(" ({dim})"));
        }
        r.text2d(0.0, 0.0, &text, 0xFFFF_FFFF, 1.0);
        let w = r.text_width(&text, 1.0);
        self.set_size(w, 9.0);
    }
}

// Re-exports the elements module needs from core, so the category files stay
// short.
pub(crate) mod meteor_core_bool {
    pub use crate::meteor_core::setting::BoolSet;
}

pub(crate) fn hud_int(settings: &Settings, name: &str, default: i32) -> i32 {
    int_setting(settings, name, default)
}

pub(crate) fn hud_double(settings: &Settings, name: &str, default: f64) -> f64 {
    double_setting(settings, name, default)
}

pub(crate) fn hud_bool(settings: &Settings, name: &str, default: bool) -> bool {
    bool_setting(settings, name, default)
}

/// Unused names kept importable so the element files can share one `use` list.
pub(crate) use crate::meteor_core::hud::Alignment as HudAlignment;
pub(crate) type HudInt = IntSet;
