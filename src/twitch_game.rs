use crate::twitch_db::{Install, Product, TwitchDb};
use failure::{err_msg, Error};
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Child, Command};

#[derive(Deserialize, Serialize)]
pub struct TwitchGame {
    pub asin: String,
    pub title: String,
    pub image_url: String,
    pub installed: bool,
    pub install_directory: Option<String>,
    pub working_subdir_override: Option<String>,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct FuelCommand {
    working_subdir_override: Option<String>,
    command: String,
    args: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Fuel {
    schema_version: String,
    post_install: Option<Vec<FuelCommand>>,
    main: FuelCommand,
}

impl TwitchGame {
    pub fn from_db(twitch_db: &TwitchDb) -> Result<Vec<TwitchGame>, Error> {
        let products = &twitch_db.products;
        let installs = &twitch_db.installs;
        let games: Vec<TwitchGame> = products
            .iter()
            .map(|p| {
                let mut installed = false;
                let mut install_directory = None;
                let install_record: Vec<&Install> = installs
                    .iter()
                    .filter(|i| i.product_asin == p.product_asin)
                    .collect();
                if install_record.len() == 1 {
                    let install_record = install_record[0];
                    installed = install_record.installed == 1;
                    install_directory = Some(install_record.install_directory.clone());
                }
                let mut working_subdir_override = None;
                let mut command: Option<String> = None;
                let mut args = None;
                if installed {
                    let install_directory = PathBuf::from(
                        install_directory
                            .as_ref()
                            .expect("Unable to find game launch command"),
                    );
                    let fuel_config = install_directory.join("fuel.json");
                    println!("Parsing launch config file: {}", fuel_config.display());
                    let fuel_file = fs::File::open(fuel_config).unwrap();
                    let fuel: Fuel = serde_json::from_reader(fuel_file).unwrap();
                    command = Some(
                        install_directory
                            .join(&fuel.main.command)
                            .to_str()
                            .unwrap()
                            .to_owned(),
                    );
                    args = fuel.main.args;
                    if fuel.main.working_subdir_override.is_some() {
                        working_subdir_override = fuel.main.working_subdir_override;
                    }
                }
                TwitchGame {
                    asin: p.product_asin.clone(),
                    title: p.product_title.clone(),
                    image_url: p.product_icon_url.clone(),
                    installed,
                    install_directory,
                    command,
                    args,
                    working_subdir_override,
                }
            })
            .collect();
        Ok(games)
    }

    fn load(cache_dir: PathBuf) -> Result<Vec<TwitchGame>, Error> {
        let games: Vec<TwitchGame> = serde_json::from_str(
            fs::read_to_string(cache_dir.join("twitch_games.json"))?.as_str(),
        )?;
        Ok(games)
    }

    fn save(cache_dir: &PathBuf, games: &Vec<TwitchGame>) -> Result<(), Error> {
        fs::File::create(cache_dir.join("twitch_games.json"))?
            .write(serde_json::to_string_pretty(games)?.as_bytes())?;
        Ok(())
    }

    pub fn launch(&self) -> Result<Child, Error> {
        let install_directory = PathBuf::from(
            self.install_directory
                .as_ref()
                .expect("Unable to launch game"),
        );
        if self.command.is_none() {
            return Err(err_msg(format!("Unable to launch game {}", self.title)));
        }
        let mut launch = Command::new(install_directory.join(self.command.as_ref().unwrap()));
        if let Some(args) = &self.args {
            launch.args(args);
        }
        Ok(launch.spawn()?)
    }
}
