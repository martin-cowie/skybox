use ssdp_client::URN;


// A simple type alias so as to DRY.
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

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