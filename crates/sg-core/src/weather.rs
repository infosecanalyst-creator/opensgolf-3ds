//! The weather (the port's own; the original has none): which days rain, which rain falls as snow in a late-winter cold
//! snap, which heavy rains are thunderstorms and which desert summer days are heat waves. Everything is a hash of the
//! course (its name and size), its place and the day, never the game's random generator, so the Enhanced graphics' rain,
//! after-the-rain, lightning, winter and heat looks (crates/simgolf fx_light.rs, fx_wet.rs, fx_winter.rs) and the
//! "Weather that matters" gameplay extra (`Today`, sg_core::extras) see the same days.
//!
//! The hashes and the climates are the ones the Enhanced graphics had before this module (moved here unchanged, so every
//! day looks as it did).

/// A day of weather in game ticks: the Enhanced graphics' day of light at its normal length (three game months,
/// fx_light.rs DAY_MONTHS). The simulation's days are always these; the light's day follows the player's day length.
pub const DAY_TICKS: u32 = 3 * 1024;

/// The render-side hash (FNV-1a over the words, then mixed): fx_light.rs's.
pub fn hash(words: &[u32]) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for w in words {
        for b in w.to_le_bytes() {
            h ^= b as u32;
            h = h.wrapping_mul(0x0100_0193);
        }
    }
    h ^= h >> 16;
    h = h.wrapping_mul(0x7feb_352d);
    h ^= h >> 15;
    h
}

/// A hash to 0..1 of a hash.
pub fn unit(h: u32) -> f32 {
    (h >> 8) as f32 / (1u32 << 24) as f32
}

/// The winter's two-word hash to 0..1 (fx_winter.rs's).
pub fn hash2(a: u32, b: u32) -> f32 {
    let mut h = a.wrapping_mul(0x9e37_79b9) ^ b.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    (h >> 8) as f32 / (1u32 << 24) as f32
}

pub fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A property's weather: how many days in a hundred it rains, and how likely a year is to end its winter with a snowy cold
/// snap in early March. By the place on the world map (land::RECORDS order); a property a mod adds takes its theme's.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Climate {
    pub rain: f32,
    pub snow: f32,
}

const fn cl(rain: f32, snow: f32) -> Climate {
    Climate { rain, snow }
}

/// Monterey, San Diego, Rocky Mtns., Las Vegas, Phoenix, Hawaii, Oahu, Nova Scotia, Northeast, Carolina, Ireland, Scotland,
/// Wales, Spain, Florida, Jamaica. Deserts and the tropics never see snow; the desert's rain is a rare shower.
pub const CLIMATES: [Climate; 16] = [
    cl(0.20, 0.0),
    cl(0.12, 0.0),
    cl(0.25, 0.7),
    cl(0.04, 0.0),
    cl(0.05, 0.0),
    cl(0.30, 0.0),
    cl(0.30, 0.0),
    cl(0.35, 0.7),
    cl(0.30, 0.6),
    cl(0.28, 0.15),
    cl(0.45, 0.2),
    cl(0.45, 0.4),
    cl(0.45, 0.25),
    cl(0.12, 0.05),
    cl(0.33, 0.0),
    cl(0.28, 0.0),
];

/// By theme (the exe's order: parkland, desert, tropical, links), for a property not in the table.
pub const THEME_CLIMATES: [Climate; 4] = [cl(0.25, 0.3), cl(0.04, 0.0), cl(0.30, 0.0), cl(0.40, 0.3)];

/// The climate of a course: its property's (`property`, the world map slot, when it has one), else its theme's; never snow
/// in the desert or the tropics, whatever a property says.
pub fn climate(property: Option<usize>, theme: u8) -> Climate {
    let theme = theme.min(3) as usize;
    let c = property.and_then(|p| CLIMATES.get(p)).copied().unwrap_or(THEME_CLIMATES[theme]);
    if theme == 1 || theme == 2 {
        Climate { snow: 0.0, ..c }
    } else {
        c
    }
}

