#![allow(unstable_name_collisions)]

mod build;
mod special;

use std::{
    io::{stdin, BufRead},
    iter::once,
    path::PathBuf,
    process::exit,
};

use anyhow::bail;
use clap::Parser;

use build::*;
use colored::Colorize;
use itertools::Itertools;
use once_cell::sync::Lazy;
use special::*;

fn main() {
    Lazy::force(&PERKS);

    let app = App::parse();

    if app.no_color || !colored::control::SHOULD_COLORIZE.should_colorize() {
        colored::control::set_override(false);
    }

    let mut build = if app.path.is_empty() {
        clear_terminal();
        Build::default()
    } else {
        let path: String = app
            .path
            .iter()
            .map(|path| path.to_string_lossy().into_owned())
            .intersperse(" ".into())
            .collect();
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
    };

    println!("\n{}", build);
    let type_help = || println!("{}\n", "Type \"help\" for usage information".bright_blue());
    type_help();

    let mut level_limit: Option<u8> = None;

    for line in stdin().lock().lines().map_while(Result::ok) {
        let args: Vec<&str> = once("fo4").chain(line.split_whitespace()).collect();
        match Command::try_parse_from(args) {
            Ok(command) => {
                let res = match command {
                    Command::Set { stat, value } => build
                        .set(stat, value)
                        .map(|_| format!("Set {:?} to {}", stat, value)),
                    Command::Add {
                        perk: head,
                        tail_and_rank: mut perk_and_rank,
                    } => catch(|| {
                        perk_and_rank.insert(0, head);
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
                    Command::Remove {
                        perk: head,
                        tail: mut perk,
                    } => catch(|| {
                        perk.insert(0, head);
                        let perk = join_perk_def(&perk)?;
                        build.remove_perk(&perk)?;
                        let name = &perk.name[build.gender.unwrap_or_default()];
                        Ok(format!("Removed {}", name))
                    }),
                    Command::Perk {
                        perk: head,
                        tail: mut perk,
                    } => {
                        perk.insert(0, head);
                        match join_perk_def(&perk) {
                            Ok(perk) => {
                                clear_terminal();
                                println!("{}", build);
                                build.print_perk(&perk);
                                println!();
                                continue;
                            }
                            Err(e) => Err(e),
                        }
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
                    Command::Bobbleheads => {
                        clear_terminal();
                        println!("{}", build);
                        build.print_perk_names(PerkKind::Bobblehead);
                        println!();
                        continue;
                    }
                    Command::Magazines => {
                        clear_terminal();
                        println!("{}", build);
                        build.print_perk_names(PerkKind::Magazine);
                        println!();
                        continue;
                    }
                    Command::Companions => {
                        clear_terminal();
                        println!("{}", build);
                        build.print_perk_names(PerkKind::Companion);
                        println!();
                        continue;
                    }
                    Command::Factions => {
                        clear_terminal();
                        println!("{}", build);
                        build.print_perk_names(PerkKind::Faction);
                        println!();
                        continue;
                    }
                    Command::OtherPerks => {
                        clear_terminal();
                        println!("{}", build);
                        build.print_perk_names(PerkKind::Other);
                        println!();
                        continue;
                    }
                    Command::Reset => {
                        build.reset();
                        Ok("Build reset!".into())
                    }
                    Command::Name { name } => catch(|| {
                        if name.is_empty() {
                            bail!("Name cannot be empty")
                        }
                        let name = name.into_iter().intersperse(" ".into()).collect();
                        let message = format!("Build name set to {:?}", name);
                        build.name = Some(name);
                        Ok(message)
                    }),
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
                    Command::Sheet => {
                        build.show_sheet = !build.show_sheet;
                        Ok(String::new())
                    }
                    Command::Save { name } => catch(|| {
                        if !name.is_empty() {
                            build.name = Some(name.into_iter().intersperse(" ".into()).collect());
                        }
                        build.save()?;
                        Ok("Build saved!".into())
                    }),
                    Command::Load { path } => catch(|| {
                        let path: String = path
                            .iter()
                            .map(|path| path.to_string_lossy().into_owned())
                            .intersperse(" ".into())
                            .collect();
                        build = Build::load(path)?;
                        level_limit = None;
                        Ok("Build loaded!".into())
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
                match e.kind() {
                    clap::ErrorKind::ValueValidation => println!("{e}\n"),
                    clap::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => type_help(),
                    clap::ErrorKind::DisplayHelp => {
                        let message = e.to_string();
                        println!(
                            "COMMANDS:{}",
                            message
                                .split("SUBCOMMANDS:")
                                .nth(1)
                                .unwrap_or(&message)
                                .replace(" fo4", "")
                        );
                    }
                    clap::ErrorKind::UnknownArgument => {
                        let text = e.to_string();
                        let command = text.split('\'').nth(1).unwrap_or(&text);
                        println!("{}\n", format!("Unknown command: {command}").bright_red());
                        type_help();
                    }
                    _ => {
                        let message = e.to_string();
                        let message =
                            message.trim_end_matches("\n\nFor more information try --help\n");
                        println!("{}\n", message)
                    }
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

#[derive(Parser)]
struct App {
    path: Vec<PathBuf>,
    #[clap(long = "nocolor", help = "Run without terminal colors")]
    no_color: bool,
}

#[derive(Debug, Parser)]
#[allow(clippy::large_enum_variant)]
enum Command {
    #[clap(display_order = 1, about = "Set a special stat")]
    Set { stat: SpecialStat, value: u8 },
    #[clap(display_order = 1, about = "Add a perk by name and rank")]
    Add {
        perk: String,
        tail_and_rank: Vec<String>,
    },
    #[clap(display_order = 1, about = "Remove a perk")]
    Remove { perk: String, tail: Vec<String> },
    #[clap(display_order = 1, about = "Display a perk")]
    Perk { perk: String, tail: Vec<String> },
    #[clap(
        display_order = 1,
        about = "Display all perks for a S.P.E.C.I.A.L. stat(s)"
    )]
    Special { stat: Option<SpecialStat> },
    #[clap(about = "Display all perk bobbleheads")]
    Bobbleheads,
    #[clap(about = "Display all perk magazines")]
    Magazines,
    #[clap(about = "Display all companion perks")]
    Companions,
    #[clap(about = "Display all faction perks")]
    Factions,
    #[clap(about = "Display all other perks")]
    OtherPerks,
    #[clap(display_order = 2, about = "Reset the build")]
    Reset,
    #[clap(display_order = 2, about = "Set the build's name")]
    Name { name: Vec<String> },
    #[clap(about = "Set the build's gender (affects perk names)")]
    Gender { gender: Gender },
    #[clap(about = "Set which stat to allocate the special book to")]
    Book { stat: Option<SpecialStat> },
    #[clap(about = "Set the difficulty (affects carry weight)", alias = "diff")]
    Difficulty { difficulty: Difficulty },
    #[clap(
        alias = "ll",
        about = "Limit the maximum required level for added perks"
    )]
    LevelLimit { level: Option<u8> },
    #[clap(about = "Toggle the build sheet display")]
    Sheet,
    #[clap(display_order = 2, about = "Save the build")]
    Save { name: Vec<String> },
    #[clap(display_order = 2, about = "Load a build")]
    Load { path: Vec<PathBuf> },
    #[clap(about = "Open the folder where builds are saved")]
    Builds,
    #[clap(display_order = 2, about = "Exit this tool")]
    Exit,
}

fn join_perk_def(parts: &[String]) -> anyhow::Result<PerkDef> {
    if parts.is_empty() {
        bail!("You must specify a perk")
    } else {
        parts.iter().map(String::as_str).collect::<String>().parse()
    }
}

fn join_perk_def_and_rank(parts: &[String]) -> anyhow::Result<(PerkDef, Option<u8>)> {
    if parts.is_empty() {
        bail!("You must specify a perk")
    } else if parts.len() == 1 {
        parts[0].parse::<PerkDef>().map(|def| (def, None))
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
