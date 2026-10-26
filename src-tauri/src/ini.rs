//! BWDecomp.ini, read as GetPrivateProfileString reads it (names compared without case, no inline comments) and
//! changed line by line so the comments that document every setting stay in place.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IniValue {
    pub section: String,
    pub key: String,
    pub value: String,
}

fn section_name(line: &str) -> Option<&str> {
    let t = line.trim();
    t.strip_prefix('[').and_then(|s| s.strip_suffix(']')).map(str::trim)
}

fn key_value(line: &str) -> Option<(&str, &str)> {
    let t = line.trim_start();
    if t.starts_with(';') || t.starts_with('#') || t.starts_with('[') {
        return None;
    }
    let (key, value) = t.split_once('=')?;
    let key = key.trim();
    (!key.is_empty()).then(|| (key, value.trim()))
}

/// Every value, in file order. A key repeated in a section counts once, first wins (as Windows reads it).
pub fn parse(text: &str) -> Vec<IniValue> {
    let mut out: Vec<IniValue> = Vec::new();
    let mut section = String::new();
    for line in text.lines() {
        if let Some(name) = section_name(line) {
            section = name.to_string();
        } else if let Some((key, value)) = key_value(line) {
            let seen = out
                .iter()
                .any(|v| v.section.eq_ignore_ascii_case(&section) && v.key.eq_ignore_ascii_case(key));
            if !seen {
                out.push(IniValue { section: section.clone(), key: key.to_string(), value: value.to_string() });
            }
        }
    }
    out
}

/// `text` with `key` in `section` set to `value`: the existing line is rewritten, a missing key is added at the end
/// of its section, a missing section at the end of the file.
pub fn set(text: &str, section: &str, key: &str, value: &str) -> String {
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let mut in_section = false;
    let mut section_end: Option<usize> = None; // index after the last non-blank line of the section
    for i in 0..lines.len() {
        if let Some(name) = section_name(&lines[i]) {
            if in_section {
                break;
            }
            in_section = name.eq_ignore_ascii_case(section);
            if in_section {
                section_end = Some(i + 1);
            }
            continue;
        }
        if !in_section {
            continue;
        }
        if let Some((existing, _)) = key_value(&lines[i]) {
            if existing.eq_ignore_ascii_case(key) {
                let indent: String = lines[i].chars().take_while(|c| c.is_whitespace()).collect();
                lines[i] = format!("{indent}{existing}={value}");
                return join(&lines, newline);
            }
        }
        if !lines[i].trim().is_empty() {
            section_end = Some(i + 1);
        }
    }
    match section_end {
        Some(at) => lines.insert(at, format!("{key}={value}")),
        None => {
            if lines.last().is_some_and(|l| !l.trim().is_empty()) {
                lines.push(String::new());
            }
            lines.push(format!("[{section}]"));
            lines.push(format!("{key}={value}"));
        }
    }
    join(&lines, newline)
}

fn join(lines: &[String], newline: &str) -> String {
    let mut out = lines.join(newline);
    out.push_str(newline);
    out
}

pub fn get<'a>(values: &'a [IniValue], section: &str, key: &str) -> Option<&'a str> {
    values
        .iter()
        .find(|v| v.section.eq_ignore_ascii_case(section) && v.key.eq_ignore_ascii_case(key))
        .map(|v| v.value.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "; header\r\n\r\n[Fixes]\r\n; about A\r\nA=1\r\n\r\n[Display]\r\n; mode\r\nWindowMode=0\r\n\r\n[Log]\r\nDebug=0\r\n";

    #[test]
    fn parses_values() {
        let values = parse(SAMPLE);
        assert_eq!(values.len(), 3);
        assert_eq!(get(&values, "display", "windowmode"), Some("0"));
    }

    #[test]
    fn rewrites_in_place_and_keeps_comments() {
        let out = set(SAMPLE, "Display", "windowmode", "2");
        assert_eq!(out, SAMPLE.replace("WindowMode=0", "WindowMode=2"));
    }

    #[test]
    fn adds_missing_keys_and_sections() {
        let out = set(SAMPLE, "Display", "VSync", "1");
        assert!(out.contains("WindowMode=0\r\nVSync=1\r\n\r\n[Log]"));
        let out = set(&out, "Textures", "Dump", "1");
        assert!(out.ends_with("Debug=0\r\n\r\n[Textures]\r\nDump=1\r\n"));
        assert_eq!(get(&parse(&out), "Textures", "Dump"), Some("1"));
    }
}
