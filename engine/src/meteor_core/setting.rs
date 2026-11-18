//! `meteordevelopment.meteorclient.settings` -?the setting table.
//!
//! The Java client has one `Setting<T>` subclass per kind, each holding its value
//! and a builder. Rust has no subclassing, so the same design is expressed as:
//!
//! * a [`Setting`] enum that owns the *metadata* (name, title, description,
//!   bounds, visibility, enum names) and a shared handle to the value, and
//! * small typed handles ([`BoolSet`], [`IntSet`], ...) a module keeps as fields
//!   and reads with `.get()` -?the equivalent of holding `Setting<Boolean>`.
//!
//! Both point at the same `Arc` cell, so the GUI (which only knows the enum) and
//! the module (which holds the typed handle) always see one value, exactly like
//! the Java objects the GUI mutates in place.

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicI64, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use crate::meteor_core::color::{Color, SettingColor};

/// `IVisible` -?a condition evaluated every frame by the GUI.
pub type VisibleFn = Arc<dyn Fn() -> bool + Send + Sync>;

/// A setting's positional identity inside its group, for the command parser and
/// the GUI's "reset" action.
pub type SettingIndex = usize;

/// Which group a setting lives in (`Settings.createGroup`).
#[derive(Clone, Default)]
pub struct SettingGroup {
    pub name: String,
    pub section_expanded: bool,
    pub settings: Vec<Setting>,
}

impl SettingGroup {
    pub fn new(name: &str, section_expanded: bool) -> Self {
        Self {
            name: name.to_string(),
            section_expanded,
            settings: Vec::new(),
        }
    }

    pub fn get(&self, name: &str) -> Option<&Setting> {
        self.settings
            .iter()
            .find(|s| s.name.eq_ignore_ascii_case(name))
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut Setting> {
        self.settings
            .iter_mut()
            .find(|s| s.name.eq_ignore_ascii_case(name))
    }

    /// `SettingGroup.wasChanged()`.
    pub fn was_changed(&self) -> bool {
        self.settings.iter().any(|s| s.was_changed())
    }

    // ---- builders ----------------------------------------------------------

    /// `BoolSetting.Builder`.
    pub fn bool_setting(&mut self, name: &str, default: bool) -> BoolSet {
        let cell = Arc::new(AtomicBool::new(default));
        self.settings.push(Setting {
            name: name.to_string(),
            title: crate::meteor_core::util::name_to_title(name),
            description: String::new(),
            value: Value::Bool(cell.clone()),
            default: Value::Bool(Arc::new(AtomicBool::new(default))),
            visible: None,
        });
        BoolSet(cell)
    }

    /// `IntSetting.Builder` with a slider range.
    pub fn int_setting(&mut self, name: &str, default: i32, min: i32, max: i32) -> IntSet {
        let cell = Arc::new(AtomicI32::new(default));
        self.settings.push(Setting {
            name: name.to_string(),
            title: crate::meteor_core::util::name_to_title(name),
            description: String::new(),
            value: Value::Int {
                cell: cell.clone(),
                min,
                max,
            },
            default: Value::Int {
                cell: Arc::new(AtomicI32::new(default)),
                min,
                max,
            },
            visible: None,
        });
        IntSet { cell, min, max }
    }

    /// `DoubleSetting.Builder` with a slider range and decimal places.
    pub fn double_setting(
        &mut self,
        name: &str,
        default: f64,
        min: f64,
        max: f64,
        decimal_places: u32,
    ) -> DoubleSet {
        let cell = Arc::new(AtomicI64::new(default.to_bits() as i64));
        self.settings.push(Setting {
            name: name.to_string(),
            title: crate::meteor_core::util::name_to_title(name),
            description: String::new(),
            value: Value::Double {
                cell: cell.clone(),
                min,
                max,
                decimal_places,
            },
            default: Value::Double {
                cell: Arc::new(AtomicI64::new(default.to_bits() as i64)),
                min,
                max,
                decimal_places,
            },
            visible: None,
        });
        DoubleSet {
            cell,
            min,
            max,
            decimal_places,
        }
    }

    /// `StringSetting.Builder`.
    pub fn string_setting(&mut self, name: &str, default: &str) -> StringSet {
        let cell = Arc::new(Mutex::new(default.to_string()));
        self.settings.push(Setting {
            name: name.to_string(),
            title: crate::meteor_core::util::name_to_title(name),
            description: String::new(),
            value: Value::String(cell.clone()),
            default: Value::String(Arc::new(Mutex::new(default.to_string()))),
            visible: None,
        });
        StringSet(cell)
    }

    /// `EnumSetting.Builder`; `names` are the enum's constants in declaration
    /// order, `default` the index.
    pub fn enum_setting(
        &mut self,
        name: &str,
        names: &'static [&'static str],
        default: usize,
    ) -> EnumSet {
        let cell = Arc::new(AtomicU32::new(default as u32));
        self.settings.push(Setting {
            name: name.to_string(),
            title: crate::meteor_core::util::name_to_title(name),
            description: String::new(),
            value: Value::Enum {
                cell: cell.clone(),
                names,
            },
            default: Value::Enum {
                cell: Arc::new(AtomicU32::new(default as u32)),
                names,
            },
            visible: None,
        });
        EnumSet { cell, names }
    }

