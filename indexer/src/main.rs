use std::time::Duration;

use anyhow::Result;
use clokwerk::{AsyncScheduler, Job, TimeUnits};
use meilisearch_sdk::Client;
use tokio::time::sleep;
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

	let mut scheduler = AsyncScheduler::new();
	let config = Config::load();

	let client = Client::new(config.meilisearch_url, config.meilisearch_api_key);
	index_comics(&client).await?;

	scheduler.every(1.day()).at("1:00 am").run(move || {
		let client = client.clone();
		async move {
			index_comics(&client).await.unwrap();
		}
	});

	loop {
		scheduler.run_pending().await;
		sleep(Duration::from_millis(500)).await;
	}
}
