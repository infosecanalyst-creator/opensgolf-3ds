//! The free set's text files, all written for OpenSGolf: golfer stories, characters with their own sayings, the famous
//! golfers and celebrities, the terrain descriptions of each theme, the lighting of each theme, the credits and a starting
//! high score table. Every name is made up; the formats are the game's (docs/FORMATS.md).
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
    "Themes/Standard/AxxxxxxxTheMulligan.txt",
    "Themes/Standard/FxxxYxxxTheApprentice.txt",
    "Themes/Standard/RxxxxOxxFirstDate.txt",
    "Themes/Standard/XxxxxxxxTimeCapsule.txt",
    "Themes/Standard/CxxxxxxxTheGadget.txt",
    "Themes/Standard/PxxxxxxxThePodcast.txt",
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

A free engine for a sunny little
game about building golf courses
and the people who complain
about them.


THE FREE SET

Pictures, sounds, music and words
drawn, synthesised and written
in code for OpenSGolf.
No pixels were harmed.


ENGINE
The OpenSGolf contributors

ART
Grown from code: trees, clubhouses,
golfers and every blade of grass
(we counted most of them)

SOUND
Club, ball, crowd, birds, waves,
wind, and one very dramatic duck

VOICES
Performed in fluent Golfish,
a language with forty words
for 'my ball went in the pond'

MUSIC
Original tunes for parkland,
desert, tropics and links,
plus a lounge band that knows
exactly one song

WORDS
Stories, sayings and gossip
overheard near the snack bar

FONT
Liberation Sans
SIL Open Font License


SPECIAL THANKS

Everyone who ever built a course
just to watch someone hit a tree

The groundskeepers, who never
get the credit (until now)

The ducks on the third hole,
for their patience

The committee, for meeting about
the meeting about the bunkers


No golf balls were lost in the
making of this game. Several
were misplaced with feeling.


Thank you for playing!
Please replace your divots.
#END
";

