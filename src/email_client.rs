use std::time::Duration;

use crate::domain::SubscriberEmail;
use reqwest::{Client, Url};
use secrecy::{ExposeSecret, SecretString};

#[derive(Clone)]
pub struct EmailClient {
    http_client: Client,
    base_url: Url,
    sender: SubscriberEmail,
    authorization_token: SecretString,
}

impl EmailClient {
    /// Creates a new email client
    ///
    /// # Arguments
    /// * `base_url` - Base URL for the email service API
    /// * `sender` - Email address of the sender
    /// * `authorization_token` - Authorization token for the email service
    /// * `timeout` - Timeout duration for API requests
    ///
    /// # Returns
    /// A new EmailClient instance
    ///
    /// # Panics
    /// Panics if the base URL is invalid or if HTTP client creation fails
    pub fn new(
        base_url: &str,
        sender: SubscriberEmail,
        authorization_token: SecretString,
        timeout: Duration,
    ) -> Self {
        let base_url = Url::parse(base_url).expect("Invalid base URL provided to EmailClient");
        let http_client = Client::builder()
            .timeout(timeout)
            .build()
            .expect("Failed to build HTTP client");
        Self {
            http_client,
            base_url,
            sender,
            authorization_token,
        }
    }

    /// Sends an email to a recipient
    ///
    /// # Arguments
    /// * `recipient` - Email address of the recipient
    /// * `subject` - Email subject
    /// * `html_content` - HTML formatted content of the email
    /// * `text_content` - Plain text content of the email
    ///
    /// # Returns
    /// Ok(()) if email was sent successfully, Err otherwise
    pub async fn send_email(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), reqwest::Error> {
        let url = self
            .base_url
            .join("/email")
            .expect("Invalid email client base URL");

        let request_body = SendEmailRequest {
            from: self.sender.as_ref(),
            to: recipient.as_ref(),
            subject,
            html_body: html_content,
            text_body: text_content,
        };

        self.http_client
            .post(url)
            .header(
                "X-Postmark-Server-Token",
                self.authorization_token.expose_secret(),
            )
            .json(&request_body)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    html_body: &'a str,
    text_body: &'a str,
}

#[cfg(test)]
mod tests {
    use crate::domain::SubscriberEmail;
    use crate::email_client::EmailClient;
    use claims::{assert_err, assert_ok};
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::{Fake, Faker};
    use secrecy::SecretString;
    use wiremock::matchers::{any, header, header_exists, method, path};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    struct SendEmailBodyMatcher;

    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &Request) -> bool {
            // Try to parse request body as JSON
            let result: Result<serde_json::Value, _> = serde_json::from_slice(&request.body);

            if let Ok(body) = result {
                // Verify all required fields are present
                body.get("From").is_some()
                    && body.get("To").is_some()
                    && body.get("Subject").is_some()
                    && body.get("HtmlBody").is_some()
                    && body.get("TextBody").is_some()
            } else {
                false
            }
        }
    }

    /// Returns a random email subject
    fn subject() -> String {
        Sentence(1..2).fake()
    }

    /// Returns a random email content
    fn content() -> String {
        Paragraph(1..10).fake()
    }

    /// Returns a random valid email address
    fn email() -> SubscriberEmail {
        SubscriberEmail::parse(SafeEmail().fake()).unwrap()
    }

    /// Creates an EmailClient instance for testing
    fn email_client(base_url: &str) -> EmailClient {
        let authorization_token = SecretString::from(Faker.fake::<String>());
        let timeout = std::time::Duration::from_millis(200);
        EmailClient::new(base_url, email(), authorization_token, timeout)
    }

    #[tokio::test]
    async fn send_email_sends_the_expected_request() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client = email_client(&mock_server.uri());

        Mock::given(header_exists("X-Postmark-Server-Token"))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        // execute
        let _ = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;

        // assert
    }

    #[tokio::test]
    async fn send_email_succeeds_if_the_server_returns_200() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client = email_client(&mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let outcome = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;

        // Assert
        assert_ok!(outcome);
    }

    #[tokio::test]
    async fn send_email_fails_if_the_server_returns_500() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client = email_client(&mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let outcome = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;

        // Assert
        assert_err!(outcome);
    }

    #[tokio::test]
    async fn send_email_times_out_if_the_server_takes_too_long() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client = email_client(&mock_server.uri());

        let response = ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(90));

        Mock::given(any())
            .respond_with(response)
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let outcome = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;

        // Assert
        assert_err!(outcome);
    }
}
