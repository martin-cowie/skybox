use super::common::{SKY_BROWSE, SKY_PLAY};
use super::common::errors::*;
use super::skybox::SkyBox;

use indicatif::ProgressBar;
use ssdp_client::{URN, SearchTarget};
use std::collections::HashMap;
use std::io::{self, BufRead};
use std::time::Duration;
use url::Url;

use futures::prelude::*;
use futures::join;

const TIMEOUT: Duration = Duration::from_secs(5);

/**
 * Scan for SkyBoxes
 */
pub struct Scanner {
    // ...
}

impl Scanner {
    pub fn new() -> Self {
        Scanner{}
    }

    pub fn get_selected(&self) -> Option<SkyBox> {
        SkyBox::load_box().ok()
    }

    pub async fn scan(&self) -> Result<()> {

        let spinner = ProgressBar::new_spinner();
        spinner.enable_steady_tick(120);
        spinner.set_message("Scanning...");

        let play: &SearchTarget = &SKY_PLAY.into(); //NB: shame this cannot be done in the `search` calls
        let browse: &SearchTarget = &SKY_BROWSE.into();

        let (sky_play, sky_browse) = (&SKY_PLAY, &SKY_BROWSE);
        let (play_urls, browse_urls) = join!(
            self.ssdp_search(play, sky_play),
            self.ssdp_search(browse, sky_browse)
        );

        let play_urls = play_urls?;
        let browse_urls = browse_urls?;

        // Merge/Zip two URL dicts together
        let mut boxes: Vec<SkyBox> = Vec::new();
        for (ip_addr, browse_url) in browse_urls {
            let play_url = match play_urls.get(&ip_addr) {
                Some(url) => url,
                None => {
                    eprintln!("No matching URL {}", ip_addr);
                    continue;
                }
            };

            println!("Found {} and {}", play_url, browse_url);
            let skybox = SkyBox::new(play_url.clone(), browse_url);
            boxes.push(skybox);
        }


        if boxes.len() < 1 {
            let message = "No sky box found";
            spinner.finish_with_message(message);
            return Err(message.into());
        }

        spinner.finish_with_message(format!("Found {} skybox", boxes.len()).as_str());
        for (i,skybox) in boxes.iter().enumerate() {
            println!("{}:\t{}", i, skybox);
        }
        eprint!("Choose a skybox: ");

        let line = match io::stdin().lock().lines().next() {
            None => panic!("Line cannot be empty"),
            Some(result) => {
                match result {
                    Err(err) => panic!("Cannot read input: {}", err),
                    Ok(line) => line
                }
            }
        };

        let line_number: usize = line.parse()?;
        let skybox = &boxes.get(line_number).ok_or("Not an existant skybox")?;
        println!("Using {}", skybox);

        // Store the user's preferences
        return skybox.save_box();
    }

    async fn get_service_url(&self, urn: &URN, location: &Url) -> Result<Url> {
        let client = reqwest::Client::new();
        let resp = client.get(location.clone())
            .header("user-agent", "SKY_skyplus")
            .send().await?
            .text().await?;

        let doc = roxmltree::Document::parse(&resp)?;

        return self.extract_service_url(&doc, urn, &location);
    }

    // Get XPath /root/device/serviceList/service[serviceType/text()='${serviceType}']/controlURL/text()
    fn extract_service_url(&self, doc: &roxmltree::Document, urn: &URN, root_url: &Url) -> Result<Url> {
        let service_type_elem = doc.descendants().find(|n|
            n.tag_name().name() == "serviceType" &&
            n.text() == Some(&urn.to_string())
        ).ok_or(format!("Cannot find service URL for URN {} at {}", urn, root_url))?;

        // Go up & down one
        let parent = service_type_elem.parent_element().expect("Cannot find element parent");
        let control_url_element = parent.descendants().find(|n|
            n.tag_name().name() == "controlURL"
        ).ok_or(format!("Cannot find service URL for URN {} at {}", urn, root_url))?;

        // Compose the request URL
        let mut result = root_url.clone();
        result.set_path(
            control_url_element
                .text()
                .ok_or(format!("Cannot find service URL for URN {} at {}", urn, root_url))?
        );

        Ok(result)
    }

    /**
     * SSDP scan.
     * Get the descriptor document for each response.
     * @return a map of <IP-Address, ServiceURL>
     */
    async fn ssdp_search(&self, st: &SearchTarget, urn: &URN) -> Result<HashMap<String, Url>> {
        let mut result: HashMap<String, Url> = HashMap::new();

        let mut responses = ssdp_client::search(st, TIMEOUT, 2).await?;
        while let Some(response) = responses.next().await {
            let location = Url::parse(response?.location())?;
            let browse_url = self.get_service_url(urn, &location).await?;

            result.insert(
                browse_url.host_str().ok_or("Absent host component from URL")?.into(),
                browse_url);
        }

        Ok(result)
    }

}