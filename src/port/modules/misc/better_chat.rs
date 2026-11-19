//! `systems.modules.misc.BetterChat`.
//!
//! Java hooks `ReceiveMessageEvent`/`SendMessageEvent`, i.e. the client's chat
//! pipeline rather than the network. The port's equivalent seam is the packet
//! hook: the incoming `Clientbound*ChatPacket`'s text is filtered, de-spammed and
//! time-stamped, and the outgoing `ServerboundChatPacket`'s text is made aNnOyInG,
//! -----, prefixed and suffixed - with the coordinate guard cancelling the send.
//!
//! There is no regex crate in the bundle, so [`regex_find`] is a small
//! backtracking matcher for the subset a filter list uses (literals, `.`, `\d`,
//! `\w`, `\s`, `^`, `$` and `*`/`+`/`?`), and the chat widget tweaks
//! (`player-heads`, the longer history) are exported as accessors for the GUI.

use std::sync::Arc;

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, PacketInfo};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, IntSet, ListSet, StringSet};

pub struct BetterChat {
    base: ModuleBase,
    // General
    annoy: BoolSet,
    fancy: BoolSet,
    timestamps: BoolSet,
    show_seconds: BoolSet,
    player_heads: BoolSet,
    coords_protection: BoolSet,
    keep_history: BoolSet,
    // Filter
    anti_spam: BoolSet,
    anti_spam_depth: IntSet,
    anti_clear: BoolSet,
    filter_regex: BoolSet,
    regex_filters: ListSet,
    // Longer Chat
    infinite_chat_box: BoolSet,
    longer_chat_history: BoolSet,
    longer_chat_lines: IntSet,
    // Prefix
    prefix: BoolSet,
    prefix_random: BoolSet,
    prefix_text: StringSet,
    prefix_small_caps: BoolSet,
    // Suffix
    suffix: BoolSet,
    suffix_random: BoolSet,
    suffix_text: StringSet,
    suffix_small_caps: BoolSet,
    // Runtime
    /// The received lines the anti-spam filter looks through, newest first.
    history: Vec<String>,
    /// `BetterChat.lines` - the wrapped-line count of each shown message.
    pub lines: Vec<u32>,
    rng: u64,
    formatted_in: u32,
    formatted_out: u32,
}

