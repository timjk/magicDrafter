extern crate clap;
extern crate reqwest;
use std::io::Error;
use std::io;
use serde::{Deserialize};
use clap::{App, SubCommand};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = 
    App::new("magic_drafter")
        .version("v0.1")
        .author("timjk <timfjk@gmail.com>")
        .about("Helps when drafting in magicArena")
        .subcommand(SubCommand::with_name("pull")
            .about("Pulls down the latest card details"))
        .get_matches();

    if let Some(_matches) = matches.subcommand_matches("pull") {
        pull()?;
    } else {
        println!("Running magic_drafter, start a draft run in arena.");
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
    }

    Ok(())
}

#[derive(Debug)]
#[derive(Deserialize)]
struct Card {
    id: String,
    name: String,
    arena_id: u32,
    set_name: String,
}

fn pull() -> Result<(), Error> {
    println!("Pulling latest card details...");
    let res: Card = reqwest::get("https://api.scryfall.com/cards/arena/67330").unwrap().json().unwrap();

    println!("{:?}", res);
    println!("done.");
    Ok(())
}