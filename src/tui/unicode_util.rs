use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Truncates a string to fit within `max_width` terminal columns safely without
/// ever splitting multi-byte UTF-8 code points or wide characters.
pub fn truncate_to_width(s: &str, max_width: usize) -> &str {
    let mut current_width = 0;
    let mut last_byte_idx = 0;

    for (idx, ch) in s.char_indices() {
        let ch_width = ch.width().unwrap_or(0);
        if current_width + ch_width > max_width {
            return &s[..last_byte_idx];
        }
        current_width += ch_width;
        last_byte_idx = idx + ch.len_utf8();
    }

    s
}

/// Truncates string to at most `target_width` terminal columns, and pads with
/// spaces if shorter than `target_width`.
pub fn pad_or_truncate(s: &str, target_width: usize) -> String {
    let truncated = truncate_to_width(s, target_width);
    let visual_width = UnicodeWidthStr::width(truncated);
    if visual_width < target_width {
        let padding = " ".repeat(target_width - visual_width);
        format!("{}{}", truncated, padding)
    } else {
        truncated.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unicode_safe_truncation() {
        // Japanese: '日本語' has 3 characters, each 2 columns wide = 6 columns total
        let s = "日本語";
        assert_eq!(truncate_to_width(s, 0), "");
        assert_eq!(truncate_to_width(s, 1), ""); // 1 column cannot fit 2-column character
        assert_eq!(truncate_to_width(s, 2), "日");
        assert_eq!(truncate_to_width(s, 3), "日");
        assert_eq!(truncate_to_width(s, 4), "日本");
        assert_eq!(truncate_to_width(s, 6), "日本語");
        assert_eq!(truncate_to_width(s, 10), "日本語");

        // Emoji: '🚀' is 2 columns wide
        let emoji_str = "🚀rocket";
        assert_eq!(truncate_to_width(emoji_str, 1), "");
        assert_eq!(truncate_to_width(emoji_str, 2), "🚀");
        assert_eq!(truncate_to_width(emoji_str, 3), "🚀r");

        // Accented
        let accent_str = "café";
        assert_eq!(truncate_to_width(accent_str, 3), "caf");
        assert_eq!(truncate_to_width(accent_str, 4), "café");
    }

    #[test]
    fn test_pad_or_truncate() {
        let s = "日"; // 2 columns wide
        assert_eq!(pad_or_truncate(s, 5), "日   "); // 2 + 3 = 5 columns
        assert_eq!(pad_or_truncate(s, 2), "日");
        assert_eq!(pad_or_truncate(s, 1), " ");
    }
}
