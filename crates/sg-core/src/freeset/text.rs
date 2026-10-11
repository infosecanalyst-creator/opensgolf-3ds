//! The free set's text files, all written for OpenSGolf: golfer stories, characters, the famous golfers and celebrities,
//! the terrain descriptions, the lighting of each theme, the credits and a starting high score table. The names are made
//! up; the formats are the game's (docs/FORMATS.md).
use crate::roster::{RECORD, SAYING, SAYINGS};

/// The files, as paths in the game folder (fonts go under fonts/ in the pack, the rest under text/).
pub const FILES: &[&str] = &[
    "KLEPTO__.TTF",
    "manu3_.TTF",
    "credits.txt",
    "ParklandLighting.txt",
    "DesertLighting.txt",
    "TropicalLighting.txt",
    "LinksLighting.txt",
    "top10.sve",
    "Themes/Standard/progolfers.dta",
    "Themes/Standard/celebrities.dta",
    "Themes/Standard/GxxxxxxxOpeningDay.txt",
    "Themes/Standard/CxxxxxxxClubhouseRumours.txt",
    "Themes/Standard/PxxxxxxxRainCheck.txt",
    "Themes/Standard/LxxxxxxxSecondChances.txt",
    "Themes/Standard/MxxxxxxxTheLostBall.txt",
    "Themes/Standard/SxxxxxxxBirdWatchers.txt",
    "Themes/Standard/HxxxxxxxGardenParty.txt",
    "Themes/Standard/Joe Pro.glf",
    "Themes/Standard/I.M.Picky.glf",
    "Themes/Standard/Ivana Richman.glf",
    "Themes/Standard/J.P.Bigdome.glf",
    "Themes/Standard/Ada Birdie.chr",
    "Themes/Standard/Bruno Sandwedge.chr",
    "Themes/Standard/Cora Fairweather.chr",
    "Themes/Standard/Dev Longdrive.chr",
    "Themes/Standard/Elsa Greenway.chr",
    "Themes/Standard/Felix Puttnam.chr",
    "Themes/Championship/Robin Fairway.pro",
];

/// The bytes of one of [`FILES`].
pub fn make(rel: &str) -> Option<Vec<u8>> {
    let low = rel.to_lowercase();
    if low.ends_with(".ttf") {
        return Some(super::paint::FONT_TTF.to_vec());
    }
    if low.ends_with("lighting.txt") {
        return Some(lighting(super::theme_of(&low)).into_bytes());
    }
    if low.ends_with(".glf") || low.ends_with(".chr") || low.ends_with(".pro") {
        return character(&low);
    }
    let file = rel.rsplit('/').next().unwrap_or(rel);
    let text = match file {
        "credits.txt" => CREDITS.to_string(),
        "top10.sve" => return Some(top10()),
        "progolfers.dta" => PROS.to_string(),
        "celebrities.dta" => CELEBRITIES.to_string(),
        _ => STORIES.iter().find(|(name, _)| file.contains(name)).map(|(_, t)| t.to_string())?,
    };
    Some(text.replace('\n', "\r\n").into_bytes())
}

/// The theme's light (the terrain renderer reads the three lines after #AMBIENT, #DIFFUSE and #SPECULAR).
fn lighting(theme: usize) -> String {
    let (a, d, s) = match theme {
        1 => ("205 195 180", "245 238 225", "255 250 240"),
        2 => ("195 205 200", "240 245 240", "255 255 250"),
        3 => ("190 195 200", "232 236 240", "250 250 255"),
        _ => ("200 200 190", "240 240 235", "255 255 250"),
    };
    format!("#AMBIENT\r\n{a}\r\n#DIFFUSE\r\n{d}\r\n#SPECULAR\r\n{s}\r\n")
}

const CREDITS: &str = "#CREDITS

OPENSGOLF

The free graphics, sounds and text
were drawn, synthesised and written
by the OpenSGolf engine itself.

ENGINE
OpenSGolf contributors

FONT
Liberation Sans
SIL Open Font License

Thank you for playing!
#END
";

