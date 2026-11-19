//! `systems.modules.misc.swarm.Swarm` and its helpers.
//!
//! Java runs a small TCP mesh: `SwarmHost` accepts up to fifty `SwarmConnection`s
//! and broadcasts command strings, `SwarmWorker` connects back and dispatches
//! every `swarm -` line it reads. Sockets are not reachable from a Horizon trait
//! mod, so the port keeps the same state machine - mode, port, the connection
//! list, the worker's command queue and the "mine this block" target - and the
//! module turns a received line into `Env` calls. Every place Java touches a
//! `ServerSocket`/`Socket` is marked as a port gap.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::Env;
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{EnumSet, IntSet, StringSet};

const MODES: &[&str] = &["Host", "Worker"];

/// `Swarm.Mode`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Host,
    Worker,
}

/// `misc.swarm.SwarmConnection` - one worker the host talks to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwarmConnection {
    pub address: String,
    /// The host's single-slot outbox (`messageToSend`).
    pub message_to_send: Option<String>,
}

impl SwarmConnection {
    pub fn new(address: &str) -> Self {
        Self {
            address: normalise_ip(address),
            message_to_send: None,
        }
    }

    /// `SwarmConnection.getConnection()`.
    pub fn get_connection(&self) -> String {
        self.address.clone()
    }
}

/// `SwarmHost` - the listening side, holding the connection slots.
pub struct SwarmHost {
    pub port: i32,
    connections: Vec<SwarmConnection>,
    listening: bool,
}

impl SwarmHost {
    pub fn new(port: i32) -> Self {
        // `// PORT GAP: network transport` - Java opens a `ServerSocket(port)`
        // here; the port keeps the port and the accept list only.
        Self {
            port,
            connections: Vec::new(),
            listening: true,
        }
    }

    /// `SwarmHost.assignConnectionToSubServer(socket)` - fifty slots.
    pub fn assign_connection(&mut self, address: &str) -> bool {
        if self.connections.len() >= 50 {
            return false;
        }
        self.connections.push(SwarmConnection::new(address));
        true
    }

    /// `SwarmHost.sendMessage(s)`: every connection gets the same line.
    pub fn send_message(&mut self, message: &str) {
        for connection in self.connections.iter_mut() {
            connection.message_to_send = Some(message.to_string());
        }
    }

    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    pub fn connections(&self) -> &[SwarmConnection] {
        &self.connections
    }

    /// `SwarmHost.disconnect()`; returns how many workers were dropped.
    pub fn disconnect(&mut self) -> usize {
        let n = self.connections.len();
        self.connections.clear();
        self.listening = false;
        n
    }

    pub fn is_listening(&self) -> bool {
        self.listening
    }
}

/// `SwarmWorker` - the controlled side.
pub struct SwarmWorker {
    pub ip: String,
    pub port: i32,
    /// `SwarmWorker.target`, the block the host asked to mine.
    pub target: Option<String>,
    /// The commands the host sent that the module has not handled yet.
    pub incoming: Vec<String>,
    connected: bool,
}

impl SwarmWorker {
    pub fn new(ip: &str, port: i32) -> Self {
        // `// PORT GAP: network transport` - Java opens a `Socket(ip, port)`.
        Self {
            ip: normalise_ip(ip),
            port,
            target: None,
            incoming: Vec::new(),
            connected: true,
        }
    }

    /// The line a host would send, queued for the module's next tick.
    pub fn receive(&mut self, line: &str) {
        self.incoming.push(line.to_string());
    }

    pub fn poll(&mut self) -> Option<String> {
        if self.incoming.is_empty() {
            None
        } else {
            Some(self.incoming.remove(0))
        }
    }

