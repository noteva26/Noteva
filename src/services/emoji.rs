//! Emoji support with Twemoji
//!
//! Converts emoji shortcodes (like :smile:) and Unicode emoji to Twemoji images.

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Twemoji CDN base URL
const TWEMOJI_BASE: &str = "https://cdn.jsdelivr.net/gh/twitter/twemoji@14.0.2/assets/svg";

/// Common emoji shortcode mappings
static EMOJI_MAP: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    // Smileys & Emotion
    m.insert("smile", "😄");
    m.insert("laughing", "😆");
    m.insert("blush", "😊");
    m.insert("smiley", "😃");
    m.insert("relaxed", "☺️");
    m.insert("smirk", "😏");
    m.insert("heart_eyes", "😍");
    m.insert("kissing_heart", "😘");
    m.insert("kissing_closed_eyes", "😚");
    m.insert("flushed", "😳");
    m.insert("relieved", "😌");
    m.insert("satisfied", "😆");
    m.insert("grin", "😁");
    m.insert("wink", "😉");
    m.insert("stuck_out_tongue_winking_eye", "😜");
    m.insert("stuck_out_tongue_closed_eyes", "😝");
    m.insert("grinning", "😀");
    m.insert("kissing", "😗");
    m.insert("kissing_smiling_eyes", "😙");
    m.insert("stuck_out_tongue", "😛");
    m.insert("sleeping", "😴");
    m.insert("worried", "😟");
    m.insert("frowning", "😦");
    m.insert("anguished", "😧");
    m.insert("open_mouth", "😮");
    m.insert("grimacing", "😬");
    m.insert("confused", "😕");
    m.insert("hushed", "😯");
    m.insert("expressionless", "😑");
    m.insert("unamused", "😒");
    m.insert("sweat_smile", "😅");
    m.insert("sweat", "😓");
    m.insert("disappointed_relieved", "😥");
    m.insert("weary", "😩");
    m.insert("pensive", "😔");
    m.insert("disappointed", "😞");
    m.insert("confounded", "😖");
    m.insert("fearful", "😨");
    m.insert("cold_sweat", "😰");
    m.insert("persevere", "😣");
    m.insert("cry", "😢");
    m.insert("sob", "😭");
    m.insert("joy", "😂");
    m.insert("astonished", "😲");
    m.insert("scream", "😱");
    m.insert("tired_face", "😫");
    m.insert("angry", "😠");
    m.insert("rage", "😡");
    m.insert("triumph", "😤");
    m.insert("sleepy", "😪");
    m.insert("yum", "😋");
    m.insert("mask", "😷");
    m.insert("sunglasses", "😎");
    m.insert("dizzy_face", "😵");
    m.insert("imp", "👿");
    m.insert("smiling_imp", "😈");
    m.insert("neutral_face", "😐");
    m.insert("no_mouth", "😶");
    m.insert("innocent", "😇");
    m.insert("alien", "👽");
    // Hearts & Love
    m.insert("heart", "❤️");
    m.insert("yellow_heart", "💛");
    m.insert("green_heart", "💚");
    m.insert("blue_heart", "💙");
    m.insert("purple_heart", "💜");
    m.insert("broken_heart", "💔");
    m.insert("heartpulse", "💗");
    m.insert("heartbeat", "💓");
    m.insert("two_hearts", "💕");
    m.insert("sparkling_heart", "💖");
    m.insert("revolving_hearts", "💞");
    m.insert("cupid", "💘");
    m.insert("gift_heart", "💝");

    // Gestures
    m.insert("thumbsup", "👍");
    m.insert("+1", "👍");
    m.insert("thumbsdown", "👎");
    m.insert("-1", "👎");
    m.insert("ok_hand", "👌");
    m.insert("punch", "👊");
    m.insert("fist", "✊");
    m.insert("v", "✌️");
    m.insert("wave", "👋");
    m.insert("hand", "✋");
    m.insert("open_hands", "👐");
    m.insert("point_up", "☝️");
    m.insert("point_down", "👇");
    m.insert("point_left", "👈");
    m.insert("point_right", "👉");
    m.insert("raised_hands", "🙌");
    m.insert("pray", "🙏");
    m.insert("clap", "👏");
    m.insert("muscle", "💪");

    // Objects & Symbols
    m.insert("fire", "🔥");
    m.insert("star", "⭐");
    m.insert("sparkles", "✨");
    m.insert("zap", "⚡");
    m.insert("sunny", "☀️");
    m.insert("cloud", "☁️");
    m.insert("snowflake", "❄️");
    m.insert("umbrella", "☔");
    m.insert("coffee", "☕");
    m.insert("beer", "🍺");
    m.insert("cake", "🎂");
    m.insert("gift", "🎁");
    m.insert("bell", "🔔");
    m.insert("tada", "🎉");
    m.insert("balloon", "🎈");
    m.insert("rocket", "🚀");
    m.insert("airplane", "✈️");
    m.insert("car", "🚗");
    m.insert("bike", "🚲");
    m.insert("warning", "⚠️");
    m.insert("x", "❌");
    m.insert("white_check_mark", "✅");
    m.insert("question", "❓");
    m.insert("exclamation", "❗");
    m.insert("100", "💯");
    m.insert("bulb", "💡");
    m.insert("memo", "📝");
    m.insert("book", "📖");
    m.insert("link", "🔗");
    m.insert("email", "📧");
    m.insert("phone", "📱");
    m.insert("computer", "💻");
    m.insert("camera", "📷");
    m.insert("video_camera", "📹");
    m.insert("tv", "📺");
    m.insert("sound", "🔊");
    m.insert("mute", "🔇");
    m.insert("lock", "🔒");
    m.insert("unlock", "🔓");
    m.insert("key", "🔑");
    m.insert("mag", "🔍");
    m.insert("eyes", "👀");
    m.insert("eye", "👁️");
    m.insert("speech_balloon", "💬");
    m.insert("thought_balloon", "💭");

    // Animals
    m.insert("dog", "🐶");
    m.insert("cat", "🐱");
    m.insert("mouse", "🐭");
    m.insert("rabbit", "🐰");
    m.insert("bear", "🐻");
    m.insert("panda_face", "🐼");
    m.insert("pig", "🐷");
    m.insert("frog", "🐸");
    m.insert("monkey_face", "🐵");
    m.insert("chicken", "🐔");
    m.insert("penguin", "🐧");
    m.insert("bird", "🐦");
    m.insert("fish", "🐟");
    m.insert("whale", "🐳");
    m.insert("bug", "🐛");
    m.insert("bee", "🐝");
    m.insert("turtle", "🐢");
    m.insert("snake", "🐍");
    m.insert("dragon", "🐉");

    m
});