/// Ten made-up records: player 64 bytes, course 64, three numbers, then the tier and the end marker (docs/FORMATS.md).
fn top10() -> Vec<u8> {
    const ROWS: [(&str, &str, u32); 10] = [
        ("Ada Birdie", "Willow Bend", 9000),
        ("Felix Puttnam", "Saltmarsh Links", 8200),
        ("Cora Fairweather", "Red Mesa", 7400),
        ("Dev Longdrive", "Coral Point", 6600),
        ("Elsa Greenway", "Maple Hollow", 5800),
        ("Bruno Sandwedge", "Dune Ridge", 5000),
        ("Robin Fairway", "Lakeside", 4200),
        ("Ada Birdie", "Pine Valley Park", 3400),
        ("Felix Puttnam", "Cliffside", 2600),
        ("Cora Fairweather", "Sunny Acres", 1800),
    ];
    let mut out = Vec::with_capacity(1560);
    for (k, (who, course, a)) in ROWS.iter().enumerate() {
        let mut r = vec![0u8; 156];
        r[..who.len()].copy_from_slice(who.as_bytes());
        r[64..64 + course.len()].copy_from_slice(course.as_bytes());
        r[128..132].copy_from_slice(&a.to_le_bytes());
        r[132..136].copy_from_slice(&(a / 10).to_le_bytes());
        r[136..140].copy_from_slice(&(10 - k as u32).to_le_bytes());
        r[150..152].copy_from_slice(&((3 - k.min(9) as u16 / 3).min(3)).to_le_bytes());
        r[152..156].copy_from_slice(&(-1i32).to_le_bytes());
        out.extend(r);
    }
    out
}

const PROS: &str = "* Famous golfers (OpenSGolf free set)
* name, body 0-7, skin 0-3, hat 0-9, shirt 0-9, pants 0-9, ten skills as hex digits and a rating:
* power hitter, long driver, accurate driver, accurate irons, accurate putter, draw, fade, backspin, recovery, luck
Ada Birdie, 4, 1, 2, 3, 1, 8AC9B76854 95
Bruno Sandwedge, 1, 2, 0, 6, 2, C9867559A5 90
Cora Fairweather, 5, 0, 4, 1, 7, 79BBA86745 92
Dev Longdrive, 0, 3, 1, 4, 3, FE76565464 88
Elsa Greenway, 6, 1, 6, 2, 5, 69AA9B8656 91
Felix Puttnam, 2, 0, 3, 8, 0, 6789E66575 87
Gus Ironside, 3, 2, 5, 5, 4, 99A9877766 85
Hana Chipshot, 7, 3, 7, 9, 6, 58989B99A4 89
Ivo Slicewell, 1, 1, 8, 0, 8, 7777777777 70
Juno Backspin, 4, 2, 9, 7, 9, 6888777C86 84
Kai Fairlane, 0, 0, 2, 6, 1, A99887A776 86
Lena Drawshot, 5, 3, 0, 3, 2, 7AA88AD675 88
Milo Roughly, 2, 1, 1, 1, 5, 87656577A3 65
Nora Teebox, 6, 2, 3, 4, 7, 6676689766 75
Otto Lofting, 3, 0, 6, 8, 0, 8676667A94 72
Pia Greenside, 7, 1, 4, 2, 3, 5689A86685 80
";

const CELEBRITIES: &str = "* Celebrities (OpenSGolf free set)
* name, type, skin 0-3, hair 0-4, shirt 0-9, pants 0-9
* types: A action star, B pop singer, C politician, D comedian, E model, F fitness coach, G comic, H celebrity,
* I film princess, J rock star, K basketball player
Rex Thunder, A, 1, 2, 3, 4
Lyra Starling, B, 0, 1, 7, 2
Senator Pat Ledger, C, 2, 3, 0, 0
Chuck Gaggle, D, 1, 0, 5, 3
Vera Glimmer, E, 0, 4, 9, 1
Fiona Flex, F, 3, 2, 2, 6
Dot Quipley, G, 2, 1, 8, 5
Max Headline, H, 0, 3, 1, 2
Princess Aurelia, I, 1, 4, 6, 7
Jett Amplifier, J, 3, 0, 4, 9
Slam Dunkworth, K, 2, 2, 7, 8
";

