//! `systems.friends.Friends` - the friend (and enemy) list every combat module
//! checks before it aims at a player.

use crate::meteor_core::config::Tag;

/// One entry of `Friends`' list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Friend {
    pub name: String,
    pub uuid: Option<String>,
    /// `Utils.getCurrentTime()` when the entry was added, for the GUI's sort.
    pub created: i64,
}

impl Friend {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            uuid: None,
            created: now_millis(),
        }
    }

    pub fn to_tag(&self) -> Tag {
        let mut t = Tag::compound();
        t.put_str("name", &self.name);
        if let Some(uuid) = &self.uuid {
            t.put_str("uuid", uuid);
        }
        t.put_long("created", self.created);
        t
    }

    pub fn from_tag(tag: &Tag) -> Option<Self> {
        let name = tag.get_str("name");
        if name.is_empty() {
            return None;
        }
        Some(Self {
            name: name.to_string(),
            uuid: if tag.get_str("uuid").is_empty() {
                None
            } else {
                Some(tag.get_str("uuid").to_string())
            },
            created: tag.get_long("created").unwrap_or_else(now_millis),
        })
    }
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// `Friends` - two lists, friends and enemies, both persisted in `friends.nbt`.
#[derive(Clone, Debug, Default)]
pub struct Friends {
    /// `Friend` list, in insertion order.
    pub friends: Vec<Friend>,
    /// `Enemies`. Meteor keeps a separate list with the same shape.
    pub enemies: Vec<Friend>,
}

impl Friends {
    pub fn new() -> Self {
        Self::default()
    }

    /// `Friends.add(Friend)` - returns false when the name is already there.
    pub fn add(&mut self, name: &str) -> bool {
        if self.is_friend(name) {
            return false;
        }
        self.friends.push(Friend::new(name));
        true
    }

    pub fn add_enemy(&mut self, name: &str) -> bool {
        if self.is_enemy(name) {
            return false;
        }
        self.enemies.push(Friend::new(name));
        true
    }

    pub fn remove(&mut self, name: &str) -> bool {
        let before = self.friends.len();
        self.friends.retain(|f| !f.name.eq_ignore_ascii_case(name));
        self.friends.len() != before
    }

    pub fn remove_enemy(&mut self, name: &str) -> bool {
        let before = self.enemies.len();
        self.enemies.retain(|f| !f.name.eq_ignore_ascii_case(name));
        self.enemies.len() != before
    }

    pub fn is_friend(&self, name: &str) -> bool {
        self.friends
            .iter()
            .any(|f| f.name.eq_ignore_ascii_case(name))
    }

    pub fn is_enemy(&self, name: &str) -> bool {
        self.enemies
            .iter()
            .any(|f| f.name.eq_ignore_ascii_case(name))
    }

    /// `Friends.get(String)`.
    pub fn get(&self, name: &str) -> Option<&Friend> {
        self.friends
            .iter()
            .find(|f| f.name.eq_ignore_ascii_case(name))
    }

    pub fn names(&self) -> Vec<String> {
        self.friends.iter().map(|f| f.name.clone()).collect()
    }

    pub fn enemy_names(&self) -> Vec<String> {
        self.enemies.iter().map(|f| f.name.clone()).collect()
    }

    /// `Friends.toTag`.
    pub fn to_tag(&self) -> Tag {
        let mut root = Tag::compound();
        root.put(
            "friends",
            Tag::List(self.friends.iter().map(Friend::to_tag).collect()),
        );
        root.put(
            "enemies",
            Tag::List(self.enemies.iter().map(Friend::to_tag).collect()),
        );
        root
    }

    /// `Friends.fromTag`.
    pub fn from_tag(&mut self, tag: &Tag) {
        self.friends.clear();
        self.enemies.clear();
        for t in tag.get("friends").map(|l| l.as_list()).unwrap_or(&[]) {
            if let Some(f) = Friend::from_tag(t) {
                self.friends.push(f);
            }
        }
        for t in tag.get("enemies").map(|l| l.as_list()).unwrap_or(&[]) {
            if let Some(f) = Friend::from_tag(t) {
                self.enemies.push(f);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_remove_and_case_insensitive_lookup() {
        let mut f = Friends::new();
        assert!(f.add("Notch"));
        assert!(!f.add("notch"), "duplicates are refused");
        assert!(f.is_friend("NOTCH"));
        assert!(f.remove("notch"));
        assert!(!f.is_friend("Notch"));
    }

    #[test]
    fn round_trips_through_a_tag() {
        let mut f = Friends::new();
        f.add("Alice");
        f.add_enemy("Bob");
        let tag = f.to_tag();
        let text = tag.to_snbt();
        let mut back = Friends::new();
        back.from_tag(&Tag::parse_snbt(&text).unwrap());
        assert!(back.is_friend("Alice"));
        assert!(back.is_enemy("Bob"));
        assert_eq!(back.names(), vec!["Alice".to_string()]);
    }
}
