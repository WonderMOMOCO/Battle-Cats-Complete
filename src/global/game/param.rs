#![allow(dead_code)]
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct Param {
    pub unknown_1: i32, // slot_font_size  18
    pub unknown_2: i32, // slot_bg_y  100
    pub unknown_3: i32, // slot_name_margin_top  2
    pub unknown_4: i32, // slot_name_margin_left  11
    pub unknown_5: i32, // slot_page_y_offset  31
    pub unknown_6: i32, // slot_font_size2  14
    pub unknown_7: i32, // slot_name_margin_top2  9
    pub unknown_8: i32, // slot_name_margin_left2  10
    pub unknown_9: i32, // battle_wave_s_st  20
    pub unknown_10: i32, // battle_wave_s_inter  2
    pub unknown_11: i32, // battle_wave_s_frame1  2
    pub unknown_12: i32, // battle_wave_s_frame2  5
    pub unknown_13: i32, // battle_slot_2lines_line  95
    pub unknown_14: i32, // battle_slot_2lines_y  -15
    pub unknown_15: i32, // battle_zoom_y  520
    pub unknown_16: i32, // battle_zoom_slot_y  500
    pub unknown_17: i32, // battle_zoom_castle_y  20
    pub hold_cat_details_time: i32, // battle_slot_desc_wait  14
    pub time_until_death_surge: i32, // battle_death_volcano_time  21
    pub behemoth_slayer_attack_multiplier: f32, // battle_super_beast_hunter  2500
    pub behemoth_slayer_defense_multiplier: f32, // battle_super_beast_hunter_df  600
    pub unknown_22: i32, // boost_speed  3
    pub logo_superfeline_roll_timer: i32, // logo_gimmick_sec  20
    pub logo_superfeline_chance_success: i32, // logo_gimmick_probability  25
    pub logo_superfeline_failure_increase: i32, // logo_gimmick_increase  10
    pub unknown_26: i32, // battle_demon_summon_frame  50
    pub unknown_27: i32, // battle_demon_summon_se_frame  18
    pub unknown_28: i32, // battle_castle_x0  130
    pub unknown_29: i32, // battle_castle_y0  -30
    pub unknown_30: i32, // battle_castle_x1  -130
    pub unknown_31: i32, // battle_castle_y1  -30
    pub unknown_32: i32, // battle_castle_x2  0
    pub unknown_33: i32, // battle_castle_y2  0
    pub volume_setting_percent_small: i32, // volume_percentage_s  20
    pub volume_setting_percent_medium: i32, // volume_percentage_m  50
    pub volume_setting_percent_large: i32, // volume_percentage_l  100
    pub spirit_position_offset: i32, // summon_position  -150
    pub sage_type_resist_weaken: f32, // battle_super_sage_weaken  70
    pub sage_type_resist_freeze: f32, // battle_super_sage_freeze  70
    pub sage_type_resist_slow: f32, // battle_super_sage_slow  70
    pub sage_type_resist_curse: f32, // battle_super_sage_curse  70
    pub sage_type_resist_knockback: f32, // battle_super_sage_knockback  70
    pub sage_slayer_resist_weaken: f32, // battle_super_sage_hunter_weaken  70
    pub sage_slayer_resist_freeze: f32, // battle_super_sage_hunter_freeze  70
    pub sage_slayer_resist_slow: f32, // battle_super_sage_hunter_slow  70
    pub sage_slayer_resist_curse: f32, // battle_super_sage_hunter_curse  70
    pub sage_slayer_resist_other: f32, // battle_super_sage_hunter_knockback  70
    pub sage_slayer_resist_warp: f32, // battle_super_sage_hunter_warp  70
    pub sage_slayer_attack_multiplier: f32, // battle_super_sage_hunter_damage1  1200
    pub sage_slayer_defense_multiplier: f32, // battle_super_sage_hunter_damage2  500
    pub metal_killer_hitpoint_behavior: i32, // battle_metal_killer_hp_type  1
    pub unknown_52: i32, // catbase_banner1  3
    pub unknown_53: i32, // catbase_banner2  -2
    pub story_sequence_hold_speed: i32, // story_fast_forward_speed  3
    pub unknown_55: i32, // battle_explosion_frame1  11
    pub unknown_56: i32, // battle_explosion_frame2  15
    pub unknown_57: i32, // battle_explosion_frame3  0
    pub unknown_58: i32, // battle_explosion_frame4  10
    pub unknown_59: i32, // battle_explosion_width1  600
    pub unknown_60: i32, // battle_explosion_width2  400
    pub unknown_61: i32, // battle_explosion_width3  400
    pub unknown_62: i32, // battle_explosion_damage1  100
    pub unknown_63: i32, // battle_explosion_damage2  70
    pub unknown_64: i32, // battle_explosion_damage3  40
    pub unknown_65: i32, // battle_af_death_volcano_distance  800
    pub unknown_66: i32, // battle_af_death_volcano_width  1200
    pub unknown_67: i32, // battle_af_condition_production_0  2
    pub unknown_68: i32, // battle_af_condition_production_1  2
    pub unknown_69: i32, // battle_af_condition_production_2  2
    pub unknown_70: i32, // battle_af_condition_production_3  2
    pub unknown_71: i32, // battle_af_condition_production_4  2
    pub unknown_72: i32, // battle_af_condition_production_5  2
    pub unknown_73: i32, // battle_af_condition_production_type  0
    pub unknown_74: i32, // autoset_level_param  3
    pub unknown_75: i32, // autoset_must_param  5
    pub unknown_76: i32, // autoset_cut_score_rate  20
    pub unknown_77: i32, // autoset_non_target_offset  340
    pub unknown_78: i32, // battle_powerup_kill_enemy  10
    pub comeback_treasure_festival_days: i32, // comeback_treasure_day  14
    pub unknown_80: i32, // update_version_notification_interval  8
    pub unknown_81: i32, // battle_sentai_recast  75
    
    pub rest: Vec<i32>,
}

