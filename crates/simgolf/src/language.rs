//! The player's language (docs/TRANSLATING.md): the `language` setting in settings.json ("en" by default), the translation
//! files found for it, and the Preferences line that switches between them.
//!
//! A translation is a file `<code>.txt` in a `lang` folder: next to the program, in the folder the game was started from,
//! or in the save folder (later ones win over earlier ones). English is built in (`sg_core::lang`). Packs add their own
//! `lang/<code>.txt` on top through `sg_core::lang::load_overlay` after `apply`.
use crate::app::save_dir;
use sg_core::lang;
use std::path::PathBuf;

/// The settings file's key for the language code.
const KEY: &str = "language";

fn settings() -> serde_json::Value {
    sg_core::fsutil::read_file(save_dir().join(crate::hd::SETTINGS_FILE))
        .and_then(|d| serde_json::from_slice::<serde_json::Value>(&d).ok())
        .filter(|v| v.is_object())
        .unwrap_or_else(|| serde_json::json!({}))
}

/// The language code saved in the settings file ("en" when there is none).
pub fn saved() -> String {
    settings()[KEY].as_str().filter(|c| valid_code(c)).unwrap_or("en").to_string()
}

/// Saves the language code in the settings file, keeping the other settings.
pub fn save(code: &str) {
    let mut v = settings();
    v[KEY] = serde_json::Value::from(code);
    if let Ok(s) = serde_json::to_string_pretty(&v) {
        sg_core::fsutil::write_file(save_dir().join(crate::hd::SETTINGS_FILE), s.as_bytes());
    }
}

/// A language code is letters, digits, '-' and '_' ("de", "pt_BR"), so it can name a file safely.
fn valid_code(c: &str) -> bool {
    !c.is_empty() && c.len() <= 16 && c.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
}

/// The `lang` folders a translation is looked for in, the weakest first.
fn dirs() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(d) = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("lang"))) {
        v.push(d);
    }
    v.push(PathBuf::from("lang"));
    v.push(save_dir().join("lang"));
    v.dedup();
    v
}

/// The translation files of a language, the weakest first.
fn files(code: &str) -> Vec<PathBuf> {
    dirs().into_iter().map(|d| d.join(format!("{code}.txt"))).filter(|p| sg_core::fsutil::read_file(p).is_some()).collect()
}

/// Switches the text to a language: English, then every translation file found for the code. Returns false (and stays
/// English) when there is no file for a language other than English.
pub fn apply(code: &str) -> bool {
    lang::reset();
    if code == "en" || !valid_code(code) {
        return code == "en";
    }
    let found = files(code);
    for p in &found {
        if let Some(d) = sg_core::fsutil::read_file(p) {
            lang::load_overlay(&String::from_utf8_lossy(&d));
        }
    }
    if found.is_empty() {
        crate::bugreport::warn(&format!("language: no lang/{code}.txt found, staying with English"));
        return false;
    }
    lang::set_language(code);
    true
}

/// Applies a translation file given by path (the --lang-file test hook): English with that file over it.
pub fn apply_file(path: &std::path::Path) -> bool {
    let Some(d) = sg_core::fsutil::read_file(path) else {
        crate::bugreport::warn(&format!("language: cannot read {}", path.display()));
        return false;
    };
    let text = String::from_utf8_lossy(&d);
    lang::reset();
    let n = lang::load_overlay(&text);
    lang::set_language(lang::file_language(&text).unwrap_or("custom"));
    println!("language: {} strings from {}", n, path.display());
    true
}

/// The languages there are text for: English, then each translation file found, as (code, the name its header gives).
pub fn available() -> Vec<(String, String)> {
    let mut v = vec![("en".to_string(), lang::file_language_name(lang::EN).unwrap_or("English").to_string())];
    for d in dirs() {
        for p in sg_core::fsutil::list_dir(&d) {
            let Some(code) = p.file_stem().and_then(|s| s.to_str()).filter(|_| p.extension().is_some_and(|e| e == "txt")) else {
                continue;
            };
            if !valid_code(code) || v.iter().any(|(c, _)| c == code) {
                continue;
            }
            let text = sg_core::fsutil::read_file(&p).map(|d| String::from_utf8_lossy(&d).into_owned()).unwrap_or_default();
            let name = lang::file_language_name(&text).unwrap_or(code).to_string();
            v.push((code.to_string(), name));
        }
    }
    v
}

/// The name of the language in use, as the Preferences line shows it.
pub fn current_name() -> String {
    let code = lang::language();
    available().into_iter().find(|(c, _)| *c == code).map(|(_, n)| n).unwrap_or(code)
}

/// The Preferences line: the next language after the one in use, applied and saved.
pub fn cycle() {
    let all = available();
    let code = lang::language();
    let i = all.iter().position(|(c, _)| *c == code).map(|i| (i + 1) % all.len()).unwrap_or(0);
    let next = all[i].0.clone();
    if apply(&next) {
        save(&next);
    }
}
