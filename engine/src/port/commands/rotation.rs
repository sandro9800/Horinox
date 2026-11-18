//! `systems.commands.RotationCommand` -?`.rotation set|add <direction|pitch> [yaw]`.
//!
//! Fully expressible through `Env::set_rotation`, so this is a straight port:
//! `set <direction>` uses `Direction.toYRot()` and `getUnitVec3i().getY() * -90`,
//! `add` clamps the pitch to -90 and wraps the yaw with `Mth.wrapDegrees`.

use crate::meteor_core::command::{CmdCtx, CmdError, CmdResult, Command};
use crate::meteor_core::env::Env;
use crate::meteor_core::math::{normalize_yaw, Rotation};

use crate::port::commands::args;
use crate::port::commands::float_between;

pub struct RotationCommand;

impl Command for RotationCommand {
    fn name(&self) -> &'static str {
        "rotation"
    }

    fn description(&self) -> &'static str {
        "Modifies your rotation."
    }

    fn usage(&self) -> String {
        "rotation set <direction|pitch> [yaw] | rotation add <pitch> [yaw]".to_string()
    }

    fn run(&self, ctx: &mut CmdCtx) -> CmdResult {
        let Some(action) = ctx.arg(0).map(|s| s.to_ascii_lowercase()) else {
            return Err(CmdError::new("Use set or add."));
        };

        match action.as_str() {
            "set" => {
                let Some(value) = ctx.arg(1).map(|s| s.to_string()) else {
                    return Err(CmdError::new("Specify a direction, a pitch or a yaw."));
                };
                // `DirectionArgumentType` first, then `FloatArgumentType`.
                if let Some(dir) = args::direction(&value) {
                    let rotation = Rotation::new(
                        normalize_yaw(args::direction_yaw(dir)),
                        args::direction_pitch(dir),
                    );
                    ctx.env.set_rotation(rotation);
                    return Ok(());
                }
                let pitch = float_between(ctx, 1, -90.0, 90.0)?;
                let yaw = if ctx.len() > 2 {
                    float_between(ctx, 2, -180.0, 180.0)?
                } else {
                    ctx.env.rotation().yaw
                };
                ctx.env
                    .set_rotation(Rotation::new(normalize_yaw(yaw), pitch));
                Ok(())
            }
            "add" => {
                let pitch_delta = float_between(ctx, 1, -90.0, 90.0)?;
                let current = ctx.env.rotation();
                // `pitch >= 0 ? min(pitch, 90) : max(pitch, -90)` is a clamp.
                let pitch = (current.pitch + pitch_delta).clamp(-90.0, 90.0);
                let yaw = if ctx.len() > 2 {
                    let yaw_delta = float_between(ctx, 2, -180.0, 180.0)?;
                    normalize_yaw(current.yaw + yaw_delta)
                } else {
                    current.yaw
                };
                ctx.env.set_rotation(Rotation::new(yaw, pitch));
                Ok(())
            }
            other => Err(CmdError::new(format!("{} is not set or add.", other))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::command::Dispatch;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::Env;
    use crate::port::commands::command_manager;

    #[test]
    fn rotation_set_by_direction_and_numbers() {
        let m = command_manager();
        let mut env = TestEnv::in_world();

        assert_eq!(m.dispatch(&mut env, ".rotation set up"), Dispatch::Handled);
        assert_eq!(env.rotation, Rotation::new(0.0, -90.0));
        assert_eq!(
            m.dispatch(&mut env, ".rotation set west"),
            Dispatch::Handled
        );
        assert_eq!(env.rotation, Rotation::new(90.0, 0.0));

        assert_eq!(
            m.dispatch(&mut env, ".rotation set 45 90"),
            Dispatch::Handled
        );
        assert_eq!(env.rotation, Rotation::new(90.0, 45.0));
        // Pitch only keeps the yaw.
        assert_eq!(m.dispatch(&mut env, ".rotation set -20"), Dispatch::Handled);
        assert_eq!(env.rotation, Rotation::new(90.0, -20.0));
    }

    #[test]
    fn rotation_add_clamps_and_wraps() {
        let m = command_manager();
        let mut env = TestEnv::in_world();
        env.rotation = Rotation::new(170.0, 80.0);
        assert_eq!(
            m.dispatch(&mut env, ".rotation add 30 30"),
            Dispatch::Handled
        );
        assert_eq!(env.rotation.pitch, 90.0);
        assert_eq!(env.rotation.yaw, -160.0);
    }

    #[test]
    fn rotation_error_paths() {
        let m = command_manager();
        let mut env = TestEnv::in_world();
        assert_eq!(m.dispatch(&mut env, ".rotation set nope"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("nope is not a number.")));
        assert_eq!(m.dispatch(&mut env, ".rotation set 100"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("Float must not be greater than 90")));
        assert_eq!(m.dispatch(&mut env, ".rotation"), Dispatch::Failed);
        assert!(env.chats.iter().any(|c| c.contains("Use set or add.")));
        assert_eq!(m.dispatch(&mut env, ".rotation nudge 10"), Dispatch::Failed);
        assert!(env
            .chats
            .iter()
            .any(|c| c.contains("nudge is not set or add.")));
    }
}
