//! `systems.modules.world.Collisions` - add collision boxes to the blocks and
//! areas Meteor lets you block off (magma, fire, pressure plates, -) and stop
//! movement into unloaded chunks.
//!
//! Java answers `CollisionShapeEvent` and rewrites the movement vector in
//! `PlayerMoveEvent`; neither hook exists in Horizon's mod API, so the port
//! computes the same set of shapes for the blocks around the player each tick
//! and leaves them for the host to use as collision overrides.

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, PacketInfo};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::BlockPos;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, ListSet};

use super::{liquid_source, listed};

pub struct Collisions {
    base: ModuleBase,
    /// `blocks` - `BlockListSetting` of blocks that get a full collision box.
    pub blocks: ListSet,
    pub magma: BoolSet,
    pub unloaded_chunks: BoolSet,
    ignore_border_flag: BoolSet,
    /// The positions whose shape the module overrides this tick.
    shapes: Vec<BlockPos>,
    /// `magma && !player.isShiftKeyDown() && air above magma block`.
    magma_blocked: bool,
}

impl Collisions {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::World,
            "collisions",
            "Adds collision boxes to certain blocks/areas.",
        );
        let mut s = base.settings_mut();
        let g = s.default_group();

        let blocks = g.list_setting("blocks", &[]);
        super::describe(g, "What blocks should be added collision box.");

        let magma = g.bool_setting("magma", false);
        super::describe(g, "Prevents you from walking over magma blocks.");

        let unloaded_chunks = g.bool_setting("unloaded-chunks", false);
        super::describe(g, "Stops you from going into unloaded chunks.");

        let ignore_border_flag = g.bool_setting("ignore-border", false);
        super::describe(g, "Removes world border collision.");

        Self {
            base,
            blocks,
            magma,
            unloaded_chunks,
            ignore_border_flag,
            shapes: Vec::new(),
            magma_blocked: false,
        }
    }

    /// `Collisions.ignoreBorder()`.
    pub fn ignore_border(&self) -> bool {
        self.ignore_border_flag.get()
    }

    /// The collision shapes the module is overriding this tick.
    pub fn shapes(&self) -> &[BlockPos] {
        &self.shapes
    }

    /// `Collisions.onCollisionShape` for one position.
    fn shape_at(&self, env: &dyn Env, pos: BlockPos) -> bool {
        let Some(state) = env.block(pos) else {
            return false;
        };
        // `if (!event.state.getFluidState().isEmpty()) return;`
        if liquid_source(&state).is_some() {
            return false;
        }
        if listed(&self.blocks, &state.id) {
            return true;
        }
        let sneaking = env.player().map(|p| p.sneaking).unwrap_or(false);
        if self.magma.get() && !sneaking && state.is_air() {
            return env
                .block(pos.offset(0, -1, 0))
                .is_some_and(|below| below.is("minecraft:magma_block"));
        }
        false
    }

    /// `PlayerUtils.isWithinChunkLoaded(...)`.
    ///
    /// PORT GAP: `Level.getChunkSource().hasChunk(x, z)` - the `Env` has no chunk
    /// query, so an unknown block (a position the host cannot read) is treated as
    /// unloaded.
    fn chunk_loaded(env: &dyn Env, pos: BlockPos) -> bool {
        env.block(pos).is_some()
    }
}

impl Default for Collisions {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Collisions {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        self.shapes.clear();
        self.magma_blocked = false;
        let Some(player) = env.player() else { return };
        if env.world().is_none() {
            return;
        }
        let feet = player.pos.to_block_pos();

        // The positions the player can collide with: the 3x3x3 around their feet,
        // in a deterministic order so a test can rely on it.
        for dy in -1..=1 {
            for dz in -1..=1 {
                for dx in -1..=1 {
                    let pos = feet.offset(dx, dy, dz);
                    if self.shape_at(env, pos) {
                        if self.magma.get() && pos == feet {
                            self.magma_blocked = true;
                        }
                        self.shapes.push(pos);
                    }
                }
            }
        }
    }

