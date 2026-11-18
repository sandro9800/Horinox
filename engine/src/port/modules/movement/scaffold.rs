//! `systems.modules.movement.Scaffold` - automatically places blocks under you.

use crate::meteor_core::category::Category;
use crate::meteor_core::color::SettingColor;
use crate::meteor_core::env::{Action, ClickMode, Env, Renderer};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::math::{BlockPos, Rotation, Vec3d};
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, ColorSet, DoubleSet, EnumSet, IntSet, ListSet};

use super::{
    add_group, block_items, describe_in, has_collision, hotbar_block_item, is_moving, jumping,
    shift_held, visible_in, MOUSE_USE,
};

pub struct Scaffold {
    base: ModuleBase,
    // General
    /// `blocks`
    blocks: ListSet,
    /// `blocks-filter`
    blocks_filter: EnumSet,
    /// `fast-tower`
    fast_tower: BoolSet,
    /// `tower-speed`
    tower_speed: DoubleSet,
    /// `while-moving`
    while_moving: BoolSet,
    /// `only-on-click`
    only_on_click: BoolSet,
    /// `swing`
    render_swing: BoolSet,
    /// `auto-switch`
    auto_switch: BoolSet,
    /// `rotate`
    rotate: BoolSet,
    /// `air-place`
    air_place: BoolSet,
    /// `ahead-distance`
    ahead_distance: DoubleSet,
    /// `closest-block-range`
    place_range: DoubleSet,
    /// `radius`
    radius: DoubleSet,
    /// `blocks-per-tick`
    blocks_per_tick: IntSet,
    // Render
    /// `render`
    render: BoolSet,
    /// `shape-mode`
    shape_mode: EnumSet,
    /// `side-color`
    side_color: ColorSet,
    /// `line-color`
    line_color: ColorSet,

    /// The blocks that were just placed, for `RenderUtils.renderTickingBlock`.
    placed: Vec<(BlockPos, u64)>,
}

impl Scaffold {
    pub fn new() -> Self {
        let mut base = ModuleBase::new(
            Category::Movement,
            "scaffold",
            "Automatically places blocks under you.",
        );
        let mut s = base.settings_mut();
        let blocks = s.default_group().list_setting("blocks", &[]);
        let blocks_filter =
            s.default_group()
                .enum_setting("blocks-filter", &["Whitelist", "Blacklist"], 1);
        let fast_tower = s.default_group().bool_setting("fast-tower", false);
        let tower_speed = s
            .default_group()
            .double_setting("tower-speed", 0.5, 0.0, 1.0, 3);
        let while_moving = s.default_group().bool_setting("while-moving", false);
        let only_on_click = s.default_group().bool_setting("only-on-click", false);
        let render_swing = s.default_group().bool_setting("swing", false);
        let auto_switch = s.default_group().bool_setting("auto-switch", true);
        let rotate = s.default_group().bool_setting("rotate", true);
        let air_place = s.default_group().bool_setting("air-place", false);
        let ahead_distance = s
            .default_group()
            .double_setting("ahead-distance", 0.0, 0.0, 1.0, 3);
        let place_range = s
            .default_group()
            .double_setting("closest-block-range", 4.0, 0.0, 8.0, 3);
        let radius = s.default_group().double_setting("radius", 0.0, 0.0, 6.0, 3);
        let blocks_per_tick = s.default_group().int_setting("blocks-per-tick", 3, 1, 64);

        let render_group = add_group(&mut s, "Render");
        let render = s.groups[render_group].bool_setting("render", true);
        let shape_mode =
            s.groups[render_group].enum_setting("shape-mode", &["Lines", "Sides", "Both"], 2);
        let side_color = s.groups[render_group]
            .color_setting("side-color", SettingColor::new(197, 137, 232, 10));
        let line_color =
            s.groups[render_group].color_setting("line-color", SettingColor::rgb(197, 137, 232));

        describe_in(
            &mut s,
            "General",
            &[
                ("blocks", "Selected blocks."),
                ("blocks-filter", "How to use the block list setting"),
                ("fast-tower", "Whether or not to scaffold upwards faster."),
                ("tower-speed", "The speed at which to tower."),
                ("while-moving", "Allows you to tower while moving."),
                (
                    "only-on-click",
                    "Only places blocks when holding right click.",
                ),
                ("swing", "Renders your client-side swing."),
                (
                    "auto-switch",
                    "Automatically swaps to a block before placing.",
                ),
                ("rotate", "Rotates towards the blocks being placed."),
                (
                    "air-place",
                    "Allow air place. This also allows you to modify scaffold radius.",
                ),
                ("ahead-distance", "How far ahead to place blocks."),
                (
                    "closest-block-range",
                    "How far can scaffold place blocks when you are in air.",
                ),
                ("radius", "Scaffold radius."),
                ("blocks-per-tick", "How many blocks to place in one tick."),
            ],
        );
        describe_in(
            &mut s,
            "Render",
            &[
                ("render", "Whether to render blocks that have been placed."),
                ("shape-mode", "How the shapes are rendered."),
                (
                    "side-color",
                    "The side color of the target block rendering.",
                ),
                (
                    "line-color",
                    "The line color of the target block rendering.",
                ),
            ],
        );
        {
            let c = fast_tower.clone();
            visible_in(&mut s, "General", "tower-speed", move || c.get());
            let c = fast_tower.clone();
            visible_in(&mut s, "General", "while-moving", move || c.get());
            let c = air_place.clone();
            visible_in(&mut s, "General", "ahead-distance", move || !c.get());
            let c = air_place.clone();
            visible_in(&mut s, "General", "closest-block-range", move || !c.get());
            let c = air_place.clone();
            visible_in(&mut s, "General", "radius", move || c.get());
            let c = air_place.clone();
            visible_in(&mut s, "General", "blocks-per-tick", move || c.get());
        }
        {
            let c = render.clone();
            visible_in(&mut s, "Render", "shape-mode", move || c.get());
            let c = render.clone();
            visible_in(&mut s, "Render", "side-color", move || c.get());
            let c = render.clone();
            visible_in(&mut s, "Render", "line-color", move || c.get());
        }

        Self {
            base,
            blocks,
            blocks_filter,
            fast_tower,
            tower_speed,
            while_moving,
            only_on_click,
            render_swing,
            auto_switch,
            rotate,
            air_place,
            ahead_distance,
            place_range,
            radius,
            blocks_per_tick,
            render,
            shape_mode,
            side_color,
            line_color,
            placed: Vec::new(),
        }
    }

