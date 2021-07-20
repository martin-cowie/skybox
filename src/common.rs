use ssdp_client::URN;

pub const SKY_PLAY: URN = URN::service("schemas-nds-com", "SkyPlay", 2);
pub const SKY_BROWSE: URN = URN::service("schemas-nds-com", "SkyBrowse", 2);


pub fn envelope(body: &str) -> String {
    format!(r#"<?xml version="1.0" encoding="utf-8"?>
        <s:Envelope s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/" xmlns:s="http://schemas.xmlsoap.org/soap/envelope/">
            <s:Body>{}</s:Body>
        </s:Envelope>"#, body)
}

pub fn as_elements(arguments: &std::collections::HashMap<&str, &str>) -> String {
    arguments.iter()
        .map(|(key, value)| format!("<{}>{}</{}>", &key, &value, &key))
        .collect::<Vec<_>>()
        .join("")
}

pub mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain! {
        foreign_links {
            Io(std::io::Error);
            ParseIntError(std::num::ParseIntError);
            PreferencesError(preferences::PreferencesError);
            ParseError(url::ParseError);
            Reqwest(reqwest::Error);
            Roxmltree(roxmltree::Error);
            Chrono(chrono::ParseError);
            SsdpClient(ssdp_client::Error);
        }
    }
}