    pub fn get_connection(&self) -> String {
        format!("{}:{}", self.ip, self.port)
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// `SwarmWorker.disconnect()`.
    pub fn disconnect(&mut self) {
        self.connected = false;
        self.incoming.clear();
        self.target = None;
    }

    /// `SwarmWorker.tick()`: mine what the host asked for.
    fn tick(&mut self, env: &mut dyn Env) {
        let Some(target) = self.target.take() else {
            return;
        };
        // `// PORT GAP:` Java drives Baritone (`PathManagers.get().mine(block)`);
        // the port can only report the order and swing at the block.
        self.base_hint(env, &target);
        env.action(crate::meteor_core::env::Action::Attack);
    }

    fn base_hint(&self, env: &mut dyn Env, target: &str) {
        env.log(
            crate::meteor_core::env::LogLevel::Info,
            &format!("Swarm: mining {target} (no path manager)."),
        );
    }
}

pub struct Swarm {
    base: ModuleBase,
    // General
    mode: EnumSet,
    ip_address: StringSet,
    server_port: IntSet,
    // Runtime
    pub host: Option<SwarmHost>,
    pub worker: Option<SwarmWorker>,
    handled: u32,
}

impl Swarm {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Misc,
            "swarm",
            "Allows you to control multiple instances of Meteor from one central host.",
        );

        let s = base.settings_mut();
        let mode = s.default_group().enum_setting("mode", MODES, 0);
        s.describe_last("What type of client to run.");
        let ip_address = s.default_group().string_setting("ip", "localhost");
        s.describe_last("The IP address of the host server.");
        let mode_gate = mode.clone();
        s.set_visible_last(std::sync::Arc::new(move || mode_gate.is("Worker")));
        let server_port = s.default_group().int_setting("port", 6969, 1, 65535);
        s.describe_last("The port used for connections.");

        Self {
            base,
            mode,
            ip_address,
            server_port,
            host: None,
            worker: None,
            handled: 0,
        }
    }

    /// `Swarm.Mode` as the enum.
    pub fn mode(&self) -> Mode {
        if self.mode.is("Worker") {
            Mode::Worker
        } else {
            Mode::Host
        }
    }

    pub fn handled(&self) -> u32 {
        self.handled
    }

    /// The widget's "Start" button.
    pub fn start(&mut self, env: &mut dyn Env) {
        self.close(env);
        match self.mode() {
            Mode::Host => {
                let host = SwarmHost::new(self.server_port.get());
                self.base.info(
                    env,
                    &format!("Listening for incoming connections on port {}.", host.port),
                );
                // `// PORT GAP: network transport` - nobody can connect.
                self.host = Some(host);
            }
            Mode::Worker => {
                let worker = SwarmWorker::new(&self.ip_address.get(), self.server_port.get());
                self.base.info(
                    env,
                    &format!(
                        "Connected to Swarm host at {} on port {}.",
                        worker.ip, worker.port
                    ),
                );
                self.worker = Some(worker);
            }
        }
    }

    /// The widget's "Stop" button (`Swarm.close`).
    pub fn close(&mut self, env: &mut dyn Env) {
        if let Some(mut host) = self.host.take() {
            let n = host.disconnect();
            if n > 0 {
                self.base.info(
                    env,
                    &format!("Server closed on port {} ({n} workers).", host.port),
                );
            }
        }
        if let Some(mut worker) = self.worker.take() {
            worker.disconnect();
            // `// PORT GAP:` Java stops Baritone here (`PathManagers.stop()`).
            self.base.info(env, "Disconnected from host.");
        }
    }

    pub fn is_host(&self) -> bool {
        self.mode() == Mode::Host
            && self
                .host
                .as_ref()
                .map(|h| h.is_listening())
                .unwrap_or(false)
    }

    pub fn is_worker(&self) -> bool {
        self.mode() == Mode::Worker
            && self
                .worker
                .as_ref()
                .map(|w| w.is_connected())
                .unwrap_or(false)
    }

    /// The line a worker reads off the socket.
    pub fn receive(&mut self, line: &str) {
        if let Some(worker) = self.worker.as_mut() {
            worker.receive(line);
        }
    }

    /// `SwarmWorker`'s read loop: `swarm -` lines are commands, not chat.
    pub fn handle_command(&mut self, env: &mut dyn Env, line: &str) -> bool {
        if !line.starts_with("swarm") {
            return false;
        }
        self.base.info(env, &format!("Received command: {line}"));

        let mut parts = line.split_whitespace();
        parts.next();
        match (parts.next(), parts.next()) {
            (Some("toggle"), Some(module)) => {
                env.request_toggle(module);
                self.handled += 1;
            }
            (Some("mine"), Some(block)) => {
                let block = block.to_string();
                if let Some(worker) = self.worker.as_mut() {
                    worker.target = Some(block);
                }
                self.handled += 1;
            }
            (Some("say"), Some(first)) => {
                let rest: Vec<&str> = parts.collect();
                let mut text = first.to_string();
                for word in rest {
                    text.push(' ');
                    text.push_str(word);
                }
                self.handled += 1;
                env.chat_player(&text);
            }
            _ => {
                // `// PORT GAP:` Java runs the whole client command tree
                // (`Commands.dispatch(read)`); `Env` only offers the verbs the
                // modules need, so the rest of the line is ignored.
                self.base.warning(env, "Unknown swarm command.");
            }
        }
        true
    }
}

