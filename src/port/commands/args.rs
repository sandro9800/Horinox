//! The argument types (`commands/arguments/*.java`) in their parsed form.
//!
//! Brigadier parses each argument while it walks the command tree; the port
//! hands a command the whole token list, so the types become small resolvers
//! here. The error texts are the ones the Java argument types throw.

use crate::meteor_core::command::{CmdCtx, CmdError};
use crate::meteor_core::env::EntityInfo;
use crate::meteor_core::env::Env;
use crate::meteor_core::math::{BlockPos, Dir, Vec3d};

/// `PlayerArgumentType`: a player in the world, matched by name.
pub fn player(env: &dyn Env, name: &str) -> Option<EntityInfo> {
    env.entities().into_iter().find(|e| {
        e.is_player
            && e.name
                .as_deref()
                .map(|n| n.eq_ignore_ascii_case(name))
                .unwrap_or(false)
    })
}

/// `PlayerArgumentType.NO_SUCH_PLAYER`.
pub fn no_such_player(name: &str) -> CmdError {
    CmdError::new(format!("Player with name {} doesn't exist.", name))
}

/// `ItemArgument.item(REGISTRY_ACCESS)`.
///
/// Meteor validates the id against the client's item registry; Horizon exposes
/// no registry, so a bare name is namespaced and the obvious failures are the
/// ones reported (`DropCommand.NO_SUCH_ITEM` is `Items.AIR`).
pub fn item_id(raw: &str) -> Result<String, CmdError> {
    let id = namespace(raw);
    let (ns, path) = id.split_once(':').unwrap_or(("", ""));
    let valid = !ns.is_empty()
        && !path.is_empty()
        && path
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-');
    if !valid || path == "air" {
        return Err(CmdError::new("Could not find an item with that name!"));
    }
    Ok(id)
}

/// `BlockStateArgument.block(REGISTRY_ACCESS)` - the same normalisation, but the
/// block state's properties are kept.
pub fn block_state(raw: &str) -> Result<String, CmdError> {
    let (id, props) = match raw.split_once('[') {
        Some((id, rest)) => (namespace(id), Some(rest)),
        None => (namespace(raw), None),
    };
    let (_, path) = id.split_once(':').unwrap_or(("", ""));
    if path.is_empty() || path == "air" {
        return Err(CmdError::new(format!("{} is not a block.", raw)));
    }
    Ok(match props {
        Some(rest) => format!("{}[{}", id, rest),
        None => id,
    })
}

fn namespace(raw: &str) -> String {
    if raw.contains(':') {
        raw.to_string()
    } else {
        format!("minecraft:{}", raw)
    }
}

/// `DirectionArgumentType` / `Direction.CODEC`.
pub fn direction(raw: &str) -> Option<Dir> {
    match raw.to_ascii_lowercase().as_str() {
        "down" => Some(Dir::Down),
        "up" => Some(Dir::Up),
        "north" => Some(Dir::North),
        "south" => Some(Dir::South),
        "west" => Some(Dir::West),
        "east" => Some(Dir::East),
        _ => None,
    }
}

/// `Direction.toYRot()`: the yaw that faces the direction (south is 0).
pub fn direction_yaw(dir: Dir) -> f32 {
    match dir {
        Dir::South => 0.0,
        Dir::West => 90.0,
        Dir::North => 180.0,
        Dir::East => -90.0,
        Dir::Up | Dir::Down => 0.0,
    }
}

/// `Direction.getUnitVec3i().getY() * -90` - the pitch that faces it.
pub fn direction_pitch(dir: Dir) -> f32 {
    match dir {
        Dir::Up => -90.0,
        Dir::Down => 90.0,
        _ => 0.0,
    }
}

