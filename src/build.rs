use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    iter::repeat,
};

use anyhow::bail;
use serde::{Deserialize, Serialize};

use crate::special::{Bobblehead, Gender, PerkDef, PerkId, SpecialStat, PERKS};

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
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub perks: BTreeMap<PerkId, u8>,
}

impl Default for Build {
    fn default() -> Self {
        Build {
            name: None,
            gender: None,
            special: SpecialStat::ALL.iter().map(|stat| (*stat, 1)).collect(),
            bobbleheads: BTreeSet::new(),
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

        if let Some(gender) = self.gender {
            writeln!(f, "Gender: {:?}", gender)?;
        }
        writeln!(f, "Required Level: {}", self.required_level())?;
        if self.remaining_initial_points() > 0 {
            writeln!(f, "Remaining Points: {}", self.remaining_initial_points())?;
        }
        writeln!(f)?;
        for (stat, level) in &self.special {
            write!(
                f,
                "{:>12} {}{}{}",
                stat.to_string(),
                level,
                if self.bobbleheads.contains(&Bobblehead::Special(*stat)) {
                    " + bobblehead"
                } else {
                    ""
                },
                if self.special_book.as_ref() == Some(stat) {
                    " + S.P.E.C.I.A.L. book"
                } else {
                    ""
                }
            )?;
            writeln!(f)?;
        }
        writeln!(f)?;
        for (id, rank) in &self.perks {
            writeln!(
                f,
                "{} {}",
                PERKS
                    .get_by_left(id)
                    .expect("Unknown perk")
                    .name
                    .get(self.gender.unwrap_or_default()),
                rank
            )?;
        }
        Ok(())
    }
}

impl Build {
    pub const INITIAL_ASSIGNABLE_POINTS: u8 = 21;
    pub fn base_stat(&self, stat: SpecialStat) -> u8 {
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
    pub fn remaining_initial_points(&self) -> u8 {
        Self::INITIAL_ASSIGNABLE_POINTS.saturating_sub(self.assigned_special_points())
    }
    pub fn assigned_special_points(&self) -> u8 {
        self.special.values().sum::<u8>() - 7
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
        allocated: u8,
        bobblehead: bool,
    ) -> anyhow::Result<()> {
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
                def.name.get(self.gender.unwrap_or_default()),
                def.ranks.len()
            )
        } else if rank == 0 {
            self.remove_perk(def)
        } else if let Some(id) = PERKS.get_by_right(def) {
            self.perks.insert(*id, rank);
            if let PerkId::Special { stat, points } = id {
                while self.base_stat(*stat) < *points {
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
}
