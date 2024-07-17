//! Types required to be used when reading CST files.

use crate::Gender;
use serde::Deserialize;

#[derive(Deserialize, Debug, PartialEq)]
pub struct HeaderParts {
    pub language: String,
    pub country: String,
    pub variant: String,
    pub age: u32,
    pub gender: Gender,
    #[serde(with = "crate::date")]
    pub build_date: chrono::NaiveDateTime,
    pub description: String,
    pub eng_shared: u32,
    pub copyright: String,
    pub num_dur_models: u32,
    pub num_param_models: u32,
    pub model_shape: u32,
    pub num_f0_models: u32,
}
