//! `systems.modules.player.FakePlayer` - spawns a client-side fake player for
//! testing usages. No need to be active.
//!
//! PORT GAP: Horizon's mod API cannot add an entity to the client level, so the
//! port keeps the fake player's state (name, health, inventory copy) but has no
//! body to render or hit. The widget's Spawn/Clear buttons are exposed as
//! [`FakePlayer::spawn`] / [`FakePlayer::clear`] and drawn as lines.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::Env;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, IntSet, StringSet};

use super::describe;

#[derive(Clone, Debug, PartialEq)]
pub struct FakePlayerData {
    pub name: String,
    pub health: i32,
    pub copy_inv: bool,
}

pub struct FakePlayer {
    base: ModuleBase,
    name: StringSet,
    copy_inv: BoolSet,
    health: IntSet,
    players: Vec<FakePlayerData>,
}

impl FakePlayer {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Player,
            "fake-player",
            "Spawns a client-side fake player for testing usages. No need to be active.",
        );
        let (name, copy_inv, health);
        {
            let mut s = base.settings_mut();
            let g = s.default_group();
            name = g.string_setting("name", "seasnail8169");
            copy_inv = g.bool_setting("copy-inv", true);
            health = g.int_setting("health", 20, 1, 100);
        }
        let mut s = base.settings_mut();
        describe(&mut s, "name", "The name of the fake player.");
        describe(
            &mut s,
            "copy-inv",
            "Copies your inventory to the fake player.",
        );
        describe(&mut s, "health", "The fake player's default health.");

        Self {
            base,
            name,
            copy_inv,
            health,
            players: Vec::new(),
        }
    }

    /// The widget's Spawn button: `FakePlayerManager.add(name, health, copyInv)`.
    pub fn spawn(&mut self) {
        self.players.push(FakePlayerData {
            name: self.name.get(),
            health: self.health.get(),
            copy_inv: self.copy_inv.get(),
        });
    }

    /// The widget's Clear All button: `FakePlayerManager.clear()`.
    pub fn clear(&mut self) {
        self.players.clear();
    }

    /// The names the widget's table lists.
    pub fn names(&self) -> Vec<String> {
        self.players.iter().map(|p| p.name.clone()).collect()
    }

    pub fn count(&self) -> usize {
        self.players.len()
    }
}

impl Default for FakePlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for FakePlayer {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_deactivate(&mut self, _env: &mut dyn Env) {
        self.clear();
    }

    fn info_string(&self) -> Option<String> {
        Some(self.count().to_string())
    }

    fn extra_widget(&self) -> Option<Vec<String>> {
        let mut lines: Vec<String> = self
            .players
            .iter()
            .map(|p| format!("{} ({} hp)", p.name, p.health))
            .collect();
        lines.push("Spawn: .fake-player spawn".to_string());
        lines.push("Clear All: .fake-player clear".to_string());
        Some(lines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn spawning_counts_and_clearing_empties() {
        let mut m = FakePlayer::new();
        let mut env = TestEnv::in_world();

        assert_eq!(m.info_string().as_deref(), Some("0"));
        m.spawn();
        assert_eq!(m.info_string().as_deref(), Some("1"));
        assert_eq!(m.names(), vec!["seasnail8169".to_string()]);
        assert!(m
            .extra_widget()
            .unwrap()
            .iter()
            .any(|l| l.contains("seasnail8169")));

        m.on_deactivate(&mut env);
        assert_eq!(m.info_string().as_deref(), Some("0"));
    }
}
