use clap::clap_app;

use futures::prelude::*;
use maplit::hashmap;
use reqwest;
use roxmltree;
use serde::Serialize;
use ssdp_client::URN;
use std::time::{Duration, Instant};
use tokio;
use url::Url;

// A simple type alias so as to DRY.
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;


const TIMEOUT: Duration = Duration::from_secs(5);

#[tokio::main]
async fn main() -> Result<()> {
    let _matches = clap_app!(myapp =>
        (version: "0.1")
        (about: "Interacts with SkyPlus PVRs")
        (@subcommand ls =>
            (about: "list recordings")
            (@arg debug: -l ... "Long items listing")
        )
    ).get_matches();




    //TODO: refactor everything below
    let search_target = URN::service("schemas-nds-com", "SkyBrowse", 2).into();
    let mut responses = ssdp_client::search(&search_target, TIMEOUT, 2).await?;


    while let Some(response) = responses.next().await {
        let response = response?;
        eprintln!("- {}", response.search_target());
        eprintln!("  - location: {}", response.location());
        eprintln!("  - usn: {}", response.usn());
        eprintln!("  - server: {}", response.server());

        let query_start = Instant::now();

        // Get the description for this service
        let client = reqwest::Client::new();
        let resp = client.get(response.location())
            .header("user-agent", "SKY_skyplus")
            .send()
            .await?
            .text()
            .await?;

        // Get XPath /root/device/serviceList/service[serviceType/text()='${serviceType}']/controlURL/text()
        let doc = roxmltree::Document::parse(&resp).unwrap();

        let service_type_elem = doc.descendants().find(|n|
            n.tag_name().name() == "serviceType" &&
            n.text() == Some("urn:schemas-nds-com:service:SkyBrowse:2"
        )).unwrap();

        // Go up & down one
        let parent = service_type_elem.parent_element().unwrap();
        let control_url_element =
            parent.descendants().find(|n| n.tag_name().name() == "controlURL").unwrap();


        // Compose the request URL
        let mut service_url = Url::parse(response.location())?;
        service_url.set_path(control_url_element.text().unwrap());

        // println!("Control URL: {:?}", service_url.as_str());
        let mut starting_index: usize = 0;
        let requested_count: usize = 25;

        let mut wtr = csv::Writer::from_writer(std::io::stdout());
        loop {
            let (items, total_items) = fetch_items(&service_url, starting_index, requested_count).await?;
            eprintln!("Fetched {}/{} items.", starting_index + items.len(), total_items);

            for item in items.iter() {
                wtr.serialize(item)?;
            }

            if items.len() < requested_count {
                break;
            }
            starting_index += items.len();
        }
        eprintln!("Fetched {} items from {} in {}s", starting_index, response.location(), query_start.elapsed().as_secs());
    }

    Ok(())
}

async fn fetch_items(url: &reqwest::Url, starting_index: usize, requested_count: usize) -> Result<(Vec<Item>, usize)> {

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

    let arguments = arguments.iter()
        .map(|(key, value)| format!("<{}>{}</{}>", &key, &value, &key))
        .collect::<Vec<_>>()
        .join("");

    let browse_elem = format!("<u:Browse xmlns:u=\"urn:schemas-nds-com:service:SkyBrowse:2\">{}</u:Browse>", arguments);

    let body = format!("<?xml version=\"1.0\" encoding=\"utf-8\"?>
        <s:Envelope s:encodingStyle=\"http://schemas.xmlsoap.org/soap/encoding/\" xmlns:s=\"http://schemas.xmlsoap.org/soap/envelope/\">
            <s:Body>{}</s:Body>
        </s:Envelope>", browse_elem);

    // println!("body: {}", body);

    let client = reqwest::Client::new();
    let resp = client.post(url.clone())
        .header("user-agent", "SKY_skyplus")
        .header("Content-Type", "text/xml; charset=\"utf-8\"")
        .header("SOAPACTION", "\"urn:schemas-nds-com:service:SkyBrowse:2#Browse\"")
        .body(body)
        .send()
        .await?
        .text()
        .await?;

    // eprintln!("Response to service call: {}", resp);

    // Parse the response and get element 'Result'
    let doc = roxmltree::Document::parse(&resp).unwrap();
    let result_elem = doc.descendants().find(|n|
        n.tag_name().name() == "Result"
    ).unwrap();

    // Get the element "/s:Envelope/s:Body/u:BrowseResponse/TotalMatches/text()"
    let total_matches = doc.descendants().find(|n| 
        n.tag_name().name() == "TotalMatches"
    ).unwrap().text().unwrap();
    let total_matches: usize = total_matches.parse().unwrap();

    let inner_xml = result_elem.text().unwrap();
    // println!("Result: {}", &inner_xml);

    // parse inner XML
    let doc = roxmltree::Document::parse(inner_xml).unwrap();
    let items: Vec<_> = doc.descendants()
        .filter(|n|n.tag_name().name() == "item")
        .map(|item_elem|Item::build(item_elem))
        .filter_map(Result::ok)
        .collect();

    Ok((items, total_matches))
}


#[derive(Debug, Serialize)]
struct Item {
    id: String,
    title: String,
    description: String,
    viewed: bool,

    recorded_starttime: String,
    recorded_duration: String,

    channel_name: String,
    series_id: Option<String>,
    service_type: i64
}

impl Item {

    fn string_of_element(elem: roxmltree::Node, name: &str) -> Option<String> { //TODO: return a simpler str& instead of String
        let elem = elem.children().find(|e| e.tag_name().name() == name);

        match elem {
            None => None,
            Some(node) => {
                match node.text() {
                    Some(text) => Some(String::from(text)), //TODO: Something less "staircasey"
                    None => None,
                }
            },
        }
    }

    pub fn build(elem: roxmltree::Node) -> Result<Item> {
        // if this is absent - there's no recording
        let recorded_duration = Item::string_of_element(elem, "recordedDuration");
        let recorded_duration = match recorded_duration {
            Some(duration) => duration,
            None => return Err("future recording".into()),
        };

        let recorded_starttime = Item::string_of_element(elem, "recordedStartDateTime").unwrap();
        let id = String::from(elem.attribute("id").unwrap());
        let title = Item::string_of_element(elem, "title").unwrap();
        let description = Item::string_of_element(elem, "description").unwrap();
        let channel_name = Item::string_of_element(elem, "channelName").unwrap();
        let service_type: i64 = Item::string_of_element(elem, "X_genre").unwrap().parse().unwrap();

        let viewed = if "1" == Item::string_of_element(elem, "X_isViewed").unwrap() {true} else {false};
        let series_id = Item::string_of_element(elem, "seriesID"); //NB: optional

        Ok(Item {
            id, title, description, viewed,
            recorded_starttime, recorded_duration,
            channel_name,
            series_id,
            service_type
        })
    }
}