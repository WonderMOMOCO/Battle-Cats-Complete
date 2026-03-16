#![allow(dead_code)]

// Global UI / Assets
pub const ASSET_IMG015_PATTERN: &str = r"^img015(?:_([a-z]{2}))?\.png$";
pub const ASSET_015CUT_PATTERN: &str = r"^img015(?:_([a-z]{2}))?\.imgcut$";
pub const ASSET_IMG022_PATTERN: &str = r"^img022(?:_([a-z]{2}))?\.png$";
pub const ASSET_022CUT_PATTERN: &str = r"^img022(?:_([a-z]{2}))?\.imgcut$";
pub const SKILL_NAME_PATTERN: &str = r"^Skill_name_\d{3}\.png$";

// Gatya Items
pub const GATYA_ITEM_D_PATTERN: &str = r"^gatyaitemD_(\d{2,3})_([fz])\.png$"; 
pub const GATYA_ITEM_BUY_PATTERN: &str = r"^Gatyaitembuy\.csv$";
pub const GATYA_ITEM_NAME_PATTERN: &str = r"^GatyaitemName(?:_([a-z]{2}))?\.csv$";

// Country Codes
pub const GLOBAL_CODES: &[&str] = &["de", "en", "es", "fr", "it", "th"];
pub const REGION_CODES: &[&str] = &["en", "jp", "kr", "tw"];

// Files that go by "if line count is higher" logic
// instead of "if file size is larger" logic
pub const CHECK_LINE_FILES: &[&str] = &[
    "SkillDescriptions.csv",
    "SkillAcquisition.csv",
    "SkillLevel.csv",
    "unitbuy.csv",
    "unitevolve.csv",
    "unitlevel.csv",
    "t_unit.csv"
];

// Files that have regional variants but no
// Country Codes within their source
pub const APP_LANGUAGES: &[(&str, &str)] = &[
    ("en", "English"),
    ("ja", "Japanese"), 
    ("tw", "Taiwanese"),
    ("ko", "Korean"),   
    ("es", "Spanish"),
    ("de", "German"),
    ("fr", "French"),
    ("it", "Italian"),
    ("th", "Thai"),
];