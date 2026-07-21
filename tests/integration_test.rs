use std::sync::Once;

use axum_test::{TestServer, TestResponse, expect_json};
use axum_test::http::StatusCode;
use serde_json::json;
use jsonwebtoken::Algorithm;

use laravel_passport_introspection::{
    app::{create_app, setup_logging},
    config::Config,
    database::AnyAccessTokenRepository,
    database::fake::FakeAccessTokenRepository,
    jwt::{init_crypto, JWTClaims},
};

mod helpers;
use helpers::{AuthorizationServer, FIXTURES, FakeRepoExt};

static INIT: Once = Once::new();

fn assert_ok_and_has_claims(response: TestResponse, claims: JWTClaims) {
    response.assert_status_ok();
    response.assert_json(&json!({
        "active": true,
        "scope": "openid",
        "scopes": claims.scopes,
        "token_type": "Bearer",
        "exp": expect_json::float(),
        "iat": expect_json::float(),
        "nbf": expect_json::float(),
        "sub": claims.sub,
        "aud": claims.aud,
        "jti": claims.jti,
    }));
}

fn assert_ok_and_inactive(response: TestResponse) {
    response.assert_status_ok();
    response.assert_json(&json!({
        "active": false,
    }));
}

async fn setup_test_server() -> (String, AuthorizationServer, FakeAccessTokenRepository, TestServer) {
    let fixtures = &FIXTURES;
    let jwt_public_key = &fixtures.jwt_public_key;
    let jwt_private_key = &fixtures.jwt_private_key;

    INIT.call_once(|| {
        let _ = setup_logging(module_path!());

        let alg = Algorithm::RS256;
        init_crypto(&jwt_public_key, alg, &None).expect("Failed to initialize crypto");
    });

    let config = Config {
        database_url: "fake".to_string(),
        database_min_connections: 10,
        database_max_connections: 100,
        gateway_secret: "test-secret-32-characters-long!!!".to_string(),
        jwt_public_key: jwt_public_key.to_string(),
        jwt_algorithm: "RS256".to_string(),
        server_port: 8080,
        server_host: "127.0.0.1".to_string(),
        client_id: None,
        token_cache_size: 2,
        token_cache_ttl: 0,
    };

    let auth_server = AuthorizationServer::new(&jwt_private_key);

    let fake = FakeAccessTokenRepository::new(
        &config.database_url, config.database_min_connections, config.database_max_connections,
    ).await.unwrap();

    // Both `fake` clones share the same data because they point to the same data wrapped in Arc
    let app = create_app(config.clone(), AnyAccessTokenRepository::Fake(fake.clone())).await;
    let app_server = TestServer::new(app);

    (config.gateway_secret, auth_server, fake, app_server)
}

#[tokio::test]
async fn test_valid_token_with_aud_in_header() {
    let client_id = "019ed4fa-9bc6-73ef-ae5a-9b63ae58ca75";
    let (gateway_secret, auth_server, fake_repo, app_server) = setup_test_server().await;

    let (token, claims) = auth_server.generate_user_access_token(&client_id);
    let _ = fake_repo.add_token_from_claims(&claims, false).await;

    let response = app_server
        .post("/introspect-json")
        .add_header("X-Gateway-Secret", &gateway_secret)
        .add_header("X-Client-Id", client_id.to_string())
        .json(&json!({ "token": token }))
        .await;
    assert_ok_and_has_claims(response, claims)
}

#[tokio::test]
async fn test_skips_aud_validation_without_config_and_header() {
    let client_id = "019ed4fa-9bc6-73ef-ae5a-9b63ae58ca75";
    let (gateway_secret, auth_server, fake_repo, app_server) = setup_test_server().await;

    let (token, claims) = auth_server.generate_user_access_token(&client_id);
    let _ = fake_repo.add_token_from_claims(&claims, false).await;

    let response = app_server
        .post("/introspect-json")
        .add_header("X-Gateway-Secret", &gateway_secret)
        .json(&json!({ "token": token }))
        .await;
    assert_ok_and_has_claims(response, claims)
}

#[tokio::test]
async fn test_mismatched_audience() {
    let client_id = "019ed4fa-9bc6-73ef-ae5a-9b63ae58ca75";
    let wrong_client_id = "different-client-id";
    let (gateway_secret, auth_server, fake_repo, app_server) = setup_test_server().await;

    let (token, claims) = auth_server.generate_user_access_token(&wrong_client_id);
    let _ = fake_repo.add_token_from_claims(&claims, false).await;

    let response = app_server
        .post("/introspect-json")
        .add_header("X-Gateway-Secret", &gateway_secret)
        .add_header("X-Client-Id", client_id.to_string())
        .json(&json!({ "token": token }))
        .await;

    assert_ok_and_inactive(response);
}

