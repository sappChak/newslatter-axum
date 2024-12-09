use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sqlx::PgPool;
use tower::util::ServiceExt;

use crate::helpers::spawn_test_app;

#[sqlx::test]
async fn subscribe_returs_200_for_valid_form_data(pool: PgPool) {
    let app = spawn_test_app(pool).await.unwrap();

    let form_data = "name=Andrii%20Konotop&email=aws.test.receiver@gmail.com";

    let response = app
        .router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/subscriptions")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(Body::from(form_data))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[sqlx::test]
async fn subscribe_persists_the_new_subscriber(pool: PgPool) {
    let app = spawn_test_app(pool).await.unwrap();

    let form_data = "name=Andrii%20Konotop&email=aws.test.receiver@gmail.com";

    let _response = app
        .router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/subscriptions")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(Body::from(form_data))
                .unwrap(),
        )
        .await
        .unwrap();

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions",)
        .fetch_one(&app.db_state.pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.name, "Andrii Konotop");
    assert_eq!(saved.email, "aws.test.receiver@gmail.com");
    assert_eq!(saved.status, "pending_confirmation");
}

#[sqlx::test]
async fn subscribe_returs_422_for_data_is_missing(pool: PgPool) {
    let app = spawn_test_app(pool).await.unwrap();

    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (form_data, error_message) in test_cases {
        let response = app
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/subscriptions")
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .body(Body::from(form_data))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "The API did not fail with 422 UNPROCESSABLE_ENTITY when the payload was {}.",
            error_message
        );
    }
}

#[sqlx::test]
async fn subscribe_returs_400_when_fields_are_present_but_invalid(pool: PgPool) {
    let app = spawn_test_app(pool).await.unwrap();

    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];

    for (form_data, error_message) in test_cases {
        let response = app
            .router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/subscriptions")
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .body(Body::from(form_data))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "The API did not fail with 400 BAD_REQUEST when the payload was {}.",
            error_message
        );
    }
}
