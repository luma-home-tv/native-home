//! Dashboard-only filtering and owned native text draft. No TV operation is exposed.
use plx_machine::machine::TextEdit;
use plx_ui::text_buffer::TextBuffer;
use unicode_normalization::UnicodeNormalization;

pub const MAX_QUERY_BYTES: usize = 256;
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Drawer {
    pub favourites_only: bool,
    pub editing: bool,
    text: TextBuffer,
    tokens: Vec<String>,
    composition: String,
}
pub const KEYS: [&str; 40] = [
    "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S",
    "T", "U", "V", "W", "X", "Y", "Z", "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "Space",
    "Delete", "Clear", "Done",
];
fn folded(text: &str) -> String {
    text.nfc()
        .collect::<String>()
        .to_lowercase()
        .nfc()
        .collect()
}
fn admitted(text: &str) -> bool {
    !text.chars().any(char::is_control)
}
impl Drawer {
    pub fn query(&self) -> &str {
        self.text.text()
    }
    pub fn caret(&self) -> usize {
        self.text.caret()
    }
    pub fn display_parts(&self) -> (String, String, String) {
        let (before, after) = self.query().split_at(self.caret());
        (
            before.nfc().collect(),
            self.composition.nfc().collect(),
            after.nfc().collect(),
        )
    }
    pub fn composition(&self) -> &str {
        &self.composition
    }
    fn refresh_tokens(&mut self) {
        self.tokens = folded(self.query())
            .split_whitespace()
            .map(str::to_owned)
            .collect();
    }
    pub fn set_query(&mut self, text: &str) -> bool {
        if text.len() > MAX_QUERY_BYTES || !admitted(text) {
            return false;
        }
        self.text = TextBuffer::new(text.into(), text.len());
        self.composition.clear();
        self.refresh_tokens();
        true
    }
    pub fn matches(&self, title: &str, favourite: bool) -> bool {
        if self.favourites_only && !favourite {
            return false;
        }
        let title = folded(title);
        self.tokens.iter().all(|word| title.contains(word))
    }
    /// Preserve SDL commit boundaries. Only the explicitly selected LG prediction path replaces words.
    pub fn commit(&mut self, text: &str, prediction: bool) -> bool {
        if !self.editing || !admitted(text) {
            return false;
        }
        let changed = if prediction {
            self.text
                .edit_bounded(&TextEdit::Commit(text.into()), MAX_QUERY_BYTES)
        } else {
            self.text.insert_bounded(text, MAX_QUERY_BYTES)
        };
        self.composition.clear();
        if changed {
            self.refresh_tokens();
        }
        changed
    }
    pub fn preedit(&mut self, text: &str) -> bool {
        if !self.editing
            || text.len() > MAX_QUERY_BYTES
            || !admitted(text)
            || self.composition == text
        {
            return false;
        }
        self.composition = text.into();
        true
    }
    pub fn edit(&mut self, edit: &TextEdit) -> bool {
        if !self.editing {
            return false;
        }
        let changed = self.text.edit_bounded(edit, MAX_QUERY_BYTES);
        if changed {
            self.composition.clear();
            self.refresh_tokens();
        }
        changed
    }
    pub fn delete_forward(&mut self) -> bool {
        if !self.editing {
            return false;
        }
        let before = self.text.clone();
        self.text.delete_forward();
        if self.text == before {
            return false;
        }
        self.composition.clear();
        self.refresh_tokens();
        true
    }
    pub fn finish_editing(&mut self) {
        self.editing = false;
        self.composition.clear();
    }
    pub fn key(&mut self, index: usize) -> bool {
        let before = self.clone();
        match KEYS.get(index).copied() {
            Some("Done") => self.finish_editing(),
            Some("Delete") => {
                self.edit(&TextEdit::Backspace);
            }
            Some("Clear") => {
                self.edit(&TextEdit::Clear);
            }
            Some("Space") => {
                self.commit(" ", false);
            }
            Some(key) if key.len() == 1 => {
                self.commit(key, false);
            }
            _ => {}
        }
        *self != before
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filters_compose_and_match_canonical_case_insensitive_words() {
        let mut d = Drawer::default();
        d.set_query("TV apple");
        assert!(d.matches("Apple TV", false));
        assert!(!d.matches("TV channel", true));
        d.favourites_only = true;
        assert!(!d.matches("Apple TV", false));
        assert!(d.matches("Apple TV", true));
        d.set_query("E\u{301}C");
        assert!(d.matches("Écran", true));
        d.set_query("ÉC");
        assert!(d.matches("E\u{301}cran", true));
        d.set_query("電視");
        assert!(d.matches("我的電視", true));
        d.set_query("Kids' TV+");
        assert!(d.matches("Kids' TV+", true));
    }
    #[test]
    fn edits_are_bounded_atomic_and_unknown_keys_do_nothing() {
        let mut d = Drawer {
            editing: true,
            ..Default::default()
        };
        for _ in 0..MAX_QUERY_BYTES + 8 {
            d.key(0);
        }
        assert_eq!(d.query().len(), MAX_QUERY_BYTES);
        assert!(!d.key(99));
        let before = d.clone();
        assert!(!d.commit("電視", false));
        assert_eq!(d, before);
        d.key(37);
        assert_eq!(d.query().len(), MAX_QUERY_BYTES - 1);
        d.key(38);
        assert!(d.query().is_empty());
        d.key(39);
        assert!(!d.editing);
        assert!(!d.commit("hidden", false));
        assert!(!d.set_query("line\nbreak"));
    }
    #[test]
    fn owned_draft_keeps_literal_batches_predictions_preedit_and_grapheme_deletion_distinct() {
        let mut d = Drawer {
            editing: true,
            ..Default::default()
        };
        d.commit("app", false);
        d.commit("le", false);
        assert_eq!(d.query(), "apple");
        assert!(d.preedit("電"));
        assert_eq!(d.query(), "apple");
        assert_eq!(d.composition(), "電");
        d.commit("Apple ", true);
        assert_eq!(d.query(), "Apple ");
        assert!(d.composition().is_empty());
        d.commit("e\u{301}", false);
        d.key(37);
        assert_eq!(d.query(), "Apple ");
        d.commit("👩‍👩‍👧‍👦", false);
        d.key(37);
        assert_eq!(d.query(), "Apple ");
        d.edit(&TextEdit::Left);
        d.commit("X", false);
        assert_eq!(d.query(), "AppleX ");
    }
}
