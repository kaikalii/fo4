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
    println!("\n{}", build);

    for line in stdin().lock().lines().filter_map(|res| res.ok()) {
        println!();
        let args: Vec<&str> = once("fo4").chain(line.split_whitespace()).collect();
        match Command::try_parse_from(args) {
            Ok(command) => {
                let res = match command {
                    Command::Set {
                        stat,
                        value,
                        bobblehead,
                    } => build.set(stat, value, bobblehead),
                    Command::Get { perk } => Ok(()),
                    Command::Book { stat } => catch(|| {
                        if let Some(stat) = stat {
                            if build.special[&stat] == 10 {
                                bail!("The S.P.E.C.I.A.L. book cannot be used on maxed-out stats");
                            }
                        }
                        build.special_book = stat;
                        Ok(())
                    }),
                    Command::Name { name } => {
                        build.name = Some(name);
                        Ok(())
                    }
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
                } else {
                    println!("{}\n", build);
                }
            }
            Err(e) => println!("{}\n", e),
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

#[derive(Clap)]
enum Command {
    Set {
        stat: SpecialStat,
        value: u8,
        #[clap(short = 'b', long = "bobblehead")]
        bobblehead: bool,
    },
    Get {
        perk: PerkDef,
    },
    Book {
        stat: Option<SpecialStat>,
    },
    Name {
        name: String,
    },
    Save {
        name: Option<String>,
    },
    Exit,
}
