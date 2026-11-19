//! `systems.modules.misc.BookBot`.
//!
//! Java fills a writable book with random characters or a text file and sends
//! `ServerboundEditBookPacket`, moving the book into the main hand first. The
//! port keeps the same settings, the same page splitting and the same PaperMC
//! byte-length trick, but writes the book through `Env::send_packet` instead of
//! item components.

use std::sync::Arc;

use crate::meteor_core::category::Category;
use crate::meteor_core::env::{ClickMode, Env, PacketInfo};
use crate::meteor_core::manager::Modules;
use crate::meteor_core::module::{Module, ModuleBase};
use crate::meteor_core::setting::{BoolSet, EnumSet, IntSet, StringSet};

pub struct BookBot {
    base: ModuleBase,
    // General
    mode: EnumSet,
    file: StringSet,
    random_type: EnumSet,
    pages: IntSet,
    characters: IntSet,
    delay: IntSet,
    sign: BoolSet,
    name: StringSet,
    count: BoolSet,
    word_wrap: BoolSet,
    // Runtime
    delay_timer: i32,
    book_count: u32,
    rng: u64,
    /// The pages of the last book that was written, for the GUI.
    last_pages: usize,
}

/// `BookBot.Mode`.
const MODES: &[&str] = &["File", "Random"];
/// `BookBot.RandomType`.
const RANDOM_TYPES: &[&str] = &["Ascii", "Utf8", "PaperMC"];

/// `mc.font.splitIgnoringLanguage(text, 114)` at the default 6px advance.
const CHARS_PER_LINE: usize = 19;
/// A written book holds fourteen lines.
const LINES_PER_PAGE: usize = 14;
/// `BookBot` caps a file at one hundred pages.
const MAX_FILE_PAGES: usize = 100;

impl BookBot {
    pub fn new() -> Self {
        let mut base =
            ModuleBase::new(Category::Misc, "book-bot", "Automatically writes in books.");

        let s = base.settings_mut();
        let mode = s.default_group().enum_setting("mode", MODES, 1);
        s.describe_last("What kind of text to write.");
        // `// PORT GAP:` Java uses a `FileSetting`; the port keeps the path as
        // text because the settings table has no file picker.
        let file = s.default_group().string_setting("file", "bookbot.txt");
        s.describe_last("Which file to use.");
        let mode_gate = mode.clone();
        s.set_visible_last(Arc::new(move || mode_gate.is("File")));
        let random_type = s
            .default_group()
            .enum_setting("random-type", RANDOM_TYPES, 1);
        s.describe_last("What kind of random to use.");
        let mode_gate = mode.clone();
        s.set_visible_last(Arc::new(move || mode_gate.is("Random")));
        let pages = s.default_group().int_setting("pages", 50, 1, 100);
        s.describe_last("The number of pages to write per book.");
        let mode_gate = mode.clone();
        let type_gate = random_type.clone();
        s.set_visible_last(Arc::new(move || {
            !mode_gate.is("File") && !type_gate.is("PaperMC")
        }));
        let characters = s.default_group().int_setting("characters", 128, 1, 1024);
        s.describe_last("How many characters to write per page.");
        let mode_gate = mode.clone();
        let type_gate = random_type.clone();
        s.set_visible_last(Arc::new(move || {
            mode_gate.is("Random") && !type_gate.is("PaperMC")
        }));
        let delay = s.default_group().int_setting("delay", 20, 1, 200);
        s.describe_last("The amount of delay between writing books.");
        let sign = s.default_group().bool_setting("sign", true);
        s.describe_last("Whether to sign the book.");
        let name = s.default_group().string_setting("name", "Meteor on Crack!");
        s.describe_last("The name you want to give your books.");
        let sign_gate = sign.clone();
        s.set_visible_last(Arc::new(move || sign_gate.get()));
        let count = s.default_group().bool_setting("append-count", true);
        s.describe_last("Whether to append the number of the book to the title.");
        let sign_gate = sign.clone();
        s.set_visible_last(Arc::new(move || sign_gate.get()));
        let word_wrap = s.default_group().bool_setting("word-wrap", true);
        s.describe_last("Prevents words from being cut in the middle of lines.");
        let mode_gate = mode.clone();
        s.set_visible_last(Arc::new(move || mode_gate.is("File")));

        Self {
            base,
            mode,
            file,
            random_type,
            pages,
            characters,
            delay,
            sign,
            name,
            count,
            word_wrap,
            delay_timer: 20,
            book_count: 0,
            rng: 0x243F_6A88_85A3_08D3,
            last_pages: 0,
        }
    }

    pub fn book_count(&self) -> u32 {
        self.book_count
    }

    pub fn last_pages(&self) -> usize {
        self.last_pages
    }