    fn on_send_packet(&mut self, env: &mut dyn Env, packet: &mut PacketInfo) -> bool {
        if !self.unloaded_chunks.get() || packet.clientbound {
            return true;
        }
        let name = packet.name.to_ascii_lowercase();
        if name != "move_player" && name != "move_vehicle" {
            return true;
        }
        // `packet.getX(mc.player.getX()) >> 4` - the packet only carries the
        // absolute position when the player moved, so fall back to the player.
        let Some(pos) = super::packet_pos(packet) else {
            return true;
        };
        if Self::chunk_loaded(env, pos) {
            return true;
        }
        // `event.cancel()`: the server must not see a move into an unloaded chunk.
        false
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(Collisions::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    #[test]
    fn settings_match_the_java_module() {
        let m = Collisions::new();
        let names: Vec<&str> = m.base.settings.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["blocks", "magma", "unloaded-chunks", "ignore-border"]
        );
        assert_eq!(m.base.title, "Collisions");
    }

    #[test]
    fn a_listed_block_gets_a_full_collision_box() {
        let mut m = Collisions::new();
        let mut env = TestEnv::in_world();
        m.blocks.set(vec!["minecraft:fire".to_string()]);
        env.push_block(BlockPos::new(1, 64, 0), "minecraft:fire");

        m.on_tick(&mut env);
        assert!(
            m.shapes().contains(&BlockPos::new(1, 64, 0)),
            "fire is in the list, so it collides"
        );
        assert!(!m.shapes().contains(&BlockPos::new(1, 65, 0)));
    }

    #[test]
    fn magma_stops_you_walking_over_it_until_you_sneak() {
        let mut m = Collisions::new();
        let mut env = TestEnv::in_world();
        // TestEnv's player stands at (0.5, 64, 0.5).
        env.push_block(BlockPos::new(0, 63, 0), "minecraft:magma_block");
        env.push_block(BlockPos::new(0, 64, 0), "minecraft:air");

        m.on_tick(&mut env);
        assert!(!m.magma_blocked, "the setting is off by default");
        assert!(m.shapes().is_empty());

        m.magma.set(true);
        m.on_tick(&mut env);
        assert!(m.magma_blocked, "air above magma collides");
        assert!(m.shapes().contains(&BlockPos::new(0, 64, 0)));

        // `!mc.player.isShiftKeyDown()`
        env.player.as_mut().unwrap().sneaking = true;
        m.on_tick(&mut env);
        assert!(!m.magma_blocked, "sneaking lets you stay on the magma");
        assert!(m.shapes().is_empty());
    }

    #[test]
    fn fluids_never_get_a_collision_box_and_the_border_setting_is_readable() {
        let mut m = Collisions::new();
        let mut env = TestEnv::in_world();
        m.blocks.set(vec!["minecraft:water".to_string()]);
        env.push_block(BlockPos::new(0, 64, 0), "minecraft:water[level=0]");
        m.on_tick(&mut env);
        assert!(
            m.shapes().is_empty(),
            "`!event.state.getFluidState().isEmpty()`"
        );

        assert!(!m.ignore_border());
        m.ignore_border_flag.set(true);
        assert!(m.ignore_border());
    }

    #[test]
    fn moves_into_unloaded_chunks_are_cancelled() {
        let mut m = Collisions::new();
        let mut env = TestEnv::in_world();
        m.unloaded_chunks.set(true);
        let mut packet = super::super::pos_packet("move_player", BlockPos::new(16, 64, 0));
        // TestEnv answers every position, so the chunk counts as loaded.
        assert!(m.on_send_packet(&mut env, &mut packet));

        struct NoBlocks(TestEnv);
        impl Env for NoBlocks {
            fn player(&self) -> Option<crate::meteor_core::env::PlayerInfo> {
                self.0.player()
            }
            fn world(&self) -> Option<crate::meteor_core::env::WorldInfo> {
                self.0.world()
            }
            fn entities(&self) -> Vec<crate::meteor_core::env::EntityInfo> {
                Vec::new()
            }
            fn crosshair_entity(&self) -> Option<crate::meteor_core::env::EntityInfo> {
                None
            }
            fn raycast(&self, _: f32) -> crate::meteor_core::env::Raycast {
                crate::meteor_core::env::Raycast::Miss
            }
            fn block(&self, _: BlockPos) -> Option<crate::meteor_core::env::BlockInfo> {
                None
            }
            fn block_light(&self, _: BlockPos) -> (u8, u8) {
                (0, 0)
            }
            fn screen(&self) -> crate::meteor_core::env::Screen {
                crate::meteor_core::env::Screen::Playing
            }
            fn now(&self) -> f64 {
                0.0
            }
            fn inventory(&self) -> Vec<crate::meteor_core::env::InvSlot> {
                Vec::new()
            }
            fn selected_stack(&self) -> Option<crate::meteor_core::env::ItemStackInfo> {
                None
            }
            fn key_down(&self, _: i32) -> bool {
                false
            }
            fn mouse_down(&self, _: i32) -> bool {
                false
            }
            fn rotation(&self) -> crate::meteor_core::math::Rotation {
                self.0.rotation()
            }
            fn can_update(&self) -> bool {
                true
            }
            fn ticks(&self) -> u64 {
                0
            }
            fn tick_rate(&self) -> f32 {
                20.0
            }
            fn recent_tps(&self) -> f32 {
                20.0
            }
            fn set_rotation(&mut self, _: crate::meteor_core::math::Rotation) {}
            fn chat(&mut self, _: &str) {}
            fn chat_player(&mut self, _: &str) {}
            fn command(&mut self, _: &str) {}
            fn action(&mut self, _: crate::meteor_core::env::Action) {}
            fn click_slot(&mut self, _: usize, _: i32, _: crate::meteor_core::env::ClickMode) {}
            fn set_block(&mut self, _: BlockPos, _: &str) -> bool {
                false
            }
            fn drop_stack(&mut self, _: usize, _: bool) {}
            fn use_item_on(
                &mut self,
                _: BlockPos,
                _: crate::meteor_core::math::Dir,
                _: crate::meteor_core::env::Hand,
            ) {
            }
            fn attack_entity(&mut self, _: i32) {}
            fn set_packet_rate(&mut self, _: f32) {}
            fn send_packet(&mut self, _: crate::meteor_core::env::PacketInfo) {}
            fn cancel_packet(&mut self, _: &str) {}
            fn play_sound(&mut self, _: &str, _: f32, _: f32) {}
            fn log(&mut self, _: crate::meteor_core::env::LogLevel, _: &str) {}
        }

        let mut unloaded = NoBlocks(TestEnv::in_world());
        let mut packet = super::super::pos_packet("move_player", BlockPos::new(16, 64, 0));
        assert!(
            !m.on_send_packet(&mut unloaded, &mut packet),
            "a cancelled move never reaches the server"
        );
    }
}