/// Stories: a title line starting with a space, then four scenes, each an opening line and replies (indented), best
/// reply first. The file name's first letter picks the landmark a happy ending unlocks, the next seven who may tell it
/// ('x' anyone), docs/FORMATS.md.
const STORIES: &[(&str, &str)] = &[
    (
        "OpeningDay",
        " Opening Day
What a morning for the first round on this course, PARTNER!
 It's perfect. I can smell the fresh-cut grass.
 Not bad. A little early for me.
 Mornings are for sleeping.
I hear the owner designed every hole personally.
 Then I'll play every hole personally too!
 Let's hope they know what they're doing.
 Hmph. We'll see.
Look, they even put flowers along the path.
 Lovely. Little touches like that make a course.
 Flowers don't fix a bad lie.
 I'm allergic.
Same time next week, PARTNER?
 Wouldn't miss it!
 Maybe, if the greens are faster.
 Don't count on it.
",
    ),
    (
        "ClubhouseRumours",
        " Clubhouse Rumours
Did you hear the clubhouse might get a new wing?
 Exciting! More room for the trophy case.
 As long as they don't raise the fees.
 Gossip, PARTNER. Pure gossip.
They say a famous pro is coming to play here.
 I'd love an autograph!
 I'll believe it when I see it.
 Who cares about famous pros?
I think the pro shop is hiding a secret putter.
 Let's go and find it after the round!
 It's probably just an old putter.
 You and your secrets.
Whatever happens, this place keeps getting better.
 It truly does. Here's to the club!
 It has its moments.
 If you say so.
",
    ),
    (
        "RainCheck",
        " Rain Check
Those clouds look like rain, PARTNER.
 A little rain never hurt a round!
 We could finish quickly.
 Let's just go home.
Funny how the ball rolls further on a wet green.
 Every shower teaches you something new.
 Funny isn't the word I'd use.
 It's not funny at all.
The sun is coming out again!
 And there's a rainbow over the fairway!
 About time.
 Too late, my socks are soaked.
Shall we shelter in the snack bar next time?
 Hot cocoa is on me!
 Sure, why not.
 Only if you pay.
",
    ),
    (
        "SecondChances",
        " Second Chances
I almost quit golf last year, PARTNER.
 I'm glad you didn't!
 What changed your mind?
 Should have stuck with it.
A friend told me to play just for fun.
 That's the best advice there is.
 Fun is overrated.
 Your friend was wrong.
Now I look forward to every weekend.
 Me too. These rounds are the best part of my week.
 They're all right.
 Weekends are for chores.
Thanks for being such good company out here.
 Thank you! Let's keep playing together.
 Don't mention it.
 Whatever.
",
    ),
    (
        "TheLostBall",
        " The Lost Ball
Have you seen my ball, PARTNER? It went into the trees.
 I'll help you look!
 Did you mark where it landed?
 Just drop another one.
Here's one! Oh, it's not yours.
 Someone's loss is our lucky find.
 Put it back, it isn't ours.
 Finders keepers.
Found it! Right behind that big rock.
 Hooray! Great eyes!
 Finally.
 You took forever.
I'll aim for the fairway next time.
 Good plan! We'll practise together.
 Sounds sensible.
 Sure you will.
",
    ),
    (
        "BirdWatchers",
        " Bird Watchers
Look, PARTNER, a heron on the pond!
 Beautiful! Let's not scare it away.
 Nice. Your turn to putt.
 It'll steal our balls.
I keep a notebook of the birds I see on courses.
 What a lovely hobby.
 Different strokes.
 Sounds dull.
That one's a woodpecker, I'm sure of it.
 Listen to it tapping!
 If you say so.
 Too noisy.
Nature makes a good round even better.
 Couldn't agree more.
 It helps, I suppose.
 Nature makes me sneeze.
",
    ),
    (
        "GardenParty",
        " Garden Party
I'm planning a garden party, PARTNER. Will you come?
 I'd be delighted!
 Maybe, if I'm free.
 Parties aren't my thing.
I'll bring my famous lemon cake.
 My mouth is watering already.
 Lemon is fine.
 I don't like lemon.
We could play a fun round of putting after lunch.
 The perfect afternoon!
 One round, then.
 Count me out.
It's settled then: see you on Sunday!
 See you Sunday, I can't wait!
 See you then.
 We'll see.
",
    ),
];

