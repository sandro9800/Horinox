//! `systems.modules.Modules` -?the registry, the active set, bind handling, the
//! tick/render dispatch and the search the GUI uses.

use std::collections::BTreeSet;

use crate::meteor_core::category::Category;
use crate::meteor_core::config::Tag;
use crate::meteor_core::env::{Env, PacketInfo, Renderer};
use crate::meteor_core::math::Rotation;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::util::{search_score, ChatColor, ChatMsg, Keybind, Prefix};

/// The module registry. One instance lives for the process, owned by the mod.
pub struct Modules {
    modules: Vec<Box<dyn Module>>,
    active: Vec<bool>,
    /// Index lists per category, in `Category::ALL` order.
    groups: Vec<Vec<usize>>,
    /// `Modules.moduleToBind`.
    binding: Option<usize>,
    awaiting_key_release: bool,
    /// Modules that were switched on when the game was left, so a rejoin can
    /// restore them (`Modules.onGameJoined`).
    suspended: Vec<usize>,
}

impl Default for Modules {
    fn default() -> Self {
        Self::new()
    }
}

impl Modules {
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
            active: Vec::new(),
            groups: vec![Vec::new(); Category::ALL.len()],
            binding: None,
            awaiting_key_release: false,
            suspended: Vec::new(),
        }
    }

    /// `Modules.add(Module)`. A second module with the same name replaces the
    /// first, as the Java registry does for addons that override a module.
    pub fn add(&mut self, module: Box<dyn Module>) {
        let name = module.base().name;
        if let Some(i) = self.index_of(name) {
            let category = self.modules[i].base().category;
            self.modules[i] = module;
            self.active[i] = false;
            let slot = category_index(category);
            if !self.groups[slot].contains(&i) {
                self.groups[slot].push(i);
            }
            return;
        }
        let category = module.base().category;
        let i = self.modules.len();
        self.modules.push(module);
        self.active.push(false);
        self.groups[category_index(category)].push(i);
    }

    /// Adds and returns the module's index, for a mod that wires handles.
    pub fn add_counted(&mut self, module: Box<dyn Module>) -> usize {
        self.add(module);
        self.modules.len() - 1
    }

    fn index_of(&self, name: &str) -> Option<usize> {
        self.modules
            .iter()
            .position(|m| m.base().name == name)
            // Names are matched case-insensitively like `Modules.get(String)`.
            .or_else(|| {
                self.modules
                    .iter()
                    .position(|m| m.base().name.eq_ignore_ascii_case(name))
            })
    }

    /// `Modules.sortModules()`: titles sorted per category.
    pub fn sort(&mut self) {
        for g in self.groups.iter_mut() {
            g.sort_by(|a, b| {
                self.modules[*a]
                    .base()
                    .title
                    .cmp(&self.modules[*b].base().title)
            });
        }
    }

    pub fn count(&self) -> usize {
        self.modules.len()
    }

    pub fn get(&self, name: &str) -> Option<&dyn Module> {
        self.index_of(name).map(|i| &*self.modules[i])
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut dyn Module> {
        match self.index_of(name) {
            Some(i) => Some(&mut *self.modules[i]),
            None => None,
        }
    }

    pub fn base(&self, name: &str) -> Option<&ModuleBase> {
        self.get(name).map(|m| m.base())
    }

    pub fn base_mut(&mut self, name: &str) -> Option<&mut ModuleBase> {
        self.get_mut(name).map(|m| m.base_mut())
    }

    pub fn all(&self) -> impl Iterator<Item = &dyn Module> {
        self.modules.iter().map(|m| &**m)
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.modules.iter().map(|m| m.base().name).collect()
    }

    pub fn group(&self, category: Category) -> Vec<&dyn Module> {
        self.groups[category_index(category)]
            .iter()
            .map(|i| &*self.modules[*i])
            .collect()
    }

    /// The modules of a category, as `(index, module)` for the GUI, which needs
    /// the index to toggle without borrowing the registry twice.
    pub fn group_indices(&self, category: Category) -> Vec<usize> {
        self.groups[category_index(category)].clone()
    }

    pub fn module_at(&self, index: usize) -> Option<&dyn Module> {
        self.modules.get(index).map(|m| &**m)
    }

    pub fn module_at_mut(&mut self, index: usize) -> Option<&mut dyn Module> {
        match self.modules.get_mut(index) {
            Some(m) => Some(&mut **m),
            None => None,
        }
    }

    pub fn is_active(&self, name: &str) -> bool {
        self.index_of(name).map(|i| self.active[i]).unwrap_or(false)
    }

    pub fn is_active_index(&self, index: usize) -> bool {
        self.active.get(index).copied().unwrap_or(false)
    }

    /// `Modules.getActive()`.
    pub fn active_names(&self) -> Vec<&'static str> {
        self.modules
            .iter()
            .zip(self.active.iter())
            .filter(|(_, a)| **a)
            .map(|(m, _)| m.base().name)
            .collect()
    }

    pub fn active_count(&self) -> usize {
        self.active.iter().filter(|a| **a).count()
    }

    // ---- toggling ---------------------------------------------------------

    fn can_run(&self, i: usize, env: &dyn Env) -> bool {
        self.modules[i].base().run_in_main_menu || env.can_update()
    }

    /// `Module.toggle()`.
    pub fn toggle_index(&mut self, i: usize, env: &mut dyn Env) {
        if i >= self.modules.len() {
            return;
        }
        if !self.active[i] {
            self.active[i] = true;
            self.modules[i].base_mut().settings.reset();
            if self.can_run(i, env) {
                self.modules[i].on_activate(env);
            }
        } else {
            if self.can_run(i, env) {
                self.modules[i].on_deactivate(env);
            }
            self.active[i] = false;
        }
    }

    pub fn toggle(&mut self, name: &str, env: &mut dyn Env) -> bool {
        match self.index_of(name) {
            Some(i) => {
                self.toggle_index(i, env);
                true
            }
            None => false,
        }
    }

    /// Toggles by name and prints the toggled message, for callers that only have
    /// a name (the `toggle` command's queue, the keybind flow).
    pub fn toggle_with_message_by_name(
        &mut self,
        name: &str,
        env: &mut dyn Env,
        feedback_enabled: bool,
    ) -> bool {
        match self.index_of(name) {
            Some(i) => {
                self.toggle_with_message(i, env, feedback_enabled);
                true
            }
            None => false,
        }
    }

    /// Toggles and prints `Module.sendToggledMsg()`.
    pub fn toggle_with_message(&mut self, i: usize, env: &mut dyn Env, feedback_enabled: bool) {
        let chat_feedback = self
            .modules
            .get(i)
            .map(|m| m.base().chat_feedback && m.base().serialize)
            .unwrap_or(false);
        self.toggle_index(i, env);
        if feedback_enabled && chat_feedback {
            let title = self.modules[i].base().title.clone();
            let on = self.active[i];
            let state = if on {
                format!("{}on", ChatColor::Green.code())
            } else {
                format!("{}off", ChatColor::Red.code())
            };
            let msg = ChatMsg::new(
                Prefix::Module(title),
                "Toggled (highlight)%s(default) %s(default).",
                &["", ""],
            );
            // The template is filled by hand: the two arguments carry the module
            // title and the coloured state, matching the Java format string.
            let text = msg
                .text
                .replace("%s", "")
                .replacen(
                    "Toggled ",
                    &format!("Toggled {} ", self.modules[i].base().title),
                    1,
                )
                .replace("  ", &format!("{} ", state))
                .trim_end()
                .to_string();
            env.chat(&text);
        }
    }

    pub fn enable(&mut self, name: &str, env: &mut dyn Env) {
        if let Some(i) = self.index_of(name) {
            if !self.active[i] {
                self.toggle_index(i, env);
            }
        }
    }

    pub fn disable(&mut self, name: &str, env: &mut dyn Env) {
        if let Some(i) = self.index_of(name) {
            if self.active[i] {
                self.toggle_index(i, env);
            }
        }
    }

    /// `Module.enable()` / `.disable()` by index, for the GUI.
    pub fn set_active(&mut self, i: usize, on: bool, env: &mut dyn Env) {
        if i < self.modules.len() && self.active[i] != on {
            self.toggle_index(i, env);
        }
    }

    pub fn disable_all(&mut self, env: &mut dyn Env) {
        for i in 0..self.modules.len() {
            if self.active[i] {
                self.toggle_index(i, env);
            }
        }
    }

    // ---- dispatch ---------------------------------------------------------

    /// Client tick: every active module's `on_tick`, then the player tick.
    pub fn tick(&mut self, env: &mut dyn Env) {
        for i in 0..self.modules.len() {
            if !self.active[i] || !self.can_run(i, env) {
                continue;
            }
            self.modules[i].on_tick(env);
            self.modules[i].on_player_tick(env);
        }
    }

    pub fn render_world(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        for i in 0..self.modules.len() {
            if self.active[i] && self.can_run(i, env) {
                self.modules[i].on_render_world(env, r);
            }
        }
    }

    pub fn render_2d(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        for i in 0..self.modules.len() {
            if self.active[i] && self.can_run(i, env) {
                self.modules[i].on_render_2d(env, r);
            }
        }
    }

    pub fn render_hud(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        for i in 0..self.modules.len() {
            if self.active[i] && self.can_run(i, env) {
                self.modules[i].on_render_hud(env, r);
            }
        }
    }

    /// Returns false when a module cancelled the packet.
    pub fn receive_packet(&mut self, env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        let mut allowed = true;
        for i in 0..self.modules.len() {
            if self.active[i] && self.can_run(i, env) {
                allowed &= self.modules[i].on_receive_packet(env, packet);
            }
        }
        allowed
    }

    pub fn send_packet(&mut self, env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        let mut allowed = true;
        for i in 0..self.modules.len() {
            if self.active[i] && self.can_run(i, env) {
                allowed &= self.modules[i].on_send_packet(env, packet);
            }
        }
        allowed
    }

    /// The rotation the highest-priority active module wants, if any.
    pub fn wanted_rotation(&self) -> Option<Rotation> {
        self.modules
            .iter()
            .zip(self.active.iter())
            .filter(|(_, a)| **a)
            .find_map(|(m, _)| m.wanted_rotation())
    }

    pub fn on_game_joined(&mut self, env: &mut dyn Env) {
        let suspended = std::mem::take(&mut self.suspended);
        for i in suspended {
            if i < self.modules.len() && !self.active[i] {
                self.active[i] = true;
            }
        }
        for i in 0..self.modules.len() {
            if self.active[i] && !self.modules[i].base().run_in_main_menu {
                self.modules[i].on_game_joined(env);
            }
        }
    }

    pub fn on_game_left(&mut self, env: &mut dyn Env) {
        self.suspended.clear();
        for i in 0..self.modules.len() {
            if self.active[i] && !self.modules[i].base().run_in_main_menu {
                self.modules[i].on_game_left(env);
                self.suspended.push(i);
            }
        }
    }

    // ---- binds ------------------------------------------------------------

    pub fn start_binding(&mut self, i: usize) {
        self.binding = Some(i);
        self.awaiting_key_release = true;
    }

    pub fn is_binding(&self) -> bool {
        self.binding.is_some()
    }

    pub fn binding_module(&self) -> Option<usize> {
        self.binding
    }

    pub fn cancel_binding(&mut self) {
        self.binding = None;
        self.awaiting_key_release = false;
    }

    /// `Modules.onAction` -?a key or button changed state.
    ///
    /// Returns true when the event was consumed by bind handling (the `bind`
    /// command's "press a key" mode).
    pub fn on_input(
        &mut self,
        env: &mut dyn Env,
        is_key: bool,
        value: i32,
        pressed: bool,
        modifiers: u8,
        feedback_enabled: bool,
    ) -> bool {
        // Bind mode: the next release binds the module.
        if let Some(i) = self.binding {
            if !pressed {
                if self.awaiting_key_release {
                    // Ignore the release of the key that started the bind.
                    if !is_key || (value != 257 && value != 335) {
                        return false;
                    }
                    self.awaiting_key_release = false;
                    return false;
                }
                let kb = if value == 256 {
                    Keybind::none()
                } else {
                    Keybind {
                        is_key,
                        value,
                        modifiers,
                    }
                };
                let title = self.modules[i].base().title.clone();
                self.modules[i].base_mut().keybind = kb;
                let msg = if kb.is_set() {
                    format!("Bound to {}.", kb.to_string())
                } else {
                    "Removed bind.".to_string()
                };
                env.chat(&ChatMsg::new(Prefix::Module(title), &msg, &[]).text);
                self.binding = None;
                return true;
            }
            return false;
        }

        if env.screen() != crate::meteor_core::env::Screen::Playing {
            return false;
        }

        // A bind with modifiers wins over the plain key, as in `Modules.onAction`.
        let modifier_matched = self.modules.iter().any(|m| {
            let kb = m.base().keybind;
            kb.has_mods() && kb.matches(is_key, value, modifiers)
        });

        for i in 0..self.modules.len() {
            let kb = self.modules[i].base().keybind;
            if modifier_matched && !kb.has_mods() {
                continue;
            }
            if kb.matches(is_key, value, modifiers)
                && (pressed || (self.modules[i].base().toggle_on_bind_release && self.active[i]))
            {
                self.toggle_with_message(i, env, feedback_enabled);
            }
        }
        false
    }

    /// Modules that toggle when the mouse press is released (`OpenScreenEvent`
    /// handling in Java toggles them off when a screen opens).
    pub fn on_screen_opened(&mut self, env: &mut dyn Env, feedback_enabled: bool) {
        if !env.can_update() {
            return;
        }
        for i in 0..self.modules.len() {
            if self.modules[i].base().toggle_on_bind_release && self.active[i] {
                self.toggle_with_message(i, env, feedback_enabled);
            }
        }
    }

    // ---- search -----------------------------------------------------------

    /// `Modules.searchTitles`.
    pub fn search(&self, query: &str) -> Vec<(&'static str, String)> {
        let mut hits: Vec<(usize, &'static str, String)> = self
            .modules
            .iter()
            .map(|m| {
                let base = m.base();
                let mut best = (search_score(base.title.as_str(), query), base.name);
                let mut shown = base.title.clone();
                for alias in base.aliases {
                    let score = search_score(alias, query);
                    if score < best.0 {
                        best = (score, base.name);
                        shown = format!("{} ({})", base.title, alias);
                    }
                }
                (best.0, best.1, shown)
            })
            .collect();
        hits.sort_by_key(|(score, name, _)| (*score, *name));
        hits.into_iter().map(|(_, n, t)| (n, t)).collect()
    }

    /// `Modules.searchSettingTitles`.
    pub fn search_settings(&self, query: &str) -> Vec<&'static str> {
        let mut out: Vec<(usize, &'static str)> = self
            .modules
            .iter()
            .map(|m| {
                let lowest = m
                    .base()
                    .settings
                    .iter()
                    .map(|s| search_score(&s.title, query))
                    .min()
                    .unwrap_or(usize::MAX);
                (lowest, m.base().name)
            })
            .collect();
        out.sort_by_key(|(score, name)| (*score, *name));
        out.into_iter().map(|(_, n)| n).collect()
    }

    // ---- serialization ----------------------------------------------------

    /// `Modules.toTag()`.
    pub fn to_tag(&self) -> Tag {
        let mut list = Vec::new();
        for (i, m) in self.modules.iter().enumerate() {
            if !m.base().serialize {
                continue;
            }
            let mut t = Tag::compound();
            t.put_str("name", m.base().name);
            t.put("keybind", keybind_tag(m.base().keybind));
            t.put_bool("toggleOnKeyRelease", m.base().toggle_on_bind_release);
            t.put_bool("chatFeedback", m.base().chat_feedback);
            t.put_bool("favorite", m.base().favorite);
            t.put(
                "settings",
                crate::meteor_core::config::settings_to_tag(&m.base().settings),
            );
            t.put_bool("active", self.active[i]);
            list.push(t);
        }
        let mut root = Tag::compound();
        root.put("modules", Tag::List(list));
        root
    }

    /// `Modules.fromTag()`: disables everything, then applies the file.
    pub fn from_tag(&mut self, tag: &Tag, env: &mut dyn Env) {
        self.disable_all(env);
        let items = match tag.get("modules") {
            Some(Tag::List(l)) => l,
            _ => return,
        };
        for item in items {
            if matches!(item, Tag::Compound(_)) {
                let t: &Tag = item;
                let name = t.get_str("name").to_string();
                let Some(i) = self.index_of(&name) else {
                    continue;
                };
                if let Some(kb) = t.get("keybind") {
                    self.modules[i].base_mut().keybind = keybind_from_tag(kb);
                }
                self.modules[i].base_mut().toggle_on_bind_release =
                    t.get_bool("toggleOnKeyRelease").unwrap_or(false);
                self.modules[i].base_mut().chat_feedback =
                    t.get_bool("chatFeedback").unwrap_or(true);
                self.modules[i].base_mut().favorite = t.get_bool("favorite").unwrap_or(false);
                if let Some(settings) = t.get("settings") {
                    crate::meteor_core::config::settings_from_tag(
                        &mut self.modules[i].base_mut().settings,
                        settings,
                    );
                }
                let active = t.get_bool("active").unwrap_or(false);
                if active != self.active[i] {
                    self.toggle_index(i, env);
                }
            }
        }
    }

    /// Module indices in `Category::ALL` order, for the GUI's windows.
    pub fn category_order(&self) -> Vec<Category> {
        Category::ALL.to_vec()
    }

    /// Names of the modules whose keybind is set, for `binds` and `BindsHud`.
    pub fn active_bindings(&self) -> Vec<(String, String)> {
        let mut out: BTreeSet<(String, String)> = BTreeSet::new();
        for m in &self.modules {
            if m.base().keybind.is_set() {
                out.insert((m.base().keybind.to_string(), m.base().title.clone()));
            }
        }
        out.into_iter().collect()
    }
}

