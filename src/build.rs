use std::{
    collections::BTreeMap,
    fmt, fs,
    ops::{Add, Mul},
    path::{Path, PathBuf},
};

use anyhow::bail;
use colored::{Color, Colorize};
use serde::{Deserialize, Serialize};

use crate::special::{
    BobbleheadId, Difficulty, FullyVariable, Gender, PerkDef, PerkId, PerkKind, Ranks, SpecialStat,
    PERKS,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Build {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gender: Option<Gender>,
    pub special: BTreeMap<SpecialStat, u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub special_book: Option<SpecialStat>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub difficulty: Option<Difficulty>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub perks: BTreeMap<PerkId, u8>,
    #[serde(default)]
    pub show_sheet: bool,
}

impl Default for Build {
    fn default() -> Self {
        Build {
            name: None,
            gender: None,
            special: PERKS
                .left_values()
                .filter_map(|id| {
                    if let PerkId::Special { stat, .. } = id {
                        Some((*stat, 1))
                    } else {
                        None
                    }
                })
                .collect(),
            difficulty: None,
            special_book: None,
            perks: BTreeMap::new(),
            show_sheet: false,
        }
    }
}

impl fmt::Display for Build {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(name) = &self.name {
            let bars: String = "-".repeat(name.len());
            writeln!(f, "{}", bars)?;
            writeln!(f, "{}", name)?;
            writeln!(f, "{}", bars)?;
        }
        if let Some(difficuly) = self.difficulty {
            writeln!(f, "{:?}", difficuly)?;
        }
        if let Some(gender) = self.gender {
            writeln!(f, "Gender: {:?}", gender)?;
        }
        writeln!(f, "Required Level: {}", self.required_level())?;
        if self.remaining_initial_points() > 0 {
            writeln!(f, "Remaining Points: {}", self.remaining_initial_points())?;
        }
        writeln!(
            f,
            "{} {}",
            format!("Base Health: {}", self.health()).bright_red(),
            format!("({} + {}/lvl)", self.base_health(), self.health_per_level()).bright_black(),
        )?;
        writeln!(
            f,
            "{}",
            format!("Base AP: {}", self.base_ap()).bright_blue()
        )?;
        writeln!(
            f,
            "{}",
            format!("{:.0}% XP", self.experience_mul() * 100.0).bright_green()
        )?;
        writeln!(
            f,
            "{}",
            format!("Melee Damage: {:.0}%", self.melee_damage_mul() * 100.0).bright_magenta()
        )?;
        writeln!(
            f,
            "{}",
            format!("Hits per Crit: {}", self.hits_per_crit()).bright_yellow()
        )?;
        writeln!(f, "Carry Weight: {}", self.carry_weight())?;
        writeln!(
            f,
            "Buy Prices: {} / Sell Prices: {}",
            format!("{:.0}%", self.buying_price_mul() * 100.0,).bright_white(),
            format!("{:.0}%", self.selling_price_mul() * 100.0).bright_white(),
        )?;
        writeln!(f, "Sprint Time: {:.1} s", self.sprint_time())?;
        writeln!(f)?;
        for &stat in self.special.keys() {
            let total_points = self.total_base_points(stat);
            let color = match total_points {
                0..=1 => Color::BrightBlack,
                2..=3 => Color::BrightYellow,
                4..=6 => Color::BrightGreen,
                9..=10 => Color::BrightBlue,
                7..=8 => Color::BrightCyan,
                _ => Color::BrightMagenta,
            };
            write!(
                f,
                "{:>12} {}",
                stat.to_string(),
                self.points_string(stat).color(color),
            )?;
            writeln!(f)?;
        }
        if self.show_sheet {
            writeln!(f)?;
            for (i, stat) in SpecialStat::ALL.iter().enumerate() {
                if i > 0 {
                    write!(f, "│")?;
                }
                let width = self.column_width(*stat);
                write!(f, "{:width$}", stat.to_string())?;
            }
            writeln!(f)?;
            for (i, stat) in SpecialStat::ALL.iter().enumerate() {
                if i > 0 {
                    write!(f, "┼")?;
                }
                let width = self.column_width(*stat);
                write!(f, "{}", "─".repeat(width))?;
            }
            writeln!(f)?;
            for point in 1..=10 {
                self.fmt_point(point, f)?;
                writeln!(f)?;
            }
        }
        if !self.perks.is_empty() {
            writeln!(f)?;
            let mut last_kind = None;
            for (id, rank) in &self.perks {
                if self.show_sheet && matches!(id, PerkId::Special { .. })
                    || matches!(id, PerkId::Bobblehead(_))
                {
                    continue;
                }
                let kind = id.kind();
                if Some(kind) != last_kind {
                    writeln!(f, "{}", kind.to_string().bright_yellow())?;
                    last_kind = Some(kind);
                }
                let def = PERKS.get_by_left(id).expect("Unknown perk");
                writeln!(
                    f,
                    "  {}{}",
                    def.name[self.gender.unwrap_or_default()],
                    if def.max_rank() > 1 {
                        format!(" {}", rank)
                    } else {
                        String::new()
                    }
                )?;
            }
        }
        Ok(())
    }
}

