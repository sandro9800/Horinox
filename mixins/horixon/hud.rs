fn horixon_hud(
    game: &mut crate::game::Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
    corners: &mut horizon_client::extras_hud::Corners,
) {
    use crate::game::horixon as m;
    use horizon_client::extras_hud::Corner;

    if !game.native_on(m::ID) { return; }
    m::load_settings(
        game.native_setting(m::ID, "array_list"),
        game.native_setting(m::ID, "coordinates"),
        game.native_setting(m::ID, "fps"),
    );

    let mut lines = Vec::new();
    if m::fps_enabled() {
        lines.push(m::status_line(game.native_frame_rate().0));
    }
    if m::coordinates_enabled() {
        if let Some(player) = game.native_player() {
            lines.push(m::position_line(player.x, player.y, player.z, player.dimension));
        } else {
            lines.push("WAITING FOR WORLD".to_owned());
        }
    }

    if m::menu_is_open() && game.native_playing() && game.native_corner_shown(Corner::TopLeft) {
        let width = 310.0;
        let module_count = m::module_count(m::selected_category());
        let height = 60.0 + m::category_count() as f32 * 16.0 + module_count as f32 * 18.0;
        let (x, y) = corners.place(Corner::TopLeft, width, height);
        game.native_fill(batch, [x, y, width, height], m::MENU_BACKGROUND);
        game.native_fill(batch, [x, y, 3.0, height], m::PANEL_ACCENT);
        game.native_text(batch, "HORIXON  /  MODULE MANAGER", [x + 12.0, y + 9.0], m::PANEL_ACCENT, true);
        game.native_text(batch, "LEFT/RIGHT: CATEGORY   UP/DOWN: MODULE", [x + 12.0, y + 25.0], m::PANEL_MUTED, true);
        for index in 0..m::category_count() {
            let row_y = y + 46.0 + index as f32 * 16.0;
            if index == m::selected_category() {
                game.native_fill(batch, [x + 7.0, row_y - 2.0, 112.0, 14.0], m::MENU_SELECTED);
            }
            let color = if index == m::selected_category() { m::PANEL_TEXT } else { m::PANEL_MUTED };
            game.native_text(batch, m::category_name(index), [x + 13.0, row_y], color, true);
        }
        let right_x = x + 132.0;
        game.native_text(batch, m::category_name(m::selected_category()), [right_x, y + 46.0], m::PANEL_ACCENT, true);
        if module_count == 0 {
            game.native_text(batch, "No modules in this category yet.", [right_x, y + 68.0], m::PANEL_MUTED, true);
        } else {
            for index in 0..module_count {
                let row_y = y + 68.0 + index as f32 * 18.0;
                if index == m::selected_module() {
                    game.native_fill(batch, [right_x - 5.0, row_y - 3.0, 166.0, 16.0], m::MENU_SELECTED);
                }
                let state = if m::module_enabled(m::selected_category(), index) { "ON" } else { "OFF" };
                let color = if m::module_enabled(m::selected_category(), index) { m::PANEL_ACCENT } else { m::PANEL_MUTED };
                game.native_text(batch, m::module_name(m::selected_category(), index), [right_x, row_y], m::PANEL_TEXT, true);
                game.native_text(batch, state, [x + width - 34.0, row_y], color, true);
            }
        }
        game.native_text(batch, "SPACE/ENTER: TOGGLE   ESC: CLOSE", [x + 12.0, y + height - 17.0], m::PANEL_MUTED, true);
    } else if !lines.is_empty() && game.native_corner_shown(Corner::TopLeft) {
        let width = lines.iter().map(|line| game.native_text_width(line))
            .fold(game.native_text_width("HORIXON"), f32::max) + 18.0;
        let height = 36.0 + lines.len() as f32 * 11.0;
        let (x, y) = corners.place(Corner::TopLeft, width, height);
        game.native_fill(batch, [x, y, width, height], m::PANEL_BACKGROUND);
        game.native_fill(batch, [x, y, 2.0, height], m::PANEL_ACCENT);
        game.native_text(batch, "HORIXON", [x + 9.0, y + 7.0], m::PANEL_ACCENT, true);
        for (index, line) in lines.iter().enumerate() {
            game.native_text(batch, line, [x + 9.0, y + 23.0 + index as f32 * 11.0],
                if index == 0 { m::PANEL_TEXT } else { m::PANEL_MUTED }, true);
        }
    }

    if m::array_list_enabled() && game.native_corner_shown(Corner::TopRight) {
        let active: Vec<&str> = (0..m::module_count(2))
            .filter(|index| m::module_enabled(2, *index))
            .map(|index| m::module_name(2, index)).collect();
        if !active.is_empty() {
            let width = active.iter().map(|name| game.native_text_width(name))
                .fold(0.0_f32, f32::max) + 18.0;
            let height = 12.0 + active.len() as f32 * 12.0;
            let (x, y) = corners.place(Corner::TopRight, width, height);
            game.native_fill(batch, [x, y, width, height], m::PANEL_BACKGROUND);
            game.native_fill(batch, [x + width - 2.0, y, 2.0, height], m::PANEL_ACCENT);
            for (index, name) in active.iter().enumerate() {
                let text_width = game.native_text_width(name);
                game.native_text(batch, name, [x + width - 9.0 - text_width, y + 6.0 + index as f32 * 12.0], m::PANEL_TEXT, true);
            }
        }
    }
}
