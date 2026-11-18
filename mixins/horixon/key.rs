fn horixon_key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    use crate::game::horixon as m;
    m::handle_key(game.native_playing(), game.native_on(m::ID), key, pressed);
}
