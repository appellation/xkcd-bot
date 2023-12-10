use std::env;

use serde::{Deserialize, Serialize};
use twilight_model::id::{marker::ApplicationMarker, Id};

#[derive(Debug, Serialize, Deserialize)]
pub struct Comic {
	pub month: String,
	pub num: usize,
	pub link: String,
	pub year: String,
	pub news: String,
	pub safe_title: String,
	pub transcript: String,
	pub alt: String,
	pub img: String,
	pub title: String,
	pub day: String,
}

pub struct Config {
	pub discord_token: String,
	pub discord_id: Id<ApplicationMarker>,
	pub meilisearch_url: String,
	pub meilisearch_api_key: String,
}

const MEILISEARCH_API_KEY: &'static str = "test";

impl Config {
	pub fn load() -> Self {
		let _ = dotenvy::dotenv();

		Self {
			discord_token: env::var("DISCORD_TOKEN").unwrap(),
			discord_id: env::var("DISCORD_APP_ID")
				.unwrap()
				.parse::<Id<ApplicationMarker>>()
				.unwrap(),
			meilisearch_url: env::var("MEILISEARCH_URL")
				.unwrap_or("http://localhost:7700".to_string()),
			meilisearch_api_key: MEILISEARCH_API_KEY.to_string(),
		}
	}
}
