//! HORIXON — a native Horizon client mod with an original module manager.
//! The launcher compiles this crate against the exact SDK pinned in horizon-mod.toml.
use std::sync::{Mutex, atomic::{AtomicBool, AtomicUsize, Ordering}};

use mod_api::client::widgets::{Button, Outcome, WidgetId, Widgets};
use mod_api::{
    ClientHost, ClientTickPostEvent, ClientTickPreEvent, Flow, GuiRenderPostEvent,
    Hooks, InputKeyEvent, KeyAction, KeyConflictContext, KeyRegistration,
    MouseScrollingEvent, RegistrationContext, Registrar, ScreenCharTypedPreEvent,
    ScreenKeyPressedPreEvent, ScreenMouseButtonPressedPreEvent,
    ScreenMouseButtonReleasedPreEvent, ScreenMouseDraggedPreEvent,
    ScreenRenderPostEvent, SettingSpec, Text, ZoomEvent,
};

const ID: &str = "horixon";
const VERSION: &str = "0.4.0";
const MENU_KEY: &str = "menu";
const ZOOM_KEY: &str = "zoom";
const MENU_DEFAULT: &str = "key.keyboard.right.shift";
const ZOOM_DEFAULT: &str = "key.keyboard.z";

const BACKGROUND: u32 = 0xF0101726;
const ACCENT: u32 = 0xFF38BDF8;
const TEXT: u32 = 0xFFF1F5F9;
const MUTED: u32 = 0xFF94A3B8;

const CATEGORIES: [&str; 6] = ["Combat", "Movement", "Render", "Player", "World", "Misc"];

#[derive(Clone, Copy)]
struct ModuleInfo {
    category: usize,
    key: &'static str,
    name: &'static str,
    default: bool,
}

const MODULES: [ModuleInfo; 5] = [
    ModuleInfo { category: 2, key: "array_list", name: "ArrayList", default: true },
    ModuleInfo { category: 2, key: "coordinates", name: "Coordinates", default: true },
    ModuleInfo { category: 2, key: "fps", name: "FPS", default: true },
    ModuleInfo { category: 2, key: "compass", name: "Compass", default: true },
    ModuleInfo { category: 5, key: "zoom_enabled", name: "Zoom", default: true },
];

fn settings() -> [SettingSpec; 8] {
    [
        SettingSpec::toggle("array_list", "setting.horixon.array_list", true)
            .tooltip("setting.horixon.array_list.tooltip"),
        SettingSpec::toggle("coordinates", "setting.horixon.coordinates", true)
            .tooltip("setting.horixon.coordinates.tooltip"),
        SettingSpec::toggle("fps", "setting.horixon.fps", true)
            .tooltip("setting.horixon.fps.tooltip"),
        SettingSpec::toggle("compass", "setting.horixon.compass", true)
            .tooltip("setting.horixon.compass.tooltip"),
        SettingSpec::toggle("zoom_enabled", "setting.horixon.zoom", true)
            .tooltip("setting.horixon.zoom.tooltip"),
        SettingSpec::range("zoom_level", "setting.horixon.zoom_level", 2.0, 10.0, 1.0, 4.0)
            .tooltip("setting.horixon.zoom_level.tooltip"),
        SettingSpec::toggle("zoom_smooth", "setting.horixon.zoom_smooth", true)
            .tooltip("setting.horixon.zoom_smooth.tooltip"),
        SettingSpec::toggle("zoom_sensitivity", "setting.horixon.zoom_sensitivity", true)
            .tooltip("setting.horixon.zoom_sensitivity.tooltip"),
    ]
}

fn enabled(host: &dyn ClientHost, key: &str, default: bool) -> bool {
    host.setting_bool(key).unwrap_or(default)
}

#[derive(Clone, Copy)]
struct ZoomSettings {
    factor: f32,
    smooth: bool,
    sensitivity: bool,
}

#[derive(Default)]
struct ZoomState {
    held: Option<ZoomSettings>,
}

