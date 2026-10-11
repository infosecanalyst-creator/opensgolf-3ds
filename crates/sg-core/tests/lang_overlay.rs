//! A translation laid over the built-in English (its own test binary: the string table is global).
use sg_core::lang::{self, tr, trf};

#[test]
fn overlay_then_english_then_key() {
    let english = tr("STR_MSG_CHALLENGE");
    assert_eq!(english, "{PRO} challenges {GOLFER} to a match!");
    let n = lang::load_overlay(
        "##language xx\n##name Test\n# a comment\nSTR_MSG_CHALLENGE  :{GOLFER} wird von {PRO} herausgefordert!\nSTR_NEW_IN_A_PACK :neu\n",
    );
    assert_eq!(n, 2);
    lang::set_language("xx");
    assert_eq!(lang::language(), "xx");
    // the translation's text, with its parameters in its own order
    assert_eq!(trf("STR_MSG_CHALLENGE", &[("PRO", &"Arnie"), ("GOLFER", &"Gary")]), "Gary wird von Arnie herausgefordert!");
    // a key the translation does not have: English; a key nobody has: the key
    assert_eq!(tr("STR_MSG_MATCH_CANCELED"), "Match canceled.");
    assert_eq!(tr("STR_NEW_IN_A_PACK"), "neu");
    assert_eq!(tr("STR_NOBODY_HAS_THIS"), "STR_NOBODY_HAS_THIS");
    // the English text stays at hand
    assert_eq!(lang::en("STR_MSG_CHALLENGE"), english);
    // a later overlay wins, then a reset is English only
    lang::load_overlay("STR_MSG_CHALLENGE:later\n");
    assert_eq!(tr("STR_MSG_CHALLENGE"), "later");
    lang::reset();
    assert_eq!(tr("STR_MSG_CHALLENGE"), english);
    assert_eq!(lang::language(), "en");
    assert_eq!(lang::file_language("##language pt_BR\n"), Some("pt_BR"));
}

#[test]
fn escapes() {
    let p = lang::parse("STR_X :a\\nb \\{NAME\\} {NAME} \\\\ \\t.\nSTR_Y:  two leading spaces\n");
    assert_eq!(lang::fill(&p[0].1, &[("NAME", &"n")]), "a\nb {NAME} n \\ \t.");
    assert_eq!(p[1].1, "  two leading spaces");
}