#[tokio::test]
async fn test_invalid_token_without_token_in_repo() {
    let client_id = "019ed4fa-9bc6-73ef-ae5a-9b63ae58ca75";
    let (gateway_secret, auth_server, _, app_server) = setup_test_server().await;

    let (token, _) = auth_server.generate_user_access_token(&client_id);

    let response = app_server
        .post("/introspect-json")
        .add_header("X-Gateway-Secret", &gateway_secret)
        .json(&json!({ "token": token }))
        .await;
    assert_ok_and_inactive(response)
}

#[tokio::test]
async fn test_revoked_token_is_invalid() {
    let client_id = "019ed4fa-9bc6-73ef-ae5a-9b63ae58ca75";
    let (gateway_secret, auth_server, fake_repo, app_server) = setup_test_server().await;

    let (token, claims) = auth_server.generate_user_access_token(&client_id);
    let _ = fake_repo.add_token_from_claims(&claims, true).await;

    let response = app_server
        .post("/introspect-json")
        .add_header("X-Gateway-Secret", &gateway_secret)
        .json(&json!({ "token": token }))
        .await;
    assert_ok_and_inactive(response)
}

#[tokio::test]
async fn test_expired_token_is_invalid() {
    let client_id = "019ed4fa-9bc6-73ef-ae5a-9b63ae58ca75";
    let (gateway_secret, auth_server, fake_repo, app_server) = setup_test_server().await;

    let (token, claims) = auth_server.generate_expired_user_access_token(&client_id);
    let _ = fake_repo.add_token_from_claims(&claims, false).await;

    let response = app_server
        .post("/introspect-json")
        .add_header("X-Gateway-Secret", &gateway_secret)
        .json(&json!({ "token": token }))
        .await;
    assert_ok_and_inactive(response)
}

#[tokio::test]
async fn test_no_token_in_body() {
    let client_id = "019ed4fa-9bc6-73ef-ae5a-9b63ae58ca75";
    let (gateway_secret, auth_server, fake_repo, app_server) = setup_test_server().await;

    let (_, claims) = auth_server.generate_user_access_token(&client_id);
    let _ = fake_repo.add_token_from_claims(&claims, false).await;

    let response = app_server
        .post("/introspect-json")
        .add_header("X-Gateway-Secret", &gateway_secret)
        .json(&json!({}))
        .await;

    response.assert_status_unprocessable_entity();
    response.assert_json(&json!({
        "message": "The given data was invalid.",
        "errors": {
            "token": "The token field is required",
        },
    }));
}

#[tokio::test]
async fn test_empty_token() {
    let client_id = "019ed4fa-9bc6-73ef-ae5a-9b63ae58ca75";
    let (gateway_secret, auth_server, fake_repo, app_server) = setup_test_server().await;

    let (_, claims) = auth_server.generate_user_access_token(&client_id);
    let _ = fake_repo.add_token_from_claims(&claims, false).await;

    let response = app_server
        .post("/introspect-json")
        .add_header("X-Gateway-Secret", &gateway_secret)
        .json(&json!({ "token": "" }))
        .await;

    response.assert_status_unprocessable_entity();
    response.assert_json(&json!({
        "message": "The given data was invalid.",
        "errors": {
            "token": "The token field is required",
        },
    }));
}

#[tokio::test]
async fn test_short_token() {
    let client_id = "019ed4fa-9bc6-73ef-ae5a-9b63ae58ca75";
    let (gateway_secret, auth_server, fake_repo, app_server) = setup_test_server().await;

    let (_, claims) = auth_server.generate_user_access_token(&client_id);
    let _ = fake_repo.add_token_from_claims(&claims, false).await;

    let response = app_server
        .post("/introspect-json")
        .add_header("X-Gateway-Secret", &gateway_secret)
        .json(&json!({ "token": "short" }))
        .await;

    response.assert_status_unprocessable_entity();
    response.assert_json(&json!({
        "message": "The given data was invalid.",
        "errors": {
            "token": "The token must be at least 10 characters",
        },
    }));
}

#[tokio::test]
async fn test_very_large_token() {
    let (gateway_secret, _, _, app_server) = setup_test_server().await;

    let large_token = "a".repeat(10000);
    let response = app_server
        .post("/introspect-json")
        .add_header("X-Gateway-Secret", &gateway_secret)
        .json(&json!({ "token": large_token }))
        .await;

    assert_ok_and_inactive(response);
}

#[tokio::test]
async fn test_malformed_jwt_token() {
    let (gateway_secret, _, _, app_server) = setup_test_server().await;

    let response = app_server
        .post("/introspect-json")
        .add_header("X-Gateway-Secret", &gateway_secret)
        .json(&json!({ "token": "not-a-valid-jwt" }))
        .await;

    assert_ok_and_inactive(response);
}

