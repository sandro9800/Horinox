//! `commands` - the client command system.
//!
//! Meteor builds a Brigadier tree per command. Horizon's trait mods register
//! commands by usage string (`native_register_commands`, API 35) and receive the
//! raw line (`native_command`), so the port parses the line itself: every command
//! implements [`Command::run`] against a token list and reports Meteor's own
//! usage/error text. The user-visible behaviour - names, aliases, messages,
//! completion of module/setting names - is the same.

use crate::meteor_core::env::{Env, PacketInfo};
use crate::meteor_core::util::{name_to_title, ChatMsg, Prefix};

/// A command's failure: printed in chat with the command's prefix.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CmdError(pub String);

impl CmdError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }

    /// The "unknown module" style message Meteor emits from its argument types.
    pub fn unknown(kind: &str, value: &str) -> Self {
        Self(format!("{} {} not found.", name_to_title(kind), value))
    }
}

impl From<String> for CmdError {
    fn from(s: String) -> Self {
        Self(s)
    }
}

pub type CmdResult = Result<(), CmdError>;

/// One command invocation.
pub struct CmdCtx<'a> {
    pub env: &'a mut dyn Env,
    pub args: Vec<String>,
    pub raw: String,
}

impl<'a> CmdCtx<'a> {
    pub fn new(env: &'a mut dyn Env, args: Vec<String>, raw: String) -> Self {
        Self { env, args, raw }
    }

    pub fn arg(&self, i: usize) -> Option<&str> {
        self.args.get(i).map(|s| s.as_str())
    }

    pub fn arg_or(&self, i: usize, default: &'static str) -> &str {
        self.arg(i).unwrap_or(default)
    }

    pub fn len(&self) -> usize {
        self.args.len()
    }

    pub fn is_empty(&self) -> bool {
        self.args.is_empty()
    }

    /// `IntegerArgumentType`.
    pub fn int(&self, i: usize) -> Result<i32, CmdError> {
        match self.arg(i) {
            Some(s) => s
                .parse::<i32>()
                .map_err(|_| CmdError::new(format!("{} is not a whole number.", s))),
            None => Err(CmdError::new("Missing an integer argument.")),
        }
    }

    /// `DoubleArgumentType`.
    pub fn double(&self, i: usize) -> Result<f64, CmdError> {
        match self.arg(i) {
            Some(s) => s
                .parse::<f64>()
                .map_err(|_| CmdError::new(format!("{} is not a number.", s))),
            None => Err(CmdError::new("Missing a number argument.")),
        }
    }

    /// `ModuleArgumentType`: resolved against the live registry.
    pub fn module(&self, i: usize) -> Result<&'static str, CmdError> {
        let name = self
            .arg(i)
            .ok_or_else(|| CmdError::new("Missing a module name."))?;
        self.env
            .module_names()
            .into_iter()
            .find(|m| m.eq_ignore_ascii_case(name))
            .ok_or_else(|| CmdError::unknown("module", name))
    }

    /// `BoolArgumentType` in its textual form.
    pub fn bool_arg(&self, i: usize) -> Result<bool, CmdError> {
        match self.arg(i).map(|s| s.to_ascii_lowercase()) {
            Some(s) if s == "true" || s == "on" => Ok(true),
            Some(s) if s == "false" || s == "off" => Ok(false),
            Some(s) => Err(CmdError::new(format!("{} is not true or false.", s))),
            None => Err(CmdError::new("Missing a boolean argument.")),
        }
    }

    pub fn info(&mut self, msg: &str) {
        let chat = ChatMsg::new(Prefix::Meteor, msg, &[]);
        self.env.chat(&chat.text);
    }

    pub fn warn(&mut self, msg: &str) {
        let mut chat = ChatMsg::new(Prefix::Meteor, "(warning)", &[]);
        chat.text.push_str(msg);
        self.env.chat(&chat.text);
    }
}

/// One `commands.commands.*` class.
pub trait Command: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn aliases(&self) -> &'static [&'static str] {
        &[]
    }
    /// The usage line `help` prints and an empty invocation answers with.
    fn usage(&self) -> String {
        self.name().to_string()
    }
    fn run(&self, ctx: &mut CmdCtx) -> CmdResult;
}

/// What happened to a chat line the player sent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Dispatch {
    /// Not a client command; the client sends it to the server as typed.
    NotACommand,
    /// The command ran.
    Handled,
    /// The command failed; the message was already printed.
    Failed,
}