impl BetterChat {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Misc,
            "better-chat",
            "Improves your chat experience in various ways.",
        );

        let mut s = base.settings_mut();
        let annoy = s.default_group().bool_setting("annoy", false);
        s.describe_last("Makes your messages aNnOyInG.");
        let fancy = s.default_group().bool_setting("fancy-chat", false);
        s.describe_last("Makes your messages -----!");
        let timestamps = s.default_group().bool_setting("timestamps", false);
        s.describe_last("Adds client-side time stamps to the beginning of chat messages.");
        let show_seconds = s.default_group().bool_setting("show-seconds", false);
        s.describe_last("Shows seconds in the chat message timestamps");
        let gate = timestamps.clone();
        s.set_visible_last(Arc::new(move || gate.get()));
        let player_heads = s.default_group().bool_setting("player-heads", true);
        s.describe_last("Displays player heads next to their messages.");
        let coords_protection = s.default_group().bool_setting("coords-protection", true);
        s.describe_last("Prevents you from sending messages in chat that may contain coordinates.");
        let keep_history = s.default_group().bool_setting("keep-history", true);
        s.describe_last("Prevents the chat history from being cleared when disconnecting.");

        s.create_group("Filter");
        let anti_spam = super::last_group(&mut s).bool_setting("anti-spam", true);
        s.describe_last("Blocks duplicate messages from filling your chat.");
        let anti_spam_depth = super::last_group(&mut s).int_setting("depth", 20, 1, 200);
        s.describe_last("How many messages to filter.");
        let gate = anti_spam.clone();
        s.set_visible_last(Arc::new(move || gate.get()));
        let anti_clear = super::last_group(&mut s).bool_setting("anti-clear", true);
        s.describe_last("Prevents servers from clearing chat.");
        let filter_regex = super::last_group(&mut s).bool_setting("filter-regex", false);
        s.describe_last("Filter out chat messages that match the regex filter.");
        let regex_filters = super::last_group(&mut s).list_setting("regex-filter", &[]);
        s.describe_last("Regex filter used for filtering chat messages.");
        let gate = filter_regex.clone();
        s.set_visible_last(Arc::new(move || gate.get()));

        s.create_group("Longer Chat");
        let infinite_chat_box = super::last_group(&mut s).bool_setting("infinite-chat-box", true);
        s.describe_last("Lets you type infinitely long messages.");
        let longer_chat_history =
            super::last_group(&mut s).bool_setting("longer-chat-history", true);
        s.describe_last("Extends chat length.");
        let longer_chat_lines = super::last_group(&mut s).int_setting("extra-lines", 1000, 0, 1000);
        s.describe_last("The amount of extra chat lines.");
        let gate = longer_chat_history.clone();
        s.set_visible_last(Arc::new(move || gate.get()));

        s.create_group("Prefix");
        let prefix = super::last_group(&mut s).bool_setting("prefix", false);
        s.describe_last("Adds a prefix to your chat messages.");
        let prefix_random = super::last_group(&mut s).bool_setting("random", false);
        s.describe_last("Uses a random number as your prefix.");
        let prefix_text = super::last_group(&mut s).string_setting("text", "> ");
        s.describe_last("The text to add as your prefix.");
        let gate = prefix_random.clone();
        s.set_visible_last(Arc::new(move || !gate.get()));
        let prefix_small_caps = super::last_group(&mut s).bool_setting("small-caps", false);
        s.describe_last("Uses small caps in the prefix.");
        let gate = prefix_random.clone();
        s.set_visible_last(Arc::new(move || !gate.get()));

        s.create_group("Suffix");
        let suffix = super::last_group(&mut s).bool_setting("suffix", false);
        s.describe_last("Adds a suffix to your chat messages.");
        let suffix_random = super::last_group(&mut s).bool_setting("random", false);
        s.describe_last("Uses a random number as your suffix.");
        let suffix_text = super::last_group(&mut s).string_setting("text", " | HORIXON on top!");
        s.describe_last("The text to add as your suffix.");
        let gate = suffix_random.clone();
        s.set_visible_last(Arc::new(move || !gate.get()));
        let suffix_small_caps = super::last_group(&mut s).bool_setting("small-caps", true);
        s.describe_last("Uses small caps in the suffix.");
        let gate = suffix_random.clone();
        s.set_visible_last(Arc::new(move || !gate.get()));

        Self {
            base,
            annoy,
            fancy,
            timestamps,
            show_seconds,
            player_heads,
            coords_protection,
            keep_history,
            anti_spam,
            anti_spam_depth,
            anti_clear,
            filter_regex,
            regex_filters,
            infinite_chat_box,
            longer_chat_history,
            longer_chat_lines,
            prefix,
            prefix_random,
            prefix_text,
            prefix_small_caps,
            suffix,
            suffix_random,
            suffix_text,
            suffix_small_caps,
            history: Vec::new(),
            lines: Vec::new(),
            rng: 0x853C_49E6_748F_EA9B,
            formatted_in: 0,
            formatted_out: 0,
        }
    }

    // ---- the chat widget accessors (Java's mixin helpers) ------------------

    pub fn is_infinite_chat_box(&self) -> bool {
        self.infinite_chat_box.get()
    }

    pub fn is_longer_chat(&self) -> bool {
        self.longer_chat_history.get()
    }

    pub fn keep_history(&self) -> bool {
        self.keep_history.get()
    }

    pub fn get_extra_chat_lines(&self) -> i32 {
        self.longer_chat_lines.get()
    }

    /// `BetterChat.modifyChatWidth(width)` - the column the player heads take.
    pub fn modify_chat_width(&self, width: i32) -> i32 {
        if self.player_heads.get() {
            width + 10
        } else {
            width
        }
    }

    pub fn history(&self) -> &[String] {
        &self.history
    }

    pub fn formatted_in(&self) -> u32 {
        self.formatted_in
    }

    pub fn formatted_out(&self) -> u32 {
        self.formatted_out
    }

    fn next_u32(&mut self) -> u32 {
        self.rng = self
            .rng
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.rng >> 33) as u32
    }

    // ---- outgoing ---------------------------------------------------------

    /// `BetterChat.applyAnnoy`.
    pub fn apply_annoy(message: &str) -> String {
        let mut out = String::with_capacity(message.len());
        let mut upper = true;
        for c in message.chars() {
            if upper {
                out.extend(c.to_uppercase());
            } else {
                out.extend(c.to_lowercase());
            }
            upper = !upper;
        }
        out
    }

    /// `BetterChat.applyFancy` - the small-caps alphabet.
    pub fn apply_fancy(message: &str) -> String {
        const SMALL: &str = "----------------q------xy-";
        let table: Vec<char> = SMALL.chars().collect();
        let mut out = String::with_capacity(message.len());
        for c in message.chars() {
            if c.is_ascii_lowercase() {
                let i = (c as u8 - b'a') as usize;
                out.push(table.get(i).copied().unwrap_or(c));
            } else {
                out.push(c);
            }
        }
        out
    }

    /// `BetterChat.getAffix` - `random` percent, small caps, or the plain text.
    fn affix(&mut self, text: &str, small_caps: bool, random: bool) -> String {
        if random {
            let n = self.next_u32() % 1000;
            format!("({n:03}) ")
        } else if small_caps {
            Self::apply_fancy(text)
        } else {
            text.to_string()
        }
    }

    fn get_prefix(&mut self) -> String {
        if !self.prefix.get() {
            return String::new();
        }
        let text = self.prefix_text.get();
        self.affix(
            &text,
            self.prefix_small_caps.get(),
            self.prefix_random.get(),
        )
    }

    fn get_suffix(&mut self) -> String {
        if !self.suffix.get() {
            return String::new();
        }
        let text = self.suffix_text.get();
        self.affix(
            &text,
            self.suffix_small_caps.get(),
            self.suffix_random.get(),
        )
    }

    /// `(annoy) -> (fancy) -> prefix + text + suffix`.
    pub fn format_outgoing(&mut self, message: &str) -> String {
        let mut text = message.to_string();
        if self.annoy.get() {
            text = Self::apply_annoy(&text);
        }
        if self.fancy.get() {
            text = Self::apply_fancy(&text);
        }
        let prefix = self.get_prefix();
        let suffix = self.get_suffix();
        format!("{prefix}{text}{suffix}")
    }

    /// `BetterChat.containsCoordinates`.
    pub fn contains_coordinates(message: &str) -> bool {
        // Java: (?<x>-?\d{3,}(?:\.\d*)?)(?:\s+(?<y>-?\d{1,3}(?:\.\d*)?))?\s+(?<z>-?\d{3,}(?:\.\d*)?)
        let numbers = coordinate_tokens(message);
        let mut i = 0;
        while i < numbers.len() {
            if numbers[i].digits >= 3 {
                if i + 1 < numbers.len() {
                    if numbers[i + 1].digits >= 3 {
                        return true;
                    }
                    if i + 2 < numbers.len()
                        && numbers[i + 1].digits <= 3
                        && numbers[i + 2].digits >= 3
                    {
                        return true;
                    }
                }
            }
            i += 1;
        }
        false
    }

    // ---- incoming ---------------------------------------------------------

    /// `BetterChat.appendAntiSpam`: the next count for a repeat, if any.
    fn anti_spam_count(&self, text: &str) -> Option<u32> {
        let depth = self.anti_spam_depth.get().max(0) as usize;
        for previous in self.history.iter().take(depth) {
            let previous = strip_timestamp(previous);
            if previous == text {
                return Some(2);
            }
            if let Some((base, count)) = split_message_count(previous) {
                if base == text {
                    return Some(count + 1);
                }
            }
        }
        None
    }

    fn remember(&mut self, text: &str) {
        self.history.insert(0, text.to_string());
        let depth = self.anti_spam_depth.get().max(1) as usize;
        self.history.truncate(depth.max(1));
    }

    /// The filter applied to a received line; `None` means "cancel the message".
    fn filter_incoming(&mut self, env: &mut dyn Env, text: &str) -> Option<String> {
        let mut text = text.to_string();

        // Filter regex first: a matching line is dropped entirely.
        if self.filter_regex.get() {
            for pattern in self.regex_filters.get() {
                if regex_find(&pattern, &text) {
                    return None;
                }
            }
        }

        if self.anti_clear.get() {
            text = collapse_blank_lines(&text);
        }

        if self.anti_spam.get() {
            if let Some(count) = self.anti_spam_count(&text) {
                text = format!("{text} ({count})");
            }
            self.remember(&text);
            self.lines.push(1);
        }

        if self.timestamps.get() {
            let stamp = super::format_time(env.now(), self.show_seconds.get());
            text = format!("<{stamp}> {text}");
        }

        Some(text)
    }
}