#[tokio::test]
async fn test_invalid_token_type_hint() {
    let client_id = "019ed4fa-9bc6-73ef-ae5a-9b63ae58ca75";
    let (gateway_secret, auth_server, fake_repo, app_server) = setup_test_server().await;

    let (token, claims) = auth_server.generate_user_access_token(&client_id);
    let _ = fake_repo.add_token_from_claims(&claims, false).await;

    let response = app_server
        .post("/introspect-json")
        .add_header("X-Gateway-Secret", &gateway_secret)
        .json(&json!({
            "token": token,
            "token_type_hint": "invalid_hint"
        }))
        .await;

    response.assert_status_unprocessable_entity();
    response.assert_json(&json!({
        "message": "The given data was invalid.",
        "errors": {
            "token_type_hint": "Must be 'access_token' or 'refresh_token'",
        },
    }));
}

#[tokio::test]
async fn test_valid_access_token_hint() {
    let client_id = "019ed4fa-9bc6-73ef-ae5a-9b63ae58ca75";
    let (gateway_secret, auth_server, fake_repo, app_server) = setup_test_server().await;

    let (token, claims) = auth_server.generate_user_access_token(&client_id);
    let _ = fake_repo.add_token_from_claims(&claims, false).await;

    let response = app_server
        .post("/introspect-json")
        .add_header("X-Gateway-Secret", &gateway_secret)
        .json(&json!({
            "token": token,
            "token_type_hint": "access_token"
        }))
        .await;

    assert_ok_and_has_claims(response, claims);
}

#[tokio::test]
async fn test_valid_refresh_token_hint() {
    let client_id = "019ed4fa-9bc6-73ef-ae5a-9b63ae58ca75";
    let (gateway_secret, auth_server, fake_repo, app_server) = setup_test_server().await;

    let (token, claims) = auth_server.generate_user_access_token(&client_id);
    let _ = fake_repo.add_token_from_claims(&claims, false).await;

    let response = app_server
        .post("/introspect-json")
        .add_header("X-Gateway-Secret", &gateway_secret)
        .json(&json!({
            "token": token,
            "token_type_hint": "refresh_token"
        }))
        .await;

    assert_ok_and_has_claims(response, claims);
}

#[tokio::test]
async fn test_missing_gateway_secret() {
    let client_id = "019ed4fa-9bc6-73ef-ae5a-9b63ae58ca75";
    let (_, auth_server, fake_repo, app_server) = setup_test_server().await;

    let (token, claims) = auth_server.generate_user_access_token(&client_id);
    let _ = fake_repo.add_token_from_claims(&claims, false).await;

    let response = app_server
        .post("/introspect-json")
        .json(&json!({ "token": token }))
        .await;

    response.assert_status_unauthorized();
    response.assert_json(&json!({
        "message": "Unauthorized.",
    }));
}

#[tokio::test]
async fn test_invalid_json_body() {
    let (gateway_secret, _, _, app_server) = setup_test_server().await;

    let response = app_server
        .post("/introspect-json")
        .add_header("X-Gateway-Secret", &gateway_secret)
        .text("{ invalid json }")  // Malformed JSON
        .await;

    response.assert_status(StatusCode::UNSUPPORTED_MEDIA_TYPE);
}

#[tokio::test]
async fn test_wrong_content_type() {
    let (gateway_secret, _, _, app_server) = setup_test_server().await;

    let response = app_server
        .post("/introspect-json")
        .add_header("X-Gateway-Secret", &gateway_secret)
        .add_header("Content-Type", "text/plain")
        .text("token=some-token")
        .await;

    response.assert_status(StatusCode::UNSUPPORTED_MEDIA_TYPE);
}

#[tokio::test]
async fn test_form_valid_token_with_aud_in_header() {
    let client_id = "019ed4fa-9bc6-73ef-ae5a-9b63ae58ca75";
    let (gateway_secret, auth_server, fake_repo, app_server) = setup_test_server().await;

    let (token, claims) = auth_server.generate_user_access_token(&client_id);
    let _ = fake_repo.add_token_from_claims(&claims, false).await;

    let response = app_server
        .post("/introspect")
        .add_header("X-Gateway-Secret", &gateway_secret)
        .add_header("X-Client-Id", client_id.to_string())
        .form(&json!({ "token": token }))
        .await;
    assert_ok_and_has_claims(response, claims)
}

