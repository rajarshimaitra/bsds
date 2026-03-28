use rand::Rng;

/// Character set for temporary passwords — excludes visually ambiguous chars
/// (O/0, I/l/1) to reduce user confusion. Matches the legacy TS generator.
const TEMP_PASSWORD_CHARS: &[u8] =
    b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghjkmnpqrstuvwxyz23456789";

/// Generate a random temporary password of the given length (default 8).
pub fn generate_temp_password(length: usize) -> String {
    let mut rng = rand::thread_rng();
    (0..length)
        .map(|_| {
            let idx = rng.gen_range(0..TEMP_PASSWORD_CHARS.len());
            TEMP_PASSWORD_CHARS[idx] as char
        })
        .collect()
}

/// Generate a temporary password with the default length of 8 characters.
pub fn generate_temp_password_default() -> String {
    generate_temp_password(8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_length_is_eight() {
        let pw = generate_temp_password_default();
        assert_eq!(pw.len(), 8);
    }

    #[test]
    fn custom_length() {
        let pw = generate_temp_password(16);
        assert_eq!(pw.len(), 16);
    }

    #[test]
    fn only_allowed_chars() {
        let charset: Vec<char> =
            String::from_utf8_lossy(TEMP_PASSWORD_CHARS).chars().collect();
        for _ in 0..100 {
            let pw = generate_temp_password(8);
            for ch in pw.chars() {
                assert!(charset.contains(&ch), "unexpected char: {ch}");
            }
        }
    }
}