impl Build {
    pub const INITIAL_ASSIGNABLE_POINTS: u8 = 21;
    pub fn health_per_level(&self) -> f32 {
        2.5 + (self.total_points(SpecialStat::Endurance) as f32 * 0.5)
    }
    pub fn base_health(&self) -> f32 {
        let endurance = self.total_points(SpecialStat::Endurance) as f32;
        let base = 80.0 + endurance * 5.0;
        let from_perks = self.fold_effect(PerkDef::hp_add, 0.0, Add::add);
        base + from_perks
    }
    pub fn health(&self) -> f32 {
        let level = self.required_level() as f32;
        self.base_health() + self.health_per_level() * (level - 1.0)
    }
    pub fn base_ap(&self) -> f32 {
        let agility = self.total_points(SpecialStat::Agility) as f32;
        let base = 60.0 + agility * 10.0;
        let from_perks = self.fold_effect(PerkDef::ap_add, 0.0, Add::add);
        base + from_perks
    }
    pub fn hits_per_crit(&self) -> u8 {
        match self.total_points(SpecialStat::Luck) {
            1 => 14,
            2 => 12,
            3 => 10,
            4 => 9,
            5 => 8,
            6..=7 => 7,
            8..=9 => 6,
            10..=12 => 5,
            13..=18 => 4,
            19..=29 => 3,
            30..=62 => 2,
            _ => 1,
        }
    }
    pub fn buying_price_mul(&self) -> f32 {
        ((3.5 - self.total_points(SpecialStat::Charisma) as f32 * 0.15)
            / (1.0 + self.fold_effect(PerkDef::buy_price_sub, 0.0, Add::add)))
        .max(1.2)
    }
    pub fn selling_price_mul(&self) -> f32 {
        (1.0 / self.buying_price_mul()).min(0.8)
    }
    pub fn experience_mul(&self) -> f64 {
        let intelligence = self.total_points(SpecialStat::Intelligence);
        1.0 + intelligence as f64 * 0.03
    }
    pub fn carry_weight(&self) -> u16 {
        let base = if self.difficulty == Some(Difficulty::Survival) {
            75
        } else {
            200
        };
        let from_strength = self.total_points(SpecialStat::Strength) as u16 * 10;
        let from_perks = self.fold_effect(PerkDef::carry_weight_add, 0, Add::add);
        base + from_strength + from_perks
    }
    pub fn melee_damage_mul(&self) -> f32 {
        1.0 + self.total_points(SpecialStat::Strength) as f32 * 0.1
            + self.fold_effect(PerkDef::melee_damage_add, 0.0, Add::add)
    }
    pub fn sprint_time(&self) -> f32 {
        let ap_per_sec = (1.05 - 0.05 * self.total_points(SpecialStat::Endurance) as f32)
            * 12.0
            * self.fold_effect(PerkDef::sprint_drain_mul, 1.0, Mul::mul);
        self.base_ap() / ap_per_sec
    }
    pub fn total_base_points(&self, stat: SpecialStat) -> u8 {
        self.special[&stat]
            + self.bobblehead_for(stat) as u8
            + if self.special_book == Some(stat) {
                1
            } else {
                0
            }
    }
    pub fn total_points(&self, stat: SpecialStat) -> u8 {
        self.total_base_points(stat)
            + match stat {
                SpecialStat::Perception => self
                    .perks
                    .get(&PerkId::Special {
                        stat: SpecialStat::Intelligence,
                        points: 1,
                    })
                    .map_or(0, |&rank| if rank >= 2 { 2 } else { 0 }),
                _ => 0,
            }
            + self.stat_increase_for(stat)
            - self.bobblehead_for(stat) as u8
    }
    pub fn bobblehead_for(&self, stat: SpecialStat) -> bool {
        self.perks
            .contains_key(&PerkId::Bobblehead(BobbleheadId::Special(stat)))
    }
    pub fn stat_increase_for(&self, stat: SpecialStat) -> u8 {
        self.fold_effect(PerkDef::stat_increase, 0, |acc, si| {
            acc + if si.stat == stat { si.increase } else { 0 }
        })
    }
    pub fn points_string(&self, stat: SpecialStat) -> String {
        format!(
            "{}{}{}",
            self.special[&stat],
            if self.bobblehead_for(stat) {
                " + bobblehead"
            } else {
                ""
            },
            if self.special_book == Some(stat) {
                " + S.P.E.C.I.A.L. book"
            } else {
                ""
            }
        )
    }
    pub fn fold_effect<'a, F, T, G, A, I>(&'a self, get: F, init: A, fold: G) -> A
    where
        F: Fn(&'a PerkDef, u8) -> I + 'a,
        G: Fn(A, T) -> A + Clone,
        I: Iterator<Item = T>,
    {
        self.perks
            .iter()
            .flat_map(|(id, rank)| get(PERKS.get_by_left(id).expect("Unknown perk"), *rank))
            .fold(init, fold)
    }
    pub fn remaining_initial_points(&self) -> u8 {
        Self::INITIAL_ASSIGNABLE_POINTS.saturating_sub(self.assigned_special_points())
    }
    pub fn assigned_special_points(&self) -> u8 {
        self.special.values().sum::<u8>() - self.special.keys().count() as u8
    }
    pub fn level_up_assigned_special_points(&self) -> u8 {
        self.assigned_special_points()
            .saturating_sub(Self::INITIAL_ASSIGNABLE_POINTS)
    }
    pub fn assigned_perk_points(&self) -> u8 {
        self.perks
            .iter()
            .filter(|(id, _)| matches!(id, PerkId::Special { .. }))
            .map(|(_, rank)| rank)
            .sum::<u8>()
    }
    pub fn level_up_assigned_points(&self) -> u8 {
        self.level_up_assigned_special_points() + self.assigned_perk_points()
    }
    pub fn required_level(&self) -> u8 {
        let for_rank_reqs = self
            .perks
            .iter()
            .map(|(id, rank)| {
                PERKS
                    .get_by_left(id)
                    .expect("Unknown perk")
                    .ranks
                    .required_level(*rank)
            })
            .max()
            .unwrap_or(1);
        let for_spent_points = self.level_up_assigned_points() + 1;
        for_rank_reqs.max(for_spent_points)
    }
    pub fn set(&mut self, stat: SpecialStat, mut allocated: u8) -> anyhow::Result<()> {
        let mut add_bobble = false;
        if allocated == 11 {
            allocated = 10;
            add_bobble = true;
        }
        if allocated > 10 {
            bail!("Cannot allocate more than 10 points to any S.P.E.C.I.A.L. stat");
        } else if allocated == 0 {
            bail!("S.P.E.C.I.A.L. stats cannot be less the 1")
        }
        self.special.insert(stat, allocated);
        if add_bobble {
            self.perks
                .insert(PerkId::Bobblehead(BobbleheadId::Special(stat)), 1);
        }
        self.remove_invalid_perks();
        Ok(())
    }
    fn add_perk_impl(&mut self, id: PerkId, rank: u8) {
        self.perks.insert(id, rank);
        if let PerkId::Special { stat, points } = id {
            while self.total_base_points(stat) < points {
                *self.special.get_mut(&stat).unwrap() += 1;
            }
        }
    }
    pub fn add_perk(&mut self, def: &PerkDef, rank: u8) -> anyhow::Result<()> {
        let id = if let Some(id) = PERKS.get_by_right(def) {
            *id
        } else {
            bail!("Unknown perk")
        };
        if rank == 0 {
            self.remove_perk(def)?;
        } else {
            match &def.ranks {
                Ranks::Single { .. } => {
                    self.add_perk_impl(id, 1);
                }
                Ranks::UniformCumulative { count, .. } => {
                    if rank > *count {
                        bail!(
                            "{} only has {} ranks",
                            def.name[self.gender.unwrap_or_default()],
                            count
                        )
                    } else {
                        self.add_perk_impl(id, rank);
                    }
                }
                Ranks::VaryingCumulative(ranks) => {
                    if rank > ranks.len() as u8 {
                        bail!(
                            "{} only has {} ranks",
                            def.name[self.gender.unwrap_or_default()],
                            ranks.len()
                        )
                    } else {
                        self.add_perk_impl(id, rank);
                    }
                }
            }
        }
        Ok(())
    }
    pub fn remove_perk(&mut self, def: &PerkDef) -> anyhow::Result<()> {
        if let Some(id) = PERKS.get_by_right(def) {
            self.perks.remove(id);
            self.remove_invalid_perks();
            Ok(())
        } else {
            bail!("Unknown perk")
        }
    }
    pub fn reset(&mut self) {
        for i in self.special.values_mut() {
            *i = 1;
        }
        self.special_book = None;
        self.perks.clear();
        self.gender = None
    }
    fn remove_invalid_perks(&mut self) {
        let special: BTreeMap<SpecialStat, u8> = self
            .special
            .keys()
            .map(|&stat| (stat, self.total_base_points(stat)))
            .collect();
        self.perks.retain(|id, _| match id {
            PerkId::Special { stat, points } => special[stat] >= *points,
            _ => true,
        });
    }
    fn column_width(&self, stat: SpecialStat) -> usize {
        PERKS
            .iter()
            .filter(|(id, _)| id.kind() == PerkKind::Special(stat))
            .map(|(id, def)| {
                def.name[self.gender.unwrap_or_default()].chars().count()
                    + (self.perks.contains_key(id) as usize) * 2
            })
            .max()
            .unwrap_or(0)
    }
    fn fmt_point(&self, point: u8, f: &mut fmt::Formatter) -> fmt::Result {
        for (perk, def) in PERKS.iter() {
            if let PerkId::Special { stat, points } = perk {
                if *points == point {
                    let color = if self.perks.contains_key(perk) {
                        Color::Cyan
                    } else if self.total_points(*stat) >= *points {
                        Color::White
                    } else {
                        Color::BrightBlack
                    };
                    let width = self.column_width(*stat);
                    let text = &def.name[self.gender.unwrap_or_default()];
                    let text = if let Some(rank) = self.perks.get(perk) {
                        format!("{text} {rank}")
                    } else {
                        text.to_string()
                    };
                    write!(f, "{}", format!("{:width$}", text).color(color))?;
                    if *stat < SpecialStat::Luck {
                        write!(f, "│")?;
                    }
                }
            }
        }
        Ok(())
    }
    pub fn dir() -> PathBuf {
        dirs::data_dir()
            .expect("No data directory")
            .join("Fallout4Builds")
    }
    pub fn path(&self) -> PathBuf {
        Self::dir()
            .join(self.name.as_deref().unwrap_or("last"))
            .with_extension("yaml")
    }
    pub fn save(&self) -> anyhow::Result<()> {
        if self.name.is_none() {
            bail!(
                "A name for the build must be specified. Try \"name <NAME>\" or \"save <NAME>\"."
            );
        };
        fs::create_dir_all(Build::dir())?;
        fs::write(self.path(), serde_yaml::to_vec(&self)?)?;
        Ok(())
    }
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let original_path = path.as_ref();
        let mut path = original_path.to_path_buf();
        if !path.exists() {
            path = path.with_extension("yaml")
        }
        if !path.exists() {
            path = Self::dir().join(path);
        }
        if !path.exists() {
            bail!(
                "Unable to find build file for \"{}\"",
                original_path.to_string_lossy()
            );
        }
        let bytes = fs::read(path)?;
        let build: Build = serde_yaml::from_slice(&bytes)?;
        Ok(build)
    }
    pub fn print_special(&self, stat: SpecialStat) {
        let gender = self.gender.unwrap_or_default();
        let total_points = self.total_base_points(stat);
        println!(
            "{} ({})",
            stat.to_string().bright_yellow(),
            self.points_string(stat)
        );
        for points in 1..=10 {
            let perk_id = PerkId::Special { stat, points };
            let perk = PERKS.get_by_left(&perk_id).expect("Unknown perk");
            let this_perk_points = self.perks.get(&perk_id);
            let color = if points <= total_points {
                if this_perk_points.is_some() {
                    Color::BrightWhite
                } else {
                    Color::White
                }
            } else {
                Color::BrightBlack
            };
            println!(
                "{:2}: {} {}",
                points,
                perk.name[gender].color(color),
                if let Some(points) = this_perk_points {
                    format!("({})", points)
                } else {
                    String::new()
                }
            );
        }
    }
    pub fn print_perk_names(&self, kind: PerkKind) {
        println!("{}", kind.to_string().bright_yellow());
        let gender = self.gender.unwrap_or_default();
        for (id, def) in PERKS.iter().filter(|(id, _)| id.kind() == kind) {
            let color = if self.perks.contains_key(id) {
                Color::White
            } else {
                Color::BrightBlack
            };
            println!("  {}", def.name[gender].color(color));
        }
    }
    pub fn print_perk(&self, perk: &PerkDef) {
        let gender = self.gender.unwrap_or_default();
        let difficulty = self.difficulty.unwrap_or_default();
        print!("{}", perk.name[gender].bright_yellow());
        let perk_id = PERKS.get_by_right(perk).expect("Unknown perk");
        let my_rank = self.perks.get(perk_id).copied().unwrap_or(0);
        let print_rank = |i: Option<usize>,
                          required_level: u8,
                          description: &FullyVariable<String>| {
            let (rank_color, desc_color) = if i.map_or(false, |i| my_rank > i as u8) {
                (Color::BrightCyan, Color::BrightWhite)
            } else {
                (Color::Cyan, Color::White)
            };
            if let Some(i) = i {
                print!("{}", format!("Rank {}", i + 1).color(rank_color),);
                if required_level > 1 {
                    println!("{}", format!(" (Level {})", required_level).bright_black())
                } else {
                    println!();
                }
            }
            let width = terminal_size::terminal_size().map_or(80, |(width, _)| width.0 as usize);
            let mut words: Vec<&str> = Vec::new();
            for word in description[difficulty][gender]
                .split_inclusive('\n')
                .flat_map(|s| s.split(|c| [' ', '\t', '\r'].contains(&c)))
                .filter(|s| !s.is_empty())
            {
                let newline = word.ends_with('\n');
                let word = word.trim();
                if newline {
                    words.push(word);
                }
                if newline
                    || words.iter().map(|s| s.len() + 1).sum::<usize>() + word.len() >= width - 2
                {
                    print!("  ");
                    for word in words.drain(..) {
                        print!("{} ", word.color(desc_color));
                    }
                    println!();
                }
                if !newline {
                    words.push(word);
                }
            }
            if !words.is_empty() {
                print!("  ");
                for word in words {
                    print!("{} ", word.color(desc_color));
                }
                println!();
            }
        };
        match &perk.ranks {
            Ranks::Single { description, .. } => {
                println!();
                print_rank(None, 1, description);
            }
            Ranks::UniformCumulative {
                count, description, ..
            } => {
                println!(" {}", format!("({}/{})", my_rank, count).bright_black());
                print_rank(None, 1, description);
            }
            Ranks::VaryingCumulative(ranks) => {
                println!(
                    " {}",
                    format!("({}/{})", my_rank, ranks.len()).bright_black()
                );
                for (i, rank) in ranks.iter().enumerate() {
                    print_rank(Some(i), rank.required_level, &rank.description);
                }
            }
        }
    }
}
