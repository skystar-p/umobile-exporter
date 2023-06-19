mod client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = client::UmobileClient::new_with_id_password("id", "password").await?;

    Ok(())
}
