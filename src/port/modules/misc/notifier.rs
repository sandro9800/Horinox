//! `systems.modules.misc.Notifier`.
//!
//! Java watches `EntityAddedEvent`/`EntityRemovedEvent`, the tab-list packets and
//! `ClientboundEntityEventPacket` (a totem pop is event 35). The port does the
//! same on the packet hooks and on `Env::entities()`: visual-range enter/leave
//! is a diff of the entity list, totem pops and joins/leaves come from packets,
//! and the join/leave lines queue up behind `notification-delay` like Java's
//! `messageQueue`.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, PacketInfo};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, EnumSet, IntSet, ListSet};

/// `EntityEvent.PROTECTED_FROM_DEATH` - what a totem pop sends.
pub const PROTECTED_FROM_DEATH: i32 = 35;

const EVENTS: &[&str] = &["Spawn", "Despawn", "Both"];
const JOIN_LEAVE_MODES: &[&str] = &["None", "Joins", "Leaves", "Both"];

pub struct Notifier {
    base: ModuleBase,
    // Totem Pops
    totem_pops: BoolSet,
    totems_distance_check: BoolSet,
    totems_distance: IntSet,
    totems_ignore_own: BoolSet,
    totems_ignore_friends: BoolSet,
    totems_ignore_others: BoolSet,
    // Visual Range
    visual_range: BoolSet,
    event: EnumSet,
    entities: ListSet,
    visual_range_ignore_friends: BoolSet,
    visual_range_ignore_fakes: BoolSet,
    visual_make_sound: BoolSet,
    // Pearl
    pearl: BoolSet,
    pearl_ignore_own: BoolSet,
    pearl_ignore_friends: BoolSet,
    // Joins/Leaves
    joins_leaves_mode: EnumSet,
    notification_delay: IntSet,
    simple_notifications: BoolSet,
    // Runtime
    timer: i32,
    login_packet: bool,
    totem_pop_map: Vec<(String, u32)>,
    message_queue: Vec<String>,
    /// The players the visual-range diff has already announced.
    seen: Vec<(i32, String)>,
    announced: u32,
}