    /// `ColorSetting.Builder`.
    pub fn color_setting(&mut self, name: &str, default: SettingColor) -> ColorSet {
        let cell = Arc::new(AtomicU32::new(default.color.argb()));
        self.settings.push(Setting {
            name: name.to_string(),
            title: crate::meteor_core::util::name_to_title(name),
            description: String::new(),
            value: Value::Color(cell.clone()),
            default: Value::Color(Arc::new(AtomicU32::new(default.color.argb()))),
            visible: None,
        });
        ColorSet(cell)
    }

    /// `KeybindSetting.Builder`.
    pub fn keybind_setting(&mut self, name: &str) -> KeybindSet {
        let cell = Arc::new(Mutex::new(crate::meteor_core::util::Keybind::none()));
        self.settings.push(Setting {
            name: name.to_string(),
            title: crate::meteor_core::util::name_to_title(name),
            description: String::new(),
            value: Value::Keybind(cell.clone()),
            default: Value::Keybind(Arc::new(Mutex::new(
                crate::meteor_core::util::Keybind::none(),
            ))),
            visible: None,
        });
        KeybindSet(cell)
    }

    /// `ItemListSetting` / `BlockListSetting` / `EntityTypeListSetting` /
    /// `StringListSetting`: a list of namespaced ids, stored as text because
    /// Horizon names everything by id too.
    pub fn list_setting(&mut self, name: &str, default: &[&str]) -> ListSet {
        let default_vec: Vec<String> = default.iter().map(|s| s.to_string()).collect();
        let cell = Arc::new(Mutex::new(default_vec.clone()));
        self.settings.push(Setting {
            name: name.to_string(),
            title: crate::meteor_core::util::name_to_title(name),
            description: String::new(),
            value: Value::List(cell.clone()),
            default: Value::List(Arc::new(Mutex::new(default_vec))),
            visible: None,
        });
        ListSet(cell)
    }

    /// A list of item ids (`ItemListSetting`), kept separate so the GUI can draw
    /// icons and the command parser can complete item names.
    pub fn item_list_setting(&mut self, name: &str, default: &[&str]) -> ItemListSet {
        let default_vec: Vec<String> = default.iter().map(|s| s.to_string()).collect();
        let cell = Arc::new(Mutex::new(default_vec.clone()));
        self.settings.push(Setting {
            name: name.to_string(),
            title: crate::meteor_core::util::name_to_title(name),
            description: String::new(),
            value: Value::ItemList(cell.clone()),
            default: Value::ItemList(Arc::new(Mutex::new(default_vec))),
            visible: None,
        });
        ItemListSet(cell)
    }
}

/// The value cell plus its type-specific metadata.
#[derive(Clone)]
pub enum Value {
    Bool(Arc<AtomicBool>),
    Int {
        cell: Arc<AtomicI32>,
        min: i32,
        max: i32,
    },
    Double {
        cell: Arc<AtomicI64>,
        min: f64,
        max: f64,
        decimal_places: u32,
    },
    String(Arc<Mutex<String>>),
    Enum {
        cell: Arc<AtomicU32>,
        names: &'static [&'static str],
    },
    Color(Arc<AtomicU32>),
    Keybind(Arc<Mutex<crate::meteor_core::util::Keybind>>),
    List(Arc<Mutex<Vec<String>>>),
    ItemList(Arc<Mutex<Vec<String>>>),
}