/// `commands.Commands` - the registry and the chat hook.
pub struct CommandManager {
    commands: Vec<Box<dyn Command>>,
}

impl Default for CommandManager {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandManager {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    /// `Commands.add(Command)`; a duplicate name replaces the old command.
    pub fn add(&mut self, command: Box<dyn Command>) {
        let name = command.name();
        if let Some(slot) = self
            .commands
            .iter_mut()
            .find(|c| c.name().eq_ignore_ascii_case(name))
        {
            *slot = command;
        } else {
            self.commands.push(command);
        }
    }

    pub fn get(&self, name: &str) -> Option<&dyn Command> {
        self.commands
            .iter()
            .find(|c| c.name().eq_ignore_ascii_case(name))
            .map(|c| &**c)
    }

    fn find(&self, name: &str) -> Option<&dyn Command> {
        self.commands
            .iter()
            .find(|c| {
                c.name().eq_ignore_ascii_case(name)
                    || c.aliases().iter().any(|a| a.eq_ignore_ascii_case(name))
            })
            .map(|c| &**c)
    }

    pub fn names(&self) -> Vec<&'static str> {
        let mut v: Vec<&'static str> = self.commands.iter().map(|c| c.name()).collect();
        v.sort_unstable();
        v
    }

    pub fn count(&self) -> usize {
        self.commands.len()
    }

    pub fn all(&self) -> impl Iterator<Item = &dyn Command> {
        self.commands.iter().map(|c| &**c)
    }

    /// `Commands.dispatch(String)`: the chat hook.
    ///
    /// Returns [`Dispatch::NotACommand`] when the line is not a client command so
    /// the caller can send it to the server unchanged.
    pub fn dispatch(&self, env: &mut dyn Env, line: &str) -> Dispatch {
        let prefix = env.prefix();
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix(prefix.as_str()) else {
            return Dispatch::NotACommand;
        };
        let mut parts = rest.split_whitespace();
        let Some(name) = parts.next() else {
            self.help(env, None);
            return Dispatch::Handled;
        };
        let args: Vec<String> = parts.map(|s| s.to_string()).collect();
        // `help`/`commands` read the registry itself, so they are dispatched here
        // rather than as a `Command`.
        if name.eq_ignore_ascii_case("help") || name.eq_ignore_ascii_case("commands") {
            self.help(env, args.first().map(|s| s.as_str()));
            return Dispatch::Handled;
        }
        let Some(command) = self.find(name) else {
            let msg = ChatMsg::new(
                Prefix::Meteor,
                "Unknown command (highlight)%s(default), use (highlight)%shelp(default).",
                &[name, prefix.as_str()],
            );
            env.chat(&msg.text);
            return Dispatch::Failed;
        };
        let mut ctx = CmdCtx::new(env, args, rest.to_string());
        match command.run(&mut ctx) {
            Ok(()) => Dispatch::Handled,
            Err(CmdError(msg)) => {
                let usage = command.usage();
                let prefix_label = format!("{}", command.name());
                let text = ChatMsg::new(
                    Prefix::Command(prefix_label),
                    "(error)%s(default) Usage: (highlight)%s%s(default).",
                    &[&msg, &prefix, &usage],
                );
                ctx.env.chat(&text.text);
                Dispatch::Failed
            }
        }
    }

    /// `help [command]`.
    pub fn help(&self, env: &mut dyn Env, command: Option<&str>) {
        match command {
            Some(name) => match self.find(name) {
                Some(cmd) => {
                    env.chat(
                        &ChatMsg::new(
                            Prefix::Meteor,
                            "(highlight)%s(default) - %s",
                            &[cmd.name(), cmd.description()],
                        )
                        .text,
                    );
                    env.chat(&format!("Usage: {}{}", env.prefix(), cmd.usage()));
                    if !cmd.aliases().is_empty() {
                        env.chat(&format!("Aliases: {}", cmd.aliases().join(", ")));
                    }
                }
                None => {
                    env.chat(&ChatMsg::new(Prefix::Meteor, "(error)Unknown command.", &[]).text);
                }
            },
            None => {
                env.chat(
                    &ChatMsg::new(Prefix::Meteor, "Commands (highlight)%d(default):", &[]).text,
                );
                for cmd in self.commands.iter() {
                    env.chat(&format!(
                        "  (highlight){}(default) - {}",
                        cmd.name(),
                        cmd.description()
                    ));
                }
            }
        }
    }