#[tokio::test]
async fn test_http_valid_token_with_aud_in_header() {
    let client_id = "019ed4fa-9bc6-73ef-ae5a-9b63ae58ca75";
    let (gateway_secret, auth_server, fake_repo, app_server) = setup_test_server().await;

    let (token, claims) = auth_server.generate_user_access_token(&client_id);
    let _ = fake_repo.add_token_from_claims(&claims, false).await;

    let response = app_server
        .get("/introspect-http")
        .add_header("X-Gateway-Secret", &gateway_secret)
        .add_header("X-Client-Id", client_id.to_string())
        .add_header("X-Token", token)
        .await;

    response.assert_status_ok()
        .assert_header("X-Sub", claims.sub)
        .assert_header("X-Aud", client_id)
        .assert_header("X-Scope", claims.scopes.as_ref().map(|s| s.join(" ")).unwrap_or_default());
}

#[tokio::test]
async fn test_http_strips_bearer_prefix_for_nginx() {
    let client_id = "019ed4fa-9bc6-73ef-ae5a-9b63ae58ca75";
    let (gateway_secret, auth_server, fake_repo, app_server) = setup_test_server().await;

    let (token, claims) = auth_server.generate_user_access_token(&client_id);
    let _ = fake_repo.add_token_from_claims(&claims, false).await;

    let response = app_server
        .get("/introspect-http")
        .add_header("X-Gateway-Secret", &gateway_secret)
        .add_header("X-Client-Id", client_id)
        .add_header("X-Token", format!("Bearer {}", token.to_string()))
        .await;

    response.assert_status_ok()
        .assert_header("X-Sub", claims.sub)
        .assert_header("X-Aud", client_id)
        .assert_header("X-Scope", claims.scopes.as_ref().map(|s| s.join(" ")).unwrap_or_default());
}

#[tokio::test]
async fn test_http_invalid_token_without_metadata() {
    let client_id = "019ed4fa-9bc6-73ef-ae5a-9b63ae58ca75";
    let (gateway_secret, auth_server, fake_repo, app_server) = setup_test_server().await;

    let (_, claims) = auth_server.generate_user_access_token(&client_id);
    let _ = fake_repo.add_token_from_claims(&claims, false).await;
    let (token_without_meta, _) = auth_server.generate_user_access_token(&client_id);

    let response = app_server
        .get("/introspect-http")
        .add_header("X-Gateway-Secret", &gateway_secret)
        .add_header("X-Client-Id", client_id.to_string())
        .add_header("X-Token", token_without_meta)
        .await;

    response.assert_status_unauthorized();
    let headers = response.headers();
    assert!(!headers.contains_key("X-Sub"));
    assert!(!headers.contains_key("X-Aud"));
    assert!(!headers.contains_key("X-Scope"));
}

#[tokio::test]
async fn test_http_revoked_token_is_invalid() {
    let client_id = "019ed4fa-9bc6-73ef-ae5a-9b63ae58ca75";
    let (gateway_secret, auth_server, fake_repo, app_server) = setup_test_server().await;

    let (token, claims) = auth_server.generate_user_access_token(&client_id);
    let _ = fake_repo.add_token_from_claims(&claims, true).await;

    let response = app_server
        .get("/introspect-http")
        .add_header("X-Gateway-Secret", &gateway_secret)
        .add_header("X-Client-Id", client_id.to_string())
        .add_header("X-Token", token)
        .await;

    response.assert_status_unauthorized();
    let headers = response.headers();
    assert!(!headers.contains_key("X-Sub"));
    assert!(!headers.contains_key("X-Aud"));
    assert!(!headers.contains_key("X-Scope"));
}

#[tokio::test]
async fn test_http_empty_token_is_bad() {
    let client_id = "019ed4fa-9bc6-73ef-ae5a-9b63ae58ca75";
    let (gateway_secret, _, _, app_server) = setup_test_server().await;

    let response = app_server
        .get("/introspect-http")
        .add_header("X-Gateway-Secret", &gateway_secret)
        .add_header("X-Client-Id", client_id.to_string())
        .add_header("X-Token", "")
        .await;

    response.assert_status_bad_request();
    let headers = response.headers();
    assert!(!headers.contains_key("X-Sub"));
    assert!(!headers.contains_key("X-Aud"));
    assert!(!headers.contains_key("X-Scope"));
}

#[tokio::test]
async fn test_http_wrong_gateway_secret_forbidden() {
    let client_id = "019ed4fa-9bc6-73ef-ae5a-9b63ae58ca75";
    let (_, auth_server, fake_repo, app_server) = setup_test_server().await;

    let (token, claims) = auth_server.generate_user_access_token(&client_id);
    let _ = fake_repo.add_token_from_claims(&claims, false).await;

    let response = app_server
        .get("/introspect-http")
        .add_header("X-Gateway-Secret", "wrong")
        .add_header("X-Client-Id", client_id.to_string())
        .add_header("X-Token", token)
        .await;

    response.assert_status_forbidden();
    let headers = response.headers();
    assert!(!headers.contains_key("X-Sub"));
    assert!(!headers.contains_key("X-Aud"));
    assert!(!headers.contains_key("X-Scope"));
}