/// One row of the settings GUI.
#[derive(Clone)]
pub struct Setting {
    pub name: String,
    pub title: String,
    pub description: String,
    pub value: Value,
    pub default: Value,
    pub visible: Option<VisibleFn>,
}

impl Setting {
    pub fn is_visible(&self) -> bool {
        match &self.visible {
            Some(f) => f(),
            None => true,
        }
    }

    pub fn was_changed(&self) -> bool {
        match (&self.value, &self.default) {
            (Value::Bool(a), Value::Bool(b)) => {
                a.load(Ordering::Relaxed) != b.load(Ordering::Relaxed)
            }
            (Value::Int { cell: a, .. }, Value::Int { cell: b, .. }) => {
                a.load(Ordering::Relaxed) != b.load(Ordering::Relaxed)
            }
            (Value::Double { cell: a, .. }, Value::Double { cell: b, .. }) => {
                a.load(Ordering::Relaxed) != b.load(Ordering::Relaxed)
            }
            (Value::String(a), Value::String(b)) => *a.lock().unwrap() != *b.lock().unwrap(),
            (Value::Enum { cell: a, .. }, Value::Enum { cell: b, .. }) => {
                a.load(Ordering::Relaxed) != b.load(Ordering::Relaxed)
            }
            (Value::Color(a), Value::Color(b)) => {
                a.load(Ordering::Relaxed) != b.load(Ordering::Relaxed)
            }
            (Value::Keybind(a), Value::Keybind(b)) => *a.lock().unwrap() != *b.lock().unwrap(),
            (Value::List(a), Value::List(b)) => *a.lock().unwrap() != *b.lock().unwrap(),
            (Value::ItemList(a), Value::ItemList(b)) => *a.lock().unwrap() != *b.lock().unwrap(),
            _ => false,
        }
    }

    pub fn reset(&self) {
        match (&self.value, &self.default) {
            (Value::Bool(a), Value::Bool(b)) => {
                a.store(b.load(Ordering::Relaxed), Ordering::Relaxed)
            }
            (Value::Int { cell: a, .. }, Value::Int { cell: b, .. }) => {
                a.store(b.load(Ordering::Relaxed), Ordering::Relaxed)
            }
            (Value::Double { cell: a, .. }, Value::Double { cell: b, .. }) => {
                a.store(b.load(Ordering::Relaxed), Ordering::Relaxed)
            }
            (Value::String(a), Value::String(b)) => *a.lock().unwrap() = b.lock().unwrap().clone(),
            (Value::Enum { cell: a, .. }, Value::Enum { cell: b, .. }) => {
                a.store(b.load(Ordering::Relaxed), Ordering::Relaxed)
            }
            (Value::Color(a), Value::Color(b)) => {
                a.store(b.load(Ordering::Relaxed), Ordering::Relaxed)
            }
            (Value::Keybind(a), Value::Keybind(b)) => *a.lock().unwrap() = *b.lock().unwrap(),
            (Value::List(a), Value::List(b)) => *a.lock().unwrap() = b.lock().unwrap().clone(),
            (Value::ItemList(a), Value::ItemList(b)) => {
                *a.lock().unwrap() = b.lock().unwrap().clone()
            }
            _ => {}
        }
    }

    /// The setting's value as display text (the GUI's sliders and the
    /// `setting` command print this form).
    pub fn display(&self) -> String {
        match &self.value {
            Value::Bool(c) => c.load(Ordering::Relaxed).to_string(),
            Value::Int { cell, .. } => cell.load(Ordering::Relaxed).to_string(),
            Value::Double {
                cell,
                decimal_places,
                ..
            } => format!(
                "{:.*}",
                *decimal_places as usize,
                f64::from_bits(cell.load(Ordering::Relaxed) as u64)
            ),
            Value::String(c) => c.lock().unwrap().clone(),
            Value::Enum { cell, names } => names
                .get(cell.load(Ordering::Relaxed) as usize)
                .copied()
                .unwrap_or("")
                .to_string(),
            Value::Color(c) => {
                let col = Color::from_argb(c.load(Ordering::Relaxed));
                format!("({}, {}, {}, {})", col.r, col.g, col.b, col.a)
            }
            Value::Keybind(c) => c.lock().unwrap().to_string(),
            Value::List(c) => c.lock().unwrap().join(", "),
            Value::ItemList(c) => c.lock().unwrap().join(", "),
        }
    }