impl Notifier {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Misc,
            "notifier",
            "Notifies you of different events.",
        );

        let mut s = base.settings_mut();
        s.create_group("Totem Pops");
        let totem_pops = super::last_group(&mut s).bool_setting("totem-pops", true);
        s.describe_last("Notifies you when a player pops a totem.");
        let totems_distance_check = super::last_group(&mut s).bool_setting("distance-check", false);
        s.describe_last("Limits the distance in which the pops are recognized.");
        let gate = totem_pops.clone();
        s.set_visible_last(std::sync::Arc::new(move || gate.get()));
        let totems_distance = super::last_group(&mut s).int_setting("player-radius", 30, 1, 100);
        s.describe_last("The radius in which to log totem pops.");
        let pops_gate = totem_pops.clone();
        let distance_gate = totems_distance_check.clone();
        s.set_visible_last(std::sync::Arc::new(move || {
            pops_gate.get() && distance_gate.get()
        }));
        let totems_ignore_own = super::last_group(&mut s).bool_setting("ignore-own", false);
        s.describe_last("Ignores your own totem pops.");
        let totems_ignore_friends = super::last_group(&mut s).bool_setting("ignore-friends", false);
        s.describe_last("Ignores friends totem pops.");
        let totems_ignore_others = super::last_group(&mut s).bool_setting("ignore-others", false);
        s.describe_last("Ignores other players totem pops.");

        s.create_group("Visual Range");
        let visual_range = super::last_group(&mut s).bool_setting("visual-range", false);
        s.describe_last("Notifies you when an entity enters your render distance.");
        let event = super::last_group(&mut s).enum_setting("event", EVENTS, 2);
        s.describe_last("When to log the entities.");
        let entities = super::last_group(&mut s).list_setting("entities", &["minecraft:player"]);
        s.describe_last("Which entities to notify about.");
        let visual_range_ignore_friends =
            super::last_group(&mut s).bool_setting("ignore-friends", true);
        s.describe_last("Ignores friends.");
        let visual_range_ignore_fakes =
            super::last_group(&mut s).bool_setting("ignore-fake-players", true);
        s.describe_last("Ignores fake players.");
        let visual_make_sound = super::last_group(&mut s).bool_setting("sound", true);
        s.describe_last("Emits a sound effect on enter / leave");

        s.create_group("Pearl");
        let pearl = super::last_group(&mut s).bool_setting("pearl", true);
        s.describe_last("Notifies you when a player is teleported using an ender pearl.");
        let pearl_ignore_own = super::last_group(&mut s).bool_setting("ignore-own", false);
        s.describe_last("Ignores your own pearls.");
        let pearl_ignore_friends = super::last_group(&mut s).bool_setting("ignore-friends", false);
        s.describe_last("Ignores friends pearls.");

        s.create_group("Joins/Leaves");
        let joins_leaves_mode =
            super::last_group(&mut s).enum_setting("player-joins-leaves", JOIN_LEAVE_MODES, 0);
        s.describe_last("How to handle player join/leave notifications.");
        let notification_delay =
            super::last_group(&mut s).int_setting("notification-delay", 0, 0, 1000);
        s.describe_last(
            "How long to wait in ticks before posting the next join/leave notification in your chat.",
        );
        let simple_notifications =
            super::last_group(&mut s).bool_setting("simple-notifications", true);
        s.describe_last(
            "Display join/leave notifications without a prefix, to reduce chat clutter.",
        );

        Self {
            base,
            totem_pops,
            totems_distance_check,
            totems_distance,
            totems_ignore_own,
            totems_ignore_friends,
            totems_ignore_others,
            visual_range,
            event,
            entities,
            visual_range_ignore_friends,
            visual_range_ignore_fakes,
            visual_make_sound,
            pearl,
            pearl_ignore_own,
            pearl_ignore_friends,
            joins_leaves_mode,
            notification_delay,
            simple_notifications,
            timer: 0,
            login_packet: true,
            totem_pop_map: Vec::new(),
            message_queue: Vec::new(),
            seen: Vec::new(),
            announced: 0,
        }
    }

    pub fn queued(&self) -> usize {
        self.message_queue.len()
    }

    pub fn totem_pop_map(&self) -> &[(String, u32)] {
        &self.totem_pop_map
    }

    pub fn announced(&self) -> u32 {
        self.announced
    }

    /// `Notifier.getChatId`/`totemPopMap` bookkeeping.
    fn record_pop(&mut self, name: &str) -> u32 {
        if let Some((_, pops)) = self
            .totem_pop_map
            .iter_mut()
            .find(|(who, _)| who.eq_ignore_ascii_case(name))
        {
            *pops += 1;
            return *pops;
        }
        self.totem_pop_map.push((name.to_string(), 1));
        1
    }

    fn queue_line(&mut self, line: String) {
        self.message_queue.push(line);
    }

    fn simple_line(sign: &str, name: &str) -> String {
        format!("[{sign}] {name}")
    }

    fn joins_enabled(&self) -> bool {
        self.joins_leaves_mode.is("Both") || self.joins_leaves_mode.is("Joins")
    }

    fn leaves_enabled(&self) -> bool {
        self.joins_leaves_mode.is("Both") || self.joins_leaves_mode.is("Leaves")
    }

    /// The names a tab-list packet carries.
    fn packet_names(packet: &PacketInfo) -> Vec<String> {
        if let Some(names) = super::packet_field(packet, &["names", "players"]) {
            return names
                .split(',')
                .map(|name| name.trim().to_string())
                .filter(|name| !name.is_empty())
                .collect();
        }
        super::packet_field(packet, &["name", "player", "profile"])
            .map(|name| vec![name.trim().to_string()])
            .unwrap_or_default()
    }

    fn announce_visual_range(&mut self, env: &mut dyn Env, name: &str, entered: bool) {
        let verb = if entered { "entered" } else { "left" };
        self.base
            .info(env, &format!("{name} has {verb} your visual range!"));
        if self.visual_make_sound.get() {
            env.play_sound("minecraft:entity.experience_orb.pickup", 3.0, 1.0);
        }
        self.announced += 1;
    }
}

impl Default for Notifier {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Notifier {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, _env: &mut dyn Env) {
        self.totem_pop_map.clear();
        self.message_queue.clear();
        self.seen.clear();
        self.announced = 0;
    }

    fn on_deactivate(&mut self, _env: &mut dyn Env) {
        self.timer = 0;
        self.message_queue.clear();
    }

    fn on_game_joined(&mut self, _env: &mut dyn Env) {
        self.timer = 0;
        self.totem_pop_map.clear();
        self.message_queue.clear();
        self.seen.clear();
        // The first tab-list update is the login burst, not a join.
        self.login_packet = true;
    }

