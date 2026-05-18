// SPDX-License-Identifier: Apache-2.0
//! Minimal ANSI SGR -> ratatui converter.
//!
//! [`ansi_to_lines`] consumes a string that mixes plain text with `CSI ... m`
//! SGR escape sequences and produces a `Vec<Line<'static>>` ready to feed
//! into a [`ratatui::widgets::Paragraph`]. It is intentionally narrow:
//!
//! - Recognises the subset of SGR parameters emitted by
//!   [`aphrody_terminal_markdown::MarkdownRenderer::render_commonmark`]: reset (`0`), bold (`1`),
//!   dim (`2`), italic (`3`), underline (`4`), reverse (`7`), strike (`9`), the basic 8 + bright 8
//!   foreground/background colours, the `38;2;R;G;B` / `48;2;R;G;B` 24-bit colour forms emitted by
//!   `syntect` highlighting, and the `38;5;N` / `48;5;N` 256-colour forms.
//! - Ignores every other CSI sequence (cursor movement, OSC, etc.) — the markdown renderer does not
//!   produce them.
//!
//! The parser is allocation-aware (one `Span` per styled run, one `Line`
//! per `\n`) but never panics: malformed sequences are dropped on the floor
//! and the offending bytes are appended verbatim.

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// Convert an ANSI-styled `String` into ratatui lines.
///
/// Every `\n` in the input opens a fresh [`Line`]; trailing empty lines are
/// preserved so wrapping calculations stay accurate.
#[must_use]
pub fn ansi_to_lines(src: &str) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();
    let mut current_text = String::new();
    let mut style = Style::default();

    let bytes = src.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];

        if b == 0x1B && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
            // CSI start.  Find the final byte (0x40..=0x7E).
            let mut j = i + 2;
            while j < bytes.len() && !(0x40..=0x7E).contains(&bytes[j]) {
                j += 1;
            }
            if j >= bytes.len() {
                // Malformed: append the lone ESC and move on.
                current_text.push(char::from(b));
                i += 1;
                continue;
            }
            let final_byte = bytes[j];
            if final_byte == b'm' {
                // SGR. Flush text accumulated so far with the current style.
                flush_text(&mut current_spans, &mut current_text, style);
                // Parse params between [ and the trailing m.
                let params_slice = &src[i + 2..j];
                style = apply_sgr(params_slice, style);
            }
            // Any non-SGR CSI is silently dropped.
            i = j + 1;
            continue;
        }

        if b == b'\n' {
            flush_text(&mut current_spans, &mut current_text, style);
            lines.push(Line::from(std::mem::take(&mut current_spans)));
            i += 1;
            continue;
        }

        // Plain byte (ASCII or part of a UTF-8 sequence). Push the full
        // char to keep multi-byte sequences intact.
        let ch_start = i;
        // Determine UTF-8 width.
        let width = if b < 0x80 {
            1
        } else if b < 0xC0 {
            // Continuation byte without a leading byte: skip.
            i += 1;
            continue;
        } else if b < 0xE0 {
            2
        } else if b < 0xF0 {
            3
        } else {
            4
        };
        let end = (ch_start + width).min(bytes.len());
        // Safe slice: we computed `width` from the leading byte, and
        // truncate to len so we never overshoot.
        if let Ok(s) = std::str::from_utf8(&bytes[ch_start..end]) {
            current_text.push_str(s);
        }
        i = end;
    }

    flush_text(&mut current_spans, &mut current_text, style);
    if !current_spans.is_empty() {
        lines.push(Line::from(current_spans));
    }
    lines
}

fn flush_text(spans: &mut Vec<Span<'static>>, text: &mut String, style: Style) {
    if text.is_empty() {
        return;
    }
    spans.push(Span::styled(std::mem::take(text), style));
}