    /// `Setting.parse(String)`: the `setting` command's setter. Returns false
    /// when the text does not fit the setting's kind or bounds.
    pub fn parse(&self, text: &str) -> bool {
        match &self.value {
            Value::Bool(cell) => match text.to_ascii_lowercase().as_str() {
                "true" | "on" | "yes" | "1" => {
                    cell.store(true, Ordering::Relaxed);
                    true
                }
                "false" | "off" | "no" | "0" => {
                    cell.store(false, Ordering::Relaxed);
                    true
                }
                _ => false,
            },
            Value::Int { cell, min, max } => match text.trim().parse::<i32>() {
                Ok(v) if v >= *min && v <= *max => {
                    cell.store(v, Ordering::Relaxed);
                    true
                }
                _ => false,
            },
            Value::Double { cell, min, max, .. } => match text.trim().parse::<f64>() {
                Ok(v) if v.is_finite() && v >= *min && v <= *max => {
                    cell.store(v.to_bits() as i64, Ordering::Relaxed);
                    true
                }
                _ => false,
            },
            Value::String(cell) => {
                *cell.lock().unwrap() = text.to_string();
                true
            }
            Value::Enum { cell, names } => {
                let needle = text.trim().to_ascii_lowercase().replace(' ', "-");
                match names
                    .iter()
                    .position(|n| n.to_ascii_lowercase().replace('_', "-") == needle)
                {
                    Some(i) => {
                        cell.store(i as u32, Ordering::Relaxed);
                        true
                    }
                    None => false,
                }
            }
            Value::Color(cell) => {
                let parsed = parse_color(text);
                match parsed {
                    Some(c) => {
                        cell.store(c.argb(), Ordering::Relaxed);
                        true
                    }
                    None => false,
                }
            }
            Value::Keybind(cell) => match crate::meteor_core::util::Keybind::parse(text) {
                Some(kb) => {
                    *cell.lock().unwrap() = kb;
                    true
                }
                None => false,
            },
            Value::List(cell) => {
                let items: Vec<String> = text
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                *cell.lock().unwrap() = items;
                true
            }
            Value::ItemList(cell) => {
                let items: Vec<String> = text
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                *cell.lock().unwrap() = items;
                true
            }
        }
    }
}

/// `(r, g, b, a)` or `(r, g, b)` text, also accepting `#RRGGBBAA`.
pub fn parse_color(text: &str) -> Option<Color> {
    let t = text.trim().trim_start_matches('(').trim_end_matches(')');
    if let Some(hex) = t.strip_prefix('#') {
        let v = u32::from_str_radix(hex, 16).ok()?;
        return Some(if hex.len() == 8 {
            // RRGGBBAA, the order the GUI's colour box shows.
            Color::new(
                ((v >> 24) & 0xFF) as u8,
                ((v >> 16) & 0xFF) as u8,
                ((v >> 8) & 0xFF) as u8,
                (v & 0xFF) as u8,
            )
        } else {
            Color::new(
                ((v >> 16) & 0xFF) as u8,
                ((v >> 8) & 0xFF) as u8,
                (v & 0xFF) as u8,
                255,
            )
        });
    }
    let parts: Vec<&str> = t.split(',').map(|p| p.trim()).collect();
    match parts.len() {
        3 | 4 => {
            let mut c = [255i32; 4];
            for i in 0..parts.len() {
                c[i] = parts[i].parse::<i32>().ok()?;
            }
            Some(Color::from_ints(c[0], c[1], c[2], c[3]))
        }
        _ => None,
    }
}

/// `Settings` -?the ordered group list of one module, HUD element or theme.
#[derive(Clone, Default)]
pub struct Settings {
    pub groups: Vec<SettingGroup>,
}

impl Settings {
    pub fn new() -> Self {
        Self { groups: Vec::new() }
    }

    /// `Settings.getDefaultGroup()`.
    pub fn default_group(&mut self) -> &mut SettingGroup {
        if self.groups.is_empty() {
            self.groups.push(SettingGroup::new("General", true));
        }
        self.groups.first_mut().unwrap()
    }

    /// `Settings.createGroup(name)`.
    pub fn create_group(&mut self, name: &str) -> &mut SettingGroup {
        self.groups.push(SettingGroup::new(name, true));
        self.groups.last_mut().unwrap()
    }

    pub fn get_group(&self, name: &str) -> Option<&SettingGroup> {
        self.groups.iter().find(|g| g.name == name)
    }

