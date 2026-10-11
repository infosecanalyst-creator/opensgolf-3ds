//! Golden tests for the string table: a sample of the game's formatted text, each compared with the exact string the
//! engine produced before its text moved into lang/en.txt (captured from the code of that time), so the English the game
//! shows stays byte for byte the same. Also checks lang/en.txt against the keys the code uses.
use crate::course::Course;
use crate::golfer::{Club, Event};
use crate::land::ExeRng;

/// The sample: (what, the text the engine makes now).
fn cases() -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = Vec::new();
    let mut add = |what: &str, text: String| v.push((what.to_string(), text));
    let mut cl = Club::default();
    let mut c = Course::default();
    for h in 1..10 {
        cl.holes[h].par = 3 + (h % 3) as i32;
    }
    cl.g[2].roster = 4;
    cl.g[3].roster = 5;
    // golfers' thoughts and the first-time narration
    for (id, arg) in [(0x01, 0), (0x02, 7), (0x05, 17), (0x0b, 0x100 + 75), (0x0c, 13), (0x13, 0), (0x1c, 61), (0x36, 4), (0x3d, 5)] {
        add(&format!("thought {id:x} {arg}"), cl.thought(&c, id, arg, 2).text);
    }
    cl.g[2].field_ae = 4;
    add("thought partner", cl.thought(&c, 0x02, 13, 2).text);
    cl.g[2].field_ae = 0;
    for (id, arg) in [(1, 0), (2, 7), (0x0b, 61), (0x15, 0), (0x16, 0), (0x41, 0)] {
        add(&format!("narration {id:x}"), cl.narration_text(&c, id, arg, 2).unwrap_or_default());
    }
    c.theme = 3;
    add("narration links", cl.narration_text(&c, 3, 10, 2).unwrap_or_default());
    c.theme = 0;
    for m in [0, 5, 8, 99] {
        add(&format!("opinion {m}"), crate::thoughts::course_opinion(m, 0).to_string());
        add(&format!("visitor remark {m}"), crate::vips::mood_remark(m).to_string());
    }
    // the hole opening and what it unlocks
    add("opening short", crate::holetool::opening_text(3, 4, 300, 300, 0x20 | 0x1000));
    add("opening long", crate::holetool::opening_text(12, 5, 590, 600, 0x40 | 0x2000));
    add("opening scenic", crate::holetool::opening_text(1, 3, 150, 150, 0));
    // special visitors
    let mut rng = ExeRng::from_clock(3);
    cl.ratings.fun = 5000;
    cl.difficulty = 0;
    for (kind, mood, hole) in [(0x60u8, 5, 12), (0x60, 1, 3), (0x40, 5, 12), (0x40, 1, 3), (0x80, 1, 3), (0x80, 8, 12)] {
        cl.g[3].kind = kind;
        cl.g[3].mood = mood;
        cl.g[3].hole = hole;
        cl.landmarks_owned = 0;
        cl.vip_verdict(&mut rng, 3);
        add(&format!("visitor {kind:x} {mood}"), messages(&mut cl));
    }
    cl.celebrities = crate::vips::parse_celebrities("Sylvester Stallion,A,1,4,0,0\n");
    let _ = cl.celebrity_home(&mut rng, 100000, 0, (5, 5));
    add("celebrity home", messages(&mut cl));
    add("landmark a", crate::vips::landmark_after_a(2));
    // the year-end log
    for v in [crate::records::log::HOLE | 3, crate::records::log::BUILT | 7, crate::records::log::CEO, crate::records::log::HEIRESS | 4] {
        add(&format!("log {v:x}"), cl.event_text(v));
    }
    for id in [0, 4, 11, 21] {
        add(&format!("award {id}"), cl.award_title(id).to_string());
    }
    // tracts for sale
    let tract = crate::tracts::Tract { oob: 250, price: 50, top: [13, 4, 17] };
    for theme in 0..4 {
        add(&format!("tract {theme}"), crate::tracts::describe(&tract, 4, theme));
    }
    // the hole names and the hole stats words
    let mut hole = cl.holes[4].clone();
    add("hole name", crate::ratings::hole_name(4, &hole));
    hole.flags |= 1;
    add("hole proper name", crate::ratings::hole_name(4, &hole));
    add("fun word", crate::ratings::fun_word(45).to_string());
    // the pro
    add("attitude", crate::pro::attitude(-3).to_string());
    add("lie", crate::pro::lie_name(9).to_string());
    cl.next_hole = 10;
    for k in [0, 1, 2, 3] {
        add(&format!("money {k}"), crate::economy::money_digits(1234567 * k));
    }
    add("marital", cl.marital(2).to_string());
    add("vip name", cl.vip_name(2));
    // a match against a famous golfer: a hole's money, the end of the match and the standing (0x427380)
    for (challenger, pro) in [(3, 3), (5, 3), (2, 3), (3, 5), (3, 4)] {
        let mut m = Club::default();
        m.holes[1].par = 4;
        m.holes[2].par = 3;
        m.next_hole = 3;
        m.wager_level = 2;
        m.gary = 3;
        m.g[2].class = 0x27;
        m.g[3].class = 0x47;
        m.g[2].partner = 3;
        m.g[3].partner = 2;
        m.g[2].card[1] = 5;
        m.g[3].card[1] = 4;
        m.g[2].hole = 1;
        m.g[3].hole = 2;
        m.match_money(2);
        m.g[2].card[2] = challenger;
        m.g[3].card[2] = pro;
        m.g[2].hole = 2;
        m.g[3].hole = 3;
        m.match_money(2);
        add(&format!("match {challenger} {pro}"), messages(&mut m));
    }
    v
}

