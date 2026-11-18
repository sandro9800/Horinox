//! `systems.hud.elements.keyboard.KeyboardHud` - the on-screen keyboard.
//!
//! Ported from the Java original including its layout maths: `LayoutContext` (a
//! 35px key unit, `spacing * 2` gaps and a 15px gap under the function row) and
//! `KeyDimensions` (the unit widths of the wide keys) both live in this file, and
//! the movement/click/action/hotbar presets use Minecraft's default bindings.

use crate::meteor_core::color::{Color, SettingColor};
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::hud::{HudBox, HudElement, HudElementInfo};
use crate::meteor_core::setting::{BoolSet, ColorSet, DoubleSet, EnumSet, ListSet, Settings};

use super::common::{self, Sg};

/// `KeyboardHud.Preset`.
const PRESETS: [&str; 6] = [
    "movement", "clicks", "actions", "hotbar", "keyboard", "custom",
];
/// `KeyboardHud.KeyboardLayout`.
const KEYBOARD_LAYOUTS: [&str; 2] = ["ansi", "iso"];
/// `KeyboardHud.Alignment` - the key text's alignment, not the HUD's.
const ALIGNMENTS: [&str; 3] = ["left", "center", "right"];

// GLFW key codes, the same numbers Meteor's `Keybind` uses.
const KEY_SPACE: i32 = 32;
const KEY_ESCAPE: i32 = 256;
const KEY_ENTER: i32 = 257;
const KEY_TAB: i32 = 258;
const KEY_BACKSPACE: i32 = 259;
const KEY_INSERT: i32 = 260;
const KEY_DELETE: i32 = 261;
const KEY_RIGHT: i32 = 262;
const KEY_LEFT: i32 = 263;
const KEY_DOWN: i32 = 264;
const KEY_UP: i32 = 265;
const KEY_PAGE_UP: i32 = 266;
const KEY_PAGE_DOWN: i32 = 267;
const KEY_HOME: i32 = 268;
const KEY_END: i32 = 269;
const KEY_CAPS_LOCK: i32 = 280;
const KEY_SCROLL_LOCK: i32 = 281;
const KEY_PRINT_SCREEN: i32 = 283;
const KEY_PAUSE: i32 = 284;
const KEY_F1: i32 = 290;
const KEY_F5: i32 = 294;
const KEY_F9: i32 = 298;
const KEY_LEFT_SHIFT: i32 = 340;
const KEY_LEFT_CONTROL: i32 = 341;
const KEY_LEFT_ALT: i32 = 342;
const KEY_LEFT_SUPER: i32 = 343;
const KEY_RIGHT_SHIFT: i32 = 344;
const KEY_RIGHT_CONTROL: i32 = 345;
const KEY_RIGHT_ALT: i32 = 346;
const KEY_RIGHT_SUPER: i32 = 347;
const KEY_MENU: i32 = 348;
const KEY_GRAVE: i32 = 96;
const KEY_MINUS: i32 = 45;
const KEY_EQUALS: i32 = 61;
const KEY_LEFT_BRACKET: i32 = 91;
const KEY_BACKSLASH: i32 = 92;
const KEY_RIGHT_BRACKET: i32 = 93;
const KEY_SEMICOLON: i32 = 59;
const KEY_APOSTROPHE: i32 = 39;
const KEY_COMMA: i32 = 44;
const KEY_PERIOD: i32 = 46;
const KEY_SLASH: i32 = 47;
const KEY_A: i32 = 65;
const KEY_Q: i32 = 81;
const KEY_Z: i32 = 90;

/// `KeyDimensions`: the unit width of a key.
const DIM_STANDARD: f64 = 1.0;
const DIM_TAB: f64 = 1.5;
const DIM_CAPS_LOCK: f64 = 1.75;
const DIM_ENTER_ANSI: f64 = 2.25;
const DIM_LEFT_SHIFT_ANSI: f64 = 2.25;
const DIM_RIGHT_SHIFT: f64 = 2.75;
const DIM_BACKSPACE: f64 = 2.0;
const DIM_LEFT_SHIFT_ISO: f64 = 1.25;
const DIM_ENTER_ISO_WIDTH: f64 = 1.25;
const DIM_ENTER_ISO_HEIGHT: f64 = 2.0;
const DIM_CTRL: f64 = 1.25;
const DIM_ALT: f64 = 1.25;
const DIM_GUI: f64 = 1.25;
const DIM_MENU: f64 = 1.25;
const DIM_SPACEBAR: f64 = 6.25;
const DIM_2U: f64 = 2.0;

/// `KeyboardHud.getShortName`: the label printed on a key cap.
fn key_label(code: i32) -> String {
    match code {
        32 => "Space".to_string(),
        39 => "'".to_string(),
        44 => ",".to_string(),
        45 => "-".to_string(),
        46 => ".".to_string(),
        47 => "/".to_string(),
        48..=57 => ((b'0' + (code as u8 - 48)) as char).to_string(),
        59 => ";".to_string(),
        61 => "=".to_string(),
        65..=90 => ((code as u8) as char).to_string(),
        91 => "[".to_string(),
        92 => "\\".to_string(),
        93 => "]".to_string(),
        96 => "`".to_string(),
        256 => "Esc".to_string(),
        257 => "Ent".to_string(),
        258 => "Tab".to_string(),
        259 => "BS".to_string(),
        260 => "Ins".to_string(),
        261 => "Del".to_string(),
        262 => "Rt".to_string(),
        263 => "Lt".to_string(),
        264 => "Dn".to_string(),
        265 => "Up".to_string(),
        266 => "PgUp".to_string(),
        267 => "PgDn".to_string(),
        268 => "Home".to_string(),
        269 => "End".to_string(),
        280 => "Caps".to_string(),
        281 => "ScrL".to_string(),
        282 => "NumL".to_string(),
        283 => "PrtS".to_string(),
        284 => "Paus".to_string(),
        290..=301 => format!("F{}", code - 289),
        340 => "LSh".to_string(),
        341 => "LCtrl".to_string(),
        342 => "LAlt".to_string(),
        343 => "LSup".to_string(),
        344 => "RSh".to_string(),
        345 => "RCtrl".to_string(),
        346 => "RAlt".to_string(),
        347 => "RSup".to_string(),
        348 => "Menu".to_string(),
        _ => "?".to_string(),
    }
}

