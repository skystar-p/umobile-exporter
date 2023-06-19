use std::time::Duration;

use serde::Deserialize;
use stopper::Stopper;

mod client;

#[derive(Clone, Debug, Deserialize)]
struct Config {
    username: String,
    password: String,

    interval: u64,
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

    tokio::spawn(fetch_loop(stopper.clone(), config.clone()));

    Ok(())
}

async fn fetch_loop(stopper: Stopper, config: Config) -> anyhow::Result<()> {
    loop {
        if stopper.is_stopped() {
            break;
        }

        let client = client::UmobileClient::new(&config.username, &config.password).await?;

        loop {
            let sleep = tokio::time::sleep(Duration::from_secs(config.interval));
            if stopper.stop_future(sleep).await.is_none() {
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
        }
    }

    Ok(())
}