#[derive(Default)]
struct FpsCounter {
    started_ms: u64,
    frames: u32,
    fps: u32,
}

impl FpsCounter {
    fn frame(&mut self, now: u64) -> u32 {
        if self.started_ms == 0 {
            self.started_ms = now;
        }
        self.frames = self.frames.saturating_add(1);
        let elapsed = now.saturating_sub(self.started_ms);
        if elapsed >= 1_000 {
            self.fps = ((self.frames as u64 * 1_000) / elapsed) as u32;
            self.frames = 0;
            self.started_ms = now;
        }
        self.fps
    }
}

#[derive(Clone, Copy)]
enum MenuAction {
    Category(usize),
    Toggle(&'static str),
    Close,
}

struct MenuScreen {
    widgets: Widgets,
    panel_x: f32,
    panel_width: f32,
    panel_height: f32,
    category_buttons: Vec<(WidgetId, usize)>,
    module_buttons: Vec<(WidgetId, &'static str)>,
    close_button: WidgetId,
    empty: bool,
}

impl MenuScreen {
    fn new(width: f32, category: usize, host: &dyn ClientHost) -> Self {
        let panel_width = width.min(640.0).max(360.0);
        let panel_x = ((width - panel_width) / 2.0).max(8.0);
        let panel_height = 304.0;
        let mut widgets = Widgets::new();
        let mut category_buttons = Vec::new();
        let mut module_buttons = Vec::new();

        let tab_gap = 6.0;
        let tab_width = (panel_width - 36.0 - tab_gap * 5.0) / 6.0;
        for (index, name) in CATEGORIES.iter().enumerate() {
            let x = panel_x + 18.0 + index as f32 * (tab_width + tab_gap);
            let id = widgets.add_button(Button::new(
                [x, 72.0, tab_width, 24.0],
                Text::literal(*name),
            ));
            category_buttons.push((id, index));
        }

        for (row, module) in MODULES.iter().filter(|m| m.category == category).enumerate() {
            let state = enabled(host, module.key, module.default);
            let label = format!("{}    {}", module.name, if state { "[ON]" } else { "[OFF]" });
            let id = widgets.add_button(Button::new(
                [panel_x + 18.0, 112.0 + row as f32 * 32.0, 280.0, 26.0],
                Text::literal(label),
            ));
            module_buttons.push((id, module.key));
        }

        let close_button = widgets.add_button(Button::new(
            [panel_x + panel_width - 84.0, 30.0, 66.0, 22.0],
            Text::literal("CLOSE"),
        ));

        let empty = module_buttons.is_empty();
        Self {
            widgets,
            panel_x,
            panel_width,
            panel_height,
            category_buttons,
            module_buttons,
            close_button,
            empty,
        }
    }

    fn action_for(&self, clicked: WidgetId) -> Option<MenuAction> {
        if clicked == self.close_button {
            return Some(MenuAction::Close);
        }
        if let Some((_, category)) = self.category_buttons.iter().find(|(id, _)| *id == clicked) {
            return Some(MenuAction::Category(*category));
        }
        self.module_buttons
            .iter()
            .find(|(id, _)| *id == clicked)
            .map(|(_, key)| MenuAction::Toggle(key))
    }

