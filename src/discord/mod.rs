use reqwest::Client;
use serde_json::json;
use std::error::Error;

pub struct DiscordWebhook {
    url: String,
    client: Client,
}

impl DiscordWebhook {
    pub fn new(url: String) -> Self {
        Self {
            url,
            client: Client::new(),
        }
    }

    pub async fn send_embed(
        &self,
        title: &str,
        description: &str,
        color: u32,
        fields: Vec<(&str, &str, bool)>,
    ) -> Result<(), Box<dyn Error>> {
        let embed = json!({
            "username": "hCaptcha Monitor",
            "avatar_url": "https://i.imgur.com/lhYfz5H.png",
            "embeds": [{
                "title": title,
                "description": description,
                "color": color,
                "thumbnail": {
                    "url": "https://i.imgur.com/lhYfz5H.png"
                },
                "fields": fields.iter().map(|(name, value, inline)| {
                    json!({
                        "name": name,
                        "value": value,
                        "inline": inline
                    })
                }).collect::<Vec<_>>()
            }]
        });

        let response = self.client.post(&self.url).json(&embed).send().await?;

        if !response.status().is_success() {
            return Err(format!("Failed to send webhook: {}", response.status()).into());
        }

        Ok(())
    }
}