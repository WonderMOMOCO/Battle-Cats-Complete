pub fn is_auditable_text(filename: &str) -> bool {
    let lower = filename.to_lowercase();
    lower.ends_with(".csv") || 
    lower.ends_with(".tsv") || 
    lower.ends_with(".mamodel") || 
    lower.ends_with(".maanim") || 
    lower.ends_with(".imgcut") ||
    lower.ends_with(".json") ||
    lower.ends_with(".list")
}

pub fn calculate_true_weight(data: &[u8], filename: &str) -> usize {
    if !is_auditable_text(filename) {
        return data.len();
    }
    
    let carriage_return_byte = b'\r';
    let carriage_return_count = data.iter().filter(|&&byte| byte == carriage_return_byte).count();
    
    data.len() - carriage_return_count
}

pub fn strip_carriage_returns(data: &[u8], filename: &str) -> Vec<u8> {
    if !is_auditable_text(filename) {
        return data.to_vec();
    }
    
    data.iter().copied().filter(|&byte| byte != b'\r').collect()
}