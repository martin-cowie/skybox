use super::item::Item;
use super::common::{envelope, as_elements, Result};
use super::lister;
use super::lister::Lister;

use std::fmt;
use maplit::hashmap;
use preferences::{AppInfo, PreferencesMap, Preferences};
use reqwest::Url;

const USER_AGENT: &str = "SKY_skyplus";
const CONTENT_TYPE: &str = r#"text/xml; charset="utf-8""#;

const APP_INFO: AppInfo = AppInfo{name: "skybox", author: "Martin Cowie"};
const PREFS_KEY: &str = "skybox/location";

#[derive(Debug)]
pub struct SkyBox {
    pub play_url: Url,
    pub browse_url: Url,

    client: reqwest::Client
}

impl SkyBox {

    pub fn new(play_url: Url, browse_url: Url) -> SkyBox {
        SkyBox{play_url, browse_url, client: reqwest::Client::new()}
    }

    pub fn save_box(&self) -> Result<()> {
        let mut map: PreferencesMap<String> = PreferencesMap::new();
        map.insert("play".into(), self.play_url.to_string());
        map.insert("browse".into(), self.browse_url.to_string());

        map.save(&APP_INFO, PREFS_KEY)
            .map_err(|error|format!("Cannot save skybox: {}", error).into())
    }

    pub fn load_box() -> Result<SkyBox> {
        let map = PreferencesMap::<String>::load(&APP_INFO, PREFS_KEY)?;
        let play_url = Url::parse(map.get("play").ok_or("Attribute `play` absent")?)?;
        let browse_url = Url::parse(map.get("browse").ok_or("Attribute `play` absent")?)?;

        Ok(SkyBox::new(play_url, browse_url))
    }

    pub async fn list_items(&self, matches: &clap::ArgMatches) -> Result<()> {
        let requested_count: usize = 25;
        let mut starting_index: usize = 0;

        let (_, total_items) = self.fetch_items(0, 0).await?;
        let mut lister = lister::build_lister(total_items, matches);

        loop {
            let (items, _) = self.fetch_items(starting_index, requested_count).await?;

            lister.list(&items);

            if items.len() < requested_count {
                break;
            }
            starting_index += items.len();
        }
        lister.close();

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

        let resp = self.client.post(self.browse_url.clone())
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
        ).ok_or("Cannot find `Result` element")?;
        let inner_xml = result_elem.text()
            .ok_or("`Result` element is empty")?;

        // Get the element "/s:Envelope/s:Body/u:BrowseResponse/TotalMatches/text()"
        let total_matches = doc.descendants().find(|n|
            n.tag_name().name() == "TotalMatches"
        ).ok_or("Cannot find `TotalMatches` element")?
        .text()
        .ok_or("`TotalMatches` element is empty")?;

        let total_matches: usize = total_matches.parse()?;


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
        let files: Vec<_> = matches.values_of("filenames")
            .ok_or("Require at least one item to remove")?
            .collect();

        for item in files.iter() {
            self.remove_item(item).await?;
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

        let resp = self.client.post(self.browse_url.clone())
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

        let resp = self.client.post(self.play_url.clone())
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

impl fmt::Display for SkyBox {

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {

        let host = self.browse_url.host();
        //FIXME:
        write!(f, "{:?}", &host)
    }

}