/// The messages the club posted since the last call, one a line.
fn messages(cl: &mut Club) -> String {
    let out: Vec<String> = cl.out.drain(..).filter_map(|e| if let Event::Message { text, .. } = e { Some(text) } else { None }).collect();
    cl.ticker_busy = false;
    out.join("\n")
}

/// The texts as the engine made them before the string table (see the module comment).
const EXPECTED: &[(&str, &str)] = &[
    ("thought 1 0", "How'd you like that shot, Ben?"),
    ("thought 2 7", "Darn, I'm in the sand trap!"),
    ("thought 5 17", "Eeek, I gotta stay away from the water."),
    ("thought b 331", "Check out this nice ornamental grass!"),
    ("thought c 13", "Darn that stupid tree!"),
    ("thought 13 0", "Par. I hate this stupid course."),
    ("thought 1c 61", "I am liking that ornamental grass."),
    ("thought 36 4", "I'll use my 3 Iron for this shot."),
    ("thought 3d 5", "Sure does."),
    ("thought partner", "Oh darn, you're under the tree, dear!"),
    ("narration 1", "'How'd you like that shot, Ben?' Larry has successfully made a tough-looking shot and is enjoying your course."),
    ("narration 2", "'Darn, I'm in the sand trap!' Larry is disappointed that an easy looking shot ended up in the sand trap. If it happens again he will become discouraged."),
    ("narration b", "'Check out this beautiful ornamental grass!' Larry is pleased to have a scenic ornamental grass in view for the next shot."),
    ("narration 15", "'I'm tired of waiting for these slowpokes.' Larry is frustrated that a lot of golfers are waiting at this hole. Placing a Ranger/Marshall near the tee will speed up play on this hole. Click the employees button on the People menu to hire course employees."),
    ("narration 16", "'Hey, that's a movie star's house!' Larry is excited to see 's vacation home on your golf course."),
    ("narration 41", "'I didn't see where that shot landed.' Larry hit a good shot but cannot see exactly where it landed because of trees or elevation. "),
    ("narration links", "'Great, now I'm in the ravine!' Larry is disappointed that the shot has ended up in a worse location, the burn."),
    ("opinion 0", "I hate this stupid course."),
    ("visitor remark 0", "I hate this stupid course!"),
    ("opinion 5", "This course is quite nice."),
    ("visitor remark 5", "This course is quite nice."),
    ("opinion 8", "This is the best course I've ever played!"),
    ("visitor remark 8", "This is the best course I've ever played!"),
    ("opinion 99", "This is the best course I've ever played!"),
    ("visitor remark 99", "This is the best course I've ever played!"),
    ("opening short", "Hole #3, a short 300 yard uphill par 4 with a dogleg to the left, is now open for play."),
    ("opening long", "Hole #12, a long 590 yard downhill par 5 with a dogleg to the right, is now open for play."),
    ("opening scenic", "Hole #1, a scenic 150 yard par 3, is now open for play."),
    ("visitor 60 5", "Corporate CEO J.P. Bigdome has decided to invest in your course. 'I'll give you §10,000 for a seat on the board' he says."),
    ("visitor 60 1", "Corporate CEO J.P. Bigdome has decided not to invest in your course. 'I don't like this course much.' he fumes."),
    ("visitor 40 5", "County commissioner I.M. Picky has decided to approve an expansion request! 'I'll be back again if your fun rating is over 300', he says."),
    ("visitor 40 1", "County commissioner I.M. Picky has decided not to approve your expansion request. 'I don't like this course much.' he comments."),
    ("visitor 80 1", "Wealthy Heiress Ivana Richman has decided not to donate a landmark to your course. 'I don't like this course much.' she comments. 'I'll be back in a few months.'"),
    ("visitor 80 8", "Wealthy Heiress Ivana Richman has decided to donate a garden sundial to your course! In the area near this landmark golfers will have happy thoughts. Go to 'Landmarks' under the Improvements menu to place your landmark. Additional sundials can be added to your course for §1,000. 'I'll be back if your fun rating is up to 100', she says."),
    ("celebrity home", "International action star Sylvester Stallion has purchased a vacation home at your golf course!"),
    ("landmark a", "n authentic Civil War cannon"),
    ("log 23", "Hole 3 opened."),
    ("log 47", "Snack Bar built."),
    ("log 100", "J.P. Bigdome joins the board."),
    ("log 164", "Ivana donates a water mill"),
    ("award 0", "1st Dogleg Right Hole"),
    ("award 4", "1st Par 5 Hole"),
    ("award 11", "1st Tournament Victory (9+ Hole)"),
    ("award 21", "1st 100 Star Rating"),
    ("tract 0", "Buy tract #5, 25 acres of trees, rough, and water"),
    ("tract 1", "Buy tract #5, 25 acres of joshua trees, desert, and water"),
    ("tract 2", "Buy tract #5, 25 acres of tropical bushes, rough, and water"),
    ("tract 3", "Buy tract #5, 25 acres of maple trees, rough, and water"),
    ("hole name", "Hole 4"),
    ("hole proper name", "Crab Apple"),
    ("fun word", "very good"),
    ("attitude", "mad"),
    ("lie", "Pot Bunker"),
    ("money 0", "0"),
    ("money 1", "1,234,567"),
    ("money 2", "2,469,134"),
    ("money 3", "3,703,701"),
    ("marital", "Single"),
    ("vip name", "Larry"),
    ("match 3 3", "Gary Golf wins hole #1 by a score of 4 to 5! Collect §4,000.\nAt the end of the match, you win §4,000.\nGary Golf leads Gary Golf by 1 shot."),
    ("match 5 3", "Gary Golf wins hole #1 by a score of 4 to 5! Collect §4,000.\nAt the end of the match, you win §4,000.\nGary Golf leads Gary Golf by 3 shots."),
    ("match 2 3", "Gary Golf wins hole #1 by a score of 4 to 5! Collect §4,000.\nAt the end of the match, the score is tied.\nGary Golf and Gary Golf are tied at even par."),
    ("match 3 5", "Gary Golf wins hole #1 by a score of 4 to 5! Collect §4,000.\nAt the end of the match, you lose §4,000.\nGary Golf trails Gary Golf by 1 shot."),
    ("match 3 4", "Gary Golf wins hole #1 by a score of 4 to 5! Collect §4,000.\nAt the end of the match, the score is tied.\nGary Golf and Gary Golf are tied at 1 over par."),
];

