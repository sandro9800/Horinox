//! `systems.hud.elements.ModuleInfosHud` - ON/OFF for a chosen set of modules.

use crate::meteor_core::color::SettingColor;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::hud::{Alignment, HudBox, HudElement, HudElementInfo};
use crate::meteor_core::setting::{BoolSet, ColorSet, EnumSet, ListSet, Settings};
use crate::meteor_core::util::name_to_title;

use super::common::{self, Sg};

pub struct ModuleInfosElement {
    pub info: &'static HudElementInfo,
    pub settings: Settings,
    pub box_: HudBox,
    pub active: bool,
    pub modules: ListSet,
    pub additional_info: BoolSet,
    pub text_shadow: BoolSet,
    pub module_color: ColorSet,
    pub on_color: ColorSet,
    pub off_color: ColorSet,
    pub alignment: EnumSet,
}

/// `ModuleInfosHud.INFO`.
pub static MODULE_INFOS_INFO: std::sync::OnceLock<HudElementInfo> = std::sync::OnceLock::new();

pub fn module_infos_info() -> &'static HudElementInfo {
    MODULE_INFOS_INFO.get_or_init(|| {
        HudElementInfo::new(
            crate::meteor_core::hud::GROUP,
            "module-infos",
            "Displays if selected modules are enabled or disabled.",
            || Box::new(ModuleInfosElement::new()),
        )
    })
}

impl ModuleInfosElement {
    pub fn new() -> Self {
        let mut settings = Settings::new();
        let (modules, additional_info, text_shadow, module_color, on_color, off_color, alignment) = {
            let mut sg = Sg::new(&mut settings);
            // `ModuleListSetting` with KillAura, CrystalAura, AnchorAura, BedAura,
            // Surround.
            let modules = sg.list_setting(
                "General",
                "modules",
                &[
                    "kill-aura",
                    "crystal-aura",
                    "anchor-aura",
                    "bed-aura",
                    "surround",
                ],
                "Which modules to display",
            );
            let additional_info = sg.bool_setting(
                "General",
                "additional-info",
                true,
                "Shows additional info from the module next to the name in the module info list.",
            );
            let text_shadow = sg.bool_setting(
                "General",
                "text-shadow",
                true,
                "Renders shadow behind text.",
            );
            let module_color = sg.color_setting(
                "General",
                "module-color",
                SettingColor::rgb(255, 255, 255),
                "Module color.",
            );
            let on_color = sg.color_setting(
                "General",
                "on-color",
                SettingColor::rgb(25, 225, 25),
                "Color when module is on.",
            );
            let off_color = sg.color_setting(
                "General",
                "off-color",
                SettingColor::rgb(225, 25, 25),
                "Color when module is off.",
            );
            let alignment = sg.enum_setting(
                "General",
                "alignment",
                &Alignment::NAMES,
                0,
                "Horizontal alignment.",
            );
            (
                modules,
                additional_info,
                text_shadow,
                module_color,
                on_color,
                off_color,
                alignment,
            )
        };

        Self {
            info: module_infos_info(),
            settings,
            box_: HudBox::default(),
            active: true,
            modules,
            additional_info,
            text_shadow,
            module_color,
            on_color,
            off_color,
            alignment,
        }
    }

    /// `Modules.get().get(...).getInfoString()`, from `Env::module_titles`.
    fn info_string(&self, env: &dyn Env, module: &str) -> Option<String> {
        env.module_titles()
            .into_iter()
            .find(|(name, _)| name == module)
            .map(|(_, info)| info)
            .filter(|info| !info.is_empty())
    }

    fn tick_impl(&mut self, _env: &mut dyn Env) {}

    fn render_impl(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        let scale = common::hud_text_scale();
        let text_height = common::text_height(scale);
        let entries = self.modules.get();

        let module_color = self.module_color.color().argb();
        if entries.is_empty() {
            let text = "Module Info";
            let width = r.text_width(text, scale);
            r.text2d(0.0, 0.0, text, module_color, scale);
            self.set_size(width, text_height);
            return;
        }

        let on_color = self.on_color.color().argb();
        let off_color = self.off_color.color().argb();
        let space = r.text_width(" ", scale);
        let alignment = common::alignment_of(&self.alignment);

        let mut y = 0.0;
        let mut width: f64 = 0.0;
        let mut height: f64 = 0.0;

        for (i, module) in entries.iter().enumerate() {
            let title = name_to_title(module);
            let active = env.module_active(module);
            let text = if active {
                let extra = if self.additional_info.get() {
                    self.info_string(env, module)
                } else {
                    None
                };
                extra.unwrap_or_else(|| "ON".to_string())
            } else {
                "OFF".to_string()
            };

            let module_width = r.text_width(&title, scale) + space + r.text_width(&text, scale);
            let x = self.align_x(module_width, alignment);
            let text_x = x + r.text_width(&title, scale);
            r.text2d(x, y, &title, module_color, scale);
            r.text2d(
                text_x + space,
                y,
                &text,
                if active { on_color } else { off_color },
                scale,
            );

            y += common::line_step(scale);
            width = width.max(module_width);
            height += text_height;
            if i > 0 {
                height += 2.0;
            }
        }

        self.set_size(width, height);
    }
}

impl Default for ModuleInfosElement {
    fn default() -> Self {
        Self::new()
    }
}

super::hud_element!(ModuleInfosElement);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::NullRenderer;

    #[test]
    fn prints_on_and_off_for_the_configured_modules() {
        let mut element = ModuleInfosElement::new();
        let mut env = TestEnv::in_world();
        let mut r = NullRenderer::default();
        element.render(&mut env, &mut r);
        // Five modules, title plus state each.
        assert_eq!(r.texts, 10);
        assert!(element.width() > 0);
        assert!(element.height() > 0);
    }

    #[test]
    fn falls_back_to_the_placeholder_without_modules() {
        let mut element = ModuleInfosElement::new();
        element.modules.set(Vec::new());
        let mut env = TestEnv::in_world();
        let mut r = NullRenderer::default();
        element.render(&mut env, &mut r);
        assert_eq!(r.texts, 1);
        assert_eq!(element.width(), "Module Info".len() as i32 * 6);
    }
}
