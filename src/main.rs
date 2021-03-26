use clap::clap_app;

use tokio;

mod common;
mod item;
mod skybox;
mod scanner;

use common::Result;
use scanner::Scanner;

#[tokio::main]
async fn main() -> Result<()> {

    let matches = clap_app!(skybox =>
        (version: "0.1")
        (about: "Interacts with SkyPlus PVRs")
        (@subcommand scan =>
            (about: "Scan for SkyPlus machines")
        )
        (@subcommand ls =>
            (about: "list recordings")
            (@arg long: -l "Long items listing")
        )
        (@subcommand rm =>
            (about: "remove recordings")
            (@arg filenames: #{1, 100} "recordings to remove") //TODO: 100 max is constraining
        )
    ).get_matches();

    let scanner = Scanner::new();

    if let Some(matches) = matches.subcommand_matches("ls") {
        match scanner.get_selected() {
            None => println!("Use subcommand `scan` to find a skybox"),
            Some(skybox) => skybox.list_items(matches).await?
        }

    } else
    if let Some(_matches) = matches.subcommand_matches("scan") {
        return scanner.scan().await;
    } else
    if let Some(matches) = matches.subcommand_matches("rm") {
        match scanner.get_selected() {
            None => println!("Use subcommand `scan` to find a skybox"),
            Some(skybox) => skybox.remove_items(matches).await?
        }
    }

    Ok(())
}
