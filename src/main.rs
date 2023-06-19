use serde::Deserialize;

mod client;

#[derive(Debug, Deserialize)]
struct Config {
    username: String,
    password: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = envy::prefixed("CONF_").from_env::<Config>()?;

    let client = client::UmobileClient::new(config.username, config.password).await?;

    let usage = client.get_realtime_usage().await?;

    let bill_usage = client.get_realtime_bill().await?;

    println!("usage: {:?}", usage);
    println!("bill: {:?}", bill_usage);

    Ok(())
}
