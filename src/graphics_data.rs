use std::convert::TryFrom;

use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl TryFrom<&str> for Color {
    type Error = eyre::Error;

    fn try_from(s: &str) -> eyre::Result<Color> {
        lazy_static! {
            static ref HEX_REGEX: Regex =
                Regex::new(r"^#([0-9a-fA-F]{2})([0-9a-fA-F]{2})([0-9a-fA-F]{2})$").unwrap();
        }
        match s {
            "red" => {
                return Ok(Color {
                    red: 255,
                    green: 0,
                    blue: 0,
                })
            }
            "green" => {
                return Ok(Color {
                    red: 0,
                    green: 255,
                    blue: 0,
                })
            }
            "yellow" => {
                return Ok(Color {
                    red: 255,
                    green: 255,
                    blue: 0,
                })
            }
            "blue" => {
                return Ok(Color {
                    red: 0,
                    green: 0,
                    blue: 255,
                })
            }
            "black" => {
                return Ok(Color {
                    red: 0,
                    green: 0,
                    blue: 0,
                })
            }
            _ => {}
        }
        match HEX_REGEX.captures(s) {
            Some(captures) => Ok(Color {
                red: u8::from_str_radix(&captures[1], 16).unwrap(),
                green: u8::from_str_radix(&captures[2], 16).unwrap(),
                blue: u8::from_str_radix(&captures[3], 16).unwrap(),
            }),
            None => Err(eyre::eyre!("")),
        }
    }
}

impl TryFrom<String> for Color {
    type Error = eyre::Error;

    fn try_from(s: String) -> eyre::Result<Color> {
        TryFrom::<&str>::try_from(s.as_ref())
    }
}

impl From<Color> for String {
    fn from(c: Color) -> String {
        format!("#{:02x}{:02x}{:02x}", c.red, c.green, c.blue)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum Size {
    #[serde(rename = "normal")]
    Normal,
    #[serde(rename = "large")]
    Large,
}

impl Default for Size {
    fn default() -> Self {
        Size::Normal
    }
}

// TODO: does this need to be public?
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
enum Marker {
    #[serde(rename = "circle")]
    Circle,
    #[serde(rename = "cross")]
    Cross,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VectorElement {
    pub x: usize,
    pub y: usize,
    marker: Marker,
    color: Color,
    text: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum ShapeRect {
    #[serde(rename = "rect")]
    Rect,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum ShapeVect {
    #[serde(rename = "vect")]
    Vect,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Drawable {
    Rectangle {
        // id: String,
        shape: ShapeRect,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        fill: Color,
        color: Color,
        // ttl: isize,
    },
    Vector {
        // id: String,
        shape: ShapeVect,
        color: Color,
        // ttl: isize,
        vector: Vec<VectorElement>,
    },
    Text {
        // id: String,
        text: String,
        #[serde(default)]
        size: Size,
        color: Color,
        x: usize,
        y: usize,
        // ttl: isize,
    },
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Graphic {
    #[serde(deserialize_with = "serde_aux::field_attributes::deserialize_string_from_number")]
    pub id: String,
    pub ttl: isize,
    #[serde(flatten)]
    pub drawable: Option<Drawable>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EmptyGraphic {
    #[serde(deserialize_with = "serde_aux::field_attributes::deserialize_string_from_number")]
    pub id: String,
    pub ttl: isize,
}
