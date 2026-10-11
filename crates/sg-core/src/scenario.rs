//! Scenario packs (docs/MODDING.md, section 5): `scenario.toml`, the goals of a game and their checks.
//!
//! A scenario names the property to start on (or a theme), the starting money and difficulty, optionally a saved game to
//! start from (the course of an exported scenario), and a list of goals: conditions on the number of holes, the cash, the
//! fun and skill ratings, the members and the course's rank, each with an optional deadline year. The game checks the goals
//! at each month start and at each year end; when every goal is met the scenario is won, when a deadline passes with its
//! goal unmet it is lost. Scripts (crate::script) can add goals of their own and complete or fail them.
use crate::data::Theme;
use serde::{Deserialize, Serialize};

/// The four difficulties, in the exe's order (0 easiest).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Difficulty {
    Easy,
    Moderate,
    Difficult,
    Impossible,
}

impl Difficulty {
    pub fn level(self) -> i32 {
        self as i32
    }
    pub fn from_level(l: i32) -> Difficulty {
        [Difficulty::Easy, Difficulty::Moderate, Difficulty::Difficult, Difficulty::Impossible][l.clamp(0, 3) as usize]
    }
}

/// The course ranks of economy::rank, as scenario.toml names them.
pub const RANK_IDS: [&str; 4] = ["municipal", "daily-fee", "country-club", "championship"];

/// The `[scenario]` table.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioDef {
    /// The scenario's name (none: the pack's name).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// What the player reads before choosing it.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// A properties.toml id, e.g. "las-vegas" (none: the first property of `theme`, or Monterey).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub property: Option<String>,
    /// parkland, desert, tropical or links: picks the property when none is named; must match the property's when one is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<Theme>,
    /// Cash at the start, in money (none: the normal start, the starting funds less the property's price).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_money: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub difficulty: Option<Difficulty>,
    /// A sandbox game (no money worries; the goals still count).
    #[serde(default, skip_serializing_if = "is_false")]
    pub sandbox: bool,
    /// A saved game in the pack (`.sgs`, as System Functions > Save writes it) to start from: its land, holes, buildings
    /// and date. The money and difficulty above, when given, replace the saved ones.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub course: Option<String>,
    /// Shown as a message when the game starts.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub intro: String,
    /// Shown when every goal is met.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub win_text: String,
    /// Shown when a goal's deadline passes unmet.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub lose_text: String,
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// What a goal asks for; every condition given must hold at once.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Conditions {
    /// At least this many open holes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub holes: Option<i32>,
    /// At least this much cash, in money.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cash: Option<i64>,
    /// A fun rating (the status bar's) of at least this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fun: Option<i32>,
    /// A skill rating (as the status bar shows it, e.g. 12.5) of at least this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill: Option<f64>,
    /// At least this many members.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub members: Option<i32>,
    /// The course's rank (the SGA class by number of holes) at least this: municipal, daily-fee, country-club, championship.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank: Option<String>,
}

impl Conditions {
    pub fn is_empty(&self) -> bool {
        *self == Conditions::default()
    }

    fn check(&self) -> Result<(), String> {
        if let Some(r) = &self.rank {
            if !RANK_IDS.contains(&r.as_str()) {
                return Err(format!("rank \"{r}\" is not one of {}", RANK_IDS.join(", ")));
            }
        }
        Ok(())
    }

    /// Whether the facts meet every condition.
    pub fn met(&self, f: &Facts) -> bool {
        let rank = self.rank.as_deref().and_then(|r| RANK_IDS.iter().position(|&x| x == r)).map(|r| r as i32);
        self.holes.is_none_or(|n| f.holes >= n)
            && self.cash.is_none_or(|c| f.cash >= c)
            && self.fun.is_none_or(|n| f.fun >= n)
            && self.skill.is_none_or(|s| f.skill as f64 >= (s * 100.0).round())
            && self.members.is_none_or(|n| f.members >= n)
            && rank.is_none_or(|r| f.rank >= r)
    }

