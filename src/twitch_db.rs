use anyhow::{anyhow, Error};
use rusqlite::{Connection, NO_PARAMS};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
pub struct Product {
    pub id: String,
    pub date_time: String,
    pub background: String,
    pub background2: String,
    pub is_developer: isize,
    pub product_asin: String,
    pub product_asin_version: String,
    pub product_description: Option<String>,
    pub product_domain: String,
    pub product_icon_url: String,
    pub product_id_str: String,
    pub product_line: String,
    pub product_publisher: String,
    pub product_sku: String,
    pub product_title: String,
    pub screenshots_json: String,
    pub state: String,
    pub videos_json: String,
}

impl Product {}

#[derive(Debug, Deserialize, Serialize)]
pub struct Install {
    pub id: String,
    pub install_date: String,
    pub install_directory: String,
    pub install_version: Option<String>,
    pub install_version_name: Option<String>,
    pub installed: isize,
    pub last_known_latest_version: String,
    pub last_known_latest_version_timestamp: String,
    pub last_updated: String,
    pub last_played: String,
    pub product_asin: String,
    pub product_title: String,
}

#[derive(Deserialize, Serialize)]
pub struct TwitchDb {
    pub products: Vec<Product>,
    pub installs: Vec<Install>,
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

    pub fn load_products(config: &PathBuf) -> Result<Vec<Product>, Error> {
        let product_info_db: PathBuf = config.join(r"Twitch\Games\Sql\GameProductInfo.sqlite");
        if !product_info_db.exists() {
            return Err(anyhow!(
                "Product info missing: {}",
                &product_info_db.display()
            ));
        }
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

    pub fn load_installs(program_data: &PathBuf) -> Result<Vec<Install>, Error> {
        let install_info_db: PathBuf =
            program_data.join(r"Twitch\Games\Sql\GameInstallInfo.sqlite");
        if !install_info_db.exists() {
            return Err(anyhow!(
                "Install info missing: {}",
                &install_info_db.display()
            ));
        }
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