/// Apply a single `CSI ... m` parameter list to `style`.
///
/// Unknown / unsupported parameters are skipped, leaving the rest of the
/// list to be processed.  Per ECMA-48, an empty parameter list (`CSI m`)
/// is equivalent to `CSI 0 m` (reset).
fn apply_sgr(params: &str, mut style: Style) -> Style {
    let trimmed = params.trim();
    if trimmed.is_empty() {
        return Style::default();
    }

    let raw: Vec<u16> = trimmed.split(';').map(|p| p.trim().parse::<u16>().unwrap_or(0)).collect();

    let mut idx = 0;
    while idx < raw.len() {
        let p = raw[idx];
        match p {
            0 => style = Style::default(),
            1 => style = style.add_modifier(Modifier::BOLD),
            2 => style = style.add_modifier(Modifier::DIM),
            3 => style = style.add_modifier(Modifier::ITALIC),
            4 => style = style.add_modifier(Modifier::UNDERLINED),
            7 => style = style.add_modifier(Modifier::REVERSED),
            9 => style = style.add_modifier(Modifier::CROSSED_OUT),
            22 => style = style.remove_modifier(Modifier::BOLD | Modifier::DIM),
            23 => style = style.remove_modifier(Modifier::ITALIC),
            24 => style = style.remove_modifier(Modifier::UNDERLINED),
            27 => style = style.remove_modifier(Modifier::REVERSED),
            29 => style = style.remove_modifier(Modifier::CROSSED_OUT),
            30..=37 => style = style.fg(basic_color(p - 30, false)),
            39 => {
                let mut s = Style::default();
                s.bg = style.bg;
                s.add_modifier = style.add_modifier;
                s.sub_modifier = style.sub_modifier;
                style = s;
            },
            40..=47 => style = style.bg(basic_color(p - 40, false)),
            49 => {
                let mut s = Style::default();
                s.fg = style.fg;
                s.add_modifier = style.add_modifier;
                s.sub_modifier = style.sub_modifier;
                style = s;
            },
            90..=97 => style = style.fg(basic_color(p - 90, true)),
            100..=107 => style = style.bg(basic_color(p - 100, true)),
            38 | 48 => {
                // 38;5;N (indexed) or 38;2;R;G;B (truecolor).
                if idx + 1 >= raw.len() {
                    break;
                }
                let mode = raw[idx + 1];
                if mode == 5 && idx + 2 < raw.len() {
                    let n = raw[idx + 2].min(255) as u8;
                    let color = Color::Indexed(n);
                    style = if p == 38 { style.fg(color) } else { style.bg(color) };
                    idx += 3;
                    continue;
                }
                if mode == 2 && idx + 4 < raw.len() {
                    let r = raw[idx + 2].min(255) as u8;
                    let g = raw[idx + 3].min(255) as u8;
                    let b = raw[idx + 4].min(255) as u8;
                    let color = Color::Rgb(r, g, b);
                    style = if p == 38 { style.fg(color) } else { style.bg(color) };
                    idx += 5;
                    continue;
                }
                // Unknown extended form — bail to avoid misaligning.
                break;
            },
            _ => {},
        }
        idx += 1;
    }
    style
}

fn basic_color(n: u16, bright: bool) -> Color {
    match (n, bright) {
        (0, false) => Color::Black,
        (1, false) => Color::Red,
        (2, false) => Color::Green,
        (3, false) => Color::Yellow,
        (4, false) => Color::Blue,
        (5, false) => Color::Magenta,
        (6, false) => Color::Cyan,
        (7, false) => Color::Gray,
        (0, true) => Color::DarkGray,
        (1, true) => Color::LightRed,
        (2, true) => Color::LightGreen,
        (3, true) => Color::LightYellow,
        (4, true) => Color::LightBlue,
        (5, true) => Color::LightMagenta,
        (6, true) => Color::LightCyan,
        (7, true) => Color::White,
        _ => Color::Reset,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_one_line() {
        let lines = ansi_to_lines("hello world");
        assert_eq!(lines.len(), 1);
        let spans = &lines[0].spans;
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "hello world");
    }

    #[test]
    fn splits_on_newlines() {
        let lines = ansi_to_lines("a\nb\nc");
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn bold_run_captured_in_style() {
        // "plain \x1b[1mbold\x1b[0m tail"
        let lines = ansi_to_lines("plain \x1b[1mbold\x1b[0m tail");
        assert_eq!(lines.len(), 1);
        let spans = &lines[0].spans;
        // plain run, bold run, tail run
        assert!(
            spans
                .iter()
                .any(|s| { s.content == "bold" && s.style.add_modifier.contains(Modifier::BOLD) })
        );
        assert!(spans.iter().any(|s| s.content == " tail"));
    }

    #[test]
    fn truecolor_fg_parsed() {
        let lines = ansi_to_lines("\x1b[38;2;255;100;50mx\x1b[0m");
        assert_eq!(lines.len(), 1);
        let spans = &lines[0].spans;
        let styled = spans.iter().find(|s| s.content == "x").expect("x span present");
        assert_eq!(styled.style.fg, Some(Color::Rgb(255, 100, 50)));
    }

    #[test]
    fn reset_clears_modifiers() {
        let lines = ansi_to_lines("\x1b[1mA\x1b[0mB");
        let spans = &lines[0].spans;
        let a = spans.iter().find(|s| s.content == "A").unwrap();
        let b = spans.iter().find(|s| s.content == "B").unwrap();
        assert!(a.style.add_modifier.contains(Modifier::BOLD));
        assert!(!b.style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn unknown_csi_is_dropped() {
        // CSI 1A (cursor up) followed by text -- text must survive intact.
        let lines = ansi_to_lines("\x1b[1Ahello");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].spans[0].content, "hello");
    }
}
