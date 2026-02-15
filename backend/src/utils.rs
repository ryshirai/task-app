pub fn is_valid_username(username: &str) -> bool {
    username.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}