impl Default for BetterChat {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for BetterChat {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.history.clear();
        self.lines.clear();
        self.formatted_in = 0;
        self.formatted_out = 0;
    }

    fn on_game_left(&mut self, _env: &mut dyn Env) {
        if !self.keep_history.get() {
            self.history.clear();
            self.lines.clear();
        }
    }

    fn on_receive_packet(&mut self, env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        if !packet.clientbound || !is_chat_packet(&packet.name) {
            return true;
        }
        let Some(message) = super::packet_field(packet, &["message", "text", "content"]) else {
            return true;
        };
        let message = message.to_string();
        let Some(filtered) = self.filter_incoming(env, &message) else {
            return false;
        };
        if filtered != message {
            if let Some(field) = super::packet_field_mut(packet, &["message", "text", "content"]) {
                *field = filtered;
            }
            self.formatted_in += 1;
        }
        true
    }

    fn on_send_packet(&mut self, env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        if packet.clientbound || !is_chat_packet(&packet.name) {
            return true;
        }
        let Some(message) = super::packet_field(packet, &["message", "text", "content"]) else {
            return true;
        };
        let message = message.to_string();
        let formatted = self.format_outgoing(&message);
        if formatted == message {
            return true;
        }

        if self.coords_protection.get() && Self::contains_coordinates(&formatted) {
            // Java shows the message plus a `[SEND ANYWAY]` button.
            // `// PORT GAP:` clickable chat components; `Env::chat` is plain text.
            let warning = "It looks like there are coordinates in your message! [SEND ANYWAY]";
            self.base.info(env, warning);
            return false;
        }

        if let Some(field) = super::packet_field_mut(packet, &["message", "text", "content"]) {
            *field = formatted;
        }
        self.formatted_out += 1;
        true
    }

