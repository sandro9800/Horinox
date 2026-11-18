//! `addons.MeteorAddon` / `AddonManager` - the addon seam.
//!
//! In Java an addon registers categories and modules from `onRegisterCategories`
//! / `onInit`. Horizon mods are linked into one executable (`hooks()` returns the
//! trait object), so the seam stays as a registry: the port's own addon plus any
//! later one register here, and `Modules`/`Categories` read the result.

use crate::meteor_core::category::Category;

/// A Meteor addon (`MeteorAddon`).
pub trait Addon: Send + Sync {
    /// `MeteorAddon.name`.
    fn name(&self) -> &'static str;
    /// `MeteorAddon.getPackage()` - the Java package prefix its modules live in;
    /// empty for the built-in `meteordevelopment.meteorclient` addon.
    fn package(&self) -> &'static str {
        ""
    }
    /// `MeteorAddon.onRegisterCategories()` - extra categories to register.
    fn extra_categories(&self) -> Vec<Category> {
        Vec::new()
    }
    /// `MeteorAddon.onInit()`.
    fn init(&mut self) {}
}

/// The built-in addon: Meteor's own modules.
pub struct MeteorAddon;

impl Addon for MeteorAddon {
    fn name(&self) -> &'static str {
        "HORIXON"
    }
    fn package(&self) -> &'static str {
        "meteordevelopment.meteorclient"
    }
}

/// `AddonManager`.
pub struct AddonManager {
    addons: Vec<Box<dyn Addon>>,
}

impl Default for AddonManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AddonManager {
    pub fn new() -> Self {
        Self {
            addons: vec![Box::new(MeteorAddon)],
        }
    }

    /// `AddonManager.init()`.
    pub fn init(&mut self) {
        for addon in self.addons.iter_mut() {
            addon.init();
        }
    }

    pub fn add(&mut self, addon: Box<dyn Addon>) {
        self.addons.push(addon);
    }

    pub fn all(&self) -> impl Iterator<Item = &dyn Addon> {
        self.addons.iter().map(|a| &**a)
    }

    /// The categories every addon registered, after the six built-ins.
    pub fn extra_categories(&self) -> Vec<Category> {
        let mut out = Vec::new();
        for addon in &self.addons {
            for c in addon.extra_categories() {
                if !out.contains(&c) {
                    out.push(c);
                }
            }
        }
        out
    }
}
