use std::path::{Path, PathBuf};
use crate::features::data::structure::{cat, enemy, global, stage};
use crate::features::cat::patterns as cat_patterns;
use crate::global::io::patterns as global_patterns;

pub struct AssetRouter {
    cat_matcher: cat::CatMatcher,
    enemy_matcher: enemy::EnemyMatcher,
    global_matcher: global::GlobalMatcher,
    stage_matcher: stage::StageMatcher,
    
    cats_dir: PathBuf,
    sheets_dir: PathBuf,
    ui_dir: PathBuf,
    tables_dir: PathBuf,
    enemy_dir: PathBuf,
    stages_dir: PathBuf,
    raw_dir: PathBuf,
    audio_dir: PathBuf,
}

impl AssetRouter {
    pub fn new(game_root: &Path) -> Self {
        Self {
            cat_matcher: cat::CatMatcher::new(),
            enemy_matcher: enemy::EnemyMatcher::new(),
            global_matcher: global::GlobalMatcher::new(),
            stage_matcher: stage::StageMatcher::new(),
            
            cats_dir: game_root.join("cats"),
            sheets_dir: game_root.join("sheets"),
            ui_dir: game_root.join("ui"),
            tables_dir: game_root.join("tables"),
            enemy_dir: game_root.join("enemies"),
            stages_dir: game_root.join("stages"),
            raw_dir: game_root.join("raw"),
            audio_dir: game_root.join("audio"),
        }
    }

    fn clean_base_name(stem: &str, ext: &str) -> String {
        for &(code, _) in global_patterns::APP_LANGUAGES {
            let suffix = format!("_{}", code);
            if stem.len() > suffix.len() && stem.ends_with(&suffix) {
                let clean_stem = &stem[..stem.len() - suffix.len()];
                return if ext.is_empty() { clean_stem.to_string() } else { format!("{}.{}", clean_stem, ext) };
            }
        }
        if ext.is_empty() { stem.to_string() } else { format!("{}.{}", stem, ext) }
    }

    fn is_cat_base_banner(name: &str, clean_name: &str) -> bool {
        if !name.starts_with("udi") || name.len() < 6 { return false; }
        let Ok(id) = name[3..6].parse::<u32>() else { return false; };
        if id > 9 { return false; }
        name != clean_name
    }

    pub fn resolve_destination(&self, original_name: &str, final_name: &str) -> PathBuf {
        let path = Path::new(original_name);
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let ext = path.extension().unwrap_or_default().to_string_lossy();
        let base_name = Self::clean_base_name(&stem, &ext);

        if Self::is_cat_base_banner(original_name, &base_name) {
            return self.cats_dir.join("CatBase").join(final_name);
        }

        if cat_patterns::CAT_UNIVERSAL_FILES.contains(&base_name.as_str()) {
            return self.cats_dir.join(final_name);
        } 

        let routed_folder = self.global_matcher.get_dest(&base_name, &self.sheets_dir, &self.ui_dir, &self.tables_dir, &self.audio_dir)
            .or_else(|| self.cat_matcher.get_dest(&base_name, &self.cats_dir))
            .or_else(|| self.enemy_matcher.get_dest(&base_name, &self.enemy_dir))
            .or_else(|| self.stage_matcher.get_dest(&base_name, &self.stages_dir));

        if let Some(folder) = routed_folder {
            folder.join(final_name)
        } else {
            self.raw_dir.join(final_name) 
        }
    }
}