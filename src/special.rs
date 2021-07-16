use std::{cmp::Ordering, collections::BTreeMap, fmt, process::exit, str::FromStr};

use anyhow::bail;
use bimap::BiBTreeMap;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PerkId {
    Special { stat: SpecialStat, points: u8 },
    Bobblehead,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PerkDef {
    pub name: MaybeGendered<String>,
    pub ranks: Vec<Rank>,
}

impl PerkDef {
    pub fn max_rank(&self) -> u8 {
        self.ranks.len() as u8
    }
}

impl FromStr for PerkDef {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();
        let s = &s;
        let (def, sim) = PERKS
            .right_values()
            .flat_map(|def| {
                def.name.iter().map(move |name| {
                    let name = name.to_lowercase();
                    (
                        def,
                        (strsim::jaro_winkler(s, &name) + strsim::normalized_levenshtein(s, &name))
                            / 2.0,
                    )
                })
            })
            .max_by_key(|(_, sim)| (*sim * 1000000.0) as u32)
            .unwrap();
        if sim >= 0.6 {
            Ok(def.clone())
        } else {
            bail!("Unknown perk: {}", s)
        }
    }
}

impl PartialEq for PerkDef {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for PerkDef {}

impl PartialOrd for PerkDef {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.name.partial_cmp(&other.name)
    }
}

impl Ord for PerkDef {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Rank {
    #[serde(default = "default_required_level", alias = "level")]
    pub required_level: u8,
    #[serde(alias = "desc")]
    pub description: MaybeGendered<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty", flatten)]
    pub effects: Effects,
}

fn default_required_level() -> u8 {
    1
}

macro_rules! effects {
    ($(($name:ident, $ty:ty, $default:expr)),* $(,)?) => {
        #[derive(Debug, Clone, Deserialize)]
        pub struct Effects {
            $(
                #[serde(default, skip_serializing_if = "Option::is_none")]
                $name: Option<$ty>,
            )*
        }
        impl PerkDef {
            $(
                #[allow(dead_code)]
                pub fn $name(&self, rank: u8) -> $ty {
                    self.ranks.iter().take(rank as usize).rev().find_map(|rank| rank.effects.$name).unwrap_or($default)
                }
            )*
        }
    };
}

effects!(
    (unarmed_damage_mul, f32, 1.0),
    (unarmed_disarm_chance, f32, 0.0),
    (unarmed_power_attack_cripple_chance, f32, 0.0),
    (unarmed_crits_paralyze, bool, false),
    (melee_damage_mul, f32, 1.0),
    (melee_disarm_chance, f32, 0.0),
    (melee_cleaves, bool, false),
    (melee_cripple_chance, f32, 0.0),
    (can_slam_heads_off, bool, false),
    (armor_mod_rank, u8, 0),
    (melee_mod_rank, u8, 0),
    (carry_weight_add, u8, 0),
    (overencumbered_run_cost_mul, Option<f32>, None),
    (overencumbered_freedom, bool, false),
    (heavy_damage_mul, f32, 1.0),
    (heavy_hip_fire_accuracy_add, f32, 1.0),
    (heavy_stagger_chance, f32, 0.0),
    (hip_fire_accuracy_add, f32, 0.0),
    (hip_fire_damage_mul, f32, 1.0),
    (hp_add, f32, 0.0),
    (ap_add, f32, 0.0),
);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(untagged)]
pub enum MaybeGendered<T> {
    Unisex(T),
    Gendered(Gendered<T>),
}

impl<T> MaybeGendered<T> {
    pub fn get(&self, gender: Gender) -> &T {
        match self {
            MaybeGendered::Unisex(s) => s,
            MaybeGendered::Gendered(gendered) => gendered.get(gender),
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        match self {
            MaybeGendered::Unisex(s) => vec![s],
            MaybeGendered::Gendered(gendered) => {
                vec![&gendered.male, &gendered.female]
            }
        }
        .into_iter()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
pub struct Gendered<T> {
    pub male: T,
    pub female: T,
}

impl<T> Gendered<T> {
    pub fn get(&self, gender: Gender) -> &T {
        match gender {
            Gender::Male => &self.male,
            Gender::Female => &self.female,
        }
    }
}

impl<T> From<T> for MaybeGendered<T> {
    fn from(val: T) -> Self {
        MaybeGendered::Unisex(val)
    }
}

impl<T> From<Gendered<T>> for MaybeGendered<T> {
    fn from(gendered: Gendered<T>) -> Self {
        MaybeGendered::Gendered(gendered)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Gender {
    Male,
    Female,
}

impl Default for Gender {
    fn default() -> Self {
        Gender::Male
    }
}

impl FromStr for Gender {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "male" | "man" | "boy" | "guy" | "gentleman" | "he" => Gender::Male,
            "female" | "woman" | "girl" | "lady" | "she" => Gender::Female,
            _ => bail!("Invalid gender: {}", s),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Bobblehead {
    Special(SpecialStat),
    Skill(SkillBobblehead),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SkillBobblehead {}

#[derive(Deserialize)]
struct AllPerksRep {
    special: BTreeMap<SpecialStat, Vec<PerkDef>>,
}

pub static PERKS: Lazy<BiBTreeMap<PerkId, PerkDef>> = Lazy::new(|| {
    let rep: AllPerksRep = match serde_yaml::from_str(include_str!("perks.yaml")) {
        Ok(rep) => rep,
        Err(e) => {
            println!("{}", e);
            exit(1);
        }
    };
    let mut perks = BiBTreeMap::new();
    for (stat, defs) in rep.special {
        for (i, def) in defs.into_iter().enumerate() {
            perks.insert(
                PerkId::Special {
                    stat,
                    points: i as u8 + 1,
                },
                def,
            );
        }
    }
    perks
});
