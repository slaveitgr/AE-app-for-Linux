use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SPDisplaysDataTypeItem {
    #[serde(
        rename = "spdisplays_device-id",
        deserialize_with = "deserialize_hex_u16"
    )]
    pub device_id: u16,
    #[serde(
        rename = "spdisplays_revision-id",
        deserialize_with = "deserialize_hex_u16"
    )]
    pub revision_id: u16,
}

fn deserialize_hex_u16<'de, D>(deserializer: D) -> Result<u16, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;
    let s = if let Some(stripped) = s.strip_prefix("0x") {
        stripped
    } else {
        s
    };

    u16::from_str_radix(s, 16).map_err(serde::de::Error::custom)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SPDisplaysDataType {
    #[serde(rename = "SPDisplaysDataType")]
    pub items: Vec<SPDisplaysDataTypeItem>,
}