    /// `scaffolding()`.
    pub fn scaffolding(&self, env: &dyn Env) -> bool {
        !self.only_on_click.get() || env.mouse_down(MOUSE_USE)
    }

    /// `validItem(itemStack, pos)` - the list filter plus "is a full block".
    ///
    /// PORT GAP: `Block.isShapeFullBlock(shaped)` and the `FallingBlock` check
    /// have no `Env` query; the block list is the whole filter here.
    fn valid_item(&self, id: &str) -> bool {
        let listed = self.blocks.contains(id);
        let allowed = if self.blocks_filter.is("Whitelist") {
            listed
        } else {
            !listed
        };
        allowed
    }

    /// `BlockUtils.getPlaceSide(bp)`.
    fn place_side(&self, env: &dyn Env, pos: BlockPos) -> Option<BlockPos> {
        for dir in BlockPos::NEIGHBORS {
            let neighbour = pos.offset_dir(dir);
            if has_collision(env, neighbour) {
                return Some(neighbour);
            }
        }
        None
    }

    /// `place(bp)`: swap to a block, aim, and write the block.
    fn place(&mut self, env: &mut dyn Env, target: BlockPos, side: Option<BlockPos>) -> bool {
        let (slot, id) = match hotbar_block_item(env) {
            Some(found) => found,
            None => {
                if !self.auto_switch.get() {
                    return false;
                }
                match block_items(env).into_iter().next() {
                    Some((slot, id)) => {
                        // `InvUtils.swap(slot, true)` - a hotbar swap click.
                        env.click_slot(slot, 0, ClickMode::Swap);
                        (slot, id)
                    }
                    None => return false,
                }
            }
        };
        let _ = slot;
        if !self.valid_item(&id) {
            return false;
        }

        if self.rotate.get() {
            if let Some(side) = side {
                let Some(player) = env.player() else {
                    return false;
                };
                let rotation = Rotation::between(player.eye_pos(), side.center());
                env.set_rotation(rotation);
            }
        }

        if !env.set_block(target, &id) {
            return false;
        }
        let _ = self.render_swing.get();
        self.placed.push((target, env.ticks()));
        true
    }