/// How wintry month `m` (3.0 the start of March .. 11.0 the end of October) is: 1 in the first week of March, nothing by
/// the end of the month's first half (a late cold snap; late October is cold mornings, not snow).
pub fn cold(m: f32) -> f32 {
    1.0 - smoothstep(3.15, 3.6, m)
}

/// How much of a desert summer month `m` is (0 in spring and autumn, 1 from mid June to mid August).
pub fn summer(m: f32) -> f32 {
    smoothstep(5.3, 6.5, m) * (1.0 - smoothstep(8.5, 9.6, m))
}

/// The calendar month of a date (3.0 the start of March .. 11.0 the end of October).
pub fn month_of(date: f32) -> f32 {
    3.0 + (date / 1024.0).rem_euclid(8.0)
}

/// The game year of a date (0 the first): March to October is one year.
pub fn year_of(date: u32) -> u32 {
    date / (1024 * 8)
}

/// Where a course is, for its weather: the seed of its hashes, its climate and its theme. The game sets it before each
/// tick (it is not saved: it follows from the course).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Place {
    pub seed: u32,
    pub climate: Climate,
    /// The exe theme (0 parkland, 1 desert, 2 tropical, 3 links).
    pub theme: u8,
    /// Test hooks (`SG_FX_RAIN`, `SG_FX_HEAT`): every day this wet (or this hot).
    pub force_rain: Option<f32>,
    pub force_heat: Option<f32>,
}

/// The salts of the hashes.
const RAIN: u32 = 0x7261_696e;
const STORM: u32 = 0x7374_6f72;
const HEAT: u32 = 0x6865_6174;

impl Place {
    pub fn new(name: &str, w: u32, h: u32, property: Option<usize>, theme: u8) -> Place {
        Place { seed: Self::seed_of(name, w, h), climate: climate(property, theme), theme, force_rain: None, force_heat: None }
    }

