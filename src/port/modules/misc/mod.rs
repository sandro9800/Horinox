//! Misc modules (`systems.modules.misc`).
//!
//! Registered in `Modules.initMisc()`'s order: AntiPacketKick, AutoReconnect,
//! BetterBeacons, BetterChat, BookBot, DiscordPresence, InventoryTweaks,
//! MessageAura, Notebot, Notifier, PacketCanceller, PacketLogger, ServerSpoof,
//! SoundBlocker, Spam, Swarm.

pub mod anti_packet_kick;
pub mod auto_reconnect;
pub mod better_beacons;
pub mod better_chat;
pub mod book_bot;
pub mod discord_presence;
pub mod inventory_tweaks;
pub mod message_aura;
pub mod notebot;
pub mod notifier;
pub mod packet_canceller;
pub mod packet_logger;
pub mod server_spoof;
pub mod sound_blocker;
pub mod spam;
pub mod swarm;

use crate::meteor_core::manager::Modules;

/// `Modules.initMisc()`.
pub fn register(modules: &mut Modules) {
    anti_packet_kick::register(modules);
    auto_reconnect::register(modules);
    better_beacons::register(modules);
    better_chat::register(modules);
    book_bot::register(modules);
    discord_presence::register(modules);
    inventory_tweaks::register(modules);
    message_aura::register(modules);
    notebot::register(modules);
    notifier::register(modules);
    packet_canceller::register(modules);
    packet_logger::register(modules);
    server_spoof::register(modules);
    sound_blocker::register(modules);
    spam::register(modules);
    swarm::register(modules);
}

/// The group a module is currently adding settings to.
///
/// `Settings` hands out `&mut SettingGroup` handles, so a handle kept alive
/// across a `describe_last`/`set_visible_last` call would borrow `Settings`
/// twice. Going through here re-fetches the last group per setting, which lets a
/// module interleave settings and their Java descriptions in builder order.
pub(crate) fn last_group(
    settings: &mut crate::meteor_core::setting::Settings,
) -> &mut crate::meteor_core::setting::SettingGroup {
    if settings.groups.is_empty() {
        settings
            .groups
            .push(crate::meteor_core::setting::SettingGroup::new(
                "General", true,
            ));
    }
    let last = settings.groups.len() - 1;
    &mut settings.groups[last]
}

// ---- helpers shared by the packet-matching settings ----------------------
//
// Java's `PacketListSetting` holds `PacketType` objects, and its
// `StringListSetting`s hold ids such as `minecraft:brand`. The port's settings
// are text (see `docs/CONTRACT.md`), so the matching below accepts the three
// spellings a user may type for a packet - `minecraft:keep_alive`,
// `keep_alive`, `ClientboundKeepAlivePacket` - and compares ids by name.

/// `id` without its `minecraft:` namespace.
pub(crate) fn strip_namespace(id: &str) -> &str {
    id.strip_prefix("minecraft:").unwrap_or(id)
}

/// True when two namespaced ids name the same thing.
pub(crate) fn same_id(a: &str, b: &str) -> bool {
    strip_namespace(a.trim()).eq_ignore_ascii_case(strip_namespace(b.trim()))
}

/// `Strings.CI.contains(id, keyword)` - the case-insensitive channel filter
/// `ServerSpoof` uses.
pub(crate) fn id_contains(id: &str, keyword: &str) -> bool {
    let keyword = keyword.trim();
    if keyword.is_empty() {
        return false;
    }
    id.to_ascii_lowercase()
        .contains(&keyword.to_ascii_lowercase())
}

/// The comparison form of a packet name or pattern: lower case, alphanumerics
/// only, without the namespace, the `Clientbound`/`Serverbound` prefix, the
/// `S2C`/`C2S` direction tag or the `Packet` suffix.
fn normalize_packet(text: &str) -> String {
    let mut out: String = text
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect();
    if let Some(rest) = out.strip_prefix("minecraft") {
        out = rest.to_string();
    }
    for prefix in ["clientbound", "serverbound"] {
        if let Some(rest) = out.strip_prefix(prefix) {
            out = rest.to_string();
            break;
        }
    }
    if let Some(rest) = out.strip_suffix("packet") {
        out = rest.to_string();
    }
    for suffix in ["s2c", "c2s"] {
        if let Some(rest) = out.strip_suffix(suffix) {
            out = rest.to_string();
            break;
        }
    }
    out
}

