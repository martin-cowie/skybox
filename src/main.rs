use clap::clap_app;

use futures::prelude::*;
use futures::join;

use indicatif::ProgressBar;
use maplit::hashmap;
use preferences::{AppInfo, PreferencesMap, Preferences};
use reqwest;
use roxmltree;
use ssdp_client::{URN, SearchTarget};
use std::io::{self, BufRead};
use std::time::{Duration, Instant};
use tokio;
use url::Url;

mod item;
use item::Item;
mod common;
use common::{envelope, as_elements, Result};

const TIMEOUT: Duration = Duration::from_secs(5);
const APP_INFO: AppInfo = AppInfo{name: "skybox", author: "Martin Cowie"};
const PREFS_KEY: &str = "skybox/location";

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

    if let Some(matches) = matches.subcommand_matches("ls") {
        return list_items(matches).await;
    } else
    if let Some(_matches) = matches.subcommand_matches("scan") {
        return scan_skyplus().await;
    } else
    if let Some(matches) = matches.subcommand_matches("rm") {
        return remove_items(matches).await;
    }

    Ok(())
}

const SKY_BROWSE_URN: &str = "urn:schemas-nds-com:service:SkyBrowse:2";
// const SEARCH_TARGET: URN = URN::service("schemas-nds-com", "SkyBrowse", 2); //FIXME: these two are the same as &str

const SKY_PLAY: URN = URN::service("schemas-nds-com", "SkyPlay", 2);
const SKY_BROWSE: URN = URN::service("schemas-nds-com", "SkyBrowse", 2);

async fn scan_skyplus() -> Result<()> {

    let spinner = ProgressBar::new_spinner();
    spinner.enable_steady_tick(120);
    spinner.set_message("Scanning...");

    let play: &SearchTarget = &SKY_PLAY.into(); //NB: shame this cannot be done in the `search` calls
    let browse: &SearchTarget = &SKY_BROWSE.into();

    let (_play_locations, browse_locations) = join!(
        ssdp_search(play),
        ssdp_search(browse)
    );

    let mut boxes: Vec<PreferencesMap<String>> = Vec::new();

    // Get get service-url for each location
    for location in browse_locations? {
        let client = reqwest::Client::new();
        let resp = client.get(location.clone())
            .header("user-agent", "SKY_skyplus")
            .send()
            .await?
            .text()
            .await?;

        let doc = roxmltree::Document::parse(&resp).unwrap();
        let browse_url = extract_service_url(&doc, SKY_BROWSE_URN, &location);

        let mut faves: PreferencesMap<String> = PreferencesMap::new();
        faves.insert(SKY_BROWSE_URN.into(), browse_url.to_string());
        boxes.push(faves);
    }


    spinner.finish_with_message(format!("Found {} skybox", boxes.len()).as_str());

    for (i,skybox) in boxes.iter().enumerate() {
        println!("{}:\t{:?}", i, skybox);
    }
    eprint!("Choose a skybox: "); //TODO: rethink all uses of unwrap

    let line = io::stdin().lock().lines().next().unwrap()?;
    let line_number: usize = line.parse().unwrap();

    let faves = &boxes[line_number];
    println!("Using {:?}", faves);

    // Store the user's preferences
    faves.save(&APP_INFO, PREFS_KEY).unwrap();

    Ok(())
}

// Get XPath /root/device/serviceList/service[serviceType/text()='${serviceType}']/controlURL/text()
fn extract_service_url(doc: &roxmltree::Document, urn: &str, root_url: &Url) -> Url {
    let service_type_elem = doc.descendants().find(|n|
        n.tag_name().name() == "serviceType" &&
        n.text() == Some(urn)
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

async fn ssdp_search(st: &SearchTarget) -> Result<Vec<Url>> {
    let mut result: Vec<Url> = Vec::new();
    let mut responses = ssdp_client::search(st.into(), TIMEOUT, 2).await?;
    while let Some(response) = responses.next().await {
        let response = response?;
        result.push(Url::parse(response.location())?);
    }
    Ok(result)
}

async fn remove_items(matches: &clap::ArgMatches) -> Result<()> {
    let files: Vec<_> = matches.values_of("filenames").unwrap().collect();

    let faves = PreferencesMap::<String>::load(&APP_INFO, PREFS_KEY)?;
    let service_url = &faves[SKY_BROWSE_URN]; //TODO: handle fails
    let service_url = Url::parse(service_url)?;

    for item in files.iter() {
        remove_item(&service_url, item).await.unwrap();
    }

    Ok(())
}

async fn remove_item(service_url: &reqwest::Url, item_id: &str) -> Result<()> {
    eprintln!("Removing: {} using {}", item_id, service_url);

    let destroy_elem = format!(
        "<u:DestroyObject xmlns:u=\"urn:schemas-nds-com:service:SkyBrowse:2\">{}</u:DestroyObject>",
        as_elements(&hashmap!{
            "ObjectID" => item_id
        }));

    let body = envelope(destroy_elem.as_str());

    let client = reqwest::Client::new();
    let resp = client.post(service_url.clone()) //TODO: refactor to address repetition
        .header("user-agent", "SKY_skyplus")
        .header("Content-Type", "text/xml; charset=\"utf-8\"")
        .header("SOAPACTION", "\"urn:schemas-nds-com:service:SkyBrowse:2#DestroyObject\"")
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

async fn list_items(matches: &clap::ArgMatches) -> Result<()> {
    let query_start = Instant::now();

    let _long_listing = matches.is_present("long"); //TODO
    let faves = PreferencesMap::<String>::load(&APP_INFO, PREFS_KEY)?;
    let service_url = &faves[SKY_BROWSE_URN]; //TODO: handle fails
    let service_url = Url::parse(service_url)?;

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
    eprintln!("Fetched {} items from {} in {}s", starting_index, service_url, query_start.elapsed().as_secs());

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

    let arguments = as_elements(&arguments);

    let browse_elem = format!("<u:Browse xmlns:u=\"urn:schemas-nds-com:service:SkyBrowse:2\">{}</u:Browse>", arguments);
    let body = envelope(browse_elem.as_str());

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