    /// The seed of the weather: the course's name and size, so a save keeps its weather.
    pub fn seed_of(name: &str, w: u32, h: u32) -> u32 {
        let n = name.bytes().fold(0u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32));
        hash(&[n, w, h])
    }

    /// How wet year `year` is against the place's usual: from half as many rainy days to half as many again.
    pub fn wet_year(&self, year: u32) -> f32 {
        0.5 + hash2(self.seed ^ 0x7e7, year)
    }

    /// Whether year `year` ends its winter with a snowy cold snap (the place's chance, a fresh roll each year).
    pub fn snowy_year(&self, year: u32) -> bool {
        hash2(self.seed ^ 0x5e0, year) < self.climate.snow
    }

    /// How hard it rains on day `day` (0 dry, else 0.6..1) in year `year`: about one day in four on parkland, more on the
    /// links, rarely in the desert.
    pub fn rain_on(&self, day: u32, year: u32) -> f32 {
        if let Some(v) = self.force_rain {
            return v.clamp(0.0, 1.0);
        }
        let chance = self.climate.rain * self.wet_year(year);
        let h = hash(&[self.seed, day, RAIN]);
        if unit(h) < chance {
            0.6 + 0.4 * unit(hash(&[h]))
        } else {
            0.0
        }
    }

    /// Whether heavy rain can bring lightning here (the Enhanced lightning's rule): rain of 0.7 or more where the climate
    /// is wet, never in the desert.
    pub fn stormy(&self, rain: f32) -> bool {
        rain >= 0.7 && self.theme != 1 && self.climate.rain >= 0.2
    }

    /// Whether day `day` is a thunderstorm day: a heavy-rain day (`stormy`) whose own roll is under `percent`.
    pub fn storm_on(&self, day: u32, year: u32, percent: i32) -> bool {
        self.stormy(self.rain_on(day, year)) && unit(hash(&[self.seed, day, STORM])) * 100.0 < percent as f32
    }

    /// Where in its day a storm day's storm breaks (0.2 .. 0.7 of the day).
    pub fn storm_at(&self, day: u32) -> f32 {
        0.2 + 0.5 * unit(hash(&[self.seed, day, STORM, 1]))
    }

    /// How hot day `day` is in month `m` (0 none .. 1): desert courses only, on about `percent` in a hundred days, in the
    /// summer months, never on a rainy day.
    pub fn heat_on(&self, day: u32, year: u32, m: f32, percent: i32) -> f32 {
        if let Some(v) = self.force_heat {
            return v.clamp(0.0, 1.0);
        }
        if self.theme != 1 || self.rain_on(day, year) > 0.0 {
            return 0.0;
        }
        let h = hash(&[self.seed, day, HEAT]);
        if unit(h) * 100.0 < percent as f32 {
            (0.7 + 0.3 * unit(hash(&[h]))) * summer(m)
        } else {
            0.0
        }
    }

    /// Whether a snowy morning's dusting lies on day `day` (fx_winter.rs): about one cold day in three.
    pub fn dusting(&self, day: u32) -> bool {
        hash2(self.seed ^ 0x5a0, day) < 0.35
    }

    /// How deep a dusting lies at phase `phase` of its day (0..1, as the light's: 0 dawn): from dawn, gone by mid-morning.
    pub fn dusting_depth(&self, day: u32, k: f32, phase: f32) -> f32 {
        let morning = if phase > 0.95 { smoothstep(0.95, 1.0, phase) } else { 1.0 - smoothstep(0.06, 0.22, phase) };
        (k * morning * (0.35 + 0.25 * hash2(self.seed, day))).clamp(0.0, 0.6)
    }

    /// The weather at game tick `tick` (date `date`, sg_core::extras::Calendar), for the Weather that matters extra.
    pub fn today(&self, tick: u32, date: u32, w: &crate::data::WeatherDef) -> Today {
        let day = tick / DAY_TICKS;
        let f = (tick % DAY_TICKS) as f32 / DAY_TICKS as f32;
        let year = year_of(date);
        let m = month_of(date as f32);
        let r = self.rain_on(day, year);
        let yesterday = if day > 0 { self.rain_on(day - 1, year) } else { 0.0 };
        // the late-winter cold snap: its rain falls as snow and the ground is not wet (fx_winter.rs)
        let k = if self.snowy_year(year) { cold(m) } else { 0.0 };
        let mut t = Today { day, rain: r * (1.0 - k), snow: r * k, ..Today::default() };
        // the ground wet while it rains and drying through the next day (fx_wet.rs's wet ground)
        if r > 0.0 {
            t.wet = r * (0.6 + 0.4 * smoothstep(0.0, 0.25, f)) * (1.0 - k);
        } else if yesterday > 0.0 {
            t.wet = yesterday * (1.0 - smoothstep(0.0, 0.7, f)) * (1.0 - k);
            t.lush = t.wet;
        }
        // the snow on the ground: a morning dusting (at the light's phase: a new game starts in the morning, fx_light.rs
        // DAY_START), or snow falling
        if k > 0.0 && self.dusting(day) {
            t.lying = self.dusting_depth(day, k, (f + 0.12).rem_euclid(1.0)) / 0.6;
        }
        t.lying = t.lying.max(t.snow);
        t.heat = self.heat_on(day, year, m, w.heat_percent);
        t.midday = smoothstep(0.25, 0.4, f) * (1.0 - smoothstep(0.65, 0.8, f));
        if self.storm_on(day, year, w.storm_percent) {
            t.storm_day = true;
            let start = (self.storm_at(day) * DAY_TICKS as f32) as u32;
            let at = tick % DAY_TICKS;
            t.storm = at >= start && at < start + w.storm_ticks.max(0) as u32;
            t.storm_start = at == start;
            t.storm_end = at == start + w.storm_ticks.max(0) as u32;
        }
        t
    }
}

