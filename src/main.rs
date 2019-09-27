extern crate clap;
extern crate reqwest;
extern crate regex;
use std::io::Error;
use std::io;
use std::{thread, time};
use std::collections::HashSet;
use serde::{Deserialize};
use clap::{App, SubCommand};
use rusqlite::types::ToSql;
use rusqlite::{Connection, NO_PARAMS};
use regex::Regex;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = 
    App::new("magic_drafter")
        .version("v0.1")
        .author("timjk <timfjk@gmail.com>")
        .about("Helps when drafting in magicArena")
        .subcommand(SubCommand::with_name("pull")
            .about("Pulls down the latest card details"))
        .subcommand(SubCommand::with_name("init")
            .about("Initializes the card db"))
        .get_matches();

    if let Some(_matches) = matches.subcommand_matches("pull") {
        pull(67330)?;
        return Ok(())
    }

    if let Some(_matches) = matches.subcommand_matches("init") {
        init_db()?;
        return Ok(())
    }

    println!("Running magic_drafter, start a draft run in arena.");
    let conn = Connection::open("test.db")?;
    let mut stmt = conn
        .prepare("SELECT id, name, scryfallId, cardSet FROM card")?;
    let card_iter = stmt
        .query_map(NO_PARAMS, |row| Ok(ScryfallCard {
            id: row.get(2)?,
            name: row.get(1)?,
            arena_id: row.get(0)?,
            set_name: row.get(3)?,
        }))?;
    for card in card_iter {
        println!("{:?}", card.unwrap());
    }
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(())
}

#[derive(Debug)]
#[derive(Deserialize)]
struct ScryfallCard {
    id: String,
    name: String,
    arena_id: u32,
    set_name: String,
}

fn pull(id: u32) -> Result<ScryfallCard, Error> {
    let res: ScryfallCard = reqwest::get(&format!("https://api.scryfall.com/cards/arena/{}", id)).unwrap().json().unwrap();
    Ok(res)
}

fn init_db() -> rusqlite::Result<()> {
    println!("Initialising collection...");

    let conn = Connection::open("test.db")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS card (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            scryfallId TEXT NOT NULL,
            cardSet TEXT NOT NULL
        )",
        NO_PARAMS)?;

    let cards = get_cards().unwrap();
    let mut stmt = conn.prepare("SELECT id FROM card")?;
    let existing_cards: HashSet<_> = stmt
        .query_map::<u32, _, _>(NO_PARAMS, |row| row.get(0))?
        .map(|x| x.unwrap())
        .collect();

    insert_card_defs(&conn, cards.difference(&existing_cards).cloned().collect()).unwrap();
    println!("done.");
    Ok(())
}

fn get_cards() -> Result<HashSet<u32>, Error> {
    let res = reqwest::get("https://raw.githubusercontent.com/mtgatracker/node-mtga/master/mtga/m20.js").unwrap().text().unwrap();

    let re = Regex::new(r"mtgaID: (\d+), ").unwrap();
    let records: HashSet<u32> = re.captures_iter(&res).map(|x| x[1].parse::<u32>().unwrap()).collect();
    
    Ok(records)
}

// This should return a vector of cards but am sneaking in a db update while we wait to make next request
fn insert_card_defs(conn: &Connection, card_ids: HashSet<u32>) -> Result<(), Error> {
    for id in card_ids {
        let card = pull(id).unwrap();
        conn.execute(
            "INSERT INTO card (id, name, scryfallId, cardSet)
             VALUES (?1, ?2, ?3, ?4)",
            &[&card.arena_id as &ToSql, &card.name, &card.id, &card.set_name],
        ).unwrap();
        println!("{:?}", card);
        thread::sleep(time::Duration::from_secs(1)); // be a good citizen
    }

    Ok(())
}