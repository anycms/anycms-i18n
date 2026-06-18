//! Axum server example demonstrating anycms-i18n integration.
//!
//! Run: `cargo run -p anycms-i18n-axum --example axum_server`
//!
//! Test:
//!     curl http://localhost:8081/
//!     curl "http://localhost:8081/?lang=zh-CN"
//!     curl http://localhost:8081/greet/Alice
//!     curl -H "Accept-Language: zh-CN" http://localhost:8081/greet/Bob
//!     curl http://localhost:8081/api/i18n/locales

use std::sync::Arc;

use anycms_i18n::i18n;
use anycms_i18n_axum::{I18nLayer, I18nRouterExt, Locale};
use axum::{Router, extract::Path, routing::get};

/// Demonstrates: t!() macro auto-detects locale from request context.
async fn index(locale: Locale) -> String {
    // t!() automatically uses the request's locale — no need to pass locale = ...
    let msg = anycms_i18n::t!("welcome");
    format!("[{}] {}", locale.as_str(), msg)
}

/// Demonstrates: t!() macro with interpolation + auto locale.
async fn greet(locale: Locale, Path(name): Path<String>) -> String {
    let msg = anycms_i18n::t!("greeting", name = &name);
    format!("[{}] {}", locale.as_str(), msg)
}

#[tokio::main]
async fn main() {
    let i18n = Arc::new(i18n!("../../locales", default = "en", fallback = "en"));

    println!("anycms-i18n axum server");
    println!("Available locales: {:?}", i18n.available_locales());
    println!("Listening on http://0.0.0.0:8081");

    let i18n_clone = i18n.clone();
    let app = Router::new()
        .route("/", get(index))
        .route("/greet/{name}", get(greet))
        .i18n_routes(i18n_clone)
        .layer(I18nLayer::new(i18n));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8081")
        .await
        .expect("failed to bind");

    axum::serve(listener, app).await.expect("server error");
}
