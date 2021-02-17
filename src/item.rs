use serde::Serialize;
use super::common::Result;

#[derive(Debug, Serialize)]
pub struct Item {
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

//TODO: needs unit tests