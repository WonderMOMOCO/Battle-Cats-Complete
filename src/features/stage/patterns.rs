#![allow(dead_code)]

// Map and Stage Data
pub const MAP_STAGE_DATA_PATTERN: &str = r"^MapStageData([a-zA-Z]+)_(\d+)\.csv$";
pub const MAP_NAME_PATTERN: &str = r"^mapname(\d+)_([a-zA-Z]{1,3})(?:_[a-zA-Z]{2})?\.(png|imgcut|maanim|mamodel)$";
pub const MAP_SN_PATTERN: &str = r"^mapsn(\d+)_(\d+)_([a-zA-Z]{1,3})(?:_[a-zA-Z]{2})?\.(png|imgcut|maanim|mamodel)$";

// Global Map Names
pub const MAP_GLOBAL_NAME_PATTERN: &str = r"^Map_Name(?:_[a-zA-Z]{2})?\.csv$";

// Stage Normal (EoC, ItF, CotC, Zombies)
pub const STAGE_NORMAL_PATTERN: &str = r"^stageNormal(\d)(?:_(\d))?(?:_Invasion)?(?:_Z)?\.csv$";

// Individual Stages & Stage Names
pub const STAGE_FILE_PATTERN: &str = r"^stage([a-zA-Z]+)?(\d+)(?:_Invasion)?(?:_(\d+))?\.csv$";
pub const STAGE_NAME_PATTERN: &str = r"^StageName_([a-zA-Z]+)(?:_[a-zA-Z]{2})?\.csv$";

// Legacy Image Stage Names (ec022_n_ko.png, wc015_n.png)
pub const LEGACY_STAGE_NAME_PATTERN: &str = r"^([a-zA-Z][cC])(\d+)_([a-zA-Z]{1,3})(?:_[a-zA-Z]{2})?\.(png|imgcut|maanim|mamodel)$";

// Castles
pub const CASTLE_PATTERN: &str = r"^(?:castle_)?([a-zA-Z][cC])(\d+)(?:_\d+)?(?:_[a-zA-Z]{2})?\.(png|imgcut|maanim|mamodel)$";

// Backgrounds
pub const BG_MAP_PATTERN: &str = r"^map(\d+)(?:_[0-9_]+)?\.(png|imgcut|maanim|mamodel|json)$";
pub const BG_BATTLE_PATTERN: &str = r"^bg(\d+)(?:_[0-9_]+)?\.(png|imgcut|maanim|mamodel)$";
pub const BG_DATA_PATTERN: &str = r"^bg(\d+)(?:_[0-9_]+)?\.json$";
pub const BG_EFFECT_PATTERN: &str = r"^bgEffect_(\d+)(?:_[0-9_]+)?\.(png|imgcut|maanim|mamodel|json)$";

// Specifics and EX
pub const LIMIT_MSG_PATTERN: &str = r"^MapStageLimitMessage(?:_[a-zA-Z]{2})?\.csv$";
pub const EX_PATTERN: &str = r"^EX_(group|lottery|option)\.csv$";
pub const CERTIFICATION_PRESET_PATTERN: &str = r"^Certification(\d+)\.preset$";