impl Param {
    pub fn from_tsv(content: &str) -> Self {
        let mut values = Vec::new();

        for line in content.lines() {
            if line.trim().is_empty() { continue; }
            
            // `param.tsv` uses a mix of spaces and tabs. 
            // Splitting by whitespace and taking the last segment isolates the integer.
            if let Some(val_str) = line.split_whitespace().last() {
                if let Ok(val) = val_str.parse::<i32>() {
                    values.push(val);
                } else {
                    values.push(0);
                }
            }
        }

        let get_val = |index: usize| -> i32 { values.get(index).copied().unwrap_or(0) };

        let mut rest_vector = Vec::new();
        if values.len() > 81 {
            rest_vector.extend_from_slice(&values[81..]);
        }

        Self {
            unknown_1: get_val(0),
            unknown_2: get_val(1),
            unknown_3: get_val(2),
            unknown_4: get_val(3),
            unknown_5: get_val(4),
            unknown_6: get_val(5),
            unknown_7: get_val(6),
            unknown_8: get_val(7),
            unknown_9: get_val(8),
            unknown_10: get_val(9),
            unknown_11: get_val(10),
            unknown_12: get_val(11),
            unknown_13: get_val(12),
            unknown_14: get_val(13),
            unknown_15: get_val(14),
            unknown_16: get_val(15),
            unknown_17: get_val(16),
            hold_cat_details_time: get_val(17),
            time_until_death_surge: get_val(18),
            behemoth_slayer_attack_multiplier: get_val(19) as f32 / 1000.0,
            behemoth_slayer_defense_multiplier: get_val(20) as f32 / 1000.0,
            unknown_22: get_val(21),
            logo_superfeline_roll_timer: get_val(22),
            logo_superfeline_chance_success: get_val(23),
            logo_superfeline_failure_increase: get_val(24),
            unknown_26: get_val(25),
            unknown_27: get_val(26),
            unknown_28: get_val(27),
            unknown_29: get_val(28),
            unknown_30: get_val(29),
            unknown_31: get_val(30),
            unknown_32: get_val(31),
            unknown_33: get_val(32),
            volume_setting_percent_small: get_val(33),
            volume_setting_percent_medium: get_val(34),
            volume_setting_percent_large: get_val(35),
            spirit_position_offset: get_val(36),
            sage_type_resist_weaken: get_val(37) as f32 / 100.0,
            sage_type_resist_freeze: get_val(38) as f32 / 100.0,
            sage_type_resist_slow: get_val(39) as f32 / 100.0,
            sage_type_resist_curse: get_val(40) as f32 / 100.0,
            sage_type_resist_knockback: get_val(41) as f32 / 100.0,
            sage_slayer_resist_weaken: get_val(42) as f32 / 100.0,
            sage_slayer_resist_freeze: get_val(43) as f32 / 100.0,
            sage_slayer_resist_slow: get_val(44) as f32 / 100.0,
            sage_slayer_resist_curse: get_val(45) as f32 / 100.0,
            sage_slayer_resist_other: get_val(46) as f32 / 100.0,
            sage_slayer_resist_warp: get_val(47) as f32 / 100.0,
            sage_slayer_attack_multiplier: get_val(48) as f32 / 1000.0,
            sage_slayer_defense_multiplier: get_val(49) as f32 / 1000.0,
            metal_killer_hitpoint_behavior: get_val(50),
            unknown_52: get_val(51),
            unknown_53: get_val(52),
            story_sequence_hold_speed: get_val(53),
            unknown_55: get_val(54),
            unknown_56: get_val(55),
            unknown_57: get_val(56),
            unknown_58: get_val(57),
            unknown_59: get_val(58),
            unknown_60: get_val(59),
            unknown_61: get_val(60),
            unknown_62: get_val(61),
            unknown_63: get_val(62),
            unknown_64: get_val(63),
            unknown_65: get_val(64),
            unknown_66: get_val(65),
            unknown_67: get_val(66),
            unknown_68: get_val(67),
            unknown_69: get_val(68),
            unknown_70: get_val(69),
            unknown_71: get_val(70),
            unknown_72: get_val(71),
            unknown_73: get_val(72),
            unknown_74: get_val(73),
            unknown_75: get_val(74),
            unknown_76: get_val(75),
            unknown_77: get_val(76),
            unknown_78: get_val(77),
            comeback_treasure_festival_days: get_val(78),
            unknown_80: get_val(79),
            unknown_81: get_val(80),
            rest: rest_vector,
        }
    }
}

pub fn load_param(data_directory: &Path, priority: &[String]) -> Option<Param> {
    let file_path = crate::global::resolver::get(data_directory, &["param.tsv"], priority).into_iter().next()?;
    
    let file_content = fs::read_to_string(&file_path).ok()?;
    Some(Param::from_tsv(&file_content))
}