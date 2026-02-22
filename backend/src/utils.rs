pub fn is_valid_username(username: &str) -> bool {
    username
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

#[cfg(test)]
mod tests {
    use super::is_valid_username;

    #[test]
    fn accepts_ascii_alphanumeric_and_allowed_symbols() {
        let valid_usernames = [
            "alice",
            "Alice123",
            "bob_smith",
            "charlie-01",
            "A_B-C_123",
        ];

        for username in valid_usernames {
            assert!(is_valid_username(username), "expected valid: {username}");
        }
    }

    #[test]
    fn rejects_disallowed_symbols() {
        let invalid_usernames = [
            "john.doe",
            "john doe",
            "john@doe",
            "john/doe",
            "john+doe",
            "name!",
        ];

        for username in invalid_usernames {
            assert!(!is_valid_username(username), "expected invalid: {username}");
        }
    }

    #[test]
    fn rejects_non_ascii_text_like_japanese() {
        let invalid_usernames = ["山田太郎", "ユーザー123", "たろう_test"];

        for username in invalid_usernames {
            assert!(!is_valid_username(username), "expected invalid: {username}");
        }
    }

    #[test]
    fn treats_empty_string_as_valid_current_behavior() {
        assert!(is_valid_username(""));
    }
}
