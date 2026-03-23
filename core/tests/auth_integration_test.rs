use bpa_engine::services::auth::AuthService;
use sqlx::postgres::PgPoolOptions;

#[tokio::test]
#[ignore] // Ignoring because it requires a live test DB
async fn test_auth_service_registration_and_login() {
    let db_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect("postgres://postgres:postgres@localhost:5432/test_db")
        .await
        .unwrap();

    let auth_service = AuthService::new(db_pool, "test_secret".to_string());

    let stakeholder_id = auth_service
        .register(
            "test1@email.com".to_string(),
            "password123".to_string(),
            "End-user".to_string(),
            "encrypted_data".to_string(),
            "123456789012".to_string(),
            "base64_doc".to_string(),
        )
        .await
        .unwrap();

    let (access, refresh, id, role) = auth_service
        .login("test1@email.com".to_string(), "password123".to_string())
        .await
        .unwrap();

    assert_eq!(id, stakeholder_id);
    assert_eq!(role, "End-user");
    assert!(!access.is_empty());
    assert!(!refresh.is_empty());
}
