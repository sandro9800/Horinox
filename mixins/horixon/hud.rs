fn horixon_hud(
    game: &mut crate::game::Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
    corners: &mut horizon_client::extras_hud::Corners,
) {
    use crate::game::horixon as m;
    use horizon_client::extras_hud::Corner;
    if !game.native_on(m::ID) || !game.native_corner_shown(Corner::TopLeft) { return; }

    let mut lines = vec![m::status_line(game.native_frame_rate().0)];
    if let Some(player) = game.native_player() {
        lines.push(m::position_line(player.x, player.y, player.z, player.dimension));
    } else { lines.push("WAITING FOR WORLD".to_owned()); }

    if m::menu_is_open() {
        let width = 166.0;
        let height = 44.0 + m::category_count() as f32 * 15.0;
        let (x, y) = corners.place(Corner::TopLeft, width, height);
        game.native_fill(batch, [x, y, width, height], m::MENU_BACKGROUND);
        game.native_fill(batch, [x, y, 3.0, height], m::PANEL_ACCENT);
        game.native_text(batch, "HORIXON  /  MODULES", [x + 10.0, y + 9.0], m::PANEL_ACCENT, true);
        game.native_text(batch, "LEFT / RIGHT  •  SELECT", [x + 10.0, y + 24.0], m::PANEL_MUTED, true);
        for index in 0..m::category_count() {
            let row_y = y + 39.0 + index as f32 * 15.0;
            if index == m::selected_category() {
                game.native_fill(batch, [x + 6.0, row_y - 2.0, width - 12.0, 13.0], m::MENU_SELECTED);
            }
            game.native_text(batch, m::category_name(index), [x + 12.0, row_y],
                if index == m::selected_category() { m::PANEL_TEXT } else { m::PANEL_MUTED }, true);
        }
        game.native_text(batch, "RIGHT SHIFT  •  CLOSE", [x + 10.0, y + height - 13.0], m::PANEL_MUTED, true);
    } else {
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
}
