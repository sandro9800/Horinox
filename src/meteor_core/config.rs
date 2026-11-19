//! `utils.misc.ISerializable` + the NBT shape Meteor stores everything in.
//!
//! The Java client writes real NBT (`modules.nbt`, `config.nbt`, -). A Horizon
//! mod has no NBT writer in its API, and a port that invented a binary format
//! would be unreadable next to the original, so the port keeps the *tag tree* -
//! same keys, same nesting - and writes it as SNBT text. The tree is identical,
//! which is what matters for fidelity of the stored state; `PORT-MAP.md` records
//! the encoding difference.

use crate::meteor_core::setting::{Setting, Settings, Value};

/// An NBT tag, as much of it as the settings/config layer uses.
#[derive(Clone, Debug, PartialEq)]
pub enum Tag {
    Bool(bool),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    String(String),
    List(Vec<Tag>),
    Compound(Vec<(String, Tag)>),
}

impl Tag {
    pub fn compound() -> Tag {
        Tag::Compound(Vec::new())
    }

    pub fn list() -> Tag {
        Tag::List(Vec::new())
    }

    pub fn put(&mut self, key: &str, value: Tag) {
        if let Tag::Compound(entries) = self {
            if let Some(slot) = entries.iter_mut().find(|(k, _)| k == key) {
                slot.1 = value;
            } else {
                entries.push((key.to_string(), value));
            }
        }
    }

    pub fn put_str(&mut self, key: &str, value: &str) {
        self.put(key, Tag::String(value.to_string()));
    }

    pub fn put_bool(&mut self, key: &str, value: bool) {
        self.put(key, Tag::Bool(value));
    }

    pub fn put_int(&mut self, key: &str, value: i32) {
        self.put(key, Tag::Int(value));
    }

    pub fn put_double(&mut self, key: &str, value: f64) {
        self.put(key, Tag::Double(value));
    }

    pub fn put_long(&mut self, key: &str, value: i64) {
        self.put(key, Tag::Long(value));
    }

    pub fn get(&self, key: &str) -> Option<&Tag> {
        match self {
            Tag::Compound(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn get_str(&self, key: &str) -> &str {
        match self.get(key) {
            Some(Tag::String(s)) => s,
            _ => "",
        }
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        match self.get(key) {
            Some(Tag::Bool(b)) => Some(*b),
            _ => None,
        }
    }

    pub fn get_int(&self, key: &str) -> Option<i32> {
        match self.get(key) {
            Some(Tag::Int(i)) => Some(*i),
            Some(Tag::Long(l)) => Some(*l as i32),
            _ => None,
        }
    }

    pub fn get_long(&self, key: &str) -> Option<i64> {
        match self.get(key) {
            Some(Tag::Long(l)) => Some(*l),
            Some(Tag::Int(i)) => Some(*i as i64),
            _ => None,
        }
    }

    pub fn get_double(&self, key: &str) -> Option<f64> {
        match self.get(key) {
            Some(Tag::Double(d)) => Some(*d),
            Some(Tag::Float(f)) => Some(*f as f64),
            _ => None,
        }
    }

    pub fn as_list(&self) -> &[Tag] {
        match self {
            Tag::List(l) => l,
            _ => &[],
        }
    }

    pub fn as_compound(&self) -> &[(String, Tag)] {
        match self {
            Tag::Compound(c) => c,
            _ => &[],
        }
    }

    /// `CompoundTag.toString()`'s SNBT form, used for `nbt` output and the files.
    pub fn to_snbt(&self) -> String {
        let mut out = String::new();
        self.write_snbt(&mut out);
        out
    }

    fn write_snbt(&self, out: &mut String) {
        match self {
            Tag::Bool(b) => out.push_str(if *b { "1b" } else { "0b" }),
            Tag::Int(i) => out.push_str(&i.to_string()),
            Tag::Long(l) => {
                out.push_str(&l.to_string());
                out.push('L');
            }
            Tag::Float(f) => {
                out.push_str(&format!("{f:?}"));
                out.push('f');
            }
            Tag::Double(d) => out.push_str(&format!("{d:?}")),
            Tag::String(s) => write_quoted(out, s),
            Tag::List(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    item.write_snbt(out);
                }
                out.push(']');
            }
            Tag::Compound(entries) => {
                out.push('{');
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_quoted(out, k);
                    out.push(':');
                    v.write_snbt(out);
                }
                out.push('}');
            }
        }
    }

    /// Parses the SNBT [`to_snbt`](Self::to_snbt) writes.
    pub fn parse_snbt(text: &str) -> Option<Tag> {
        let mut p = SnbtParser {
            chars: text.chars().collect(),
            pos: 0,
        };
        p.skip_ws();
        let tag = p.value()?;
        p.skip_ws();
        if p.pos == p.chars.len() {
            Some(tag)
        } else {
            None
        }
    }
}

fn write_quoted(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            _ => out.push(c),
        }
    }
    out.push('"');
}