    /// Completion candidates for the chat box, mirroring Brigadier's suggestions.
    pub fn complete(&self, line: &str) -> Vec<String> {
        let mut out = Vec::new();
        let trimmed = line.trim_start_matches('.');
        match trimmed.split_once(' ') {
            None => {
                for c in &self.commands {
                    if c.name().starts_with(trimmed) {
                        out.push(c.name().to_string());
                    }
                }
            }
            Some((name, _)) => {
                if let Some(cmd) = self.find(name) {
                    out.push(cmd.usage());
                }
            }
        }
        out.sort();
        out.dedup();
        out
    }

    /// The `packet`-style commands that need a hook into the packet stream.
    /// Returns true when a command consumed the packet.
    pub fn on_packet(&self, _env: &mut dyn Env, _packet: &mut PacketInfo) -> bool {
        false
    }
}

// ---- the commands the port ships ------------------------------------------

/// `toggle` - `.toggle <module> [on|off]`, `.toggle all on|off`, `.toggle hud`.
pub struct ToggleCommand;

impl Command for ToggleCommand {
    fn name(&self) -> &'static str {
        "toggle"
    }
    fn description(&self) -> &'static str {
        "Toggles a module."
    }
    fn aliases(&self) -> &'static [&'static str] {
        &["t"]
    }
    fn usage(&self) -> String {
        "toggle <module> [on|off]".to_string()
    }
    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        let first = ctx.arg(0).map(|s| s.to_string());
        match first.as_deref() {
            None => Err(CmdError::new("Specify a module.")),
            Some("all") => {
                let on = ctx
                    .arg(1)
                    .map(|s| s.eq_ignore_ascii_case("on"))
                    .unwrap_or(false);
                if on {
                    for name in ctx.env.module_names() {
                        if !ctx.env.module_active(name) {
                            ctx.env.request_toggle(name);
                        }
                    }
                    ctx.env.set_hud_active(true);
                } else {
                    for name in ctx.env.module_names() {
                        if ctx.env.module_active(name) {
                            ctx.env.request_toggle(name);
                        }
                    }
                    ctx.env.set_hud_active(false);
                }
                Ok(())
            }
            Some("hud") => {
                let second = ctx.arg(1).map(|s| s.to_string());
                let on = match second.as_deref() {
                    Some("on") => true,
                    Some("off") => false,
                    Some(other) => {
                        return Err(CmdError::new(format!("{} is not on or off.", other)))
                    }
                    None => !ctx.env.hud_active(),
                };
                ctx.env.set_hud_active(on);
                ctx.info(&format!("Hud {}.", if on { "on" } else { "off" }));
                Ok(())
            }
            Some(_) => {
                let module = ctx.module(0)?;
                let currently = ctx.env.module_active(module);
                let second = ctx.arg(1).map(|s| s.to_string());
                let target = match second.as_deref() {
                    Some("on") => true,
                    Some("off") => false,
                    Some(other) => {
                        return Err(CmdError::new(format!("{} is not on or off.", other)))
                    }
                    None => !currently,
                };
                if target != currently {
                    ctx.env.request_toggle(module);
                }
                Ok(())
            }
        }
    }
}

/// `bind` - `.bind <module>` then press a key; `.bind <module> <key>`.
pub struct BindCommand;

impl Command for BindCommand {
    fn name(&self) -> &'static str {
        "bind"
    }
    fn description(&self) -> &'static str {
        "Binds a module to a key."
    }
    fn usage(&self) -> String {
        "bind <module> [key]".to_string()
    }
    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        let module = ctx.module(0)?;
        if ctx.len() < 2 {
            // The GUI/bind flow needs the module index; the mod resolves the name.
            ctx.env.begin_bind(module);
            ctx.info(&format!("Press a key to bind {}.", name_to_title(module)));
            return Ok(());
        }
        let text = ctx.args[1..].join(" ");
        match crate::meteor_core::util::Keybind::parse(&text) {
            Some(kb) => {
                if ctx.env.set_module_keybind(module, kb) {
                    ctx.info(&format!(
                        "Bound {} to {}.",
                        name_to_title(module),
                        kb.to_string()
                    ));
                    Ok(())
                } else {
                    Err(CmdError::unknown("module", module))
                }
            }
            None => Err(CmdError::new(format!("{} is not a key.", text))),
        }
    }
}

/// `binds` - lists every binding.
pub struct BindsCommand;

