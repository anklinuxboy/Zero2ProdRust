use crate::helpers::spawn_app;
use reqwest::Url;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn confirmations_without_token_are_rejected_with_a_400() {
    let app = spawn_app().await;

    let response = reqwest::get(&format!("{}/subscriptions/confirm", app.address))
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 400);
}

#[tokio::test]
async fn link_returned_by_subscribe_returns_200_if_called() {
    let app = spawn_app().await;
    let body = "name=le%20mans&email=test%40gmail.com";

    Mock::given(path("/mail/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();
    let confirmation_link = app.get_confirmation_links(&email_request);

    let response = reqwest::get(confirmation_link.html).await.unwrap();

    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn clicking_on_confirmation_link_confirms_subscriber() {
    let app = spawn_app().await;
    let body = "name=le%20mans&email=test%40gmail.com";

    Mock::given(path("/mail/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();
    let confirmation_link = app.get_confirmation_links(&email_request);

    reqwest::get(confirmation_link.html).await.unwrap();

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription");

    assert_eq!(saved.email, "test@gmail.com");
    assert_eq!(saved.name, "le mans");
    assert_eq!(saved.status, "confirmed");
}
