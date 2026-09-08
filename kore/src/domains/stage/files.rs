use nyanko::chapter::Category;

pub(crate) fn gatya_item_img(id: u32) -> String {
    format!("gatyaitemD_{:02}_f.png", id)
}

pub(crate) fn cat_form_img(id: u32, form: &str) -> String {
    format!("uni{:03}_{}00.png", id, form)
}

pub(crate) fn empty_cat_icon() -> String {
    "uni.png".to_string()
}

pub(crate) fn stage_name_targets(cat_prefix: &str) -> Vec<String> {
    let mut targets = vec![
        format!("StageName_{}.csv", cat_prefix),
        format!("StageName_R{}.csv", cat_prefix),
    ];

    if cat_prefix == "EC" {
        targets.push("StageName.csv".to_string());
    }

    match cat_prefix {
        "EC" => targets.push("StageName0.csv".to_string()),
        "W" => targets.push("StageName1.csv".to_string()),
        "Space" => targets.push("StageName2.csv".to_string()),
        _ => (),
    }

    targets
}

pub const MAP_NAME: &str = "Map_Name.csv";
pub const DROP_ITEM: &str = "DropItem.csv";
pub const DROP_CHARA: &str = "drop_chara.csv";
pub const GATYA_ITEM_BUY: &str = "Gatyaitembuy.csv";
pub const GATYA_ITEM_NAME: &str = "GatyaitemName.csv";

pub fn map_banner_file(map_id: u32, image_prefix: &str) -> String {
    format!("mapname{:03}{}.png", map_id, artwork_suffix(image_prefix))
}

pub fn stage_banner_file(category: &Category, map_id: u32, stage_id: u32, image_prefix: &str) -> String {
    story_banner_file(category, map_id, stage_id)
        .unwrap_or_else(|| format!("mapsn{:03}_{:02}{}.png", map_id, stage_id, artwork_suffix(image_prefix)))
}

fn artwork_suffix(image_prefix: &str) -> String {
    if image_prefix.is_empty() { String::new() } else { format!("_{}", image_prefix) }
}

fn story_banner_file(category: &Category, map_id: u32, stage_id: u32) -> Option<String> {
    let file_code = match category {
        Category::EmpireOfCats => "ec",
        Category::IntoTheFuture => "wc",
        Category::CatsOfTheCosmos => "sc",
        Category::ZombieOutbreaks => match map_id {
            0..=2 => "ec",
            4..=6 => "wc",
            7..=9 => "sc",
            _ => return None,
        },
        _ => return None,
    };

    let image_index = match stage_id {
        0..=45 => 45 - stage_id,
        49 | 50 => 47,
        id => id,
    };

    Some(format!("{}0{:02}_n.png", file_code, image_index))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn story_chapters_count_their_banners_backwards() {
        // Empire of Cats stage 45 is the first plate, ec000_n.png, and stage 0 is the last.
        let first = stage_banner_file(&Category::EmpireOfCats, 0, 45, "ec");
        let last = stage_banner_file(&Category::EmpireOfCats, 0, 0, "ec");

        assert_eq!(first, "ec000_n.png");
        assert_eq!(last, "ec045_n.png");

        // Zombie Outbreaks borrows whichever story chapter its map sits in, and has none past map 9.
        assert_eq!(stage_banner_file(&Category::ZombieOutbreaks, 5, 45, "z"), "wc000_n.png");
        assert_eq!(stage_banner_file(&Category::ZombieOutbreaks, 3, 45, "z"), "mapsn003_45_z.png");
    }

    #[test]
    fn event_chapters_name_plates_by_map_and_stage() {
        assert_eq!(map_banner_file(3, "v"), "mapname003_v.png");
        assert_eq!(stage_banner_file(&Category::TowersAndCitadels, 3, 0, "v"), "mapsn003_00_v.png");
    }
}
