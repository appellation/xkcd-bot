use std::time::Duration;

use anyhow::Result;
use meilisearch_sdk::Client;
use tokio::time::interval;
use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};
use xkcd::Config;
use xkcd_indexer::index_comics;

#[tokio::main]
async fn main() -> Result<()> {
	tracing_subscriber::fmt()
		.with_env_filter(EnvFilter::from_default_env())
		.with_level(true)
		.with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
		.init();

	let config = Config::load();

	let client = Client::new(config.meilisearch_url, Some(config.meilisearch_api_key));
	let mut interval = interval(Duration::from_secs(21_600)); // 6 hrs

	loop {
		interval.tick().await;
		index_comics(&client).await?;
	}
}
