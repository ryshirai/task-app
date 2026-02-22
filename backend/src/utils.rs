pub fn is_valid_username(username: &str) -> bool {
    username
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

pub fn is_secure_password(password: &str) -> bool {
    if password.len() < 8 {
        return false;
    }

    let mut has_upper = false;
    let mut has_lower = false;
    let mut has_digit = false;
    let mut has_symbol = false;

    for c in password.chars() {
        if c.is_ascii_uppercase() {
            has_upper = true;
        } else if c.is_ascii_lowercase() {
            has_lower = true;
        } else if c.is_ascii_digit() {
            has_digit = true;
        } else if c.is_ascii_punctuation() {
            has_symbol = true;
        }
    }

    has_upper && has_lower && has_digit && has_symbol
}

#[cfg(test)]
mod tests {
    use super::{is_secure_password, is_valid_username};

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

    #[test]
    fn accepts_secure_password() {
        assert!(is_secure_password("Abcd1234!"));
    }

    #[test]
    fn rejects_password_shorter_than_8() {
        assert!(!is_secure_password("Ab1!xyz"));
    }

    #[test]
    fn rejects_password_missing_required_character_types() {
        let invalid_passwords = [
            "abcd1234!",
            "ABCD1234!",
            "Abcdefg!",
            "Abcd1234",
            "Ａbcd1234!",
        ];

        for password in invalid_passwords {
            assert!(!is_secure_password(password), "expected invalid: {password}");
        }
    }
}
