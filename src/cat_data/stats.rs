use std::fs;
use std::path::Path;

pub fn load_from_id(id: i32) -> Option<CatRaw> {
    let path = format!("game/cats/{:03}/unit{:03}.csv", id, id + 1);
    let p = Path::new(&path);
    
    if p.exists() {
        if let Ok(content) = fs::read_to_string(p) {
            if let Some(first_line) = content.lines().next() {
                return CatRaw::from_csv_line(first_line);
            }
        }
    }
    None
}

pub const ICON_SIZE: f32 = 40.0;

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct CatRaw {
    pub hitpoints: i32,
    pub knockbacks: i32,
    pub speed: i32,
    pub attack_1: i32,
    pub time_before_attack_1: i32,
    pub standing_range: i32,
    pub eoc1_cost: i32,
    pub cooldown: i32, 
    pub hitbox_position: i32,
    pub hitbox_width: i32,
    pub target_red: i32,
    pub unknown_11: i32,
    pub area_attack: i32,
    pub pre_attack_animation: i32,
    pub minimum_z_layer: i32,
    pub maximum_z_layer: i32,
    pub target_floating: i32,
    pub target_black: i32,
    pub target_metal: i32,
    pub target_traitless: i32,
    pub target_angel: i32,
    pub target_alien: i32,
    pub target_zombie: i32,
    pub strong_against: i32,
    pub knockback_chance: i32,
    pub freeze_chance: i32,
    pub freeze_duration: i32,
    pub slow_chance: i32,
    pub slow_duration: i32,
    pub resist: i32,
    pub massive_damage: i32,
    pub critical_chance: i32,
    pub attack_only: i32,
    pub double_bounty: i32,
    pub base_destroyer: i32,
    pub wave_chance: i32,
    pub wave_level: i32,
    pub weaken_chance: i32,
    pub weaken_duration: i32,
    pub weaken_to: i32,
    pub strengthen_threshold: i32,
    pub strengthen_boost: i32,
    pub survive: i32,
    pub metal: i32,
    pub long_distance_1_anchor: i32,
    pub long_distance_1_span: i32,
    pub wave_immune: i32,
    pub wave_block: i32,
    pub knockback_immune: i32,
    pub freeze_immune: i32,
    pub slow_immune: i32,
    pub weaken_immune: i32,
    pub zombie_killer: i32,
    pub witch_killer: i32,
    pub target_witch: i32,
    pub unknown_55: i32,        
    pub boss_wave_immune: i32,  
    pub unknown_57: i32,        
    pub kamikaze: i32,          
    pub attack_2: i32,
    pub attack_3: i32,
    pub time_before_attack_2: i32,
    pub time_before_attack_3: i32,
    pub attack_1_abilities: i32,
    pub attack_2_abilities: i32,
    pub attack_3_abilities: i32,
    pub unknown_66: i32,        
    pub soul_animation: i32,
    pub spawn_animation: i32,
    pub unknown_69: i32,
    pub barrier_breaker_chance: i32,
    pub warp_chance: i32,
    pub warp_duration: i32,
    pub warp_distance_minimum: i32,
    pub warp_distance_maximum: i32,
    pub warp_immune: i32,
    pub target_eva: i32,
    pub eva_killer: i32,
    pub target_relic: i32,
    pub curse_immune: i32,
    pub insanely_tough: i32,
    pub insane_damage: i32,
    pub savage_blow_chance: i32,
    pub savage_blow_boost: i32,
    pub dodge_chance: i32,
    pub dodge_duration: i32,
    pub surge_chance: i32,
    pub surge_spawn_anchor: i32, 
    pub surge_spawn_span: i32,   
    pub surge_level: i32,
    pub toxic_immune: i32,
    pub surge_immune: i32,
    pub curse_chance: i32,
    pub curse_duration: i32,
    pub mini_wave_flag: i32,
    pub shield_pierce_chance: i32,
    pub target_aku: i32,
    pub colossus_slayer: i32,
    pub soulstrike: i32,
    pub long_distance_2_flag: i32,
    pub long_distance_2_anchor: i32,
    pub long_distance_2_span: i32,
    pub long_distance_3_flag: i32,
    pub long_distance_3_anchor: i32,
    pub long_distance_3_span: i32,
    pub behemoth_slayer: i32,
    pub behemoth_dodge_chance: i32,
    pub behemoth_dodge_duration: i32,
    pub mini_surge_flag: i32,
    pub counter_surge: i32,
    pub conjure_unit_id: i32,
    pub sage_slayer: i32,
    pub metal_killer_percent: i32,
    pub explosion_chance: i32,
    pub explosion_spawn_anchor: i32,     
    pub explosion_spawn_span: i32, 
    pub explosion_immune: i32,
}