struct SnbtParser {
    chars: Vec<char>,
    pos: usize,
}

impl SnbtParser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_whitespace()) {
            self.pos += 1;
        }
    }

    fn value(&mut self) -> Option<Tag> {
        self.skip_ws();
        match self.peek()? {
            '{' => self.compound(),
            '[' => self.list(),
            '"' => self.string().map(Tag::String),
            _ => self.number(),
        }
    }

    fn compound(&mut self) -> Option<Tag> {
        self.pos += 1; // {
        let mut entries = Vec::new();
        loop {
            self.skip_ws();
            match self.peek()? {
                '}' => {
                    self.pos += 1;
                    return Some(Tag::Compound(entries));
                }
                _ => {}
            }
            let key = self.string()?;
            self.skip_ws();
            if self.peek()? != ':' {
                return None;
            }
            self.pos += 1;
            let value = self.value()?;
            entries.push((key, value));
            self.skip_ws();
            if self.peek()? == ',' {
                self.pos += 1;
            }
        }
    }

    fn list(&mut self) -> Option<Tag> {
        self.pos += 1; // [
        let mut items = Vec::new();
        loop {
            self.skip_ws();
            if self.peek()? == ']' {
                self.pos += 1;
                return Some(Tag::List(items));
            }
            items.push(self.value()?);
            self.skip_ws();
            if self.peek()? == ',' {
                self.pos += 1;
            }
        }
    }

    fn string(&mut self) -> Option<String> {
        if self.peek() != Some('"') {
            // Unquoted keys are legal SNBT; read to the next delimiter.
            let mut s = String::new();
            while let Some(c) = self.peek() {
                if c == ':' || c == ',' || c == '}' || c == ']' || c.is_whitespace() {
                    break;
                }
                s.push(c);
                self.pos += 1;
            }
            return if s.is_empty() { None } else { Some(s) };
        }
        self.pos += 1;
        let mut s = String::new();
        while let Some(c) = self.peek() {
            self.pos += 1;
            match c {
                '"' => return Some(s),
                '\\' => {
                    let esc = self.peek()?;
                    self.pos += 1;
                    s.push(match esc {
                        'n' => '\n',
                        't' => '\t',
                        other => other,
                    });
                }
                other => s.push(other),
            }
        }
        None
    }

    fn number(&mut self) -> Option<Tag> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit()
                || c == '-'
                || c == '+'
                || c == '.'
                || c == 'e'
                || c == 'E'
                || c == 'b'
                || c == 'B'
                || c == 'f'
                || c == 'F'
                || c == 'd'
                || c == 'D'
                || c == 'L'
                || c == 'l'
                || c == 's'
            {
                self.pos += 1;
            } else {
                break;
            }
        }
        let raw: String = self.chars[start..self.pos].iter().collect();
        if raw.is_empty() {
            return None;
        }
        let last = raw.chars().last().unwrap();
        match last {
            'b' | 'B' => parse_bool(&raw[..raw.len() - 1]).map(Tag::Bool),
            'f' | 'F' => raw[..raw.len() - 1].parse::<f32>().ok().map(Tag::Float),
            'd' | 'D' => raw[..raw.len() - 1].parse::<f64>().ok().map(Tag::Double),
            'l' | 'L' => raw[..raw.len() - 1].parse::<i64>().ok().map(Tag::Long),
            's' => raw[..raw.len() - 1].parse::<i32>().ok().map(Tag::Int),
            _ => {
                if raw.contains('.') || raw.contains('e') || raw.contains('E') {
                    raw.parse::<f64>().ok().map(Tag::Double)
                } else {
                    raw.parse::<i32>().ok().map(Tag::Int)
                }
            }
        }
    }
}

fn parse_bool(s: &str) -> Option<bool> {
    match s.trim() {
        "1" | "true" => Some(true),
        "0" | "false" => Some(false),
        _ => None,
    }
}

