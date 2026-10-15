//! Portable names (META-6) and default ignores (IGN-2).
//!
//! A name is portable if every platform can store it: in practice, if Windows can, since it is
//! the strictest. Collisions are checked after Unicode normalization and case-folding, which is
//! how macOS and Windows compare names.

use unicode_normalization::UnicodeNormalization;

use crate::util::TEMP_PREFIX;

const WINDOWS_RESERVED: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Why a single path component is not portable, or `None` if it is.
pub fn component_problem(name: &str) -> Option<String> {
    if name.is_empty() || name == "." || name == ".." {
        return Some("empty or relative name".into());
    }
    if name.len() > 255 {
        return Some("longer than 255 bytes".into());
    }
    if let Some(c) = name
        .chars()
        .find(|c| matches!(c, '<' | '>' | ':' | '"' | '\\' | '|' | '?' | '*') || (*c as u32) < 32)
    {
        return Some(format!("contains {c:?}, which Windows does not allow"));
    }
    if name.ends_with(' ') || name.ends_with('.') {
        return Some("ends with a space or dot, which Windows does not allow".into());
    }
    let stem = name
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end()
        .to_ascii_uppercase();
    if WINDOWS_RESERVED.contains(&stem.as_str()) {
        return Some(format!("{stem} is a reserved name on Windows"));
    }
    None
}

/// The form in which two paths count as "the same name" on some platform.
pub fn collision_key(path: &str) -> String {
    path.nfc().collect::<String>().to_lowercase()
}

/// Built-in ignores for OS and application junk (IGN-2), plus the client's own temporary files.
pub fn ignored(name: &str) -> bool {
    const EXACT: &[&str] = &[
        ".DS_Store",
        "Thumbs.db",
        "desktop.ini",
        ".directory",
        "$RECYCLE.BIN",
    ];
    const PREFIX: &[&str] = &[TEMP_PREFIX, "~$", ".~lock.", "._"];
    const SUFFIX: &[&str] = &[".swp", ".swo", ".swx", "~", ".part", ".crdownload"];
    EXACT.contains(&name)
        || PREFIX.iter().any(|p| name.starts_with(p))
        || SUFFIX.iter().any(|s| name.ends_with(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portability() {
        assert!(component_problem("report.pdf").is_none());
        assert!(component_problem("café.txt").is_none());
        assert!(component_problem("Q3: results.xlsx").is_some());
        assert!(component_problem("aux.txt").is_some());
        assert!(component_problem("auxiliary.txt").is_none());
        assert!(component_problem("notes.").is_some());
        assert_eq!(collision_key("Report.PDF"), collision_key("report.pdf"));
        assert_eq!(collision_key("cafe\u{301}"), collision_key("caf\u{e9}"));
    }

    #[test]
    fn ignores() {
        assert!(ignored(".DS_Store"));
        assert!(ignored("~$budget.xlsx"));
        assert!(ignored(".~filesync-abc.tmp"));
        assert!(!ignored("budget.xlsx"));
    }
}
