use crate::domain::SubscriberEmail;
use reqwest::Client;
use secrecy::{ExposeSecret, Secret};

#[derive(Clone)]
pub struct EmailClient {
    http_client: Client,
    base_url: String,
    sender: SubscriberEmail,
    authorization_token: Secret<String>,
}

impl EmailClient {
    pub fn new(
        base_url: String,
        sender: SubscriberEmail,
        authorization_token: Secret<String>,
    ) -> Self {
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap();
        Self {
            http_client,
            base_url,
            sender,
            authorization_token,
        }
    }

    pub async fn send_email(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_content: &str,
        _text_content: &str,
    ) -> Result<(), reqwest::Error> {
        let url = format!("{}/mail/send", self.base_url);
        let text_type = if html_content.is_empty() {
            "text/plain"
        } else {
            "text/html"
        };
        let request_body = SendEmailRequest {
            personalization: vec![SendEmailPersonalization {
                to: vec![SendEmailKey {
                    email: recipient.as_ref(),
                }],
            }],
            from: SendEmailKey {
                email: self.sender.as_ref(),
            },
            subject,
            content: vec![SendEmailContent {
                type_: text_type,
                value: html_content,
            }],
        };

        self.http_client
            .post(&url)
            .header(
                "Authorization",
                "Bearer ".to_owned() + self.authorization_token.expose_secret(),
            )
            .json(&request_body)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}

#[derive(serde::Serialize)]
struct SendEmailRequest<'a> {
    personalization: Vec<SendEmailPersonalization<'a>>,
    from: SendEmailKey<'a>,
    subject: &'a str,
    content: Vec<SendEmailContent<'a>>,
}

#[derive(serde::Serialize)]
struct SendEmailPersonalization<'a> {
    to: Vec<SendEmailKey<'a>>,
}

#[derive(serde::Serialize)]
struct SendEmailKey<'a> {
    email: &'a str,
}

#[derive(serde::Serialize)]
struct SendEmailContent<'a> {
    #[serde(rename = "type")]
    type_: &'a str,
    value: &'a str,
}

#[cfg(test)]
mod tests {
    use crate::domain::SubscriberEmail;
    use crate::email_client::EmailClient;
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::{Fake, Faker};
    use secrecy::Secret;
    use claim::{assert_err, assert_ok};
    use wiremock::matchers::{any, header, header_exists, method, path};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    struct SendEmailBodyMatcher;

    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &Request) -> bool {
            let result: Result<serde_json::Value, _> = serde_json::from_slice(&request.body);
            if let Ok(body) = result {
                dbg!(&body);
                body.get("personalization").is_some()
                    && body.get("from").is_some()
                    && body.get("subject").is_some()
                    && body.get("content").is_some()
            } else {
                false
            }
        }
    }

    fn subject() -> String {
        Sentence(1..2).fake()
    }

    fn content() -> String {
        Paragraph(1..10).fake()
    }

    fn email() -> SubscriberEmail {
        SubscriberEmail::parse(SafeEmail().fake()).unwrap()
    }

    fn email_client(base_url: String) -> EmailClient {
        EmailClient::new(base_url, email(), Secret::new(Faker.fake()))
    }

    #[tokio::test]
    async fn send_email_sends_the_expected_request() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(header_exists("Authorization"))
            .and(header("Content-Type", "application/json"))
            .and(path("/mail/send"))
            .and(method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let _ = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;
    }

    #[tokio::test]
    async fn send_email_succeeds_if_the_server_returns_200() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;

        assert_ok!(outcome);
    }

    #[tokio::test]
    async fn send_email_fails_if_the_server_returns_500() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;

        assert_err!(outcome);
    }

    #[tokio::test]
    async fn send_email_times_out_if_the_server_takes_too_long() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        let response = ResponseTemplate::new(200)
            .set_delay(std::time::Duration::from_secs(180));
        Mock::given(any())
            .respond_with(response)
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;

        assert_err!(outcome);
    }
}