    fn on_game_left(&mut self, _env: &mut dyn Env) {
        self.login_packet = true;
    }

    fn on_receive_packet(&mut self, env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        if super::packet_matches(&packet.name, "player_info_update")
            || super::packet_matches(&packet.name, "player_info_add")
        {
            if !self.joins_enabled() {
                return true;
            }
            if self.login_packet {
                self.login_packet = false;
                return true;
            }
            for name in Self::packet_names(packet) {
                let line = if self.simple_notifications.get() {
                    Self::simple_line("+", &name)
                } else {
                    format!("{name} joined.")
                };
                self.queue_line(line);
            }
            return true;
        }

        if super::packet_matches(&packet.name, "player_info_remove") {
            if !self.leaves_enabled() {
                return true;
            }
            for name in Self::packet_names(packet) {
                let line = if self.simple_notifications.get() {
                    Self::simple_line("-", &name)
                } else {
                    format!("{name} left.")
                };
                self.queue_line(line);
            }
            return true;
        }

        if self.totem_pops.get() && super::packet_matches(&packet.name, "entity_event") {
            let event_id = super::packet_field(packet, &["event", "event_id", "id"])
                .and_then(|value| value.trim().parse::<i32>().ok());
            if event_id != Some(PROTECTED_FROM_DEATH) {
                return true;
            }
            let Some(name) = super::packet_field(packet, &["name", "player", "entity_name"]) else {
                return true;
            };
            let name = name.trim().to_string();
            if name.is_empty() {
                return true;
            }

            let own = env
                .player()
                .map(|player| player.name.eq_ignore_ascii_case(&name))
                .unwrap_or(false);
            let friend = env.is_friend(&name);
            if (own && self.totems_ignore_own.get())
                || (friend && self.totems_ignore_friends.get())
                || (!friend && self.totems_ignore_others.get())
            {
                return true;
            }

            let pops = self.record_pop(&name);
            if self.totems_distance_check.get() {
                let distance = env
                    .entities()
                    .into_iter()
                    .find(|entity| {
                        entity
                            .name
                            .as_deref()
                            .map(|who| who.eq_ignore_ascii_case(&name))
                            .unwrap_or(false)
                    })
                    .map(|entity| entity.distance);
                if let Some(distance) = distance {
                    if distance > self.totems_distance.get() as f64 {
                        return true;
                    }
                }
            }

            let noun = if pops == 1 { "totem" } else { "totems" };
            // `// PORT GAP:` Java builds a coloured component and reuses one chat
            // id per player; `Env::chat` takes plain text.
            self.base
                .info(env, &format!("{name} popped {pops} {noun}."));
            return true;
        }

        // `// PORT GAP:` the pearl notice needs a thrown pearl's owner and start
        // position, which the decoded packet the port sees does not carry.
        if self.pearl.get() && super::packet_matches(&packet.name, "add_entity") {
            let _ = (&self.pearl_ignore_own, &self.pearl_ignore_friends);
        }

        true
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        // Joins/leaves queue, one line per `notification-delay` ticks.
        if !self.joins_leaves_mode.is("None") {
            self.timer += 1;
            while !self.message_queue.is_empty() && self.timer >= self.notification_delay.get() {
                self.timer = 0;
                let line = self.message_queue.remove(0);
                if self.simple_notifications.get() {
                    env.chat(&line);
                } else {
                    self.base.info(env, &line);
                }
            }
        }

        let entities = env.entities();

        // Totem pops of players that have since died.
        if self.totem_pops.get() {
            let mut deaths = Vec::new();
            for entity in &entities {
                if entity.health > 0.0 && entity.alive {
                    continue;
                }
                let Some(name) = entity.name.clone() else {
                    continue;
                };
                if let Some((_, pops)) = self
                    .totem_pop_map
                    .iter()
                    .find(|(who, _)| who.eq_ignore_ascii_case(&name))
                {
                    deaths.push((name, *pops));
                }
            }
            for (name, pops) in deaths {
                self.totem_pop_map
                    .retain(|(who, _)| !who.eq_ignore_ascii_case(&name));
                let noun = if pops == 1 { "totem" } else { "totems" };
                self.base
                    .info(env, &format!("{name} died after popping {pops} {noun}."));
            }
        }

        // Visual range: the diff of the entity list against the last tick.
        if self.visual_range.get() {
            let event = self.event.get().to_string();
            let wanted = self.entities.get();
            let mut now: Vec<(i32, String)> = Vec::new();
            for entity in &entities {
                let matches_kind = wanted.iter().any(|kind| super::same_id(kind, &entity.kind));
                if !matches_kind {
                    continue;
                }
                let Some(name) = entity.name.clone() else {
                    continue;
                };
                if entity.is_player
                    && self.visual_range_ignore_friends.get()
                    && env.is_friend(&name)
                {
                    continue;
                }
                if entity.is_player
                    && self.visual_range_ignore_fakes.get()
                    && name.starts_with("Fake")
                {
                    // `// PORT GAP:` fake players are a Meteor class; the port can
                    // only go by the name the mod gives them.
                    continue;
                }
                now.push((entity.id, name));
            }

            if event != "Despawn" {
                for (id, name) in &now {
                    let known = self.seen.iter().any(|(seen, _)| seen == id);
                    if !known {
                        let name = name.clone();
                        self.announce_visual_range(env, &name, true);
                    }
                }
            }
            if event != "Spawn" {
                let left: Vec<String> = self
                    .seen
                    .iter()
                    .filter(|(id, _)| !now.iter().any(|(current, _)| current == id))
                    .map(|(_, name)| name.clone())
                    .collect();
                for name in left {
                    self.announce_visual_range(env, &name, false);
                }
            }
            self.seen = now;
        }
    }