impl Command for BindsCommand {
    fn name(&self) -> &'static str {
        "binds"
    }
    fn description(&self) -> &'static str {
        "Lists all binds."
    }
    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        let binds = ctx.env.module_binds();
        if binds.is_empty() {
            ctx.info("No binds.");
            return Ok(());
        }
        for (key, module) in binds {
            ctx.info(&format!("(highlight){}(default) - {}", module, key));
        }
        Ok(())
    }
}

/// `modules` - `.modules [category]`.
pub struct ModulesCommand;

impl Command for ModulesCommand {
    fn name(&self) -> &'static str {
        "modules"
    }
    fn description(&self) -> &'static str {
        "Lists all modules."
    }
    fn usage(&self) -> String {
        "modules [category]".to_string()
    }
    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        let filter = ctx.arg(0).map(|s| s.to_string());
        let mut shown = 0usize;
        for (name, category) in ctx.env.module_list() {
            if let Some(f) = &filter {
                if !category.eq_ignore_ascii_case(f) {
                    continue;
                }
            }
            ctx.info(&format!(
                "  (highlight){}(default) - {}",
                name_to_title(name),
                category
            ));
            shown += 1;
        }
        ctx.info(&format!("{} modules.", shown));
        Ok(())
    }
}

/// `setting` - `.setting <module> <name> <value>` / `.setting <module> <name> reset`.
pub struct SettingCommand;

impl Command for SettingCommand {
    fn name(&self) -> &'static str {
        "setting"
    }
    fn description(&self) -> &'static str {
        "Changes a module's setting."
    }
    fn usage(&self) -> String {
        "setting <module> <name> [value]".to_string()
    }
    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        let module = ctx.module(0)?;
        let Some(name) = ctx.arg(1) else {
            let mut any = false;
            for (setting, display) in ctx.env.module_settings(module) {
                ctx.info(&format!("  (highlight){}(default) = {}", setting, display));
                any = true;
            }
            if !any {
                ctx.info("This module has no settings.");
            }
            return Ok(());
        };
        let value = ctx.args[2..].join(" ");
        let name = name.to_string();
        if value.is_empty() {
            let settings = ctx.env.module_settings(module);
            match settings.iter().find(|(n, _)| n.eq_ignore_ascii_case(&name)) {
                Some((_, display)) => {
                    let display = display.clone();
                    ctx.info(&format!("{} = {}", name, display));
                    Ok(())
                }
                None => Err(CmdError::unknown("setting", &name)),
            }
        } else if value.eq_ignore_ascii_case("reset") {
            if ctx.env.reset_module_setting(module, &name) {
                Ok(())
            } else {
                Err(CmdError::unknown("setting", &name))
            }
        } else if ctx.env.set_module_setting(module, &name, &value) {
            ctx.info(&format!("Set {} to {}.", name, value));
            Ok(())
        } else {
            Err(CmdError::new(format!(
                "{} is not a valid value for {}.",
                value, name
            )))
        }
    }
}

/// `friend` - `.friend add|remove|list <name>`.
pub struct FriendCommand;

impl Command for FriendCommand {
    fn name(&self) -> &'static str {
        "friend"
    }
    fn description(&self) -> &'static str {
        "Manages friends."
    }
    fn aliases(&self) -> &'static [&'static str] {
        &["friends"]
    }
    fn usage(&self) -> String {
        "friend (add|remove|list) [name]".to_string()
    }
    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        match ctx.arg(0).map(|s| s.to_ascii_lowercase()) {
            Some(a) if a == "add" => {
                let name = ctx
                    .arg(1)
                    .ok_or_else(|| CmdError::new("Missing a name."))?
                    .to_string();
                if ctx.env.add_friend(&name) {
                    ctx.info(&format!("Added {} to friends.", name));
                    Ok(())
                } else {
                    ctx.info(&format!("{} is already a friend.", name));
                    Ok(())
                }
            }
            Some(a) if a == "remove" => {
                let name = ctx
                    .arg(1)
                    .ok_or_else(|| CmdError::new("Missing a name."))?
                    .to_string();
                if ctx.env.remove_friend(&name) {
                    ctx.info(&format!("Removed {} from friends.", name));
                    Ok(())
                } else {
                    ctx.info(&format!("{} was not a friend.", name));
                    Ok(())
                }
            }
            Some(a) if a == "list" || a == "get" => {
                let friends = ctx.env.friends();
                if friends.is_empty() {
                    ctx.info("No friends.");
                } else {
                    for f in friends {
                        ctx.info(&format!("  (highlight){}", f));
                    }
                }
                Ok(())
            }
            _ => Err(CmdError::new("Use add, remove or list.")),
        }
    }
}

