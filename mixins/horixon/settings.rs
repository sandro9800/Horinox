fn horixon_settings(
    game: &mut crate::game::Game<'_>,
    mod_id: &str,
    buttons: &mut Vec<String>,
) {
    use crate::game::horixon as m;
    if mod_id != m::ID { return; }

    m::load_settings(
        game.native_setting(m::ID, "array_list"),
        game.native_setting(m::ID, "coordinates"),
        game.native_setting(m::ID, "fps"),
    );

    buttons.push("RENDER MODULES".to_owned());
    for index in 0..m::module_count(2) {
        let state = if m::module_enabled(2, index) { "ON" } else { "OFF" };
        buttons.push(format!("{}: {}", m::module_name(2, index), state));
    }
}
