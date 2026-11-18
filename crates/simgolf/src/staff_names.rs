//! OpenSGolf extras: staff names (extras.rs `staff_names`; the port's own, docs/PLAYING.md). In the original every employee
//! of a job shares one name ("Randy Groundskeeper", "Sally Soda Vendor"). With this extra each hired employee gets a first
//! and a last name of their own, chosen once and kept with the game (`Employee::name_seed`), shown wherever the original
//! shows the employee's name: the Employee panel, the People screen, name tags, the routing map's list, the advisor. A name
//! given with Rename still wins. The Soda Vendors are women and the others men, as the original's names and pictures have
//! them. The lists are the port's own.
use crate::app::App;
use sg_core::staff::{self, Employee};

const MEN: [&str; 64] = [
    "Adam", "Alan", "Albert", "Andy", "Arthur", "Barry", "Ben", "Bill", "Bob", "Brian", "Carl", "Chris", "Colin", "Dale", "Dan", "Dave",
    "Dennis", "Doug", "Eddie", "Eric", "Frank", "Fred", "Gary", "George", "Glen", "Greg", "Hank", "Harry", "Ian", "Jack", "Jake", "Jim",
    "Joe", "John", "Keith", "Ken", "Larry", "Leo", "Luke", "Mark", "Matt", "Max", "Neil", "Nick", "Owen", "Pat", "Paul", "Pete", "Phil",
    "Ray", "Rick", "Rob", "Ron", "Ross", "Sam", "Scott", "Sean", "Steve", "Ted", "Tim", "Tom", "Tony", "Vic", "Walt",
];

const WOMEN: [&str; 48] = [
    "Abby", "Alice", "Amy", "Anna", "Becky", "Beth", "Carla", "Carol", "Cindy", "Claire", "Dana", "Debbie", "Donna", "Ellie", "Emma",
    "Fran", "Gina", "Grace", "Hazel", "Helen", "Holly", "Ivy", "Jane", "Jenny", "Jill", "Joan", "Judy", "Kate", "Kelly", "Laura", "Linda",
    "Lucy", "Maggie", "Mary", "Megan", "Molly", "Nancy", "Nora", "Pam", "Peggy", "Rose", "Ruth", "Sandy", "Sue", "Tina", "Vera", "Wendy",
    "Zoe",
];

const SURNAMES: [&str; 96] = [
    "Abbott", "Archer", "Bailey", "Baker", "Barnes", "Bell", "Bennett", "Brooks", "Burke", "Butler", "Carter", "Chapman", "Clark", "Cole",
    "Collins", "Cooper", "Cox", "Crane", "Dalton", "Davies", "Dixon", "Doyle", "Duncan", "Ellis", "Evans", "Farley", "Fisher", "Fleming",
    "Ford", "Foster", "Fox", "Gardner", "Gibson", "Grant", "Gray", "Hale", "Hall", "Harper", "Hayes", "Hill", "Holt", "Howard", "Hughes",
    "Hunt", "Jenkins", "Kane", "Keller", "Kemp", "Knight", "Lane", "Lawson", "Lewis", "Lloyd", "Marsh", "Mason", "Miller", "Mills",
    "Moore", "Morgan", "Murphy", "Nash", "Nolan", "Norris", "Owens", "Palmer", "Parker", "Pearce", "Porter", "Price", "Quinn", "Reed",
    "Reyes", "Riley", "Roberts", "Rowe", "Russell", "Sharp", "Shaw", "Simms", "Smith", "Stone", "Sutton", "Tanner", "Taylor", "Turner",
    "Vaughn", "Wade", "Walker", "Ward", "Watts", "Webb", "Wells", "West", "Wilson", "Wood", "Young",
];

/// The name a seed gives an employee (`woman`: a Soda Vendor).
pub fn name_of(seed: u32, woman: bool) -> String {
    let h = sg_core::weather::hash(&[seed, 0x7374_6166]);
    let first = if woman { WOMEN[(h % WOMEN.len() as u32) as usize] } else { MEN[(h % MEN.len() as u32) as usize] };
    let last = SURNAMES[((h >> 8) % SURNAMES.len() as u32) as usize];
    format!("{first} {last}")
}

/// An employee's own name with the extra on and once one is chosen (never for the player's pro).
pub fn own_name(e: &Employee) -> Option<String> {
    (crate::extras::staff_names() && e.name_seed != 0 && e.job != staff::job::OWNER)
        .then(|| name_of(e.name_seed, e.job == staff::job::SODA_VENDOR))
}

impl App {
    /// Gives each employee without a name one (each game tick while the extra is on): from the slot, the month hired and the
    /// job, so the same game names the same employees, whenever the extra is ticked. Two employees in a row never share a
    /// first and last name.
    pub fn name_staff(&mut self) {
        if !crate::extras::staff_names() {
            return;
        }
        for i in 0..self.employees.len() {
            let e = self.employees[i];
            if !e.active || e.name_seed != 0 || e.job == staff::job::OWNER {
                continue;
            }
            let woman = e.job == staff::job::SODA_VENDOR;
            let taken: Vec<String> = self
                .employees
                .iter()
                .filter(|o| o.active && o.name_seed != 0 && o.job != staff::job::OWNER)
                .map(|o| name_of(o.name_seed, o.job == staff::job::SODA_VENDOR))
                .collect();
            let mut seed = sg_core::weather::hash(&[i as u32, e.hired as u32, e.job as u32 & 0xff, e.upgraded as u32]) | 1;
            for _ in 0..16 {
                if !taken.contains(&name_of(seed, woman)) {
                    break;
                }
                seed = sg_core::weather::hash(&[seed]) | 1;
            }
            self.employees[i].name_seed = seed;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_stable_and_varied() {
        assert_eq!(name_of(12345, false), name_of(12345, false));
        let names: std::collections::HashSet<String> = (1..200u32).map(|s| name_of(s, false)).collect();
        assert!(names.len() > 150, "{}", names.len());
        assert!(WOMEN.iter().any(|w| name_of(77, true).starts_with(w)));
    }
}
