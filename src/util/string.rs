pub fn snake_case(name: &str) -> String {
    name.chars()
        .map(|c| if c == ' ' || c == '-' { '_' } else { c })
        .collect()
}

pub fn eq_no_case(s1: &str, s2: &str) -> bool {
    s1.to_lowercase() == s2.to_lowercase()
}

pub fn ellipsis(s: &str, n: usize) -> String {
    s.chars().take(n).collect::<String>()
}