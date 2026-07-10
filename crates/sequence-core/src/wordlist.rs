//! Custom word-list generator: comma-separated values that cycle when the
//! cursor count exceeds the list length.
//!
//! Example (from the PRD): `apple, banana, cherry` with 5 cursors ->
//! `apple`, `banana`, `cherry`, `apple`, `banana`.

use crate::error::SequenceError;
use crate::generator::Generator;

#[derive(Debug, Clone)]
pub struct WordListGenerator {
    words: Vec<String>,
}

impl WordListGenerator {
    pub fn new(raw: &str) -> Result<Self, SequenceError> {
        let words: Vec<String> = raw.split(',').map(|w| w.trim().to_string()).collect();
        if words.iter().any(|w| w.is_empty()) {
            return Err(SequenceError::InvalidSyntax(format!(
                "word list contains an empty entry: {raw}"
            )));
        }
        if words.is_empty() {
            return Err(SequenceError::InvalidSyntax("empty word list".to_string()));
        }
        Ok(Self { words })
    }
}

impl Generator for WordListGenerator {
    fn value_at(&self, index: usize) -> String {
        // Modulo wrap-around: seamlessly loops back to the start of the
        // list once the cursor count exceeds the number of words.
        self.words[index % self.words.len()].clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_word_list() {
        let gen = WordListGenerator::new("apple, banana, cherry").unwrap();
        let values: Vec<String> = (0..3).map(|i| gen.value_at(i)).collect();
        assert_eq!(values, vec!["apple", "banana", "cherry"]);
    }

    #[test]
    fn wraps_around_when_cursors_exceed_list_length() {
        let gen = WordListGenerator::new("apple, banana, cherry").unwrap();
        let values: Vec<String> = (0..5).map(|i| gen.value_at(i)).collect();
        assert_eq!(values, vec!["apple", "banana", "cherry", "apple", "banana"]);
    }

    #[test]
    fn rejects_empty_entries() {
        assert!(WordListGenerator::new("apple, , cherry").is_err());
    }
}
