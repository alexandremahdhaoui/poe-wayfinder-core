pub const MAX_CHARS: usize = 20_000;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Notepad {
    text: String,
    saved: String,
}

impl Notepad {
    pub fn opened_with(text: &str) -> Self {
        let text = trim_to_limit(text);

        Self {
            saved: text.clone(),
            text,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn write(&mut self, text: &str) {
        self.text = trim_to_limit(text);
    }

    pub fn is_dirty(&self) -> bool {
        self.text != self.saved
    }

    pub fn saved(&mut self) {
        self.saved = self.text.clone();
    }

    pub fn line_count(&self) -> usize {
        match self.text.is_empty() {
            true => 0,
            false => self.text.lines().count(),
        }
    }
}

fn trim_to_limit(text: &str) -> String {
    match text.chars().count() > MAX_CHARS {
        true => text.chars().take(MAX_CHARS).collect(),
        false => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_notepad_is_empty_and_clean() {
        let pad = Notepad::default();

        assert_eq!(pad.text(), "");
        assert!(!pad.is_dirty());
        assert_eq!(pad.line_count(), 0);
    }

    #[test]
    fn what_was_opened_is_not_dirty() {
        let pad = Notepad::opened_with("chaos recipe: 60-74");

        assert!(!pad.is_dirty(), "opening a file is not an edit");
    }

    #[test]
    fn writing_makes_it_dirty_and_saving_makes_it_clean() {
        let mut pad = Notepad::opened_with("one");

        pad.write("two");

        assert!(pad.is_dirty());

        pad.saved();

        assert!(!pad.is_dirty());
    }

    #[test]
    fn writing_back_the_same_text_is_not_an_edit() {
        let mut pad = Notepad::opened_with("one");

        pad.write("one");

        assert!(!pad.is_dirty());
    }

    #[test]
    fn lines_are_counted_for_the_status_line() {
        let pad = Notepad::opened_with("a\nb\nc");

        assert_eq!(pad.line_count(), 3);
    }

    #[test]
    fn a_note_longer_than_the_limit_is_cut_rather_than_kept_whole() {
        let huge = "x".repeat(MAX_CHARS * 2);
        let pad = Notepad::opened_with(&huge);

        assert_eq!(pad.text().chars().count(), MAX_CHARS);
    }

    #[test]
    fn cutting_at_the_limit_never_splits_a_character_in_half() {
        let huge = "é".repeat(MAX_CHARS * 2);
        let pad = Notepad::opened_with(&huge);

        assert_eq!(pad.text().chars().count(), MAX_CHARS);
        assert!(pad.text().chars().all(|c| c == 'é'));
    }
}
