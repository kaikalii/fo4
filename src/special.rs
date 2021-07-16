use std::{
    array, cmp::Ordering, collections::BTreeMap, fmt, ops::Index, process::exit, str::FromStr,
};

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
    Bobblehead(usize),
}

fn similarity(a: impl AsRef<str>, b: impl AsRef<str>) -> f64 {
    (strsim::jaro_winkler(a.as_ref(), b.as_ref()) * 2.0
        + strsim::normalized_levenshtein(a.as_ref(), b.as_ref()))
        / 3.0
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
                    (def, similarity(s, &name))
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

pub type FullyVariable<T> = MaybeDifficultied<MaybeGendered<T>>;

#[derive(Debug, Clone, Deserialize)]
pub struct Rank {
    #[serde(default = "default_required_level", alias = "level")]
    pub required_level: u8,
    #[serde(alias = "desc")]
    pub description: FullyVariable<String>,
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
    (carry_weight_add, u16, 0),
    (overencumbered_run_cost_mul, Option<f32>, None),
    (overencumbered_freedom, bool, false),
    (heavy_damage_mul, f32, 1.0),
    (heavy_hip_fire_accuracy_add, f32, 1.0),
    (heavy_stagger_chance, f32, 0.0),
    (hip_fire_accuracy_add, f32, 0.0),
    (hip_fire_damage_mul, f32, 1.0),
    (hp_add, f32, 0.0),
    (ap_add, f32, 0.0),
    (buy_price_sub, f32, 0.0)
);

pub trait Selectable<T>: Index<Self::Selector, Output = T> {
    type Selector: Copy + 'static;
    fn selectors() -> &'static [Self::Selector];
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(untagged)]
pub enum MaybeVaried<T, M> {
    One(T),
    Multi(M),
}

impl<T, M> MaybeVaried<T, M>
where
    M: Selectable<T>,
{
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        match self {
            MaybeVaried::One(once) => vec![once],
            MaybeVaried::Multi(multi) => M::selectors()
                .iter()
                .map(|selector| &multi[*selector])
                .collect(),
        }
        .into_iter()
    }
}

impl<T, M> Index<M::Selector> for MaybeVaried<T, M>
where
    M: Selectable<T>,
{
    type Output = T;
    fn index(&self, selector: M::Selector) -> &Self::Output {
        match self {
            MaybeVaried::One(one) => one,
            MaybeVaried::Multi(multi) => &multi[selector],
        }
    }
}

impl<T, M> From<T> for MaybeVaried<T, M> {
    fn from(val: T) -> Self {
        MaybeVaried::One(val)
    }
}

pub type MaybeGendered<T> = MaybeVaried<T, Gendered<T>>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
pub struct Gendered<T> {
    pub male: T,
    pub female: T,
}

impl<T> Index<Gender> for Gendered<T> {
    type Output = T;
    fn index(&self, gender: Gender) -> &Self::Output {
        match gender {
            Gender::Male => &self.male,
            Gender::Female => &self.female,
        }
    }
}

impl<T> Selectable<T> for Gendered<T> {
    type Selector = Gender;
    fn selectors() -> &'static [Self::Selector] {
        &[Gender::Male, Gender::Female]
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
pub enum Difficulty {
    VeryEasy,
    Easy,
    Normal,
    Hard,
    VeryHard,
    Survival,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
pub struct Difficultied<T> {
    pub normal: T,
    pub survival: T,
}

impl<T> Index<Difficulty> for Difficultied<T> {
    type Output = T;
    fn index(&self, difficulty: Difficulty) -> &Self::Output {
        match difficulty {
            Difficulty::Survival => &self.survival,
            _ => &self.normal,
        }
    }
}

impl<T> Selectable<T> for Difficultied<T> {
    type Selector = Difficulty;
    fn selectors() -> &'static [Self::Selector] {
        &[Difficulty::Normal, Difficulty::Survival]
    }
}

pub type MaybeDifficultied<T> = MaybeVaried<T, Difficultied<T>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Bobblehead {
    Special(SpecialStat),
    Skill(SkillBobblehead),
}

impl Default for Difficulty {
    fn default() -> Self {
        Difficulty::Normal
    }
}

impl FromStr for Difficulty {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();
        let (difficulty, sim) = array::IntoIter::new([
            Difficulty::VeryEasy,
            Difficulty::Easy,
            Difficulty::Normal,
            Difficulty::Hard,
            Difficulty::VeryHard,
            Difficulty::Survival,
        ])
        .map(|difficulty| {
            (
                difficulty,
                similarity(format!("{:?}", difficulty).to_lowercase(), &s),
            )
        })
        .max_by_key(|(_, sim)| (*sim * 1000000.0) as u64)
        .unwrap();
        println!("{:?}: {}", difficulty, sim);
        if sim >= 0.6 {
            Ok(difficulty)
        } else {
            bail!("Invalid difficulty: {}", s)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SkillBobblehead {
    Barter,
    BigGuns,
    EnergyWeapons,
    Explosives,
    Lockpicking,
    Medicine,
    Melee,
    Repair,
    Science,
    SmallGuns,
    Sneak,
    Speech,
    Unarmed,
}

#[derive(Deserialize)]
struct AllPerksRep {
    special: BTreeMap<SpecialStat, Vec<PerkDef>>,
    bobbleheads: BTreeMap<MaybeGendered<String>, Rank>,
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
    for (i, (name, rank)) in rep.bobbleheads.into_iter().enumerate() {
        perks.insert(
            PerkId::Bobblehead(i),
            PerkDef {
                name,
                ranks: vec![rank],
            },
        );
    }
    perks
});
