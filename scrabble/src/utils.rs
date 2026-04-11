use std::collections::HashMap;

/// Creates a map that maps the chars from a string to how many times each char occurs.
pub fn char_count_to_map(chars: &str) -> HashMap<char, usize> {
    let mut result = HashMap::new();
    for c in chars.chars() {
        *result.entry(c).or_insert(0) += 1;
    }

    result
}

/// Encode a character as a bitmask. 'A' is 1, 'B' is 2, 'C' is 4, etc.
pub fn encode_char(c: char) -> u32 {
    1 << (c as u32 - 'A' as u32)
}

/// Encodes a set of chars into a single bitmask.
pub fn encode_chars(chars: &str) -> u32 {
    chars.chars().fold(0, |acc, c| acc | encode_char(c))
}

pub fn convert_chars_to_bit_vec(word: &str) -> Vec<u32> {
    word.chars().map(|c| encode_char(c)).collect()
}

/// Checks if a word matches a given char bitmask.
pub fn word_matches_bitmask(word: &String, mask: &Vec<u32>) -> bool {
    if mask.len() != word.len() {
        return false;
    }

    for i in 0..mask.len() {
        if encode_char(word.as_bytes()[i] as char) & mask[i] == 0 {
            return false;
        }
    }

    true
}

pub fn bitmasks_match(left: &Vec<u32>, right: &Vec<u32>) -> bool {
    if left.len() != right.len() {
        return false;
    }

    for i in 0..left.len() {
        if left[i] & right[i] == 0 {
            return false;
        }
    }

    true
}