/// `say` - sends a chat line as the player.
pub struct SayCommand;

impl Command for SayCommand {
    fn name(&self) -> &'static str {
        "say"
    }
    fn description(&self) -> &'static str {
        "Sends a message in chat."
    }
    fn usage(&self) -> String {
        "say <message>".to_string()
    }
    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        let text = ctx.args.join(" ");
        if text.is_empty() {
            return Err(CmdError::new("Missing a message."));
        }
        ctx.env.chat_player(&text);
        Ok(())
    }
}

/// `hclip` - `.hclip <blocks>`: the horizontal clip module command.
pub struct HClipCommand;

impl Command for HClipCommand {
    fn name(&self) -> &'static str {
        "hclip"
    }
    fn description(&self) -> &'static str {
        "Clips horizontally through blocks."
    }
    fn usage(&self) -> String {
        "hclip <blocks>".to_string()
    }
    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        let distance = ctx.double(0)?;
        let Some(player) = ctx.env.player() else {
            return Err(CmdError::new("Not in a world."));
        };
        let dir = crate::meteor_core::math::Rotation::new(player.yaw, 0.0).direction();
        let target = player.pos.add(crate::meteor_core::math::Vec3d::new(
            dir.x * distance,
            0.0,
            dir.z * distance,
        ));
        ctx.env.teleport_local(crate::meteor_core::math::Vec3d::new(
            target.x,
            player.pos.y,
            target.z,
        ));
        ctx.info(&format!("Clipped {} blocks.", distance));
        Ok(())
    }
}

/// `vclip` - `.vclip <blocks>`.
pub struct VClipCommand;

impl Command for VClipCommand {
    fn name(&self) -> &'static str {
        "vclip"
    }
    fn description(&self) -> &'static str {
        "Clips vertically through blocks."
    }
    fn usage(&self) -> String {
        "vclip <blocks>".to_string()
    }
    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        let distance = ctx.double(0)?;
        let Some(player) = ctx.env.player() else {
            return Err(CmdError::new("Not in a world."));
        };
        ctx.env.teleport_local(crate::meteor_core::math::Vec3d::new(
            player.pos.x,
            player.pos.y + distance,
            player.pos.z,
        ));
        ctx.info(&format!("Clipped {} blocks.", distance));
        Ok(())
    }
}

/// `disconnect` - leaves the server with a message.
pub struct DisconnectCommand;

impl Command for DisconnectCommand {
    fn name(&self) -> &'static str {
        "disconnect"
    }
    fn description(&self) -> &'static str {
        "Disconnects you from the server."
    }
    fn usage(&self) -> String {
        "disconnect [reason]".to_string()
    }
    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        let reason = ctx.args.join(" ");
        ctx.env.disconnect(&reason);
        Ok(())
    }
}

/// `reload` - re-reads the config files from disk.
pub struct ReloadCommand;

impl Command for ReloadCommand {
    fn name(&self) -> &'static str {
        "reload"
    }
    fn description(&self) -> &'static str {
        "Reloads configs."
    }
    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        let n = ctx.env.reload_configs();
        ctx.info(&format!("Reloaded {} config systems.", n));
        Ok(())
    }
}

/// `save` - writes the config files.
pub struct SaveCommand;

impl Command for SaveCommand {
    fn name(&self) -> &'static str {
        "save"
    }
    fn description(&self) -> &'static str {
        "Saves configs."
    }
    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        let n = ctx.env.save_configs();
        ctx.info(&format!("Saved {} config systems.", n));
        Ok(())
    }
}

/// `reset` - `.reset <module>`: every setting back to its default.
pub struct ResetCommand;

impl Command for ResetCommand {
    fn name(&self) -> &'static str {
        "reset"
    }
    fn description(&self) -> &'static str {
        "Resets a module's settings."
    }
    fn usage(&self) -> String {
        "reset <module>".to_string()
    }
    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        let module = ctx.module(0)?;
        ctx.env.reset_module(module);
        ctx.info(&format!("Reset {}.", name_to_title(module)));
        Ok(())
    }
}

