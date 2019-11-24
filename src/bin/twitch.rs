extern crate twitch;

use clap::{App, Arg};
use dirs;
use env_logger;
use failure::Error;
use serde_json;
use std::process::exit;
use std::str::FromStr;
use twitch::{TwitchDb, TwitchGame};

fn run() -> Result<(), Error> {
    env_logger::init();
    let matches = App::new("twitch")
        .about("Launcher for Twitch Prime games.")
        .arg(
            Arg::with_name("installed")
                .long("installed")
                .short("i")
                .takes_value(true)
                .help("Limit operations to just the installed games."),
        )
        .arg(
            Arg::with_name("refresh")
                .long("refresh")
                .help("Refresh the list of known games from the Twitch install."),
        )
        .arg(
            Arg::with_name("list")
                .long("list")
                .help("List the known games."),
        )
        .arg(
            Arg::with_name("json")
                .long("json")
                .help("Output data in json format"),
        )
        .arg(
            Arg::with_name("launch")
                .long("launch")
                .short("l")
                .takes_value(true)
                .help("Launch the specified game."),
        )
        .get_matches();

    let home = dirs::home_dir().unwrap();
    let twitch_cache = home.join(".twitch");
    let config = dirs::config_dir().unwrap();
    // TODO: Put cache file path in a central location.
    if matches.is_present("refresh") || !twitch_cache.join("twitchdb.json").exists() {
        println!("Refreshing Twitch game cache...");
        let products = TwitchDb::load_products(&config)?;
        let installs = TwitchDb::load_installs(&"c:/programdata".into())?;
        let twitch_db = TwitchDb { products, installs };
        twitch_db.save(&twitch_cache)?;
    }
    let mut games = TwitchGame::from_db(&TwitchDb::load(&twitch_cache)?)?;
    games.sort_unstable_by(|e1, e2| e1.title.cmp(&e2.title));

    if matches.is_present("list") {
        if let Some(installed) = matches.value_of("installed") {
            let installed = bool::from_str(installed)?;
            games = games
                .into_iter()
                .filter(|g| g.installed == installed)
                .collect();
        }
        if matches.is_present("json") {
            println!("{}", serde_json::to_string(&games)?);
        } else {
            for game in &games {
                println!("{}", game.title);
            }
        }
        return Ok(());
    }

    if let Some(game_to_launch) = matches.value_of("launch") {
        for game in &games {
            if game.title == game_to_launch {
                game.launch()?;
                return Ok(());
            }
        }
        eprintln!("Unable to find game {}", game_to_launch);
        exit(1);
    }

    Ok(())
}

fn main() {
    #[cfg(target_family = "unix")]
    {
        eprintln!("ERROR: This utility only works on Windows!");
        exit(1);
    }
    match run() {
        Ok(_) => (),
        Err(err) => {
            eprintln!("{}", err);
            exit(1);
        }
    }
}
