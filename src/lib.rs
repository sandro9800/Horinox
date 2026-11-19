//! HORIXON — native Rust mod for Horizon, Minecraft 26.3.
//! The launcher-generated `hooks()` entry point is provided by `#[horizon_mod]`.

extern crate mod_api;

pub mod horizon_api_shim;
pub mod meteor_core;
pub mod port;

// Horizon hook routing for HORIXON's native ClickGUI and in-game modules.

use std::sync::Mutex;

use crate::horizon_api_shim::mod_api::{
    ClientTickPostEvent, ClientTickPreEvent, Flow, GuiRenderPostEvent, Hooks, Host, InputKeyEvent,
    ScreenCharTypedPreEvent, ScreenKeyPressedPreEvent, ScreenMouseButtonPressedPreEvent,
    ScreenMouseButtonReleasedPreEvent, ScreenMouseDraggedPreEvent, ScreenMouseScrolledPreEvent,
    ScreenRenderPostEvent, WorldPrimitivesEvent,
};
use crate::meteor_core::MOD_ID;
use crate::port::env_impl::{DrawQueue, HeadlessEnv};
use crate::port::shared::Shared;

pub struct HorixonHooks {
    shared: Mutex<Shared>,
    gui: Mutex<crate::port::gui::ClickGui>,
}

impl Default for HorixonHooks {
    fn default() -> Self {
        Self::new()
    }
}

impl HorixonHooks {
    pub fn new() -> Self {
        let mut shared = Shared::new();
        shared.load();
        Self {
            shared: Mutex::new(shared),
            gui: Mutex::new(crate::port::gui::ClickGui::new()),
        }
    }
}

fn glfw_key(key: &str) -> Option<i32> {
    Some(match key {
        "key.keyboard.escape" => 256,
        "key.keyboard.enter" => 257,
        "key.keyboard.tab" => 258,
        "key.keyboard.backspace" => 259,
        "key.keyboard.right" => 262,
        "key.keyboard.left" => 263,
        "key.keyboard.down" => 264,
        "key.keyboard.up" => 265,
        "key.keyboard.space" => 32,
        "key.keyboard.h" => 72,
        "key.keyboard.right.shift" => 344,
        _ => return None,
    })
}

#[mod_api::horizon_mod]
impl Hooks for HorixonHooks {
    fn client_tick_pre(&self, host: &mut dyn Host, _event: &mut ClientTickPreEvent) {
        if let Ok(mut shared) = self.shared.lock() {
            shared.in_world = host.player().is_some();
            shared.tick(host, None, None);
        }
    }

    fn client_tick_post(&self, host: &mut dyn Host, _event: &mut ClientTickPostEvent) {
        let own_screen = host.mod_screen() == Some(MOD_ID);
        if let (Ok(mut shared), Ok(mut gui)) = (self.shared.lock(), self.gui.lock()) {
            if gui.is_open() && !own_screen {
                gui.close();
            } else if !gui.is_open() && own_screen {
                host.mod_screen_close(MOD_ID);
            }
            gui.tick(&mut shared);
        }
    }

    fn gui_render_post(&self, host: &dyn Host, event: &mut GuiRenderPostEvent<'_>) {
        if host.mod_screen() == Some(MOD_ID) {
            return;
        }
        let mut queue = DrawQueue::new((event.batch.width as f64, event.batch.height as f64));
        if let Ok(mut shared) = self.shared.lock() {
            let snapshot = shared.snapshot.clone();
            let mut pending = Vec::new();
            let ticks = shared.ticks;
            let mut env = HorizonEnv::new_headless(&snapshot, &mut pending, ticks);
            shared.hud.render(&mut env, &mut queue);
            shared.modules.render_2d(&mut env, &mut queue);
        }
        queue.flush_to_hud(host, event.batch);
    }

    fn world_primitives(&self, _host: &dyn Host, event: &mut WorldPrimitivesEvent<'_>) {
        let mut queue = DrawQueue::new((0.0, 0.0));
        if let Ok(mut shared) = self.shared.lock() {
            shared.render_world(&mut queue);
        }
        queue.flush_3d(event.primitives);
    }

