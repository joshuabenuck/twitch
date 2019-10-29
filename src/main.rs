use clap::{App, Arg};
use dirs;
use failure::Error;
use ggez::event;
use ggez::{self, graphics, Context, GameResult};
use image;
use image_grid;
use image_grid::grid::{Grid, Tile, TileAction};
use reqwest;
use rusqlite::{Connection, NO_PARAMS};
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command};
use url::Url;

#[derive(Debug, Deserialize, Serialize)]
struct Product {
    id: String,
    date_time: String,
    background: String,
    background2: String,
    is_developer: isize,
    product_asin: String,
    product_asin_version: String,
    product_description: Option<String>,
    product_domain: String,
    product_icon_url: String,
    product_id_str: String,
    product_line: String,
    product_publisher: String,
    product_sku: String,
    product_title: String,
    screenshots_json: String,
    state: String,
    videos_json: String,
}

impl Product {}

struct Products {}

impl Products {
    fn load(config: &PathBuf) -> Result<Vec<Product>, Error> {
        let product_info_db: PathBuf = config.join("Twitch/Games/Sql/GameProductInfo.sqlite");
        assert!(product_info_db.exists());
        let product_info = Connection::open(product_info_db)?;
        let mut stmt = product_info.prepare("select * from DbSet;")?;
        let products = stmt.query_map(NO_PARAMS, |row| {
            Ok(Product {
                id: row.get(0)?,
                date_time: row.get(1)?,
                background: row.get(2)?,
                background2: row.get(3)?,
                is_developer: row.get(4)?,
                product_asin: row.get(5)?,
                product_asin_version: row.get(6)?,
                product_description: row.get(7)?,
                product_domain: row.get(8)?,
                product_icon_url: row.get(9)?,
                product_id_str: row.get(10)?,
                product_line: row.get(11)?,
                product_publisher: row.get(12)?,
                product_sku: row.get(13)?,
                product_title: row.get(14)?,
                screenshots_json: row.get(15)?,
                state: row.get(16)?,
                videos_json: row.get(17)?,
            })
        })?;
        Ok(products.into_iter().filter_map(Result::ok).collect())
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Install {
    id: String,
    install_date: String,
    install_directory: String,
    install_version: Option<String>,
    install_version_name: Option<String>,
    installed: isize,
    last_known_latest_version: String,
    last_known_latest_version_timestamp: String,
    last_updated: String,
    last_played: String,
    product_asin: String,
    product_title: String,
}

struct Installs {}

impl Installs {
    fn load(program_data: &PathBuf) -> Result<Vec<Install>, Error> {
        let install_info_db: PathBuf = program_data.join("Twitch/Games/Sql/GameInstallInfo.sqlite");
        assert!(install_info_db.exists());
        let install_info = Connection::open(install_info_db)?;
        let mut stmt = install_info.prepare("select * from DbSet;")?;
        let installs = stmt.query_map(NO_PARAMS, |row| {
            Ok(Install {
                // TODO: Switch to named columns to guard against additions / order changes
                id: row.get(0)?,
                install_date: row.get(1)?,
                install_directory: row.get(2)?,
                install_version: row.get(3)?,
                install_version_name: row.get(4)?,
                installed: row.get(5)?,
                last_known_latest_version: row.get(6)?,
                last_known_latest_version_timestamp: row.get(7)?,
                last_updated: row.get(8)?,
                last_played: row.get(9)?,
                product_asin: row.get(10)?,
                product_title: row.get(11)?,
            })
        })?;
        Ok(installs.into_iter().filter_map(Result::ok).collect())
    }
}

#[derive(Deserialize, Serialize)]
struct TwitchDb {
    products: Vec<Product>,
    installs: Vec<Install>,
}

impl TwitchDb {
    fn open_db(cache_dir: &PathBuf) -> Result<fs::File, Error> {
        Ok(fs::File::create(cache_dir.join("twitchdb.json"))?)
    }

    pub fn save(&self, cache_dir: &PathBuf) -> Result<(), Error> {
        TwitchDb::open_db(cache_dir)?.write(serde_json::to_string(&self)?.as_bytes())?;
        Ok(())
    }

    pub fn load(cache_dir: &PathBuf) -> Result<TwitchDb, Error> {
        Ok(serde_json::from_str(
            fs::read_to_string(cache_dir.join("twitchdb.json"))?.as_str(),
        )?)
    }
}

#[derive(Deserialize, Serialize)]
struct TwitchGame {
    asin: String,
    title: String,
    #[serde(skip)]
    image: Option<graphics::Image>,
    image_path: Option<PathBuf>,
    image_url: String,
    installed: bool,
    install_directory: Option<String>,
    kids: Option<bool>,
    players: Option<usize>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct FuelCommand {
    command: String,
    args: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Fuel {
    schema_version: String,
    post_install: Option<Vec<FuelCommand>>,
    main: FuelCommand,
}

impl TwitchGame {
    fn download_img(&self, path: &PathBuf) -> Result<PathBuf, Error> {
        assert!(path.exists(), "Path for image download does not exist!");
        let url = Url::parse(&self.image_url).expect("Unable to parse url for image");
        let filename = url
            .path_segments()
            .expect("Unable to segments from image url")
            .last()
            .expect("Unable to get filename from image url");
        let image = path.join(filename);
        if image.exists() {
            return Ok(image);
        }
        let mut resp = reqwest::get(url.as_str()).expect("Unable to retrieve image from url");
        assert!(resp.status().is_success());
        let mut buffer = Vec::new();
        resp.read_to_end(&mut buffer)?;
        fs::write(&image, buffer)?;
        Ok(image)
    }

    fn read_img(&self, full_path: &PathBuf) -> Result<Vec<u8>, Error> {
        Ok(fs::read(&full_path)?)
    }

    fn launch(&self) -> Result<Child, Error> {
        let install_directory = PathBuf::from(
            self.install_directory
                .as_ref()
                .expect("Unable to launch game"),
        );
        let fuel_config = install_directory.join("fuel.json");
        println!("Parsing launch config file: {}", fuel_config.display());
        let fuel_file = fs::File::open(fuel_config)?;
        let fuel: Fuel = serde_json::from_reader(fuel_file)?;
        let mut launch = Command::new(install_directory.join(&fuel.main.command));
        launch.args(&fuel.main.args);
        Ok(launch.spawn()?)
    }
}

impl Tile for TwitchGame {
    fn image(&self) -> &graphics::Image {
        &self.image.as_ref().unwrap()
    }

    fn act(&self) -> TileAction {
        TileAction::Launch(self.launch())
    }
}

struct Twitch {
    games: Vec<TwitchGame>,
    image_folder: PathBuf,
}

impl Twitch {
    fn load(cache_dir: PathBuf) -> Result<Twitch, Error> {
        let games: Vec<TwitchGame> = serde_json::from_str(
            fs::read_to_string(cache_dir.join("twitch_games.json"))?.as_str(),
        )?;
        let mut twitch = Twitch {
            games,
            image_folder: cache_dir.join("images"),
        };
        twitch.sort();
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

    fn from_db(image_folder: PathBuf, twitch_db: &TwitchDb) -> Twitch {
        Twitch::from(image_folder, &twitch_db.products, &twitch_db.installs)
    }

    fn from(image_folder: PathBuf, products: &Vec<Product>, installs: &Vec<Install>) -> Twitch {
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
                TwitchGame {
                    asin: p.product_asin.clone(),
                    title: p.product_title.clone(),
                    image: None,
                    image_url: p.product_icon_url.clone(),
                    image_path: None,
                    installed,
                    install_directory,
                    kids: None,
                    players: None,
                }
            })
            .collect();
        let mut twitch = Twitch {
            games,
            image_folder,
        };
        twitch.sort();
        twitch
    }

    fn load_imgs(&mut self, ctx: &mut Context) -> Result<&Twitch, Error> {
        for game in &mut self.games {
            game.image_path = Some(game.download_img(&self.image_folder).unwrap());
            let bytes = game.read_img(&game.image_path.as_ref().unwrap())?;
            let image = image::load_from_memory(&bytes)?.to_rgba();
            let (width, height) = image.dimensions();
            game.image = Some(graphics::Image::from_rgba8(
                ctx,
                width as u16,
                height as u16,
                &image,
            )?);
        }
        Ok(self)
    }

    fn sort(&mut self) {
        self.games
            .sort_unstable_by(|e1, e2| e1.title.cmp(&e2.title));
    }
}

fn main() -> Result<(), Error> {
    let matches = App::new("twitch")
        .about("Launcher for Twitch Prime games.")
        .arg(
            Arg::with_name("launcher")
                .long("launcher")
                .help("Display graphical launcher."),
        )
        .arg(
            Arg::with_name("installed")
                .long("installed")
                .short("i")
                .default_value("true")
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
    let mut twitch = if matches.is_present("refresh") {
        let products = Products::load(&config)?;
        let installs = Installs::load(&"c:/programdata".into())?;
        let twitch_db = TwitchDb { products, installs };
        twitch_db.save(&twitch_cache)?;
        let twitch = Twitch::load(twitch_cache.clone())?.merge_with(Twitch::from_db(
            image_folder,
            &TwitchDb::load(&twitch_cache)?,
        ));
        twitch.save(&twitch_cache)?;
        twitch
    } else {
        Twitch::load(twitch_cache.clone())?
    };

    if matches.is_present("launcher") {
        let cb = ggez::ContextBuilder::new("Image Grid", "Joshua Benuck");
        let (mut ctx, mut event_loop) = cb.build()?;
        // TODO: Add support for downloading of images without loading into textures
        twitch.load_imgs(&mut ctx)?;
        twitch.save(&twitch_cache)?;
        let mut grid = Grid::new(
            twitch
                .games
                .into_iter()
                .filter(|g| g.installed)
                .filter(|g| g.kids.unwrap_or(false))
                .map(|g| Box::new(g) as Box<dyn Tile>)
                .collect(),
            200,
            200,
        );
        graphics::set_resizable(&mut ctx, true)?;
        event::run(&mut ctx, &mut event_loop, &mut grid)?;
        return Ok(());
    }

    if matches.is_present("list") {
        let installed_only = matches.value_of("installed").unwrap().parse::<bool>()?;
        for game in twitch.games {
            if installed_only && !game.installed {
                continue;
            }
            println!("{}", game.title);
        }
        return Ok(());
    }

    if let Some(game_to_launch) = matches.value_of("launch") {
        for game in twitch.games {
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