    /// The conditions in words, e.g. "9 holes, $50,000 cash".
    pub fn describe(&self) -> String {
        let mut v = Vec::new();
        if let Some(n) = self.holes {
            v.push(format!("{n} holes"));
        }
        if let Some(c) = self.cash {
            v.push(format!("${} cash", crate::economy::money_digits(c)));
        }
        if let Some(n) = self.fun {
            v.push(format!("fun rating {n}"));
        }
        if let Some(s) = self.skill {
            v.push(format!("skill rating {s:.2}"));
        }
        if let Some(n) = self.members {
            v.push(format!("{n} members"));
        }
        if let Some(r) = &self.rank {
            let k = RANK_IDS.iter().position(|&x| x == r).unwrap_or(0);
            v.push(format!("{} rank", crate::economy::rank_name(k)));
        }
        v.join(", ")
    }
}

/// One `[[goal]]` of scenario.toml.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoalDef {
    /// For scripts (complete_goal); none: "goal-1", "goal-2", ...
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// What the goals report shows.
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub holes: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cash: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fun: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub members: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank: Option<String>,
    /// The goal must be met by the end of this year (e.g. 2003); none: any time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by_year: Option<i32>,
}

impl GoalDef {
    /// The goal's conditions (the fields of Conditions, which a goal table holds directly).
    pub fn when(&self) -> Conditions {
        Conditions { holes: self.holes, cash: self.cash, fun: self.fun, skill: self.skill, members: self.members, rank: self.rank.clone() }
    }
}

/// A whole scenario.toml.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub scenario: ScenarioDef,
    #[serde(default, rename = "goal", skip_serializing_if = "Vec::is_empty")]
    pub goals: Vec<GoalDef>,
}

impl Scenario {
    /// Reads and checks a scenario.toml.
    pub fn parse(text: &str) -> Result<Scenario, String> {
        let s: Scenario = toml::from_str(text).map_err(|e| format!("scenario.toml: {}", crate::data::toml_error(text, &e)))?;
        s.check().map_err(|e| format!("scenario.toml: {e}"))?;
        Ok(s)
    }

    fn check(&self) -> Result<(), String> {
        let d = &self.scenario;
        if let Some(p) = &d.property {
            let Some(i) = crate::data::property_index(p) else {
                return Err(format!("property \"{p}\" is not a property id (e.g. monterey, las-vegas, scotland)"));
            };
            if let Some(t) = d.theme {
                if crate::data::property_theme(i) != t {
                    return Err(format!("property \"{p}\" is not a {t:?} property").to_lowercase());
                }
            }
        }
        if let Some(m) = d.start_money {
            if !(-100_000_000..=100_000_000).contains(&m) {
                return Err(format!("start_money = {m}: out of range"));
            }
        }
        if let Some(c) = &d.course {
            if c.contains("..") || c.starts_with('/') || c.contains('\\') {
                return Err(format!("course = \"{c}\": give a path inside the pack, like \"course.sgs\""));
            }
        }
        let mut ids = std::collections::HashSet::new();
        for (k, g) in self.goals.iter().enumerate() {
            let id = g.id.clone().unwrap_or_else(|| format!("goal-{}", k + 1));
            if !ids.insert(id.clone()) {
                return Err(format!("two goals have the id \"{id}\""));
            }
            if g.text.trim().is_empty() {
                return Err(format!("goal {id}: text is empty"));
            }
            if g.when().is_empty() {
                return Err(format!("goal {id}: give at least one of holes, cash, fun, skill, members, rank"));
            }
            g.when().check().map_err(|e| format!("goal {id}: {e}"))?;
            if let Some(y) = g.by_year {
                if !(2001..=2200).contains(&y) {
                    return Err(format!("goal {id}: by_year = {y}: the game starts in 2001"));
                }
            }
        }
        Ok(())
    }

