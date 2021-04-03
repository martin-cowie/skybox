use super::common::Result;

use serde::Serialize;
use regex::Regex;
use std::time::Duration;
use lazy_static::lazy_static;


#[serde(rename_all = "PascalCase")]
#[derive(Debug, Serialize)]
pub struct Item {
    id: String,
    res: String,

    title: String,
    description: String,
    viewed: bool,

    recorded_starttime: String,
    recorded_duration: u64, //Seconds

    channel_name: String,
    series_id: Option<String>,
    service_type: ServiceType,
}

#[derive(Debug, Serialize)]
#[repr(u8)]
pub enum ServiceType {
    Music = 16,
    Documentary = 11,
    Lifestyle = 8,
    Sport = 7,
    Movies = 6,
    News = 5,
    Entertainment = 3,
    Kids = 2,

    Unknown = 0
}

fn service_type_from(num: i32) -> ServiceType { //TODO: surely something more idiomatic
    match num {
        16 => ServiceType::Music,
        11 => ServiceType::Documentary,
        8 => ServiceType::Lifestyle,
        7 => ServiceType::Sport,
        6 => ServiceType::Movies,
        5 => ServiceType::News,
        3 => ServiceType::Entertainment,
        2 => ServiceType::Kids,
        _ => ServiceType::Unknown
    }
}

lazy_static! {
    static ref DURATION_RE: Regex = Regex::new(r"P0D(\d+):(\d+):(\d+)").expect("Cannot compile regex!");
}

//TODO: return a simpler str& instead of String
fn string_of_element(elem: roxmltree::Node, name: &str) -> Result<String> {
    let elem = elem.children().find(|e| e.tag_name().name() == name);
    let result: String = elem.ok_or(format!("Element `{}` is absent", name))?
        .text()
        .ok_or(format!("Element `{}` is empty", name))?
        .into();
    Ok(result)
}

fn parse_duration(duration: &str) -> Result<Duration> {
    let caps = match DURATION_RE.captures(duration) {
        None => return Err(format!("Cannot parse duration: {}", duration).into()),
        Some(caps) => caps
    };

    let hours: u32 = caps.get(1).ok_or("Hours field is absent")?.as_str().parse()?;
    let mins: u32 = caps.get(2).ok_or("Minutes field is absent")?.as_str().parse()?;
    let secs: u32 = caps.get(3).ok_or("Seconds field is absent")?.as_str().parse()?;

    Ok(Duration::new((secs + (mins * 60) + (hours * (60 * 60))).into(), 0))
}


impl Item {

    pub fn build(elem: roxmltree::Node) -> Result<Item> {

        let recorded_duration = string_of_element(elem, "recordedDuration")?;
        let recorded_duration = parse_duration(recorded_duration.as_str())?.as_secs();

        let recorded_starttime = string_of_element(elem, "recordedStartDateTime")?;
        let id = elem.attribute("id").ok_or("Field `id` is absent")?.into();
        let title = string_of_element(elem, "title")?;
        let description = string_of_element(elem, "description")?;
        let channel_name = string_of_element(elem, "channelName")?;
        let res = string_of_element(elem, "res")?;
        let service_type: i32 = string_of_element(elem, "X_genre")?.parse()?;

        let service_type = service_type_from(service_type);

        let viewed = "1" == string_of_element(elem, "X_isViewed")?;
        let series_id = elem.children()
            .find(|e| e.tag_name().name() == "seriesID")
            .map_or(None, |node| node.text())
            .map_or(None, |s| Some(String::from(s)));

        Ok(Item {
            id, title, description, viewed, res,
            recorded_starttime, recorded_duration,
            channel_name,
            series_id,
            service_type,
        })
    }
}

//TODO: needs unit tests