    /// The block the module would place into (`BlockPos.MutableBlockPos bp`).
    fn target_pos(&self, env: &dyn Env) -> Option<BlockPos> {
        let player = env.player()?;
        let vec = player
            .pos
            .add(player.velocity)
            .add(Vec3d::new(0.0, -0.75, 0.0));

        let mut pos = player.pos;
        if !self.air_place.get()
            && self.ahead_distance.get() != 0.0
            && self
                .place_side(env, player.pos.to_block_pos().offset(0, -1, 0))
                .is_some()
        {
            let dir = Rotation::new(player.yaw, 0.0).direction();
            let ahead = self.ahead_distance.get();
            if env.key_down(super::keys::W) {
                pos = pos.add(Vec3d::new(dir.x, 0.0, dir.z).mul(ahead));
            }
            if env.key_down(super::keys::S) {
                pos = pos.add(Vec3d::new(dir.x, 0.0, dir.z).mul(-ahead));
            }
            if env.key_down(super::keys::A) {
                pos = pos.add(Vec3d::new(dir.z, 0.0, -dir.x).mul(ahead));
            }
            if env.key_down(super::keys::D) {
                pos = pos.add(Vec3d::new(-dir.z, 0.0, dir.x).mul(ahead));
            }
        }

        let mut x = pos.x.floor() as i32;
        let mut y = vec.y.floor() as i32;
        let mut z = pos.z.floor() as i32;

        if shift_held(env) && !jumping(env) && (player.pos.y + vec.y) > -1.0 {
            y -= 1;
        }
        if y >= player.pos.to_block_pos().y {
            y = player.pos.to_block_pos().y - 1;
        }

        // The "closest block" search when there is nothing to place against.
        if !self.air_place.get() && self.place_side(env, BlockPos::new(x, y, z)).is_none() {
            let range = self.place_range.get();
            let mut best: Option<(f64, BlockPos)> = None;
            let min_y = env.world().map(|w| w.min_y).unwrap_or(-64) as f64;
            let height = env.world().map(|w| w.height).unwrap_or(384) as f64;

            let mut bx = (player.pos.x - range).floor() as i32;
            while (bx as f64) < player.pos.x + range {
                let mut bz = (player.pos.z - range).floor() as i32;
                while (bz as f64) < player.pos.z + range {
                    let mut by = min_y.max(player.pos.y - range).floor() as i32;
                    while (by as f64) < height.min(player.pos.y + range) {
                        let candidate = BlockPos::new(bx, by, bz);
                        if self.place_side(env, candidate).is_some()
                            && !has_collision(env, candidate)
                        {
                            let d = candidate.distance(BlockPos::new(x, y, z));
                            if best.as_ref().map(|(bd, _)| d < *bd).unwrap_or(true) {
                                best = Some((d, candidate));
                            }
                        }
                        by += 1;
                    }
                    bz += 1;
                }
                bx += 1;
            }

            match best {
                Some((_, candidate)) => {
                    x = candidate.x;
                    y = candidate.y;
                    z = candidate.z;
                }
                None => return None,
            }
        }

        Some(BlockPos::new(x, y, z))
    }
}

impl Default for Scaffold {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for Scaffold {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_deactivate(&mut self, _env: &mut dyn Env) {
        self.placed.clear();
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        if self.only_on_click.get() && !env.mouse_down(MOUSE_USE) {
            return;
        }

        let Some(target) = self.target_pos(env) else {
            return;
        };

