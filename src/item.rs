use super::common::Result;

use serde::Serialize;
use regex::Regex;
use std::time::Duration;
use lazy_static::lazy_static;
use num_traits::FromPrimitive;
use chrono::{DateTime, FixedOffset};

#[serde(rename_all = "PascalCase")]
#[derive(Debug, Serialize, Clone)]
pub struct Item {
    pub id: String,
    pub res: String,

    pub title: String,
    pub description: String,
    pub viewed: bool,

    pub recorded_starttime: DateTime<FixedOffset>,
    pub recorded_duration: u64, //Seconds

    pub channel_name: String,
    pub series_id: Option<String>,
    pub service_type: ServiceType,
}

#[derive(Debug, Serialize, FromPrimitive, Clone)]
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

lazy_static! {
    static ref DURATION_RE: Regex = Regex::new(r"P0D(\d+):(\d+):(\d+)").expect("Cannot compile regex!");
}

fn string_of_element(elem: &roxmltree::Node, name: &str) -> Result<String> {
    let elem = elem.children().find(|e| e.tag_name().name() == name);
    let result: String = elem.ok_or(format!("Element `{}` is absent", name))?
        .text()
        .ok_or(format!("Element `{}` is empty", name))?
        .into();
    Ok(result)
}

fn parse_duration(duration: &str) -> Result<Duration> {
    let caps = DURATION_RE
        .captures(duration)
        .ok_or(format!("Cannot parse duration: {}", duration))?;

    let hours: u32 = caps.get(1).ok_or("Hours field is absent")?.as_str().parse()?;
    let mins: u32 = caps.get(2).ok_or("Minutes field is absent")?.as_str().parse()?;
    let secs: u32 = caps.get(3).ok_or("Seconds field is absent")?.as_str().parse()?;

    Ok(Duration::new((secs + (mins * 60) + (hours * (60 * 60))).into(), 0))
}


impl Item {

    pub fn build(elem: roxmltree::Node) -> Result<Item> {

        let recorded_duration = string_of_element(&elem, "recordedDuration")?;
        let recorded_duration = parse_duration(recorded_duration.as_str())?.as_secs();

        let recorded_starttime = DateTime::parse_from_rfc3339(&string_of_element(&elem, "recordedStartDateTime")?)?;
        let id = elem.attribute("id").ok_or("Field `id` is absent")?.into();
        let title = string_of_element(&elem, "title")?;
        let description = string_of_element(&elem, "description")?;
        let channel_name = string_of_element(&elem, "channelName")?;
        let res = string_of_element(&elem, "res")?;
        let service_type: i32 = string_of_element(&elem, "X_genre")?.parse()?;

        let service_type = FromPrimitive::from_i32(service_type)
            .unwrap_or(ServiceType::Unknown);

        let viewed = "1" == string_of_element(&elem, "X_isViewed")?;
        let series_id = elem.children()
            .find(|e| e.tag_name().name() == "seriesID")
            .and_then(|node| node.text()).map(String::from);

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