    /// The property slot to start on: the one named, else the first of the theme, else Monterey.
    pub fn property_index(&self) -> usize {
        let d = &self.scenario;
        if let Some(i) = d.property.as_deref().and_then(crate::data::property_index) {
            return i;
        }
        d.theme.and_then(|t| (0..16).find(|&i| crate::data::property_theme(i) == t)).unwrap_or(0)
    }

    /// The scenario as TOML (what "Export as scenario" writes).
    pub fn to_toml(&self) -> String {
        toml::to_string_pretty(self).unwrap_or_default()
    }

    /// The goals a game starts with.
    pub fn start_goals(&self, pack: &str, name: &str) -> Goals {
        Goals {
            scenario: Some(pack.to_string()),
            name: name.to_string(),
            win_text: self.scenario.win_text.clone(),
            lose_text: self.scenario.lose_text.clone(),
            list: self
                .goals
                .iter()
                .enumerate()
                .map(|(k, g)| Goal {
                    id: g.id.clone().unwrap_or_else(|| format!("goal-{}", k + 1)),
                    text: g.text.clone(),
                    when: g.when(),
                    by_year: g.by_year,
                    state: GoalState::Open,
                    owner: pack.to_string(),
                })
                .collect(),
            outcome: None,
        }
    }
}

// ---- the goals of a game ------------------------------------------------------------------------------------------------------

/// What the goal checks read of the game.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Facts {
    /// The calendar year (2001 is the first).
    pub year: i32,
    /// Cash in money.
    pub cash: i64,
    pub holes: i32,
    pub fun: i32,
    /// Skill rating in hundredths (as the club keeps it).
    pub skill: i32,
    pub members: i32,
    /// economy::rank: 0 Municipal .. 3 Championship.
    pub rank: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GoalState {
    Open,
    Done,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Goal {
    pub id: String,
    pub text: String,
    /// Empty for a script's goal: only the script completes it.
    #[serde(default)]
    pub when: Conditions,
    #[serde(default)]
    pub by_year: Option<i32>,
    pub state: GoalState,
    /// The pack that set it (the scenario's, or a script's).
    #[serde(default)]
    pub owner: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    Won,
    Lost,
}

/// The goals of a game (saved with it).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Goals {
    /// The scenario pack the game was started from.
    #[serde(default)]
    pub scenario: Option<String>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub win_text: String,
    #[serde(default)]
    pub lose_text: String,
    #[serde(default)]
    pub list: Vec<Goal>,
    #[serde(default)]
    pub outcome: Option<Outcome>,
}

/// Something the player is told about the goals.
#[derive(Clone, Debug, PartialEq)]
pub enum Note {
    Done(String),
    Failed(String),
    Won(String),
    Lost(String),
}

impl Note {
    pub fn text(&self) -> String {
        match self {
            Note::Done(t) => format!("Goal reached: {t}"),
            Note::Failed(t) => format!("Goal missed: {t}"),
            Note::Won(t) | Note::Lost(t) => t.clone(),
        }
    }
}

impl Goals {
    pub fn is_empty(&self) -> bool {
        self.list.is_empty() && self.scenario.is_none()
    }

    /// Checks the goals against the game. `year_ended` is the year that has just ended (at a year end), when the
    /// deadlines of that year are due. Returns what changed, in order.
    pub fn check(&mut self, f: &Facts, year_ended: Option<i32>) -> Vec<Note> {
        let mut notes = Vec::new();
        if self.outcome.is_some() {
            return notes;
        }
        for g in self.list.iter_mut().filter(|g| g.state == GoalState::Open) {
            if !g.when.is_empty() && g.when.met(f) {
                g.state = GoalState::Done;
                notes.push(Note::Done(g.text.clone()));
            } else if let (Some(by), Some(y)) = (g.by_year, year_ended) {
                if y >= by {
                    g.state = GoalState::Failed;
                    notes.push(Note::Failed(g.text.clone()));
                }
            }
        }
        notes.extend(self.settle());
        notes
    }

