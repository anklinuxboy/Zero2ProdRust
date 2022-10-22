use std::fmt::format;
use std::net::TcpListener;
use zero2prod::startup::run;
use sqlx::{PgConnection, Connection};
use zero2prod::configuration::get_configuration;

#[tokio::test]
async fn health_check_works() {
    spawn_app();
    let client = reqwest::Client::new();
    let address = spawn_app();

    let response = client
        .get(&format!("{}/health_check", &address))
        .send()
        .await
        .expect("Failed to execute requests");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0")
        .expect("Failed to bind to a random port");
    let port = listener.local_addr().unwrap().port();
    let server = run(listener)
        .expect("Failed to bind address");

    let _ = tokio::spawn(server);

    format!("http://127.0.0.1:{}", port)
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let app_address = spawn_app();
    let config = get_configuration().expect("failed to read config");
    let conn_string = config.database.connection_string();

    let mut db_conn = PgConnection::connect(&conn_string)
        .await
        .expect("Failed to connect to postgres");
    let client = reqwest::Client::new();

    let body = "name=ankit%20sharma&email=email%40gmail.com";
    let response = client
        .post(&format!("{}/subscriptions", app_address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions",)
        .fetch_one(&mut db_conn)
        .await
        .expect("Failed to fetch saved subscription");

    assert_eq!(saved.email, "email@gmail.com");
    assert_eq!(saved.name, "ankit sharma");
}

#[tokio::test]
async fn subscribe_returns_400_when_data_missing() {
    let app_address = spawn_app();
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=ankit%20sharma", "missing email"),
        ("email=email%40gmail.com", "missing name"),
        ("", "missing both email and gmail"),
    ];

    for (body, error_message) in test_cases {
        let response = client
            .post(&format!("{}/subscriptions", app_address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("failed to execute");

        assert_eq!(
            400,
            response.status().as_u16(),
            "The api did failed with code 400 when payload was {}",
            error_message
        );
    }
}