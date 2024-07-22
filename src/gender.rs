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

impl core::str::FromStr for Gender {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "male" => Ok(Gender::Male),
            "female" => Ok(Gender::Female),
            "unknown" | "none" => Ok(Gender::Unknown),
            _ => Err("invalid variant for gender"),
        }
    }
}
