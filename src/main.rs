extern crate clap;
extern crate reqwest;
extern crate regex;
use std::io::Error;
use std::io;
use std::str;
use std::{thread, time};
use std::collections::HashSet;
use std::collections::HashMap;
use std::cmp::max;
use serde::{Deserialize};
use clap::{App, SubCommand};
use rusqlite::types::ToSql;
use rusqlite::{Connection, NO_PARAMS};
use regex::Regex;
use fuzzy_matcher::skim::{fuzzy_match};

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
        get_ranks()?;
        return Ok(())
    }

    if let Some(_matches) = matches.subcommand_matches("init") {
        init_db()?;
        return Ok(())
    }

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
        println!("{:?}", card.unwrap());
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
            cardSet TEXT NOT NULL,
            cardRank TEXT NOT NULL
        )",
        NO_PARAMS)?;

    let cards = get_cards().unwrap();
    let mut stmt = conn.prepare("SELECT id FROM card")?;
    let existing_cards: HashSet<_> = stmt
        .query_map::<u32, _, _>(NO_PARAMS, |row| row.get(0))?
        .map(|x| x.unwrap())
        .collect();

    let ranks = get_ranks().unwrap();

    insert_card_defs(&conn, cards.difference(&existing_cards).cloned().collect(), &ranks).unwrap();
    println!("done.");
    Ok(())
}

fn get_cards() -> Result<HashSet<u32>, Error> {
    let res = reqwest::get("https://raw.githubusercontent.com/mtgatracker/node-mtga/master/mtga/m20.js").unwrap().text().unwrap();

    let re = Regex::new(r"mtgaID: (\d+), ").unwrap();
    let records: HashSet<u32> = re.captures_iter(&res).map(|x| x[1].parse::<u32>().unwrap()).collect();
    
    Ok(records)
}

fn get_ranks() -> Result<HashMap<String, String>, Error> {
    let res = reqwest::get("https://docs.google.com/spreadsheets/d/1BAPtQv4U9KUAtVzkccJlPS8cb0s_uOcGEDORip5uaQg/gviz/tq?headers=0&sheet=Staging%20Sheet&tq=select+A,D")
        .unwrap().text().unwrap();

    let re = Regex::new(r#"c[^v]+v.{3}([^"]+)".{8}([^"]+)"#).unwrap();

    let card_ranks: HashMap<String, String> = re.captures_iter(&res)
        .map(|x| (x[1].to_owned(), x[2].to_owned()))
        .collect();
    
    Ok(card_ranks)
}

// This should return a vector of cards but am sneaking in a db update while we wait to make next request
fn insert_card_defs(conn: &Connection, card_ids: HashSet<u32>, card_ranks: &HashMap<String, String>) -> Result<(), Error> {
    for id in card_ids {
        let card = pull(id).unwrap();
        let card_rank = get_closest_match(&card.name, card_ranks);
        conn.execute(
            "INSERT INTO card (id, name, scryfallId, cardSet, cardRank)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            &[&card.arena_id as &ToSql, &card.name, &card.id, &card.set_name, &card_rank],
        ).unwrap();
        println!("{:?}{:?}", card, card_rank);
        thread::sleep(time::Duration::from_secs(1)); // be a good citizen
    }

    Ok(())
}

fn get_closest_match<'a>(card_name: &str, card_ranks: &'a HashMap<String, String>) -> &'a str {
    let result = card_ranks.into_iter().fold((0, ""), |acc, x| score_card_name(acc, x, card_name));
    result.1
}

// TODO: how do we have anonymous methods with assignment
fn score_card_name<'a>(acc: (i64, &'a str), x: (&String, &'a String), card_name: &str) -> (i64, &'a str) {
    let max = max(acc.0, match fuzzy_match(&card_name, x.0) { Some(y) => y, None => 0 } );
    if max > acc.0 { (max, x.1) } else { acc }
}


// Community set reviews https://docs.google.com/spreadsheets/d/1BAPtQv4U9KUAtVzkccJlPS8cb0s_uOcGEDORip5uaQg/gviz/tq?headers=2&range=A1%3AZ&sheet=Table%20Source&tqx=reqId%3A1;responseHandler%3A__JsonpRequest_cb__.getResponse1_
// card-name & rating https://docs.google.com/spreadsheets/d/1BAPtQv4U9KUAtVzkccJlPS8cb0s_uOcGEDORip5uaQg/gviz/tq?headers=0&sheet=Staging%20Sheet&tq=select+A,D 
// https://www.mtgcommunityreview.com/core-set-2020