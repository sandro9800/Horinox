//! `commands.commmands.*` - the game-facing half of Meteor's command set.
//!
//! The systems half (`.toggle`, `.bind`, `.setting`, `.say`, -) lives in
//! `crate::meteor_core::command`; the commands that touch the game live here, one file
//! per `commands/commands/<Name>Command.java`. Every one of them implements
//! [`crate::meteor_core::command::Command`] and reaches the game through
//! [`CmdCtx::env`], so the same code runs against the mod's `HorizonEnv` and
//! against `TestEnv` in the unit tests.
//!
//! Java builds a Brigadier tree per command and calls `mc.*` directly; the port
//! parses the token list itself (see `crate::meteor_core::command`) and goes through
//! `Env`. Where Java calls something Horizon's mod API does not expose, the file
//! carries a `// PORT GAP:` comment naming the missing hook and the command
//! reports the gap to the player instead of panicking.
//!
//! [`register`] adds the whole set to a `CommandManager` and
//! [`command_manager`] returns the full registry (core systems commands plus
//! these), which is what `shared::Shared::new` builds.

use crate::meteor_core::command::{CmdCtx, CmdError, CommandManager};
use crate::meteor_core::env::Env;
use crate::meteor_core::env::{GameMode, ItemStackInfo, PacketInfo, PlayerInfo};
use crate::meteor_core::util::{ChatMsg, Prefix};

pub mod args;
pub mod data;
pub mod state;

pub mod damage;
pub mod dismount;
pub mod drop;
pub mod enchant;
pub mod enderchest;
pub mod fakeplayer;
pub mod fov;
pub mod gamemode;
pub mod give;
pub mod input;
pub mod inventory;
pub mod locate;
pub mod macro_command;
pub mod namehistory;
pub mod nbt;
pub mod notebot;
pub mod peek;
pub mod profiles;
pub mod rotation;
pub mod savemap;
pub mod server;
pub mod spectate;
pub mod swarm;
pub mod wasp;
pub mod waypoint;

/// Adds every command in this package to `m`.
///
/// `crate::meteor_core::command::builtin_commands` calls this through
/// [`crate::meteor_core::command::set_extra_commands`], so the registry the mod shares
/// carries the full Meteor command set.
pub fn register(m: &mut CommandManager) {
    m.add(Box::new(damage::DamageCommand));
    m.add(Box::new(dismount::DismountCommand));
    m.add(Box::new(drop::DropCommand));
    m.add(Box::new(enchant::EnchantCommand));
    m.add(Box::new(enderchest::EnderChestCommand));
    m.add(Box::new(fakeplayer::FakePlayerCommand));
    m.add(Box::new(fov::FovCommand));
    m.add(Box::new(gamemode::GamemodeCommand));
    m.add(Box::new(give::GiveCommand));
    m.add(Box::new(input::InputCommand));
    m.add(Box::new(inventory::InventoryCommand));
    m.add(Box::new(locate::LocateCommand));
    m.add(Box::new(macro_command::MacroCommand));
    m.add(Box::new(namehistory::NameHistoryCommand));
    m.add(Box::new(nbt::NbtCommand));
    m.add(Box::new(notebot::NotebotCommand));
    m.add(Box::new(peek::PeekCommand));
    m.add(Box::new(profiles::ProfilesCommand));
    m.add(Box::new(rotation::RotationCommand));
    m.add(Box::new(savemap::SaveMapCommand));
    m.add(Box::new(server::ServerCommand));
    m.add(Box::new(spectate::SpectateCommand));
    m.add(Box::new(swarm::SwarmCommand));
    m.add(Box::new(wasp::WaspCommand));
    m.add(Box::new(waypoint::WaypointCommand));
}

/// The registry the mod ships: `Commands.init()`'s list.
pub fn command_manager() -> CommandManager {
    let mut m = crate::meteor_core::command::builtin_commands();
    register(&mut m);
    m
}

// ---- helpers shared by the command files ----------------------------------

/// Java's `Command.error(...)`: a red line with the command's own prefix, which
/// does **not** abort the command (Brigadier's `SINGLE_SUCCESS` still returns).
pub(crate) fn error(ctx: &mut CmdCtx, command: &str, msg: &str) {
    let chat = ChatMsg::new(
        Prefix::Command(command.to_string()),
        "(error)%s(default)",
        &[msg],
    );
    ctx.env.chat(&chat.text);
}

/// `IntegerArgumentType.integer(min, max)` - Brigadier's own range messages.
pub(crate) fn int_between(ctx: &CmdCtx, i: usize, min: i32, max: i32) -> Result<i32, CmdError> {
    let value = ctx.int(i)?;
    if value < min {
        return Err(CmdError::new(format!(
            "Integer must not be less than {}, found {}.",
            min, value
        )));
    }
    if value > max {
        return Err(CmdError::new(format!(
            "Integer must not be greater than {}, found {}.",
            max, value
        )));
    }
    Ok(value)
}

/// `FloatArgumentType.floatArg()`.
pub(crate) fn float(ctx: &CmdCtx, i: usize) -> Result<f32, CmdError> {
    match ctx.arg(i) {
        Some(s) => s
            .parse::<f32>()
            .map_err(|_| CmdError::new(format!("{} is not a number.", s))),
        None => Err(CmdError::new("Missing a number argument.")),
    }
}

