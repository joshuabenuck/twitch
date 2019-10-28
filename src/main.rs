use dirs;
use failure::Error;
use ggez::event;
use ggez::{self, graphics, Context, GameResult};
use image;
use image_grid;
use image_grid::grid::{Grid, Tile};
use reqwest;
use rusqlite::{Connection, NO_PARAMS};
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use url::Url;

#[derive(Debug)]
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

#[derive(Debug)]
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

struct TwitchGame {
    asin: String,
    title: String,
    image: Option<graphics::Image>,
    image_path: Option<PathBuf>,
    image_url: String,
    installed: bool,
    install_directory: Option<String>,
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
}

impl Tile for TwitchGame {
    fn image(&self) -> &graphics::Image {
        &self.image.as_ref().unwrap()
    }
}

struct Twitch {
    games: Vec<TwitchGame>,
    image_folder: PathBuf,
}

impl Twitch {
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
                }
            })
            .collect();
        Twitch {
            games,
            image_folder,
        }
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
}

fn main() -> Result<(), Error> {
    let cb = ggez::ContextBuilder::new("Image Grid", "Joshua Benuck");
    let (mut ctx, mut event_loop) = cb.build()?;

    let home = dirs::home_dir().unwrap();
    let image_folder = home.join(".twitch").join("images");
    let config = dirs::config_dir().unwrap();
    let products = Products::load(&config)?;
    let installs = Installs::load(&"c:/programdata".into())?;

    let mut twitch = Twitch::from(image_folder, &products, &installs);
    twitch.load_imgs(&mut ctx)?;
    twitch.games.sort_unstable_by_key(|g| g.title.clone());

    let mut grid = Grid::new(
        twitch
            .games
            .into_iter()
            .filter(|g| g.installed)
            .map(|g| Box::new(g) as Box<dyn Tile>)
            .collect(),
        200,
        200,
    );
    graphics::set_resizable(&mut ctx, true)?;
    event::run(&mut ctx, &mut event_loop, &mut grid)?;
    Ok(())
}
