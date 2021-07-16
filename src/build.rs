use std::{
    collections::{BTreeMap, BTreeSet},
    fmt, fs,
    iter::repeat,
    path::{Path, PathBuf},
};

use anyhow::bail;
use colored::{Color, Colorize};
use serde::{Deserialize, Serialize};

use crate::special::{Bobblehead, Difficulty, Gender, PerkDef, PerkId, SpecialStat, PERKS};

#[derive(Debug, Serialize, Deserialize)]
pub struct Build {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gender: Option<Gender>,
    pub special: BTreeMap<SpecialStat, u8>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub bobbleheads: BTreeSet<Bobblehead>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub special_book: Option<SpecialStat>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub difficulty: Option<Difficulty>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub perks: BTreeMap<PerkId, u8>,
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
            bobbleheads: BTreeSet::new(),
            difficulty: None,
            special_book: None,
            perks: BTreeMap::new(),
        }
    }
}

impl fmt::Display for Build {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(name) = &self.name {
            let bars: String = repeat('-').take(name.len()).collect();
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
            format!("Base AP: {}", self.base_agility()).bright_blue()
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
        writeln!(f, "Max Settlement Pop: {}", self.max_settlers())?;
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
        if !self.perks.is_empty() {
            writeln!(f)?;
            for (id, rank) in &self.perks {
                writeln!(
                    f,
                    "{} {}",
                    PERKS.get_by_left(id).expect("Unknown perk").name
                        [self.gender.unwrap_or_default()],
                    rank
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
        let from_perks = self.effect_iter(PerkDef::hp_add).sum::<f32>();
        base + from_perks
    }
    pub fn health(&self) -> f32 {
        let level = self.required_level() as f32;
        self.base_health() + self.health_per_level() * (level - 1.0)
    }
    pub fn base_agility(&self) -> f32 {
        let agility = self.total_points(SpecialStat::Agility) as f32;
        let base = 60.0 + agility * 10.0;
        let from_perks = self.effect_iter(PerkDef::ap_add).sum::<f32>();
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
    pub fn max_settlers(&self) -> u8 {
        10 + self.total_points(SpecialStat::Charisma)
    }
    pub fn buying_price_mul(&self) -> f32 {
        3.5 - self.total_points(SpecialStat::Charisma) as f32 * 0.15
    }
    pub fn selling_price_mul(&self) -> f32 {
        1.0 / self.buying_price_mul()
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
        let from_perks = self.effect_iter(PerkDef::carry_weight_add).sum::<u16>();
        base + from_strength + from_perks
    }
    pub fn melee_damage_mul(&self) -> f32 {
        1.0 + self.total_points(SpecialStat::Strength) as f32 * 0.1
    }
    pub fn total_base_points(&self, stat: SpecialStat) -> u8 {
        self.special[&stat]
            + if self.bobbleheads.contains(&Bobblehead::Special(stat)) {
                1
            } else {
                0
            }
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
    }
    pub fn points_string(&self, stat: SpecialStat) -> String {
        format!(
            "{}{}{}",
            self.special[&stat].to_string(),
            if self.bobbleheads.contains(&Bobblehead::Special(stat)) {
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
    pub fn effect_iter<'a, F, T>(&'a self, f: F) -> impl Iterator<Item = T> + 'a
    where
        F: Fn(&PerkDef, u8) -> T + 'a,
    {
        self.perks
            .iter()
            .map(move |(id, rank)| f(PERKS.get_by_left(id).expect("Unknown perk"), *rank))
    }
    pub fn remaining_initial_points(&self) -> u8 {
        Self::INITIAL_ASSIGNABLE_POINTS.saturating_sub(self.assigned_special_points())
    }
    pub fn assigned_special_points(&self) -> u8 {
        self.special.values().sum::<u8>() - self.special.keys().count() as u8
    }
    pub fn assigned_perk_points(&self) -> u8 {
        self.perks.values().sum::<u8>()
    }
    pub fn assigned_points(&self) -> u8 {
        self.assigned_special_points() + self.assigned_perk_points()
    }
    pub fn required_level(&self) -> u8 {
        let for_rank_reqs = self
            .perks
            .iter()
            .map(|(id, rank)| {
                PERKS.get_by_left(id).expect("Unknown perk").ranks[*rank as usize - 1]
                    .required_level
            })
            .max()
            .unwrap_or(1);
        let for_spent_points = self
            .assigned_points()
            .saturating_sub(Self::INITIAL_ASSIGNABLE_POINTS);
        for_rank_reqs.max(for_spent_points)
    }
    pub fn set(
        &mut self,
        stat: SpecialStat,
        mut allocated: u8,
        mut bobblehead: bool,
    ) -> anyhow::Result<()> {
        if allocated == 11 {
            allocated = 10;
            bobblehead = true;
        }
        if allocated > 10 {
            bail!("Cannot allocate more than 10 points to any S.P.E.C.I.A.L. stat");
        } else if allocated == 0 {
            bail!("S.P.E.C.I.A.L. stats cannot be less the 1")
        }
        self.special.insert(stat, allocated);
        if bobblehead {
            self.bobbleheads.insert(Bobblehead::Special(stat));
        } else {
            self.bobbleheads.remove(&Bobblehead::Special(stat));
        }
        Ok(())
    }
    pub fn add_perk(&mut self, def: &PerkDef, rank: u8) -> anyhow::Result<()> {
        if rank > def.ranks.len() as u8 {
            bail!(
                "{} only has {} ranks",
                def.name[self.gender.unwrap_or_default()],
                def.ranks.len()
            )
        } else if rank == 0 {
            self.remove_perk(def)
        } else if let Some(id) = PERKS.get_by_right(def) {
            self.perks.insert(*id, rank);
            if let PerkId::Special { stat, points } = id {
                while self.total_base_points(*stat) < *points {
                    *self.special.get_mut(stat).unwrap() += 1;
                }
            }
            Ok(())
        } else {
            bail!("Unknown perk")
        }
    }
    pub fn remove_perk(&mut self, def: &PerkDef) -> anyhow::Result<()> {
        if let Some(id) = PERKS.get_by_right(def) {
            self.perks.remove(id);
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
        self.bobbleheads.clear();
        self.perks.clear();
        self.gender = None
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
        fs::write(self.path(), &serde_yaml::to_vec(&self)?)?;
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
    pub fn print_perk(&self, perk: &PerkDef) {
        let gender = self.gender.unwrap_or_default();
        let difficulty = self.difficulty.unwrap_or_default();
        println!("{}", perk.name[gender].bright_yellow());
        let perk_id = PERKS.get_by_right(perk).expect("Unknown perk");
        let my_rank = self.perks.get(&perk_id).copied().unwrap_or(0);
        for (i, rank) in perk.ranks.iter().enumerate() {
            let (rank_color, desc_color) = if my_rank > i as u8 {
                (Color::BrightCyan, Color::BrightWhite)
            } else {
                (Color::Cyan, Color::White)
            };
            println!(
                "{} {}",
                format!("Rank {}", i + 1).color(rank_color),
                format!("(Level {})", rank.required_level).bright_black(),
            );
            let width = terminal_size::terminal_size().map_or(80, |(width, _)| width.0 as usize);
            let mut words: Vec<&str> = Vec::new();
            for word in rank.description[difficulty][gender].split_whitespace() {
                if words.iter().map(|s| s.len() + 1).sum::<usize>() + word.len() >= width - 1 {
                    print!("  ");
                    for word in words.drain(..) {
                        print!("{} ", word.color(desc_color));
                    }
                    println!();
                }
                words.push(word);
            }
            if !words.is_empty() {
                print!("  ");
                for word in words {
                    print!("{} ", word.color(desc_color));
                }
                println!();
            }
        }
    }
}
