use agymux::core::transcript::TranscriptParser;
use agymux::sanitize_title;
use proptest::prelude::*;
use std::io::Cursor;

proptest! {
    #[test]
    fn test_fuzz_sanitize_title(s in ".*") {
        let sanitized = sanitize_title(&s);
        prop_assert!(sanitized.chars().count() <= 20);
        prop_assert!(!sanitized.is_empty());
        for c in sanitized.chars() {
            prop_assert!(c.is_alphanumeric() || c == ' ' || c == '_' || c == '-' || c == '.');
            prop_assert!(!c.is_control());
        }
    }

    #[test]
    fn test_fuzz_clean_user_prompt(s in ".*") {
        let cleaned = TranscriptParser::clean_user_prompt(&s);
        // Never panics on arbitrary string input
        prop_assert!(!cleaned.contains("<USER_REQUEST>"));
        prop_assert!(!cleaned.contains("</USER_REQUEST>"));
        prop_assert!(!cleaned.contains("<ADDITIONAL_METADATA>"));
        prop_assert!(!cleaned.contains("</ADDITIONAL_METADATA>"));
        prop_assert!(!cleaned.contains("<USER_SETTINGS_CHANGE>"));
        prop_assert!(!cleaned.contains("</USER_SETTINGS_CHANGE>"));
    }

    #[test]
    fn test_fuzz_transcript_parse_reader(s in ".*") {
        let cursor = Cursor::new(s);
        // Parser must never panic on arbitrary input stream
        let res = TranscriptParser::parse_reader(cursor);
        prop_assert!(res.is_ok());
    }
}