/// The key code a `custom-keys` entry names: `lmb`/`rmb`/`mmb`, a single
/// character, or any label `key_label` prints.
fn code_for_name(name: &str) -> Option<i32> {
    let needle = name.trim().to_ascii_lowercase();
    match needle.as_str() {
        "lmb" => return Some(-1),
        "rmb" => return Some(-2),
        "mmb" => return Some(-3),
        _ => {}
    }
    if needle.chars().count() == 1 {
        let c = needle.chars().next()?;
        if c.is_ascii_alphabetic() {
            return Some(KEY_A + (c.to_ascii_uppercase() as i32 - 'A' as i32));
        }
        if c.is_ascii_digit() {
            return Some(48 + (c as i32 - '0' as i32));
        }
    }
    (0..=348).find(|code| key_label(*code).to_ascii_lowercase() == needle)
}

/// `LayoutContext`: units to pixels, with the gaps between keys counted in.
#[derive(Clone, Copy, Debug)]
struct Layout {
    key_unit: f64,
    key_gap: f64,
    step: f64,
    function_row_gap: f64,
}

impl Layout {
    fn new(spacing: f64) -> Self {
        let key_unit = 35.0;
        let key_gap = spacing * 2.0;
        Self {
            key_unit,
            key_gap,
            step: key_unit + key_gap,
            function_row_gap: 15.0,
        }
    }

    /// `ux(units)`: a horizontal offset.
    fn ux(&self, units: f64) -> f64 {
        units * self.step
    }

    /// `y(rows)`: a vertical offset.
    fn y(&self, rows: f64) -> f64 {
        rows * self.step
    }

    /// `uy(rows)`: `y` plus the gap under the function row.
    fn uy(&self, rows: f64) -> f64 {
        rows * self.step
            + if rows > 0.0 {
                self.function_row_gap
            } else {
                0.0
            }
    }

    /// `px(dimension)`: a wide key's width; `n` units span `n - 1` gaps.
    fn px(&self, units: f64) -> f64 {
        units * self.key_unit + (units - 1.0) * self.key_gap
    }

    fn key(&self, code: i32, x: f64, y: f64) -> Key {
        Key::new(code, x, y, self.px(DIM_STANDARD), self.px(DIM_STANDARD))
    }

    fn key_wide(&self, code: i32, x: f64, y: f64, units: f64) -> Key {
        Key::new(code, x, y, self.px(units), self.px(DIM_STANDARD))
    }
}

/// One key cap: `KeyboardHud.Key`.
#[derive(Clone, Debug, Default)]
pub struct Key {
    pub name: String,
    /// A GLFW key code, when the cap shows a keyboard key.
    pub key: Option<i32>,
    /// A mouse button, when the cap shows `LMB`/`RMB`.
    pub mouse: Option<i32>,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub show_cps: bool,
    /// The ISO `Enter`, whose top bar reaches left of the stem.
    pub iso_top_bar_start: Option<f64>,
    pub pressed: bool,
    prev_pressed: bool,
    /// Fade progress, `0` released and `1` pressed (`color-fade`).
    pub delta: f32,
    /// Click timestamps, for the rolling CPS count.
    clicks: Vec<f64>,
}

impl Key {
    fn new(code: i32, x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            name: String::new(),
            key: Some(code),
            mouse: None,
            x,
            y,
            width,
            height,
            show_cps: false,
            iso_top_bar_start: None,
            pressed: false,
            prev_pressed: false,
            delta: 0.0,
            clicks: Vec::new(),
        }
    }

    fn mouse_button(button: i32, name: &str, x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            name: name.to_string(),
            key: None,
            mouse: Some(button),
            ..Self::new(0, x, y, width, height)
        }
    }

    fn named(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    /// `Key.getName`: the explicit label, else the key's short name.
    pub fn label(&self) -> String {
        if !self.name.is_empty() {
            return self.name.clone();
        }
        match (self.key, self.mouse) {
            (Some(code), _) => key_label(code),
            (_, Some(button)) => match button {
                0 => "LMB".to_string(),
                1 => "RMB".to_string(),
                2 => "MMB".to_string(),
                other => format!("M{other}"),
            },
            _ => "?".to_string(),
        }
    }

    /// `Key.isNativelyPressed`, read from `Env` instead of the input event bus.
    fn is_down(&self, env: &dyn Env) -> bool {
        match (self.key, self.mouse) {
            (Some(code), _) if code >= 0 => env.key_down(code),
            (_, Some(button)) => env.mouse_down(button),
            _ => false,
        }
    }

    /// The rolling one-second click count (`RollingCps`).
    pub fn cps(&self, now: f64) -> usize {
        self.clicks.iter().filter(|t| **t + 1.0 >= now).count()
    }

    fn update(&mut self, pressed: bool, now: f64) {
        if pressed && !self.prev_pressed && self.show_cps {
            self.clicks.push(now);
            self.clicks.retain(|t| *t + 1.0 >= now);
        }
        self.pressed = pressed;
        self.prev_pressed = pressed;
    }
}

