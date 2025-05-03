include!(concat!(env!("OUT_DIR"), "/posthog_api_key.rs"));

use std::collections::HashMap;
use std::env::consts::{ARCH, OS};
use crate::config::Config;
use once_cell::sync::Lazy;
use reqwest::Client;
use serde_json::{json, Value};
use tokio::time::{sleep, Duration};
use chrono::Utc;
use log::{debug, error};

static HTTP_CLIENT: Lazy<Client> = Lazy::new(|| Client::builder().timeout(Duration::from_secs(5)).build().unwrap());

#[derive(thiserror::Error, Debug)]
pub enum TelemetryError {
    #[error("config error: {0}")]
    Config(anyhow::Error),
    #[error("http error: {0}")]
    Http(reqwest::Error),
    #[error("status code: {0}")]
    Status(reqwest::StatusCode),
}

// Only StatusCode needs manual From since it’s not an Error
impl From<reqwest::StatusCode> for TelemetryError {
    fn from(code: reqwest::StatusCode) -> Self {
        TelemetryError::Status(code)
    }
}

pub async fn publish_event(event: &str, properties: HashMap<&str, Value>) -> Result<(), TelemetryError> {
    if POSTHOG_API_KEY.is_empty() || POSTHOG_API_KEY == "undefined" {
        debug!("POSTHOG_API_KEY is not set");
        return Ok(());
    }
    let config = Config::load().map_err(TelemetryError::Config)?;
    if !config.metrics_enabled {
        debug!("Metrics are disabled");
        return Ok(());
    }

    // build mutable props with additional info
    let mut props = properties; // HashMap<&str, Value>
    props.insert("arch",     Value::String(ARCH.to_string()));
    props.insert("os",       Value::String(OS.to_string()));
    props.insert("version",  Value::String(env!("CARGO_PKG_VERSION").to_string()));
    props.insert("distinct_id", Value::String(config.user_id.clone()));

    let payload = json!({
        "api_key": POSTHOG_API_KEY,
        "event": event,
        "properties": props,
        "timestamp": Utc::now().to_rfc3339(),
    });

    // retry up to 3 times on failure
    // let url = "https://eu.i.posthog.com/capture/";
    let url = "https://eu.i.posthog.com/i/v0/e/";
    for attempt in 1..=3 {
        match HTTP_CLIENT.post(url).json(&payload).send().await {
            Ok(resp) if resp.status().is_success() => {
                debug!("Event sent: {}", event);
                return Ok(());
            }
            Ok(resp) if resp.status().is_client_error() || resp.status().is_server_error() => {
                if attempt == 3 { return Err(TelemetryError::Status(resp.status())); }
            }
            Err(e) => {
                error!("Attempt {} error: {}", attempt, e);
                if attempt == 3 { return Err(TelemetryError::Http(e)); }
            }
            _ => {}
        }
        sleep(Duration::from_secs(2u64.pow(attempt - 1))).await;
    }
    Ok(())
}