    fn draw(&mut self, host: &dyn ClientHost, event: &mut ScreenRenderPostEvent<'_>) {
        let [width, _height] = event.draw.size();
        event.draw.fill([0.0, 0.0, width, height], 0x99000000);
        event.draw.fill(
            [self.panel_x, 22.0, self.panel_width, self.panel_height],
            BACKGROUND,
        );
        event.draw.fill([self.panel_x, 22.0, self.panel_width, 2.0], ACCENT);
        event.draw.text("HORIXON", [self.panel_x + 18.0, 36.0], TEXT, true);
        let subtitle = format!("NATIVE CLIENT  /  MODULE MANAGER  /  v{VERSION}");
        event.draw.text(&subtitle, [self.panel_x + 18.0, 52.0], MUTED, true);
        if self.empty {
            event.draw.text(
                "No modules ported in this category yet.",
                [self.panel_x + 20.0, 124.0],
                MUTED,
                true,
            );
        }
        event.draw.text(
            "Right Shift: close  |  Z: hold to zoom  |  Tab: navigate",
            [self.panel_x + 18.0, 278.0],
            MUTED,
            true,
        );
        let _ = self.widgets.draw(&mut *event.draw, host, event.screen.mouse);
    }
}

#[derive(Default)]
struct Horixon {
    screen: Mutex<Option<MenuScreen>>,
    selected_category: AtomicUsize,
    close_screen: AtomicBool,
    fps: Mutex<FpsCounter>,
    zoom: Mutex<ZoomState>,
}

impl Horixon {
    fn input(
        &self,
        host: &mut dyn ClientHost,
        input: impl FnOnce(&mut Widgets) -> Outcome,
    ) -> Flow {
        if host.mod_screen() != Some(ID) {
            return Flow::Continue;
        }

        let (handled, action) = {
            let mut slot = self.screen.lock().unwrap_or_else(|e| e.into_inner());
            let Some(screen) = slot.as_mut() else {
                return Flow::Continue;
            };
            let outcome = input(&mut screen.widgets);
            let handled = outcome.handled();
            let action = match outcome {
                Outcome::Pressed(id) => screen.action_for(id),
                _ => None,
            };
            (handled, action)
        };

        if let Some(action) = action {
            match action {
                MenuAction::Category(category) => {
                    self.selected_category.store(category, Ordering::Relaxed);
                }
                MenuAction::Toggle(key) => {
                    let value = !host.setting_bool(key).unwrap_or(true);
                    host.set_setting(ID, key, if value { "true" } else { "false" });
                }
                MenuAction::Close => {
                    self.close_screen.store(true, Ordering::Relaxed);
                }
            }
            *self.screen.lock().unwrap_or_else(|e| e.into_inner()) = None;
        }

        if handled { Flow::Cancel } else { Flow::Continue }
    }

    fn current_zoom(&self, host: &dyn ClientHost) -> Option<ZoomSettings> {
        let active = host.playing()
            && host.on(ID)
            && enabled(host, "zoom_enabled", true)
            && host.is_down(ID, ZOOM_KEY);
        if !active {
            self.zoom.lock().unwrap_or_else(|e| e.into_inner()).held = None;
            return None;
        }

        let starting = ZoomSettings {
            factor: host.setting_f64("zoom_level").unwrap_or(4.0) as f32,
            smooth: enabled(host, "zoom_smooth", true),
            sensitivity: enabled(host, "zoom_sensitivity", true),
        };
        let mut state = self.zoom.lock().unwrap_or_else(|e| e.into_inner());
        if state.held.is_none() {
            state.held = Some(starting);
        }
        state.held
    }
}

fn cardinal_direction(yaw: f32) -> &'static str {
    const DIRECTIONS: [&str; 8] = ["S", "SW", "W", "NW", "N", "NE", "E", "SE"];
    let sector = ((yaw.rem_euclid(360.0) / 45.0).round() as usize) % DIRECTIONS.len();
    DIRECTIONS[sector]
}

#[mod_api::horizon_mod]
impl Hooks for Horixon {
    fn register(&self, registrar: &mut dyn Registrar, _: &RegistrationContext) {
        let _ = registrar.register_key_mapping(
            KeyRegistration::new(MENU_KEY, MENU_DEFAULT)
                .label("key.horixon.menu")
                .context(KeyConflictContext::InGame),
        );
        let _ = registrar.register_key_mapping(
            KeyRegistration::new(ZOOM_KEY, ZOOM_DEFAULT)
                .label("key.horixon.zoom")
                .context(KeyConflictContext::InGame),
        );
        for setting in settings() {
            let _ = registrar.register_setting(setting);
        }
    }

