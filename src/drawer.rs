//! Dashboard-only app filtering and native keyboard state. No TV operation is exposed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Drawer {
    pub favourites_only: bool,
    pub query: String,
    pub editing: bool,
}
pub const KEYS: [&str; 40] = [
    "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S",
    "T", "U", "V", "W", "X", "Y", "Z", "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "Space",
    "Delete", "Clear", "Done",
];
impl Drawer {
    pub fn matches(&self, title: &str, favourite: bool) -> bool {
        if self.favourites_only && !favourite {
            return false;
        }
        let title = title.to_lowercase();
        self.query
            .split_whitespace()
            .all(|word| title.contains(&word.to_lowercase()))
    }
    /// Apply one committed key, with a bound on both query size and admitted key identity.
    pub fn key(&mut self, index: usize) -> bool {
        let before = self.clone();
        match KEYS.get(index).copied() {
            Some("Done") => self.editing = false,
            Some("Delete") => {
                self.query.pop();
            }
            Some("Clear") => self.query.clear(),
            Some("Space") if self.query.len() < 32 => self.query.push(' '),
            Some(key) if key.len() == 1 && self.query.len() < 32 => self.query.push_str(key),
            _ => {}
        }
        *self != before
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filters_compose_and_match_case_insensitive_words() {
        let mut d = Drawer {
            query: "TV apple".into(),
            ..Default::default()
        };
        assert!(d.matches("Apple TV", false));
        assert!(!d.matches("TV channel", true));
        d.favourites_only = true;
        assert!(!d.matches("Apple TV", false));
        assert!(d.matches("Apple TV", true));
        d.query = "ÉC".into();
        assert!(d.matches("Écran", true));
    }
    #[test]
    fn keyboard_edits_are_bounded_and_unknown_keys_do_nothing() {
        let mut d = Drawer {
            editing: true,
            ..Default::default()
        };
        for _ in 0..40 {
            d.key(0);
        }
        assert_eq!(d.query.len(), 32);
        assert!(!d.key(99));
        d.key(37);
        assert_eq!(d.query.len(), 31);
        d.key(38);
        assert!(d.query.is_empty());
        d.key(39);
        assert!(!d.editing);
    }
}