pub struct KeyboardElement {
    pub info: &'static HudElementInfo,
    pub settings: Settings,
    pub box_: HudBox,
    pub active: bool,
    pub preset: EnumSet,
    pub custom_keys: ListSet,
    pub scale: DoubleSet,
    pub spacing: DoubleSet,
    pub keyboard_layout: EnumSet,
    pub show_cps: BoolSet,
    pub alignment: EnumSet,
    pub pressed_color: ColorSet,
    pub unpressed_color: ColorSet,
    pub color_fade: BoolSet,
    pub fade_time: DoubleSet,
    pub text_color: ColorSet,
    pub border: BoolSet,
    pub border_color: ColorSet,
    pub border_width: DoubleSet,
    pub opacity: DoubleSet,
    pub background: BoolSet,
    pub background_color: ColorSet,
    pub keys: Vec<Key>,
    pub min_x: f64,
    pub min_y: f64,
    /// `env.now()` of the previous frame, for the fade step and the CPS window.
    last_render_time: Option<f64>,
}

/// `KeyboardHud.INFO`.
pub static KEYBOARD_INFO: std::sync::OnceLock<HudElementInfo> = std::sync::OnceLock::new();

pub fn keyboard_info() -> &'static HudElementInfo {
    KEYBOARD_INFO.get_or_init(|| {
        HudElementInfo::new(
            crate::meteor_core::hud::GROUP,
            "keyboard",
            "Displays pressed keys.",
            || Box::new(KeyboardElement::new()),
        )
    })
}

impl KeyboardElement {
    pub fn new() -> Self {
        let mut settings = Settings::new();
        let (
            preset,
            custom_keys,
            scale,
            spacing,
            keyboard_layout,
            show_cps,
            alignment,
            pressed_color,
            unpressed_color,
            color_fade,
            fade_time,
            text_color,
            border,
            border_color,
            border_width,
            opacity,
            background,
            background_color,
        ) = {
            let mut sg = Sg::new(&mut settings);
            sg.group("Color");
            sg.group("Border");
            let preset =
                sg.enum_setting("General", "preset", &PRESETS, 0, "Which keys to display.");
            // `CustomKeyListSetting`: the port keeps the keys as text ids, since a
            // key's size and position are per-key settings of their own.
            let custom_keys = sg.list_setting(
                "General",
                "custom-keys",
                &["space"],
                "Configure the custom keys display.",
            );
            let p = preset.clone();
            sg.visible_last(move || p.is("custom"));
            let scale = sg.double_setting(
                "General",
                "scale",
                1.5,
                0.5,
                5.0,
                1,
                "Scale of the keyboard.",
            );
            let spacing = sg.double_setting(
                "General",
                "spacing",
                1.0,
                0.0,
                10.0,
                1,
                "Spacing between keys.",
            );
            let p = preset.clone();
            sg.visible_last(move || !p.is("custom"));
            let keyboard_layout = sg.enum_setting(
                "General",
                "keyboard-layout",
                &KEYBOARD_LAYOUTS,
                0,
                "Physical keyboard layout (ANSI or ISO).",
            );
            let p = preset.clone();
            sg.visible_last(move || p.is("keyboard"));
            // Java spells this one `.name("Show CPS")`; the port keeps the kebab-case
            // name every other setting uses.
            let show_cps = sg.bool_setting(
                "General",
                "show-cps",
                false,
                "Shows clicks per second on keys.",
            );
            let p = preset.clone();
            sg.visible_last(move || !p.is("custom"));
            let alignment = sg.enum_setting(
                "General",
                "alignment",
                &ALIGNMENTS,
                1,
                "Horizontal alignment of the text.",
            );
            let pressed_color = sg.color_setting(
                "Color",
                "pressed-color",
                SettingColor::new(200, 200, 200, 100),
                "Color of pressed keys.",
            );
            let unpressed_color = sg.color_setting(
                "Color",
                "unpressed-color",
                SettingColor::new(0, 0, 0, 100),
                "Color of unpressed keys.",
            );
            let color_fade = sg.bool_setting(
                "Color",
                "color-fade",
                false,
                "Whether to fade the key color when pressing/unpressing.",
            );
            let fade_time = sg.double_setting(
                "Color",
                "fade-time",
                0.1,
                0.01,
                0.5,
                2,
                "How long to fade the color for, in seconds.",
            );
            let fade = color_fade.clone();
            sg.visible_last(move || fade.get());
            let text_color = sg.color_setting(
                "Color",
                "text-color",
                SettingColor::rgb(255, 255, 255),
                "Color of the key name.",
            );
            let border = sg.bool_setting("Border", "border", false, "Draw a border around keys.");
            let border_color = sg.color_setting(
                "Border",
                "border-color",
                SettingColor::new(255, 255, 255, 200),
                "Color of the key border.",
            );
            let on = border.clone();
            sg.visible_last(move || on.get());
            let border_width = sg.double_setting(
                "Border",
                "border-width",
                1.0,
                0.5,
                5.0,
                2,
                "Width of the key border.",
            );
            let on = border.clone();
            sg.visible_last(move || on.get());
            let opacity = sg.double_setting(
                "Color",
                "opacity",
                1.0,
                0.0,
                1.0,
                2,
                "Opacity of the whole element.",
            );
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
                preset,
                custom_keys,
                scale,
                spacing,
                keyboard_layout,
                show_cps,
                alignment,
                pressed_color,
                unpressed_color,
                color_fade,
                fade_time,
                text_color,
                border,
                border_color,
                border_width,
                opacity,
                background,
                background_color,
            )
        };

