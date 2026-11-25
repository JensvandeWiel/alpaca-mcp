//! Turns Woodpecker log entries into compact text.

use base64::{Engine, engine::general_purpose::STANDARD};

use crate::model::LogEntry;

/// Upper bound of lines per tool call.
pub const MAX_LINES: usize = 2000;
const MAX_LINE_CHARS: usize = 1000;
const MAX_TEXT_BYTES: usize = 60_000;

const STDOUT: i64 = 0;
const STDERR: i64 = 1;

#[derive(Debug, PartialEq)]
pub struct Line {
    /// 1-based position in the step's output.
    pub number: i64,
    pub text: String,
}

/// Decodes the output lines; exit codes, metadata and progress are left out.
pub fn decode(mut entries: Vec<LogEntry>) -> Vec<Line> {
    entries.retain(|entry| matches!(entry.kind, STDOUT | STDERR));
    entries.sort_by_key(|entry| entry.line);
    entries
        .into_iter()
        .map(|entry| Line {
            number: entry.line + 1,
            text: clean(&decode_base64(entry.data.as_deref())),
        })
        .collect()
}

pub fn decode_base64(data: Option<&str>) -> String {
    match data {
        None => String::new(),
        Some(data) => match STANDARD.decode(data) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            Err(_) => "<undecodable data>".to_owned(),
        },
    }
}

/// Removes terminal escape sequences and control characters, keeps only the
/// last state of a line redrawn with `\r` and truncates very long lines.
fn clean(raw: &str) -> String {
    let raw = raw.trim_end_matches(['\r', '\n']);
    let raw = raw.rsplit('\r').next().unwrap_or_default();

    let mut out = String::new();
    let mut length = 0;
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            match chars.next() {
                // CSI: parameters, then one final byte.
                Some('[') => {
                    for c in chars.by_ref() {
                        if ('\x40'..='\x7e').contains(&c) {
                            break;
                        }
                    }
                }
                // OSC: ends with BEL or ESC \.
                Some(']') => {
                    while let Some(c) = chars.next() {
                        if c == '\x07' || (c == '\x1b' && chars.next_if_eq(&'\\').is_some()) {
                            break;
                        }
                    }
                }
                _ => {}
            }
        } else if c == '\t' || !c.is_control() {
            if length == MAX_LINE_CHARS {
                out.push_str(" [line truncated]");
                break;
            }
            out.push(c);
            length += 1;
        }
    }
    out
}

/// Renders the last `count` lines that match `search` (case-insensitive) and
/// come at or before line `end_line`, with a header saying what was left out.
pub fn render(lines: &[Line], count: usize, search: Option<&str>, end_line: Option<i64>) -> String {
    let needle = search.map(str::to_lowercase);
    let matching: Vec<&Line> = lines
        .iter()
        .filter(|line| end_line.is_none_or(|end| line.number <= end))
        .filter(|line| {
            needle
                .as_ref()
                .is_none_or(|needle| line.text.to_lowercase().contains(needle))
        })
        .collect();

    // Fill from the end: the last lines usually explain a failure.
    let mut shown = Vec::new();
    let mut bytes = 0;
    for line in matching.iter().rev().take(count.clamp(1, MAX_LINES)) {
        let rendered = format!("{}: {}", line.number, line.text);
        bytes += rendered.len() + 1;
        if bytes > MAX_TEXT_BYTES && !shown.is_empty() {
            break;
        }
        shown.push(rendered);
    }
    shown.reverse();

    let what = if search.is_some() {
        "matching lines"
    } else {
        "lines"
    };
    let mut header = format!(
        "[{} of {} {what}; the step logged {} lines]",
        shown.len(),
        matching.len(),
        lines.len()
    );
    if shown.len() < matching.len() {
        let first = matching[matching.len() - shown.len()].number;
        header.push_str(&format!(" [for earlier ones use end_line={}]", first - 1));
    }
    if shown.is_empty() {
        header
    } else {
        format!("{header}\n{}", shown.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(line: i64, kind: i64, text: &str) -> LogEntry {
        LogEntry {
            line,
            kind,
            data: Some(STANDARD.encode(text)),
        }
    }

    #[test]
    fn decodes_in_order_and_skips_non_output() {
        let lines = decode(vec![
            entry(1, STDERR, "second"),
            entry(2, 2, "0"),
            entry(0, STDOUT, "first"),
            LogEntry {
                line: 3,
                kind: STDOUT,
                data: None,
            },
        ]);
        let text: Vec<(i64, &str)> = lines.iter().map(|l| (l.number, l.text.as_str())).collect();
        assert_eq!(text, [(1, "first"), (2, "second"), (4, "")]);
    }

    #[test]
    fn removes_escape_sequences_and_redraws() {
        assert_eq!(clean("\x1b[1;31merror\x1b[0m: bad"), "error: bad");
        assert_eq!(clean("\x1b]0;title\x07text"), "text");
        assert_eq!(clean("\x1b]8;;http://x\x1b\\link"), "link");
        assert_eq!(clean("10%\r50%\r100%\r\n"), "100%");
        assert_eq!(clean("a\x08b\tc"), "ab\tc");
        assert!(clean(&"é".repeat(5000)).ends_with(" [line truncated]"));
    }

    #[test]
    fn renders_tail_search_and_paging() {
        let lines: Vec<Line> = (1..=10)
            .map(|number| Line {
                number,
                text: if number % 2 == 0 { "Even" } else { "odd" }.to_owned(),
            })
            .collect();

        let tail = render(&lines, 2, None, None);
        assert!(tail.contains("[2 of 10 lines"), "{tail}");
        assert!(tail.contains("end_line=8"), "{tail}");
        assert!(tail.ends_with("9: odd\n10: Even"), "{tail}");

        let search = render(&lines, 2, Some("even"), Some(7));
        assert!(search.ends_with("4: Even\n6: Even"), "{search}");
    }

    #[test]
    fn bounds_the_total_size() {
        let lines: Vec<Line> = (1..=500)
            .map(|number| Line {
                number,
                text: "x".repeat(900),
            })
            .collect();
        let text = render(&lines, MAX_LINES, None, None);
        assert!(text.len() < MAX_TEXT_BYTES + 200);
        assert!(text.contains("500: x"));
    }
}