/// `PacketListSetting.contains(type)` for a packet name held as text.
pub(crate) fn packet_matches(name: &str, pattern: &str) -> bool {
    let pattern = pattern.trim();
    if pattern.is_empty() {
        return false;
    }
    let name = normalize_packet(name);
    let pat = normalize_packet(pattern);
    if name.is_empty() || pat.is_empty() {
        return false;
    }
    name == pat || (pat.len() >= 4 && name.contains(&pat))
}

/// The text of a packet's field, whatever case the hook spelled its key with.
pub(crate) fn packet_field<'a>(
    packet: &'a crate::meteor_core::env::PacketInfo,
    keys: &[&str],
) -> Option<&'a str> {
    packet
        .fields
        .iter()
        .find(|(k, _)| keys.iter().any(|want| k.eq_ignore_ascii_case(want)))
        .map(|(_, v)| v.as_str())
}

/// `PacketInfo`'s fields as `key`, `value` slices, for the modules that rewrite
/// a packet in place.
pub(crate) fn packet_field_mut<'a>(
    packet: &'a mut crate::meteor_core::env::PacketInfo,
    keys: &[&str],
) -> Option<&'a mut String> {
    packet
        .fields
        .iter_mut()
        .find(|(k, _)| keys.iter().any(|want| k.eq_ignore_ascii_case(want)))
        .map(|(_, v)| v)
}

/// The decoded size of a packet, in bytes of field text.
pub(crate) fn packet_len(packet: &crate::meteor_core::env::PacketInfo) -> usize {
    packet.fields.iter().map(|(k, v)| k.len() + v.len()).sum()
}

/// `HH:mm` / `HH:mm:ss`, for `BetterChat`'s timestamps. `now` is seconds since
/// the epoch (`Env::now`); the port shows UTC because the `Env` carries no zone.
pub(crate) fn format_time(now: f64, seconds: bool) -> String {
    let (hh, mm, ss, _ms) = split_clock(now);
    if seconds {
        format!("{hh:02}:{mm:02}:{ss:02}")
    } else {
        format!("{hh:02}:{mm:02}")
    }
}

/// `HH:mm:ss.SSS`, for `PacketLogger`'s lines.
pub(crate) fn format_time_ms(now: f64) -> String {
    let (hh, mm, ss, ms) = split_clock(now);
    format!("{hh:02}:{mm:02}:{ss:02}.{ms:03}")
}

fn split_clock(now: f64) -> (u64, u64, u64, u64) {
    let t = if now.is_finite() && now > 0.0 {
        now
    } else {
        0.0
    };
    let total_ms = (t * 1000.0) as u64;
    let secs = total_ms / 1000;
    (
        (secs / 3600) % 24,
        (secs % 3600) / 60,
        secs % 60,
        total_ms % 1000,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packet_names_match_in_every_spelling() {
        assert!(packet_matches("minecraft:keep_alive", "keep_alive"));
        assert!(packet_matches("keep_alive", "minecraft:keep_alive"));
        assert!(packet_matches(
            "ClientboundKeepAlivePacket",
            "minecraft:keep_alive"
        ));
        assert!(packet_matches(
            "ServerboundPlayerActionPacket",
            "player_action"
        ));
        assert!(!packet_matches("minecraft:keep_alive", "chat"));
    }

    #[test]
    fn clock_formats_from_env_now() {
        assert_eq!(format_time(0.0, false), "00:00");
        assert_eq!(format_time(3661.0, true), "01:01:01");
        assert_eq!(format_time_ms(3661.5), "01:01:01.500");
    }

    #[test]
    fn channel_and_id_helpers_ignore_case() {
        assert!(same_id("minecraft:brand", "brand"));
        assert!(!id_contains("minecraft:register", "fabric"));
        assert!(id_contains("Fabric|API", "fabric"));
    }
}
