//! Types required to be used when reading CST files.

use crate::Gender;
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};

#[derive(Deserialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum Language {
    #[serde(rename = "eng")]
    English,
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EndOfFeatures {
    EndOfFeatures,
}

#[serde_as]
#[derive(Deserialize, Debug, PartialEq)]
pub struct Features {
    pub language: String,
    pub country: String,
    pub variant: String,
    #[serde_as(as = "DisplayFromStr")]
    pub age: u32,
    pub gender: Gender,
    #[serde(with = "crate::date")]
    pub build_date: chrono::NaiveDateTime,
    pub description: String,
    #[serde_as(as = "DisplayFromStr")]
    pub eng_shared: u32,
    pub copyright: String,
    #[serde_as(as = "DisplayFromStr")]
    pub num_dur_models: u32,
    #[serde_as(as = "DisplayFromStr")]
    pub num_param_models: u32,
    #[serde_as(as = "DisplayFromStr")]
    pub model_shape: u32,
    #[serde_as(as = "DisplayFromStr")]
    pub num_f0_models: u32,
    pub end_of_features: EndOfFeatures,
}

#[derive(Deserialize, Debug, PartialEq)]
// "Why not deserialize Header directly?"
// https://github.com/serde-rs/serde/issues/1803
// basically, the named fields (even if flattened) cause Serde to ask for the `Content` (private
// serde) type, and it needs to use `deserialize_any`, which this format does not support
#[serde(from = "_Header", into = "_Header")]
pub struct Header {
    pub features: Features,
    pub name: String,
}

#[derive(Deserialize, Debug, PartialEq)]
struct _Header(pub Features, pub String);
impl From<Header> for _Header {
    fn from(head: Header) -> _Header {
        _Header(head.features, head.name)
    }
}
impl From<_Header> for Header {
    fn from(head: _Header) -> Header {
        Header {
            features: head.0,
            name: head.1,
        }
    }
}
