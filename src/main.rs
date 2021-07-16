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
    Lazy::force(&PERKS);

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

    println!("\n{}", build);
    println!("{}\n", "Type \"help\" for usage information".bright_blue());

    let mut level_limit: Option<u8> = None;

    for line in stdin().lock().lines().filter_map(|res| res.ok()) {
        let args: Vec<&str> = once("fo4").chain(line.split_whitespace()).collect();
        match Command::try_parse_from(args) {
            Ok(command) => {
                let res = match command {
                    Command::Set {
                        stat,
                        value,
                        bobblehead,
                    } => build
                        .set(stat, value, bobblehead)
                        .map(|_| format!("Set {:?} to {}", stat, value)),
                    Command::Add { perk_and_rank } => catch(|| {
                        let (perk, rank) = join_perk_def_and_rank(&perk_and_rank)?;
                        let rank = rank.unwrap_or_else(|| perk.max_rank()).min(
                            perk.ranks
                                .highest_rank_within_level(level_limit.unwrap_or(u8::MAX)),
                        );
                        build.add_perk(&perk, rank)?;
                        let name = &perk.name[build.gender.unwrap_or_default()];
                        Ok(if rank == 0 {
                            format!("Removed {}", name)
                        } else {
                            format!("Added {} rank {}", name, rank)
                        })
                    }),
                    Command::Remove { perk } => catch(|| {
                        let perk = join_perk_def(&perk)?;
                        build.remove_perk(&perk)?;
                        let name = &perk.name[build.gender.unwrap_or_default()];
                        Ok(format!("Removed {}", name))
                    }),
                    Command::Perk { perk } => match join_perk_def(&perk) {
                        Ok(perk) => {
                            clear_terminal();
                            println!("{}", build);
                            build.print_perk(&perk);
                            println!();
                            continue;
                        }
                        Err(e) => Err(e),
                    },
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
                    Command::Bobbleheads => {
                        clear_terminal();
                        println!("{}", build);
                        build.print_bobbleheads();
                        println!();
                        continue;
                    }
                    Command::Reset => {
                        build.reset();
                        Ok("Build reset!".into())
                    }
                    Command::Name { name } => {
                        let message = format!("Build name set to {:?}", name);
                        build.name = Some(name);
                        Ok(message)
                    }
                    Command::Gender { gender } => {
                        build.gender = Some(gender);
                        Ok(format!("Gender set to {:?}", gender))
                    }
                    Command::Book { stat } => catch(|| {
                        let message = if let Some(stat) = stat {
                            if build.special[&stat] == 10 {
                                bail!("The S.P.E.C.I.A.L. book cannot be used on a maxed-out stat");
                            }
                            format!("Special book set to {:?}", stat)
                        } else {
                            "Special book reset".into()
                        };
                        build.special_book = stat;
                        Ok(message)
                    }),
                    Command::Difficulty { difficulty } => {
                        build.difficulty = Some(difficulty);
                        Ok(format!("Difficulty set to {:?}", difficulty))
                    }
                    Command::LevelLimit { level } => {
                        level_limit = level;
                        Ok(if let Some(level) = level {
                            format!("Level limit set to {}", level)
                        } else {
                            "Removed level limit".into()
                        })
                    }
                    Command::Save { name } => catch(|| {
                        if let Some(name) = name {
                            build.name = Some(name);
                        }
                        build.save()?;
                        Ok("Build saved!".into())
                    }),
                    Command::Builds => catch(|| {
                        open::that(Build::dir())?;
                        Ok(String::new())
                    }),
                    Command::Exit => break,
                };
                clear_terminal();
                println!("{}", build);
                match res {
                    Ok(message) => {
                        if !message.is_empty() {
                            println!("{}\n", message.bright_green())
                        }
                    }
                    Err(e) => println!("{}\n", e.to_string().bright_red()),
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
#[allow(clippy::large_enum_variant)]
enum Command {
    #[clap(about = "Set a special stat")]
    Set {
        stat: SpecialStat,
        value: u8,
        #[clap(short = 'b', long = "bobblehead")]
        bobblehead: bool,
    },
    #[clap(about = "Add a perk by name and rank")]
    Add { perk_and_rank: Vec<String> },
    #[clap(about = "Remove a perk")]
    Remove { perk: Vec<String> },
    #[clap(about = "Display a perk")]
    Perk { perk: Vec<String> },
    #[clap(about = "Display all the perks for a S.P.E.C.I.A.L. stat(s)")]
    Special { stat: Option<SpecialStat> },
    #[clap(about = "Display all the perk bobbleheads")]
    Bobbleheads,
    #[clap(about = "Reset the build")]
    Reset,
    #[clap(about = "Set the build's name")]
    Name { name: String },
    #[clap(about = "Set the build's gender (affects perk names)")]
    Gender { gender: Gender },
    #[clap(about = "Set which stat to allocate the special book to")]
    Book { stat: Option<SpecialStat> },
    #[clap(about = "Set the difficulty (affects carry weight)", alias = "diff")]
    Difficulty { difficulty: Difficulty },
    #[clap(about = "Limit the maximum required level for added perks")]
    LevelLimit { level: Option<u8> },
    #[clap(about = "Save the build")]
    Save { name: Option<String> },
    #[clap(about = "Open the folder where builds are saved")]
    Builds,
    #[clap(about = "Exit this tool")]
    Exit,
}

fn join_perk_def(parts: &[String]) -> anyhow::Result<PerkDef> {
    parts.iter().map(String::as_str).collect::<String>().parse()
}

fn join_perk_def_and_rank(parts: &[String]) -> anyhow::Result<(PerkDef, Option<u8>)> {
    if parts.len() <= 1 {
        parts
            .get(0)
            .map(String::as_str)
            .unwrap_or_default()
            .parse::<PerkDef>()
            .map(|def| (def, None))
    } else if let Ok(last) = parts.last().unwrap().parse::<u8>() {
        let sub = &parts[..(parts.len() - 1)];
        if sub
            .last()
            .and_then(|part| part.parse::<u8>().ok())
            .is_some()
        {
            join_perk_def(sub).map(|def| (def, Some(last)))
        } else if let Ok(def) = join_perk_def(sub) {
            Ok((def, Some(last)))
        } else {
            join_perk_def(parts).map(|def| (def, None))
        }
    } else {
        join_perk_def(parts).map(|def| (def, None))
    }
}
