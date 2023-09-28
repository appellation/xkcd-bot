use std::sync::Arc;

use anyhow::{bail, ensure, Result};
use futures::StreamExt;
use meilisearch_sdk::{search::SearchResult, Client};
use tracing_subscriber::EnvFilter;
use twilight_gateway::{
	stream::{self, ShardEventStream},
	Event, Intents,
};
use twilight_model::{
	application::{
		command::{CommandOptionChoice, CommandOptionChoiceValue, CommandOptionType},
		interaction::{
			application_command::{CommandData, CommandDataOption, CommandOptionValue},
			Interaction, InteractionData,
		},
	},
	channel::message::MessageFlags,
	http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
	id::{marker::ApplicationMarker, Id},
};
use xkcd::{Comic, Config};

#[tokio::main]
async fn main() -> Result<()> {
	tracing_subscriber::fmt()
		.with_env_filter(EnvFilter::from_default_env())
		.with_level(true)
		.init();

	let config = Config::load();
	let search = Client::new(config.meilisearch_url, config.meilisearch_api_key);

	let rest = Arc::new(twilight_http::Client::new(config.discord_token.clone()));
	let mut shards = stream::create_recommended(
		&rest,
		twilight_gateway::Config::new(config.discord_token, Intents::empty()),
		|_, builder| builder.build(),
	)
	.await?
	.collect::<Vec<_>>();

	let mut events = ShardEventStream::new(shards.iter_mut());

	while let Some((_, event)) = events.next().await {
		match event {
			Ok(Event::InteractionCreate(interaction)) => {
				handle_interaction_create(
					config.discord_id,
					search.clone(),
					rest.clone(),
					interaction.0,
				)
				.await?;
			}
			ev => eprintln!("{:?}", ev),
		}
	}

	Ok(())
}

async fn handle_interaction_create(
	discord_app_id: Id<ApplicationMarker>,
	search: Client,
	rest: Arc<twilight_http::Client>,
	interaction: Interaction,
) -> Result<()> {
	let response = match interaction.data {
		Some(InteractionData::ApplicationCommand(data)) if data.name == "xkcd" => {
			handle_xkcd(search, *data).await
		}
		_ => bail!("unknown interaction"),
	}
	.unwrap_or_else(|e| InteractionResponse {
		kind: InteractionResponseType::ChannelMessageWithSource,
		data: Some(InteractionResponseData {
			content: Some(e.to_string()),
			flags: Some(MessageFlags::EPHEMERAL),
			..Default::default()
		}),
	});

	rest.interaction(discord_app_id)
		.create_response(interaction.id, &interaction.token, &response)
		.await?;

	Ok(())
}

async fn handle_xkcd(client: Client, data: CommandData) -> Result<InteractionResponse> {
	let CommandDataOption { name, value } = &data.options[0];
	ensure!(name == "query");

	match value {
		CommandOptionValue::Focused(query, CommandOptionType::String) => {
			let results = client
				.index("comics")
				.search()
				.with_query(query)
				.with_limit(20)
				.execute::<Comic>()
				.await?;

			Ok(InteractionResponse {
				kind: InteractionResponseType::ApplicationCommandAutocompleteResult,
				data: Some(InteractionResponseData {
					choices: Some(
						results
							.hits
							.into_iter()
							.map(|SearchResult { result, .. }| CommandOptionChoice {
								name: format!("{}: {}", result.num, result.title),
								value: CommandOptionChoiceValue::String(result.num.to_string()),
								name_localizations: None,
							})
							.collect(),
					),
					..Default::default()
				}),
			})
		}
		CommandOptionValue::String(id_or_query) => {
			if id_or_query.chars().all(|ch| ch.is_digit(10)) {
				Ok(InteractionResponse {
					kind: InteractionResponseType::ChannelMessageWithSource,
					data: Some(InteractionResponseData {
						content: Some(format!("https://xkcd.com/{}/", id_or_query)),
						..Default::default()
					}),
				})
			} else {
				let results = client
					.index("comics")
					.search()
					.with_query(id_or_query)
					.with_limit(5)
					.execute::<Comic>()
					.await?;

				Ok(InteractionResponse {
					kind: InteractionResponseType::ChannelMessageWithSource,
					data: Some(InteractionResponseData {
						content: Some(
							results
								.hits
								.into_iter()
								.map(|SearchResult { result, .. }| {
									format!("{}: <https://xkcd.com/{}/>", result.title, result.num)
								})
								.collect::<Vec<_>>()
								.join("\n"),
						),
						..Default::default()
					}),
				})
			}
		}
		_ => bail!("unexpected command data value"),
	}
}