/// `Settings.toTag()` - only groups that were changed, as Java does.
pub fn settings_to_tag(settings: &Settings) -> Tag {
    let mut groups = Vec::new();
    for group in &settings.groups {
        if !group.was_changed() {
            continue;
        }
        let mut g = Tag::compound();
        g.put_str("name", &group.name);
        g.put_bool("sectionExpanded", group.section_expanded);
        let mut list = Vec::new();
        for s in &group.settings {
            if !s.was_changed() {
                continue;
            }
            let mut t = Tag::compound();
            t.put_str("name", &s.name);
            if let Some(v) = setting_value_to_tag(s) {
                t.put("value", v);
            }
            list.push(t);
        }
        if !list.is_empty() {
            g.put("settings", Tag::List(list));
        }
        groups.push(g);
    }
    let mut root = Tag::compound();
    if !groups.is_empty() {
        root.put("groups", Tag::List(groups));
    }
    root
}

fn setting_value_to_tag(setting: &Setting) -> Option<Tag> {
    Some(match &setting.value {
        Value::Bool(c) => Tag::Bool(c.load(std::sync::atomic::Ordering::Relaxed)),
        Value::Int { cell, .. } => Tag::Int(cell.load(std::sync::atomic::Ordering::Relaxed)),
        Value::Double { cell, .. } => Tag::Double(f64::from_bits(
            cell.load(std::sync::atomic::Ordering::Relaxed) as u64,
        )),
        Value::String(c) => Tag::String(c.lock().unwrap().clone()),
        Value::Enum { cell, names } => Tag::String(
            names
                .get(cell.load(std::sync::atomic::Ordering::Relaxed) as usize)
                .copied()
                .unwrap_or("")
                .to_string(),
        ),
        Value::Color(c) => Tag::Int(c.load(std::sync::atomic::Ordering::Relaxed) as i32),
        Value::Keybind(c) => {
            let kb = *c.lock().unwrap();
            let mut t = Tag::compound();
            t.put_bool("isKey", kb.is_key);
            t.put_int("value", kb.value);
            t.put_int("modifiers", kb.modifiers as i32);
            t
        }
        Value::List(c) => Tag::List(
            c.lock()
                .unwrap()
                .iter()
                .map(|s| Tag::String(s.clone()))
                .collect(),
        ),
        Value::ItemList(c) => Tag::List(
            c.lock()
                .unwrap()
                .iter()
                .map(|s| Tag::String(s.clone()))
                .collect(),
        ),
    })
}

/// `Settings.fromTag()`: resets first, then applies every stored value.
pub fn settings_from_tag(settings: &mut Settings, tag: &Tag) {
    settings.reset();
    let groups = tag.get("groups").map(|g| g.as_list()).unwrap_or(&[]);
    for group_tag in groups {
        let name = group_tag.get_str("name").to_string();
        let Some(group) = settings.groups.iter_mut().find(|g| g.name == name) else {
            continue;
        };
        if let Some(expanded) = group_tag.get_bool("sectionExpanded") {
            group.section_expanded = expanded;
        }
        let items = group_tag
            .get("settings")
            .map(|s| s.as_list())
            .unwrap_or(&[]);
        for setting_tag in items {
            let setting_name = setting_tag.get_str("name").to_string();
            let Some(setting) = group.settings.iter().find(|s| s.name == setting_name) else {
                continue;
            };
            if let Some(value) = setting_tag.get("value") {
                apply_setting_tag(setting, value);
            }
        }
    }
}