/// `TimeArgument.time()`: ticks, with `s`/`d`/`t` suffixes.
pub fn time_ticks(raw: &str) -> Option<u32> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }
    let (number, scale) = match s.chars().last()? {
        's' | 'S' => (&s[..s.len() - 1], 20.0),
        'd' | 'D' => (&s[..s.len() - 1], 24_000.0),
        't' | 'T' => (&s[..s.len() - 1], 1.0),
        _ => (s, 1.0),
    };
    number
        .parse::<f64>()
        .ok()
        .map(|v| (v * scale).round().max(0.0) as u32)
}

/// `BlockPosArgumentType.blockPos()`: `x y z`, `~ ~ ~` or `^ ^ ^`.
///
/// Three tokens are consumed starting at `first`; the caller checks
/// [`looks_like_pos`] first when the position is optional.
pub fn block_pos(ctx: &CmdCtx, first: usize) -> Result<BlockPos, CmdError> {
    let tokens: Vec<&str> = (first..first + 3).filter_map(|i| ctx.arg(i)).collect();
    if tokens.len() < 3 {
        return Err(CmdError::new("Incomplete position argument."));
    }
    let player = ctx.env.player();
    let local = tokens.iter().any(|t| t.starts_with('^'));
    let pos = if local {
        if !tokens.iter().all(|t| t.starts_with('^')) {
            return Err(CmdError::new(
                "Cannot mix world and local coordinates in one position.",
            ));
        }
        let Some(player) = player else {
            return Err(CmdError::new("Not in a world."));
        };
        let x = local_offset(tokens[0])?;
        let y = local_offset(tokens[1])?;
        let z = local_offset(tokens[2])?;
        local_position(&player, x, y, z).to_block_pos()
    } else {
        let base = player.as_ref().map(|p| p.pos).unwrap_or(Vec3d::ZERO);
        let x = world_offset(tokens[0], base.x)?;
        let y = world_offset(tokens[1], base.y)?;
        let z = world_offset(tokens[2], base.z)?;
        Vec3d::new(x, y, z).to_block_pos()
    };
    // `Level.isInSpawnableBounds`.
    if pos.x.abs() > 30_000_000 || pos.z.abs() > 30_000_000 || pos.y.abs() > 20_000_000 {
        return Err(CmdError::new("Position is out of bounds."));
    }
    Ok(pos)
}

/// `^ ^ ^` offsets: `^1.5` is a number, a bare `^` is zero.
fn local_offset(token: &str) -> Result<f64, CmdError> {
    let rest = token.trim_start_matches('^');
    if rest.is_empty() {
        return Ok(0.0);
    }
    rest.parse::<f64>()
        .map_err(|_| CmdError::new(format!("{} is not a number.", rest)))
}

/// `~ ~ ~` offsets: `~` is the player's own coordinate, `~1.5` relative to it.
fn world_offset(token: &str, base: f64) -> Result<f64, CmdError> {
    let t = token.trim();
    if t == "~" {
        return Ok(base);
    }
    if let Some(rest) = t.strip_prefix('~') {
        return rest
            .parse::<f64>()
            .map(|v| base + v)
            .map_err(|_| CmdError::new(format!("{} is not a number.", rest)));
    }
    t.parse::<f64>()
        .map_err(|_| CmdError::new(format!("{} is not a number.", t)))
}

/// `BlockPosArgumentType.LookingPosArgument.getPosition`: local coordinates are
/// built from the player's rotation and feet position.
fn local_position(player: &crate::meteor_core::env::PlayerInfo, x: f64, y: f64, z: f64) -> Vec3d {
    let rot = player.rotation();
    let rad = std::f64::consts::PI / 180.0;
    let yaw = rot.yaw as f64 + 90.0;
    let pitch = rot.pitch as f64;
    let f = (yaw * rad).cos();
    let g = (yaw * rad).sin();
    let h = (-pitch * rad).cos();
    let i = (-pitch * rad).sin();
    let j = ((-pitch + 90.0) * rad).cos();
    let k = ((-pitch + 90.0) * rad).sin();
    // forward, up and the right-hand vector
    let forward = Vec3d::new(f * h, i, g * h);
    let up = Vec3d::new(f * j, k, g * j);
    let right = Vec3d::new(
        -(forward.y * up.z - forward.z * up.y),
        -(forward.z * up.x - forward.x * up.z),
        -(forward.x * up.y - forward.y * up.x),
    );
    let feet = player.pos;
    feet.add(forward.mul(z)).add(up.mul(y)).add(right.mul(x))
}