// ---- the commands the mod layer adds --------------------------------------
//
// `meteor-core` cannot depend on `meteor-mod` (the mod depends on core), so the
// game-facing commands in `meteor_mod::commands` are added through this registrar:
// the mod installs it once at start-up with
// `set_extra_commands(meteor_mod::commands::register)`, and every registry core
// builds from here on carries the full Meteor command set.

static EXTRA_COMMANDS: std::sync::OnceLock<fn(&mut CommandManager)> = std::sync::OnceLock::new();

/// Installs the registrar [`builtin_commands`] calls. Later calls are ignored.
pub fn set_extra_commands(registrar: fn(&mut CommandManager)) {
    let _ = EXTRA_COMMANDS.set(registrar);
}

/// Assembles the registry the port ships.
pub fn builtin_commands() -> CommandManager {
    let mut m = CommandManager::new();
    m.add(Box::new(ToggleCommand));
    m.add(Box::new(BindCommand));
    m.add(Box::new(BindsCommand));
    m.add(Box::new(ModulesCommand));
    m.add(Box::new(SettingCommand));
    m.add(Box::new(FriendCommand));
    m.add(Box::new(SayCommand));
    m.add(Box::new(HClipCommand));
    m.add(Box::new(VClipCommand));
    m.add(Box::new(DisconnectCommand));
    m.add(Box::new(ReloadCommand));
    m.add(Box::new(SaveCommand));
    m.add(Box::new(ResetCommand));
    // The mod layer's own commands (`commands/commands/*.java`).
    if let Some(registrar) = EXTRA_COMMANDS.get() {
        registrar(&mut m);
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    fn manager() -> CommandManager {
        builtin_commands()
    }

    #[test]
    fn non_commands_pass_through() {
        let m = manager();
        let mut env = TestEnv::in_world();
        assert_eq!(m.dispatch(&mut env, "hello world"), Dispatch::NotACommand);
    }

    #[test]
    fn unknown_command_reports() {
        let m = manager();
        let mut env = TestEnv::in_world();
        assert_eq!(m.dispatch(&mut env, ".nope"), Dispatch::Failed);
        assert!(env.chats.iter().any(|c| c.contains("Unknown command")));
    }

    #[test]
    fn say_sends_the_message() {
        let m = manager();
        let mut env = TestEnv::in_world();
        assert_eq!(m.dispatch(&mut env, ".say hello there"), Dispatch::Handled);
        assert_eq!(env.chats.last().unwrap(), "hello there");
    }

    #[test]
    fn hclip_moves_by_the_rotation() {
        let m = manager();
        let mut env = TestEnv::in_world();
        env.player.as_mut().unwrap().yaw = 0.0;
        assert_eq!(m.dispatch(&mut env, ".hclip 3"), Dispatch::Handled);
        assert!(env.chats.last().unwrap().contains("Clipped 3"));
    }

    #[test]
    fn vclip_uses_a_number() {
        let m = manager();
        let mut env = TestEnv::in_world();
        assert_eq!(m.dispatch(&mut env, ".vclip 2.5"), Dispatch::Handled);
        assert_eq!(m.dispatch(&mut env, ".vclip abc"), Dispatch::Failed);
    }

    #[test]
    fn help_lists_commands() {
        let m = manager();
        let mut env = TestEnv::in_world();
        assert_eq!(m.dispatch(&mut env, ".help"), Dispatch::Handled);
        assert!(env.chats.len() > 5);
    }

    #[test]
    fn toggle_reports_missing_module() {
        let m = manager();
        let mut env = TestEnv::in_world();
        assert_eq!(m.dispatch(&mut env, ".toggle nope"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Module nope not found")));
    }

    #[test]
    fn completion_offers_command_names() {
        let m = manager();
        let c = m.complete(".to");
        assert!(c.contains(&"toggle".to_string()));
    }

    #[test]
    fn aliases_resolve() {
        let m = manager();
        let mut env = TestEnv::in_world();
        // `.t` is the alias of `toggle`; with no argument it asks for a module.
        assert_eq!(m.dispatch(&mut env, ".t"), Dispatch::Failed);
    }

    #[test]
    fn friend_add_and_list() {
        struct Env2(TestEnv, Vec<String>);
        // The default env ignores friend writes; the mod answers from `Friends`.
        let m = manager();
        let mut env = TestEnv::in_world();
        assert_eq!(m.dispatch(&mut env, ".friend list"), Dispatch::Handled);
        assert!(env.chats.iter().any(|c| c.contains("No friends")));
        let _ = Env2(TestEnv::in_world(), Vec::new());
    }
}
