extern crate clap;
extern crate reqwest;
use std::io::Error;
use std::io;
use std::io::Read;
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
        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                println!("{}", input);
            }
            Err(error) => println!("error: {}", error)
        }
    }

    Ok(())
}

fn pull() -> Result<(), Error> {
    println!("Pulling latest card details...");
    let mut res = reqwest::get("https://api.scryfall.com/cards/arena/67330").unwrap();
    let mut body = String::new();
    res.read_to_string(&mut body)?;

    println!("{}", body);
    println!("done.");
    Ok(())
}