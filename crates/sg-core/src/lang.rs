//! The string table: every text the game shows has a key, OpenTTD style (docs/TRANSLATING.md).
//!
//! `lang/en.txt` is built into the program and is the fallback for every key. A translation (`lang/<code>.txt`, from the
//! player's language setting or from a pack) is laid over it with [`load_overlay`]; a key it does not have shows the
//! English text, and a key nobody has shows the key itself.
//!
//! File format: UTF-8, one string per line, `STR_KEY  :text`. Lines starting with `#` are comments, blank lines are
//! ignored. The text is everything after the first colon, spaces included. Escapes: `\n` a line break, `\t` a tab, `\\` a
//! backslash, `\{` and `\}` literal braces. Parameters are names in braces (`{GOLFER}`, `{MONEY}`) or positions (`{0}`),
//! filled by [`trf`].
//!
//! [`tr`] hands out `&'static str`: the English table lives for the whole run, and overlay texts are kept for the rest of
//! the run too (a language is loaded a handful of times at most), so the code can use a key wherever it used a literal.

use std::collections::HashMap;
use std::fmt::{Display, Write};
use std::sync::{OnceLock, RwLock};

/// The English text, built in.
pub const EN: &str = include_str!("../../../lang/en.txt");

/// One parsed line: the key and the text with its escapes resolved.
pub fn parse(text: &str) -> Vec<(&str, String)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.strip_suffix('\r').unwrap_or(line);
        let t = line.trim_start();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let Some((key, val)) = t.split_once(':') else { continue };
        let key = key.trim_end();
        if key.is_empty() || key.contains(char::is_whitespace) {
            continue;
        }
        out.push((key, unescape(val)));
    }
    out
}

fn unescape(s: &str) -> String {
    if !s.contains('\\') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            // a brace that is not a parameter is kept as a private marker until formatting (see trf)
            Some('{') => out.push(LBRACE),
            Some('}') => out.push(RBRACE),
            Some(o) => out.push(o),
            None => out.push('\\'),
        }
    }
    out
}

// Private use characters stand in for escaped braces inside the table; tr and trf turn them back into braces.
const LBRACE: char = '\u{F8F0}';
const RBRACE: char = '\u{F8F1}';

struct Table {
    en: HashMap<&'static str, &'static str>,
    overlay: HashMap<&'static str, &'static str>,
    language: String,
}

fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

fn fix_braces(s: String) -> String {
    if s.contains([LBRACE, RBRACE]) {
        s.replace(LBRACE, "{").replace(RBRACE, "}")
    } else {
        s
    }
}

fn table() -> &'static RwLock<Table> {
    static T: OnceLock<RwLock<Table>> = OnceLock::new();
    T.get_or_init(|| {
        let en = parse(EN).into_iter().map(|(k, v)| (k, leak(v))).collect();
        RwLock::new(Table { en, overlay: HashMap::new(), language: "en".into() })
    })
}

/// The raw table text for a key (escaped braces still marked), or None.
fn raw(key: &str) -> Option<&'static str> {
    let t = table().read().unwrap_or_else(|e| e.into_inner());
    t.overlay.get(key).or_else(|| t.en.get(key)).copied()
}

