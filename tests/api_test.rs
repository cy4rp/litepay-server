use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

/// Build the app router with an in-memory SQLite database.
async fn app() -> axum::Router {
    use litepay_server::build_app;
    build_app("sqlite::memory:").await.unwrap()
}

#[tokio::test]
async fn health_check() {
    let app = app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["server"], "LitePay");
}

#[tokio::test]
async fn create_wallet_and_get() {
    let app = app().await;

    // Create wallet
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/wallets")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"name":"test"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let wallet: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(wallet["name"], "test");
    assert_eq!(wallet["balance_msat"], 0);

    let invoice_key = wallet["invoice_key"].as_str().unwrap();
    assert_eq!(invoice_key.len(), 32);

    // Get wallet via API key
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/wallet")
                .header("X-Api-Key", invoice_key)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let fetched: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(fetched["id"], wallet["id"]);
}

#[tokio::test]
async fn create_invoice_and_list() {
    let app = app().await;

    // Create wallet first
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/wallets")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"name":"inv-test"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let wallet: Value = serde_json::from_slice(&body).unwrap();
    let invoice_key = wallet["invoice_key"].as_str().unwrap();

    // Create invoice
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/payments")
                .header("Content-Type", "application/json")
                .header("X-Api-Key", invoice_key)
                .body(Body::from(r#"{"amount":500,"memo":"test pay"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let invoice: Value = serde_json::from_slice(&body).unwrap();
    assert!(invoice["payment_request"]
        .as_str()
        .unwrap()
        .starts_with("lnbc"));
    assert_eq!(invoice["payment_hash"].as_str().unwrap().len(), 64);

    // List payments
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/payments")
                .header("X-Api-Key", invoice_key)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let payments: Vec<Value> = serde_json::from_slice(&body).unwrap();
    assert_eq!(payments.len(), 1);
    assert_eq!(payments[0]["amount"], 500);
}

#[tokio::test]
async fn auth_rejection() {
    let app = app().await;

    // No key
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/wallet")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // Invalid key
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/wallet")
                .header("X-Api-Key", "bad-key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn lnurl_pay_flow() {
    let app = app().await;

    // Create wallet
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/wallets")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"name":"lnurl-test"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let wallet: Value = serde_json::from_slice(&body).unwrap();
    let wallet_id = wallet["id"].as_str().unwrap();

    // LNURL-pay step 1
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/lnurlp/{}", wallet_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let lnurl: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(lnurl["tag"], "payRequest");
    assert_eq!(lnurl["minSendable"], 1000);
    assert_eq!(lnurl["maxSendable"], 100_000_000);

    // LNURL-pay callback with valid amount
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/lnurlp/{}/callback?amount=5000", wallet_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let cb: Value = serde_json::from_slice(&body).unwrap();
    assert!(cb["pr"].as_str().unwrap().starts_with("lnbc"));

    // LNURL-pay callback with invalid amount (too low)
    let resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/lnurlp/{}/callback?amount=500", wallet_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
