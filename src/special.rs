use std::{collections::BTreeMap, fmt, str::FromStr};

use anyhow::bail;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SpecialStat {
    Strength,
    Perception,
    Endurance,
    Charisma,
    Intelligence,
    Agility,
    Luck,
}

impl SpecialStat {
    pub const ALL: &'static [Self] = &[
        SpecialStat::Strength,
        SpecialStat::Perception,
        SpecialStat::Endurance,
        SpecialStat::Charisma,
        SpecialStat::Intelligence,
        SpecialStat::Agility,
        SpecialStat::Luck,
    ];
}

impl FromStr for SpecialStat {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.to_lowercase();
        for stat in Self::ALL {
            if format!("{:?}", stat).to_lowercase().starts_with(&lower) {
                return Ok(*stat);
            }
        }
        Err(format!("Invalid S.P.E.C.I.A.L. stat: {}", s))
    }
}

impl fmt::Display for SpecialStat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SpecialPerk {}

impl fmt::Display for SpecialPerk {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut chars = format!("{:?}", self).chars().peekable();
        while let Some(c) = chars.next() {
            write!(f, "{}", c)?;
            if c.is_lowercase() && chars.peek().map_or(false, |c| c.is_uppercase()) {
                write!(f, " ")?;
            }
        }
        Ok(())
    }
}

impl FromStr for SpecialPerk {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for perk in PERKS.all_special() {
            let lower = s.to_lowercase();
            if def.name.iter().any(|name| {
                name.split_whitespace()
                    .map(|s| s.to_lowercase())
                    .any(|s| s == lower)
            }) {
                return Ok(def.clone());
            }
        }
        bail!("Unknown perk: {}", s)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Rank {
    #[serde(default = "default_required_level", alias = "level")]
    pub required_level: u8,
    #[serde(alias = "desc")]
    pub description: GenderedText,
}

fn default_required_level() -> u8 {
    1
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum GenderedText {
    Unisex(String),
    Gendered { male: String, female: String },
}

impl GenderedText {
    pub fn new(level: u8, description: impl Into<GenderedText>) -> Rank {
        Rank {
            required_level: level,
            description: description.into(),
        }
    }
    pub fn get(&self, gender: Gender) -> &str {
        match self {
            GenderedText::Unisex(s) => s,
            GenderedText::Gendered { male, female } => match gender {
                Gender::Male => male,
                Gender::Female => female,
            },
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        match self {
            GenderedText::Unisex(s) => vec![s.as_str()],
            GenderedText::Gendered { male, female } => vec![male.as_str(), female.as_str()],
        }
        .into_iter()
    }
}

impl<S> From<S> for GenderedText
where
    S: Into<String>,
{
    fn from(s: S) -> Self {
        GenderedText::Unisex(s.into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Gender {
    Male,
    Female,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Bobblehead {
    Special(SpecialStat),
    Skill(SkillBobblehead),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SkillBobblehead {}

#[derive(Clone)]
pub struct AllPerks {
    special: BTreeMap<SpecialStat, BTreeMap<SpecialPerk, Vec<Rank>>>,
}

impl AllPerks {
    pub fn all_special(&self) -> impl Iterator<Item = &SpecialPerk> {
        self.special.values().flatten()
    }
}

pub static PERKS: Lazy<AllPerks> =
    Lazy::new(|| serde_yaml::from_str(include_str!("perks.yaml")).unwrap());