/// The text of a key in the player's language: the overlay's, else the English, else the key itself.
pub fn tr(key: &'static str) -> &'static str {
    match raw(key) {
        Some(s) if s.contains([LBRACE, RBRACE]) => {
            // rare: cache the unmarked form so the result can stay 'static
            static FIXED: OnceLock<RwLock<HashMap<&'static str, &'static str>>> = OnceLock::new();
            let m = FIXED.get_or_init(Default::default);
            if let Some(v) = m.read().unwrap_or_else(|e| e.into_inner()).get(s) {
                return v;
            }
            let v = leak(fix_braces(s.to_string()));
            m.write().unwrap_or_else(|e| e.into_inner()).insert(s, v);
            v
        }
        Some(s) => s,
        None => key,
    }
}

/// The built-in English text of a key whatever the language (for comparing with text saved by an older version), or the
/// key itself.
pub fn en(key: &'static str) -> &'static str {
    let t = table().read().unwrap_or_else(|e| e.into_inner());
    t.en.get(key).copied().unwrap_or(key)
}

/// A name from a data table (docs/MODDING.md section 3), which holds either a text key (the built-in names, so they follow
/// the language) or a pack's own text (shown as it is).
pub fn text(s: &'static str) -> &'static str {
    if s.starts_with("STR_") {
        tr(s)
    } else {
        s
    }
}

/// True when the table (English or the overlay) has the key.
pub fn has(key: &str) -> bool {
    raw(key).is_some()
}

/// The text of a key with its parameters filled: `{NAME}` takes the argument named NAME, `{0}`, `{1}` the arguments by
/// position. A parameter with no argument is left as it is.
pub fn trf(key: &'static str, args: &[(&str, &dyn Display)]) -> String {
    fill(raw(key).unwrap_or(key), args)
}

/// Fills the parameters of a text (see [`trf`]).
pub fn fill(s: &str, args: &[(&str, &dyn Display)]) -> String {
    let mut out = String::with_capacity(s.len() + 16 * args.len());
    let mut rest = s;
    while let Some(i) = rest.find('{') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        let Some(j) = after.find('}') else {
            out.push_str(&rest[i..]);
            rest = "";
            break;
        };
        let name = &after[..j];
        let arg = match name.parse::<usize>() {
            Ok(n) => args.get(n).map(|a| a.1),
            Err(_) => args.iter().find(|a| a.0 == name).map(|a| a.1),
        };
        match arg {
            Some(a) => {
                let _ = write!(out, "{a}");
            }
            None => {
                out.push('{');
                out.push_str(name);
                out.push('}');
            }
        }
        rest = &after[j + 1..];
    }
    out.push_str(rest);
    fix_braces(out)
}

/// Picks the singular or the plural key by a count, where the game words a text differently for one.
pub fn plural(n: i64, one: &'static str, other: &'static str) -> &'static str {
    if n == 1 {
        one
    } else {
        other
    }
}

/// Lays a translation over the table: each key in `text` replaces the text the game shows for that key. Later overlays win
/// over earlier ones. Unknown keys are kept (a pack may carry text for a newer version). Returns how many strings it set.
pub fn load_overlay(text: &str) -> usize {
    let lines = parse(text);
    let n = lines.len();
    let mut t = table().write().unwrap_or_else(|e| e.into_inner());
    for (k, v) in lines {
        let key: &'static str = match t.en.get_key_value(k) {
            Some((k, _)) => k,
            None => leak(k.to_string()),
        };
        t.overlay.insert(key, leak(v));
    }
    n
}

/// Drops every overlay: English only again.
pub fn reset() {
    let mut t = table().write().unwrap_or_else(|e| e.into_inner());
    t.overlay.clear();
    t.language = "en".into();
}

/// Records the language code in use (the overlays are loaded separately with [`load_overlay`]).
pub fn set_language(code: &str) {
    table().write().unwrap_or_else(|e| e.into_inner()).language = code.to_string();
}

/// The language code in use ("en" by default).
pub fn language() -> String {
    table().read().unwrap_or_else(|e| e.into_inner()).language.clone()
}

/// The language a translation file names in its header line `##language <code>`, if any.
pub fn file_language(text: &str) -> Option<&str> {
    text.lines().find_map(|l| l.trim().strip_prefix("##language")).map(str::trim).filter(|c| !c.is_empty())
}

/// The name a translation file gives its language in its header line `##name <name>`, if any.
pub fn file_language_name(text: &str) -> Option<&str> {
    text.lines().find_map(|l| l.trim().strip_prefix("##name")).map(str::trim).filter(|c| !c.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_fill() {
        let p = parse("# c\n\nSTR_A   :Hello {NAME}!\nSTR_B:  two\\nlines \\{x\\}\nbad line\n");
        assert_eq!(p.len(), 2);
        assert_eq!(p[0], ("STR_A", "Hello {NAME}!".to_string()));
        assert_eq!(fill(&p[0].1, &[("NAME", &"Gary")]), "Hello Gary!");
        assert_eq!(fill(&p[1].1, &[]), "  two\nlines {x}");
        assert_eq!(fill("{0} and {1} {MISSING}", &[("A", &1), ("B", &"b")]), "1 and b {MISSING}");
    }

    #[test]
    fn missing_key_is_itself() {
        assert_eq!(tr("STR_NO_SUCH_KEY_ANYWHERE"), "STR_NO_SUCH_KEY_ANYWHERE");
        assert_eq!(trf("STR_NO_SUCH_KEY_ANYWHERE", &[]), "STR_NO_SUCH_KEY_ANYWHERE");
    }
}
