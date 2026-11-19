//! Small shared helpers: keybinds, title formatting, chat formatting, fuzzy
//! search (`utils.Utils.nameToTitle`, `utils.misc.input.Input`,
//! `utils.misc.Keybind`, `utils.player.ChatUtils`).

use crate::meteor_core::color::Color;

/// `Utils.nameToTitle("auto-totem") == "Auto Totem"`.
pub fn name_to_title(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut upper = true;
    for c in name.chars() {
        if c == '-' || c == '_' || c == ' ' {
            out.push(' ');
            upper = true;
        } else if upper {
            out.extend(c.to_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// `Utils.searchLevenshteinDefault(text, query, ignoreCase)` - the distance the
/// module search box sorts by, plus the "starts with" bonus the GUI relies on
/// staying at the top.
pub fn search_score(text: &str, query: &str) -> usize {
    if query.is_empty() {
        return 0;
    }
    let hay = text.to_ascii_lowercase();
    let needle = query.to_ascii_lowercase();
    if hay == needle {
        return 0;
    }
    if hay.starts_with(&needle) {
        return 1;
    }
    if hay.contains(&needle) {
        return 2;
    }
    levenshtein(&hay, &needle)
}

/// Plain Levenshtein distance, used by [`search_score`].
pub fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for i in 1..=a.len() {
        cur[0] = i;
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

/// `utils.misc.Keybind`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Keybind {
    /// `false` for a mouse button, `true` for a keyboard key.
    pub is_key: bool,
    /// The GLFW key or button code.
    pub value: i32,
    /// GLFW modifier bits (shift 1, control 2, alt 4, super 8).
    pub modifiers: u8,
}

impl Default for Keybind {
    fn default() -> Self {
        Self::none()
    }
}

impl Keybind {
    pub const fn none() -> Self {
        Self {
            is_key: false,
            value: -1,
            modifiers: 0,
        }
    }

    pub const fn key(value: i32) -> Self {
        Self {
            is_key: true,
            value,
            modifiers: 0,
        }
    }

    pub const fn mouse(value: i32) -> Self {
        Self {
            is_key: false,
            value,
            modifiers: 0,
        }
    }

    pub fn is_set(&self) -> bool {
        self.value != -1
    }

    pub fn has_mods(&self) -> bool {
        self.modifiers != 0
    }

    /// `Keybind.matches(isKey, value, modifiers)`.
    pub fn matches(&self, is_key: bool, value: i32, modifiers: u8) -> bool {
        self.is_set() && self.is_key == is_key && self.value == value && self.modifiers == modifiers
    }

    /// `Keybind.toString()` - the key's name, as Meteor's bind widget shows it.
    pub fn to_string(self) -> String {
        if !self.is_set() {
            return "None".to_string();
        }
        let mut s = String::new();
        if self.modifiers & 2 != 0 {
            s.push_str("Ctrl + ");
        }
        if self.modifiers & 1 != 0 {
            s.push_str("Shift + ");
        }
        if self.modifiers & 4 != 0 {
            s.push_str("Alt + ");
        }
        s.push_str(&if self.is_key {
            key_name(self.value).to_string()
        } else {
            format!("Mouse {}", self.value)
        });
        s
    }

    /// `Keybind.parse(String)` for the `bind` command and config files.
    pub fn parse(text: &str) -> Option<Keybind> {
        let text = text.trim();
        if text.is_empty() || text.eq_ignore_ascii_case("none") {
            return Some(Keybind::none());
        }
        let mut modifiers = 0u8;
        let mut rest = text;
        loop {
            let lower = rest.to_ascii_lowercase();
            if let Some(r) = lower.strip_prefix("ctrl+") {
                modifiers |= 2;
                rest = &rest[rest.len() - r.len()..];
            } else if let Some(r) = lower.strip_prefix("shift+") {
                modifiers |= 1;
                rest = &rest[rest.len() - r.len()..];
            } else if let Some(r) = lower.strip_prefix("alt+") {
                modifiers |= 4;
                rest = &rest[rest.len() - r.len()..];
            } else {
                break;
            }
        }
        let rest = rest.trim();
        if let Some(n) = rest
            .to_ascii_lowercase()
            .strip_prefix("mouse ")
            .and_then(|n| n.trim().parse::<i32>().ok())
        {
            return Some(Keybind {
                is_key: false,
                value: n,
                modifiers,
            });
        }
        match key_code(rest) {
            Some(k) => Some(Keybind {
                is_key: true,
                value: k,
                modifiers,
            }),
            None => None,
        }
    }
}

/// GLFW key names, trimmed to what the client binds in practice.
pub fn key_name(code: i32) -> &'static str {
    match code {
        32 => "Space",
        39 => "Apostrophe",
        44 => "Comma",
        45 => "Minus",
        46 => "Period",
        47 => "Slash",
        48..=57 => "0",
        59 => "Semicolon",
        61 => "Equal",
        65..=90 => "",
        91 => "Left Bracket",
        92 => "Backslash",
        93 => "Right Bracket",
        96 => "Grave Accent",
        256 => "Escape",
        257 => "Enter",
        258 => "Tab",
        259 => "Backspace",
        260 => "Insert",
        261 => "Delete",
        262 => "Right",
        263 => "Left",
        264 => "Down",
        265 => "Up",
        266 => "Page Up",
        267 => "Page Down",
        268 => "Home",
        269 => "End",
        280 => "Caps Lock",
        290..=299 => "F",
        320..=329 => "Num",
        _ => "Unknown",
    }
}

/// Maps a key's display name back to its GLFW code, the inverse of
/// [`key_name`]. Only the names a config file or the `bind` command may carry
/// are accepted; a bare letter or digit maps to its uppercase GLFW code.
pub fn key_code(name: &str) -> Option<i32> {
    let n = name.trim();
    if n.chars().count() == 1 {
        let c = n.chars().next().unwrap().to_ascii_uppercase();
        if c.is_ascii_alphabetic() || c.is_ascii_digit() {
            return Some(c as i32);
        }
    }
    let lower = n.to_ascii_lowercase();
    let fixed = [
        ("space", 32),
        ("apostrophe", 39),
        ("comma", 44),
        ("minus", 45),
        ("period", 46),
        ("slash", 47),
        ("semicolon", 59),
        ("equal", 61),
        ("left bracket", 91),
        ("backslash", 92),
        ("right bracket", 93),
        ("grave accent", 96),
        ("escape", 256),
        ("enter", 257),
        ("tab", 258),
        ("backspace", 259),
        ("insert", 260),
        ("delete", 261),
        ("right", 262),
        ("left", 263),
        ("down", 264),
        ("up", 265),
        ("page up", 266),
        ("page down", 267),
        ("home", 268),
        ("end", 269),
        ("caps lock", 280),
    ];
    for (k, v) in fixed {
        if lower == k {
            return Some(v);
        }
    }
    if let Some(rest) = lower.strip_prefix('f') {
        if let Ok(i) = rest.parse::<i32>() {
            if (1..=25).contains(&i) {
                return Some(289 + i);
            }
        }
    }
    None
}

/// `ChatUtils`' text colours and the `(highlight)-(default)` template syntax.
#[derive(Clone, Copy, Debug)]
pub enum ChatColor {
    Default,
    Highlight,
    Accent,
    Warning,
    Error,
    Green,
    Red,
}

impl ChatColor {
    pub fn code(self) -> &'static str {
        match self {
            ChatColor::Default => "\u{00a7}7",
            ChatColor::Highlight => "\u{00a7}9",
            ChatColor::Accent => "\u{00a7}d",
            ChatColor::Warning => "\u{00a7}e",
            ChatColor::Error => "\u{00a7}c",
            ChatColor::Green => "\u{00a7}a",
            ChatColor::Red => "\u{00a7}c",
        }
    }
}

/// Which prefix a chat line gets (`ChatUtils.forceNextPrefixClass`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Prefix {
    /// `[Meteor]`
    Meteor,
    /// `[Module]` - a module's own messages.
    Module(String),
    /// `[Command]`
    Command(String),
}

impl Prefix {
    pub fn text(&self) -> String {
        match self {
            Prefix::Meteor => "[HORIXON] ".to_string(),
            Prefix::Module(m) => format!("[{}] ", m),
            Prefix::Command(c) => format!("[{}] ", name_to_title(c)),
        }
    }
}

/// A chat line the client will show, already carrying its formatting codes.
#[derive(Clone, Debug)]
pub struct ChatMsg {
    pub text: String,
}

impl ChatMsg {
    pub fn raw(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }

    /// `ChatUtils.info(message, args)` with Meteor's `(highlight)` markers.
    pub fn new(prefix: Prefix, template: &str, args: &[&str]) -> Self {
        let body = format_template(template, args);
        Self {
            text: format!("{}{}", prefix.text(), body),
        }
    }

    pub fn plain(&self) -> String {
        strip_codes(&self.text)
    }
}

/// Replaces `(highlight)`, `(default)`, `(accent)` - and `%s` placeholders the
/// way `ChatUtils` does.
pub fn format_template(template: &str, args: &[&str]) -> String {
    let mut out = String::new();
    let mut arg = 0usize;
    let bytes: Vec<char> = template.chars().collect();
    let mut i = 0usize;
    while i < bytes.len() {
        let c = bytes[i];
        if c == '(' {
            // A colour marker: consume up to ')'.
            if let Some(close) = (i..bytes.len()).find(|&j| bytes[j] == ')') {
                let name: String = bytes[i + 1..close].iter().collect();
                let colour = match name.as_str() {
                    "highlight" => Some(ChatColor::Highlight),
                    "accent" => Some(ChatColor::Accent),
                    "default" => Some(ChatColor::Default),
                    "warning" => Some(ChatColor::Warning),
                    "error" => Some(ChatColor::Error),
                    _ => None,
                };
                if let Some(colour) = colour {
                    out.push_str(colour.code());
                    i = close + 1;
                    continue;
                }
            }
        }
        if c == '%' && bytes.get(i + 1) == Some(&'s') {
            out.push_str(args.get(arg).copied().unwrap_or(""));
            arg += 1;
            i += 2;
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

/// Removes `Sx` formatting pairs.
pub fn strip_codes(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\u{00a7}' {
            chars.next();
        } else {
            out.push(c);
        }
    }
    out
}

/// `Color.fromHsv` over a stable hash, so a module keeps the same accent colour
/// across runs (Meteor randomises it once per session; the port derives it from
/// the name so config files stay comparable).
pub fn module_color(name: &str) -> Color {
    let mut hash: u64 = 1469598103934665603;
    for b in name.bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    Color::from_hsv((hash % 360) as f64, 0.35, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles() {
        assert_eq!(name_to_title("auto-totem"), "Auto Totem");
        assert_eq!(name_to_title("esp"), "Esp");
    }

    #[test]
    fn keybind_parse_and_show() {
        let kb = Keybind::parse("Ctrl+Shift+G").unwrap();
        assert!(kb.is_key);
        assert_eq!(kb.value, 'G' as i32);
        assert_eq!(kb.modifiers, 3);
        assert!(kb.to_string().contains("Ctrl + Shift + "));
        assert!(Keybind::parse("none").unwrap() == Keybind::none());
    }

    #[test]
    fn keybind_matches_exactly() {
        let kb = Keybind::key('R' as i32);
        assert!(kb.matches(true, 'R' as i32, 0));
        assert!(!kb.matches(true, 'R' as i32, 2));
        assert!(!kb.matches(false, 'R' as i32, 0));
    }

    #[test]
    fn chat_template_formats_like_meteor() {
        let s = format_template("Toggled (highlight)%s(default).", &["Kill Aura"]);
        assert_eq!(strip_codes(&s), "Toggled Kill Aura.");
        assert!(s.contains(ChatColor::Highlight.code()));
    }

    #[test]
    fn search_prefers_prefix_then_fuzzy() {
        assert!(search_score("Kill Aura", "kill") < search_score("Kill Aura", "aura"));
        // "killaura" is one edit away from "kill aura": still near the top.
        assert!(search_score("Kill Aura", "killaura") <= 1);
        assert_eq!(search_score("Kill Aura", "kill aura"), 0);
    }

    #[test]
    fn levenshtein_basics() {
        assert_eq!(levenshtein("kitten", "sitting"), 3);
        assert_eq!(levenshtein("", "abc"), 3);
    }
}
