//! `systems.hud` -?the HUD element model and the box/anchoring maths.
//!
//! The anchor arithmetic is ported from `HudBox.java` line for line (thirds of
//! the screen decide the anchor, `border` clamps), because the editor's drag
//! behaviour and the default layouts players share depend on it.

use crate::meteor_core::config::Tag;
use crate::meteor_core::env::{Env, Renderer};
use crate::meteor_core::setting::Settings;

/// `Hud.Alignment`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Alignment {
    #[default]
    Auto,
    Left,
    Center,
    Right,
}

impl Alignment {
    pub const NAMES: [&'static str; 4] = ["auto", "left", "center", "right"];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum XAnchor {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum YAnchor {
    #[default]
    Top,
    Center,
    Bottom,
}

/// `HudGroup` -?the dropdown grouping in the editor.
#[derive(Clone, Copy, Debug)]
pub struct HudGroup(pub &'static str);

/// `HudElementInfo` -?one HUD element's registration: its heading, and the
/// factory the editor calls to add another instance.
pub struct HudElementInfo {
    pub group: HudGroup,
    pub name: &'static str,
    pub title: String,
    pub description: &'static str,
    pub factory: fn() -> Box<dyn HudElement>,
    pub presets: Vec<Preset>,
}

impl HudElementInfo {
    pub fn new(
        group: HudGroup,
        name: &'static str,
        description: &'static str,
        factory: fn() -> Box<dyn HudElement>,
    ) -> Self {
        Self {
            group,
            name,
            title: crate::meteor_core::util::name_to_title(name),
            description,
            factory,
            presets: Vec::new(),
        }
    }

    pub fn create(&self) -> Box<dyn HudElement> {
        (self.factory)()
    }

    pub fn has_presets(&self) -> bool {
        !self.presets.is_empty()
    }
}

/// A named configuration a player can apply from the editor.
pub struct Preset {
    pub title: &'static str,
    pub apply: fn(&mut dyn HudElement),
}

/// `HudBox` -?the element's rectangle plus the anchor it is measured from.
#[derive(Clone, Debug)]
pub struct HudBox {
    pub x_anchor: XAnchor,
    pub y_anchor: YAnchor,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    /// `Utils.getWindowWidth()` / `getWindowHeight()` at layout time.
    pub screen_width: i32,
    pub screen_height: i32,
    /// `Hud.border`.
    pub border: i32,
    pub auto_anchors: bool,
}

impl Default for HudBox {
    fn default() -> Self {
        Self {
            x_anchor: XAnchor::Left,
            y_anchor: YAnchor::Top,
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            screen_width: 1920,
            screen_height: 1080,
            border: 4,
            auto_anchors: true,
        }
    }
}

impl HudBox {
    pub fn set_size(&mut self, width: f64, height: f64) {
        if width >= 0.0 {
            self.width = width.ceil() as i32;
        }
        if height >= 0.0 {
            self.height = height.ceil() as i32;
        }
    }

    pub fn set_pos(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    pub fn set_x_anchor(&mut self, anchor: XAnchor) {
        if self.x_anchor == anchor {
            return;
        }
        let render_x = self.render_x();
        self.x = match anchor {
            XAnchor::Left => render_x,
            XAnchor::Center => render_x + self.width / 2 - self.screen_width / 2,
            XAnchor::Right => render_x + self.width - self.screen_width,
        };
        self.x_anchor = anchor;
    }

    pub fn set_y_anchor(&mut self, anchor: YAnchor) {
        if self.y_anchor == anchor {
            return;
        }
        let render_y = self.render_y();
        self.y = match anchor {
            YAnchor::Top => render_y,
            YAnchor::Center => render_y + self.height / 2 - self.screen_height / 2,
            YAnchor::Bottom => render_y + self.height - self.screen_height,
        };
        self.y_anchor = anchor;
    }

    pub fn update_anchors(&mut self) {
        let x = self.get_x_anchor(self.render_x() as f64);
        self.set_x_anchor(x);
        let y = self.get_y_anchor(self.render_y() as f64);
        self.set_y_anchor(y);
    }

    pub fn move_by(&mut self, delta_x: i32, delta_y: i32) {
        self.x += delta_x;
        self.y += delta_y;
        if self.auto_anchors {
            self.update_anchors();
        }
        // Clamp against the border, as `HudBox.move` does.
        if self.x_anchor == XAnchor::Left && self.x < self.border {
            self.x = self.border;
        } else if self.x_anchor == XAnchor::Right && self.x > self.border {
            self.x = self.border;
        }
        if self.y_anchor == YAnchor::Top && self.y < self.border {
            self.y = self.border;
        } else if self.y_anchor == YAnchor::Bottom && self.y > self.border {
            self.y = self.border;
        }
    }

    pub fn get_x_anchor(&self, x: f64) -> XAnchor {
        let split_left = self.screen_width as f64 / 3.0;
        let split_right = split_left * 2.0;
        let left = x <= split_left;
        let right = x + self.width as f64 >= split_right;
        if (left && right) || (!left && !right) {
            XAnchor::Center
        } else if left {
            XAnchor::Left
        } else {
            XAnchor::Right
        }
    }

    pub fn get_y_anchor(&self, y: f64) -> YAnchor {
        let split_top = self.screen_height as f64 / 3.0;
        let split_bottom = split_top * 2.0;
        let top = y <= split_top;
        let bottom = y + self.height as f64 >= split_bottom;
        if (top && bottom) || (!top && !bottom) {
            YAnchor::Center
        } else if top {
            YAnchor::Top
        } else {
            YAnchor::Bottom
        }
    }

    pub fn render_x(&self) -> i32 {
        match self.x_anchor {
            XAnchor::Left => self.x,
            XAnchor::Center => self.screen_width / 2 - self.width / 2 + self.x,
            XAnchor::Right => self.screen_width - self.width + self.x,
        }
    }

    pub fn render_y(&self) -> i32 {
        match self.y_anchor {
            YAnchor::Top => self.y,
            YAnchor::Center => self.screen_height / 2 - self.height / 2 + self.y,
            YAnchor::Bottom => self.screen_height - self.height + self.y,
        }
    }

    pub fn align_x(&self, self_width: f64, width: f64, alignment: Alignment) -> f64 {
        let anchor = match alignment {
            Alignment::Auto => self.x_anchor,
            Alignment::Left => XAnchor::Left,
            Alignment::Center => XAnchor::Center,
            Alignment::Right => XAnchor::Right,
        };
        match anchor {
            XAnchor::Left => 0.0,
            XAnchor::Center => self_width / 2.0 - width / 2.0,
            XAnchor::Right => self_width - width,
        }
    }

    pub fn to_tag(&self) -> Tag {
        let mut t = Tag::compound();
        t.put_str(
            "x-anchor",
            match self.x_anchor {
                XAnchor::Left => "Left",
                XAnchor::Center => "Center",
                XAnchor::Right => "Right",
            },
        );
        t.put_str(
            "y-anchor",
            match self.y_anchor {
                YAnchor::Top => "Top",
                YAnchor::Center => "Center",
                YAnchor::Bottom => "Bottom",
            },
        );
        t.put_int("x", self.x);
        t.put_int("y", self.y);
        t
    }

    pub fn from_tag(&mut self, tag: &Tag) {
        self.x_anchor = match tag.get_str("x-anchor") {
            "Center" => XAnchor::Center,
            "Right" => XAnchor::Right,
            _ => XAnchor::Left,
        };
        self.y_anchor = match tag.get_str("y-anchor") {
            "Center" => YAnchor::Center,
            "Bottom" => YAnchor::Bottom,
            _ => YAnchor::Top,
        };
        if let Some(x) = tag.get_int("x") {
            self.x = x;
        }
        if let Some(y) = tag.get_int("y") {
            self.y = y;
        }
    }
}

/// A HUD element. Java's `HudElement` subclasses `HudRenderer`'s drawing helpers;
/// here the element takes a [`Renderer`] and draws through it.
pub trait HudElement: Send + Sync {
    fn info(&self) -> &'static HudElementInfo;

    fn settings(&self) -> &Settings;
    fn settings_mut(&mut self) -> &mut Settings;

    fn box_ref(&self) -> &HudBox;
    fn box_mut(&mut self) -> &mut HudBox;

    fn is_active(&self) -> bool;
    fn set_active(&mut self, active: bool);

    /// Client tick, before the frame draws -?`HudElement.tick(HudRenderer)`.
    fn tick(&mut self, _env: &mut dyn Env) {}
    /// Draw. `renderer` is already translated to the element's corner, as
    /// `HudRenderer` does in Java.
    fn render(&mut self, _env: &mut dyn Env, _r: &mut dyn Renderer) {}
    fn on_font_changed(&mut self) {}

    // ---- shared geometry helpers ----
    fn auto_anchors(&self) -> bool {
        self.box_ref().auto_anchors
    }

    fn set_auto_anchors(&mut self, on: bool) {
        self.box_mut().auto_anchors = on;
    }

    /// `HudElement.setSize`.
    fn set_size(&mut self, width: f64, height: f64) {
        self.box_mut().set_size(width, height);
    }

    fn width(&self) -> i32 {
        self.box_ref().width
    }

    fn height(&self) -> i32 {
        self.box_ref().height
    }

    /// Where the element draws, in GUI pixels.
    fn render_pos(&self) -> (i32, i32) {
        (self.box_ref().render_x(), self.box_ref().render_y())
    }

    /// `HudElement.setPos` from the editor's drag.
    fn set_pos(&mut self, x: i32, y: i32) {
        if self.auto_anchors() {
            let border = self.box_ref().border;
            {
                let b = self.box_mut();
                b.set_pos(x, y);
                b.x_anchor = XAnchor::Left;
                b.y_anchor = YAnchor::Top;
                b.update_anchors();
            }
            let _ = border;
        } else {
            let (px, py) = (self.box_ref().x, self.box_ref().y);
            let b = self.box_mut();
            b.set_pos(b.x + (x - px), b.y + (y - py));
        }
    }

    fn move_by(&mut self, delta_x: i32, delta_y: i32) {
        self.box_mut().move_by(delta_x, delta_y);
    }

    fn align_x(&self, width: f64, alignment: Alignment) -> f64 {
        self.box_ref()
            .align_x(self.width() as f64, width, alignment)
    }

    /// `HudElement.toTag`.
    fn to_tag(&self) -> Tag {
        let mut t = Tag::compound();
        t.put_str("name", self.info().name);
        t.put_bool("active", self.is_active());
        t.put(
            "settings",
            crate::meteor_core::config::settings_to_tag(self.settings()),
        );
        t.put("box", self.box_ref().to_tag());
        t.put_bool("autoAnchors", self.auto_anchors());
        t
    }

    /// `HudElement.fromTag`.
    fn from_tag(&mut self, tag: &Tag) {
        self.settings().reset();
        if let Some(active) = tag.get_bool("active") {
            self.set_active(active);
        }
        if let Some(settings) = tag.get("settings") {
            crate::meteor_core::config::settings_from_tag(self.settings_mut(), settings);
        }
        if let Some(b) = tag.get("box") {
            self.box_mut().from_tag(b);
        }
        if let Some(auto) = tag.get_bool("autoAnchors") {
            self.set_auto_anchors(auto);
        }
    }
}

/// `Hud` itself: the element list, the global settings and the draw order.
pub struct Hud {
    pub active: bool,
    pub settings: Settings,
    pub elements: Vec<Box<dyn HudElement>>,
    pub registry: Vec<&'static HudElementInfo>,
    /// The editor's current element, if any.
    pub selected: Option<usize>,
}

/// `Hud.GROUP`.
pub const GROUP: HudGroup = HudGroup("HORIXON");

impl Default for Hud {
    fn default() -> Self {
        Self::new()
    }
}

impl Hud {
    pub fn new() -> Self {
        let mut settings = Settings::new();
        {
            let g = settings.default_group();
            g.bool_setting("custom-font", true);
            g.bool_setting("hide-in-menus", false);
            let scale = g.double_setting("text-scale", 1.0, 0.5, 3.0, 2);
            let _ = scale;
        }
        {
            let e = settings.create_group("Editor");
            e.int_setting("border", 4, 0, 20);
            e.int_setting("snapping-range", 10, 0, 20);
        }
        Self {
            active: true,
            settings,
            elements: Vec::new(),
            registry: Vec::new(),
            selected: None,
        }
    }

    pub fn register(&mut self, info: &'static HudElementInfo) {
        if !self.registry.iter().any(|i| i.name == info.name) {
            self.registry.push(info);
        }
    }

    /// Adds a default element (`Hud.add` in Java's `init`).
    pub fn add(&mut self, element: Box<dyn HudElement>) -> usize {
        self.elements.push(element);
        self.elements.len() - 1
    }

    /// Adds a fresh element from a registered info.
    pub fn add_info(&mut self, info: &'static HudElementInfo) -> usize {
        let element = info.create();
        self.add(element)
    }

    pub fn remove(&mut self, index: usize) {
        if index < self.elements.len() {
            self.elements.remove(index);
            if self.selected == Some(index) {
                self.selected = None;
            }
        }
    }

    pub fn get(&self, name: &str) -> Option<&dyn HudElement> {
        self.elements
            .iter()
            .find(|e| e.info().name == name)
            .map(|e| &**e)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut dyn HudElement> {
        for e in self.elements.iter_mut() {
            if e.info().name == name {
                return Some(&mut **e);
            }
        }
        None
    }

    pub fn border(&self) -> i32 {
        int_setting(&self.settings, "border", 4)
    }

    /// Lays every element out against the current screen size, then draws it.
    /// Elements draw in creation order, as `Hud.render` iterates its list.
    pub fn render(&mut self, env: &mut dyn Env, r: &mut dyn Renderer) {
        if !self.active {
            return;
        }
        let (sw, sh) = r.gui_size();
        if bool_setting(&self.settings, "hide-in-menus", false)
            && env.screen() == crate::meteor_core::env::Screen::Other
        {
            return;
        }
        let border = self.border();
        for element in self.elements.iter_mut() {
            {
                let b = element.box_mut();
                b.screen_width = sw as i32;
                b.screen_height = sh as i32;
                b.border = border;
            }
            if element.is_active() {
                element.tick(env);
            }
        }
        for element in self.elements.iter_mut() {
            if !element.is_active() {
                continue;
            }
            let (x, y) = element.render_pos();
            let mut t = Translated::new(r);
            t.push(x as f64, y as f64);
            element.render(env, &mut t);
            t.pop();
        }
    }

    /// `Hud.toTag`.
    pub fn to_tag(&self) -> Tag {
        let mut root = Tag::compound();
        root.put_bool("active", self.active);
        root.put(
            "settings",
            crate::meteor_core::config::settings_to_tag(&self.settings),
        );
        let mut list = Vec::new();
        for e in &self.elements {
            list.push(e.to_tag());
        }
        root.put("elements", Tag::List(list));
        root
    }

    /// `Hud.fromTag`, including "add the defaults when the file has none".
    pub fn from_tag(&mut self, tag: &Tag) {
        if let Some(active) = tag.get_bool("active") {
            self.active = active;
        }
        if let Some(s) = tag.get("settings") {
            crate::meteor_core::config::settings_from_tag(&mut self.settings, s);
        }
        let list = tag.get("elements").map(|l| l.as_list()).unwrap_or(&[]);
        if list.is_empty() {
            return;
        }
        self.elements.clear();
        for e in list {
            let name = e.get_str("name").to_string();
            if let Some(info) = self.registry.iter().find(|i| i.name == name).copied() {
                let mut element = info.create();
                element.from_tag(e);
                self.elements.push(element);
            }
        }
    }
}

/// Reads a bool setting of any `Settings` bag, with a default for a missing one.
pub fn bool_setting(settings: &Settings, name: &str, default: bool) -> bool {
    match settings.get(name).map(|s| &s.value) {
        Some(crate::meteor_core::setting::Value::Bool(c)) => {
            c.load(std::sync::atomic::Ordering::Relaxed)
        }
        _ => default,
    }
}

/// Reads an int setting of any `Settings` bag.
pub fn int_setting(settings: &Settings, name: &str, default: i32) -> i32 {
    match settings.get(name).map(|s| &s.value) {
        Some(crate::meteor_core::setting::Value::Int { cell, .. }) => {
            cell.load(std::sync::atomic::Ordering::Relaxed)
        }
        _ => default,
    }
}

/// Reads a double setting of any `Settings` bag.
pub fn double_setting(settings: &Settings, name: &str, default: f64) -> f64 {
    match settings.get(name).map(|s| &s.value) {
        Some(crate::meteor_core::setting::Value::Double { cell, .. }) => {
            f64::from_bits(cell.load(std::sync::atomic::Ordering::Relaxed) as u64)
        }
        _ => default,
    }
}

/// A renderer wrapper that offsets every 2D call, so an element can draw in its
/// own coordinate space (Java's `HudRenderer` translate stack).
pub struct Translated<'a> {
    pub inner: &'a mut dyn Renderer,
    pub dx: f64,
    pub dy: f64,
    pub stack: Vec<(f64, f64)>,
}

impl<'a> Translated<'a> {
    pub fn new(inner: &'a mut dyn Renderer) -> Self {
        Self {
            inner,
            dx: 0.0,
            dy: 0.0,
            stack: Vec::new(),
        }
    }

    pub fn push(&mut self, x: f64, y: f64) {
        self.stack.push((self.dx, self.dy));
        self.dx += x;
        self.dy += y;
    }

    pub fn pop(&mut self) {
        if let Some((dx, dy)) = self.stack.pop() {
            self.dx = dx;
            self.dy = dy;
        }
    }
}

impl Renderer for Translated<'_> {
    fn line(
        &mut self,
        a: crate::meteor_core::math::Vec3d,
        b: crate::meteor_core::math::Vec3d,
        c: u32,
        w: f32,
        t: bool,
    ) {
        self.inner.line(a, b, c, w, t)
    }
    fn box_outline(
        &mut self,
        a: crate::meteor_core::math::Vec3d,
        b: crate::meteor_core::math::Vec3d,
        c: u32,
        w: f32,
        t: bool,
    ) {
        self.inner.box_outline(a, b, c, w, t)
    }
    fn filled_box(
        &mut self,
        a: crate::meteor_core::math::Vec3d,
        b: crate::meteor_core::math::Vec3d,
        c: u32,
        t: bool,
    ) {
        self.inner.filled_box(a, b, c, t)
    }
    fn text3d(&mut self, a: crate::meteor_core::math::Vec3d, s: &str, c: u32, sc: f32, t: bool) {
        self.inner.text3d(a, s, c, sc, t)
    }
    fn rect(&mut self, x: f64, y: f64, w: f64, h: f64, c: u32) {
        self.inner.rect(x + self.dx, y + self.dy, w, h, c)
    }
    fn rect_outline(&mut self, x: f64, y: f64, w: f64, h: f64, c: u32, bw: f64) {
        self.inner
            .rect_outline(x + self.dx, y + self.dy, w, h, c, bw)
    }
    fn text2d(&mut self, x: f64, y: f64, s: &str, c: u32, sc: f64) {
        self.inner.text2d(x + self.dx, y + self.dy, s, c, sc)
    }
    fn item2d(&mut self, x: f64, y: f64, s: &crate::meteor_core::env::ItemStackInfo, size: f64) {
        self.inner.item2d(x + self.dx, y + self.dy, s, size)
    }
    fn text_width(&self, s: &str, sc: f64) -> f64 {
        self.inner.text_width(s, sc)
    }
    fn gui_size(&self) -> (f64, f64) {
        self.inner.gui_size()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchors_follow_screen_thirds() {
        let mut b = HudBox {
            width: 100,
            height: 20,
            screen_width: 1920,
            screen_height: 1080,
            ..Default::default()
        };
        assert_eq!(b.get_x_anchor(10.0), XAnchor::Left);
        assert_eq!(b.get_x_anchor(1850.0), XAnchor::Right);
        assert_eq!(b.get_x_anchor(900.0), XAnchor::Center);
        assert_eq!(b.get_y_anchor(5.0), YAnchor::Top);
        assert_eq!(b.get_y_anchor(1060.0), YAnchor::Bottom);

        b.update_anchors();
        assert_eq!(b.x_anchor, XAnchor::Left);
        assert_eq!(b.y_anchor, YAnchor::Top);
    }

    #[test]
    fn right_anchor_sticks_to_the_right_edge() {
        let mut b = HudBox {
            width: 100,
            height: 20,
            screen_width: 1000,
            screen_height: 500,
            x_anchor: XAnchor::Right,
            x: 0,
            ..Default::default()
        };
        assert_eq!(b.render_x(), 900);
        b.set_pos(950, 10);
        // `setPos` stores the anchor-relative x, so the render position shifts by
        // the same amount (Java's `HudBox.setPos` does the same).
        assert_eq!(b.render_x(), 1850);
    }

    #[test]
    fn move_clamps_to_border() {
        let mut b = HudBox {
            border: 4,
            ..Default::default()
        };
        b.move_by(-50, -50);
        assert_eq!(b.x, 4);
        assert_eq!(b.y, 4);
    }

    #[test]
    fn box_tag_round_trip() {
        let mut b = HudBox {
            x_anchor: XAnchor::Right,
            y_anchor: YAnchor::Bottom,
            x: 12,
            y: -3,
            ..Default::default()
        };
        let tag = b.to_tag();
        let mut b2 = HudBox::default();
        b2.from_tag(&tag);
        assert_eq!(b2.x_anchor, XAnchor::Right);
        assert_eq!(b2.y_anchor, YAnchor::Bottom);
        assert_eq!((b2.x, b2.y), (12, -3));
    }
}