fn apply_setting_tag(setting: &Setting, tag: &Tag) {
    use std::sync::atomic::Ordering;
    match (&setting.value, tag) {
        (Value::Bool(cell), Tag::Bool(b)) => cell.store(*b, Ordering::Relaxed),
        (Value::Bool(cell), Tag::Int(i)) => cell.store(*i != 0, Ordering::Relaxed),
        (Value::Int { cell, .. }, Tag::Int(i)) => cell.store(*i, Ordering::Relaxed),
        (Value::Int { cell, .. }, Tag::Long(l)) => cell.store(*l as i32, Ordering::Relaxed),
        (Value::Double { cell, .. }, Tag::Double(d)) => {
            cell.store(d.to_bits() as i64, Ordering::Relaxed)
        }
        (Value::Double { cell, .. }, Tag::Float(f)) => {
            cell.store((*f as f64).to_bits() as i64, Ordering::Relaxed)
        }
        (Value::String(cell), Tag::String(s)) => *cell.lock().unwrap() = s.clone(),
        (Value::Enum { cell, names }, Tag::String(s)) => {
            if let Some(i) = names.iter().position(|n| n == s) {
                cell.store(i as u32, Ordering::Relaxed);
            }
        }
        (Value::Enum { cell, names }, Tag::Int(i)) => {
            if (*i as usize) < names.len() {
                cell.store(*i as u32, Ordering::Relaxed);
            }
        }
        (Value::Color(cell), Tag::Int(i)) => cell.store(*i as u32, Ordering::Relaxed),
        (Value::Keybind(cell), Tag::Compound(_)) => {
            let kb = crate::meteor_core::manager::keybind_from_tag(tag);
            *cell.lock().unwrap() = kb;
        }
        (Value::List(cell), Tag::List(items)) => {
            *cell.lock().unwrap() = items
                .iter()
                .filter_map(|t| match t {
                    Tag::String(s) => Some(s.clone()),
                    _ => None,
                })
                .collect();
        }
        (Value::ItemList(cell), Tag::List(items)) => {
            *cell.lock().unwrap() = items
                .iter()
                .filter_map(|t| match t {
                    Tag::String(s) => Some(s.clone()),
                    _ => None,
                })
                .collect();
        }
        _ => {}
    }
}

/// A `System<T>`'s file: `name.nbt` holding one tag tree, written as SNBT.
pub struct ConfigFile {
    pub name: String,
    pub tag: Tag,
}

impl ConfigFile {
    pub fn new(name: &str, tag: Tag) -> Self {
        Self {
            name: name.to_string(),
            tag,
        }
    }

    /// `System.save(File folder)`.
    pub fn save(&self, folder: &std::path::Path) -> std::io::Result<()> {
        std::fs::create_dir_all(folder)?;
        let path = folder.join(format!("{}.nbt", self.name));
        let tmp = folder.join(format!("{}.nbt.tmp", self.name));
        std::fs::write(&tmp, self.tag.to_snbt())?;
        std::fs::rename(tmp, path)
    }

    /// `System.load(File folder)`; a missing or unreadable file loads nothing,
    /// as Java logs and carries on.
    pub fn load(folder: &std::path::Path, name: &str) -> Option<Tag> {
        let path = folder.join(format!("{}.nbt", name));
        let text = std::fs::read_to_string(path).ok()?;
        Tag::parse_snbt(&text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::setting::Settings;

    #[test]
    fn snbt_round_trip() {
        let mut t = Tag::compound();
        t.put_str("name", "kill-aura");
        t.put_bool("active", true);
        t.put_int("count", -3);
        t.put_double("range", 4.25);
        let mut inner = Tag::compound();
        inner.put_str("k", "v");
        t.put("nested", inner);
        t.put(
            "list",
            Tag::List(vec![Tag::String("a".into()), Tag::String("b".into())]),
        );
        let text = t.to_snbt();
        let back = Tag::parse_snbt(&text).expect("parses");
        assert_eq!(back.get_str("name"), "kill-aura");
        assert_eq!(back.get_bool("active"), Some(true));
        assert_eq!(back.get_int("count"), Some(-3));
        assert_eq!(back.get_double("range"), Some(4.25));
        assert_eq!(back.get("list").unwrap().as_list().len(), 2);
    }

    #[test]
    fn settings_round_trip_through_tags() {
        let mut s = Settings::new();
        let b = s.default_group().bool_setting("enabled", false);
        let i = s.create_group("Ranges").int_setting("delay", 2, 0, 10);
        let e = s
            .default_group()
            .enum_setting("mode", &["vanilla", "packet"], 0);
        b.set(true);
        i.set(7);
        e.set_index(1);

        let tag = settings_to_tag(&s);
        let text = tag.to_snbt();
        let parsed = Tag::parse_snbt(&text).unwrap();

        let mut restored = Settings::new();
        let b2 = restored.default_group().bool_setting("enabled", false);
        let i2 = restored
            .create_group("Ranges")
            .int_setting("delay", 2, 0, 10);
        let e2 = restored
            .default_group()
            .enum_setting("mode", &["vanilla", "packet"], 0);
        settings_from_tag(&mut restored, &parsed);
        assert!(b2.get());
        assert_eq!(i2.get(), 7);
        assert_eq!(e2.index(), 1);
    }

    #[test]
    fn unchanged_settings_are_not_written() {
        let mut s = Settings::new();
        let _ = s.default_group().bool_setting("untouched", false);
        let tag = settings_to_tag(&s);
        assert!(tag.get("groups").is_none());
    }
}