/// Whether the next three tokens are a position (`BlockPosArgumentType`'s
/// presence in the tree decides this in Java).
pub fn looks_like_pos(ctx: &CmdCtx, first: usize) -> bool {
    let tokens: Vec<&str> = (first..first + 3).filter_map(|i| ctx.arg(i)).collect();
    tokens.len() == 3
        && tokens.iter().all(|t| {
            let t = t.trim();
            !t.is_empty()
                && (t.starts_with('~')
                    || t.starts_with('^')
                    || t.starts_with('-')
                    || t.starts_with('+')
                    || t.starts_with('.')
                    || t.chars()
                        .next()
                        .map(|c| c.is_ascii_digit())
                        .unwrap_or(false))
        })
}

/// The rest of the line from `first`, joined - `StringArgumentType.greedyString`.
pub fn greedy(ctx: &CmdCtx, first: usize) -> String {
    if first >= ctx.args.len() {
        return String::new();
    }
    ctx.args[first..].join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;

    fn ctx<'a>(env: &'a mut TestEnv, args: &[&str]) -> CmdCtx<'a> {
        CmdCtx::new(
            env,
            args.iter().map(|s| s.to_string()).collect(),
            args.join(" "),
        )
    }

    #[test]
    fn item_ids_are_namespaced() {
        assert_eq!(item_id("stone").unwrap(), "minecraft:stone");
        assert_eq!(item_id("minecraft:diamond").unwrap(), "minecraft:diamond");
        assert!(item_id("air").is_err());
        assert!(item_id("bad id").is_err());
    }

    #[test]
    fn time_argument_matches_vanilla() {
        assert_eq!(time_ticks("20"), Some(20));
        assert_eq!(time_ticks("1s"), Some(20));
        assert_eq!(time_ticks("1d"), Some(24_000));
        assert_eq!(time_ticks("0"), Some(0));
        assert_eq!(time_ticks("x"), None);
    }

    #[test]
    fn absolute_relative_and_local_positions() {
        let mut env = TestEnv::in_world();
        assert_eq!(
            block_pos(&ctx(&mut env, &["1", "2", "3"]), 0).unwrap(),
            BlockPos::new(1, 2, 3)
        );
        // The test player stands at 0.5 / 64 / 0.5.
        assert_eq!(
            block_pos(&ctx(&mut env, &["~", "~", "~"]), 0).unwrap(),
            BlockPos::new(0, 64, 0)
        );
        assert_eq!(
            block_pos(&ctx(&mut env, &["~2", "~1", "~-1"]), 0).unwrap(),
            BlockPos::new(2, 65, -1)
        );
        // yaw 0 looks south, so `^ ^ ^2` is two blocks south.
        assert_eq!(
            block_pos(&ctx(&mut env, &["^", "^", "^2"]), 0).unwrap(),
            BlockPos::new(0, 64, 2)
        );
        assert!(block_pos(&ctx(&mut env, &["1", "2"]), 0).is_err());
    }

    #[test]
    fn direction_and_looks_like_pos() {
        assert_eq!(direction("north"), Some(Dir::North));
        assert_eq!(direction_yaw(Dir::West), 90.0);
        assert_eq!(direction_pitch(Dir::Down), 90.0);
        let mut env = TestEnv::in_world();
        assert!(looks_like_pos(&ctx(&mut env, &["~", "~1", "~"]), 0));
        assert!(!looks_like_pos(
            &ctx(&mut env, &["home", "sweet", "home"]),
            0
        ));
    }
}
