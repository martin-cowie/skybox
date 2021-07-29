use super::common::errors::Result;

use serde::Serialize;
use regex::Regex;
use std::time::Duration;
use lazy_static::lazy_static;
use num_traits::FromPrimitive;
use chrono::{DateTime, FixedOffset};

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "PascalCase")]
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

#[derive(Debug, Serialize, FromPrimitive, Clone, PartialEq)]
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

        Ok(Item { id, res, title, description, viewed, recorded_starttime, recorded_duration, channel_name, series_id, service_type })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_build_fails() {

        let xml_source = r#"<?xml version="1.0"?>
    <note>
        <to>Tove</to>
        <from>Jani</from>
        <heading>Reminder</heading>
        <body>Don't forget me this weekend!</body>
    </note>
    "#;

        let document = roxmltree::Document::parse(xml_source).unwrap();
        let result = Item::build(document.root_element());
        assert!(result.is_err());
    }

    #[test]
    fn test_item_build_ok() {

        // Taken from the wild
        let xml_source = r#"<?xml version="1.0"?>
<DIDL-Lite xmlns="urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/" xmlns:upnp="urn:schemas-upnp-org:metadata-1-0/upnp/" xmlns:vx="urn:schemas-nds-com:metadata-1-0" xmlns:dc="http://purl.org/dc/elements/1.1/">
<item id="BOOK:687878212" restricted="0" parentID="3">
    <dc:title>Ewan McGregor: Cold Chain Mission</dc:title>
    <upnp:class>object.item.videoItem</upnp:class>
    <upnp:programID type="nds.com_URI">xsi://7D6;B1C9</upnp:programID>
    <res protocolInfo="internal:192.168.59.177:*:*" duration="1:03:57" size="1957124932">file://pvr/29003044</res>
    <vx:X_genre id="nds.com_internal" extended="11,2828">11</vx:X_genre>
    <upnp:rating type="nds.com_internal">0</upnp:rating>
    <upnp:scheduledStartTime>2012-04-22T21:00:00+01:00</upnp:scheduledStartTime>
    <upnp:scheduledEndTime>2012-04-22T21:00:00Z</upnp:scheduledEndTime>
    <upnp:scheduledDuration>P0D01:00:00</upnp:scheduledDuration>
    <upnp:seriesID type="nds.com_internal">13369</upnp:seriesID>
    <dc:description>1/2. Ewan McGregor is on a mission to immunise some of the hardest-to-reach children in the world. He starts in India and then continues to Nepal. Contains some strong language.  Also in HD. [AD,S]</dc:description>
    <upnp:channelNr>102</upnp:channelNr>
    <upnp:channelName>BBC 2 England</upnp:channelName>
    <upnp:channelID type="nds.com_URI">xsi://7D6</upnp:channelID>
    <vx:X_serviceType>1</vx:X_serviceType>
    <vx:X_cgmsa>0</vx:X_cgmsa>
    <vx:X_audioType>2</vx:X_audioType>
    <vx:X_flags hasForeignSubtitles="1" hd="0" hasAudioDesc="1" widescreen="1" copyProtected="0" isLinked="1" allowAnalogTaping="1" currentSeries="0" ippv="0" oppv="0" is3D="0" isAdult="0" firstRun="0" currentShow="0" uhd="0"/>
    <vx:X_baseType>2</vx:X_baseType>
    <vx:X_bookingTime>2012-04-22T19:01:01Z</vx:X_bookingTime>
    <vx:X_bookingType>2</vx:X_bookingType>
    <vx:X_bookingDiskQuotaName>user</vx:X_bookingDiskQuotaName>
    <vx:X_guardStartDur>120000</vx:X_guardStartDur>
    <vx:X_guardEndDur>120000</vx:X_guardEndDur>
    <vx:X_bookedAsOPPV>0</vx:X_bookedAsOPPV>
    <vx:X_extensionStartDur>0</vx:X_extensionStartDur>
    <vx:X_bookingActive>1</vx:X_bookingActive>
    <vx:X_bookingKeep>0</vx:X_bookingKeep>
    <vx:X_bookingLock>0</vx:X_bookingLock>
    <upnp:recordedStartDateTime>2012-04-22T20:58:02+01:00</upnp:recordedStartDateTime>
    <upnp:recordedDuration>P0D01:03:57</upnp:recordedDuration>
    <vx:X_recStatus failed="0" contentStatus="3" exception="100" recState="7" ContentType="0">5</vx:X_recStatus>
    <vx:X_lastPlaybackPosition>0</vx:X_lastPlaybackPosition>
    <vx:X_isViewed>1</vx:X_isViewed>
    <vx:X_reminderStatus isVcrTimer="0">1</vx:X_reminderStatus>
    <vx:X_isSeriesLinked>0</vx:X_isSeriesLinked>
    <vx:X_pdlPlaybackAvailable>0</vx:X_pdlPlaybackAvailable>
    <vx:X_pdlDownloadStatus>0</vx:X_pdlDownloadStatus>
    <upnp:srsRecordTaskID>RT:29003044</upnp:srsRecordTaskID>
    <vx:X_bookingSource>1</vx:X_bookingSource>
    <vx:X_canonicalName>EWAN MCGREGOR: COLD CHAIN MISSION</vx:X_canonicalName>
    <vx:X_isPlaying>0</vx:X_isPlaying>
    <vx:X_groupID>0</vx:X_groupID>
    <vx:X_subGroupID>0</vx:X_subGroupID>
    <vx:X_estimatedBitRate>5767168</vx:X_estimatedBitRate>
    <vx:X_recordingID>xsi://7D6;B1C9</vx:X_recordingID>
    <vx:X_cmdcMemberNumber>0</vx:X_cmdcMemberNumber>
    <vx:X_isBTO>0</vx:X_isBTO>
    <vx:X_subsubGroupID>0</vx:X_subsubGroupID>
    <vx:X_isShowLinked>0</vx:X_isShowLinked>
    <vx:X_showID>0</vx:X_showID>
    <vx:X_bookingARRFilters>0,0,0,0,0</vx:X_bookingARRFilters>
    <vx:X_isPdlTrailer>0</vx:X_isPdlTrailer>
    <vx:X_isTemporary>0</vx:X_isTemporary>
    <vx:X_isImmediate>0</vx:X_isImmediate>
    <vx:X_pushTrailerOffset>0</vx:X_pushTrailerOffset>
    <vx:X_pdlQueuePosition>0</vx:X_pdlQueuePosition>
    <vx:X_isSplitEvent>0</vx:X_isSplitEvent>
    <vx:X_lastViewedTime>2021-05-25T21:06:04Z</vx:X_lastViewedTime>
    <vx:X_purchasePacketsId>0</vx:X_purchasePacketsId>
    <vx:X_bookingJobDeletionTime>1970-01-01T00:00:00Z</vx:X_bookingJobDeletionTime>
    <vx:X_actualEndTime>2012-04-22T21:01:59Z</vx:X_actualEndTime>
    <vx:X_localActualEndTime>2012-04-22T22:01:59Z</vx:X_localActualEndTime>
    <vx:X_bookingExpirationTime>1970-01-01T00:00:00Z</vx:X_bookingExpirationTime>
    <vx:X_totalChildSize>0</vx:X_totalChildSize>
    <vx:X_pushHasValidTrailer>1</vx:X_pushHasValidTrailer>
    <vx:X_allowCopyToPlanner>1</vx:X_allowCopyToPlanner>
    <vx:X_expireFromPlanner>0</vx:X_expireFromPlanner>
    <vx:X_oigProgId>0</vx:X_oigProgId>
    <vx:X_parentalRatingScheme>1</vx:X_parentalRatingScheme>
    <vx:X_pinRating>0</vx:X_pinRating>
    <vx:X_pinRatingScheme>1</vx:X_pinRatingScheme>
    <vx:X_serviceFlags isDTT="0"/>
    <vx:X_purchaseType>0</vx:X_purchaseType>
    <vx:X_isEntitled>0</vx:X_isEntitled>
    <vx:X_dynamicRange>0</vx:X_dynamicRange>
</item>
</DIDL-Lite>
        "#;

            let document = roxmltree::Document::parse(xml_source).unwrap();
            let result = Item::build(document.root_element().first_element_child().unwrap());
            assert!(result.is_ok());

            let item = result.unwrap();
            println!("{:?}", item);

            assert_eq!(item.id, "BOOK:687878212");
            assert_eq!(item.res, "file://pvr/29003044");

            assert_eq!(item.title, "Ewan McGregor: Cold Chain Mission");
            assert_eq!(item.description, "1/2. Ewan McGregor is on a mission to immunise some of the hardest-to-reach children in the world. He starts in India and then continues to Nepal. Contains some strong language.  Also in HD. [AD,S]");
            assert_eq!(item.viewed, true);

            assert_eq!(item.recorded_starttime, DateTime::parse_from_rfc3339("2012-04-22T20:58:02+01:00").unwrap());
            assert_eq!(item.recorded_duration, 3837);

            assert_eq!(item.channel_name, "BBC 2 England");
            assert_eq!(item.series_id, Some("13369".into()));
            assert_eq!(item.service_type, ServiceType::Documentary);
       }

}