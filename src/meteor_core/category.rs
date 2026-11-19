//! `systems.modules.Category` + `Categories` -?the six built-in categories and
//! their icons.

/// One module category. The Java class is a `(name, icon supplier)` pair; here
/// it is an enum, because the six built-ins are the whole set a port needs and
/// the GUI reads them in this order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Category {
    Combat,
    Player,
    Movement,
    Render,
    World,
    Misc,
}

impl Category {
    /// `Categories.init()`'s registration order, which is also the GUI's
    /// window order.
    pub const ALL: [Category; 6] = [
        Category::Combat,
        Category::Player,
        Category::Movement,
        Category::Render,
        Category::World,
        Category::Misc,
    ];

    /// `Category.name`.
    pub const fn name(self) -> &'static str {
        match self {
            Category::Combat => "Combat",
            Category::Player => "Player",
            Category::Movement => "Movement",
            Category::Render => "Render",
            Category::World => "World",
            Category::Misc => "Misc",
        }
    }

    /// The item whose icon the GUI draws when `category-icons` is on
    /// (`Categories.Combat` etc.).
    pub const fn icon(self) -> &'static str {
        match self {
            Category::Combat => "minecraft:golden_sword",
            Category::Player => "minecraft:armor_stand",
            Category::Movement => "minecraft:diamond_boots",
            Category::Render => "minecraft:glass",
            Category::World => "minecraft:grass_block",
            Category::Misc => "minecraft:lava_bucket",
        }
    }

    pub fn from_name(name: &str) -> Option<Category> {
        Category::ALL
            .into_iter()
            .find(|c| c.name().eq_ignore_ascii_case(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_and_names_match_categories_java() {
        assert_eq!(Category::ALL[0], Category::Combat);
        assert_eq!(Category::ALL[5], Category::Misc);
        assert_eq!(Category::Movement.name(), "Movement");
        assert_eq!(Category::from_name("render"), Some(Category::Render));
        assert_eq!(Category::from_name("nope"), None);
    }
}
