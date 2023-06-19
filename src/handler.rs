use std::sync::Arc;

use axum::{routing::get, Router, Server};
use tokio::sync::RwLock;

use crate::{Config, State};

pub async fn run_http_server(config: Config, state: Arc<RwLock<State>>) -> anyhow::Result<()> {
    let app = Router::new().route(
        "/metrics",
        get({
            let state = Arc::clone(&state);
            move || metric_handler(state)
        }),
    );

    let addr = format!("0.0.0.0:{}", config.listen_port);
    Server::bind(&addr.parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

async fn metric_handler(state: Arc<RwLock<State>>) -> String {
    let state = state.read().await;
    let response = format!(
        "
# HELP data_used Mobile data used, in GB
# TYPE data_used gauge
data_used {}

# HELP call_used Call time used, in minutes
# TYPE call_used gauge
call_used {}

# HELP sms_used SMS used, in count
# TYPE sms_used gauge
sms_used {}

# HELP bill_usage Bill usage, in KRW
# TYPE bill_usage gauge
bill_usage {}
",
        state.usage.mobile_data_used.unwrap_or_default(),
        state.usage.call_used.unwrap_or_default(),
        state.usage.sms_used.unwrap_or_default(),
        state.bill.usage.unwrap_or_default(),
    );
    drop(state);

    response
}
