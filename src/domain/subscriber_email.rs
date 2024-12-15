use validator::ValidateEmail;

use crate::http::{ApiError, Result};

#[derive(Debug)]
pub struct SubscriberEmail(String);

impl SubscriberEmail {
    /// Parse a string into a valid `SubscriberEmail`.
    pub fn parse(s: String) -> Result<SubscriberEmail> {
        if s.validate_email() {
            Ok(Self(s))
        } else {
            Err(ApiError::InvalidValue(
                "invalid subscriber email".to_string(),
            ))
        }
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_err;

    #[test]
    fn empty_string_is_rejected() {
        let email = "".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "ursula.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_domain_is_rejected() {
        let email = "ursula@".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_username_is_rejected() {
        let email = "@gmail.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }
}