    fn info_string(&self) -> Option<String> {
        if self.announced == 0 {
            None
        } else {
            Some(self.announced.to_string())
        }
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(Notifier::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::EntityInfo;

    #[test]
    fn joins_are_queued_and_printed_after_the_login_burst() {
        let mut m = Notifier::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.on_game_joined(&mut env);
        m.joins_leaves_mode.set_index(1); // Joins

        let mut first = PacketInfo {
            name: "ClientboundPlayerInfoUpdatePacket".to_string(),
            clientbound: true,
            fields: vec![("names".to_string(), "Alex".to_string())],
        };
        m.on_receive_packet(&mut env, &mut first);
        assert_eq!(m.queued(), 0, "the login burst is not a join");

        let mut second = PacketInfo {
            name: "ClientboundPlayerInfoUpdatePacket".to_string(),
            clientbound: true,
            fields: vec![("names".to_string(), "Alex,Steve".to_string())],
        };
        m.on_receive_packet(&mut env, &mut second);
        assert_eq!(m.queued(), 2);

        m.on_tick(&mut env);
        assert_eq!(
            env.chats,
            vec!["[+] Alex".to_string(), "[+] Steve".to_string()]
        );
    }

    #[test]
    fn totem_pops_are_counted() {
        let mut m = Notifier::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);

        for _ in 0..2 {
            let mut pop = PacketInfo {
                name: "ClientboundEntityEventPacket".to_string(),
                clientbound: true,
                fields: vec![
                    ("event".to_string(), "35".to_string()),
                    ("name".to_string(), "Alex".to_string()),
                ],
            };
            m.on_receive_packet(&mut env, &mut pop);
        }

        assert_eq!(m.totem_pop_map(), &[("Alex".to_string(), 2)]);
        assert_eq!(env.chats.len(), 2);
        assert!(env.chats[1].contains("Alex popped 2 totems."));

        // Own pops are ignored when the setting says so.
        m.totems_ignore_own.set(true);
        let mut own = PacketInfo {
            name: "ClientboundEntityEventPacket".to_string(),
            clientbound: true,
            fields: vec![
                ("event".to_string(), "35".to_string()),
                ("name".to_string(), "Dev".to_string()),
            ],
        };
        m.on_receive_packet(&mut env, &mut own);
        assert_eq!(env.chats.len(), 2);
    }

    #[test]
    fn visual_range_reports_enter_and_leave() {
        let mut m = Notifier::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.visual_range.set(true);

        env.entities.push(EntityInfo {
            id: 9,
            kind: "minecraft:player".into(),
            name: Some("Alex".into()),
            is_player: true,
            health: 20.0,
            alive: true,
            distance: 8.0,
            ..Default::default()
        });
        m.on_tick(&mut env);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Alex has entered your visual range!")));

        env.entities.clear();
        m.on_tick(&mut env);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Alex has left your visual range!")));
        assert_eq!(m.announced(), 2);
    }
}