        if self.air_place.get() {
            let Some(player) = env.player() else { return };
            let radius = self.radius.get();
            let mut blocks: Vec<BlockPos> = Vec::new();
            let mut x = (target.x as f64 - radius).floor() as i32;
            while (x as f64) <= target.x as f64 + radius {
                let mut z = (target.z as f64 - radius).floor() as i32;
                while (z as f64) <= target.z as f64 + radius {
                    let pos = BlockPos::new(x, target.y, z);
                    if player.pos.distance_to(pos.center()) <= radius
                        || (x == target.x && z == target.z)
                    {
                        blocks.push(pos);
                    }
                    z += 1;
                }
                x += 1;
            }

            blocks.sort_by(|a, b| {
                a.center()
                    .distance_to(player.pos)
                    .partial_cmp(&b.center().distance_to(player.pos))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            let mut counter = 0;
            for block in blocks {
                if self.place(env, block, None) {
                    counter += 1;
                }
                if counter >= self.blocks_per_tick.get() {
                    break;
                }
            }
        } else {
            let side = self.place_side(env, target);
            self.place(env, target, side);
        }

        // `fastTower`: hold jump to keep placing under yourself.
        if self.fast_tower.get() && jumping(env) && !shift_held(env) {
            let has_block = hotbar_block_item(env).is_some() || self.auto_switch.get();
            if has_block && (self.while_moving.get() || !is_moving(env)) {
                // PORT GAP: `setDeltaMovement(y = towerSpeed)` - the `Env` has no
                // velocity write, so the tower is asked for with a jump.
                env.action(Action::Jump);
                let _speed = self.tower_speed.get();
            }
        }
    }

    fn on_render_world(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        if !self.render.get() {
            return;
        }
        let now = env.ticks();
        self.placed.retain(|(_, at)| now.saturating_sub(*at) <= 8);

        let side = self.side_color.color();
        let line = self.line_color.color();
        let lines = self.shape_mode.is("Lines") || self.shape_mode.is("Both");
        let sides = self.shape_mode.is("Sides") || self.shape_mode.is("Both");

        for (pos, _) in &self.placed {
            let min = Vec3d::new(pos.x as f64, pos.y as f64, pos.z as f64);
            let max = Vec3d::new(pos.x as f64 + 1.0, pos.y as f64 + 1.0, pos.z as f64 + 1.0);
            if sides {
                r.filled_box(min, max, side.argb(), true);
            }
            if lines {
                r.box_outline(min, max, line.argb(), 1.0, true);
            }
        }
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(Scaffold::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::NullRenderer;
    use crate::meteor_core::math::Vec3d;

    fn placeable(env: &mut TestEnv) {
        // A block to place against, one to the east of the block under the feet.
        env.push_block(BlockPos::new(1, 63, 0), "minecraft:stone");
        env.inventory.push(crate::meteor_core::env::InvSlot {
            index: 0,
            stack: Some(crate::meteor_core::env::ItemStackInfo {
                id: "minecraft:obsidian".into(),
                count: 64,
                ..Default::default()
            }),
        });
    }

    #[test]
    fn a_block_is_placed_under_the_feet() {
        let mut m = Scaffold::new();
        let mut env = TestEnv::in_world();
        placeable(&mut env);

        m.on_tick(&mut env);
        assert_eq!(env.placed.len(), 1, "placed was {:?}", env.placed);
        assert_eq!(env.placed[0].0, BlockPos::new(0, 63, 0));
        assert_eq!(env.placed[0].1, "minecraft:obsidian");
    }

    #[test]
    fn rotate_aims_at_the_place_side() {
        let mut m = Scaffold::new();
        let mut env = TestEnv::in_world();
        placeable(&mut env);

        m.on_tick(&mut env);
        // The support block is to the east, so the yaw points towards +x.
        assert!(env.rotation.yaw.abs() > 1.0, "yaw {}", env.rotation.yaw);
    }

    #[test]
    fn the_placement_is_rendered_for_a_few_ticks() {
        let mut m = Scaffold::new();
        let mut env = TestEnv::in_world();
        placeable(&mut env);
        m.on_tick(&mut env);

        let mut r = NullRenderer::default();
        m.on_render_world(&mut env, &mut r);
        assert_eq!(r.boxes, 2, "a filled box and its outline");

        env.ticks = 20;
        m.on_render_world(&mut env, &mut r);
        assert_eq!(m.placed.len(), 0, "the placed block stops rendering");
    }

    #[test]
    fn only_on_click_needs_the_use_key() {
        let mut m = Scaffold::new();
        let mut env = TestEnv::in_world();
        placeable(&mut env);
        m.only_on_click.set(true);
        env.player.as_mut().unwrap().pos = Vec3d::new(0.5, 64.0, 0.5);

        m.on_tick(&mut env);
        assert!(env.placed.is_empty());
        assert!(env.rotation.yaw.abs() < 1.0e-6);
    }
}
