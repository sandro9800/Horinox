fn horixon_settings_click(
    game: &mut crate::game::Game<'_>,
    mod_id: &str,
    button: usize,
) {
    use crate::game::horixon as m;
    if mod_id != m::ID || button == 0 { return; }

    m::load_settings(
        game.native_setting(m::ID, "array_list"),
        game.native_setting(m::ID, "coordinates"),
        game.native_setting(m::ID, "fps"),
    );

    let index = button - 1;
    if let Some(enabled) = m::toggle_module(2, index) {
        game.native_set_setting(
            m::ID,
            m::module_id(2, index),
            if enabled { "true" } else { "false" },
        );
    }
}
