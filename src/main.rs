extern crate core;

use std::fs;
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

use anyhow::{Context, Result};
use env_logger::Builder;
use lazy_static::lazy_static;
use log::{debug, info, LevelFilter};
use serde::Deserialize;
use tokio::time::sleep;

mod hcaptcha;
use crate::hcaptcha::HCaptcha;

lazy_static! {
    static ref VERSION: Mutex<String> = Mutex::new(String::new());
}

#[derive(Deserialize, Debug)]
struct Config {
    interval: u64,
    websites: Vec<Website>,
    scripts: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct Website {
    host: String,
    sitekey: String,
}

fn load_config() -> Result<Config> {
    debug!("Attempting to open config.json");
    let file = fs::File::open("config.json")
        .context("Failed to open config.json")?;

    debug!("Parsing config.json");
    let config: Config = serde_json::from_reader(file)
        .context("Failed to parse config.json")?;

    info!("Successfully loaded configuration");
    Ok(config)
}

#[tokio::main]
async fn main() -> Result<()> {
    Builder::new()
        .filter_level(LevelFilter::Info)
        .init();

    fs::create_dir_all("output").context("Failed to create output directory")?;

    info!("Starting application");
    let config = load_config().context("Failed to load configuration")?;
    println!();

    for website in &config.websites {
        fs::create_dir_all(format!("output/{}", website.host))
            .context("Failed to create website output directory")?;
    }

    info!("Configured downloads: {}", config.scripts.join(", "));
    info!("Loaded {} websites", config.websites.len());
    process_websites(&config).await
}

async fn process_websites(config: &Config) -> Result<()> {
    let clients: Vec<HCaptcha> = config.websites.iter()
        .map(|website| HCaptcha::new(&website.host, &website.sitekey, &VERSION))
        .collect();

    info!("Checking on a {} second interval.\n", config.interval);

    loop {
        for client in &clients {
            info!("Locating version for {} ({})", client.host, client.sitekey);

            match client.check().await {
                Ok(result) => {
                    if result.version_changed {
                        info!("hCaptcha version for {} has changed: {:?} -> {}",
                              client.host, result.previous_version, result.version);

                        let dir_path = format!("output/{}/{}", client.host, result.version);
                        fs::create_dir_all(&dir_path).context("Failed to create directory")?;

                        info!("Downloading hCaptcha contents for {}", client.host);
                        if let Ok(url) = client.get_resource_url(result.version.clone()).await {
                            let resource_url = format!("https://newassets.hcaptcha.com{}", url);
                            for script in &config.scripts {
                                match client.download_contents(&resource_url, script, Path::new(&dir_path)).await {
                                    Ok(_) => info!("Downloaded {} to {}/{}", script, dir_path, script),
                                    Err(e) => info!("Error downloading {}: {}", script, e),
                                }
                            }
                        } else {
                            info!("Error getting resource URL for {}", client.host);
                        }
                    } else {
                        info!("hCaptcha version for {} is unchanged: {}", client.host, result.version);
                    }
                },
                Err(e) => info!("Error checking hCaptcha version for {}: {}", client.host, e),
            }

            println!();
        }

        sleep(Duration::from_secs(config.interval)).await;
    }
}