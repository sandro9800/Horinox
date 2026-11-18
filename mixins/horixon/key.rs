fn horixon_key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    use crate::game::horixon as m;
    m::load_settings(
        game.native_setting(m::ID, "array_list"),
        game.native_setting(m::ID, "coordinates"),
        game.native_setting(m::ID, "fps"),
    );
    if let Some((setting, enabled)) = m::handle_key(
        game.native_playing(), game.native_on(m::ID), key, pressed,
    ) {
        game.native_set_setting(m::ID, setting, if enabled { "true" } else { "false" });
    }
}
