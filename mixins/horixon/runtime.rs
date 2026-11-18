// Shared, testable HORIXON runtime logic.
// This file is both the standalone Cargo library and the injected
// crate::game::horixon module; no generated copy is maintained.

pub fn block_break_message(pos: [i32; 3]) -> String {
    format!(
        "[HORIXON] Block-break hook observed position {}, {}, {}",
        pos[0], pos[1], pos[2]
    )
}

pub fn before_break(pos: [i32; 3]) {
    eprintln!("{}", block_break_message(pos));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_preserves_signed_block_coordinates() {
        assert_eq!(
            block_break_message([-3, 64, 12]),
            "[HORIXON] Block-break hook observed position -3, 64, 12"
        );
    }
}
