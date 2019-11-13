extern crate twitch;

use clap::{App, Arg};
use dirs;
use failure::Error;
use serde_json;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;
use twitch::{TwitchDb, TwitchGame};

struct Twitch {
    games: Vec<TwitchGame>,
}

impl Twitch {
    fn load(cache_dir: PathBuf) -> Result<Twitch, Error> {
        let games: Vec<TwitchGame> = serde_json::from_str(
            fs::read_to_string(cache_dir.join("twitch_games.json"))?.as_str(),
        )?;
        let mut twitch = Twitch { games };
        Ok(twitch)
    }

    fn save(&self, cache_dir: &PathBuf) -> Result<(), Error> {
        fs::File::create(cache_dir.join("twitch_games.json"))?
            .write(serde_json::to_string_pretty(&self.games)?.as_bytes())?;
        Ok(())
    }

    fn merge_with(mut self, other: Twitch) -> Twitch {
        let mut to_add: Vec<TwitchGame> = Vec::new();
        for orig in other.games.into_iter() {
            let mut found = false;
            for custom in &mut self.games {
                if orig.asin == custom.asin {
                    found = true;
                    custom.title = orig.title.clone();
                    custom.image_url = orig.image_url.clone();
                    custom.install_directory = orig.install_directory.clone();
                    custom.installed = orig.installed.clone();
                }
            }
            if !found {
                to_add.push(orig);
            }
        }
        self.games.extend(to_add);
        self
    }
}

fn main() -> Result<(), Error> {
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
    let image_folder = twitch_cache.join("images");
    let config = dirs::config_dir().unwrap();
    let mut games = {
        let products = TwitchDb::load_products(&config)?;
        let installs = TwitchDb::load_installs(&"c:/programdata".into())?;
        let twitch_db = TwitchDb { products, installs };
        twitch_db.save(&twitch_cache)?;
        let games = TwitchGame::from_db(&TwitchDb::load(&twitch_cache)?)?;
        games
    };

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
        println!("Unable to find game {}", game_to_launch);
        return Ok(());
    }

    Ok(())
}