/// Convert emoji shortcode to Unicode emoji
pub fn shortcode_to_emoji(shortcode: &str) -> Option<&'static str> {
    EMOJI_MAP.get(shortcode).copied()
}

/// Convert Unicode emoji to Twemoji image URL
pub fn emoji_to_twemoji_url(emoji: &str) -> String {
    let codepoints: Vec<String> = emoji
        .chars()
        .filter(|c| *c != '\u{FE0F}') // Remove variation selector
        .map(|c| format!("{:x}", c as u32))
        .collect();

    format!("{}/{}.svg", TWEMOJI_BASE, codepoints.join("-"))
}

/// Convert Unicode emoji to Twemoji img tag
pub fn emoji_to_twemoji_img(emoji: &str) -> String {
    let url = emoji_to_twemoji_url(emoji);
    format!(
        r#"<img class="twemoji" draggable="false" alt="{}" src="{}">"#,
        emoji, url
    )
}

/// Process text and convert emoji shortcodes to Twemoji images
/// Converts :shortcode: syntax to Twemoji img tags
pub fn process_shortcodes(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if c == ':' {
            // Try to parse shortcode
            let mut shortcode = String::new();
            let mut found_end = false;

            // Collect characters until we find another ':'
            while let Some(&next) = chars.peek() {
                if next == ':' {
                    chars.next(); // consume the ':'
                    found_end = true;
                    break;
                } else if next.is_alphanumeric() || next == '_' || next == '+' || next == '-' {
                    shortcode.push(next);
                    chars.next();
                } else {
                    break;
                }
            }

            if found_end && !shortcode.is_empty() {
                // Try to convert shortcode to emoji
                if let Some(emoji) = shortcode_to_emoji(&shortcode) {
                    result.push_str(&emoji_to_twemoji_img(emoji));
                } else {
                    // Not a valid shortcode, output as-is
                    result.push(':');
                    result.push_str(&shortcode);
                    result.push(':');
                }
            } else {
                // Not a valid shortcode format
                result.push(':');
                result.push_str(&shortcode);
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// Process HTML and convert Unicode emoji to Twemoji images
/// This should be called after markdown rendering
pub fn process_unicode_emoji(html: &str) -> String {
    let mut result = String::with_capacity(html.len() * 2);
    let mut chars = html.chars().peekable();
    let mut in_tag = false;
    let mut in_code = false;
    let mut tag_name = String::new();

    while let Some(c) = chars.next() {
        // Track HTML tags to avoid processing emoji inside tags
        if c == '<' {
            in_tag = true;
            tag_name.clear();
            result.push(c);
            continue;
        }

        if in_tag {
            if c == '>' {
                in_tag = false;
                // Check if entering/exiting code block
                let tag_lower = tag_name.to_lowercase();
                if tag_lower == "code" || tag_lower == "pre" {
                    in_code = true;
                } else if tag_lower == "/code" || tag_lower == "/pre" {
                    in_code = false;
                }
            } else if c != '/' && !c.is_whitespace() {
                tag_name.push(c);
            }
            result.push(c);
            continue;
        }

        // Don't process emoji inside code blocks
        if in_code {
            result.push(c);
            continue;
        }

        // Check if this is an emoji
        if is_emoji_start(c) {
            let mut emoji = String::new();
            emoji.push(c);

            // Collect potential emoji sequence (including ZWJ sequences)
            while let Some(&next) = chars.peek() {
                if is_emoji_continuation(next) {
                    emoji.push(next);
                    chars.next();
                } else {
                    break;
                }
            }

            // Convert to Twemoji if it's a valid emoji
            if emoji.chars().count() >= 1 && is_likely_emoji(&emoji) {
                result.push_str(&emoji_to_twemoji_img(&emoji));
            } else {
                result.push_str(&emoji);
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// Check if a character could start an emoji
fn is_emoji_start(c: char) -> bool {
    let code = c as u32;
    // Common emoji ranges
    (0x1F300..=0x1F9FF).contains(&code) ||  // Misc Symbols, Emoticons, etc.
    (0x2600..=0x26FF).contains(&code) ||    // Misc Symbols
    (0x2700..=0x27BF).contains(&code) ||    // Dingbats
    (0x231A..=0x231B).contains(&code) ||    // Watch, Hourglass
    (0x23E9..=0x23F3).contains(&code) ||    // Media controls
    (0x23F8..=0x23FA).contains(&code) ||    // Media controls
    (0x25AA..=0x25AB).contains(&code) ||    // Squares
    (0x25B6..=0x25C0).contains(&code) ||    // Triangles
    (0x25FB..=0x25FE).contains(&code) ||    // Squares
    (0x2614..=0x2615).contains(&code) ||    // Umbrella, Hot beverage
    (0x2648..=0x2653).contains(&code) ||    // Zodiac
    (0x267F..=0x267F).contains(&code) ||    // Wheelchair
    (0x2693..=0x2693).contains(&code) ||    // Anchor
    (0x26A1..=0x26A1).contains(&code) ||    // High voltage
    (0x26AA..=0x26AB).contains(&code) ||    // Circles
    (0x26BD..=0x26BE).contains(&code) ||    // Sports
    (0x26C4..=0x26C5).contains(&code) ||    // Weather
    (0x26CE..=0x26CE).contains(&code) ||    // Ophiuchus
    (0x26D4..=0x26D4).contains(&code) ||    // No entry
    (0x26EA..=0x26EA).contains(&code) ||    // Church
    (0x26F2..=0x26F3).contains(&code) ||    // Fountain, Golf
    (0x26F5..=0x26F5).contains(&code) ||    // Sailboat
    (0x26FA..=0x26FA).contains(&code) ||    // Tent
    (0x26FD..=0x26FD).contains(&code) ||    // Fuel pump
    (0x2702..=0x2702).contains(&code) ||    // Scissors
    (0x2705..=0x2705).contains(&code) ||    // Check mark
    (0x2708..=0x270D).contains(&code) ||    // Airplane to Writing hand
    (0x270F..=0x270F).contains(&code) ||    // Pencil
    (0x2712..=0x2712).contains(&code) ||    // Black nib
    (0x2714..=0x2714).contains(&code) ||    // Check mark
    (0x2716..=0x2716).contains(&code) ||    // X mark
    (0x271D..=0x271D).contains(&code) ||    // Latin cross
    (0x2721..=0x2721).contains(&code) ||    // Star of David
    (0x2728..=0x2728).contains(&code) ||    // Sparkles
    (0x2733..=0x2734).contains(&code) ||    // Eight spoked asterisk
    (0x2744..=0x2744).contains(&code) ||    // Snowflake
    (0x2747..=0x2747).contains(&code) ||    // Sparkle
    (0x274C..=0x274C).contains(&code) ||    // Cross mark
    (0x274E..=0x274E).contains(&code) ||    // Cross mark
    (0x2753..=0x2755).contains(&code) ||    // Question marks
    (0x2757..=0x2757).contains(&code) ||    // Exclamation
    (0x2763..=0x2764).contains(&code) ||    // Heart exclamation, Heart
    (0x2795..=0x2797).contains(&code) ||    // Math symbols
    (0x27A1..=0x27A1).contains(&code) ||    // Right arrow
    (0x27B0..=0x27B0).contains(&code) ||    // Curly loop
    (0x27BF..=0x27BF).contains(&code) ||    // Double curly loop
    (0x2934..=0x2935).contains(&code) ||    // Arrows
    (0x2B05..=0x2B07).contains(&code) ||    // Arrows
    (0x2B1B..=0x2B1C).contains(&code) ||    // Squares
    (0x2B50..=0x2B50).contains(&code) ||    // Star
    (0x2B55..=0x2B55).contains(&code) ||    // Circle
    (0x3030..=0x3030).contains(&code) ||    // Wavy dash
    (0x303D..=0x303D).contains(&code) ||    // Part alternation mark
    (0x3297..=0x3297).contains(&code) ||    // Circled Ideograph Congratulation
    (0x3299..=0x3299).contains(&code) // Circled Ideograph Secret
}

/// Check if a character could continue an emoji sequence
fn is_emoji_continuation(c: char) -> bool {
    let code = c as u32;
    is_emoji_start(c) ||
    code == 0xFE0F ||  // Variation selector
    code == 0x200D ||  // Zero-width joiner
    (0x1F3FB..=0x1F3FF).contains(&code) // Skin tone modifiers
}

/// Check if a string is likely a valid emoji
fn is_likely_emoji(s: &str) -> bool {
    let first = s.chars().next();
    first.map(is_emoji_start).unwrap_or(false)
}

/// Process both shortcodes and Unicode emoji in HTML
pub fn process_all_emoji(html: &str) -> String {
    let with_shortcodes = process_shortcodes(html);
    process_unicode_emoji(&with_shortcodes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shortcode_to_emoji() {
        assert_eq!(shortcode_to_emoji("smile"), Some("😄"));
        assert_eq!(shortcode_to_emoji("heart"), Some("❤️"));
        assert_eq!(shortcode_to_emoji("thumbsup"), Some("👍"));
        assert_eq!(shortcode_to_emoji("+1"), Some("👍"));
        assert_eq!(shortcode_to_emoji("nonexistent"), None);
    }

    #[test]
    fn test_emoji_to_twemoji_url() {
        let url = emoji_to_twemoji_url("😄");
        assert!(url.contains("1f604"));
        assert!(url.ends_with(".svg"));
    }

    #[test]
    fn test_process_shortcodes() {
        let result = process_shortcodes("Hello :smile: world");
        assert!(result.contains("twemoji"));
        assert!(result.contains("1f604"));
        assert!(!result.contains(":smile:"));
    }

    #[test]
    fn test_process_shortcodes_invalid() {
        let result = process_shortcodes("Hello :invalid_emoji: world");
        assert!(result.contains(":invalid_emoji:"));
    }

    #[test]
    fn test_process_unicode_emoji() {
        let result = process_unicode_emoji("<p>Hello 😄 world</p>");
        assert!(result.contains("twemoji"));
        assert!(result.contains("<p>Hello"));
        assert!(result.contains("world</p>"));
    }

    #[test]
    fn test_process_unicode_emoji_in_code() {
        let result = process_unicode_emoji("<code>😄</code>");
        // Should not convert emoji inside code tags
        assert!(result.contains("😄"));
        assert!(!result.contains("twemoji"));
    }
}
