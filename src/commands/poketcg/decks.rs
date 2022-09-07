use std::collections::HashMap;
use std::time::Duration;

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
	// builder::{
	// 	CreateEmbed
	// },
	model::{
		channel::{
			Message,
		},
	},
	// utils::{
	// 	Colour
	// },
	prelude::*
};

use crate::{
	commands::get_client,
	player::{
		get_player,
		update_player,
		Player
	},
	card::{
		get_multiple_cards_by_id,
		Card
	},
	commands::poketcg::Scrollable,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Deck {
	#[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
	id: Option<ObjectId>,
	pub discord_id: i64,
	pub name: String,
	pub cards: HashMap<String, i64>,
	pub display_card: String
}

impl Deck {
	pub fn empty(discord_id: i64, name: String) -> Self {
		Self {
			id: None,
			discord_id,
			name,
			cards: HashMap::new(),
			display_card: "".into()
		}
	}

	pub fn is_valid(&self) -> bool {
		self.cards.values().sum::<i64>() == 60
	}

	pub async fn get_cards(&self) -> Vec<super::card::Card> {
		let card_ids = self.cards.keys().into_iter().map(|k| k.into()).collect::<Vec<String>>();
		let cards = get_multiple_cards_by_id(card_ids).await;

		cards
	}
}

// TODO: Implement Scrollable for deck

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

pub async fn delete_deck(deck: &Deck) {
	let deck_collection = get_deck_collection().await;
	deck_collection
		.delete_one(
			doc! { "_id": &deck.id.unwrap() },
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
		_ => {
			let content = decks.iter().map(|d| d.name.clone()).collect::<Vec<String>>().join("\n");
			msg.reply(&ctx.http, content).await?;
		} // Need to revamp set_paginated_embed to take Trait PaginatedEmbed + HasCards
	}

	Ok(())
}

#[command("deck")]
#[aliases("dk")]
#[sub_commands(deck_view, deck_create, deck_delete, deck_add, deck_remove)]
async fn deck_main(ctx: &Context, msg: &Message) -> CommandResult {
	let content = "Here are the available deck commands:
	**.decks** to see all your current decks.
	**.deck view <name>** to view a specific deck
	**.deck create <name>** to create a new deck.
	**.deck delete <name>** to delete a deck that you've created.
	**.deck add <name> [<cardID:amount>/...]** to add cards to a deck.
	**.deck remove <name> [<cardID:amount>/...]** to remove cards from a deck.
	**.deck energy add <name> <type> [amount - Default: 1]** to add a basic energy to a deck.
	**.deck energy remove <name> <type> [amount - Default: 1]** to remove a basic energy from a deck.
	**.deck display <name> <cardID>** to set the display card of the deck";
	msg
		.channel_id
		.send_message(&ctx.http, |m| m.content(content))
		.await?;

	Ok(())
}

#[command("view")]
#[aliases("v")]
async fn deck_view(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
	let deck_name = args.rest().to_lowercase();
	let player = get_player(msg.author.id.0).await;
	if deck_name == String::from("") {
		return decks_command(ctx, msg, args).await;
	}
	let deck = get_deck(player.discord_id, deck_name.clone()).await;
	match deck {
		Some(_) => (),
		None => {
			msg.reply(&ctx.http, "You don't have a deck with that name.").await?;
			return Ok(());
		}
	}
	let deck = deck.unwrap();
	let card_ids = deck.cards
		.keys()
		.into_iter()
		.map(|c| c.into())
		.collect::<Vec<String>>();
	let cards = get_multiple_cards_by_id(card_ids).await;
	// let cards: Vec<super::card::Card> = vec![];
	cards.scroll_through(ctx, msg).await?;
	
	Ok(())
}

#[command("create")]
#[aliases("c")]
async fn deck_create(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
	let deck_name = args.rest().to_lowercase();
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
async fn deck_delete(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
	let deck_name = args.rest().to_lowercase();
	if deck_name == String::from("") {
		msg.reply(&ctx.http, "You didn't provide a deck name.").await?;
		return Ok(());
	}
	let mut player = get_player(msg.author.id.0).await;
	let deck = get_deck(player.discord_id, deck_name.clone()).await;
	match deck {
		Some(_) => (),
		None => {
			msg.reply(&ctx.http, "You don't have a deck with that name.").await?;
			return Ok(());
		}
	}
	let deck = deck.unwrap();
	let _ = msg.reply(&ctx.http, format!("Are you sure you want to delete this deck?\nOnce you delete **{}** it's gone forever. (y/n)", deck.name)).await?;
	if let Some(confirmation_reply) = &msg.author.await_reply(&ctx).timeout(Duration::from_secs(30)).await {
		if confirmation_reply.content.to_lowercase() != "y" {
			msg.reply(&ctx.http, format!("You did not delete **{}**.", deck.name)).await?;
			return Ok(());
		}
	} else {
		msg.reply(&ctx.http, format!("You did not delete **{}**.", deck.name)).await?;
		return Ok(());
	}
	// Player said "y" to get here
	for (crd, amt) in deck.cards.iter() {
		*player.cards.entry(crd.clone()).or_insert(0) += amt;
	}
	// Update the player
	let mut player_update = Document::new();
	let mut player_cards_update = Document::new();
	for (crd, amt) in player.cards.iter() {
		player_cards_update.insert(crd, amt);
	}
	player_update.insert("cards", player_cards_update);
	update_player(&player, doc! { "$set": player_update }).await;
	delete_deck(&deck).await;
	msg.reply(&ctx.http, format!("You deleted **{}**", deck.name)).await?;

	Ok(())
}

#[command("add")]
#[aliases("a")]
async fn deck_add(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
	let deck_name = args.find::<String>().unwrap_or(String::from(""));
	if deck_name == String::from("") {
		msg.reply(&ctx.http, "You didn't provide a deck name.").await?;
		return Ok(());
	}
	let card_str = args.rest();
	if card_str == "" {
		msg.reply(&ctx.http, "You didn't provide cards to add.").await?;
		return Ok(());
	}
	let mut player = get_player(msg.author.id.0).await;
	let deck = get_deck(player.discord_id, deck_name.clone()).await;
	match deck {
		Some(_) => (),
		None => {
			msg.reply(&ctx.http, "You don't have a deck with that name.").await?;
			return Ok(());
		}
	}
	let mut deck = deck.unwrap();
	let deckcards = DeckCards::from_card_str(card_str);
	if !deckcards.player_has_all(&player) {
		msg.reply(&ctx.http, "You don't own all of what you're putting in the deck!").await?;
		return Ok(());
	}
	if !deckcards.is_valid_addition(&deck) {
		// Maybe update this to list what's not valid
		msg.reply(&ctx.http, "You have invalid additions to this deck!").await?;
		return Ok(());
	}
	for (card_id, amt) in deckcards.cards {
		*player.cards.entry(card_id.clone()).or_insert(0) -= amt;
		if *player.cards.entry(card_id.clone()).or_insert(0) == 0 {
			player.cards.remove(&card_id);
		}
		*deck.cards.entry(card_id.clone()).or_insert(0) += amt;
	}
	// Update the player
	let mut player_update = Document::new();
	let mut player_cards_update = Document::new();
	for (crd, amt) in player.cards.iter() {
		player_cards_update.insert(crd, amt);
	}
	player_update.insert("cards", player_cards_update);
	update_player(&player, doc! { "$set": player_update }).await;

	// Update the deck
	let mut deck_update = Document::new();
	let mut deck_card_update = Document::new();
	for (crd, amt) in deck.cards.iter() {
		deck_card_update.insert(crd, amt);
	}
	deck_update.insert("cards", deck_card_update);
	update_deck(&deck, doc! { "$set": deck_update }).await;

	msg.reply(&ctx.http, format!{"You added **{}** to **{}**", card_str, deck.name}).await?;

	Ok(())
}

#[command("remove")]
#[aliases("r")]
async fn deck_remove(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
	let deck_name = args.find::<String>().unwrap_or(String::from(""));
	if deck_name == String::from("") {
		msg.reply(&ctx.http, "You didn't provide a deck name.").await?;
		return Ok(());
	}
	let card_str = args.rest();
	if card_str == "" {
		msg.reply(&ctx.http, "You didn't provide cards to remove.").await?;
		return Ok(());
	}
	let mut player = get_player(msg.author.id.0).await;
	let deck = get_deck(player.discord_id, deck_name.clone()).await;
	match deck {
		Some(_) => (),
		None => {
			msg.reply(&ctx.http, "You don't have a deck with that name.").await?;
			return Ok(());
		}
	}
	let mut deck = deck.unwrap();
	let deckcards = DeckCards::from_card_str(card_str);
	if !deckcards.deck_has_all(&deck) {
		msg.reply(&ctx.http, "The deck doesn't have all of what you're removing!").await?;
		return Ok(());
	}
	for (card_id, amt) in deckcards.cards {
		*deck.cards.entry(card_id.clone()).or_insert(0) -= amt;
		if *deck.cards.entry(card_id.clone()).or_insert(0) == 0 {
			deck.cards.remove(&card_id);
		}
		*player.cards.entry(card_id.clone()).or_insert(0) += amt;
	}
	// Update the player
	let mut player_update = Document::new();
	let mut player_cards_update = Document::new();
	for (crd, amt) in player.cards.iter() {
		player_cards_update.insert(crd, amt);
	}
	player_update.insert("cards", player_cards_update);
	update_player(&player, doc! { "$set": player_update }).await;

	// Update the deck
	let mut deck_update = Document::new();
	let mut deck_card_update = Document::new();
	for (crd, amt) in deck.cards.iter() {
		deck_card_update.insert(crd, amt);
	}
	deck_update.insert("cards", deck_card_update);
	update_deck(&deck, doc! { "$set": deck_update }).await;

	msg.reply(&ctx.http, format!{"You removed **{}** from **{}**", card_str, deck.name}).await?;

	Ok(())
}

pub struct DeckCards {
	pub cards: Vec<(String, i64)>
}

impl DeckCards {
	pub fn from_card_str(card_str: &str) -> Self {
		let inputs = card_str.split("/").collect::<Vec<&str>>();
		let mut cards = vec![];
		for input in inputs {
			let card_amt = input
				.split(":")
				.collect::<Vec<&str>>();
			let card = String::from(card_amt[0]);
			if card_amt.len() == 1 {
				cards.push((card, 1));
			} else {
				let mut amt = card_amt[1].parse::<i64>().unwrap_or(1);
				if amt > 4 {
					amt = 4;
				}
				cards.push((card, amt));
			}
		}

		Self {
			cards
		}
	}

	pub fn player_has_all(&self, player: &Player) -> bool {
		for (card_id, amt) in &self.cards {
			if player.cards.get(card_id).unwrap_or(&0) < amt {
				return false;
			}
		}

		true
	}

	pub fn is_valid_addition(&self, deck: &Deck) -> bool {
		let deckcards_sum = self.cards
			.iter()
			.map(|ca| ca.1)
			.collect::<Vec<i64>>()
			.iter()
			.sum::<i64>();
		let deck_sum = deck.cards.values().sum::<i64>();
		if deckcards_sum + deck_sum > 60 {
			return false;
		}
		for (card_id, amt) in &self.cards {
			if deck.cards.get(card_id).unwrap_or(&0) + amt > 4 {
				return false;
			}
		}

		true
	}

	pub fn deck_has_all(&self, deck: &Deck) -> bool {
		for (card_id, amt) in &self.cards {
			if deck.cards.get(card_id).unwrap_or(&0) < amt {
				return false;
			}
		}

		true
	}
}