    /// `Settings.get(String)`, searching every group.
    pub fn get(&self, name: &str) -> Option<&Setting> {
        self.groups.iter().find_map(|g| g.get(name))
    }

    pub fn iter(&self) -> impl Iterator<Item = &Setting> {
        self.groups.iter().flat_map(|g| g.settings.iter())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Setting> {
        self.groups.iter_mut().flat_map(|g| g.settings.iter_mut())
    }

    pub fn len(&self) -> usize {
        self.groups.iter().map(|g| g.settings.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// `Settings.reset()`.
    pub fn reset(&self) {
        for s in self.iter() {
            s.reset();
        }
    }

    pub fn was_changed(&self) -> bool {
        self.iter().any(|s| s.was_changed())
    }

    /// Attach a visibility predicate to the last added setting (Java's
    /// `visible(() -> ...)` builder call, which comes before `.build()`).
    pub fn set_visible_last(&mut self, visible: VisibleFn) {
        if let Some(g) = self.groups.last_mut() {
            if let Some(s) = g.settings.last_mut() {
                s.visible = Some(visible);
            }
        }
    }

    /// Attach a description to the last added setting.
    pub fn describe_last(&mut self, description: &str) {
        if let Some(g) = self.groups.last_mut() {
            if let Some(s) = g.settings.last_mut() {
                s.description = description.to_string();
            }
        }
    }

    /// Find the group index and setting index of a name, for the GUI's scroll
    /// position bookkeeping and the `setting` command's reset form.
    pub fn locate(&self, name: &str) -> Option<(usize, usize)> {
        for (gi, g) in self.groups.iter().enumerate() {
            for (si, s) in g.settings.iter().enumerate() {
                if s.name.eq_ignore_ascii_case(name) {
                    return Some((gi, si));
                }
            }
        }
        None
    }
}

// ---- typed handles ---------------------------------------------------------

#[derive(Clone)]
pub struct BoolSet(pub Arc<AtomicBool>);

impl BoolSet {
    pub fn get(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
    pub fn set(&self, v: bool) {
        self.0.store(v, Ordering::Relaxed);
    }
    pub fn toggle(&self) -> bool {
        let v = !self.get();
        self.set(v);
        v
    }
}

#[derive(Clone)]
pub struct IntSet {
    pub cell: Arc<AtomicI32>,
    pub min: i32,
    pub max: i32,
}

impl IntSet {
    pub fn get(&self) -> i32 {
        self.cell.load(Ordering::Relaxed)
    }
    pub fn set(&self, v: i32) {
        self.cell
            .store(v.clamp(self.min, self.max), Ordering::Relaxed);
    }
}

#[derive(Clone)]
pub struct DoubleSet {
    pub cell: Arc<AtomicI64>,
    pub min: f64,
    pub max: f64,
    pub decimal_places: u32,
}

impl DoubleSet {
    pub fn get(&self) -> f64 {
        f64::from_bits(self.cell.load(Ordering::Relaxed) as u64)
    }
    pub fn set(&self, v: f64) {
        let v = if v.is_finite() { v } else { self.min };
        self.cell.store(
            v.clamp(self.min, self.max).to_bits() as i64,
            Ordering::Relaxed,
        );
    }
    pub fn get_float(&self) -> f32 {
        self.get() as f32
    }
    pub fn get_i32(&self) -> i32 {
        self.get().round() as i32
    }
}

#[derive(Clone)]
pub struct StringSet(pub Arc<Mutex<String>>);

impl StringSet {
    pub fn get(&self) -> String {
        self.0.lock().unwrap().clone()
    }
    pub fn set(&self, v: &str) {
        *self.0.lock().unwrap() = v.to_string();
    }
    pub fn is(&self, v: &str) -> bool {
        self.0.lock().unwrap().eq_ignore_ascii_case(v)
    }
}

#[derive(Clone)]
pub struct EnumSet {
    pub cell: Arc<AtomicU32>,
    pub names: &'static [&'static str],
}

impl EnumSet {
    pub fn index(&self) -> usize {
        self.cell.load(Ordering::Relaxed) as usize
    }
    pub fn get(&self) -> &'static str {
        self.names.get(self.index()).copied().unwrap_or("")
    }
    pub fn set_index(&self, i: usize) {
        self.cell.store(
            (i.min(self.names.len().saturating_sub(1))) as u32,
            Ordering::Relaxed,
        );
    }
    pub fn is(&self, name: &str) -> bool {
        self.get().eq_ignore_ascii_case(name)
    }
    pub fn names(&self) -> &'static [&'static str] {
        self.names
    }
}

#[derive(Clone)]
pub struct ColorSet(pub Arc<AtomicU32>);

impl ColorSet {
    pub fn get(&self) -> SettingColor {
        SettingColor::from_color(Color::from_argb(self.0.load(Ordering::Relaxed)))
    }
    pub fn color(&self) -> Color {
        self.get().color
    }
    pub fn set(&self, c: Color) {
        self.0.store(c.argb(), Ordering::Relaxed);
    }
}

#[derive(Clone)]
pub struct KeybindSet(pub Arc<Mutex<crate::meteor_core::util::Keybind>>);

impl KeybindSet {
    pub fn get(&self) -> crate::meteor_core::util::Keybind {
        *self.0.lock().unwrap()
    }
    pub fn set(&self, kb: crate::meteor_core::util::Keybind) {
        *self.0.lock().unwrap() = kb;
    }
}

#[derive(Clone)]
pub struct ListSet(pub Arc<Mutex<Vec<String>>>);

impl ListSet {
    pub fn get(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
    pub fn contains(&self, id: &str) -> bool {
        self.0
            .lock()
            .unwrap()
            .iter()
            .any(|s| s.eq_ignore_ascii_case(id))
    }
    pub fn has_any(&self) -> bool {
        !self.0.lock().unwrap().is_empty()
    }
    pub fn set(&self, ids: Vec<String>) {
        *self.0.lock().unwrap() = ids;
    }
    pub fn is_empty(&self) -> bool {
        self.0.lock().unwrap().is_empty()
    }
}

#[derive(Clone)]
pub struct ItemListSet(pub Arc<Mutex<Vec<String>>>);

impl ItemListSet {
    pub fn get(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
    pub fn contains(&self, id: &str) -> bool {
        self.0
            .lock()
            .unwrap()
            .iter()
            .any(|s| s.eq_ignore_ascii_case(id))
    }
    pub fn is_empty(&self) -> bool {
        self.0.lock().unwrap().is_empty()
    }
    pub fn set(&self, ids: Vec<String>) {
        *self.0.lock().unwrap() = ids;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_and_table_share_one_value() {
        let mut s = Settings::new();
        let b = s.default_group().bool_setting("test", false);
        assert!(!b.get());
        s.get("test").unwrap().parse("true");
        assert!(b.get());
        assert!(s.get("test").unwrap().was_changed());
    }

    #[test]
    fn int_bounds_are_enforced() {
        let mut s = Settings::new();
        let i = s.default_group().int_setting("range", 5, 0, 10);
        s.get("range").unwrap().parse("42");
        assert_eq!(i.get(), 5, "out-of-range parse must be rejected");
        i.set(42);
        assert_eq!(i.get(), 10);
    }

    #[test]
    fn double_round_trips_bits() {
        let mut s = Settings::new();
        let d = s.default_group().double_setting("speed", 1.5, 0.0, 10.0, 2);
        assert_eq!(d.get(), 1.5);
        d.set(2.25);
        assert_eq!(s.get("speed").unwrap().display(), "2.25");
    }

    #[test]
    fn enum_by_name() {
        let mut s = Settings::new();
        let e = s
            .default_group()
            .enum_setting("mode", &["vanilla", "packet", "bypass"], 0);
        assert!(e.is("vanilla"));
        assert!(s.get("mode").unwrap().parse("bypass"));
        assert_eq!(e.index(), 2);
        assert!(!s.get("mode").unwrap().parse("nonsense"));
    }

    #[test]
    fn color_parsing() {
        assert_eq!(parse_color("(1, 2, 3, 4)"), Some(Color::new(1, 2, 3, 4)));
        assert_eq!(
            parse_color("#11223344"),
            Some(Color::new(0x11, 0x22, 0x33, 0x44))
        );
        assert_eq!(parse_color("#112233"), Some(Color::rgb(0x11, 0x22, 0x33)));
        assert_eq!(parse_color("nope"), None);
    }

    #[test]
    fn reset_restores_defaults() {
        let mut s = Settings::new();
        let i = s.default_group().int_setting("x", 3, 0, 9);
        i.set(8);
        assert!(s.was_changed());
        s.reset();
        assert_eq!(i.get(), 3);
        assert!(!s.was_changed());
    }
}