        let mut element = Self {
            info: keyboard_info(),
            settings,
            box_: HudBox::default(),
            active: true,
            preset,
            custom_keys,
            scale,
            spacing,
            keyboard_layout,
            show_cps,
            alignment,
            pressed_color,
            unpressed_color,
            color_fade,
            fade_time,
            text_color,
            border,
            border_color,
            border_width,
            opacity,
            background,
            background_color,
            keys: Vec::new(),
            min_x: 0.0,
            min_y: 0.0,
            last_render_time: None,
        };
        // `onPresetChanged(preset.get())` in the constructor.
        element.rebuild();
        element
    }

    // ---- layout ----------------------------------------------------------

    /// `KeyboardHud.onPresetChanged`.
    pub fn rebuild(&mut self) {
        self.keys.clear();
        let layout = Layout::new(self.spacing.get());

        match self.preset.get() {
            "clicks" => {
                self.add(Key::mouse_button(
                    0,
                    "LMB",
                    0.0,
                    0.0,
                    layout.px(1.0),
                    layout.px(1.0),
                ));
                self.add(Key::mouse_button(
                    1,
                    "RMB",
                    layout.ux(1.0),
                    0.0,
                    layout.px(1.0),
                    layout.px(1.0),
                ));
            }
            "actions" => {
                // Minecraft's defaults: drop Q, swap off hand F, inventory E.
                self.add(layout.key(KEY_Q, 0.0, 0.0));
                self.add(layout.key(70, layout.ux(1.0), 0.0)); // F
                self.add(layout.key(69, layout.ux(2.0), 0.0)); // E
            }
            "hotbar" => {
                for i in 0..9 {
                    self.add(layout.key(49 + i, layout.ux(i as f64), 0.0));
                }
            }
            "keyboard" => {
                if self.keyboard_layout.is("iso") {
                    self.build_iso(&layout);
                } else {
                    self.build_ansi(&layout);
                }
            }
            "custom" => {
                let entries = self.custom_keys.get();
                // PORT GAP: a custom key's own size/position settings
                // (`CustomKeySettingScreen`) have no setting-kind counterpart, so the
                // port lays the ids out in a row at the size `Key()` defaults to.
                let mut x = 0.0;
                for entry in entries {
                    let Some(code) = code_for_name(&entry) else {
                        continue;
                    };
                    let key = if code < 0 {
                        Key::mouse_button(-code - 1, &entry, x, 0.0, 60.0, 40.0)
                    } else {
                        Key::new(code, x, 0.0, 60.0, 40.0)
                    };
                    x += 70.0;
                    self.add(key);
                }
            }
            _ => {
                // `Movement`: W A S D, shift and jump in Minecraft's default binds.
                self.add(layout.key(87, layout.ux(1.0), 0.0)); // W
                self.add(layout.key(KEY_A, 0.0, layout.y(1.0)));
                self.add(layout.key(83, layout.ux(1.0), layout.y(1.0))); // S
                self.add(layout.key(68, layout.ux(2.0), layout.y(1.0))); // D
                self.add(layout.key(KEY_LEFT_SHIFT, 0.0, layout.y(2.0)));
                self.add(layout.key_wide(KEY_SPACE, layout.ux(1.0), layout.y(2.0), DIM_2U));
            }
        }

        self.calculate_size();
    }

    fn add(&mut self, mut key: Key) {
        key.show_cps = self.show_cps.get();
        self.keys.push(key);
    }

    fn build_ansi(&mut self, l: &Layout) {
        let (row0, row1, row2, row3, row4, row5) = (
            l.uy(0.0),
            l.uy(1.0),
            l.uy(2.0),
            l.uy(3.0),
            l.uy(4.0),
            l.uy(5.0),
        );

        // Row 0: Esc, F1-F12, Print/Scroll/Pause.
        self.add(l.key(KEY_ESCAPE, 0.0, row0));
        for i in 0..4 {
            self.add(l.key(KEY_F1 + i, l.ux(2.0 + i as f64), row0));
        }
        for i in 0..4 {
            self.add(l.key(KEY_F5 + i, l.ux(6.5 + i as f64), row0));
        }
        for i in 0..4 {
            self.add(l.key(KEY_F9 + i, l.ux(11.0 + i as f64), row0));
        }
        self.add(l.key(KEY_PRINT_SCREEN, l.ux(15.5), row0));
        self.add(l.key(KEY_SCROLL_LOCK, l.ux(16.5), row0));
        self.add(l.key(KEY_PAUSE, l.ux(17.5), row0));

        // Row 1: ` 1-0 - = BS, Ins/Home/PgUp.
        let row1_keys = [
            KEY_GRAVE, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, KEY_MINUS, KEY_EQUALS,
        ];
        for (i, code) in row1_keys.iter().enumerate() {
            self.add(l.key(*code, l.ux(i as f64), row1));
        }
        self.add(l.key_wide(KEY_BACKSPACE, l.ux(13.0), row1, DIM_BACKSPACE));
        self.add(l.key(KEY_INSERT, l.ux(15.5), row1));
        self.add(l.key(KEY_HOME, l.ux(16.5), row1));
        self.add(l.key(KEY_PAGE_UP, l.ux(17.5), row1));

        // Row 2: Tab QWERTY... backslash, Del/End/PgDn.
        self.add(l.key_wide(KEY_TAB, 0.0, row2, DIM_TAB));
        let row2_keys = [
            KEY_Q,
            87,
            69,
            82,
            84,
            89,
            85,
            73,
            79,
            80,
            KEY_LEFT_BRACKET,
            KEY_RIGHT_BRACKET,
        ];
        let tab_end = l.px(DIM_TAB) + l.key_gap;
        for (i, code) in row2_keys.iter().enumerate() {
            self.add(l.key(*code, tab_end + l.ux(i as f64), row2));
        }
        self.add(l.key_wide(KEY_BACKSLASH, tab_end + l.ux(12.0), row2, DIM_TAB));
        self.add(l.key(KEY_DELETE, l.ux(15.5), row2));
        self.add(l.key(KEY_END, l.ux(16.5), row2));
        self.add(l.key(KEY_PAGE_DOWN, l.ux(17.5), row2));

        // Row 3: Caps ASDF... Enter.
        self.add(l.key_wide(KEY_CAPS_LOCK, 0.0, row3, DIM_CAPS_LOCK));
        let row3_keys = [
            KEY_A,
            83,
            68,
            70,
            71,
            72,
            74,
            75,
            76,
            KEY_SEMICOLON,
            KEY_APOSTROPHE,
        ];
        let caps_end = l.px(DIM_CAPS_LOCK) + l.key_gap;
        for (i, code) in row3_keys.iter().enumerate() {
            self.add(l.key(*code, caps_end + l.ux(i as f64), row3));
        }
        self.add(l.key_wide(KEY_ENTER, caps_end + l.ux(11.0), row3, DIM_ENTER_ANSI));

        // Row 4: LShift ZXCV... RShift, Up.
        self.add(l.key_wide(KEY_LEFT_SHIFT, 0.0, row4, DIM_LEFT_SHIFT_ANSI));
        let row4_keys = [
            KEY_Z, 88, 67, 86, 66, 78, 77, KEY_COMMA, KEY_PERIOD, KEY_SLASH,
        ];
        let lshift_end = l.px(DIM_LEFT_SHIFT_ANSI) + l.key_gap;
        for (i, code) in row4_keys.iter().enumerate() {
            self.add(l.key(*code, lshift_end + l.ux(i as f64), row4));
        }
        self.add(l.key_wide(
            KEY_RIGHT_SHIFT,
            lshift_end + l.ux(10.0),
            row4,
            DIM_RIGHT_SHIFT,
        ));
        self.add(l.key(KEY_UP, l.ux(16.5), row4));

        // Row 5: Ctrl/Win/Alt/Space/Alt/Win/Menu/Ctrl, arrows.
        let mut x = 0.0;
        self.add(l.key_wide(KEY_LEFT_CONTROL, x, row5, DIM_CTRL));
        x += l.px(DIM_CTRL) + l.key_gap;
        self.add(l.key_wide(KEY_LEFT_SUPER, x, row5, DIM_GUI));
        x += l.px(DIM_GUI) + l.key_gap;
        self.add(l.key_wide(KEY_LEFT_ALT, x, row5, DIM_ALT));
        x += l.px(DIM_ALT) + l.key_gap;
        self.add(l.key_wide(KEY_SPACE, x, row5, DIM_SPACEBAR));
        x += l.px(DIM_SPACEBAR) + l.key_gap;
        self.add(l.key_wide(KEY_RIGHT_ALT, x, row5, DIM_ALT));
        x += l.px(DIM_ALT) + l.key_gap;
        self.add(l.key_wide(KEY_RIGHT_SUPER, x, row5, DIM_GUI));
        x += l.px(DIM_GUI) + l.key_gap;
        self.add(l.key_wide(KEY_MENU, x, row5, DIM_MENU));
        x += l.px(DIM_MENU) + l.key_gap;
        self.add(l.key_wide(KEY_RIGHT_CONTROL, x, row5, DIM_CTRL));
        self.add(l.key(KEY_LEFT, l.ux(15.5), row5));
        self.add(l.key(KEY_DOWN, l.ux(16.5), row5));
        self.add(l.key(KEY_RIGHT, l.ux(17.5), row5));
    }

    fn build_iso(&mut self, l: &Layout) {
        let (row0, row1, row2, row3, row4, row5) = (
            l.uy(0.0),
            l.uy(1.0),
            l.uy(2.0),
            l.uy(3.0),
            l.uy(4.0),
            l.uy(5.0),
        );

        // Row 0: Esc, F1-F12, Print/Scroll/Pause.
        self.add(l.key(KEY_ESCAPE, 0.0, row0));
        for i in 0..4 {
            self.add(l.key(KEY_F1 + i, l.ux(2.0 + i as f64), row0));
        }
        for i in 0..4 {
            self.add(l.key(KEY_F5 + i, l.ux(6.5 + i as f64), row0));
        }
        for i in 0..4 {
            self.add(l.key(KEY_F9 + i, l.ux(11.0 + i as f64), row0));
        }
        self.add(l.key(KEY_PRINT_SCREEN, l.ux(15.5), row0));
        self.add(l.key(KEY_SCROLL_LOCK, l.ux(16.5), row0));
        self.add(l.key(KEY_PAUSE, l.ux(17.5), row0));

        // Row 1: ` 1-0 - = BS, Ins/Home/PgUp.
        let row1_keys = [
            KEY_GRAVE, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, KEY_MINUS, KEY_EQUALS,
        ];
        for (i, code) in row1_keys.iter().enumerate() {
            self.add(l.key(*code, l.ux(i as f64), row1));
        }
        self.add(l.key_wide(KEY_BACKSPACE, l.ux(13.0), row1, DIM_BACKSPACE));
        self.add(l.key(KEY_INSERT, l.ux(15.5), row1));
        self.add(l.key(KEY_HOME, l.ux(16.5), row1));
        self.add(l.key(KEY_PAGE_UP, l.ux(17.5), row1));

        // Row 2: Tab QWERTY... brackets (no backslash on ISO).
        self.add(l.key_wide(KEY_TAB, 0.0, row2, DIM_TAB));
        let row2_keys = [
            KEY_Q,
            87,
            69,
            82,
            84,
            89,
            85,
            73,
            79,
            80,
            KEY_LEFT_BRACKET,
            KEY_RIGHT_BRACKET,
        ];
        let tab_end = l.px(DIM_TAB) + l.key_gap;
        for (i, code) in row2_keys.iter().enumerate() {
            self.add(l.key(*code, tab_end + l.ux(i as f64), row2));
        }
        self.add(l.key(KEY_DELETE, l.ux(15.5), row2));
        self.add(l.key(KEY_END, l.ux(16.5), row2));
        self.add(l.key(KEY_PAGE_DOWN, l.ux(17.5), row2));

        // Row 3: Caps ASDF... the ISO `#` key, then the L-shaped Enter.
        self.add(l.key_wide(KEY_CAPS_LOCK, 0.0, row3, DIM_CAPS_LOCK));
        let row3_keys = [
            KEY_A,
            83,
            68,
            70,
            71,
            72,
            74,
            75,
            76,
            KEY_SEMICOLON,
            KEY_APOSTROPHE,
        ];
        let caps_end = l.px(DIM_CAPS_LOCK) + l.key_gap;
        for (i, code) in row3_keys.iter().enumerate() {
            self.add(l.key(*code, caps_end + l.ux(i as f64), row3));
        }
        self.add(l.key(KEY_BACKSLASH, caps_end + l.ux(11.0), row3));

        // The ISO Enter: its stem sits at the right edge of the letter block and its
        // top bar reaches back to `tab_end + ux(12)`.
        let top_bar_start_x = tab_end + l.ux(12.0);
        let main_block_right = top_bar_start_x + l.px(DIM_TAB);
        let stem_width = l.px(DIM_ENTER_ISO_WIDTH);
        let stem_height = l.px(DIM_ENTER_ISO_HEIGHT);
        let mut enter = Key::new(
            KEY_ENTER,
            main_block_right - stem_width,
            row2,
            stem_width,
            stem_height,
        );
        enter.iso_top_bar_start = Some(top_bar_start_x);
        self.add(enter);

        // Row 4: LShift, ZXCV..., RShift, Up.
        self.add(l.key_wide(KEY_LEFT_SHIFT, 0.0, row4, DIM_LEFT_SHIFT_ISO));
        let lshift_end = l.px(DIM_LEFT_SHIFT_ISO) + l.key_gap;
        let row4_keys = [
            KEY_Z, 88, 67, 86, 66, 78, 77, KEY_COMMA, KEY_PERIOD, KEY_SLASH,
        ];
        for (i, code) in row4_keys.iter().enumerate() {
            self.add(l.key(*code, lshift_end + l.ux(1.0 + i as f64), row4));
        }
        let rshift_x = lshift_end + l.ux(11.0);
        self.add(l.key_wide(KEY_RIGHT_SHIFT, rshift_x, row4, DIM_RIGHT_SHIFT));
        self.add(l.key(KEY_UP, l.ux(16.5), row4));

        // Row 5: Ctrl/Win/Alt/Space/AltGr/Win/Menu/Ctrl, arrows.
        let mut x = 0.0;
        self.add(l.key_wide(KEY_LEFT_CONTROL, x, row5, DIM_CTRL));
        x += l.px(DIM_CTRL) + l.key_gap;
        self.add(l.key_wide(KEY_LEFT_SUPER, x, row5, DIM_GUI));
        x += l.px(DIM_GUI) + l.key_gap;
        self.add(l.key_wide(KEY_LEFT_ALT, x, row5, DIM_ALT));
        x += l.px(DIM_ALT) + l.key_gap;
        self.add(l.key_wide(KEY_SPACE, x, row5, DIM_SPACEBAR));
        x += l.px(DIM_SPACEBAR) + l.key_gap;
        self.add(l.key_wide(KEY_RIGHT_ALT, x, row5, DIM_ALT).named("AltGr"));
        x += l.px(DIM_ALT) + l.key_gap;
        self.add(l.key_wide(KEY_RIGHT_SUPER, x, row5, DIM_GUI));
        x += l.px(DIM_GUI) + l.key_gap;
        self.add(l.key_wide(KEY_MENU, x, row5, DIM_MENU));
        x += l.px(DIM_MENU) + l.key_gap;
        self.add(l.key_wide(KEY_RIGHT_CONTROL, x, row5, DIM_CTRL));
        self.add(l.key(KEY_LEFT, l.ux(15.5), row5));
        self.add(l.key(KEY_DOWN, l.ux(16.5), row5));
        self.add(l.key(KEY_RIGHT, l.ux(17.5), row5));
    }

    /// `KeyboardHud.calculateSize`.
    fn calculate_size(&mut self) {
        if self.keys.is_empty() {
            self.box_.set_size(0.0, 0.0);
            return;
        }
        let mut min_x: f64 = 0.0;
        let mut min_y: f64 = 0.0;
        let mut max_x: f64 = 0.0;
        let mut max_y: f64 = 0.0;
        for key in &self.keys {
            min_x = min_x.min(key.x);
            min_y = min_y.min(key.y);
            max_x = max_x.max(key.x + key.width);
            max_y = max_y.max(key.y + key.height);
        }
        self.min_x = min_x;
        self.min_y = min_y;
        let scale = self.scale.get();
        self.box_
            .set_size((max_x - min_x) * scale, (max_y - min_y) * scale);
    }

    // ---- colour ----------------------------------------------------------

    fn with_opacity(&self, color: Color) -> Color {
        let a = (color.a as f64 * self.opacity.get()).clamp(0.0, 255.0);
        Color {
            a: a as u8,
            ..color
        }
    }

    /// `KeyboardHud.getKeyColor`, including the `color-fade` HSB blend.
    fn key_color(&self, key: &Key) -> Color {
        if !self.color_fade.get() {
            let color = if key.pressed {
                self.pressed_color.color()
            } else {
                self.unpressed_color.color()
            };
            return self.with_opacity(color);
        }
        let pressed = self.pressed_color.color();
        let unpressed = self.unpressed_color.color();
        let t = key.delta as f64;
        let rgb = lerp_hsb(unpressed, pressed, t);
        let a = unpressed.a as f64 + (pressed.a as f64 - unpressed.a as f64) * t;
        self.with_opacity(Color {
            a: a.clamp(0.0, 255.0) as u8,
            ..rgb
        })
    }

    /// `Mth.lerp`/`Color.HSBtoRGB` blend Java does while a key fades.
    fn fade_step(&self, dt: f64, pressed: bool) -> f64 {
        let fade_time = self.fade_time.get().max(0.01);
        let sign = if pressed { 1.0 } else { -1.0 };
        dt / fade_time * sign
    }

    fn tick_impl(&mut self, env: &mut dyn Env) {
        if self.keys.is_empty() {
            self.rebuild();
            return;
        }
        let now = env.now();
        for key in self.keys.iter_mut() {
            let pressed = key.is_down(env);
            key.update(pressed, now);
        }
    }

    fn render_impl(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        if self.keys.is_empty() {
            self.rebuild();
            return;
        }

        let now = env.now();
        let dt = match self.last_render_time {
            Some(last) => (now - last).clamp(0.0, 0.1),
            None => 1.0 / 60.0,
        };
        self.last_render_time = Some(now);

        // PORT GAP: the Java element also synthesises the key-up for Meteor's own
        // "open GUI" key, because the module search box swallowed its release; the
        // port reads the key state from `Env` every frame instead of tracking
        // events, so the workaround is not needed.
        let scale = self.scale.get();
        let width = self.width() as f64;
        let height = self.height() as f64;

        if self.background.get() {
            let color = self.with_opacity(self.background_color.color());
            r.rect(0.0, 0.0, width, height, color.argb());
        }

        let border_enabled = self.border.get();
        let border_width = self.border_width.get();
        let border_color = self.with_opacity(self.border_color.color()).argb();
        let text_color = self.with_opacity(self.text_color.color()).argb();
        let align_left = self.alignment.is("left");
        let align_right = self.alignment.is("right");

        for index in 0..self.keys.len() {
            // The fade advances a fraction of the key's own target each frame.
            {
                let pressed = self.keys[index].pressed;
                let step = self.fade_step(dt, pressed);
                let key = &mut self.keys[index];
                key.delta = (key.delta as f64 + step).clamp(0.0, 1.0) as f32;
            }
            let key = self.keys[index].clone();
            let color = self.key_color(&key);

            let kx = (key.x - self.min_x) * scale;
            let ky = (key.y - self.min_y) * scale;
            let kw = key.width * scale;
            let kh = key.height * scale;

            if let Some(top_bar_start) = key.iso_top_bar_start {
                // `IsoEnterKey.render`: an L made of three non-overlapping quads.
                let top_bar_x = (top_bar_start - self.min_x) * scale;
                let unit = 35.0 * scale;
                let stem_right = kx + kw;
                let left_width = stem_right - top_bar_x - kw;
                if left_width > 0.0 {
                    r.rect(top_bar_x, ky, left_width, unit, color.argb());
                }
                r.rect(kx, ky, kw, unit, color.argb());
                r.rect(kx, ky + unit, kw, kh - unit, color.argb());
                if border_enabled {
                    let full_top = left_width + kw;
                    r.rect(top_bar_x, ky, full_top, border_width, border_color);
                    r.rect(top_bar_x, ky, border_width, unit, border_color);
                    r.rect(
                        top_bar_x + full_top - border_width,
                        ky,
                        border_width,
                        kh,
                        border_color,
                    );
                    r.rect(kx, ky + kh - border_width, kw, border_width, border_color);
                    r.rect(kx, ky + unit, border_width, kh - unit, border_color);
                    if left_width > 0.0 {
                        r.rect(
                            top_bar_x,
                            ky + unit - border_width,
                            left_width,
                            border_width,
                            border_color,
                        );
                    }
                }

                let label = key.label();
                let padding = 2.0 * scale;
                let available = kw - padding * 2.0;
                let text_width = r.text_width(&label, 1.0);
                let text_scale = if text_width > available && text_width > 0.0 {
                    available / text_width
                } else {
                    1.0
                };
                let drawn = r.text_width(&label, text_scale);
                let text_height = common::text_height(text_scale);
                let y = ky + (kh - text_height) / 2.0;
                let x = text_x(kx, kw, drawn, padding, align_left, align_right);
                r.text2d(x, y, &label, text_color, text_scale);
                continue;
            }

            r.rect(kx, ky, kw, kh, color.argb());

            if border_enabled {
                r.rect(kx, ky, kw, border_width, border_color);
                r.rect(kx, ky + kh - border_width, kw, border_width, border_color);
                r.rect(kx, ky, border_width, kh, border_color);
                r.rect(kx + kw - border_width, ky, border_width, kh, border_color);
            }

            let label = key.label();
            let padding = 2.0 * scale;
            let available = kw - padding;
            let label_width = r.text_width(&label, 1.0);

            if !key.show_cps {
                let text_scale = if label_width > available && label_width > 0.0 {
                    available / label_width
                } else {
                    1.0
                };
                let drawn = r.text_width(&label, text_scale);
                let y = ky + (kh - common::text_height(text_scale)) / 2.0;
                let x = text_x(kx, kw, drawn, padding, align_left, align_right);
                r.text2d(x, y, &label, text_color, text_scale);
            } else {
                let top_scale = if label_width > available && label_width > 0.0 {
                    available / label_width
                } else {
                    1.0
                };
                let cps_text = format!("{} CPS", key.cps(now));
                let cps_width = r.text_width(&cps_text, 1.0);
                let bottom_scale = if cps_width > available && cps_width > 0.0 {
                    available / cps_width
                } else {
                    1.0
                };
                let top_height = common::text_height(top_scale);
                let bottom_height = common::text_height(bottom_scale);
                let start_y = ky + (kh - (top_height + bottom_height)) / 2.0;

                let drawn = r.text_width(&label, top_scale);
                let x = text_x(kx, kw, drawn, padding, align_left, align_right);
                r.text2d(x, start_y, &label, text_color, top_scale);

                let drawn = r.text_width(&cps_text, bottom_scale);
                let x = text_x(kx, kw, drawn, padding, align_left, align_right);
                r.text2d(x, start_y + top_height, &cps_text, text_color, bottom_scale);
            }
        }

        let scale = self.scale.get();
        self.calculate_size();
        let _ = scale;
    }
}

