extern crate regex;
extern crate reqwest;
use fuzzy_matcher::skim::fuzzy_match;
use regex::Regex;
use rusqlite::types::ToSql;
use rusqlite::{Connection, NO_PARAMS};
use serde::{Deserialize};
use std::cmp::max;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::{io, thread, time};

pub fn run() -> Result<(), Box<dyn Error>> {
    println!("Running magic_drafter, start a draft run in arena.");
    let f = File::open("output_log.txt")?;
    let mut reader = BufReader::new(f);
    let re = Regex::new(r"<== Draft\.MakePick(?s).*?(\{.*?\})")?;

    loop {
        let mut line = String::new();
        if reader.read_to_string(&mut line)? > 0 {
           if let Some(ref m) = re.captures_iter(&line).last() {
                let pick: DraftPick = serde_json::from_str(&m[1]).unwrap();
                println!("{:?}", pick.draftPack)
           }
        } else {
            println!("waiting...");
            thread::sleep(time::Duration::from_millis(5000));
        }
    }

    Ok(())
}

#[derive(Deserialize, Debug)]
struct Card {
    id: String,
    name: String,
    arena_id: u32,
    set_name: String,
    card_rank: String,
}

#[derive(Deserialize, Debug)]
struct DraftPick {
    // playerId: String,
    // eventName: String,
    // draftId: String,
    // draftStatus: String,
    // packNumber: u32,
    // pickNumber: u32,
    draftPack: Vec<String>,
    pickedCards: Vec<String>,
    // requestUnits: f32
}

#[derive(Deserialize, Debug)]
struct ScryfallCard {
    id: String,
    name: String,
    arena_id: u32,
    set_name: String,
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
        NO_PARAMS,
    )?;

    let mut stmt = conn.prepare("SELECT id FROM card")?;
    let existing_cards: HashSet<_> = stmt
        .query_map::<u32, _, _>(NO_PARAMS, |row| row.get(0))?
        .map(|x| x.unwrap()) // This doesn't feel idiomatic
        .collect();

    let ranks = pull_latest_card_definitions()?;
    let cards = fetch_arena_cards()?;
    insert_card_defs(
        &conn,
        cards.difference(&existing_cards).cloned().collect(),
        &ranks,
    )?;
    println!("done.");
    Ok(())
}

pub fn pull_latest_card_definitions() -> Result<HashMap<String, String>, Box<dyn Error>> {
    let res = reqwest::get("https://docs.google.com/spreadsheets/d/1BAPtQv4U9KUAtVzkccJlPS8cb0s_uOcGEDORip5uaQg/gviz/tq?headers=0&sheet=Staging%20Sheet&tq=select+A,D")?.text()?;

    let re = Regex::new(r#"c[^v]+v.{3}([^"]+)".{8}([^"]+)"#)?;
    let card_ranks = re
        .captures_iter(&res)
        .map(|x| {
            (
                str::replace(&x[1].to_owned(), "\\u0027", "'"),
                x[2].to_owned(),
            )
        })
        .collect::<HashMap<String, String>>(); // turbo-fish syntax >::() - very fishy

    Ok(card_ranks) // Is there a way to automatically return Result without needing Ok(item)?
}

fn fetch_arena_cards() -> Result<HashSet<u32>, Box<dyn Error>> {
    let res =
        reqwest::get("https://raw.githubusercontent.com/mtgatracker/node-mtga/master/mtga/m20.js")?
            .text()?;

    let re = Regex::new(r"mtgaID: (\d+), ")?;
    let records: HashSet<u32> = re
        .captures_iter(&res)
        .map(|x| x[1].parse::<u32>().unwrap())
        .collect();
    Ok(records)
}

fn fetch_card_details(id: u32) -> Result<ScryfallCard, reqwest::Error> {
    reqwest::get(&format!("https://api.scryfall.com/cards/arena/{}", id))?.json()
}

// This should return a vector of cards but haven't figured a way to do multi-insert with rusqlite
fn insert_card_defs(
    conn: &Connection,
    card_ids: HashSet<u32>,
    card_ranks: &HashMap<String, String>,
) -> Result<(), Box<dyn Error>> {
    for id in card_ids {
        let card = match fetch_card_details(id) {
            Ok(x) => x,
            _ => {
                println!(
                    "Unable to retrieve card details with id {}, skipping...",
                    id
                );
                continue;
            }
        };
        let card_rank = get_closest_match(&card.name, &card_ranks);
        conn.execute(
            "INSERT INTO card (id, name, scryfallId, cardSet, cardRank)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            &[
                &card.arena_id as &dyn ToSql,
                &card.name,
                &card.id,
                &card.set_name,
                &card_rank,
            ],
        )
        .unwrap();
        println!("{:?}{:?}", card, card_rank);
        thread::sleep(time::Duration::from_secs(1)); // be a good citizen
    }

    Ok(())
}

fn get_closest_match<'a>(card_name: &str, card_ranks: &'a HashMap<String, String>) -> &'a str {
    let result = card_ranks.into_iter().fold((0, ""), |acc, x| {
        let max = max(
            acc.0,
            match fuzzy_match(&card_name, x.0) {
                Some(y) => y,
                None => 0,
            },
        );
        if max > acc.0 {
            (max, x.1)
        } else {
            acc
        }
    });
    result.1
}