/// The weather of one game tick, as the Weather that matters extra plays it (`Place::today`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Today {
    /// The weather day (`DAY_TICKS`).
    pub day: u32,
    /// Rain and snow falling (0 none .. 1).
    pub rain: f32,
    pub snow: f32,
    /// The ground wet (0 dry .. 1), and the part of it that is the day after rain (soft, lush greens).
    pub wet: f32,
    pub lush: f32,
    /// Snow lying on the ground (0 none .. 1).
    pub lying: f32,
    /// A desert heat wave (0 none .. 1), and how much of the day's middle it is now (0..1).
    pub heat: f32,
    pub midday: f32,
    /// A thunderstorm day; the storm is on now (play waits); it broke or passed this very tick.
    pub storm_day: bool,
    pub storm: bool,
    pub storm_start: bool,
    pub storm_end: bool,
}

/// What the morning's ticker says of a day (the text key), if anything.
pub fn forecast_key(t: &Today) -> Option<&'static str> {
    if t.storm_day {
        Some("STR_WEATHER_STORM_TODAY")
    } else if t.snow > 0.0 && t.snow >= t.rain {
        Some("STR_WEATHER_SNOW_TODAY")
    } else if t.rain > 0.0 {
        Some("STR_WEATHER_RAIN_TODAY")
    } else if t.heat > 0.0 {
        Some("STR_WEATHER_HEAT_TODAY")
    } else {
        None
    }
}

impl Today {
    /// The ball's extra drag on tee, fairway and green ground, per mille of its speed each tick (negative: it runs on):
    /// wet ground and snow hold it, a heat wave's baked ground lets it run.
    pub fn drag(&self, w: &crate::data::WeatherDef) -> i32 {
        let snow = self.lying.min(1.0);
        let wet = self.wet.min(1.0) * (1.0 - snow);
        let d = w.rain_drag as f32 * wet + w.snow_drag as f32 * snow + w.heat_drag as f32 * self.heat;
        d.round() as i32
    }

    /// How hard a ball bounces on soft ground the day after rain, in percent (100: as ever).
    pub fn bounce_percent(&self, w: &crate::data::WeatherDef) -> i32 {
        if self.lush > 0.0 {
            100 - ((100 - w.lush_bounce_percent) as f32 * self.lush.min(1.0)).round() as i32
        } else {
            100
        }
    }

    /// The share of the time new golfers keep coming, in percent: fewer in rain and snow and at a heat wave's midday,
    /// none while a storm is on.
    pub fn turnout(&self, w: &crate::data::WeatherDef) -> i32 {
        if self.storm {
            return 0;
        }
        let mut t = 100.0;
        let less = |pct: i32, k: f32| 1.0 - (100 - pct) as f32 / 100.0 * k.min(1.0);
        t *= less(w.rain_turnout_percent, self.rain);
        t *= less(w.snow_turnout_percent, self.snow.max(self.lying));
        t *= less(w.heat_turnout_percent, self.heat * self.midday);
        t.round() as i32
    }

    /// Whether the day is bad weather for a golfer's mood (rain, snow or heat).
    pub fn foul(&self) -> bool {
        self.rain > 0.0 || self.snow > 0.0 || self.lying > 0.0 || self.heat > 0.0
    }
}

impl crate::golfer::Club {
    /// The Weather that matters extra, before the golfers' tick: this tick's weather (`today`), the drag and bounce the
    /// ball and the shot planner use, the morning's forecast on the ticker, and a storm breaking or passing. With the extra
    /// switched off the weather is cleared and the planner's numbers go back to 0.
    pub(crate) fn weather_tick(&mut self, c: &crate::course::Course) {
        let (drag, cut) = if self.extras.weather {
            let w = crate::data::economy().weather;
            let t = self.place.today(self.tick, self.date(), &w);
            self.today = Some(t);
            if self.announced != Some(t.day) {
                self.announced = Some(t.day);
                if let Some(k) = forecast_key(&t) {
                    self.message(crate::lang::tr(k).into());
                }
            }
            if t.storm_start {
                self.message(crate::lang::tr("STR_WEATHER_STORM_BREAKS").into());
                // everyone at the club shelters in the clubhouse a while, and buys a drink or a bite there
                let n = self.g.iter().take(crate::golfer::SLOTS).filter(|g| g.hole != 0).count() as i32;
                let units = n * w.storm_spend / 100;
                if units > 0 {
                    let at = (c.door.0 * crate::geom::UNIT + 512, c.door.1 * crate::geom::UNIT + 512);
                    self.earn(units, crate::golfer::Column::FoodDrink, at);
                }
            }
            if t.storm_end {
                self.message(crate::lang::tr("STR_WEATHER_STORM_PASSES").into());
            }
            (t.drag(&w), 100 - t.bounce_percent(&w))
        } else {
            self.today = None;
            self.announced = None;
            (0, 0)
        };
        if drag != self.planner.drag {
            // the putts' launch speeds cached for the old green go
            self.planner.drag = drag;
            self.planner.cache.clear();
            self.planner.cache_at = 0;
        }
        self.planner.bounce_cut = cut;
    }

