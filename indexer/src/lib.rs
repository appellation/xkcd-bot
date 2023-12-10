use std::time::Duration;

use anyhow::{Context, Error, Result};
use async_recursion::async_recursion;
use meilisearch_sdk::{indexes::Index, search::SearchResult, Client};
use tokio::{task::JoinSet, time::sleep};
use tracing::{info, instrument, warn, Span};
use xkcd::Comic;

const RANGKING_RULES: [&'static str; 7] = [
	"words",
	"typo",
	"proximity",
	"attribute",
	"sort",
	"exactness",
	"num:desc",
];

#[instrument(err, skip(client))]
#[async_recursion]
async fn load_comic(client: reqwest::Client, num: usize) -> Result<Option<Comic>> {
	let res = client
		.get(format!("https://xkcd.com/{}/info.0.json", num))
		.send()
		.await?;

	if res.status().is_success() {
		Ok(Some(
			res.json::<Comic>()
				.await
				.context(format!("comic num {}", num))?,
		))
	} else if res.status().is_server_error() {
		warn!("encountered server error; retrying in 3s");
		sleep(Duration::from_secs(3)).await;
		load_comic(client, num).await
	} else {
		warn!(?res);
		Ok(None)
	}
}

#[instrument(err, fields(end_num))]
async fn load_comics(start_num: usize) -> Result<Vec<Comic>> {
	let client = reqwest::Client::new();
	let first = client
		.get("https://xkcd.com/info.0.json")
		.send()
		.await?
		.json::<Comic>()
		.await?;

	Span::current().record("end_num", first.num);

	if start_num == first.num {
		info!("up-to-date; stopping loading");
		return Ok(vec![]);
	}

	let mut set = JoinSet::new();

	for num in start_num..first.num {
		set.spawn(load_comic(client.clone(), num));
	}

	let mut comics = Vec::with_capacity(set.len() + 1);
	comics.push(first);

	while let Some(maybe_comic) = set.join_next().await {
		if let Some(comic) = maybe_comic?? {
			comics.push(comic);
		}
	}

	Ok(comics)
}

#[instrument(skip_all, err)]
async fn configure_index(client: &Client, index: &Index) -> Result<()> {
	index
		.set_ranking_rules(RANGKING_RULES)
		.await?
		.wait_for_completion(&client, None, None)
		.await?;

	index
		.set_sortable_attributes(["num"])
		.await?
		.wait_for_completion(&client, None, None)
		.await?;

	Ok::<_, Error>(())
}

#[instrument(skip_all, err, fields(num_comics = comics.len()))]
async fn add_documents(client: &Client, index: &Index, comics: &[Comic]) -> Result<()> {
	index
		.add_documents(comics, Some("num"))
		.await?
		.wait_for_completion(&client, None, None)
		.await?;

	Ok(())
}

#[instrument(skip_all, err)]
pub async fn index_comics(client: &Client) -> Result<()> {
	let comics_idx = client.index("comics");
	configure_index(client, &comics_idx).await?;

	let results = comics_idx
		.search()
		.with_sort(&["num:desc"])
		.with_limit(1)
		.execute::<Comic>()
		.await?;

	let start_num = results
		.hits
		.get(0)
		.map(|SearchResult { result, .. }| result.num)
		.unwrap_or(1);

	let comics = load_comics(start_num).await?;
	if comics.len() > 0 {
		add_documents(&client, &comics_idx, &comics).await?;
	}

	Ok(())
}