impl CatRaw {
    pub fn from_csv_line(line: &str) -> Option<Self> {
        let parts: Vec<&str> = line.split(',').collect();
        let get = |index: usize| parts.get(index).and_then(|v| v.trim().parse::<i32>().ok()).unwrap_or(0);
        let get_neg = |index: usize| parts.get(index).and_then(|v| v.trim().parse::<i32>().ok()).unwrap_or(-1);

        if parts.len() < 10 { return None; }

        Some(Self {
            hitpoints: get(0),
            knockbacks: get(1),
            speed: get(2),
            attack_1: get(3),
            time_before_attack_1: get(4) * 2,
            standing_range: get(5),
            eoc1_cost: get(6),
            cooldown: get(7) * 2,
            hitbox_position: get(8),
            hitbox_width: get(9),
            target_red: get(10),
            unknown_11: get(11),
            area_attack: get(12),
            pre_attack_animation: get(13),
            minimum_z_layer: get(14),
            maximum_z_layer: get(15),
            target_floating: get(16),
            target_black: get(17),
            target_metal: get(18),
            target_traitless: get(19),
            target_angel: get(20),
            target_alien: get(21),
            target_zombie: get(22),
            strong_against: get(23),
            knockback_chance: get(24),
            freeze_chance: get(25),
            freeze_duration: get(26),
            slow_chance: get(27),
            slow_duration: get(28),
            resist: get(29),
            massive_damage: get(30),
            critical_chance: get(31),
            attack_only: get(32),
            double_bounty: get(33),
            base_destroyer: get(34),
            wave_chance: get(35),
            wave_level: get(36),
            weaken_chance: get(37),
            weaken_duration: get(38),
            weaken_to: get(39),
            strengthen_threshold: get(40),
            strengthen_boost: get(41),
            survive: get(42),
            metal: get(43),
            long_distance_1_anchor: get(44),
            long_distance_1_span: get(45),
            wave_immune: get(46),
            wave_block: get(47),
            knockback_immune: get(48),
            freeze_immune: get(49),
            slow_immune: get(50),
            weaken_immune: get(51),
            zombie_killer: get(52),
            witch_killer: get(53),
            target_witch: get(54),
            unknown_55: get_neg(55),
            boss_wave_immune: get_neg(56),
            unknown_57: get_neg(57),
            kamikaze: get(58),
            attack_2: get(59),
            attack_3: get(60),
            time_before_attack_2: get(61),
            time_before_attack_3: get(62),
            attack_1_abilities: get(63),
            attack_2_abilities: get(64),
            attack_3_abilities: get(65),
            unknown_66: get_neg(66),
            soul_animation: get(67),
            spawn_animation: get(68),
            unknown_69: get(69),
            barrier_breaker_chance: get(70),
            warp_chance: get(71),
            warp_duration: get(72),
            warp_distance_minimum: get(73),
            warp_distance_maximum: get(74),
            warp_immune: get(75),
            target_eva: get(76),
            eva_killer: get(77),
            target_relic: get(78),
            curse_immune: get(79),
            insanely_tough: get(80),
            insane_damage: get(81),
            savage_blow_chance: get(82),
            savage_blow_boost: get(83),
            dodge_chance: get(84),
            dodge_duration: get(85),
            surge_chance: get(86),
            surge_spawn_anchor: get(87) / 4,
            surge_spawn_span: get(88) / 4,
            surge_level: get(89),
            toxic_immune: get(90),
            surge_immune: get(91),
            curse_chance: get(92),
            curse_duration: get(93),
            mini_wave_flag: get(94),
            shield_pierce_chance: get(95),
            target_aku: get(96),
            colossus_slayer: get(97),
            soulstrike: get(98),
            long_distance_2_flag: get(99),
            long_distance_2_anchor: get(100),
            long_distance_2_span: get(101),
            long_distance_3_flag: get(102),
            long_distance_3_anchor: get(103),
            long_distance_3_span: get(104),
            behemoth_slayer: get(105),
            behemoth_dodge_chance: get(106),
            behemoth_dodge_duration: get(107),
            mini_surge_flag: get(108),
            counter_surge: get(109),
            conjure_unit_id: get(110),
            sage_slayer: get(111),
            metal_killer_percent: get(112),
            explosion_chance: get(113),
            explosion_spawn_anchor: get(114) / 4,
            explosion_spawn_span: get(115) / 4,
            explosion_immune: get(116),
        })
    }
    
    pub fn attack_cycle(&self, anim_frames: i32) -> i32 {
        let mut effective_foreswing = self.pre_attack_animation;
        
        if self.attack_3 > 0 && self.time_before_attack_3 > 0 {
            effective_foreswing = self.time_before_attack_3;
        } 
        else if self.attack_2 > 0 && self.time_before_attack_2 > 0 {
            effective_foreswing = self.time_before_attack_2;
        }

        let cooldown = self.time_before_attack_1.saturating_sub(1);
        
        (effective_foreswing + cooldown).max(anim_frames)
    }

    pub fn effective_cooldown(&self) -> i32 {
        (self.cooldown - 264).max(60)
    }
}

#[derive(Clone, Debug, Default)]
pub struct CatLevelCurve {
    pub increments: Vec<u16>, 
}

impl CatLevelCurve {
    pub fn from_csv_line(line: &str) -> Self {
        let parts: Vec<&str> = line.split(',').collect();
        let mut increments = Vec::new();
        for part in parts {
            if let Ok(val) = part.trim().parse::<u16>() {
                increments.push(val);
            }
        }
        Self { increments }
    }

    pub fn calculate_stat(&self, base: i32, level: i32) -> i32 {
        let base_f = base as f64;
        let mut stat = base_f;

        let max_scaled_level = (self.increments.len() * 10) as i32;
        let limit = std::cmp::min(level, max_scaled_level);

        for l in 2..=limit {
            let index = ((l as f64 / 10.0).ceil() as usize).saturating_sub(1);
            if let Some(&scaling) = self.increments.get(index) {
                stat += base_f * (scaling as f64) / 100.0;
            }
        }

        if level > max_scaled_level {
            let to_apply = level - max_scaled_level;
            if let Some(&last_scaling) = self.increments.last() {
                stat += base_f * (last_scaling as f64) * (to_apply as f64) / 100.0;
            }
        }

        let rounded_stat = stat.round();
        let final_val = (rounded_stat * 2.5).floor();
        final_val as i32
    }
}