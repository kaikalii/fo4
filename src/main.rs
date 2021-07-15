mod build;
mod special;

use std::{
    fs,
    io::{stdin, BufRead},
    iter::once,
    path::PathBuf,
    process::exit,
};

use anyhow::bail;
use clap::Clap;

use build::*;
use once_cell::sync::Lazy;
use special::*;

fn main() {
    let app = App::parse();

    let mut build = if let Some(original_path) = app.path {
        match catch(|| {
            let mut path = original_path.clone();
            if !path.exists() {
                path = path.with_extension("yaml")
            }
            if !path.exists() {
                path = build_dir().join(path);
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
        }) {
            Ok(build) => build,
            Err(e) => {
                println!("{}", e);
                println!();
                println!("Press ENTER to close");
                stdin().lock().lines().next();
                exit(1);
            }
        }
    } else {
        Build::default()
    };

    Lazy::force(&PERKS);

    print!("{}[2J", 27 as char);
    println!("{}\n", Command::try_parse_from(&[""]).unwrap_err());
    println!("\n{}", build);

    for line in stdin().lock().lines().filter_map(|res| res.ok()) {
        print!("{}[2J", 27 as char);
        let args: Vec<&str> = once("fo4").chain(line.split_whitespace()).collect();
        match Command::try_parse_from(args) {
            Ok(command) => {
                let res = match command {
                    Command::Set {
                        stat,
                        value,
                        bobblehead,
                    } => build.set(stat, value, bobblehead),
                    Command::Add { perk, rank } => catch(|| {
                        build.add_perk(&perk, rank)?;
                        let name = perk.name.get(build.gender.unwrap_or_default());
                        if rank == 0 {
                            println!("Removed {}\n", name)
                        } else {
                            println!("Added {} rank {}\n", name, rank);
                        }
                        Ok(())
                    }),
                    Command::Remove { perk } => catch(|| {
                        build.remove_perk(&perk)?;
                        let name = perk.name.get(build.gender.unwrap_or_default());
                        println!("Removed {}\n", name);
                        Ok(())
                    }),
                    Command::Reset => {
                        build.reset();
                        Ok(())
                    }
                    Command::Name { name } => {
                        build.name = Some(name);
                        Ok(())
                    }
                    Command::Gender { gender } => {
                        build.gender = Some(gender);
                        Ok(())
                    }
                    Command::Book { stat } => catch(|| {
                        if let Some(stat) = stat {
                            if build.special[&stat] == 10 {
                                bail!("The S.P.E.C.I.A.L. book cannot be used on maxed-out stats");
                            }
                        }
                        build.special_book = stat;
                        Ok(())
                    }),
                    Command::Save { name } => catch(|| {
                        if let Some(name) = name {
                            build.name = Some(name);
                        }
                        let name = if let Some(name) = &build.name {
                            name
                        } else {
                            bail!("A name for the build must be specified. Try \"name <NAME>\" or \"save <NAME>\"");
                        };
                        fs::create_dir_all(&build_dir())?;
                        fs::write(
                            build_dir().join(name).with_extension("yaml"),
                            &serde_yaml::to_vec(&build)?,
                        )?;
                        Ok(())
                    }),
                    Command::Exit => break,
                };
                if let Err(e) = res {
                    println!("{}\n", e);
                }
                println!("{}\n", build);
            }
            Err(e) => {
                println!("{}", build);
                println!("{}\n", e);
            }
        }
    }
}

fn build_dir() -> PathBuf {
    dirs::data_dir()
        .expect("No data directory")
        .join("Fallout4Builds")
}

fn catch<F, T>(f: F) -> anyhow::Result<T>
where
    F: FnOnce() -> anyhow::Result<T>,
{
    f()
}

#[derive(Clap)]
struct App {
    path: Option<PathBuf>,
}

#[derive(Debug, Clap)]
enum Command {
    #[clap(about = "Set a special stat")]
    Set {
        stat: SpecialStat,
        value: u8,
        #[clap(short = 'b', long = "bobblehead")]
        bobblehead: bool,
    },
    #[clap(about = "Add a perk by name and rank")]
    Add {
        perk: PerkDef,
        #[clap(default_value = "1")]
        rank: u8,
    },
    #[clap(about = "Remove a perk")]
    Remove { perk: PerkDef },
    #[clap(about = "Reset the build")]
    Reset,
    #[clap(about = "Set the build's name")]
    Name { name: String },
    #[clap(about = "Set the build's gender")]
    Gender { gender: Gender },
    #[clap(about = "The which stat to allocate the special book to")]
    Book { stat: Option<SpecialStat> },
    #[clap(about = "Save the build")]
    Save { name: Option<String> },
    #[clap(about = "Exit this tool")]
    Exit,
}
