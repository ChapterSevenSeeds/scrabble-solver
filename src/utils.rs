use std::collections::HashMap;

pub fn char_count_to_map(chars: &str) -> HashMap<char, usize> {
    let mut result = HashMap::new();
    for c in chars.chars() {
        *result.entry(c).or_insert(0) += 1;
    }
    
    result
}