use validator::ValidateEmail;

#[derive(Debug, Clone)]
pub struct SubscriberEmail(String);

impl SubscriberEmail {
    /// Parses a string into a valid `SubscriberEmail`.
    ///
    /// # Arguments
    /// * `s` - The string to be parsed as an email address
    ///
    /// # Returns
    /// * `Ok(SubscriberEmail)` - If the email is valid
    /// * `Err(String)` - If the email is invalid, with an error message
    pub fn parse(s: String) -> Result<SubscriberEmail, String> {
        if s.is_empty() {
            return Err("Email address cannot be empty".to_string());
        }

        if !s.validate_email() {
            return Err(format!("'{}' is not a valid email address", s));
        }

        Ok(Self(s))
    }

    /// Returns the email address as a string reference
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok};
    use fake::faker::internet::en::SafeEmail;
    use fake::Fake;
    use proptest::arbitrary::any;
    use proptest::proptest;
    use proptest::strategy::Strategy;

    /// Strategy to generate valid fake email addresses
    fn fake_email_strategy() -> impl Strategy<Value = String> {
        any::<u32>().prop_map(|_| SafeEmail().fake())
    }

    proptest!(
        #[test]
        fn valid_emails_are_parsed_successfully(email in fake_email_strategy()) {
            assert_ok!(SubscriberEmail::parse(email));
        }
    );

    #[test]
    fn empty_string_is_rejected() {
        let email = "".to_string();
        let result = SubscriberEmail::parse(email);
        assert_err!(&result);
        assert_eq!(
            result.unwrap_err(),
            "Email address cannot be empty".to_string()
        );
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