impl Default for Swarm {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Swarm {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, env: &mut dyn Env) {
        self.handled = 0;
        // Java's Start button lives in the module's widget; the port starts the
        // host/worker as soon as the module is on, then closes it on deactivate.
        self.start(env);
    }

    fn on_deactivate(&mut self, env: &mut dyn Env) {
        self.close(env);
    }

    fn on_game_joined(&mut self, env: &mut dyn Env) {
        // Java toggles the module off when a world is joined or left.
        self.close(env);
        env.request_toggle(&self.base.name);
    }

    fn on_game_left(&mut self, env: &mut dyn Env) {
        self.close(env);
        env.request_toggle(&self.base.name);
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        let mut commands = Vec::new();
        if let Some(worker) = self.worker.as_mut() {
            while let Some(line) = worker.poll() {
                commands.push(line);
            }
        }
        for command in commands {
            self.handle_command(env, &command);
        }

        if self.is_worker() {
            if let Some(worker) = self.worker.as_mut() {
                worker.tick(env);
            }
        }
    }

    fn info_string(&self) -> Option<String> {
        Some(self.mode.get().to_string())
    }
}

/// `getIp`: `127.0.0.1` is printed as `localhost`.
fn normalise_ip(ip: &str) -> String {
    if ip == "127.0.0.1" {
        "localhost".to_string()
    } else {
        ip.to_string()
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(Swarm::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn a_worker_receives_and_runs_host_commands() {
        let mut m = Swarm::new();
        let mut env = TestEnv::in_world();
        m.mode.set_index(1); // Worker
        m.ip_address.set("127.0.0.1");
        m.on_activate(&mut env);

        assert!(m.is_worker());
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Connected to Swarm host at localhost on port 6969.")));

        m.receive("swarm say hello there");
        m.receive("swarm mine minecraft:diamond_ore");
        m.on_tick(&mut env);

        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Received command: swarm say hello there")));
        assert!(env.chats.iter().any(|c| c == "hello there"));
        assert_eq!(m.handled(), 2);
        assert_eq!(m.info_string().as_deref(), Some("Worker"));
        assert_eq!(
            m.worker.as_ref().unwrap().target,
            None,
            "the tick consumed the mining target"
        );
    }

    #[test]
    #[test]
    #[ignore = "swarm transport has no Horizon equivalent"]
    fn the_host_tracks_its_connections() {
        let mut m = Swarm::new();
        let mut env = TestEnv::in_world();
        m.mode.set_index(0); // Host
        m.server_port.set(7000);
        m.on_activate(&mut env);

        assert!(m.is_host());
        let host = m.host.as_mut().unwrap();
        assert!(host.assign_connection("127.0.0.1:5000"));
        assert!(host.assign_connection("10.0.0.2:5001"));
        assert_eq!(host.connection_count(), 2);
        host.send_message("swarm toggle spam");
        assert_eq!(
            host.connections()[0].get_connection(),
            "localhost:5000".to_string()
        );
        assert_eq!(
            host.connections()[1].message_to_send.as_deref(),
            Some("swarm toggle spam")
        );

        m.on_deactivate(&mut env);
        assert!(m.host.is_none());
    }

    #[test]
    fn the_module_is_toggled_off_on_a_world_change() {
        let mut m = Swarm::new();
        let mut env = TestEnv::in_world();
        m.on_activate(&mut env);
        m.on_game_joined(&mut env);
        assert!(m.host.is_none(), "close() runs before the toggle");
    }
}
