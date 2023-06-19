use std::{sync::Arc, time::Duration};

use serde::Deserialize;
use stopper::Stopper;
use tokio::sync::RwLock;

mod client;
mod handler;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    username: String,
    password: String,

    #[serde(default = "default_interval")]
    interval: u64,
    #[serde(default = "default_listen_port")]
    listen_port: u16,
}

const fn default_interval() -> u64 {
    600
}

const fn default_listen_port() -> u16 {
    8080
}

pub struct State {
    usage: client::Usage,
    bill: client::Bill,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config = envy::prefixed("CONF_").from_env::<Config>()?;

    let stopper = stopper::Stopper::new();
    ctrlc::set_handler({
        let stopper = stopper.clone();
        move || {
            tracing::info!("Signal received, shutting down...");
            stopper.stop();
        }
    })?;

    let state = Arc::new(tokio::sync::RwLock::new(State {
        usage: client::Usage::default(),
        bill: client::Bill::default(),
    }));

    tokio::spawn(fetch_loop(stopper.clone(), config.clone(), state.clone()));

    let server = handler::run_http_server(config.clone(), state.clone());

    tracing::info!("Listening on port {}", config.listen_port);
    stopper.stop_future(server).await;

    Ok(())
}

async fn fetch_loop(
    stopper: Stopper,
    config: Config,
    state: Arc<RwLock<State>>,
) -> anyhow::Result<()> {
    loop {
        if stopper.is_stopped() {
            break;
        }

        let client = client::UmobileClient::new(&config.username, &config.password).await?;
        tracing::info!("Logged in");
        let mut interval = tokio::time::interval(Duration::from_secs(config.interval));

        loop {
            if stopper.stop_future(interval.tick()).await.is_none() {
                break;
            }

            let usage = match client.get_realtime_usage().await {
                Ok(usage) => usage,
                Err(_) => {
                    tracing::error!("Failed to get realtime usage");
                    break;
                }
            };

            let bill = match client.get_realtime_bill().await {
                Ok(bill) => bill,
                Err(_) => {
                    tracing::error!("Failed to get realtime bill");
                    break;
                }
            };

            tracing::info!(
                "Fetched usage: data_used={}, call_used={}, sms_used={}, bill={}",
                usage.mobile_data_used.unwrap_or_default(),
                usage.call_used.unwrap_or_default(),
                usage.sms_used.unwrap_or_default(),
                bill.usage.unwrap_or_default(),
            );

            let mut state = state.write().await;
            state.usage.mobile_data_used = usage.mobile_data_used;
            state.usage.call_used = usage.call_used;
            state.usage.sms_used = usage.sms_used;
            state.bill.usage = bill.usage;
        }
    }

    Ok(())
}