/// The x of a key's text, per the element's `alignment`.
fn text_x(kx: f64, kw: f64, text_width: f64, padding: f64, left: bool, right: bool) -> f64 {
    if left {
        kx + padding
    } else if right {
        kx + kw - padding - text_width
    } else {
        kx + (kw - text_width) / 2.0
    }
}

/// `java.awt.Color.RGBtoHSB`'s hue in degrees plus saturation/value in `0..1`.
fn rgb_to_hsv(c: Color) -> (f64, f64, f64) {
    let r = c.r as f64 / 255.0;
    let g = c.g as f64 / 255.0;
    let b = c.b as f64 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let hue = if delta.abs() < f64::EPSILON {
        0.0
    } else if (max - r).abs() < f64::EPSILON {
        60.0 * (((g - b) / delta) % 6.0)
    } else if (max - g).abs() < f64::EPSILON {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    let hue = if hue < 0.0 { hue + 360.0 } else { hue };
    let sat = if max.abs() < f64::EPSILON {
        0.0
    } else {
        delta / max
    };
    (hue, sat, max)
}

/// The HSB blend `getKeyColor` does between the unpressed and pressed colours.
fn lerp_hsb(a: Color, b: Color, t: f64) -> Color {
    let t = t.clamp(0.0, 1.0);
    let (h1, s1, v1) = rgb_to_hsv(a);
    let (h2, s2, v2) = rgb_to_hsv(b);
    let mut dh = h2 - h1;
    if dh > 180.0 {
        dh -= 360.0;
    } else if dh < -180.0 {
        dh += 360.0;
    }
    Color::from_hsv(h1 + dh * t, s1 + (s2 - s1) * t, v1 + (v2 - v1) * t)
}

impl Default for KeyboardElement {
    fn default() -> Self {
        Self::new()
    }
}

super::hud_element!(KeyboardElement);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::NullRenderer;

    #[test]
    fn movement_preset_draws_the_six_keys() {
        let mut element = KeyboardElement::new();
        assert!(element.preset.is("movement"));
        assert_eq!(element.keys.len(), 6);
        assert_eq!(
            element.keys.iter().map(|k| k.label()).collect::<Vec<_>>(),
            vec!["W", "A", "S", "D", "LSh", "Space"]
        );

        let mut env = TestEnv::in_world();
        let mut r = NullRenderer::default();
        element.render(&mut env, &mut r);
        assert_eq!(r.texts, 6);
        assert!(element.width() > 0 && element.height() > 0);
    }

    #[test]
    fn presets_rebuild_the_layout() {
        let mut element = KeyboardElement::new();
        element.preset.set_index(3); // hotbar
        element.rebuild();
        assert_eq!(element.keys.len(), 9);
        assert_eq!(element.keys[0].label(), "1");

        element.preset.set_index(1); // clicks
        element.rebuild();
        assert_eq!(element.keys.len(), 2);
        assert_eq!(element.keys[0].label(), "LMB");
        assert!(element.keys[0].mouse.is_some());

        element.preset.set_index(4); // keyboard, ANSI
        element.rebuild();
        assert_eq!(element.keys.len(), 87, "the ANSI layout's key count");

        element.keyboard_layout.set_index(1); // ISO
        element.rebuild();
        assert!(element.keys.iter().any(|k| k.iso_top_bar_start.is_some()));
    }

    #[test]
    fn pressed_keys_count_clicks_and_fade() {
        let mut element = KeyboardElement::new();
        element.show_cps.set(true);
        element.color_fade.set(true);
        element.rebuild();

        let mut env = TestEnv::in_world();
        env.keys.push(87); // W held
        element.tick(&mut env);
        assert!(element.keys[0].pressed);
        assert_eq!(element.keys[0].cps(0.0), 1, "one press recorded");

        let mut r = NullRenderer::default();
        element.render(&mut env, &mut r);
        assert!(element.keys[0].delta > 0.0, "the fade advanced");
    }

    #[test]
    fn custom_preset_reads_the_id_list() {
        let mut element = KeyboardElement::new();
        element.preset.set_index(5);
        element
            .custom_keys
            .set(vec!["space".into(), "lmb".into(), "z".into()]);
        element.rebuild();
        assert_eq!(element.keys.len(), 3);
        assert_eq!(element.keys[0].label(), "Space");
        assert_eq!(element.keys[1].label(), "lmb");
        assert_eq!(element.keys[2].label(), "Z");
    }
}