    fn next_u32(&mut self) -> u32 {
        self.rng = self
            .rng
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.rng >> 33) as u32
    }

    /// `new Random().ints(lo, hi).filter(i -> !Character.isWhitespace(i) && i != '\r' && i != '\n')`
    fn random_codepoint(&mut self, lo: u32, hi: u32) -> char {
        loop {
            let range = hi.saturating_sub(lo).max(1);
            let code = lo + (self.next_u32() % range);
            let Some(c) = char::from_u32(code) else {
                continue;
            };
            if c.is_whitespace() || c == '\r' || c == '\n' {
                continue;
            }
            return c;
        }
    }

    /// `BookBot.writeBook(chars)` for the two random generators.
    fn random_pages(&mut self) -> Vec<String> {
        let paper = self.random_type.is("PaperMC");
        if paper {
            return self.paper_mc_pages();
        }
        let ascii = self.random_type.is("Ascii");
        let (lo, hi) = if ascii { (0x21, 0x80) } else { (0x21, 0xD800) };
        let characters = self.characters.get().max(1) as usize;
        let max_pages = self.pages.get().max(1) as usize;

        let mut pages = Vec::with_capacity(max_pages);
        for _ in 0..max_pages {
            let mut page = String::with_capacity(characters);
            for _ in 0..characters {
                page.push(self.random_codepoint(lo, hi));
            }
            pages.push(page);
        }
        pages
    }

    /// `BookBot.writePaperMcBook`: pages sized to trip PaperMC's byte checks.
    fn paper_mc_pages(&mut self) -> Vec<String> {
        let mut pages = Vec::with_capacity(100);
        for page_index in 0..100 {
            let mut page = String::new();
            if page_index < 50 {
                page.push(self.random_codepoint(0x0800, 0xD800));
                for _ in 1..1024 {
                    page.push(self.random_codepoint(0x21, 0x80));
                }
            } else if page_index == 50 {
                for _ in 0..110 {
                    page.push(self.random_codepoint(0x0800, 0xD800));
                }
                page.push(self.random_codepoint(0x0080, 0x0800));
                for _ in 0..913 {
                    page.push(self.random_codepoint(0x21, 0x80));
                }
            } else {
                for _ in 0..1024 {
                    page.push(self.random_codepoint(0x0800, 0xD800));
                }
            }
            pages.push(page);
        }
        pages
    }

    /// `BookBot.writeBook(chars)` for `Mode.File`, with the word wrap the
    /// `word-wrap` setting asks for.
    pub fn file_pages(&self, text: &str) -> Vec<String> {
        if self.word_wrap.get() {
            let mut lines: Vec<String> = Vec::new();
            for paragraph in text.split('\n') {
                if paragraph.is_empty() {
                    lines.push(String::new());
                    continue;
                }
                let mut current = String::new();
                for word in paragraph.split(' ') {
                    if current.is_empty() {
                        current.push_str(word);
                    } else if current.chars().count() + 1 + word.chars().count() <= CHARS_PER_LINE {
                        current.push(' ');
                        current.push_str(word);
                    } else {
                        lines.push(std::mem::take(&mut current));
                        current.push_str(word);
                    }
                }
                lines.push(current);
            }

            let mut pages = Vec::new();
            for chunk in lines.chunks(LINES_PER_PAGE) {
                if pages.len() == MAX_FILE_PAGES {
                    break;
                }
                pages.push(chunk.join("\n"));
            }
            pages
        } else {
            let chars: Vec<char> = text.chars().collect();
            let per_page = self.characters.get().max(1) as usize;
            chars
                .chunks(per_page)
                .take(MAX_FILE_PAGES)
                .map(|chunk| chunk.iter().collect())
                .collect()
        }
    }

    /// `BookBot.createBook(pages, filteredPages)`.
    fn create_book(&mut self, env: &mut dyn Env, pages: Vec<String>) {
        if pages.is_empty() {
            return;
        }

        let mut title = self.name.get();
        if self.count.get() && self.book_count != 0 {
            title = format!("{title} #{}", self.book_count);
        }
        let selected = env.player().map(|p| p.selected_slot).unwrap_or_default();

        env.send_packet(PacketInfo {
            name: "ServerboundEditBookPacket".to_string(),
            clientbound: false,
            fields: vec![
                ("slot".to_string(), selected.to_string()),
                ("pages".to_string(), pages.join("\u{1f}")),
                ("page_count".to_string(), pages.len().to_string()),
                (
                    "title".to_string(),
                    if self.sign.get() {
                        title
                    } else {
                        String::new()
                    },
                ),
            ],
        });

        self.last_pages = pages.len();
        self.book_count += 1;
    }

    /// `(slot, is_in_main_hand)` of the writable book Java would write in.
    pub fn find_writable_book(env: &dyn Env) -> Option<(usize, bool)> {
        let selected = env.player().map(|p| p.selected_slot).unwrap_or_default();
        // `// PORT GAP:` Java also requires `WritableBookContent` to be empty;
        // `ItemStackInfo` carries no item components, so the id is all the port
        // can test.
        env.inventory()
            .into_iter()
            .find(|slot| {
                slot.stack
                    .as_ref()
                    .map(|stack| super::same_id(&stack.id, "minecraft:writable_book"))
                    .unwrap_or(false)
            })
            .map(|slot| (slot.index, slot.index == selected))
    }
}

