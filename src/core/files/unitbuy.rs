use std::fs;
use std::path::Path;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct UnitBuyRow {
    pub guide_order: i32,
    pub egg_id_normal: i32, 
    pub egg_id_evolved: i32, 
}

impl UnitBuyRow {
    pub fn from_csv_line(csv_line: &str) -> Option<Self> {
        let line_parts: Vec<&str> = csv_line.split(',').map(|part| part.trim()).collect();
        
        // Filter out empty trailing parts often found in these CSVs
        let clean_parts: Vec<&str> = line_parts.iter()
            .rev()
            .skip_while(|part| part.is_empty())
            .cloned() 
            .collect();
        
        let true_length = clean_parts.len();
        if true_length < 28 { return None; }

        let get_raw_value = |index: usize| line_parts.get(index).and_then(|val| val.parse::<i32>().ok()).unwrap_or(-1);

        let egg_evolved_str = clean_parts.get(0).unwrap_or(&"-1");
        let egg_normal_str = clean_parts.get(1).unwrap_or(&"-1");

        Some(Self {
            guide_order: get_raw_value(27),
            egg_id_normal: egg_normal_str.parse::<i32>().unwrap_or(-1),
            egg_id_evolved: egg_evolved_str.parse::<i32>().unwrap_or(-1),
        })
    }
}

pub fn load_unitbuy(cats_directory: &Path) -> HashMap<u32, UnitBuyRow> {
    let mut unit_buy_map = HashMap::new();
    let file_path = cats_directory.join("unitbuy.csv");
    
    if let Ok(file_content) = fs::read_to_string(&file_path) {
        for (line_index, csv_line) in file_content.lines().enumerate() {
            if let Some(row_data) = UnitBuyRow::from_csv_line(csv_line) {
                unit_buy_map.insert(line_index as u32, row_data);
            }
        }
    } 
    unit_buy_map
}