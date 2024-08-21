use std::fs;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use env_logger::Builder;
use log::{debug, info, LevelFilter};
use serde::Deserialize;
use tokio::time::sleep;

mod hcaptcha;
mod discord;

use crate::hcaptcha::HCaptcha;

#[derive(Deserialize, Debug)]
struct Config {
    interval: u64,
    discord_webhook: String,
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
    let file = fs::File::open("config.json").context("Failed to open config.json")?;

    debug!("Parsing config.json");
    let config: Config = serde_json::from_reader(file).context("Failed to parse config.json")?;

    info!("Successfully loaded configuration");
    Ok(config)
}

#[tokio::main]
async fn main() -> Result<()> {
    Builder::new().filter_level(LevelFilter::Info).init();

    fs::create_dir_all("output").context("Failed to create output directory")?;

    info!("Starting application");
    let config = load_config().context("Failed to load configuration")?;

    for website in &config.websites {
        fs::create_dir_all(format!("output/{}", website.host))
            .context("Failed to create website output directory")?;
    }

    info!("Configured downloads: {}", config.scripts.join(", "));
    info!("Loaded {} websites", config.websites.len());
    process_websites(&config).await
}

async fn process_websites(config: &Config) -> Result<()> {
    let clients: Vec<HCaptcha> = config.websites
        .iter()
        .map(|website| HCaptcha::new(&website.host, &website.sitekey))
        .collect();

    let webhook = discord::DiscordWebhook::new(config.discord_webhook.clone());
    info!("Checking on a {} second interval.\n", config.interval);

    loop {
        for client in &clients {
            if let Ok(result) = client.get_version().await {
                if let Ok(url) = client.get_resource_url(result.clone()).await {
                    let script_version = url.split('/').last().unwrap();
                    let dir_path = format!("output/{}/{}", client.host, script_version);
                    if fs::metadata(&dir_path).is_ok() {
                        continue;
                    }

                    info!("Located version: {}/{}", result, script_version);
                    fs::create_dir_all(&dir_path).context("Failed to create directory")?;

                    let _ = webhook.send_embed(
                        "hCaptcha Monitor",
                        "New hCaptcha version detected.",
                        0x2B2D31,
                        vec![
                            ("Website", &client.host, false),
                            ("Sitekey", &client.sitekey, false),
                            ("Version", &result, false),
                            ("Resource URL", &format!("[View]({})", url), false),
                        ],
                    ).await;

                    for script in &config.scripts {
                        if let Err(e) = client.download_contents(&url, script, Path::new(&dir_path)).await {
                            info!("Error downloading {}: {}", script, e);
                        }
                    }

                    info!("Archiving complete");
                } else {
                    info!("Error getting resource URL for {}", client.host);
                }
            } else {
                info!("Error checking hCaptcha version for {}", client.host);
            }
        }

        sleep(Duration::from_secs(config.interval)).await;
    }
}