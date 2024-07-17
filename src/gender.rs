use serde::Deserialize;

#[derive(Deserialize, Debug, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Gender {
    Male,
    Female,
    #[default]
    #[serde(alias = "none")]
    // TODO: make Option<Gender>
    Unknown,
}
