use super::common::{Result, SKY_BROWSE, SKY_PLAY};
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
        SkyBox::load_box()
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

        let play_urls = play_urls.unwrap();
        let browse_urls = browse_urls.unwrap();

        // Merge/Zip two URL dicts together
        let mut boxes: Vec<SkyBox> = Vec::new();
        for (ip_addr, browse_url) in browse_urls {
            let play_url = play_urls.get(&ip_addr).unwrap(); //TODO: handle missing k/v pair

            println!("Found {} and {}", play_url, browse_url);
            let skybox = SkyBox {
                play_url: play_url.to_string() ,
                browse_url: browse_url.to_string()
            };
            boxes.push(skybox);
        }

        // TODO: rethink this struct
        spinner.finish_with_message(format!("Found {} skybox", boxes.len()).as_str());

        for (i,skybox) in boxes.iter().enumerate() {
            println!("{}:\t{:?}", i, skybox);
        }
        eprint!("Choose a skybox: "); //TODO: rethink all uses of unwrap

        let line = io::stdin().lock().lines().next().unwrap()?;
        let line_number: usize = line.parse().unwrap();

        let skybox = &boxes[line_number];
        println!("Using {:?}", skybox);

        // Store the user's preferences
        skybox.save_box();

        Ok(())
    }

    async fn get_service_url(&self, urn: &URN, location: &Url) -> Result<Url> {
        let client = reqwest::Client::new();
        let resp = client.get(location.clone())
            .header("user-agent", "SKY_skyplus")
            .send().await?
            .text().await?;

        let doc = roxmltree::Document::parse(&resp).unwrap();
        let browse_url = self.extract_service_url(&doc, urn, &location);

        Ok(browse_url)
    }

    // Get XPath /root/device/serviceList/service[serviceType/text()='${serviceType}']/controlURL/text()
    fn extract_service_url(&self, doc: &roxmltree::Document, urn: &URN, root_url: &Url) -> Url {
        let service_type_elem = doc.descendants().find(|n|
            n.tag_name().name() == "serviceType" &&
            n.text() == Some(&urn.to_string())
        ).unwrap();

        // Go up & down one
        let parent = service_type_elem.parent_element().unwrap();
        let control_url_element =
            parent.descendants().find(|n| n.tag_name().name() == "controlURL").unwrap();

        // Compose the request URL
        let mut result = root_url.clone();
        result.set_path(control_url_element.text().unwrap());
        result
    }

    /**
     * SSDP scan.
     * Get the descriptor document for each response.
     * @return a map of <IP-Address, ServiceURL>
     */
    async fn ssdp_search(&self, st: &SearchTarget, urn: &URN) -> Result<HashMap<String, Url>> {
        let mut locations: Vec<Url> = Vec::new();
        let mut responses = ssdp_client::search(st.into(), TIMEOUT, 2).await?;
        while let Some(response) = responses.next().await {
            let response = response?;
            locations.push(Url::parse(response.location())?);
        }

        // Get service-url for each location
        let mut result: HashMap<String, Url> = HashMap::new();
        for location in locations {
            let browse_url = self.get_service_url(urn, &location).await?;
            result.insert(
                browse_url.host_str().unwrap().to_string(),
                browse_url);
        }

        Ok(result)
    }

}