/// A character file: the roster record (job, name, traits, looks, head), the dialogue slots, and for a .pro file the skills.
fn character(low: &str) -> Option<Vec<u8>> {
    struct C {
        file: &'static str,
        job: &'static str,
        name: &'static str,
        female: bool,
        head: u8,
        traits: u8,
        bio: &'static str,
        say: &'static [(usize, &'static str)],
    }
    const SAY: &[(usize, &str)] = &[
        (0, "What a shot!"),
        (1, "Oops, that one got away."),
        (2, "Not my best, not my worst."),
        (13, "It's getting crowded out here!"),
        (14, "I could use a cold drink."),
        (15, "Ah, that hit the spot."),
        (16, "My stomach is rumbling."),
        (17, "Delicious, thank you!"),
        (18, "My feet are tired."),
        (19, "A short rest and I'm ready again."),
    ];
    const ALL: [C; 11] = [
        C {
            file: "joe pro.glf",
            job: "Golf Pro",
            name: "Sam Club",
            female: false,
            head: 8,
            traits: 0x11,
            bio: "The club's own professional.",
            say: SAY,
        },
        C {
            file: "i.m.picky.glf",
            job: "Inspector",
            name: "Iris Fussworth",
            female: true,
            head: 3,
            traits: 0x02,
            bio: "Notices every divot.",
            say: SAY,
        },
        C {
            file: "ivana richman.glf",
            job: "Investor",
            name: "Goldie Vault",
            female: true,
            head: 6,
            traits: 0x04,
            bio: "Buys land, loves views.",
            say: SAY,
        },
        C {
            file: "j.p.bigdome.glf",
            job: "Tycoon",
            name: "Monty Banks",
            female: false,
            head: 11,
            traits: 0x08,
            bio: "Owns half the town.",
            say: SAY,
        },
        C {
            file: "ada birdie.chr",
            job: "Teacher",
            name: "Ada Birdie",
            female: true,
            head: 1,
            traits: 0x01,
            bio: "Plays every Saturday.",
            say: SAY,
        },
        C {
            file: "bruno sandwedge.chr",
            job: "Chef",
            name: "Bruno Sandwedge",
            female: false,
            head: 4,
            traits: 0x10,
            bio: "Finds every bunker.",
            say: SAY,
        },
        C {
            file: "cora fairweather.chr",
            job: "Pilot",
            name: "Cora Fairweather",
            female: true,
            head: 9,
            traits: 0x06,
            bio: "Only plays in sunshine.",
            say: SAY,
        },
        C {
            file: "dev longdrive.chr",
            job: "Engineer",
            name: "Dev Longdrive",
            female: false,
            head: 14,
            traits: 0x11,
            bio: "Hits it long, not always straight.",
            say: SAY,
        },
        C {
            file: "elsa greenway.chr",
            job: "Gardener",
            name: "Elsa Greenway",
            female: true,
            head: 12,
            traits: 0x0a,
            bio: "Admires every flower bed.",
            say: SAY,
        },
        C {
            file: "felix puttnam.chr",
            job: "Librarian",
            name: "Felix Puttnam",
            female: false,
            head: 2,
            traits: 0x02,
            bio: "A wizard on the greens.",
            say: SAY,
        },
        C {
            file: "robin fairway.pro",
            job: "Golf Pro",
            name: "Robin Fairway",
            female: false,
            head: 5,
            traits: 0x11,
            bio: "A steady championship player.",
            say: SAY,
        },
    ];
    let name = low.rsplit('/').next()?;
    let c = ALL.iter().find(|c| c.file == name)?;
    let mut b = vec![0u8; RECORD + SAYINGS * SAYING + 16];
    let put = |b: &mut [u8], at: usize, len: usize, s: &str| {
        let s = s.as_bytes();
        let n = s.len().min(len - 1);
        b[at..at + n].copy_from_slice(&s[..n]);
    };
    put(&mut b, 0, 16, c.job);
    put(&mut b, 0x10, 16, c.name);
    b[0x20] = c.traits;
    b[0x21] = if c.female { 0x80 | (c.head & 3) } else { c.head & 3 };
    b[0x22] = c.head;
    b[0x24] = c.head % 10;
    b[0x25] = (c.head * 3) % 10;
    b[0x27] = c.head % 4;
    b[0x28] = c.head % 5;
    put(&mut b, 0x30, RECORD - 0x30, c.bio);
    for &(slot, line) in c.say {
        put(&mut b, RECORD + slot * SAYING, SAYING, line);
    }
    if name.ends_with(".pro") {
        // the pro's skill bytes after the sayings
        let at = RECORD + SAYINGS * SAYING;
        b[at..at + 10].copy_from_slice(&[6, 7, 6, 7, 6, 5, 5, 4, 5, 6]);
    }
    Some(b)
}

