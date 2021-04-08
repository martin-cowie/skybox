#[macro_use]
extern crate num_derive;

use clap::clap_app;
mod common;
mod item;
mod skybox;
mod scanner;
mod lister;

use common::Result;
use scanner::Scanner;

#[tokio::main]
async fn main() -> Result<()> {

    let mut config = clap_app!(skybox =>
        (version: "0.1")
        (about: "Interacts with SkyPlus PVRs")
        (@subcommand scan =>
            (about: "Scan for SkyPlus machines")
        )
        (@subcommand ls =>
            (about: "list recordings")
            (@arg UNWATCHED: -u "Exclude viewed recordings")
            (@arg TIME_ORDER: -t "list in time order")
            (@arg REVERSE_TIME: -r "reverse time order")
            (@arg FORMAT: -o --output +takes_value "Output: JSON|CSV")
        )
        (@subcommand rm =>
            (about: "remove recordings")
            (@arg filenames: ... "recordings to remove, e.g. BOOK:688476834 BOOK:688555858")
        )
        (@subcommand play =>
            (about: "play a recording")
            (@arg filename: +required "recording to play back, e.g. file://pvr/290B3177")
        )
    );
    let matches = config.clone().get_matches();

    let scanner = Scanner::new();

    if let Some(_matches) = matches.subcommand_matches("scan") {
        return scanner.scan().await;
    }

    else if let Some(matches) = matches.subcommand_matches("ls") {
        match scanner.get_selected() {
            None => println!("Use subcommand `scan` to find a skybox"),
            Some(skybox) => skybox.list_items(matches).await?
        }
    }

    else if let Some(matches) = matches.subcommand_matches("rm") {
        match scanner.get_selected() {
            None => println!("Use subcommand `scan` to find a skybox"),
            Some(skybox) => skybox.remove_items(matches).await?
        }
    }

    else if let Some(matches) = matches.subcommand_matches("play") {
        match scanner.get_selected() {
            None => println!("Use subcommand `scan` to find a skybox"),
            Some(skybox) => skybox.play(matches).await?
        }
    }

    else {
        config.print_help()?;
    }

    Ok(())
}
