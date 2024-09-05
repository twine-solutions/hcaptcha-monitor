use std::path::Path;

use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use regex::Regex;
use reqwest::Client;
use serde_json::Value;

pub struct HCaptcha {
    client: Client,
    pub host: String,
    pub sitekey: String,
}

impl HCaptcha {
    pub fn new(host: &str, sitekey: &str) -> Self {
        Self {
            client: Client::new(),
            host: host.to_string(),
            sitekey: sitekey.to_string(),
        }
    }

    pub async fn download_contents(&self, url: &str, file: &str, path: &Path) -> Result<()> {
        let url_path = format!("https://newassets.hcaptcha.com{}/{}", url, file);
        let response = self.client.get(&url_path)
            .send()
            .await
            .context("Failed to fetch hCaptcha resource")?;

        let content = response.bytes().await
            .context("Failed to get bytes from response")?;

        let file_path = path.join(file);
        let comment = format!("/* Source URL: {} */\n", url_path);
        let mut final_content = comment.into_bytes();
        final_content.extend_from_slice(&content);

        std::fs::write(&file_path, &final_content)
            .context("Failed to write hCaptcha resource to file")
    }

    pub async fn get_resource_url(&self, version: String) -> Result<String> {
        let response = self.client.get("https://api2.hcaptcha.com/checksiteconfig")
            .query(&[
                ("v", &version),
                ("host", &self.host),
                ("sitekey", &self.sitekey)
            ])
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/58.0.3029.110 Safari/537.3")
            .send()
            .await
            .context("Failed to fetch hCaptcha api script")?;

        let content = response.text().await
            .context("Failed to get text from response")?;

        let json: Value = serde_json::from_str(&content)
            .context("Failed to parse JSON response")?;

        let req = json["c"]["req"].as_str()
            .context("Failed to extract 'req' field from JSON")?;

        let payload = req.split('.').nth(1)
            .context("Failed to extract payload from JWT")?;

        let payload_padded = match payload.len() % 4 {
            0 => payload.to_string(),
            2 => format!("{}==", payload),
            3 => format!("{}=", payload),
            _ => payload.to_string(),
        };

        let decoded_bytes = general_purpose::STANDARD.decode(payload_padded)
            .context("Failed to decode base64")?;

        let decoded_payload = String::from_utf8(decoded_bytes)
            .context("Failed to convert decoded bytes to UTF-8")?;

        let payload_json: Value = serde_json::from_str(&decoded_payload)
            .context("Failed to parse decoded payload as JSON")?;

        payload_json["l"].as_str()
            .context("Failed to extract 'l' field from payload JSON")
            .map(ToString::to_string)
    }

    pub async fn get_version(&self) -> Result<String> {
        let response = self.client.get("https://hcaptcha.com/1/api.js?render=explicit&onload=hcaptchaOnLoad")
            .send()
            .await
            .context("Failed to fetch hCaptcha api script")?;

        let content = response.text().await
            .context("Failed to get text from response")?;

        let version_regex = Regex::new(r"/captcha/v1/([a-f0-9]+)/static")?;
        version_regex.captures(&content)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().to_string())
            .context("Failed to extract hCaptcha version")
    }
}