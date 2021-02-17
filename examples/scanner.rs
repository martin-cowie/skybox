use ssdp_client::*;
use std::time::Duration;
use futures::prelude::*;
use futures::join;


/*
 * How to simultaneously scan for two URNs.
 */

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

const SKY_PLAY: URN = URN::service("schemas-nds-com", "SkyPlay", 2);
const SKY_BROWSE: URN = URN::service("schemas-nds-com", "SkyBrowse", 2);

const TIMEOUT: Duration = Duration::from_secs(5);

#[tokio::main]
async fn main() -> Result<()> {

    let play: &SearchTarget = &SKY_PLAY.into(); //NB: shame this cannot be done in the `search` calls
    let browse: &SearchTarget = &SKY_BROWSE.into();

    let (play_vec, browse_vec) = join!(
        search(play), 
        search(browse)
    );

    println!("{} => {:?}\n", SKY_PLAY, play_vec);
    println!("{} => {:?}", SKY_BROWSE, browse_vec);

    Ok(())
}

async fn search(st: &SearchTarget) -> Result<Vec<String>> {   

    let mut result: Vec<String> = Vec::new();
    let mut responses = ssdp_client::search(st.into(), TIMEOUT, 2).await?;
    eprintln!("Searching for {}", st);
    while let Some(response) = responses.next().await {
        let response = response?;

        let client = reqwest::Client::new();
        eprintln!("Found {:?}", response);
        let description = client.get(response.location())
            .header("user-agent", "SKY_skyplus")
            .send()
            .await?
            .text()
            .await?;

        // println!("Description: {:?}", description);
        result.push(description);
    }
    eprintln!("Searching for {} complete", st);

    Ok(result)
}