use crate::twitch_db::{Install, TwitchDb};
use anyhow::{anyhow, Error};
use log::debug;
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs;
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
    pub launch_url: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
#[allow(dead_code)]
struct FuelCommand {
    working_subdir_override: Option<String>,
    command: String,
    args: Option<Vec<String>>,
    auth_scopes: Option<Vec<String>>,
    client_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
#[allow(dead_code)]
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
                let mut launch_url = None;
                let mut command: Option<String> = None;
                let mut args = None;
                if installed {
                    let install_directory = PathBuf::from(
                        install_directory
                            .as_ref()
                            .expect("Unable to find game launch command"),
                    );
                    let game_id = install_directory.file_stem().unwrap();
                    let fuel_config = install_directory.join("fuel.json");
                    debug!("Parsing launch config file: {}", fuel_config.display());
                    let fuel_file = fs::File::open(fuel_config).unwrap();
                    let fuel: Fuel = serde_json::from_reader(fuel_file).unwrap();
                    if fuel.main.client_id.is_some() {
                        launch_url = Some(format!(
                            "twitch://fuel-launch/{}",
                            game_id.to_str().unwrap()
                        ));
                    } else {
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
                }
                TwitchGame {
                    asin: p.product_asin.clone(),
                    title: p.product_title.clone(),
                    image_url: p.product_icon_url.clone(),
                    installed,
                    install_directory,
                    command,
                    args,
                    launch_url,
                    working_subdir_override,
                }
            })
            .collect();
        Ok(games)
    }

    pub fn launch(&self) -> Result<Child, Error> {
        debug!(
            "Launching {:?} {:?} {:?} {:?} {:?}",
            self.install_directory,
            self.working_subdir_override,
            self.command,
            self.args,
            self.launch_url
        );
        if self.install_directory.is_some() && self.command.is_some() {
            let install_directory = PathBuf::from(
                self.install_directory
                    .as_ref()
                    .expect("Unable to launch game"),
            );
            let full_command =
                PathBuf::from(install_directory.join(self.command.as_ref().unwrap()));
            let mut launch = Command::new(&full_command);
            if self.working_subdir_override.is_some() {
                launch.current_dir(
                    install_directory.join(self.working_subdir_override.as_ref().unwrap()),
                );
            } else {
                launch.current_dir(install_directory);
            }
            launch.args(self.args.as_ref().unwrap());
            return Ok(launch.spawn()?);
        }
        if self.launch_url.is_some() {
            let mut launch = Command::new("cmd");
            launch.args(&["/C", "start", self.launch_url.as_ref().unwrap()]);
            return Ok(launch.spawn()?);
        }
        Err(anyhow!("Unable to launch: Missing launch_url or command"))
    }
}
