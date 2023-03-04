use anyhow::{Context, Error, Result};
use meilisearch_sdk::{
	documents::{DocumentsQuery, DocumentsResults},
	Client,
};
use tokio::task::JoinSet;
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

async fn load_comics(start_num: usize) -> Result<Vec<Comic>> {
	let client = reqwest::Client::new();
	let first = client
		.get("https://xkcd.com/info.0.json")
		.send()
		.await?
		.json::<Comic>()
		.await?;

	if start_num == first.num {
		return Ok(vec![]);
	}

	let mut set = JoinSet::new();

	for num in start_num..first.num {
		let client = client.clone();

		set.spawn(async move {
			let res = client
				.get(format!("https://xkcd.com/{}/info.0.json", num))
				.send()
				.await?;

			Ok::<_, Error>(if res.status().is_success() {
				Some(
					res.json::<Comic>()
						.await
						.context(format!("comic num {}", num))?,
				)
			} else {
				None
			})
		});
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

pub async fn index_comics(client: &Client) -> Result<()> {
	let comics_idx = client.index("comics");
	let start_num = comics_idx
		.get_documents_with::<Comic>(DocumentsQuery::new(&comics_idx).with_limit(1))
		.await
		.map(|DocumentsResults { results, .. }| results)
		.unwrap_or(vec![])
		.get(0)
		.map(|comic| comic.num)
		.unwrap_or(1);

	let comics = load_comics(start_num).await?;

	let task = comics_idx.add_documents(&comics, Some("num")).await?;
	client.wait_for_task(task, None, None).await?;

	let task = comics_idx.set_ranking_rules(RANGKING_RULES).await?;
	client.wait_for_task(task, None, None).await?;

	Ok(())
}