    fn input_key(&self, host: &mut dyn ClientHost, event: &mut InputKeyEvent<'_>) {
        let menu_key = host.mod_key(ID, MENU_KEY).map(|(key, _)| key);
        if event.action != KeyAction::Press
            || menu_key.as_deref() != Some(event.key)
            || !host.playing()
            || !host.on(ID)
        {
            return;
        }
        if host.mod_screen() == Some(ID) {
            self.close_screen.store(true, Ordering::Relaxed);
        } else {
            *self.screen.lock().unwrap_or_else(|e| e.into_inner()) = None;
            host.mod_screen_open(ID);
        }
    }

    fn client_tick_pre(&self, host: &mut dyn ClientHost, _: &mut ClientTickPreEvent) {
        if self.close_screen.swap(false, Ordering::Relaxed) {
            host.mod_screen_close(ID);
            *self.screen.lock().unwrap_or_else(|e| e.into_inner()) = None;
        } else if host.mod_screen() != Some(ID) {
            *self.screen.lock().unwrap_or_else(|e| e.into_inner()) = None;
        }
    }

    fn client_tick_post(&self, host: &mut dyn ClientHost, _: &mut ClientTickPostEvent) {
        let _ = self.current_zoom(host);
    }

    fn zoom(&self, host: &dyn ClientHost, _: &mut ZoomEvent) -> Option<mod_api::Zoom> {
        let zoom = self.current_zoom(host)?;
        Some(mod_api::Zoom {
            factor: zoom.factor,
            smooth: zoom.smooth,
            turn: if zoom.sensitivity { 1.0 / zoom.factor } else { 1.0 },
        })
    }

