pub fn format_energy_cost(category_prefix: &str, raw_energy_cost: u32) -> String {
    if category_prefix != "B" {
        return raw_energy_cost.to_string();
    }

    if raw_energy_cost < 1000 {
        return format!("{}A", raw_energy_cost);
    }
    
    if raw_energy_cost < 2000 {
        return format!("{}B", raw_energy_cost % 1000);
    }
    
    format!("{}C", raw_energy_cost % 1000)
}

pub fn format_difficulty_level(raw_difficulty: u16) -> String {
    if raw_difficulty == 0 {
        return "-".to_string();
    }
    format!("★{}", raw_difficulty)
}

pub fn format_crown_display(target_crowns: i8, max_crowns: u8) -> String {
    let crown_symbol = "♔"; 
    
    if target_crowns != -1 {
        return format!("{}{}", target_crowns + 1, crown_symbol);
    }
    
    if max_crowns > 1 {
        return format!("1{}~{}{}", crown_symbol, max_crowns, crown_symbol);
    }
    
    format!("1{}", crown_symbol)
}

pub fn format_boolean_status(is_active: bool, active_label: &str, inactive_label: &str) -> String {
    if is_active {
        return active_label.to_string();
    }
    inactive_label.to_string()
}

pub fn format_base_display(anim_base_id: u32, standard_base_id: i32) -> (String, String) {
    if anim_base_id != 0 {
        let calculated_enemy_id = if anim_base_id >= 2 { anim_base_id - 2 } else { 0 };
        return ("Anim Base".to_string(), format!("E-{:03}", calculated_enemy_id));
    }
    ("Base Img".to_string(), standard_base_id.to_string())
}

pub fn format_boss_track(boss_track: u32, bgm_change_percent: u32) -> String {
    if boss_track == 0 && bgm_change_percent == 0 {
        return "-".to_string();
    }
    format!("Trk {} ({}%)", boss_track, bgm_change_percent)
}