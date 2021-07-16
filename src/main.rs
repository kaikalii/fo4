mod build;
mod special;

use std::{
    io::{stdin, BufRead},
    iter::once,
    path::PathBuf,
    process::exit,
};

use anyhow::bail;
use clap::Clap;

use build::*;
use colored::Colorize;
use once_cell::sync::Lazy;
use special::*;

fn main() {
    let app = App::parse();

    if app.no_color || !colored::control::ShouldColorize::from_env().should_colorize() {
        colored::control::set_override(false);
    }

    let mut build = if let Some(path) = app.path {
        match Build::load(path) {
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
        clear_terminal();
        Build::default()
    };

    Lazy::force(&PERKS);
    println!("\n{}", build);
    println!("{}\n", "Type \"help\" for usage information".bright_blue());

    for line in stdin().lock().lines().filter_map(|res| res.ok()) {
        let args: Vec<&str> = once("fo4").chain(line.split_whitespace()).collect();
        match Command::try_parse_from(args) {
            Ok(command) => {
                let mut message = String::new();
                let res = match command {
                    Command::Set {
                        stat,
                        value,
                        bobblehead,
                    } => build
                        .set(stat, value, bobblehead)
                        .map(|_| message = format!("Set {:?} to {}", stat, value)),
                    Command::Add { perk, rank } => catch(|| {
                        let rank = rank.unwrap_or_else(|| perk.max_rank());
                        build.add_perk(&perk, rank)?;
                        let name = perk.name.get(build.gender.unwrap_or_default());
                        message = if rank == 0 {
                            format!("Removed {}", name)
                        } else {
                            format!("Added {} rank {}", name, rank)
                        };
                        Ok(())
                    }),
                    Command::Remove { perk } => catch(|| {
                        build.remove_perk(&perk)?;
                        let name = perk.name.get(build.gender.unwrap_or_default());
                        message = format!("Removed {}", name);
                        Ok(())
                    }),
                    Command::Perk { perk } => {
                        clear_terminal();
                        println!("{}", build);
                        build.print_perk(&perk);
                        println!();
                        continue;
                    }
                    Command::Special { stat } => {
                        clear_terminal();
                        println!("{}", build);
                        if let Some(stat) = stat {
                            build.print_special(stat);
                        } else {
                            for stat in build.special.keys() {
                                build.print_special(*stat);
                                println!();
                            }
                        }
                        println!();
                        continue;
                    }
                    Command::Reset => {
                        build.reset();
                        message = "Build reset!".into();
                        Ok(())
                    }
                    Command::Name { name } => {
                        message = format!("Build name set to {:?}", name);
                        build.name = Some(name);
                        Ok(())
                    }
                    Command::Gender { gender } => {
                        build.gender = Some(gender);
                        message = format!("Gender set to {:?}", gender);
                        Ok(())
                    }
                    Command::Book { stat } => catch(|| {
                        if let Some(stat) = stat {
                            if build.special[&stat] == 10 {
                                bail!("The S.P.E.C.I.A.L. book cannot be used on a maxed-out stat");
                            }
                            message = format!("Special book set to {:?}", stat);
                        } else {
                            message = "Special book reset".into();
                        }
                        build.special_book = stat;
                        Ok(())
                    }),
                    Command::Save { name } => catch(|| {
                        if let Some(name) = name {
                            build.name = Some(name);
                        }
                        build.save()?;
                        message = "Build saved!".into();
                        Ok(())
                    }),
                    Command::Builds => catch(|| {
                        open::that(Build::dir())?;
                        Ok(())
                    }),
                    Command::Exit => break,
                };
                clear_terminal();
                println!("{}", build);
                if !message.is_empty() {
                    println!("{}\n", message.bright_green());
                }
                if let Err(e) = res {
                    println!("{}\n", e.to_string().bright_red());
                }
            }
            Err(e) => {
                clear_terminal();
                println!("{}", build);
                match e.kind {
                    clap::ErrorKind::ValueValidation => {
                        println!("{}\n", e.info[2].bright_red())
                    }
                    clap::ErrorKind::MissingArgumentOrSubcommand => {
                        println!("{}\n", "Type \"help\" for usage information".bright_blue());
                    }
                    clap::ErrorKind::DisplayHelp => {
                        let message = e.to_string();
                        println!(
                            "COMMANDS:{}",
                            message.split("SUBCOMMANDS:").nth(1).unwrap_or(&message)
                        );
                    }
                    _ => println!("{}\n", e),
                }
            }
        }
    }
}

fn clear_terminal() {
    print!("{}[2J", 27 as char);
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
    #[clap(long = "nocolor", about = "Run without terminal colors")]
    no_color: bool,
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
    Add { perk: PerkDef, rank: Option<u8> },
    #[clap(about = "Remove a perk")]
    Remove { perk: PerkDef },
    #[clap(about = "Display a perk")]
    Perk { perk: PerkDef },
    #[clap(about = "Display all the perks for a S.P.E.C.I.A.L. stat(s)")]
    Special { stat: Option<SpecialStat> },
    #[clap(about = "Reset the build")]
    Reset,
    #[clap(about = "Set the build's name")]
    Name { name: String },
    #[clap(about = "Set the build's gender (affects perk names)")]
    Gender { gender: Gender },
    #[clap(about = "Set which stat to allocate the special book to")]
    Book { stat: Option<SpecialStat> },
    #[clap(about = "Save the build")]
    Save { name: Option<String> },
    #[clap(about = "Open the folder where builds are saved")]
    Builds,
    #[clap(about = "Exit this tool")]
    Exit,
}