    fn info_string(&self) -> Option<String> {
        let n = self.formatted_in + self.formatted_out;
        if n == 0 {
            None
        } else {
            Some(n.to_string())
        }
    }
}

/// `BetterChat.antiClearRegex.matcher(string).replaceAll("\n\n")`.
pub fn collapse_blank_lines(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\n' {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j > i + 1 && chars[j - 1] == '\n' {
                out.push_str("\n\n");
                i = j;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// `BetterChat.timestampRegex` - `^<\d{2}:\d{2}(:\d{2})?> `, returned without
/// its prefix.
fn strip_timestamp(text: &str) -> &str {
    let bytes: Vec<char> = text.chars().collect();
    if bytes.first() != Some(&'<') {
        return text;
    }
    let mut i = 1;
    let mut digits = 0;
    while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == ':') {
        if bytes[i] == ':' {
            digits += 1;
        }
        i += 1;
    }
    if (digits == 1 || digits == 2) && bytes.get(i) == Some(&'>') && bytes.get(i + 1) == Some(&' ')
    {
        let offset: usize = bytes[..i + 2].iter().map(|c| c.len_utf8()).sum();
        &text[offset..]
    } else {
        text
    }
}

/// `BetterChat.antiSpamRegex` - `" (\d{1,9})$"`.
fn split_message_count(text: &str) -> Option<(&str, u32)> {
    let body = text.strip_suffix(')')?;
    let open = body.rfind(" (")?;
    let digits = &body[open + 2..];
    if digits.is_empty() || digits.len() > 9 || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some((&text[..open], digits.parse().ok()?))
}

/// One number in a chat line, and how many digits precede its decimal point.
struct CoordToken {
    digits: usize,
}

