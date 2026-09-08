pub const UNIT_BUY: &str = "unitbuy.csv";
pub const UNIT_LEVEL: &str = "unitlevel.csv";
pub const SKILL_ACQUISITION: &str = "SkillAcquisition.csv";
pub const SKILL_LEVEL: &str = "SkillLevel.csv";
pub const SKILL_DESCRIPTIONS: &str = "SkillDescriptions.csv";
pub(crate) const UNIT_EVOLVE: &str = "unitevolve.csv";
pub const NYANCOMBO_DATA: &str = "NyancomboData.csv";
pub const NYANCOMBO_NAME: &str = "Nyancombo.csv";
pub(crate) const NYANCOMBO_EFFECT: &str = "Nyancombo1.csv";
pub(crate) const NYANCOMBO_BAND: &str = "Nyancombo2.csv";
pub(crate) const NYANCOMBO_FILTER: &str = "NyancomboFilter.tsv";
pub(crate) const NYANCOMBO_PARAM: &str = "NyancomboParam.tsv";
pub(crate) const EMPTY_ICON: &str = "uni.png";

#[derive(Copy, Clone, PartialEq)]
pub(crate) enum AssetType {
    Icon,
    Banner,
}

impl AssetType {
    pub(crate) fn prefix(self) -> &'static str {
        match self {
            AssetType::Icon => "uni",
            AssetType::Banner => "udi",
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum AnimType {
    Mamodel,
    Imgcut,
    Png,
}

impl AnimType {
    pub fn ext(&self) -> &str {
        match self {
            AnimType::Mamodel => "mamodel",
            AnimType::Imgcut => "imgcut",
            AnimType::Png => "png",
        }
    }
}

pub const FORM_COUNT: usize = 4;

pub fn form_name(form: usize) -> &'static str {
    match form {
        0 => "Normal",
        1 => "Evolved",
        2 => "True",
        _ => "Ultra",
    }
}

pub fn anim_base_filename(id: u32, form: usize, egg_ids: (i32, i32)) -> String {
    let (egg_norm, egg_evol) = egg_ids;
    let form_char = match form { 0 => "f", 1 => "c", 2 => "s", _ => "u" };

    if form == 0 && egg_norm != -1 {
        format!("{:03}_m", egg_norm)
    } else if form == 1 && egg_evol != -1 {
        format!("{:03}_m", egg_evol)
    } else {
        format!("{:03}_{}", id, form_char)
    }
}

pub(crate) fn image_stem(asset_type: AssetType, id: u32, form: usize, egg_ids: (i32, i32)) -> String {
    let (egg_norm, egg_evol) = egg_ids;
    let prefix = asset_type.prefix();
    let form_char = match form { 0 => "f", 1 => "c", 2 => "s", _ => "u" };

    if form == 0 && egg_norm != -1 {
        return format!("{}{:03}_m00", prefix, egg_norm);
    }
    if form == 1 && egg_evol != -1 {
        return format!("{}{:03}_m01", prefix, egg_evol);
    }
    match asset_type {
        AssetType::Icon => format!("{}{:03}_{}00", prefix, id, form_char),
        AssetType::Banner => format!("{}{:03}_{}", prefix, id, form_char),
    }
}

pub fn anim_file(id: u32, form: usize, egg_ids: (i32, i32), file_type: AnimType) -> String {
    format!("{}.{}", anim_base_filename(id, form, egg_ids), file_type.ext())
}

pub fn maanim_file(id: u32, form: usize, egg_ids: (i32, i32), index: usize) -> String {
    format!("{}{:02}.maanim", anim_base_filename(id, form, egg_ids), index)
}

pub fn banner_file(id: u32, form: usize, egg_ids: (i32, i32)) -> String {
    format!("{}.png", image_stem(AssetType::Banner, id, form, egg_ids))
}

pub(crate) fn icon_file(id: u32, form: usize, egg_ids: (i32, i32)) -> String {
    format!("{}.png", image_stem(AssetType::Icon, id, form, egg_ids))
}

pub fn banner_base(id: u32) -> String {
    format!("{}{:03}", AssetType::Banner.prefix(), id)
}

pub fn explanation_file(id: u32) -> String {
    format!("Unit_Explanation{}.csv", id + 1)
}

pub fn stats_file(id: u32) -> String {
    format!("unit{:03}.csv", id + 1)
}

const STATS_DIGITS: usize = 3;

pub fn stats_id(filename: &str) -> Option<u32> {
    let digits = filename.strip_prefix("unit")?.strip_suffix(".csv")?;

    if digits.len() != STATS_DIGITS || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }

    digits.parse::<u32>().ok()?.checked_sub(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stats_file_and_stats_id_are_inverses() {
        for id in 0..900u32 {
            assert_eq!(stats_id(&stats_file(id)), Some(id), "round trip for cat {id}");
        }
    }

    #[test]
    fn stats_id_rejects_the_roster_wide_tables() {
        for name in [UNIT_BUY, UNIT_LEVEL, UNIT_EVOLVE, "unitexp.csv", "unitlimit.csv", "unit000.csv", "unit0001.csv", "unit44.csv", "uni044.png"] {
            assert_eq!(stats_id(name), None, "{name} is not a per-unit stats file");
        }
    }
}
