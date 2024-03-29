extern crate clap;
use clap::{App, SubCommand};
use magic_drafter;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let matches = App::new("magic_drafter")
        .version("v0.1")
        .author("timjk <timfjk@gmail.com>")
        .about("Helps when drafting in magicArena")
        .subcommand(
            SubCommand::with_name("pull")
                .about("Pulls down the latest card definitions for the user"),
        )
        .subcommand(
            SubCommand::with_name("update").about("Updates the card definitions for magic_drafter"),
        )
        .get_matches();

    if let Some(_matches) = matches.subcommand_matches("pull") {
        // Pull the latest definitions from arena drafter google doc & insert them into their
        // local sqlite DB
        magic_drafter::pull_latest_card_definitions()?;
        return Ok(());
    }

    if let Some(_matches) = matches.subcommand_matches("update") {
        // Pull the latest definitions from the various sites and update the arena drafter google doc
        // for now it updates a local sqlite DB
        magic_drafter::init_db()?;
        return Ok(());
    }

    magic_drafter::run()
}
