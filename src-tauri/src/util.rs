//! Small shared helpers.

use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

/// RFC 3339 UTC timestamp. The UI formats it for display with `Intl`.
pub fn now_iso() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| String::from("1970-01-01T00:00:00Z"))
}

/// Set `FLOWMACRO_DEBUG_INPUT=1` to trace what the global hook sees and what
/// the hotkeys do with it. Off by default and read once: this sits on the
/// input path, which must stay cheap.
pub fn input_debug() -> bool {
    use std::sync::OnceLock;
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("FLOWMACRO_DEBUG_INPUT").is_some())
}

const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";

/// Short collision-resistant id. Not a UUID: these end up in JSON the user may
/// read and edit by hand, so brevity matters more than formal uniqueness.
pub fn new_id(prefix: &str) -> String {
    let mut id = String::with_capacity(prefix.len() + 13);
    id.push_str(prefix);
    id.push('_');
    for _ in 0..12 {
        let index = fastrand::usize(..ALPHABET.len());
        id.push(ALPHABET[index] as char);
    }
    id
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamps_are_rfc3339() {
        let stamp = now_iso();
        assert!(stamp.contains('T'), "not a timestamp: {stamp}");
        assert!(OffsetDateTime::parse(&stamp, &Rfc3339).is_ok());
    }

    #[test]
    fn ids_are_prefixed_and_unique_enough() {
        let a = new_id("macro");
        let b = new_id("macro");
        assert!(a.starts_with("macro_"));
        assert_eq!(a.len(), "macro_".len() + 12);
        assert_ne!(a, b);
    }
}