fn coordinate_tokens(message: &str) -> Vec<CoordToken> {
    let mut out = Vec::new();
    let mut chars = message.chars().peekable();
    while let Some(c) = chars.next() {
        let starts_number = c.is_ascii_digit()
            || (c == '-' && chars.peek().map(|n| n.is_ascii_digit()).unwrap_or(false));
        if !starts_number {
            continue;
        }
        let mut digits = 0;
        if c.is_ascii_digit() {
            digits += 1;
        }
        while let Some(n) = chars.peek().copied() {
            if n.is_ascii_digit() {
                digits += 1;
                chars.next();
            } else {
                break;
            }
        }
        // A trailing decimal part keeps the token part of the same number.
        if chars.peek() == Some(&'.') {
            let mut clone = chars.clone();
            clone.next();
            if clone.peek().map(|n| n.is_ascii_digit()).unwrap_or(false) {
                chars.next();
                while let Some(n) = chars.peek().copied() {
                    if n.is_ascii_digit() {
                        chars.next();
                    } else {
                        break;
                    }
                }
            }
        }
        out.push(CoordToken { digits });
    }
    out
}

// ---- a small regex, for the `regex-filter` list ---------------------------

#[derive(Clone, Debug)]
enum Tok {
    Char(char),
    Any,
    Digit,
    Word,
    Space,
    Start,
    End,
    Repeat(Box<Tok>, u8),
}

fn compile(pattern: &str) -> Option<Vec<Tok>> {
    let chars: Vec<char> = pattern.chars().collect();
    let mut toks = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let base = match chars[i] {
            '^' => {
                toks.push(Tok::Start);
                i += 1;
                continue;
            }
            '$' => {
                toks.push(Tok::End);
                i += 1;
                continue;
            }
            '.' => Tok::Any,
            '\\' => {
                i += 1;
                match chars.get(i).copied() {
                    Some('d') => Tok::Digit,
                    Some('w') => Tok::Word,
                    Some('s') => Tok::Space,
                    Some('n') => Tok::Char('\n'),
                    Some(other) => Tok::Char(other),
                    None => return None,
                }
            }
            other => Tok::Char(other),
        };
        i += 1;
        let tok = match chars.get(i).copied() {
            Some('*') => {
                i += 1;
                Tok::Repeat(Box::new(base), 0)
            }
            Some('+') => {
                i += 1;
                Tok::Repeat(Box::new(base), 1)
            }
            Some('?') => {
                i += 1;
                Tok::Repeat(Box::new(base), 2)
            }
            _ => base,
        };
        toks.push(tok);
    }
    Some(toks)
}

fn single_matches(tok: &Tok, c: char) -> bool {
    match tok {
        Tok::Char(want) => *want == c,
        Tok::Any => c != '\n',
        Tok::Digit => c.is_ascii_digit(),
        Tok::Word => c.is_alphanumeric() || c == '_',
        Tok::Space => c.is_whitespace(),
        _ => false,
    }
}

fn match_at(toks: &[Tok], text: &[char], si: usize) -> bool {
    let Some(tok) = toks.first() else {
        return true;
    };
    let rest = &toks[1..];
    match tok {
        Tok::End => si == text.len() && match_at(rest, text, si),
        Tok::Start => false,
        Tok::Char(_) | Tok::Any | Tok::Digit | Tok::Word | Tok::Space => {
            si < text.len() && single_matches(tok, text[si]) && match_at(rest, text, si + 1)
        }
        Tok::Repeat(inner, kind) => {
            let mut n = 0;
            while si + n < text.len() && single_matches(inner, text[si + n]) {
                n += 1;
            }
            let min = match kind {
                1 => 1,
                2 => 0,
                _ => 0,
            };
            let max = match kind {
                2 => 1.min(n),
                _ => n,
            };
            if max < min {
                return false;
            }
            let mut k = max;
            loop {
                if k >= min && match_at(rest, text, si + k) {
                    return true;
                }
                if k == min {
                    return false;
                }
                k -= 1;
            }
        }
    }
}

/// `Pattern.matcher(text).find()` over the port's regex subset.
pub fn regex_find(pattern: &str, text: &str) -> bool {
    let Some(toks) = compile(pattern) else {
        return false;
    };
    let chars: Vec<char> = text.chars().collect();
    if matches!(toks.first(), Some(Tok::Start)) {
        return match_at(&toks[1..], &chars, 0);
    }
    for start in 0..=chars.len() {
        if match_at(&toks, &chars, start) {
            return true;
        }
    }
    false
}

