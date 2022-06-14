use futures::TryStreamExt;
use serde::{Serialize, Deserialize};
use mongodb::{
	bson::{
		doc,
		Document,
		oid::ObjectId,
	}, 
	Collection
};
use serenity::{
	framework::{
		standard::{
			macros::{
				command
			},
			Args,
			CommandResult
		},
	},
	builder::{
		CreateEmbed
	},
	model::{
		channel::{
			Message,
		},
	},
	utils::{
		Colour
	},
	prelude::*
};

use crate::{
	commands::get_client,
	player::{
		get_player
	}
};


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Deck {
	#[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
	id: Option<ObjectId>,
	pub discord_id: i64,
	pub name: String,
	pub cards: Vec<String>
}

impl Deck {
	pub fn empty(discord_id: i64, name: String) -> Self {
		Self {
			id: None,
			discord_id,
			name,
			cards: vec![]
		}
	}

	pub fn is_valid(&self) -> bool {
		self.cards.len() == 60 as usize
	}
}

async fn get_deck_collection() -> Collection<Deck> {
	let client = get_client().await.unwrap();
	let collection = client.database("poketcg").collection::<Deck>("decks");

	collection
}

pub async fn add_deck(deck: &Deck) {
	let deck_collection = get_deck_collection().await;
	deck_collection
		.insert_one(deck, None)
		.await
		.unwrap();
}

pub async fn get_decks_by_player(discord_id: i64) -> Vec<Deck> {
	let deck_collection = get_deck_collection().await;
	let decks = deck_collection
		.find(doc! { "discord_id": discord_id }, None)
		.await
		.unwrap()
		.try_collect::<Vec<Deck>>()
		.await
		.unwrap();

	decks
}

pub async fn get_deck(discord_id: i64, name: String) -> Option<Deck> {
	let deck_collection = get_deck_collection().await;
	let deck = deck_collection
		.find_one(doc! { "discord_id": discord_id, "name": name }, None)
		.await
		.unwrap();

	deck
}

pub async fn update_deck(deck: &Deck, update: Document) {
	let deck_collection = get_deck_collection().await;
	deck_collection
		.update_one(
			doc! { "_id": &deck.id.unwrap() },
			update,
			None
		)
		.await
		.unwrap();
}

#[command("decks")]
#[aliases("dks")]
async fn decks_command(ctx: &Context, msg: &Message) -> CommandResult {
	let player = get_player(msg.author.id.0).await;
	let decks = get_decks_by_player(player.discord_id).await;
	match decks.len() {
		0 => {
			msg.reply(&ctx.http, "You don't have any decks! Use **.deck create <name>** to create one!").await?;
		},
		_ => () // Need to revamp set_paginated_embed to take Trait PaginatedEmbed + HasCards
	}

	Ok(())
}

#[command("view")]
#[aliases("v")]
async fn deck_view(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
	

	Ok(())
}

#[command("create")]
#[aliases("c")]
async fn deck_create(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
	let deck_name = match args.find::<String>() {
		Ok(x) => x.to_lowercase(),
		Err(_) => String::from("")
	};
	if deck_name == String::from("") {
		msg.reply(&ctx.http, "You didn't provide a deck name.").await?;
		return Ok(());
	}
	let player = get_player(msg.author.id.0).await;
	match get_deck(player.discord_id, deck_name.clone()).await {
		Some(_) => {
			msg.reply(&ctx.http, "You already have a deck with that name!").await?;
			return Ok(());
		},
		None => ()
	}
	let deck = Deck::empty(player.discord_id, deck_name.clone());
	add_deck(&deck).await;
	msg.reply(&ctx.http, format!("You created the deck **{}**", deck_name)).await?;

	Ok(())
}

#[command("delete")]
#[aliases("d")]
async fn deck_delete(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
	

	Ok(())
}

#[command("add")]
#[aliases("a")]
async fn deck_add(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
	

	Ok(())
}

#[command("remove")]
#[aliases("r")]
async fn deck_remove(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
	

	Ok(())
}