    fn input_key(&self, host: &mut dyn Host, event: &mut InputKeyEvent<'_>) {
        if host.mod_screen().is_some() || host.player().is_none() {
            return;
        }
        let code = format!("{:?}", event.event.physical_key);
        let pressed = format!("{:?}", event.event.state).contains("Pressed");
        if !pressed || !code.contains("ShiftRight") {
            return;
        }
        let Ok(mut gui) = self.gui.lock() else { return };
        if gui.is_open() {
            return;
        }
        gui.open();
        host.mod_screen_open(MOD_ID);
    }

    fn screen_render_post(&self, host: &dyn Host, event: &mut ScreenRenderPostEvent<'_>) {
        if host.mod_screen() != Some(MOD_ID) {
            return;
        }
        if let (Ok(shared), Ok(mut gui)) = (self.shared.lock(), self.gui.lock()) {
            let mut queue = DrawQueue::new((event.batch.width as f64, event.batch.height as f64));
            gui.render_screen(&shared, &mut queue, event.screen.mouse);
            queue.flush_to_hud(host, event.batch);
        }
    }

    fn screen_key_pressed_pre(
        &self,
        host: &dyn Host,
        event: &mut ScreenKeyPressedPreEvent<'_>,
    ) -> Flow {
        if host.mod_screen() != Some(MOD_ID) {
            return Flow::Continue;
        }
        let (Ok(mut shared), Ok(mut gui)) = (self.shared.lock(), self.gui.lock()) else {
            return Flow::Cancel;
        };
        if let Some(key) = glfw_key(event.key) {
            gui.on_key(&mut shared, key, true, event.modifiers as i32);
        }
        if !gui.is_open() {
            host.mod_screen_close(MOD_ID);
        }
        Flow::Cancel
    }

    fn screen_char_typed_pre(
        &self,
        host: &dyn Host,
        event: &mut ScreenCharTypedPreEvent<'_>,
    ) -> Flow {
        if host.mod_screen() != Some(MOD_ID) {
            return Flow::Continue;
        }
        if let (Ok(mut shared), Ok(mut gui)) = (self.shared.lock(), self.gui.lock()) {
            gui.on_char(&mut shared, event.c);
        }
        Flow::Cancel
    }

    fn screen_mouse_button_pressed_pre(
        &self,
        host: &dyn Host,
        event: &mut ScreenMouseButtonPressedPreEvent<'_>,
    ) -> Flow {
        if host.mod_screen() != Some(MOD_ID) {
            return Flow::Continue;
        }
        let cursor = event.screen.mouse;
        if let (Ok(mut shared), Ok(mut gui)) = (self.shared.lock(), self.gui.lock()) {
            gui.on_mouse_move(cursor[0] as f64, cursor[1] as f64);
            let mut env = HeadlessEnv::new();
            gui.on_mouse(&mut shared, &mut env, event.button, true);
        }
        Flow::Cancel
    }

    fn screen_mouse_button_released_pre(
        &self,
        host: &dyn Host,
        event: &mut ScreenMouseButtonReleasedPreEvent<'_>,
    ) -> Flow {
        if host.mod_screen() != Some(MOD_ID) {
            return Flow::Continue;
        }
        let cursor = event.screen.mouse;
        if let (Ok(mut shared), Ok(mut gui)) = (self.shared.lock(), self.gui.lock()) {
            gui.on_mouse_move(cursor[0] as f64, cursor[1] as f64);
            let mut env = HeadlessEnv::new();
            gui.on_mouse(&mut shared, &mut env, event.button, false);
        }
        Flow::Cancel
    }

    fn screen_mouse_dragged_pre(
        &self,
        host: &dyn Host,
        event: &mut ScreenMouseDraggedPreEvent<'_>,
    ) -> Flow {
        if host.mod_screen() != Some(MOD_ID) {
            return Flow::Continue;
        }
        if let Ok(mut gui) = self.gui.lock() {
            gui.on_mouse_move(event.screen.mouse[0] as f64, event.screen.mouse[1] as f64);
        }
        Flow::Cancel
    }

    fn screen_mouse_scrolled_pre(
        &self,
        host: &dyn Host,
        event: &mut ScreenMouseScrolledPreEvent<'_>,
    ) -> Flow {
        if host.mod_screen() != Some(MOD_ID) {
            return Flow::Continue;
        }
        if let Ok(mut gui) = self.gui.lock() {
            gui.on_mouse_move(event.screen.mouse[0] as f64, event.screen.mouse[1] as f64);
            gui.scroll(event.scroll[1]);
        }
        Flow::Cancel
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn hooks_constructs() {
        let _ = HorixonHooks::new();
    }
}