    /// A loaded game's weather as the last tick left it (the port's own): `today`, the day already announced and the drag
    /// and bounce the planner uses are not saved, so without this the first tick after a load announced the forecast
    /// again and, the drag having gone back to 0, cleared the planner's cache the uninterrupted game keeps. The game sets
    /// `place` first. Nothing changes with the extra off.
    pub fn weather_restore(&mut self) {
        if !self.extras.weather {
            return;
        }
        let w = crate::data::economy().weather;
        let t = self.place.today(self.tick, self.date(), &w);
        self.today = Some(t);
        self.announced = Some(t.day);
        self.planner.drag = t.drag(&w);
        self.planner.bounce_cut = 100 - t.bounce_percent(&w);
    }

    /// Whether new golfers come this tick under the weather (`Today::turnout`): the time is cut in blocks of 64 game ticks
    /// (about six seconds), and a hash of the block keeps that share of them open.
    pub(crate) fn weather_turnout(&self) -> bool {
        let Some(t) = self.today else { return true };
        let w = crate::data::economy().weather;
        let pct = t.turnout(&w);
        pct >= 100 || unit(hash(&[self.place.seed, self.tick / 64, 0x7475_726e])) * 100.0 < pct as f32
    }

    /// A golfer teeing off in foul weather: some start a point less happy (no umbrella), and in the snow a few who love
    /// it start a point happier. A hash of the golfer and the tick decides, not the game's generator.
    pub(crate) fn weather_mood(&mut self, g: usize) {
        let Some(t) = self.today.filter(|t| t.foul()) else { return };
        let w = crate::data::economy().weather;
        let roll = unit(hash(&[self.place.seed, self.tick, g as u32, 0x6d6f_6f64])) * 100.0;
        let below = |pct: i32| roll < pct as f32;
        let d = if t.snow > 0.0 || t.lying > 0.0 {
            if below(w.snow_lover_percent) {
                1
            } else if below(w.snow_lover_percent + w.snow_mood_percent) {
                -1
            } else {
                0
            }
        } else if t.rain > 0.0 {
            -(below(w.rain_mood_percent) as i32)
        } else {
            -((roll < w.heat_mood_percent as f32 * t.heat) as i32)
        };
        // Freeze golfer happiness keeps the weather from lowering a mood too
        if d == 0 || (d < 0 && self.extras.freeze_moods) {
            return;
        }
        let gg = &mut self.g[g];
        gg.mood = (gg.mood + d).clamp(-10, 10);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w() -> crate::data::WeatherDef {
        crate::data::WeatherDef::default()
    }

    #[test]
    fn the_hashes_are_the_graphics_ones() {
        // fixed values (the Enhanced graphics' hash before it moved here): the rainy days must not move
        assert_eq!(hash(&[0x1234_5678, 7, 0x7261_696e]), 0x5052_d9d0);
        assert_eq!(hash2(3, 4), hash2(3, 4));
        assert_eq!(Place::seed_of("Abc", 50, 50), hash(&[(65 * 31 * 31 + 98 * 31 + 99) as u32, 50, 50]));
        assert_eq!(cold(3.0), 1.0);
        assert_eq!(cold(6.0), 0.0);
        assert_eq!(climate(None, 1).rain, 0.04);
        assert_eq!(climate(Some(2), 2).snow, 0.0, "never snow in the tropics");
    }

    #[test]
    fn rain_comes_on_about_its_share_of_days() {
        let p = Place::new("Rainy Glen", 50, 50, Some(10), 3);
        let wet = (0..4000).filter(|&d| p.rain_on(d, d / 3) > 0.0).count();
        // Ireland: 45 in a hundred days, wetter and drier years averaging out
        assert!((1400..2200).contains(&wet), "{wet}");
        let desert = Place::new("Dry Gulch", 50, 50, Some(3), 1);
        assert!((0..4000).filter(|&d| desert.rain_on(d, d / 3) > 0.0).count() < 400);
    }

    #[test]
    fn storms_fall_on_heavy_rain_days_only_and_heat_in_desert_summers() {
        let p = Place::new("Storm Point", 50, 50, Some(11), 3);
        let mut storms = 0;
        for d in 0..2000 {
            if p.storm_on(d, d / 3, w().storm_percent) {
                storms += 1;
                assert!(p.stormy(p.rain_on(d, d / 3)));
            }
        }
        assert!(storms > 50, "{storms}");
        let none = Place::new("Storm Point", 50, 50, Some(11), 3);
        assert_eq!((0..2000).filter(|&d| none.storm_on(d, d / 3, 0)).count(), 0);
        // heat: desert only, summer only
        let desert = Place::new("Dry Gulch", 50, 50, Some(3), 1);
        let hot = (0..2000).filter(|&d| desert.heat_on(d, 0, 7.0, w().heat_percent) > 0.0).count();
        assert!((400..1300).contains(&hot), "{hot}");
        assert_eq!((0..2000).filter(|&d| desert.heat_on(d, 0, 4.0, w().heat_percent) > 0.0).count(), 0, "not in April");
        let park = Place::new("Dry Gulch", 50, 50, Some(3), 0);
        assert_eq!((0..2000).filter(|&d| park.heat_on(d, 0, 7.0, 100) > 0.0).count(), 0, "not off the desert");
    }

    #[test]
    fn the_ground_dries_through_the_day_after_rain() {
        let p = Place { force_rain: Some(0.8), ..Place::new("x", 50, 50, Some(0), 0) };
        let t = p.today(DAY_TICKS + 100, 5 * 1024, &w());
        assert!(t.rain > 0.0 && t.wet > 0.4 && t.lush == 0.0);
        assert!(t.drag(&w()) > 0);
        assert!(t.turnout(&w()) < 100);
        // a dry day after a wet one (found among the days)
        let p = Place::new("Drying Lane", 50, 50, Some(0), 0);
        let d = (1..500).find(|&d| p.rain_on(d - 1, 0) > 0.0 && p.rain_on(d, 0) == 0.0).unwrap();
        let morning = p.today(d * DAY_TICKS + 10, 4096, &w());
        let evening = p.today(d * DAY_TICKS + DAY_TICKS - 10, 4096, &w());
        assert!(morning.lush > 0.5 && morning.bounce_percent(&w()) < 100);
        assert_eq!(evening.wet, 0.0);
        assert_eq!(evening.drag(&w()), 0);
    }

    #[test]
    fn a_storm_breaks_once_and_passes() {
        let p = Place::new("Storm Point", 50, 50, Some(11), 3);
        let d = (0..2000).find(|&d| p.storm_on(d, 0, w().storm_percent)).unwrap();
        let ticks: Vec<Today> = (d * DAY_TICKS..(d + 1) * DAY_TICKS).map(|t| p.today(t, 2048, &w())).collect();
        assert_eq!(ticks.iter().filter(|t| t.storm_start).count(), 1);
        assert_eq!(ticks.iter().filter(|t| t.storm_end).count(), 1);
        assert_eq!(ticks.iter().filter(|t| t.storm).count(), w().storm_ticks as usize);
        assert!(ticks.iter().all(|t| t.storm_day));
        assert_eq!(forecast_key(&ticks[0]), Some("STR_WEATHER_STORM_TODAY"));
    }
}
