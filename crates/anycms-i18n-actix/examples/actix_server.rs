//! Actix-web server example demonstrating anycms-i18n integration.
//!
//! Run: `cargo run -p anycms-i18n-actix --example actix_server`
//!
//! Test:
//!     curl http://localhost:8080/
//!     curl "http://localhost:8080/?lang=zh-CN"
//!     curl http://localhost:8080/greet/Alice
//!     curl -H "Accept-Language: zh-CN" http://localhost:8080/greet/Bob
//!     curl http://localhost:8080/api/i18n/locales
//!     curl http://localhost:8080/api/i18n/zh-CN

use std::sync::Arc;

use actix_web::{App, HttpResponse, HttpServer, web};
use anycms_i18n::i18n;
use anycms_i18n_actix::{I18nAppExt, I18nMiddleware, LocaleExtractor};

/// Demonstrates: t!() macro auto-detects locale from request context.
async fn index(locale: LocaleExtractor) -> HttpResponse {
    // t!() automatically uses the request's locale — no need to pass locale = ...
    let welcome = anycms_i18n::t!("welcome");
    let locale_str = locale.as_str();

    HttpResponse::Ok()
        .content_type("text/plain; charset=utf-8")
        .body(format!(
            "[{locale_str}] {welcome}\n\nTry:\n  \
         curl http://localhost:8080/?lang=zh-CN\n  \
         curl http://localhost:8080/greet/Alice\n  \
         curl -H 'Accept-Language: zh-CN' http://localhost:8080/greet/Bob\n  \
         curl http://localhost:8080/api/i18n/locales"
        ))
}

/// Demonstrates: t!() macro with interpolation + auto locale.
async fn greet(locale: LocaleExtractor, path: web::Path<String>) -> HttpResponse {
    let name = path.into_inner();
    let greeting = anycms_i18n::t!("greeting", name = &name);
    let locale_str = locale.as_str();

    HttpResponse::Ok()
        .content_type("text/plain; charset=utf-8")
        .body(format!("[{locale_str}] {greeting}"))
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let i18n = i18n!("../../locales", default = "en", fallback = "en");
    let i18n = Arc::new(i18n);

    println!("anycms-i18n actix-web server");
    println!("Available locales: {:?}", i18n.available_locales());
    println!("Listening on http://0.0.0.0:8080");

    HttpServer::new(move || {
        let i18n_clone = i18n.clone();
        App::new()
            .wrap(I18nMiddleware::new(i18n_clone.clone()))
            .i18n_routes(i18n_clone)
            .route("/", web::get().to(index))
            .route("/greet/{name}", web::get().to(greet))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