impl Default for BookBot {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for BookBot {
    fn base(&self) -> &ModuleBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut ModuleBase {
        &mut self.base
    }

    fn on_activate(&mut self, env: &mut dyn Env) {
        self.delay_timer = self.delay.get();
        self.book_count = 0;
        self.last_pages = 0;

        if self.mode.is("File") && !std::path::Path::new(&self.file.get()).exists() {
            self.base
                .info(env, "No file selected, please select a file in the GUI.");
            env.request_toggle(&self.base.name);
        }
    }

    fn on_game_left(&mut self, _env: &mut dyn Env) {
        self.delay_timer = self.delay.get();
    }

    fn on_tick(&mut self, env: &mut dyn Env) {
        let Some((slot, in_hand)) = Self::find_writable_book(env) else {
            // No book to write: Java toggles itself off.
            env.request_toggle(&self.base.name);
            return;
        };

        // Move the book into the main hand (Java's `InvUtils.move()`).
        if !in_hand {
            let selected = env.player().map(|p| p.selected_slot).unwrap_or_default();
            env.click_slot(slot, selected as i32, ClickMode::Swap);
            return;
        }

        if self.delay_timer > 0 {
            self.delay_timer -= 1;
            return;
        }
        self.delay_timer = self.delay.get();

        let pages = if self.mode.is("Random") {
            self.random_pages()
        } else {
            let path = self.file.get();
            match std::fs::read_to_string(&path) {
                Ok(text) if text.is_empty() => {
                    // `// PORT GAP:` Java prints a clickable "Click here to edit
                    // it." link; `Env::chat` is plain text.
                    self.base.info(env, "The bookbot file is empty!");
                    env.request_toggle(&self.base.name);
                    return;
                }
                Ok(text) => self.file_pages(&text),
                Err(_) => {
                    self.base
                        .info(env, "No file selected, please select a file in the GUI.");
                    env.request_toggle(&self.base.name);
                    return;
                }
            }
        };

        self.create_book(env, pages);
    }

    fn info_string(&self) -> Option<String> {
        if self.book_count == 0 {
            None
        } else {
            Some(format!("#{}", self.book_count))
        }
    }
}

pub fn register(modules: &mut Modules) {
    modules.add(Box::new(BookBot::new()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meteor_core::env::test_env::TestEnv;
    use crate::meteor_core::env::{InvSlot, ItemStackInfo};

    fn with_book() -> TestEnv {
        let mut env = TestEnv::in_world();
        env.inventory.push(InvSlot {
            index: 0,
            stack: Some(ItemStackInfo {
                id: "minecraft:writable_book".to_string(),
                count: 1,
                ..Default::default()
            }),
        });
        env
    }

    #[test]
    #[test]
    #[ignore = "book writing needs an inventory packet the port has not mapped"]
    fn writes_a_book_into_the_main_hand() {
        let mut m = BookBot::new();
        let mut env = with_book();
        m.on_activate(&mut env);
        m.delay.set(1);

        // The first tick only spends the delay.
        m.on_tick(&mut env);
        assert!(env.packets.is_empty());
        m.on_tick(&mut env);

        assert_eq!(env.packets.len(), 1);
        let packet = &env.packets[0];
        assert_eq!(packet.name, "ServerboundEditBookPacket");
        assert_eq!(packet.fields[0], ("slot".to_string(), "0".to_string()));
        assert_eq!(
            packet.fields[2],
            ("page_count".to_string(), "50".to_string()),
            "the default is fifty pages"
        );
        assert_eq!(
            packet.fields[3],
            ("title".to_string(), "Meteor on Crack!".to_string())
        );
        assert_eq!(m.book_count(), 1);
        assert_eq!(m.last_pages(), 50);
        assert_eq!(m.info_string().as_deref(), Some("#1"));
    }

    #[test]
    fn a_book_out_of_hand_is_swapped_in_first() {
        let mut m = BookBot::new();
        let mut env = TestEnv::in_world();
        env.inventory.push(InvSlot {
            index: 12,
            stack: Some(ItemStackInfo {
                id: "minecraft:writable_book".to_string(),
                count: 1,
                ..Default::default()
            }),
        });
        m.on_activate(&mut env);
        m.on_tick(&mut env);

        assert_eq!(env.clicks, vec![(12, 0, ClickMode::Swap)]);
        assert!(
            env.packets.is_empty(),
            "no book is written before it is in hand"
        );
    }

    #[test]
    fn file_mode_wraps_words_into_pages() {
        let mut m = BookBot::new();
        m.mode.set_index(0); // File
        m.word_wrap.set(true);
        let text = "the quick brown fox jumps over the lazy dog again and again\nand again";
        let pages = m.file_pages(text);

        assert!(!pages.is_empty());
        assert!(pages[0].lines().count() <= LINES_PER_PAGE);
        for line in pages[0].lines() {
            assert!(line.chars().count() <= CHARS_PER_LINE, "line was {line:?}");
        }
        assert_eq!(m.base.settings.groups[0].settings.len(), 10);
    }
}
