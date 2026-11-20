use crate::meteor_core::category::Category;
use crate::meteor_core::env::{Env, Screen};
use crate::meteor_core::setting::Value;
use crate::meteor_core::util::name_to_title;
use crate::port::env_impl::DrawQueue;
use crate::port::shared::Shared;

const VERSION: &str = "0.5.0";
const PANEL_BG: u32 = 0xF00A1422;
const SIDEBAR_BG: u32 = 0xF00D1A2B;
const SURFACE: u32 = 0xF0132235;
const SURFACE_HOVER: u32 = 0xF01A3047;
const BORDER: u32 = 0xFF263A50;
const MUTED: u32 = 0xFF91A4B8;
const GREEN: u32 = 0xFF34D399;
const OFF: u32 = 0xFF52657A;

#[derive(Clone, Copy, Default)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}
impl Rect {
    fn contains(self, x: f64, y: f64) -> bool {
        x >= self.x && y >= self.y && x <= self.x + self.w && y <= self.y + self.h
    }
}

enum UiDraw {
    Fill(Rect, u32),
    Outline(Rect, u32),
    Text(f64, f64, String, u32),
}

pub struct ClickGui {
    open: bool,
    selected_category: Category,
    selected: Option<&'static str>,
    search: String,
    search_focused: bool,
    settings_focus: bool,
    selected_setting: usize,
    mouse: (f64, f64),
    panel: Rect,
    sidebar: Rect,
    module_area: Rect,
    module_list: Rect,
    settings_area: Rect,
    settings_list: Rect,
    search_box: Rect,
    module_scroll: f64,
    settings_scroll: f64,
    module_scroll_max: f64,
    settings_scroll_max: f64,
    scale: f64,
    hud_editor: bool,
    dragging_hud: Option<usize>,
    last_drag_mouse: (f64, f64),
}

impl ClickGui {
    pub fn new() -> Self {
        Self {
            open: false,
            selected_category: Category::Combat,
            selected: None,
            search: String::new(),
            search_focused: false,
            settings_focus: false,
            selected_setting: 0,
            mouse: (0.0, 0.0),
            panel: Rect::default(),
            sidebar: Rect::default(),
            module_area: Rect::default(),
            module_list: Rect::default(),
            settings_area: Rect::default(),
            settings_list: Rect::default(),
            search_box: Rect::default(),
            module_scroll: 0.0,
            settings_scroll: 0.0,
            module_scroll_max: 0.0,
            settings_scroll_max: 0.0,
            scale: 1.0,
            hud_editor: false,
            dragging_hud: None,
            last_drag_mouse: (0.0, 0.0),
        }
    }

