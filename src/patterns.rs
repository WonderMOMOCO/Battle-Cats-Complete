// Basic patterns
pub const CAT_ID: &str = r"(\d{3})";            // 001
pub const CAT_FORM: &str = r"([fcsu])";         // f, c, s, u
pub const ANIM_TYPE: &str = r"(0[0-3])";        // 00, 01, 02, 03
pub const COUNTRY_CODE: &str = r"([a-z]{2})";   // ja, en, tw, ko

pub const ENEMY_CODE: &str = concat!(r"(\d{3})_e"); 

// Combined patterns
pub const CAT_CODE: &str = concat!(r"(\d{3})_", r"([fcsu])"); 

// File strings
pub const CAT_STATS_PATTERN: &str = concat!(r"^unit", r"(\d{3})", r"\.csv$");
pub const CAT_ICON_PATTERN: &str = concat!(r"^uni", r"(\d{3})_", r"([fcsu])", r"00\.png$");
pub const CAT_UPGRADE_PATTERN: &str = concat!(r"^udi", r"(\d{3})_", r"([fcsu])", r"\.png$");
pub const CAT_GACHA_PATTERN: &str = concat!(r"^gatyachara_", r"(\d{3})", r"_[fz]\.png$");
pub const CAT_ANIM_PATTERN: &str = concat!(r"^", r"(\d{3})_", r"([fcsu])", r"\.(png|imgcut|mamodel)$");
pub const CAT_MAANIM_PATTERN: &str = concat!(r"^", r"(\d{3})_", r"([fcsu])", r"(0[0-3])", r"\.maanim$");
pub const CAT_EXPLAIN_PATTERN: &str = concat!(r"^Unit_Explanation", r"(\d{1,3})_", r"([a-z]{2})", r"\.csv$");

pub const CAT_UNIVERSAL_PATTERN: &str = concat!(r"^unitevolve_", r"([a-z]{2})", r"\.csv$");

pub const ASSET_IMG015_PATTERN: &str = r"^img015_([a-z]{2})\.png$";
pub const ASSET_015CUT_PATTERN: &str = r"^img015\.imgcut$";

// Master files for Cat Data
pub const CAT_UNIVERSAL_FILES: &[&str] = &[
    "unitbuy.csv", 
    "unitexp.csv", 
    "unitlevel.csv", 
    "unitlimit.csv",
    "uni.png" 
];

// Country Codes for the 6 languages in Global version
pub const GLOBAL_CODES: &[&str] = &[
    "de", 
    "en", 
    "es", 
    "fr", 
    "it", 
    "th"
];

pub const REGION_CODES: &[&str] = &[
    "ja", 
    "en", 
    "tw", 
    "ko", 
];

// Files that go by "if line count is higher" logic
// instead of "if file size is larger" logic
pub const CHECK_LINE_FILES: &[&str] = &[
    "unitbuy.csv", 
    "unitexp.csv", 
    "unitlevel.csv", 
    "unitlimit.csv",
    "SkillAcquisition.csv", 
    "SkillLevel.csv"
];

// Files essential for app function
// (Folder Path, Filename Prefix, [Required Extensions])
pub const ESSENTIAL_FILES: &[(&str, &str, &[&str])] = &[
    ("game/assets/img015", "img015", &["png", "imgcut"]),
];

// Files that have regional variants but no
// Country Codes within their source
pub const REGION_SENSITIVE_FILES: &[&str] = &[
    "img015.imgcut", 
    "img015.png", 
    "SkillDescriptions.csv"];