/// Ten made-up records: player 64 bytes, course 64, three numbers, then the tier and the end marker (docs/FORMATS.md).
fn top10() -> Vec<u8> {
    const ROWS: [(&str, &str, u32); 10] = [
        ("Ada Birdie", "Puttering Pines", 9000),
        ("Felix Puttnam", "Saltmarsh Links", 8200),
        ("Cora Fairweather", "Red Mesa Dunes", 7400),
        ("Dev Longdrive", "Coral Cove", 6600),
        ("Elsa Greenway", "Maple Hollow", 5800),
        ("Bruno Sandwedge", "The Sandbox", 5000),
        ("Robin Fairway", "Lakeside Loop", 4200),
        ("Gus Ironside", "Windy Knolls", 3400),
        ("Hana Chipshot", "Cliffside Greens", 2600),
        ("Milo Roughly", "Somewhere Nice", 1800),
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
Quinn Overlap, 0, 2, 5, 9, 6, 9A8878897A 83
Ruby Dimple, 4, 0, 7, 1, 9, 77899A7B86 86
Sol Divot, 1, 3, 9, 7, 4, B8766678C7 78
Tess Albatross, 5, 2, 1, 5, 8, DB98877786 93
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
/// ('x' anyone; see stories.rs), docs/FORMATS.md. Lines stay under about 60 characters for the speech bubbles.
const STORIES: &[(&str, &str)] = &[
    (
        "OpeningDay",
        " Opening Day
Big day, PARTNER! First round on a brand new course!
 I've been counting down the days!
 Let's see if it lives up to the brochure.
 New courses are just old fields with flags.
The owner designed every single hole, apparently.
 Then I'll enjoy every single hole, personally.
 Bold. Let's hope they had a plan.
 It shows. Not in a good way.
Look, they planted flowers along the cart path.
 Lovely! Little touches like that make a course.
 Flowers don't fix a bad lie.
 I'm allergic to effort. And pollen.
Same time next week, PARTNER?
 Wouldn't miss it! Save me the first tee time.
 Maybe, if the greens are any faster.
 Don't count on it.
",
    ),
    (
        "ClubhouseRumours",
        " Clubhouse Rumours
Psst, PARTNER. The committee is meeting about the new wing.
 Ooh! More room for the trophy case!
 Another meeting? About a meeting?
 Wake me when they've built something.
They say socks with sandals may soon be allowed.
 A bold new era for fashion!
 The committee will never stand for it.
 Over my dead, well-dressed body.
And the pro shop is hiding a secret putter.
 Let's sneak in and look after the round!
 It's probably just a putter. In a drawer.
 You and your secrets.
Whatever's true, this place keeps getting better.
 Here's to the club! May the gossip be true!
 It has its moments.
 If you say so.
",
    ),
    (
        "RainCheck",
        " Rain Check
Those clouds look a bit damp, PARTNER.
 A little rain never hurt a round!
 We could play fast and finish dry.
 I'm going home. Goodbye forever.
Funny how the ball goes skiing on a wet green.
 Every shower teaches you something new.
 Funny isn't the word I'd use.
 My ball is now a boat.
The sun is out! And there's a rainbow on the fairway!
 I'm aiming for the pot of gold!
 About time.
 Too late. My socks have resigned.
Next time it rains: snack bar, cocoa, marshmallows?
 Cocoa's on me! Extra marshmallows!
 Sure, why not.
 Only if you're paying.
",
    ),
    (
        "SecondChances",
        " Second Chances
I nearly quit golf last year, PARTNER.
 I'm so glad you didn't!
 What changed your mind?
 Honestly, you should have.
A friend said: just play for fun. Forget the score.
 Best advice in the whole sport.
 Easy for them to say.
 Your friend has clearly never seen you putt.
Now I look forward to every single weekend.
 Same! These rounds are the best part of my week.
 They're all right, I suppose.
 Weekends are for chores.
Thanks for being such good company out here.
 Thank you! Same time, same tee, next week.
 Don't mention it.
 Whatever.
",
    ),
    (
        "TheLostBall",
        " The Lost Ball
Have you seen my ball, PARTNER? It went into the trees.
 I'll help you look. Two pairs of eyes!
 Did you see where it landed?
 Just drop another one. We'll be here all day.
Found one! Oh. It's not yours. It's very old.
 Someone's loss is our lucky find!
 Put it back, it isn't ours.
 Finders keepers.
There it is! Hiding behind that big rock.
 Hooray! Great eyes!
 Finally.
 Took you long enough.
I'll aim for the fairway next time. Really.
 Good plan! We'll practise together.
 Sounds sensible.
 Sure you will.
",
    ),
    (
        "BirdWatchers",
        " Bird Watchers
Look, PARTNER! A heron, right there by the pond!
 Shh! Let's not scare it away.
 Nice. Your turn to putt, though.
 It's eyeing my ball. I don't trust it.
I keep a notebook of every bird I see on a course.
 What a lovely hobby! How many so far?
 Different strokes for different folks.
 That sounds like homework.
That tapping? A woodpecker, I'm sure of it.
 Listen to that rhythm! It's a drummer!
 If you say so.
 It's doing that on purpose to put me off.
Birds make a good round even better, don't they?
 Couldn't agree more. Even the noisy ones.
 They help, I suppose.
 One of them stole my sandwich.
",
    ),
    (
        "GardenParty",
        " Garden Party
I'm throwing a garden party, PARTNER. Will you come?
 I'd be delighted! What shall I bring?
 Maybe, if I'm free.
 Parties aren't really my thing.
I'm baking my famous lemon cake.
 My mouth is watering already.
 Lemon's fine.
 I don't trust a cake with a reputation.
After lunch, a little putting contest on the lawn?
 The perfect afternoon! Loser does the dishes.
 One round, then.
 Count me out.
So it's settled: Sunday, three o'clock!
 See you Sunday! I'll bring my lucky putter.
 See you then.
 We'll see.
",
    ),
    (
        "TheMulligan",
        " The Mulligan
PARTNER, can I tell you a secret? I take mulligans.
 Everyone does! It's practically a tradition.
 Only on the first tee, I hope.
 That's called cheating. Loudly.
I took three on the last hole. And a fourth.
 Practice swings with a ball in the way!
 That's... a lot of mulligans.
 I'm telling the club.
Today I'm going to try a whole round without one.
 You can do it! I'll cheer every shot.
 Brave. Let's see how long it lasts.
 I'll believe it on the eighteenth.
I did it! Eighteen holes, no do-overs!
 I knew you could! That calls for a soda!
 Well done. Mostly.
 Your score says otherwise.
",
    ),
    (
        "TheApprentice",
        " The Apprentice
PARTNER, will you teach me to hit it like you do?
 I'd be honoured! Lesson one: relax.
 I can try. No promises.
 Some things can't be taught.
How do you keep so calm over a long putt?
 Breathe, look at the hole, and trust the line.
 Practice. Lots of it.
 I don't. I just look calm.
I tried your tip. The ball actually went straight!
 See? You're a natural!
 Beginner's luck, probably.
 Don't get used to it.
One day I'll beat you, you know.
 I'm counting on it. That's how I'll know I taught well.
 We'll see about that.
 Not in this lifetime.
",
    ),
    (
        "FirstDate",
        " First Date
So, PARTNER... is it strange that I asked you golfing?
 Not at all! Best first date idea ever.
 A little. But I'm having fun.
 It's strange. Very strange.
I should warn you, I'm terrible at bunkers.
 Then we'll get sandy together.
 Noted. I'll stand well back.
 I noticed. Twice.
You have a really nice swing, you know.
 Why thank you! Yours isn't bad either.
 It's the shoes.
 Please focus on the ball.
Dinner at the clubhouse after the round?
 I'd love that. I'll even let you pick dessert.
 Sure, if they have a table.
 I've already eaten.
",
    ),
    (
        "TimeCapsule",
        " The Time Capsule
PARTNER, the club is burying a time capsule!
 What fun! What should we put in it?
 Bit early to be nostalgic, isn't it?
 Who's ever going to dig it up?
I'm putting in my old scorecard. My best round ever.
 A treasure! Future golfers will be amazed.
 Is that the one with the eraser marks?
 Future golfers will laugh.
I wonder what golf will be like in fifty years.
 Hover carts! Self-raking bunkers!
 Probably the same. Grass, ball, hole.
 Robots will win everything.
There. Buried under the old oak. Our secret.
 Let's come back together in fifty years!
 I'll try to remember the spot.
 I've already forgotten where it is.
",
    ),
    (
        "TheGadget",
        " The Gadget
PARTNER, look at my new rangefinder! It talks!
 Ooh, shiny! What does it say?
 Does it hit the ball for you too?
 Gadgets are for people who can't golf.
It says: one hundred and forty yards, slight breeze.
 Precision! I'm impressed.
 My guess was one-forty too. For free.
 It's wrong. Obviously.
Huh. It just told me to take up tennis.
 Ha! It's got a sense of humour!
 Maybe it knows something.
 Finally, a gadget that talks sense.
I think I'll go back to trusting my eyes.
 Eyes and a good friend. Best tools in golf.
 Sensible.
 Took you a whole round to figure that out.
",
    ),
    (
        "ThePodcast",
        " The Podcast
PARTNER, I'm starting a podcast about golf!
 Amazing! What's it called?
 Another one? Well, good luck.
 Nobody listens to podcasts about golf.
It's called The Back Nine. Episode one: grip.
 Catchy! I'd subscribe!
 Grip. Gripping stuff.
 I'm gripped. With dread.
Would you be my first guest? Talk about your putting?
 Me? On air? I'd love to!
 Maybe. If I can keep my hat on.
 Absolutely not.
We had forty listeners! Forty! Mostly my aunts.
 Forty fans! That's how every legend starts!
 Aunts count, I suppose.
 That's thirty-nine aunts too many.
",
    ),
];

/// A character file: the roster record (job, name, traits, looks, head), the twenty dialogue slots (the Customise rows:
/// signature, good shot, too easy, hard, really hard, bad, worse, almost hit, tree, water, scenic, ugly, animal, long wait,
/// thirsty, found a drink, hungry, found food, tired, found a bench; roster::SAYING_LABEL_KEYS), and for a .pro the skills.
fn character(low: &str) -> Option<Vec<u8>> {
    struct C {
        file: &'static str,
        job: &'static str,
        name: &'static str,
        female: bool,
        head: u8,
        traits: u8,
        bio: &'static str,
        say: [&'static str; 20],
    }
    const ALL: [C; 11] = [
        C {
            file: "joe pro.glf",
            job: "Golf Pro",
            name: "Sam Club",
            female: false,
            head: 8,
            traits: 0x11,
            bio: "The club pro. Has given four thousand lessons and still says 'keep your head down' like it's breaking news. Owns eleven visors.",
            say: [
                "Head down, hopes up, wallet open!",
                "Textbook. Somebody write that down.",
                "I could play this hole with a broom. And have.",
                "Ah, a shot with some character.",
                "Now THIS is why they pay me.",
                "I meant to do that. For teaching purposes.",
                "Let's call that a teaching moment.",
                "Fore means FORE, folks!",
                "That tree has been in my lessons for years.",
                "Lesson nine: balls do not float. Write it down.",
                "Now that's a view worth a bogey.",
                "Who approved THAT? Not me. Check the minutes.",
                "Hello, little fella. Mind the green.",
                "In my day we played through. Politely.",
                "A pro needs fluids. Professionally.",
                "Ah. Hydrated and dangerous.",
                "I could eat a whole bag of tees.",
                "Fuel for the back nine. Perfect.",
                "My legs are telling me about my age.",
                "A bench! The unsung hero of golf.",
            ],
        },
        C {
            file: "i.m.picky.glf",
            job: "Inspector",
            name: "Iris Fussworth",
            female: true,
            head: 3,
            traits: 0x02,
            bio: "Course inspector. Carries a ruler, a magnifying glass and a notebook titled 'Concerns, Volume 7'. Has never given ten out of ten.",
            say: [
                "I'll be making a note of that.",
                "Acceptable. Barely, but acceptable.",
                "This hole needs more challenge. Noted.",
                "Hmm. A fair test. I'll allow it.",
                "Whoever designed this owes me a nap.",
                "The wind was clearly out of regulation.",
                "That shot will not appear in my report.",
                "Fore! And also: unacceptable.",
                "That tree is three inches out of place.",
                "Water: wet. As predicted in my report.",
                "Oh. Oh, that's rather lovely. Noted.",
                "An eyesore. Underlined. Twice. In red.",
                "Is that animal on the approved list?",
                "Twelve minutes at a tee. Noted. Twice.",
                "A drink would improve my mood. Slightly.",
                "Adequately refreshing. Three stars.",
                "Is there food on this course? At all?",
                "Edible. I'll be generous: four stars.",
                "My feet would like to file a complaint.",
                "A bench. Correctly placed. Remarkable.",
            ],
        },
        C {
            file: "ivana richman.glf",
            job: "Investor",
            name: "Goldie Vault",
            female: true,
            head: 6,
            traits: 0x04,
            bio: "Buys land, loves views, keeps a diamond divot tool for emergencies. Thinks 'par' is a type of champagne.",
            say: [
                "Darling, I could buy this hole.",
                "Divine. Like a good investment.",
                "Is this a beginner's hole? How quaint.",
                "Ooh, a challenge. How thrilling!",
                "I'll need my lucky diamond tee for this.",
                "My caddie will hear about this.",
                "Let's never speak of that shot again.",
                "Careful! This visor is couture!",
                "Who put a tree in my way? Buy it. Move it.",
                "Splash. That ball was monogrammed, darling.",
                "What a view! I'll take two.",
                "Hideous. Gift-wrap it and send it to a rival.",
                "Aww! Can I adopt it?",
                "Waiting? I don't do waiting.",
                "Something sparkling, please. Anything.",
                "Bubbles! Now we're talking.",
                "I could nibble a canape or nine.",
                "Mmm, simply scrumptious.",
                "These shoes were not made for walking.",
                "A bench! How delightfully rustic.",
            ],
        },
        C {
            file: "j.p.bigdome.glf",
            job: "Tycoon",
            name: "Monty Banks",
            female: false,
            head: 11,
            traits: 0x08,
            bio: "Owns half the town and is negotiating for the other half. Tips caddies in shares. Calls every hole 'an opportunity'.",
            say: [
                "Time is money. So is golf.",
                "Ka-ching! That's how it's done.",
                "Easy. Like a hostile takeover.",
                "A risky play. I like risk.",
                "Nobody makes this shot. Watch me.",
                "I'll have my lawyers look at that.",
                "That shot has been downsized.",
                "Do you know who I am? Duck!",
                "I'm acquiring that tree. Then firing it.",
                "Liquid assets. Extremely liquid.",
                "Now that's prime real estate.",
                "That thing is bad for business.",
                "Is that critter on the payroll?",
                "Waiting costs me money, you know.",
                "Get me a drink. Bill it to someone.",
                "Refreshing. Put it on my tab.",
                "Lunch meeting, anyone? I'm starving.",
                "A fine deal on a sandwich.",
                "Even tycoons need a sit-down.",
                "A bench. I'll buy the next one.",
            ],
        },
        C {
            file: "ada birdie.chr",
            job: "Teacher",
            name: "Ada Birdie",
            female: true,
            head: 1,
            traits: 0x01,
            bio: "Teaches maths on weekdays and golf on Saturdays. Grades every hole out of ten and the clubhouse sandwiches out of five.",
            say: [
                "That's a gold star for the course!",
                "A+! Top of the class!",
                "Too easy. Let's add some homework.",
                "Show your working on this one...",
                "This hole is the final exam.",
                "Hmm. A C minus, at best.",
                "See me after the round.",
                "Eyes on your own ball, please!",
                "That tree is getting detention. And a note home.",
                "Splash! Points off. And for spelling.",
                "Gorgeous! Ten out of ten!",
                "That needs a red pen. Lots of red pen.",
                "Look! Nature study!",
                "Is it recess yet? This line is long.",
                "Water break, everyone!",
                "Ah, much better. Class resumes.",
                "My tummy's ringing the lunch bell.",
                "Yum! Best packed lunch ever.",
                "Long day. Need a sit-down.",
                "A bench! The quiet corner!",
            ],
        },
        C {
            file: "bruno sandwedge.chr",
            job: "Chef",
            name: "Bruno Sandwedge",
            female: false,
            head: 4,
            traits: 0x10,
            bio: "Head chef at a famous bistro. Finds every bunker, then rates the sand for texture. Brings his own sauce to the snack bar.",
            say: [
                "Golf is like soup. Needs patience.",
                "Magnifique! Chef's kiss!",
                "Too easy. Where is the spice?",
                "A tricky recipe, this one.",
                "This hole is a souffle. Do not rush.",
                "Overcooked. Completely overcooked.",
                "That one goes back to the kitchen.",
                "Whoa! Nearly a flambe!",
                "The tree ate my ball. Rude, but well seasoned.",
                "Into the soup! Needs croutons.",
                "Beautiful. Like a fresh baguette.",
                "That is not a garnish, that is ugly.",
                "Hello, little friend. You look... tasty.",
                "This queue is slower than a stew.",
                "I need a drink. A tall one.",
                "Ahh. Refreshing. A little sweet.",
                "My stomach is growling in French.",
                "Not bad! Needs salt, but not bad.",
                "My feet are simmering.",
                "Ah! A bench! Like a good sauce, it rests.",
            ],
        },
        C {
            file: "cora fairweather.chr",
            job: "Pilot",
            name: "Cora Fairweather",
            female: true,
            head: 9,
            traits: 0x06,
            bio: "Airline pilot. Only plays in sunshine, reads the wind like a weather report and announces every shot over an imaginary intercom.",
            say: [
                "Clear skies, light breeze. Perfect!",
                "Smooth landing! Right on the runway!",
                "Easy. Like autopilot.",
                "Crosswind. Adjusting the approach.",
                "Turbulence ahead. Hold on to your caps!",
                "Mayday, mayday!",
                "We seem to have lost an engine.",
                "Incoming! Brace, brace!",
                "Bird strike! Well, tree strike.",
                "Ladies and gentlemen, we have landed. In water.",
                "Wow. What a view from up here.",
                "That's not on any map I want.",
                "Look! Wildlife on the runway!",
                "We're in a holding pattern. Again. Snacks?",
                "Cabin crew, drinks please!",
                "Ahh, that's first class.",
                "Is there an in-flight meal?",
                "Delicious. Better than airline food.",
                "I'm due for a layover.",
                "A bench. Seatbelt sign off!",
            ],
        },
        C {
            file: "dev longdrive.chr",
            job: "Engineer",
            name: "Dev Longdrive",
            female: false,
            head: 14,
            traits: 0x11,
            bio: "Engineer. Hits it long, not always straight. Has a spreadsheet for his slice and a second spreadsheet explaining the first.",
            say: [
                "Velocity times angle. Easy.",
                "Calculations confirmed!",
                "Too easy. Needs a harder problem.",
                "Tricky physics on this one.",
                "Okay. This needs a whiteboard.",
                "There's a bug in my swing.",
                "That's a critical error.",
                "Whoa! Unexpected trajectory!",
                "Tree collision. Logging the incident.",
                "Test result: ball is not waterproof.",
                "Elegant design! Very efficient.",
                "Who signed off on that? Unacceptable tolerances.",
                "A squirrel! Fascinating mechanics.",
                "This queue needs optimising.",
                "Low on coolant. Need a drink.",
                "System rehydrated. Back online.",
                "Running on empty. Need snacks.",
                "Refuelled! Performance restored.",
                "Battery at five percent.",
                "Bench detected. Entering sleep mode.",
            ],
        },
        C {
            file: "elsa greenway.chr",
            job: "Gardener",
            name: "Elsa Greenway",
            female: true,
            head: 12,
            traits: 0x0a,
            bio: "Award-winning gardener. Admires every flower bed and talks to the trees. The trees are, frankly, terrible listeners.",
            say: [
                "Ooh, look at those petunias!",
                "Lovely! Like a rose in bloom.",
                "Easy as planting daisies.",
                "Mind the weeds on this one.",
                "This hole needs some serious pruning.",
                "Oh dear. Into the compost.",
                "That's what I call a weed.",
                "Careful! I'm not a shrub!",
                "Sorry, Mr Tree! Let's not tell Mrs Tree.",
                "At least the pond lilies are happy.",
                "Gorgeous! Who planted these?",
                "That needs a hedge in front of it.",
                "Hello, little gardener's friend!",
                "Like watching grass grow. I'd enjoy that more.",
                "I'm wilting! Water please!",
                "Ahh. Like rain on a dry lawn.",
                "I could eat a whole tomato patch.",
                "Delicious. Fresh from the garden?",
                "My back is begging for a break.",
                "A bench by the flowers! Bliss.",
            ],
        },
        C {
            file: "felix puttnam.chr",
            job: "Librarian",
            name: "Felix Puttnam",
            female: false,
            head: 2,
            traits: 0x02,
            bio: "Head librarian and a wizard on the greens. Shushes the crowd, the birds and once, memorably, a lawnmower.",
            say: [
                "Shh! Quiet on the green, please.",
                "A classic! Worth a reread.",
                "Too easy. A picture book of a hole.",
                "Hmm. A page-turner, this one.",
                "This chapter is very long indeed.",
                "That shot is overdue. Very overdue.",
                "I'm putting that one back on the shelf.",
                "Shh! Indoor voices! And indoor balls!",
                "The tree has borrowed my ball.",
                "That shot is now a soggy paperback.",
                "Splendid. Like a good first edition.",
                "Ghastly. Return it to the publisher.",
                "Oh! A visiting critter. Hello.",
                "This wait is longer than an epic.",
                "Parched as an old manuscript.",
                "Ahh. Most refreshing.",
                "Is there a snack in the reading room?",
                "Delightful. Five stars.",
                "Time for a bookmark and a rest.",
                "A bench! A reading nook at last.",
            ],
        },
        C {
            file: "robin fairway.pro",
            job: "Golf Pro",
            name: "Robin Fairway",
            female: false,
            head: 5,
            traits: 0x11,
            bio: "A steady championship player who has never once raised his voice. Not even at a bunker. Especially not at a bunker.",
            say: [
                "Fairways and greens, my friends.",
                "Right where I aimed it.",
                "Fun hole. Bring a wedge and a smile.",
                "A thinking golfer's shot.",
                "All right. Deep breath. Here we go.",
                "Happens to the best of us.",
                "Even I have to laugh at that one.",
                "Heads up, everyone!",
                "Tree: one. Robin: nil. Rematch at four.",
                "The fish are building a collection.",
                "What a setting for a game of golf.",
                "Hmm, someone should move that.",
                "Hello there. Keep off the line, please.",
                "Patience is part of the game.",
                "Time for some water.",
                "Ahh. Good as new.",
                "A banana would really help right now.",
                "That'll get me through the back nine.",
                "Long day. Good day, though.",
                "A bench with a view. Lovely.",
            ],
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
    for (slot, line) in c.say.iter().enumerate() {
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
    let t = super::theme_of(low);
    let theme = ["Parkland", "Desert", "Tropical", "Links"][t];
    let mut s = format!("Terrain descriptions\r\n\r\n{theme}\r\n\r\n");
    for k in keys {
        if k.eq_ignore_ascii_case("END") {
            continue;
        }
        s.push_str(&format!("*{k}\r\n{}\r\n\r\n", describe(t, k)));
    }
    s.push_str("*END\r\n");
    s.into_bytes()
}

/// A tile's description (the terrain tooltip wraps it in a narrow column: kept to about ninety characters).
fn describe(theme: usize, k: &str) -> &'static str {
    let k = k.to_lowercase();
    match (k.as_str(), theme) {
        ("tees", _) => "Flat, tidy and full of hope. Enjoy it: it's all downhill from here.",
        ("green", _) => "Grass cut with nail scissors. Whisper. Do not stand in anyone's line.",
        ("fairway", 1) => "A green ribbon through the sand. The only friendly ground for miles.",
        ("fairway", _) => "Neat striped grass. Golfers aim for it, then wonder why they never land on it.",
        ("firm fairway", _) => "Hard and fast. The ball runs, and runs, and keeps running. Wave goodbye.",
        ("sand trap", 1) => "A bunker in the desert. Somehow it is still the worst sand here.",
        ("sand trap", _) => "A beach with no sea and no fun. Balls check in; they rarely check out.",
        ("pot bunker", _) => "Small, deep and grumpy. Bring a ladder, a wedge and a therapist.",
        ("waste bunker", _) => "A big scruffy sandpit. Playable, if you don't mind looking scruffy too.",
        ("rough", 3) => "Wiry seaside grass. The ball sits down in it and sulks.",
        ("rough", _) => "Longer grass beside the fairway. Close, but no cigar.",
        ("deep rough", _) => "Grass so tall it has its own weather. Send a search party.",
        ("desert", _) => "Sand, stones and sunshine. Lovely postcard, terrible lie.",
        ("water", 2) => "A turquoise lagoon. The fish run a thriving golf ball business.",
        ("water", _) => "A pond with a bottomless appetite for brand new golf balls.",
        ("stream", _) => "A babbling brook. It babbles about your last shot. Loudly.",
        ("burn", _) => "A wee Scottish stream: small, sneaky and always exactly in the way.",
        ("ravine", _) => "A dry gully with no bottom worth finding. Fly over it. Bravely.",
        ("brush", _) => "Low, scratchy shrubs. Your ball is in there somewhere, laughing.",
        ("gorse", _) => "Pretty yellow flowers on furious little spikes. Look, don't touch.",
        ("rocks", _) => "Boulders and stones. Bounces go anywhere, mostly sideways, sometimes back.",
        ("cactus", _) => "Prickly and proud. Do not hug. Do not try to retrieve your ball.",
        ("joshua tree", _) => "A spiky desert tree that waves at passing shots. Mostly it catches them.",
        ("palm tree", _) => "A tall palm: shade for golfers, a goalpost for balls, coconuts for luck.",
        ("tropical bush", _) => "Lush leaves and bright flowers. A five-star resort for lost balls.",
        ("tropical tree", _) => "A big leafy tree, perfect for a nap in the shade and for stopping shots.",
        ("pine", _) | ("pine tree", _) => "A tall evergreen. Smells like a holiday, plays like a wall.",
        ("scots pine", _) => "A hardy pine bent sideways by the sea wind. Golfers know the feeling.",
        ("maple tree", _) => "A grand leafy maple. Gorgeous in autumn, sticky in spring, solid all year.",
        _ => "Trees frame the holes, give shade, and quietly collect the odd wild shot.",
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

    #[test]
    fn text_fits_the_game() {
        // stories: four scenes of an opener and three replies, lines short enough for a speech bubble
        // (the game reads a scene as an unindented opener and the indented replies after it, stories.rs)
        for (name, text) in STORIES {
            let mut scenes: Vec<(String, usize)> = Vec::new();
            for l in text.lines().skip(1).filter(|l| !l.is_empty()) {
                assert!(l.len() <= 60, "{name}: {l}");
                if l.starts_with(' ') {
                    scenes.last_mut().expect("a reply before any opener").1 += 1;
                } else {
                    scenes.push((l.to_string(), 0));
                }
            }
            assert_eq!(scenes.len(), 4, "{name}");
            assert!(scenes.iter().all(|s| s.1 == 3), "{name}");
            assert!(FILES.iter().any(|f| f.contains(name)), "{name} is not listed");
        }
        // sayings fit their 50-byte slots, names their 16 bytes
        for f in FILES.iter().filter(|f| f.ends_with(".chr") || f.ends_with(".glf") || f.ends_with(".pro")) {
            let b = make(f).unwrap();
            let p = crate::roster::Person::parse_file(&b);
            assert!(!p.name.is_empty() && p.name.len() < 16, "{f}");
            for k in 0..20 {
                let at = RECORD + k * SAYING;
                let line = &b[at..at + SAYING];
                assert!(line[0] != 0 && line[SAYING - 2] == 0, "{f} saying {k} is too long or empty");
            }
        }
        // terrain descriptions stay short for the tooltip
        for t in 0..4 {
            for k in ["Tees", "Joshua Tree", "Gorse", "Burn", "Tropical Bush", "Deep rough"] {
                assert!(describe(t, k).len() <= 90, "{k}");
            }
        }
    }
}
