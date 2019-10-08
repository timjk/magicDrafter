extern crate reqwest;
extern crate regex;
use std::error::Error;
use std::{io, thread, time};
use std::collections::{HashSet, HashMap};
use std::cmp::max;
use serde::{Deserialize};
use rusqlite::types::ToSql;
use rusqlite::{Connection, NO_PARAMS};
use regex::Regex;
use fuzzy_matcher::skim::{fuzzy_match};

pub fn run() -> Result<(), Box<dyn Error>> {
    println!("Running magic_drafter, start a draft run in arena.");
    let conn = Connection::open("test.db")?;
    let mut stmt = conn
        .prepare("SELECT id, name, scryfallId, cardSet, cardRank FROM card")?;
    let card_iter = stmt
        .query_map(NO_PARAMS, |row| Ok(Card {
            id: row.get(2)?,
            name: row.get(1)?,
            arena_id: row.get(0)?,
            set_name: row.get(3)?,
            card_rank: row.get(4)?
        }))?;
    for card in card_iter {
        println!("{:?}", card?);
    }
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(())
}

#[derive(Debug)]
#[derive(Deserialize)]
struct Card {
    id: String,
    name: String,
    arena_id: u32,
    set_name: String,
    card_rank: String
}

#[derive(Debug)]
#[derive(Deserialize)]
struct ScryfallCard {
    id: String,
    name: String,
    arena_id: u32,
    set_name: String,
}

fn fetch_card_details(id: u32) -> Result<ScryfallCard, reqwest::Error> {
    reqwest::get(&format!("https://api.scryfall.com/cards/arena/{}", id))?.json()
}

pub fn init_db() -> Result<(), Box<dyn Error>> {
    println!("Initialising collection...");

    let conn = Connection::open("test.db")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS card (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            scryfallId TEXT NOT NULL,
            cardSet TEXT NOT NULL,
            cardRank TEXT
        )",
        NO_PARAMS)?;

    let mut stmt = conn.prepare("SELECT id FROM card")?;
    let existing_cards: HashSet<_> = stmt
        .query_map::<u32, _, _>(NO_PARAMS, |row| row.get(0))?
        .map(|x| x.unwrap()) // This doesn't feel idiomatic
        .collect();

    let ranks = pull_latest_card_definitions()?;
    let cards = fetch_arena_cards()?;
    insert_card_defs(&conn, cards.difference(&existing_cards).cloned().collect(), &ranks)?;
    println!("done.");
    Ok(())
}

fn fetch_arena_cards() -> Result<HashSet<u32>, Box<dyn Error>> {
    let res = reqwest::get("https://raw.githubusercontent.com/mtgatracker/node-mtga/master/mtga/m20.js")?.text()?;

    let re = Regex::new(r"mtgaID: (\d+), ")?;
    let records: HashSet<u32> = re.captures_iter(&res).map(|x| x[1].parse::<u32>().unwrap()).collect();
    
    Ok(records)
}

pub fn pull_latest_card_definitions() -> Result<HashMap<String, String>, Box<dyn Error>> {
    let res = reqwest::get("https://docs.google.com/spreadsheets/d/1BAPtQv4U9KUAtVzkccJlPS8cb0s_uOcGEDORip5uaQg/gviz/tq?headers=0&sheet=Staging%20Sheet&tq=select+A,D")?.text()?;

    let re = Regex::new(r#"c[^v]+v.{3}([^"]+)".{8}([^"]+)"#)?;
    let card_ranks = re.captures_iter(&res)
        .map(|x| (str::replace(&x[1].to_owned(), "\\u0027", "'"), x[2].to_owned()))
        .collect::<HashMap<String, String>>(); // turbo-fish syntax

    Ok(card_ranks) // Is there a way to automatically return Result without needing Ok(item)?
}

// This should return a vector of cards but haven't figured a way to do multi-insert with rusqlite
fn insert_card_defs(conn: &Connection, card_ids: HashSet<u32>, card_ranks: &HashMap<String, String>) -> Result<(), Box<dyn Error>> {
    for id in card_ids {
        let card = match fetch_card_details(id) {
            Ok(x) => x,
            _ => {
                println!("Unable to retrieve card details with id {}, skipping...", id);
                continue;
            }
        };
        let card_rank = get_closest_match(&card.name, card_ranks);
        conn.execute(
            "INSERT INTO card (id, name, scryfallId, cardSet, cardRank)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            &[&card.arena_id as &dyn ToSql, &card.name, &card.id, &card.set_name, &card_rank],
        ).unwrap();
        println!("{:?}{:?}", card, card_rank);
        thread::sleep(time::Duration::from_secs(1)); // be a good citizen
    }

    Ok(())
}

fn get_closest_match<'a>(card_name: &str, card_ranks: &'a HashMap<String, String>) -> &'a str {
    let result = card_ranks.into_iter().fold((0, ""), |acc, x| {
        let max = max(acc.0, match fuzzy_match(&card_name, x.0) { Some(y) => y, None => 0 } );
        if max > acc.0 { (max, x.1) } else { acc }
    });
    result.1
}