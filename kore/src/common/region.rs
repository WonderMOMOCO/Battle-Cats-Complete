use nyanko::combat::Separator;
use serde::Deserialize;
use serde::Serialize;

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, Deserialize, Serialize)]
pub enum Region {
    Ja,
    En,
    Tw,
    Ko,
}

#[derive(Clone, Copy, Debug)]
pub struct RegionMetadata {
    pub package_suffix: &'static str,
    pub display_name: &'static str,
}

impl Region {
    pub const fn metadata(&self) -> RegionMetadata {
        match self {
            Region::Ja => RegionMetadata {
                package_suffix: "",
                display_name: "Japan",
            },
            Region::En => RegionMetadata {
                package_suffix: "en",
                display_name: "Global",
            },
            Region::Tw => RegionMetadata {
                package_suffix: "tw",
                display_name: "Taiwan",
            },
            Region::Ko => RegionMetadata {
                package_suffix: "kr",
                display_name: "Korea",
            },
        }
    }

}

impl std::str::FromStr for Region {
    type Err = ();

    fn from_str(input_string: &str) -> Result<Self, Self::Err> {
        match input_string.to_lowercase().as_str() {
            "ja" | "jp" | "battlecats" => Ok(Region::Ja),
            "en" => Ok(Region::En),
            "tw" => Ok(Region::Tw),
            "ko" | "kr" => Ok(Region::Ko),
            _ => Err(()),
        }
    }
}

const JAPANESE: &str = "ja";

pub fn text_separator(name: &str) -> Separator {
    let stem = name.rsplit_once('.').map_or(name, |(head, _)| head);
    let japanese = stem.rsplit_once('_').is_some_and(|(_, code)| code == JAPANESE);

    if japanese { Separator::Comma } else { Separator::Pipe }
}
