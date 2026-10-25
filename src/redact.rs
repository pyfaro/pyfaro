//! Keep credentials out of the text Pyfaro prints.
//!
//! Pyfaro echoes arguments, configuration values and tool diagnostics in
//! verbose lines and error messages. Any URL in that text has the secret part
//! of its userinfo masked: `https://user:secret@host` becomes
//! `https://user:****@host` and a lone token `https://token@host` becomes
//! `https://****@host`. Text that is not a URL is left alone, and so is the
//! output of child processes that streams straight to the terminal.

const MASK: &str = "****";

/// Mask the userinfo secret of every `scheme://userinfo@host` in `input`,
/// including after `--flag=` and inside quotes. Idempotent.
pub(crate) fn text(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(index) = rest.find("://") {
        let (head, tail) = rest.split_at(index + 3);
        output.push_str(head);
        let scheme = head[..index].chars().next_back();
        if !scheme
            .is_some_and(|c| c.is_ascii_alphanumeric() || "+.-".contains(c))
        {
            rest = tail;
            continue;
        }
        let end = tail.find(ends_authority).unwrap_or(tail.len());
        let (authority, after) = tail.split_at(end);
        match authority.rfind('@') {
            Some(at) if at > 0 => {
                output.push_str(&mask(&authority[..at]));
                output.push_str(&authority[at..]);
            }
            _ => output.push_str(authority),
        }
        rest = after;
    }
    output.push_str(rest);
    output
}

/// The characters that end a URL authority, or that cannot be part of one
/// in the quoted, escaped or bracketed contexts Pyfaro prints.
fn ends_authority(character: char) -> bool {
    character.is_whitespace()
        || matches!(
            character,
            '/' | '?' | '#' | '"' | '\'' | '\\' | '<' | '>' | '`'
        )
}

fn mask(userinfo: &str) -> String {
    match userinfo.split_once(':') {
        Some((user, _)) => format!("{user}:{MASK}"),
        None => MASK.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::text;

    #[test]
    fn masks_the_password_and_keeps_the_user() {
        assert_eq!(
            text("https://user:secret@host/simple"),
            "https://user:****@host/simple"
        );
    }

    #[test]
    fn masks_a_user_only_token() {
        assert_eq!(text("https://token@host/x"), "https://****@host/x");
    }

    #[test]
    fn masks_flags_quotes_and_escapes() {
        assert_eq!(
            text("--index-url=https://u:p@host/simple --x"),
            "--index-url=https://u:****@host/simple --x"
        );
        assert_eq!(
            text("invalid \"pkg @ git+https://u:p@h/r.git\"; fix\\nnext"),
            "invalid \"pkg @ git+https://u:****@h/r.git\"; fix\\nnext"
        );
    }

    #[test]
    fn masks_every_url_and_handles_an_at_sign_in_the_password() {
        assert_eq!(
            text("a https://u:p@s@h/x b http://t@h2"),
            "a https://u:****@h/x b http://****@h2"
        );
    }

    #[test]
    fn leaves_other_text_alone() {
        for input in [
            "https://host/path@thing",
            "https://host:8080/x",
            "user@example.com",
            "no url here",
            "://@x",
            "",
        ] {
            assert_eq!(text(input), input);
        }
    }

    #[test]
    fn is_idempotent() {
        let once = text("https://user:secret@host https://tok@host");
        assert_eq!(text(&once), once);
    }
}