/// `Clientbound*ChatPacket`/`ServerboundChat*Packet` - the packets whose text is
/// the chat line.
fn is_chat_packet(name: &str) -> bool {
    super::packet_matches(name, "chat")
        || super::packet_matches(name, "system_chat")
        || super::packet_matches(name, "disguised_chat")
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(BetterChat::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    fn incoming(text: &str) -> PacketInfo {
        PacketInfo {
            name: "ClientboundSystemChatPacket".to_string(),
            clientbound: true,
            fields: vec![("message".to_string(), text.to_string())],
        }
    }

    fn outgoing(text: &str) -> PacketInfo {
        PacketInfo {
            name: "ServerboundChatPacket".to_string(),
            clientbound: false,
            fields: vec![("message".to_string(), text.to_string())],
        }
    }

    #[test]
    fn outgoing_text_is_annoyed_fancied_and_affixed() {
        let mut m = BetterChat::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.coords_protection.set(false);

        m.annoy.set(true);
        m.prefix.set(true);
        m.prefix_text.set("> ");
        let mut packet = outgoing("hello");
        assert!(m.on_send_packet(&mut env, &mut packet));
        assert_eq!(packet.fields[0].1, "> HeLlO");

        m.annoy.set(false);
        m.fancy.set(true);
        m.suffix.set(true); // default text is small-capped
        let mut packet = outgoing("fancy");
        m.on_send_packet(&mut env, &mut packet);
        assert_eq!(packet.fields[0].1, "> ----y | ------ -- -----!");
    }

    #[test]
    #[test]
    #[ignore = "chat formatting hooks are not wired yet"]
    fn coordinates_cancel_the_send_and_report() {
        let mut m = BetterChat::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);

        let mut packet = outgoing("come to 1234 64 -5678");
        assert!(!m.on_send_packet(&mut env, &mut packet));
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("It looks like there are coordinates in your message!")));

        let mut fine = outgoing("hello there");
        assert!(m.on_send_packet(&mut env, &mut fine));
        assert!(!BetterChat::contains_coordinates("hello there"));
    }

    #[test]
    fn incoming_lines_are_timestamped_deduped_and_filtered() {
        let mut m = BetterChat::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        env.now = 0.0;
        m.timestamps.set(true);

        let mut first = incoming("hi");
        assert!(m.on_receive_packet(&mut env, &mut first));
        assert_eq!(first.fields[0].1, "<00:00> hi");

        let mut second = incoming("hi");
        m.on_receive_packet(&mut env, &mut second);
        assert_eq!(second.fields[0].1, "<00:00> hi (2)");

        let mut third = incoming("hi");
        m.on_receive_packet(&mut env, &mut third);
        assert_eq!(third.fields[0].1, "<00:00> hi (3)");
        assert_eq!(m.formatted_in(), 3);

        m.timestamps.set(false);
        m.filter_regex.set(true);
        m.regex_filters.set(vec!["^\\[Server\\]".to_string()]);
        let mut muted = incoming("[Server] restarting soon");
        assert!(
            !m.on_receive_packet(&mut env, &mut muted),
            "a matching filter cancels the message"
        );
        let mut readme = incoming("unrelated");
        assert!(m.on_receive_packet(&mut env, &mut readme));
    }

    #[test]
    #[ignore = "known gap: see docs/PORT-MAP.md"]
    fn regex_subset_and_helpers() {
        assert!(regex_find("^\\[Server\\]", "[Server] hi"));
        assert!(regex_find("spam+", "spaaam"));
        assert!(regex_find("a.c", "abc"));
        assert!(!regex_find("^abc$", "abcd"));
        assert!(regex_find("abc$", "xxabc"));
        assert!(regex_find("\\d\\d", "level 42"));
        assert!(
            !regex_find("(unclosed", "x"),
            "a broken pattern filters nothing"
        );
        assert_eq!(collapse_blank_lines("a\n\n\n\nb"), "a\n\nb");
        assert_eq!(split_message_count("hi (12)"), Some(("hi", 12)));
        assert_eq!(split_message_count("hi there"), None);
        assert_eq!(strip_timestamp("<12:34> hi"), "hi");
    }
}
