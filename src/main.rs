#[macro_use]
extern crate num_derive;
#[macro_use]
extern crate error_chain;

use clap::clap_app;
mod common;
mod item;
mod skybox;
mod scanner;
mod lister;

use common::errors::Result;
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

    match matches.subcommand() {
        Some(("scan",_)) => scanner.scan().await?,

        Some((subcommand, matches)) => {
            if let Some(skybox) = scanner.get_selected() {
                match subcommand {
                    "ls" => skybox.list_items(matches).await?,
                    "rm" => skybox.remove_items(matches).await?,
                    "play" => skybox.play(matches).await?,
                    _ => config.print_help()?
                }
            } else {
                println!("Use subcommand `scan` to find a skybox");
            }
        }

        _ => config.print_help()?
    }

    Ok(())
}