#[test]
fn golden_text_is_unchanged() {
    let got = cases();
    if std::env::var_os("LANG_GOLDEN_PRINT").is_some() {
        for (w, t) in &got {
            println!("    ({w:?}, {t:?}),");
        }
    }
    assert_eq!(got.len(), EXPECTED.len(), "the sample and the expected table differ in length");
    for ((w, t), (ew, et)) in got.iter().zip(EXPECTED) {
        assert_eq!(w, ew);
        assert_eq!(t, et, "{w}");
    }
}

/// Every `"STR_..."` literal in the engine's sources (the crates' src folders; lang.rs's test keys and this file aside),
/// with where it is.
fn keys_in_code() -> Vec<(String, String)> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut out = Vec::new();
    let mut dirs: Vec<std::path::PathBuf> = std::fs::read_dir(&root).unwrap().flatten().map(|e| e.path().join("src")).collect();
    while let Some(d) = dirs.pop() {
        let Ok(list) = std::fs::read_dir(&d) else { continue };
        for e in list.flatten() {
            let p = e.path();
            if p.is_dir() {
                dirs.push(p);
            } else if p.extension().is_some_and(|x| x == "rs")
                && !p.ends_with("sg-core/src/lang.rs")
                && !p.ends_with("sg-core/src/lang_golden.rs")
            {
                let src = std::fs::read_to_string(&p).unwrap();
                let mut rest = src.as_str();
                while let Some(i) = rest.find("\"STR_") {
                    let tail = &rest[i + 1..];
                    let n = tail.find(|c: char| !(c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')).unwrap_or(tail.len());
                    if tail[n..].starts_with('"') {
                        out.push((tail[..n].to_string(), p.display().to_string()));
                    }
                    rest = &tail[n..];
                }
            }
        }
    }
    out
}

#[test]
fn en_txt_has_every_key_once_and_no_unused_key() {
    let en = crate::lang::parse(crate::lang::EN);
    let mut seen = std::collections::HashSet::new();
    for (k, _) in &en {
        assert!(seen.insert(*k), "{k} is in lang/en.txt twice");
    }
    // every line is a comment, blank or a key
    for l in crate::lang::EN.lines() {
        let t = l.trim();
        assert!(t.is_empty() || t.starts_with('#') || t.starts_with("STR_") && t.contains(':'), "bad line in lang/en.txt: {l}");
    }
    let used = keys_in_code();
    let used_set: std::collections::HashSet<&str> = used.iter().map(|(k, _)| k.as_str()).collect();
    for (k, file) in &used {
        assert!(seen.contains(k.as_str()), "{k} (used in {file}) is not in lang/en.txt");
    }
    for (k, _) in &en {
        assert!(used_set.contains(k), "{k} is in lang/en.txt but no code uses it");
    }
}

#[test]
fn trailing_spaces_are_kept() {
    // the original builds some messages from pieces that end in a space: an editor that trims lines would break them
    const KEYS: [&str; 36] = [
        "STR_WELCOME_TROPICAL",
        "STR_MSG_MEMBER_APPLIES",
        "STR_NARR_41_CANNOT_SEE",
        "STR_VIP_PARTNER_QUIT",
        "STR_CHALLENGE_WAGER",
        "STR_CHALLENGE_AND",
        "STR_SHOT_LAUNCHES",
        "STR_SHOT_SNAKES",
        "STR_SHOT_KNOCKS",
        "STR_SHOT_LOFTS",
        "STR_SHOT_TRICKY",
        "STR_SHOT_TOUGH",
        "STR_SHOT_DRIVE",
        "STR_SHOT_PUTT",
        "STR_SHOT_CLUB",
        "STR_SHOT_YARDS",
        "STR_SHOT_YANKS",
        "STR_SHOT_FLUBS",
        "STR_SHOT_SCUFFS",
        "STR_SHOT_FLUFFS",
        "STR_SHOT_INTO_THE",
        "STR_SHOT_AT_HOLE",
        "STR_SHOT_SKILL_IMPROVES",
        "STR_SHOT_SKILL_CAP",
        "STR_UNLOCK_RANK",
        "STR_UNLOCK_NEW_FACILITIES",
        "STR_UNLOCK_NEW_PLAYERS",
        "STR_HOLE_OPEN_SHORT",
        "STR_HOLE_OPEN_LONG",
        "STR_HOLE_OPEN_SCENIC",
        "STR_HOLE_OPEN_START",
        "STR_HOLE_OPEN_UPHILL",
        "STR_HOLE_OPEN_DOWNHILL",
        "STR_LIST_SEPARATOR",
        "STR_HOLESTATS_AVG_DRIVE",
        "STR_HOLESTATS_PAR",
    ];
    for k in KEYS {
        assert!(crate::lang::tr(k).ends_with(' '), "{k} lost its trailing space");
    }
}

#[test]
fn keys_are_never_built_at_run_time() {
    // a key made with format! could not be checked above
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    for krate in ["sg-core", "simgolf"] {
        for e in std::fs::read_dir(root.join(krate).join("src")).unwrap().flatten() {
            let src = std::fs::read_to_string(e.path()).unwrap();
            assert!(!src.contains("format!(\"STR_"), "{} builds a key at run time", e.path().display());
        }
    }
}

#[test]
fn every_parameter_is_a_name_or_a_position() {
    // a text's parameters are names in capitals or positions; a brace that is not one must be escaped (\{ and \})
    for (k, v) in crate::lang::parse(crate::lang::EN) {
        let mut rest = v.as_str();
        while let Some(i) = rest.find('{') {
            let tail = &rest[i + 1..];
            let j = tail.find('}').unwrap_or_else(|| panic!("{k}: an unclosed brace"));
            let name = &tail[..j];
            assert!(
                !name.is_empty() && name.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_'),
                "{k}: {{{name}}} is not a parameter name"
            );
            rest = &tail[j + 1..];
        }
    }
}