    /// A scenario is won when every goal is reached and lost when one is missed.
    fn settle(&mut self) -> Option<Note> {
        if self.scenario.is_none() || self.outcome.is_some() || self.list.is_empty() {
            return None;
        }
        if self.list.iter().any(|g| g.state == GoalState::Failed) {
            self.outcome = Some(Outcome::Lost);
            let t = if self.lose_text.is_empty() { "You did not reach the scenario's goals in time." } else { &self.lose_text };
            return Some(Note::Lost(t.to_string()));
        }
        if self.list.iter().all(|g| g.state == GoalState::Done) {
            self.outcome = Some(Outcome::Won);
            let t = if self.win_text.is_empty() { "Congratulations! You reached every goal of the scenario." } else { &self.win_text };
            return Some(Note::Won(t.to_string()));
        }
        None
    }

    /// A script's goal: added, or its text changed when the pack already has one with that id.
    pub fn set(&mut self, owner: &str, id: &str, text: &str) {
        match self.list.iter_mut().find(|g| g.owner == owner && g.id == id) {
            Some(g) => g.text = text.to_string(),
            None => self.list.push(Goal {
                id: id.to_string(),
                text: text.to_string(),
                when: Conditions::default(),
                by_year: None,
                state: GoalState::Open,
                owner: owner.to_string(),
            }),
        }
    }

    /// A script completes (`done`) or fails one of its goals, or a goal of the scenario its pack started. Returns the notes
    /// (none when the goal is unknown or already settled).
    pub fn finish(&mut self, owner: &str, id: &str, done: bool) -> Vec<Note> {
        let scenario = self.scenario.clone();
        let Some(g) = self
            .list
            .iter_mut()
            .find(|g| g.id == id && (g.owner == owner || scenario.as_deref() == Some(owner)) && g.state == GoalState::Open)
        else {
            return Vec::new();
        };
        g.state = if done { GoalState::Done } else { GoalState::Failed };
        let mut notes = vec![if done { Note::Done(g.text.clone()) } else { Note::Failed(g.text.clone()) }];
        notes.extend(self.settle());
        notes
    }
}

/// A pack.toml for an exported scenario.
pub fn export_manifest(id: &str, name: &str, author: &str) -> String {
    let m = crate::packs::Manifest {
        id: id.to_string(),
        name: name.to_string(),
        version: "1.0.0".into(),
        kind: crate::packs::Kind::Scenario,
        authors: if author.is_empty() { Vec::new() } else { vec![author.to_string()] },
        license: "CC-BY-SA-4.0".into(),
        description: format!("The course {name}, exported from OpenSGolf."),
        requires: vec!["original >= 1.0".into()],
        conflicts: Vec::new(),
    };
    #[derive(Serialize)]
    struct File<'a> {
        pack: &'a crate::packs::Manifest,
    }
    toml::to_string_pretty(&File { pack: &m }).unwrap_or_default()
}