    fn mouse_scrolling(&self, host: &mut dyn ClientHost, event: &mut MouseScrollingEvent) -> Flow {
        if event.scroll[1] == 0.0 || self.current_zoom(host).is_none() {
            return Flow::Continue;
        }
        let mut state = self.zoom.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(zoom) = state.held.as_mut() {
            zoom.factor = (zoom.factor * 1.25_f32.powf(event.scroll[1] as f32)).clamp(1.25, 10.0);
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    fn gui_render_post(&self, host: &dyn ClientHost, event: &mut GuiRenderPostEvent<'_>) {
        if !host.playing() || !host.on(ID) || host.mod_screen() == Some(ID) {
            return;
        }

        let fps = self.fps.lock().unwrap_or_else(|e| e.into_inner()).frame(host.millis());
        let camera = host.camera();
        let mut left_lines: Vec<String> = Vec::new();

        if enabled(host, "fps", true) {
            left_lines.push(format!("FPS  {fps}"));
        }
        if enabled(host, "coordinates", true) {
            if let Some(camera) = camera.as_ref() {
                let [x, y, z] = camera.position;
                left_lines.push(format!("XYZ  {:.1}  {:.1}  {:.1}  {}", x, y, z, camera.dimension));
            }
        }
        if enabled(host, "compass", true) {
            if let Some(camera) = camera.as_ref() {
                left_lines.push(format!("FACING  {}", cardinal_direction(camera.yaw)));
            }
        }

        if !left_lines.is_empty() {
            let max_width = left_lines.iter()
                .map(|line| event.batch.text_width(line))
                .fold(0.0_f32, f32::max);
            let panel_height = left_lines.len() as f32 * 14.0 + 10.0;
            event.batch.fill([10.0, 10.0, max_width + 20.0, panel_height], 0xB0101726);
            event.batch.fill([10.0, 10.0, 2.0, panel_height], ACCENT);
            for (index, line) in left_lines.iter().enumerate() {
                event.batch.text(line, [18.0, 14.0 + index as f32 * 14.0], TEXT, true);
            }
        }

        if enabled(host, "array_list", true) {
            let active: Vec<&str> = MODULES.iter()
                .filter(|module| module.key != "array_list" && enabled(host, module.key, module.default))
                .map(|module| module.name)
                .collect();
            if !active.is_empty() {
                let screen_width = event.batch.size()[0];
                let max_width = active.iter()
                    .map(|name| event.batch.text_width(name))
                    .fold(0.0_f32, f32::max);
                let panel_width = max_width + 20.0;
                let panel_height = active.len() as f32 * 14.0 + 10.0;
                let panel_x = screen_width - panel_width - 10.0;
                event.batch.fill([panel_x, 10.0, panel_width, panel_height], 0xB0101726);
                event.batch.fill([panel_x, 10.0, 2.0, panel_height], ACCENT);
                for (index, name) in active.iter().enumerate() {
                    let text_x = screen_width - 18.0 - event.batch.text_width(name);
                    event.batch.text(name, [text_x, 14.0 + index as f32 * 14.0], ACCENT, true);
                }
            }
        }
    }

    fn screen_render_post(&self, host: &dyn ClientHost, event: &mut ScreenRenderPostEvent<'_>) {
        if host.mod_screen() != Some(ID) {
            return;
        }
        let width = event.draw.size()[0];
        let mut slot = self.screen.lock().unwrap_or_else(|e| e.into_inner());
        if slot.is_none() {
            *slot = Some(MenuScreen::new(
                width,
                self.selected_category.load(Ordering::Relaxed) % CATEGORIES.len(),
                host,
            ));
        }
        if let Some(screen) = slot.as_mut() {
            screen.draw(host, event);
        }
    }

    fn screen_mouse_button_pressed_pre(
        &self,
        host: &mut dyn ClientHost,
        event: &mut ScreenMouseButtonPressedPreEvent<'_>,
    ) -> Flow {
        let (at, button, modifiers) = (event.screen.mouse, event.button, event.modifiers);
        self.input(host, |widgets| widgets.mouse_pressed(host, at, button, modifiers))
    }

    fn screen_mouse_button_released_pre(
        &self,
        host: &mut dyn ClientHost,
        event: &mut ScreenMouseButtonReleasedPreEvent<'_>,
    ) -> Flow {
        let button = event.button;
        self.input(host, |widgets| widgets.mouse_released(button))
    }

    fn screen_mouse_dragged_pre(
        &self,
        host: &mut dyn ClientHost,
        event: &mut ScreenMouseDraggedPreEvent<'_>,
    ) -> Flow {
        let (at, button) = (event.screen.mouse, event.button);
        self.input(host, |widgets| widgets.mouse_dragged(host, at, button))
    }

    fn screen_key_pressed_pre(
        &self,
        host: &mut dyn ClientHost,
        event: &mut ScreenKeyPressedPreEvent<'_>,
    ) -> Flow {
        let menu_key = host.mod_key(ID, MENU_KEY).map(|(key, _)| key);
        if menu_key.as_deref() == Some(event.key) {
            self.close_screen.store(true, Ordering::Relaxed);
            return Flow::Cancel;
        }
        let (key, modifiers) = (event.key, event.modifiers);
        self.input(host, |widgets| widgets.key_pressed(host, key, modifiers))
    }

    fn screen_char_typed_pre(
        &self,
        host: &mut dyn ClientHost,
        event: &mut ScreenCharTypedPreEvent<'_>,
    ) -> Flow {
        let character = event.c;
        self.input(host, |widgets| widgets.char_typed(host, character))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compass_uses_minecraft_yaw_quadrants() {
        assert_eq!(cardinal_direction(0.0), "S");
        assert_eq!(cardinal_direction(90.0), "W");
        assert_eq!(cardinal_direction(180.0), "N");
        assert_eq!(cardinal_direction(270.0), "E");
        assert_eq!(cardinal_direction(-90.0), "E");
    }

    #[test]
    fn fps_counter_does_not_divide_by_zero() {
        let mut fps = FpsCounter::default();
        assert_eq!(fps.frame(0), 0);
        assert_eq!(fps.frame(500), 0);
        assert_eq!(fps.frame(1_000), 3);
    }
}
