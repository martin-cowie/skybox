use super::item::Item;
use super::common::{envelope, as_elements, Result};

use serde::{Serialize, Deserialize};
use std::time::Instant;
use maplit::hashmap;
use preferences::{AppInfo, Preferences};

const USER_AGENT: &str = "SKY_skyplus";
const CONTENT_TYPE: &str = r#"text/xml; charset="utf-8""#;

const APP_INFO: AppInfo = AppInfo{name: "skybox", author: "Martin Cowie"};
const PREFS_KEY: &str = "skybox/location";

#[derive(Serialize, Deserialize, Debug)]
pub struct SkyBox {
    pub play_url: String,
    pub browse_url: String
}

impl SkyBox {

    pub fn save_box(&self) -> Result<()> {
        self.save(&APP_INFO, PREFS_KEY)
            .map_err(|error|format!("Cannot save skybox: {}", error).into())
    }

    pub fn load_box() -> Option<SkyBox> {
        SkyBox::load(&APP_INFO, PREFS_KEY).ok()
    }

    pub async fn list_items(&self, _matches: &clap::ArgMatches) -> Result<()> {
        let query_start = Instant::now();

        let mut starting_index: usize = 0;
        let requested_count: usize = 25;

        let mut wtr = csv::Writer::from_writer(std::io::stdout());

        loop {
            let (items, total_items) = self.fetch_items(starting_index, requested_count).await?;
            eprintln!("Fetched {}/{} items.", starting_index + items.len(), total_items);

            for item in items.iter() {
                wtr.serialize(item)?;
            }

            if items.len() < requested_count {
                break;
            }
            starting_index += items.len();
        }
        eprintln!("Fetched {} items from {} in {}s", starting_index, &self.browse_url, query_start.elapsed().as_secs());

        Ok(())
    }

    async fn fetch_items(&self, starting_index: usize, requested_count: usize) -> Result<(Vec<Item>, usize)> {

        let starting_index = starting_index.to_string();
        let requested_count = requested_count.to_string();
        let arguments = hashmap!{
            "ObjectID" => "3",
            "BrowseFlag" => "BrowseDirectChildren",
            "Filter" => "*",
            "StartingIndex" => starting_index.as_str(),
            "RequestedCount" => requested_count.as_str(),
            "SortCriteria" => ""
        };

        let arguments = as_elements(&arguments);

        let browse_elem = format!(r#"<u:Browse xmlns:u="urn:schemas-nds-com:service:SkyBrowse:2">{}</u:Browse>"#, arguments);
        let body = envelope(browse_elem.as_str());

        let client = reqwest::Client::new();
        let resp = client.post(&self.browse_url)
            .header("user-agent", USER_AGENT)
            .header("Content-Type", CONTENT_TYPE)
            .header("SOAPACTION", r#""urn:schemas-nds-com:service:SkyBrowse:2#Browse""#)
            .body(body)
            .send()
            .await?
            .text()
            .await?;

        // Parse the response and get element 'Result'
        let doc = roxmltree::Document::parse(&resp)?;
        let result_elem = doc.descendants().find(|n|
            n.tag_name().name() == "Result"
        ).unwrap();

        // Get the element "/s:Envelope/s:Body/u:BrowseResponse/TotalMatches/text()"
        let total_matches = doc.descendants().find(|n|
            n.tag_name().name() == "TotalMatches"
        ).unwrap().text().unwrap();
        let total_matches: usize = total_matches.parse().unwrap();

        let inner_xml = result_elem.text().unwrap();

        // parse inner XML
        let doc = roxmltree::Document::parse(inner_xml)?;
        let items: Vec<_> = doc.descendants()
            .filter(|n|n.tag_name().name() == "item")
            .map(Item::build)
            .filter_map(Result::ok)
            .collect();

        Ok((items, total_matches))
    }


    pub async fn remove_items(&self,  matches: &clap::ArgMatches) -> Result<()> {
        let files: Vec<_> = matches.values_of("filenames").unwrap().collect();

        for item in files.iter() {
            self.remove_item(item).await.unwrap();
        }

        Ok(())
    }

    async fn remove_item(&self, item_id: &str) -> Result<()> {
        eprintln!("Removing: {} using {}", item_id, self.browse_url);

        let destroy_elem = format!(
            r#"<u:DestroyObject xmlns:u="urn:schemas-nds-com:service:SkyBrowse:2">{}</u:DestroyObject>"#,
            as_elements(&hashmap!{
                "ObjectID" => item_id
            }));

        let body = envelope(destroy_elem.as_str());

        let client = reqwest::Client::new();
        let resp = client.post(&self.browse_url)
            .header("user-agent", USER_AGENT)
            .header("Content-Type", CONTENT_TYPE)
            .header("SOAPACTION", r#""urn:schemas-nds-com:service:SkyBrowse:2#DestroyObject""#)
            .body(body)
            .send()
            .await?;

        if resp.status() == 200 {
            println!("removed: {}", item_id);
            Ok(())
        } else {
            Err("Delete failed".into())
        }
    }

    pub async fn play(&self,  matches: &clap::ArgMatches) -> Result<()> {
        let item_res = matches.value_of("filename").expect("Expecting argument");

        let uri = format!("{}?position=0&amp;speed=1", item_res);

        let play_elem = format!(
            r#"<u:SetAVTransportURI xmlns:u="urn:schemas-nds-com:service:SkyPlay:2">{}</u:SetAVTransportURI>"#,
            as_elements(&hashmap!{
                "InstanceID" => "0",
                "CurrentURI" => &uri,
                "CurrentURIMetaData" => "NOT_IMPLEMENTED"
            }));
        let body = envelope(play_elem.as_str());

        let client = reqwest::Client::new();
        let resp = client.post(&self.play_url)
            .header("user-agent", USER_AGENT)
            .header("Content-Type", CONTENT_TYPE)
            .header("SOAPACTION", r#""urn:schemas-nds-com:service:SkyPlay:2#SetAVTransportURI""#)
            .body(body)
            .send()
            .await?;

        if resp.status() == 200 {
            println!("Playing: {}", item_res);
            Ok(())
        } else {
            Err("Play request failed".into())
        }

    }

}