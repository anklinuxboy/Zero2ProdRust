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
        Self {
            http_client: Client::new(),
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
        text_content: &str,
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
                    email: recipient.as_ref().to_owned(),
                }],
            }],
            from: SendEmailKey {
                email: self.sender.as_ref().to_owned(),
            },
            subject: subject.to_owned(),
            content: vec![SendEmailContent {
                type_: text_type.to_owned(),
                value: html_content.to_owned(),
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
            .await?;
        Ok(())
    }
}

#[derive(serde::Serialize)]
struct SendEmailRequest {
    personalization: Vec<SendEmailPersonalization>,
    from: SendEmailKey,
    subject: String,
    content: Vec<SendEmailContent>,
}

#[derive(serde::Serialize)]
struct SendEmailPersonalization {
    to: Vec<SendEmailKey>,
}

#[derive(serde::Serialize)]
struct SendEmailKey {
    email: String,
}

#[derive(serde::Serialize)]
struct SendEmailContent {
    #[serde(rename = "type")]
    type_: String,
    value: String,
}

#[cfg(test)]
mod tests {
    use crate::domain::SubscriberEmail;
    use crate::email_client::EmailClient;
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::{Fake, Faker};
    use secrecy::Secret;
    use wiremock::matchers::any;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn send_email_fires_a_request_to_base_url() {
        let mock_server = MockServer::start().await;
        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let email_client = EmailClient::new(mock_server.uri(), sender, Secret::new(Faker.fake()));

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let subscriber_email = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let subject: String = Sentence(1..2).fake();
        let content: String = Paragraph(1..10).fake();

        let _ = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;
    }
}
