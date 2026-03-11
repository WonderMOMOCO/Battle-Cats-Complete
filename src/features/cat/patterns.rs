#![allow(dead_code)]

// Basic patterns
pub const CAT_ID: &str = r"(\d{3})";
pub const CAT_FORM: &str = r"([fcsu])";
pub const ANIM_TYPE: &str = r"(0[0-3])";
pub const COUNTRY_CODE: &str = r"([a-z]{2})";

// Combined patterns
pub const CAT_CODE: &str = concat!(r"(\d{3})_", r"([fcsu])");

// Cat file strings
pub const CAT_STATS_PATTERN: &str = concat!(r"^unit", r"(\d{3})", r"\.csv$");
pub const CAT_ICON_PATTERN: &str = concat!(r"^uni", r"(\d{3})_", r"([fcsu])", r"00\.png$");
pub const CAT_UPGRADE_PATTERN: &str = concat!(r"^udi", r"(\d{3})_", r"([fcsu])", r"\.png$");
pub const CAT_GACHA_PATTERN: &str = concat!(r"^gatyachara_", r"(\d{3})", r"_[fz]\.png$");
pub const CAT_ANIM_PATTERN: &str = concat!(r"^", r"(\d{3})_", r"([fcsu])", r"\.(imgcut|mamodel|png)$");
pub const CAT_MAANIM_PATTERN: &str = concat!(r"^", r"(\d{3})_", r"([fcsu])", r"(0[0-3]|_zombie0[0-2])\.maanim$");
pub const CAT_EXPLAIN_PATTERN: &str = r"^Unit_Explanation(\d+)_([a-z]{2})\.csv$";

// Egg file strings
pub const EGG_ICON_PATTERN: &str = r"^uni(\d{3})_(0[0-2])\.png$";
pub const EGG_UPGRADE_PATTERN: &str = r"^udi(\d{3})_(0[0-2])\.png$";
pub const EGG_GACHA_PATTERN: &str = r"^gatyachara_(\d{3})_[fz]\.png$";
pub const EGG_ANIM_PATTERN: &str = r"^(\d{3})_(0[0-2])\.(imgcut|mamodel|png)$";
pub const EGG_MAANIM_PATTERN: &str = r"^(\d{3})_(0[0-2])(0[0-3]|_zombie0[0-2])\.maanim$";

// Skill patterns
pub const SKILL_DESC_PATTERN: &str = r"^SkillDescriptions(?:_([a-z]{2}))?\.csv$";

pub const CAT_UNIVERSAL_PATTERN: &str = r"^unitevolve(?:_([a-z]{2}))?\.csv$";

// Master files for Cat Data
pub const CAT_UNIVERSAL_FILES: &[&str] = &[
    "SkillAcquisition.csv", 
    "SkillLevel.csv",
    "unitbuy.csv", 
    "unitexp.csv", 
    "unitlevel.csv", 
    "unitlimit.csv",
    "uni.png" 
];