    pub fn open(&mut self) {
        self.open = true;
        self.search_focused = false;
        self.settings_focus = false;
        self.selected_setting = 0;
        self.module_scroll = 0.0;
        self.settings_scroll = 0.0;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.search_focused = false;
        self.hud_editor = false;
        self.dragging_hud = None;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn on_screen_change(&mut self, screen: Screen, _opened: bool) {
        if screen != Screen::Playing {
            self.close();
        }
    }

    pub fn tick(&mut self, shared: &mut Shared) {
        if let Some(index) = self.dragging_hud {
            let dx = (self.mouse.0 - self.last_drag_mouse.0).round() as i32;
            let dy = (self.mouse.1 - self.last_drag_mouse.1).round() as i32;
            if let Some(element) = shared.hud.elements.get_mut(index) {
                element.move_by(dx, dy);
            }
        }
        self.last_drag_mouse = self.mouse;
    }

    pub fn nav(&mut self, shared: &Shared, delta: i32) {
        let items = self.module_items(shared);
        if items.is_empty() {
            self.selected = None;
            return;
        }
        let current = self
            .selected
            .and_then(|name| items.iter().position(|(_, item)| *item == name))
            .unwrap_or(0);
        let next = (current as i32 + delta).rem_euclid(items.len() as i32) as usize;
        self.selected_category = items[next].0;
        self.selected = Some(items[next].1);
        self.selected_setting = 0;
        self.settings_scroll = 0.0;
    }

    pub fn change_category(&mut self, delta: i32, shared: &Shared) {
        let all = Category::ALL;
        let current = all
            .iter()
            .position(|category| *category == self.selected_category)
            .unwrap_or(0);
        let next = (current as i32 + delta).rem_euclid(all.len() as i32) as usize;
        self.selected_category = all[next];
        self.selected_setting = 0;
        self.settings_scroll = 0.0;
        self.settings_focus = false;
        self.module_scroll = 0.0;
        self.search.clear();
        self.ensure_selected(shared);
    }

    pub fn activate_selected(&mut self, shared: &mut Shared) {
        let Some(name) = self.selected else { return };
        let feedback = shared.config.chat_feedback;
        let mut env = crate::port::env_impl::HeadlessEnv::new();
        shared
            .modules
            .toggle_with_message_by_name(name, &mut env, feedback);
    }

    pub fn toggle_settings_of_selected(&mut self) {
        self.settings_focus = !self.settings_focus;
        self.search_focused = false;
        self.selected_setting = 0;
        self.settings_scroll = 0.0;
    }

    pub fn on_key(
        &mut self,
        shared: &mut Shared,
        key: i32,
        pressed: bool,
        _modifiers: i32,
    ) -> bool {
        if !pressed {
            return false;
        }
        match key {
            256 | 344 => {
                self.close();
                return true;
            }
            259 if self.search_focused => {
                self.search.pop();
                self.module_scroll = 0.0;
                self.ensure_selected(shared);
                return true;
            }
            257 if self.search_focused => {
                self.search_focused = false;
                return true;
            }
            72 => {
                self.hud_editor = !self.hud_editor;
                self.dragging_hud = None;
                return true;
            }
            258 => {
                self.toggle_settings_of_selected();
                return true;
            }
            _ => {}
        }

        if self.settings_focus {
            match key {
                265 => self.move_setting(shared, -1),
                264 => self.move_setting(shared, 1),
                263 => self.adjust_selected_setting(shared, -1, false),
                262 => self.adjust_selected_setting(shared, 1, false),
                257 | 32 => self.adjust_selected_setting(shared, 0, true),
                _ => return false,
            }
            true
        } else {
            match key {
                265 => self.nav(shared, -1),
                264 => self.nav(shared, 1),
                263 => self.change_category(-1, shared),
                262 => self.change_category(1, shared),
                257 | 32 => self.activate_selected(shared),
                _ => return false,
            }
            true
        }
    }

    pub fn on_char(&mut self, shared: &mut Shared, c: char) -> bool {
        if !self.open || !self.search_focused || c.is_control() {
            return false;
        }
        self.search.push(c);
        self.module_scroll = 0.0;
        self.ensure_selected(shared);
        true
    }

    pub fn on_mouse_move(&mut self, x: f64, y: f64) {
        self.mouse = (x, y);
    }

    pub fn scroll(&mut self, amount: f64) {
        if !self.open {
            return;
        }
        let (x, y) = self.mouse;
        if self.module_list.contains(x, y) {
            self.module_scroll = (self.module_scroll - amount * 28.0 * self.scale)
                .clamp(0.0, self.module_scroll_max);
        } else if self.settings_list.contains(x, y) {
            self.settings_scroll = (self.settings_scroll - amount * 28.0 * self.scale)
                .clamp(0.0, self.settings_scroll_max);
        }
    }

    pub fn on_mouse(
        &mut self,
        shared: &mut Shared,
        env: &mut dyn Env,
        button: i32,
        pressed: bool,
    ) -> bool {
        if !self.open {
            return false;
        }
        if !pressed {
            self.dragging_hud = None;
            return true;
        }
        let (mx, my) = self.mouse;
        if self.hud_editor {
            if button == 0 {
                self.dragging_hud = self.hud_element_at(shared, mx, my);
                self.last_drag_mouse = self.mouse;
            }
            return true;
        }
        if self.search_box.contains(mx, my) {
            self.search_focused = true;
            self.settings_focus = false;
            return true;
        }
        self.search_focused = false;
        for category in Category::ALL {
            if self.category_rect(category).contains(mx, my) {
                self.selected_category = category;
                self.search.clear();
                self.settings_focus = false;
                self.selected_setting = 0;
                self.settings_scroll = 0.0;
                self.module_scroll = 0.0;
                self.ensure_selected(shared);
                return true;
            }
        }
        for (category, name, rect) in self.module_rects(shared) {
            if !rect.contains(mx, my) {
                continue;
            }
            self.selected_category = category;
            if self.selected != Some(name) {
                self.selected_setting = 0;
                self.settings_scroll = 0.0;
            }
            self.selected = Some(name);
            let toggle = Rect {
                x: rect.x + rect.w - 64.0 * self.scale,
                y: rect.y + 5.0 * self.scale,
                w: 54.0 * self.scale,
                h: rect.h - 10.0 * self.scale,
            };
            if button == 0 && toggle.contains(mx, my) {
                self.settings_focus = false;
                let feedback = shared.config.chat_feedback;
                shared
                    .modules
                    .toggle_with_message_by_name(name, env, feedback);
            } else if button == 1 {
                self.settings_focus = true;
                self.selected_setting = 0;
            } else {
                self.settings_focus = false;
            }
            return true;
        }
        if let Some(module) = self.selected {
            for (index, setting, rect) in self.setting_rects(shared) {
                if rect.contains(mx, my) {
                    self.settings_focus = true;
                    self.selected_setting = index;
                    self.click_setting(shared, module, &setting, mx);
                    return true;
                }
            }
        }
        true
    }

    pub fn render_screen(&mut self, shared: &Shared, queue: &mut DrawQueue, mouse: [f32; 2]) {
        let width = queue.gui_size.0;
        let height = queue.gui_size.1;
        self.mouse = (mouse[0] as f64, mouse[1] as f64);
        self.ensure_selected(shared);
        self.layout(shared, width, height);
        self.draw_dashboard(shared, width, height, |command| match command {
            UiDraw::Fill(rect, color) => queue.fill(rect.x, rect.y, rect.w, rect.h, color),
            UiDraw::Outline(rect, color) => {
                queue.outline(rect.x, rect.y, rect.w, rect.h, color, 1.0)
            }
            UiDraw::Text(x, y, text, color) => queue.text(x, y, &text, color, 1.0),
        });
    }

    fn layout(&mut self, shared: &Shared, width: f64, height: f64) {
        self.scale = shared.theme.scale.get().clamp(0.75, 1.35);
        let s = self.scale;
        let panel_w = (920.0 * s).min((width - 28.0).max(320.0));
        let panel_h = (570.0 * s).min((height - 28.0).max(260.0));
        let x = (width - panel_w) / 2.0;
        let y = (height - panel_h) / 2.0;
        self.panel = Rect {
            x,
            y,
            w: panel_w,
            h: panel_h,
        };
        let header_h = 52.0 * s;
        let footer_h = 28.0 * s;
        let sidebar_w = (154.0 * s).min(panel_w * 0.27);
        let content_x = x + sidebar_w + 12.0 * s;
        let available = panel_w - sidebar_w - 36.0 * s;
        let module_w = (276.0 * s).min(available * 0.46);
        let settings_w = (available - module_w).max(120.0 * s);
        let content_y = y + header_h + 12.0 * s;
        let content_h = panel_h - header_h - footer_h - 24.0 * s;
        self.sidebar = Rect {
            x,
            y: y + header_h,
            w: sidebar_w,
            h: panel_h - header_h - footer_h,
        };
        self.module_area = Rect {
            x: content_x,
            y: content_y,
            w: module_w,
            h: content_h,
        };
        self.settings_area = Rect {
            x: content_x + module_w + 12.0 * s,
            y: content_y,
            w: settings_w,
            h: content_h,
        };
        self.module_list = Rect {
            x: self.module_area.x + 6.0 * s,
            y: self.module_area.y + 34.0 * s,
            w: self.module_area.w - 12.0 * s,
            h: self.module_area.h - 40.0 * s,
        };
        self.settings_list = Rect {
            x: self.settings_area.x + 6.0 * s,
            y: self.settings_area.y + 76.0 * s,
            w: self.settings_area.w - 12.0 * s,
            h: self.settings_area.h - 82.0 * s,
        };
        self.search_box = Rect {
            x: x + panel_w - 254.0 * s,
            y: y + 14.0 * s,
            w: 236.0 * s,
            h: 25.0 * s,
        };
        let rows = self.module_items(shared).len();
        self.module_scroll_max = (rows as f64 * 34.0 * s - self.module_list.h).max(0.0);
        let settings = self
            .selected
            .map(|name| shared.setting_rows(name).len())
            .unwrap_or(0);
        self.settings_scroll_max = (settings as f64 * 34.0 * s - self.settings_list.h).max(0.0);
        self.selected_setting = self.selected_setting.min(settings.saturating_sub(1));
        self.module_scroll = self.module_scroll.clamp(0.0, self.module_scroll_max);
        self.settings_scroll = self.settings_scroll.clamp(0.0, self.settings_scroll_max);
    }

    fn module_items(&self, shared: &Shared) -> Vec<(Category, &'static str)> {
        let query = self.search.trim().to_lowercase();
        let mut out = Vec::new();
        for category in Category::ALL {
            if query.is_empty() && category != self.selected_category {
                continue;
            }
            for name in shared.group(category) {
                if query.is_empty()
                    || name.to_lowercase().contains(&query)
                    || shared.title_of(name).to_lowercase().contains(&query)
                {
                    out.push((category, name));
                }
            }
        }
        out
    }

    fn ensure_selected(&mut self, shared: &Shared) {
        let items = self.module_items(shared);
        if items.is_empty() {
            self.selected = None;
            return;
        }
        if !self
            .selected
            .is_some_and(|name| items.iter().any(|(_, item)| *item == name))
        {
            self.selected_category = items[0].0;
            self.selected = Some(items[0].1);
            self.selected_setting = 0;
            self.settings_scroll = 0.0;
        }
    }

    fn category_rect(&self, category: Category) -> Rect {
        let index = Category::ALL
            .iter()
            .position(|item| *item == category)
            .unwrap_or(0);
        let s = self.scale;
        Rect {
            x: self.sidebar.x + 10.0 * s,
            y: self.sidebar.y + 16.0 * s + index as f64 * 38.0 * s,
            w: self.sidebar.w - 20.0 * s,
            h: 31.0 * s,
        }
    }

    fn module_rects(&self, shared: &Shared) -> Vec<(Category, &'static str, Rect)> {
        let s = self.scale;
        let row_h = 34.0 * s;
        self.module_items(shared)
            .into_iter()
            .enumerate()
            .filter_map(|(index, (category, name))| {
                let y = self.module_list.y + index as f64 * row_h - self.module_scroll;
                let rect = Rect {
                    x: self.module_list.x,
                    y,
                    w: self.module_list.w,
                    h: row_h - 3.0 * s,
                };
                if y + rect.h < self.module_list.y || y > self.module_list.y + self.module_list.h {
                    None
                } else {
                    Some((category, name, rect))
                }
            })
            .collect()
    }

    fn setting_rects(&self, shared: &Shared) -> Vec<(usize, String, Rect)> {
        let Some(module) = self.selected else {
            return Vec::new();
        };
        let s = self.scale;
        let row_h = 34.0 * s;
        shared
            .setting_rows(module)
            .into_iter()
            .enumerate()
            .filter_map(|(index, (name, _))| {
                let y = self.settings_list.y + index as f64 * row_h - self.settings_scroll;
                let rect = Rect {
                    x: self.settings_list.x,
                    y,
                    w: self.settings_list.w,
                    h: row_h - 3.0 * s,
                };
                if y + rect.h < self.settings_list.y
                    || y > self.settings_list.y + self.settings_list.h
                {
                    None
                } else {
                    Some((index, name, rect))
                }
            })
            .collect()
    }

    fn move_setting(&mut self, shared: &Shared, delta: i32) {
        let Some(module) = self.selected else { return };
        let count = shared.setting_rows(module).len();
        if count == 0 {
            return;
        }
        self.selected_setting =
            (self.selected_setting as i32 + delta).rem_euclid(count as i32) as usize;
        let row_height = 34.0 * self.scale;
        let top = self.selected_setting as f64 * row_height;
        let bottom = top + row_height;
        if top < self.settings_scroll {
            self.settings_scroll = top;
        } else if bottom > self.settings_scroll + self.settings_list.h {
            self.settings_scroll = bottom - self.settings_list.h;
        }
        self.settings_scroll = self.settings_scroll.clamp(0.0, self.settings_scroll_max);
    }

    fn adjust_selected_setting(&mut self, shared: &mut Shared, delta: i32, activate: bool) {
        let Some(module) = self.selected else { return };
        let rows = shared.setting_rows(module);
        let Some((setting, _)) = rows.get(self.selected_setting) else {
            return;
        };
        let Some(entry) = shared
            .modules
            .base(module)
            .and_then(|base| base.settings.get(setting))
        else {
            return;
        };
        match &entry.value {
            Value::Bool(cell) if activate => {
                let next = !cell.load(std::sync::atomic::Ordering::Relaxed);
                entry.parse(if next { "true" } else { "false" });
            }
            Value::Int { cell, min, max } => {
                let step = if delta == 0 && activate { 1 } else { delta };
                if step != 0 {
                    let current = cell.load(std::sync::atomic::Ordering::Relaxed);
                    let next = current.saturating_add(step).clamp(*min, *max);
                    entry.parse(&next.to_string());
                }
            }
            Value::Double {
                cell,
                min,
                max,
                decimal_places,
            } => {
                let amount = if delta == 0 && activate { 1 } else { delta };
                if amount != 0 {
                    let current =
                        f64::from_bits(cell.load(std::sync::atomic::Ordering::Relaxed) as u64);
                    let step = 10_f64.powi(-(*decimal_places as i32));
                    let next = (current + amount as f64 * step).clamp(*min, *max);
                    entry.parse(&format!("{:.*}", *decimal_places as usize, next));
                }
            }
            Value::Enum { cell, names } if !names.is_empty() => {
                let current = cell.load(std::sync::atomic::Ordering::Relaxed) as i32;
                let amount = if delta == 0 && activate { 1 } else { delta };
                let next = (current + amount).rem_euclid(names.len() as i32) as usize;
                if amount != 0 {
                    entry.parse(names[next]);
                }
            }
            _ => {}
        }
    }

    fn slider_rect(&self, _module: &str) -> (f64, f64) {
        (
            self.settings_list.x + self.settings_list.w - 104.0 * self.scale,
            88.0 * self.scale,
        )
    }

    fn click_setting(&mut self, shared: &mut Shared, module: &str, setting: &str, mx: f64) {
        let Some(entry) = shared
            .modules
            .base(module)
            .and_then(|base| base.settings.get(setting))
        else {
            return;
        };
        match &entry.value {
            Value::Bool(cell) => {
                let value = !cell.load(std::sync::atomic::Ordering::Relaxed);
                entry.parse(if value { "true" } else { "false" });
            }
            Value::Int { min, max, .. } => {
                let (x, width) = self.slider_rect(module);
                let t = ((mx - x) / width).clamp(0.0, 1.0);
                let value = *min as f64 + (*max - *min) as f64 * t;
                entry.parse(&format!("{}", value.round() as i32));
            }
            Value::Double {
                min,
                max,
                decimal_places,
                ..
            } => {
                let (x, width) = self.slider_rect(module);
                let t = ((mx - x) / width).clamp(0.0, 1.0);
                let value = *min + (*max - *min) * t;
                entry.parse(&format!("{:.*}", *decimal_places as usize, value));
            }
            Value::Enum { cell, names } => {
                if names.is_empty() {
                    return;
                }
                let current = cell.load(std::sync::atomic::Ordering::Relaxed) as usize;
                entry.parse(names[(current + 1) % names.len()]);
            }
            _ => {}
        }
    }

    fn draw_dashboard(
        &mut self,
        shared: &Shared,
        width: f64,
        height: f64,
        mut emit: impl FnMut(UiDraw),
    ) {
        let s = self.scale;
        let accent = shared.theme.accent().argb();
        let text = shared.theme.text_color().argb();
        let muted = shared.theme.text_secondary().argb();
        let total: usize = Category::ALL
            .iter()
            .map(|category| shared.group(*category).len())
            .sum();
        emit(UiDraw::Fill(
            Rect {
                x: 0.0,
                y: 0.0,
                w: width,
                h: height,
            },
            0x99040A12,
        ));
        emit(UiDraw::Fill(self.panel, PANEL_BG));
        emit(UiDraw::Outline(self.panel, BORDER));
        emit(UiDraw::Fill(
            Rect {
                x: self.panel.x,
                y: self.panel.y,
                w: self.panel.w,
                h: 3.0 * s,
            },
            accent,
        ));
        emit(UiDraw::Text(
            self.panel.x + 22.0 * s,
            self.panel.y + 13.0 * s,
            "HORIXON".into(),
            accent,
        ));
        emit(UiDraw::Text(
            self.panel.x + 120.0 * s,
            self.panel.y + 16.0 * s,
            format!("CLIENT COMMAND CENTER  /  v{VERSION}"),
            muted,
        ));
        emit(UiDraw::Fill(self.search_box, SURFACE));
        emit(UiDraw::Outline(
            self.search_box,
            if self.search_focused { accent } else { BORDER },
        ));
        let search_text = if self.search.is_empty() {
            "Search modules...".to_string()
        } else {
            self.search.clone()
        };
        emit(UiDraw::Text(
            self.search_box.x + 9.0 * s,
            self.search_box.y + 7.0 * s,
            search_text,
            if self.search.is_empty() { muted } else { text },
        ));
        emit(UiDraw::Fill(self.sidebar, SIDEBAR_BG));
        emit(UiDraw::Text(
            self.sidebar.x + 18.0 * s,
            self.sidebar.y + 10.0 * s,
            "CATEGORIES".into(),
            muted,
        ));
        for category in Category::ALL {
            let rect = self.category_rect(category);
            let selected = category == self.selected_category && self.search.is_empty();
            if selected {
                emit(UiDraw::Fill(rect, SURFACE_HOVER));
                emit(UiDraw::Fill(
                    Rect {
                        x: rect.x,
                        y: rect.y + 2.0 * s,
                        w: 3.0 * s,
                        h: rect.h - 4.0 * s,
                    },
                    accent,
                ));
            }
            emit(UiDraw::Text(
                rect.x + 12.0 * s,
                rect.y + 9.0 * s,
                category.name().to_uppercase(),
                if selected { accent } else { text },
            ));
            emit(UiDraw::Text(
                rect.x + rect.w - 27.0 * s,
                rect.y + 9.0 * s,
                shared.group(category).len().to_string(),
                muted,
            ));
        }
        emit(UiDraw::Text(
            self.module_area.x + 8.0 * s,
            self.module_area.y + 8.0 * s,
            "MODULES".into(),
            text,
        ));
        emit(UiDraw::Text(
            self.module_area.x + self.module_area.w - 42.0 * s,
            self.module_area.y + 8.0 * s,
            self.module_items(shared).len().to_string(),
            muted,
        ));
        emit(UiDraw::Fill(self.module_list, 0x66060E19));
        for (category, name, rect) in self.module_rects(shared) {
            let selected = self.selected == Some(name);
            let active = shared.modules.is_active(name);
            if selected {
                emit(UiDraw::Fill(rect, SURFACE_HOVER));
                emit(UiDraw::Outline(rect, accent));
            } else if rect.contains(self.mouse.0, self.mouse.1) {
                emit(UiDraw::Fill(rect, SURFACE));
            }
            emit(UiDraw::Text(
                rect.x + 9.0 * s,
                rect.y + 5.0 * s,
                shared.title_of(name),
                if selected { accent } else { text },
            ));
            if !self.search.is_empty() {
                emit(UiDraw::Text(
                    rect.x + 9.0 * s,
                    rect.y + 19.0 * s,
                    category.name().to_string(),
                    muted,
                ));
            }
            let badge = Rect {
                x: rect.x + rect.w - 58.0 * s,
                y: rect.y + 7.0 * s,
                w: 48.0 * s,
                h: 19.0 * s,
            };
            emit(UiDraw::Fill(
                badge,
                if active { 0xCC064E5A } else { 0xCC243449 },
            ));
            emit(UiDraw::Text(
                badge.x + 10.0 * s,
                badge.y + 4.0 * s,
                if active { "ON".into() } else { "OFF".into() },
                if active { accent } else { MUTED },
            ));
        }
        if self.module_items(shared).is_empty() {
            emit(UiDraw::Text(
                self.module_list.x + 12.0 * s,
                self.module_list.y + 14.0 * s,
                if self.search.is_empty() {
                    "No modules in this category".into()
                } else {
                    "No modules match your search".into()
                },
                muted,
            ));
        }
        emit(UiDraw::Text(
            self.settings_area.x + 8.0 * s,
            self.settings_area.y + 8.0 * s,
            "MODULE SETTINGS".into(),
            text,
        ));
        emit(UiDraw::Fill(self.settings_list, 0x66060E19));
        if let Some(module) = self.selected {
            emit(UiDraw::Text(
                self.settings_area.x + 8.0 * s,
                self.settings_area.y + 34.0 * s,
                shared.title_of(module),
                accent,
            ));
            let description = shared
                .modules
                .base(module)
                .map(|base| base.description.to_string())
                .unwrap_or_default();
            let desc = if description.chars().count() > 80 {
                format!("{}...", description.chars().take(77).collect::<String>())
            } else {
                description
            };
            emit(UiDraw::Text(
                self.settings_area.x + 8.0 * s,
                self.settings_area.y + 52.0 * s,
                desc,
                muted,
            ));
            let rows = shared.setting_rows(module);
            if rows.is_empty() {
                emit(UiDraw::Text(
                    self.settings_list.x + 10.0 * s,
                    self.settings_list.y + 12.0 * s,
                    "No configurable settings".into(),
                    muted,
                ));
            } else {
                for (index, setting, rect) in self.setting_rects(shared) {
                    let Some((_, display)) = rows.iter().find(|(name, _)| *name == setting) else {
                        continue;
                    };
                    emit(UiDraw::Fill(rect, SURFACE));
                    // Draw focus after the row background so keyboard selection stays visible.
                    if self.settings_focus && index == self.selected_setting {
                        emit(UiDraw::Outline(rect, accent));
                    }
                    emit(UiDraw::Text(
                        rect.x + 8.0 * s,
                        rect.y + 9.0 * s,
                        name_to_title(&setting),
                        muted,
                    ));
                    let value = shared
                        .modules
                        .base(module)
                        .and_then(|base| base.settings.get(&setting))
                        .map(|entry| &entry.value);
                    match value {
                        Some(Value::Bool(cell)) => {
                            let on = cell.load(std::sync::atomic::Ordering::Relaxed);
                            let toggle = Rect {
                                x: rect.x + rect.w - 46.0 * s,
                                y: rect.y + 7.0 * s,
                                w: 36.0 * s,
                                h: 18.0 * s,
                            };
                            emit(UiDraw::Fill(toggle, if on { accent } else { OFF }));
                            emit(UiDraw::Text(
                                toggle.x + 10.0 * s,
                                toggle.y + 4.0 * s,
                                if on { "ON".into() } else { "OFF".into() },
                                if on { PANEL_BG } else { text },
                            ));
                        }
                        Some(Value::Int { .. }) | Some(Value::Double { .. }) => {
                            let (sx, sw) = self.slider_rect(module);
                            let fraction =
                                slider_fraction(shared, module, &setting).clamp(0.0, 1.0);
                            emit(UiDraw::Fill(
                                Rect {
                                    x: sx,
                                    y: rect.y + 14.0 * s,
                                    w: sw,
                                    h: 5.0 * s,
                                },
                                OFF,
                            ));
                            emit(UiDraw::Fill(
                                Rect {
                                    x: sx,
                                    y: rect.y + 14.0 * s,
                                    w: sw * fraction,
                                    h: 5.0 * s,
                                },
                                accent,
                            ));
                            emit(UiDraw::Text(
                                rect.x + rect.w - 44.0 * s,
                                rect.y + 3.0 * s,
                                display.clone(),
                                text,
                            ));
                        }
                        Some(Value::Enum { .. }) => emit(UiDraw::Text(
                            rect.x + rect.w - 100.0 * s,
                            rect.y + 9.0 * s,
                            display.clone(),
                            accent,
                        )),
                        _ => emit(UiDraw::Text(
                            rect.x + rect.w - 112.0 * s,
                            rect.y + 9.0 * s,
                            display.clone(),
                            text,
                        )),
                    }
                }
            }
        } else {
            emit(UiDraw::Text(
                self.settings_list.x + 10.0 * s,
                self.settings_list.y + 12.0 * s,
                "Select a module to inspect its settings".into(),
                muted,
            ));
        }
        emit(UiDraw::Fill(
            Rect {
                x: self.panel.x,
                y: self.panel.y + self.panel.h - 28.0 * s,
                w: self.panel.w,
                h: 28.0 * s,
            },
            SIDEBAR_BG,
        ));
        emit(UiDraw::Text(
            self.panel.x + 16.0 * s,
            self.panel.y + self.panel.h - 19.0 * s,
            format!(
                "{total} MODULES  /  RIGHT SHIFT: CLOSE  /  H: HUD EDITOR  /  TAB: SETTINGS FOCUS"
            ),
            muted,
        ));
        if self.hud_editor {
            self.draw_hud_editor(shared, &mut emit);
        }
    }

    fn draw_hud_editor(&self, shared: &Shared, emit: &mut impl FnMut(UiDraw)) {
        let s = self.scale;
        for element in &shared.hud.elements {
            let (x, y) = element.render_pos();
            let rect = Rect {
                x: x as f64,
                y: y as f64,
                w: element.width() as f64,
                h: element.height() as f64,
            };
            emit(UiDraw::Outline(
                rect,
                if element.is_active() { GREEN } else { MUTED },
            ));
            emit(UiDraw::Text(
                x as f64 + 2.0 * s,
                y as f64 - 12.0 * s,
                element.info().title.to_string(),
                if element.is_active() { GREEN } else { MUTED },
            ));
        }
        emit(UiDraw::Text(
            self.panel.x + 18.0 * s,
            self.panel.y + self.panel.h - 48.0 * s,
            "HUD EDITOR ACTIVE - DRAG ELEMENTS TO REPOSITION".into(),
            GREEN,
        ));
    }

    fn hud_element_at(&self, shared: &Shared, x: f64, y: f64) -> Option<usize> {
        shared
            .hud
            .elements
            .iter()
            .enumerate()
            .find_map(|(index, element)| {
                let (ex, ey) = element.render_pos();
                let rect = Rect {
                    x: ex as f64,
                    y: ey as f64,
                    w: element.width() as f64,
                    h: element.height() as f64,
                };
                if rect.contains(x, y) {
                    Some(index)
                } else {
                    None
                }
            })
    }
}

fn slider_fraction(shared: &Shared, module: &str, setting: &str) -> f64 {
    let Some(entry) = shared
        .modules
        .base(module)
        .and_then(|base| base.settings.get(setting))
    else {
        return 0.0;
    };
    match &entry.value {
        Value::Int { cell, min, max } => {
            if max == min {
                0.0
            } else {
                (cell.load(std::sync::atomic::Ordering::Relaxed) - *min) as f64
                    / (*max - *min) as f64
            }
        }
        Value::Double { cell, min, max, .. } => {
            if (*max - *min).abs() < f64::EPSILON {
                0.0
            } else {
                let value = f64::from_bits(cell.load(std::sync::atomic::Ordering::Relaxed) as u64);
                (value - *min) / (*max - *min)
            }
        }
        _ => 0.0,
    }
}
