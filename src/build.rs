use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    iter::repeat,
};

use anyhow::bail;
use serde::{Deserialize, Serialize};

use crate::special::{Bobblehead, SpecialStat};

#[derive(Debug, Serialize, Deserialize)]
pub struct Build {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub special: BTreeMap<SpecialStat, u8>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub bobbleheads: BTreeSet<Bobblehead>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub special_book: Option<SpecialStat>,
}

impl Default for Build {
    fn default() -> Self {
        Build {
            name: None,
            special: SpecialStat::ALL.iter().map(|stat| (*stat, 1)).collect(),
            bobbleheads: BTreeSet::new(),
            special_book: None,
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
            writeln!(f, " ")?;
        }
        Ok(())
    }
}

impl Build {
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
}