/// `FloatArgumentType.floatArg(min, max)`.
pub(crate) fn float_between(ctx: &CmdCtx, i: usize, min: f32, max: f32) -> Result<f32, CmdError> {
    let value = float(ctx, i)?;
    if value < min {
        return Err(CmdError::new(format!(
            "Float must not be less than {}, found {}.",
            min, value
        )));
    }
    if value > max {
        return Err(CmdError::new(format!(
            "Float must not be greater than {}, found {}.",
            max, value
        )));
    }
    Ok(value)
}

/// `mc.player.getAbilities().instabuild` / `isCreative()`.
pub(crate) fn creative(ctx: &CmdCtx) -> Result<PlayerInfo, CmdError> {
    let Some(player) = ctx.env.player() else {
        return Err(CmdError::new("You must be in creative mode to use this."));
    };
    if player.game_mode != GameMode::Creative {
        return Err(CmdError::new("You must be in creative mode to use this."));
    }
    Ok(player)
}

/// `mc.player.isSpectator()`.
pub(crate) fn is_spectator(ctx: &CmdCtx) -> bool {
    ctx.env
        .player()
        .map(|p| p.game_mode == GameMode::Spectator)
        .unwrap_or(false)
}

/// `mc.player.getInventory().getSelectedItem()`.
pub(crate) fn selected(ctx: &CmdCtx) -> Option<ItemStackInfo> {
    ctx.env.selected_stack().filter(|s| !s.is_empty())
}

/// The slot the creative-slot packets address: `36 + selectedSlot`.
pub(crate) fn creative_slot(player: &PlayerInfo) -> usize {
    36 + player.selected_slot
}

/// `ServerboundSetCreativeModeSlotPacket` - how Meteor writes a stack back to
/// the server (`give`, `enchant`, `nbt`).
pub(crate) fn creative_slot_packet(
    slot: usize,
    item: &str,
    count: i32,
    extra: Vec<(String, String)>,
) -> PacketInfo {
    let mut fields = vec![
        ("slot".to_string(), slot.to_string()),
        ("item".to_string(), item.to_string()),
        ("count".to_string(), count.to_string()),
    ];
    fields.extend(extra);
    PacketInfo {
        name: "ServerboundSetCreativeModeSlotPacket".to_string(),
        clientbound: false,
        fields,
    }
}

// ---- the tick and packet hooks the command set needs ----------------------

/// `TickEvent.Post` for the command state that counts in ticks: `input`'s
/// keypress handlers, `macro`'s scheduler and `notebot`'s recorder.
///
/// Java subscribes these from `MeteorClient.EVENT_BUS`; a Horizon mod has no
/// event bus, so the mod's client-tick hook calls this once per tick.
pub fn on_tick(env: &mut dyn Env) {
    state::input_tick(env);
    state::macro_tick(env);
    state::notebot_tick();
}

/// `PacketEvent.Receive` for the commands that read the packet stream:
/// `locate`'s eye-of-ender trajectory, `notebot`'s sound capture and `server`'s
/// command tree. Returns true when a command consumed the packet.
pub fn on_receive_packet(env: &mut dyn Env, packet: &PacketInfo) -> bool {
    let mut consumed = false;
    if locate::on_receive(env, packet) {
        consumed = true;
    }
    if notebot::on_receive(packet) {
        consumed = true;
    }
    if server::on_receive(packet) {
        consumed = true;
    }
    consumed
}

/// `PacketEvent.Send`: `server plugins` drops the suggestion packet it sent.
pub fn on_send_packet(packet: &PacketInfo) -> bool {
    server::on_send(packet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::command::Dispatch;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::Env;

    /// Every command `commands/commands/` holds, in `Commands.init()` order for
    /// the ones the core does not own.
    pub(crate) const PORTED: [&str; 25] = [
        "damage",
        "dismount",
        "drop",
        "enchant",
        "ender-chest",
        "fake-player",
        "fov",
        "gamemode",
        "give",
        "input",
        "inventory",
        "locate",
        "macro",
        "name-history",
        "nbt",
        "notebot",
        "peek",
        "profiles",
        "rotation",
        "save-map",
        "server",
        "spectate",
        "swarm",
        "wasp",
        "waypoint",
    ];

    #[test]
    fn every_command_is_registered() {
        let m = command_manager();
        for name in PORTED {
            assert!(m.get(name).is_some(), "{name} is missing from the registry");
        }
        // 13 systems commands in core + 25 here.
        assert_eq!(m.count(), 13 + PORTED.len());
    }

    #[test]
    fn help_and_commands_take_the_command_names() {
        let m = command_manager();
        assert!(m.complete(".way").contains(&"waypoint".to_string()));
        // The core dispatcher answers `help <command>` from the same registry.
        let mut env = TestEnv::in_world();
        assert_eq!(m.dispatch(&mut env, ".help waypoint"), Dispatch::Handled);
        assert!(env.chats.iter().any(|c| c.contains("Manages waypoints.")));
    }

    #[test]
    fn unknown_command_still_reports() {
        let m = command_manager();
        let mut env = TestEnv::in_world();
        assert_eq!(m.dispatch(&mut env, ".nope"), Dispatch::Failed);
    }
}