/// A pack id made from a name ("My Course!" -> "my-course"), never empty.
pub fn id_from_name(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let out: String = out.trim_end_matches('-').chars().take(48).collect();
    let out = out.trim_end_matches('-').to_string();
    if out.is_empty() || out == crate::packs::ORIGINAL {
        "my-scenario".into()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = r#"
[scenario]
name = "Test"
property = "las-vegas"
start_money = 50000
difficulty = "moderate"
win_text = "Won!"

[[goal]]
text = "Build nine holes"
holes = 9

[[goal]]
id = "rich"
text = "Have $100,000 and a fun rating of 20 by 2003"
cash = 100000
fun = 20
by_year = 2003
"#;

    fn facts(holes: i32, cash: i64, fun: i32) -> Facts {
        Facts { year: 2001, cash, holes, fun, skill: 0, members: 0, rank: crate::economy::rank(holes as usize) }
    }

    #[test]
    fn parse_and_errors() {
        let s = Scenario::parse(EXAMPLE).unwrap();
        assert_eq!(s.property_index(), crate::data::property_index("las-vegas").unwrap());
        assert_eq!(s.scenario.difficulty, Some(Difficulty::Moderate));
        assert_eq!(s.goals.len(), 2);
        assert!(Scenario::parse("[scenario]\nproperty = \"atlantis\"\n").unwrap_err().contains("atlantis"));
        assert!(Scenario::parse("[scenario]\nproperty = \"las-vegas\"\ntheme = \"links\"\n").unwrap_err().contains("links"));
        assert!(Scenario::parse("[scenario]\n[[goal]]\ntext = \"x\"\n").unwrap_err().contains("at least one"));
        assert!(Scenario::parse("[scenario]\n[[goal]]\ntext = \"x\"\nrank = \"best\"\n").unwrap_err().contains("rank"));
        assert!(Scenario::parse("[scenario]\ncolour = 1\n").is_err());
        assert!(Scenario::parse("[scenario]\ncourse = \"../x.sgs\"\n").is_err());
        // a theme alone picks a property of that theme
        let s = Scenario::parse("[scenario]\ntheme = \"links\"\n").unwrap();
        assert_eq!(crate::data::property_theme(s.property_index()), Theme::Links);
        // the TOML written back reads the same
        let s = Scenario::parse(EXAMPLE).unwrap();
        assert_eq!(Scenario::parse(&s.to_toml()).unwrap(), s);
    }

    #[test]
    fn goals_are_reached_won_and_lost() {
        let s = Scenario::parse(EXAMPLE).unwrap();
        let mut g = s.start_goals("pack", "Test");
        assert!(g.check(&facts(3, 0, 0), None).is_empty());
        assert_eq!(g.check(&facts(9, 0, 0), None), vec![Note::Done("Build nine holes".into())]);
        let n = g.check(&facts(9, 200000, 25), None);
        assert_eq!(n.len(), 2);
        assert_eq!(n[1], Note::Won("Won!".into()));
        assert_eq!(g.outcome, Some(Outcome::Won));
        assert!(g.check(&facts(0, 0, 0), Some(2010)).is_empty(), "settled");

        // the deadline passes
        let mut g = s.start_goals("pack", "Test");
        assert!(g.check(&facts(9, 0, 0), Some(2002)).len() == 1);
        let n = g.check(&facts(9, 0, 0), Some(2003));
        assert!(matches!(n[0], Note::Failed(_)) && matches!(n[1], Note::Lost(_)));
        assert_eq!(g.outcome, Some(Outcome::Lost));
    }

    #[test]
    fn script_goals() {
        let mut g = Goals::default();
        g.set("bonus", "ace", "Make a hole in one");
        g.set("bonus", "ace", "Make a hole-in-one");
        assert_eq!(g.list.len(), 1);
        assert!(g.check(&facts(18, 1_000_000, 99), None).is_empty(), "a script's goal waits for the script");
        assert!(g.finish("other", "ace", true).is_empty(), "another pack cannot complete it");
        assert_eq!(g.finish("bonus", "ace", true), vec![Note::Done("Make a hole-in-one".into())]);
        assert_eq!(g.outcome, None, "no scenario: nothing to win");
    }

    #[test]
    fn export_files_read_back() {
        let id = id_from_name("Sunny Pines G.C.!");
        assert_eq!(id, "sunny-pines-g-c");
        assert_eq!(id_from_name("***"), "my-scenario");
        let m = crate::packs::Manifest::parse(&export_manifest(&id, "Sunny Pines", "Me")).unwrap();
        assert_eq!(m.kind, crate::packs::Kind::Scenario);
    }

    #[test]
    fn the_example_scenario_parses() {
        let m = crate::packs::Manifest::parse(include_str!("../../../packs/example-scenario-desert-challenge/pack.toml")).unwrap();
        assert_eq!(m.kind, crate::packs::Kind::Scenario);
        let s = Scenario::parse(include_str!("../../../packs/example-scenario-desert-challenge/scenario.toml")).unwrap();
        assert_eq!(crate::data::property_theme(s.property_index()), Theme::Desert);
        assert!(!s.goals.is_empty());
    }
}