/// The terrain descriptions of a theme: "*Name" and the description on the next line, for every key the engine looks up.
pub fn terrain_text(low: &str, keys: &[String]) -> Vec<u8> {
    let theme = ["Parkland", "Desert", "Tropical", "Links"][super::theme_of(low)];
    let mut s = format!("Terrain descriptions\r\n\r\n{theme}\r\n\r\n");
    for k in keys {
        if k.eq_ignore_ascii_case("END") {
            continue;
        }
        s.push_str(&format!("*{k}\r\n{}\r\n\r\n", describe(k)));
    }
    s.push_str("*END\r\n");
    s.into_bytes()
}

fn describe(k: &str) -> &'static str {
    let k = k.to_lowercase();
    match k.as_str() {
        "tees" => "Level ground where every hole begins.",
        "green" => "Short, smooth grass around the hole. Putt with care.",
        "sand trap" => "Soft sand that grabs the ball. Hard to escape.",
        "pot bunker" => "A small, deep bunker. Golfers dread it.",
        "water" => "A pond or lake. Balls that land here are lost.",
        "fairway" => "Short grass: the easiest place to play from.",
        "firm fairway" => "Hard, fast grass. The ball rolls a long way.",
        "rough" | "desert" => "Longer grass beside the fairway. A tougher lie.",
        "deep rough" => "Thick, tall grass that swallows the ball.",
        "waste bunker" => "A wide sandy area, easier than a trap.",
        "brush" | "gorse" => "Low, scrubby plants. Find the ball if you can.",
        "rocks" => "Stones and boulders. Unpredictable bounces.",
        "stream" | "burn" | "ravine" => "A narrow channel of water or a gully crossing the hole.",
        "cactus" => "Prickly desert plants. Keep your distance.",
        _ => "Trees: they block shots and frame the holes.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_files_parse() {
        for f in FILES {
            assert!(make(f).is_some(), "{f}");
        }
        let pros = crate::formats::parse_pro_golfers(&String::from_utf8(make("Themes/Standard/progolfers.dta").unwrap()).unwrap()).unwrap();
        assert!(pros.len() >= 10);
        let p = crate::roster::Person::parse_file(&make("Themes/Standard/Ada Birdie.chr").unwrap());
        assert_eq!((p.name.as_str(), p.female()), ("Ada Birdie", true));
        assert!(crate::championship::parse_pro(&make("Themes/Championship/Robin Fairway.pro").unwrap()).is_some());
        assert_eq!(make("top10.sve").unwrap().len(), 1560);
        let s = crate::formats::parse_story(&String::from_utf8(make("Themes/Standard/GxxxxxxxOpeningDay.txt").unwrap()).unwrap());
        assert!(s.is_ok());
    }
}