fn category_index(c: Category) -> usize {
    Category::ALL.iter().position(|x| *x == c).unwrap_or(0)
}

pub fn keybind_tag(kb: Keybind) -> Tag {
    let mut t = Tag::compound();
    t.put_bool("isKey", kb.is_key);
    t.put_int("value", kb.value);
    t.put_int("modifiers", kb.modifiers as i32);
    t
}

pub fn keybind_from_tag(tag: &Tag) -> Keybind {
    Keybind {
        is_key: tag.get_bool("isKey").unwrap_or(false),
        value: tag.get_int("value").unwrap_or(-1),
        modifiers: tag.get_int("modifiers").unwrap_or(0) as u8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::module::tests::Counter;

    fn registry() -> Modules {
        let mut m = Modules::new();
        m.add(Box::new(Counter::new()));
        m
    }

    #[test]
    fn add_and_lookup() {
        let m = registry();
        assert_eq!(m.count(), 1);
        assert!(m.get("counter").is_some());
        assert!(m.get("COUNTER").is_some(), "lookup is case-insensitive");
        assert!(m.get("nope").is_none());
        assert_eq!(m.group(Category::Misc).len(), 1);
        assert_eq!(m.group(Category::Combat).len(), 0);
    }

    #[test]
    fn toggle_calls_lifecycle_and_ticks_only_when_active() {
        let mut m = registry();
        let mut env = TestEnv::in_world();
        m.tick(&mut env);
        let i = m.group_indices(Category::Misc)[0];
        {
            let c = m.module_at(i).unwrap();
            assert_eq!(c.info_string().as_deref(), Some("0"));
        }
        m.toggle_index(i, &mut env);
        m.tick(&mut env);
        m.tick(&mut env);
        assert!(m.is_active("counter"));
        assert_eq!(m.module_at(i).unwrap().info_string().as_deref(), Some("2"));
    }

    #[test]
    fn bind_flow_consumes_the_release() {
        let mut m = registry();
        let mut env = TestEnv::in_world();
        let i = m.group_indices(Category::Misc)[0];
        m.start_binding(i);
        // The release of the key that started the bind is ignored.
        assert!(!m.on_input(&mut env, true, 257, false, 0, false));
        // Then the binding release binds the module.
        assert!(m.on_input(&mut env, true, 'G' as i32, false, 0, false));
        assert_eq!(m.base("counter").unwrap().keybind, Keybind::key('G' as i32));
    }

    #[test]
    fn bind_toggles_the_module() {
        let mut m = registry();
        let mut env = TestEnv::in_world();
        let i = m.group_indices(Category::Misc)[0];
        m.base_mut("counter").unwrap().keybind = Keybind::key('K' as i32);
        m.on_input(&mut env, true, 'K' as i32, true, 0, false);
        assert!(m.is_active_index(i));
    }

    #[test]
    fn search_orders_by_score() {
        let mut m = Modules::new();
        struct M(ModuleBase);
        impl Module for M {
            fn base(&self) -> &ModuleBase {
                &self.0
            }
            fn base_mut(&mut self) -> &mut ModuleBase {
                &mut self.0
            }
        }
        m.add(Box::new(M(ModuleBase::new(
            Category::Combat,
            "kill-aura",
            "",
        )
        .with_aliases(&["ka"]))));
        m.add(Box::new(M(ModuleBase::new(Category::Render, "esp", ""))));
        let hits = m.search("kill");
        assert_eq!(hits.first().unwrap().0, "kill-aura");
        let alias_hits = m.search("ka");
        assert_eq!(alias_hits.first().unwrap().0, "kill-aura");
        assert!(alias_hits.first().unwrap().1.contains("(ka)"));
    }

    #[test]
    fn serialization_round_trip() {
        let mut m = registry();
        let mut env = TestEnv::in_world();
        let i = m.group_indices(Category::Misc)[0];
        m.base_mut("counter").unwrap().keybind = Keybind::key('B' as i32);
        m.base_mut("counter").unwrap().favorite = true;
        m.toggle_index(i, &mut env);
        let tag = m.to_tag();

        let mut restored = registry();
        restored.from_tag(&tag, &mut env);
        assert!(restored.is_active("counter"));
        assert_eq!(
            restored.base("counter").unwrap().keybind,
            Keybind::key('B' as i32)
        );
        assert!(restored.base("counter").unwrap().favorite);
    }
}
