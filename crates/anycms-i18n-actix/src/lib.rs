//! Actix-web integration for anycms-i18n.
//!
//! Provides middleware for automatic locale detection and an extractor
//! for accessing the current locale in handlers.
//!
//! # Example
//!
//! ```rust,ignore
//! use actix_web::{web, App, HttpServer};
//! use anycms_i18n::I18nBuilder;
//! use anycms_i18n_actix::{I18nMiddleware, LocaleExtractor};
//!
//! let i18n = I18nBuilder::new()
//!     .default_locale("en")
//!     .embedded_translations(&[
//!         ("en", include_str!("../../locales/en.toml")),
//!         ("zh-CN", include_str!("../../locales/zh-CN.toml")),
//!     ])
//!     .build()
//!     .unwrap();
//!
//! HttpServer::new(move || {
//!     App::new()
//!         .wrap(I18nMiddleware::new(std::sync::Arc::new(i18n.clone())))
//!         .route("/greet", web::get().to(greet))
//! })
//! .bind("0.0.0.0:8080")?
//! .run()
//! .await
//!
//! async fn greet(locale: LocaleExtractor) -> String {
//!     format!("Your locale: {}", locale.as_str())
//! }
//! ```

use std::sync::Arc;

use actix_web::{
    Error, FromRequest, HttpMessage, HttpRequest,
    dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
    web,
};
use futures::future::{LocalBoxFuture, Ready, ok};

use anycms_i18n::{I18n, negotiate_locale};

/// Actix-web middleware that detects the request locale and stores it in request extensions.
///
/// Locale detection order:
/// 1. Query parameter `?lang=zh-CN`
/// 2. Cookie `locale`
/// 3. `Accept-Language` header
/// 4. Default locale from i18n config
pub struct I18nMiddleware {
    i18n: Arc<I18n>,
}

impl I18nMiddleware {
    /// Create a new middleware with the given i18n instance.
    pub fn new(i18n: Arc<I18n>) -> Self {
        Self { i18n }
    }
}

/// The resolved locale stored in request extensions.
#[derive(Clone, Debug)]
pub struct ResolvedLocale {
    locale: String,
    i18n: Arc<I18n>,
}

impl ResolvedLocale {
    /// Get the locale string.
    pub fn as_str(&self) -> &str {
        &self.locale
    }

    /// Get a reference to the i18n instance.
    pub fn i18n(&self) -> &I18n {
        &self.i18n
    }

    /// Translate a key using the resolved locale.
    pub fn t(&self, key: &str) -> String {
        self.i18n.t_with_locale(key, &self.locale)
    }

    /// Translate a key with interpolation using the resolved locale.
    pub fn t_with_args(&self, key: &str, args: &[(&str, &str)]) -> String {
        self.i18n.t_with_args(key, &self.locale, args)
    }
}

impl<S, B> Transform<S, ServiceRequest> for I18nMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = I18nMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(I18nMiddlewareService {
            service,
            i18n: self.i18n.clone(),
        })
    }
}

/// The middleware service implementation.
pub struct I18nMiddlewareService<S> {
    service: S,
    i18n: Arc<I18n>,
}

impl<S, B> Service<ServiceRequest> for I18nMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let locale = self.detect_locale(&req);

        req.extensions_mut().insert(ResolvedLocale {
            locale: locale.clone(),
            i18n: self.i18n.clone(),
        });

        let fut = self.service.call(req);

        // Wrap the request in CURRENT_LOCALE scope so t!() auto-detects locale
        Box::pin(async move { anycms_i18n::CURRENT_LOCALE.scope(locale, fut).await })
    }
}

impl<S> I18nMiddlewareService<S> {
    fn detect_locale(&self, req: &ServiceRequest) -> String {
        let available = self.i18n.available_locales();
        let available_refs: Vec<&str> = available.iter().map(|s| s.as_str()).collect();

        // 1. Query parameter
        if let Some(lang) = req.uri().query().and_then(|q| {
            q.split('&').find_map(|pair| {
                let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
                if k == "lang" {
                    Some(v.to_string())
                } else {
                    None
                }
            })
        }) && self.i18n.backend().has_locale(&lang)
        {
            return lang;
        }

        // 2. Cookie
        if let Some(cookie) = req.cookie("locale") {
            let val = cookie.value().to_string();
            if self.i18n.backend().has_locale(&val) {
                return val;
            }
        }

        // 3. Accept-Language header
        if let Some(accept) = req
            .headers()
            .get("accept-language")
            .and_then(|v| v.to_str().ok())
        {
            let negotiated = negotiate_locale(accept, &available_refs, self.i18n.default_locale());
            if self.i18n.backend().has_locale(&negotiated) {
                return negotiated;
            }
        }

        // 4. Default
        self.i18n.default_locale().to_string()
    }
}

/// Extractor that provides the resolved locale in an Actix-web handler.
///
/// ```rust,ignore
/// async fn handler(locale: LocaleExtractor) -> String {
///     locale.t("welcome")
/// }
/// ```
#[derive(Clone, Debug)]
pub struct LocaleExtractor {
    resolved: ResolvedLocale,
}

impl LocaleExtractor {
    /// Get the locale string.
    pub fn as_str(&self) -> &str {
        self.resolved.as_str()
    }

    /// Get the resolved locale (includes i18n reference).
    pub fn resolved(&self) -> &ResolvedLocale {
        &self.resolved
    }

    /// Translate a key using the request's locale.
    pub fn t(&self, key: &str) -> String {
        self.resolved.t(key)
    }

    /// Translate a key with interpolation.
    pub fn t_with_args(&self, key: &str, args: &[(&str, &str)]) -> String {
        self.resolved.t_with_args(key, args)
    }
}

impl FromRequest for LocaleExtractor {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut actix_web::dev::Payload) -> Self::Future {
        let resolved = req.extensions().get::<ResolvedLocale>().cloned();

        match resolved {
            Some(r) => ok(LocaleExtractor { resolved: r }),
            None => {
                // Middleware not registered — return default locale
                ok(LocaleExtractor {
                    resolved: ResolvedLocale {
                        locale: "en".to_string(),
                        i18n: match anycms_i18n::global() {
                            Some(i) => Arc::new(i.clone()),
                            None => {
                                // This should not happen in practice
                                return ok(LocaleExtractor {
                                    resolved: ResolvedLocale {
                                        locale: "en".to_string(),
                                        i18n: Arc::new(
                                            I18nBuilder::new().build().unwrap_or_else(|_| {
                                                panic!("I18nMiddleware not registered and no global i18n set")
                                            })
                                        ),
                                    },
                                });
                            }
                        },
                    },
                })
            }
        }
    }
}

/// Extension trait for registering i18n routes on an Actix App.
///
/// Provides endpoints for frontend i18n resource loading:
/// - `GET /api/i18n/locales` — list available locales
/// - `GET /api/i18n/{locale}` — get all translations for a locale
pub trait I18nAppExt {
    /// Register i18n API routes under the given path prefix (default: `/api/i18n`).
    fn i18n_routes(self, i18n: Arc<I18n>) -> Self;
}

impl<T> I18nAppExt for actix_web::App<T>
where
    T: actix_web::dev::ServiceFactory<
            actix_web::dev::ServiceRequest,
            Config = (),
            Error = Error,
            InitError = (),
        >,
{
    fn i18n_routes(self, i18n: Arc<I18n>) -> Self {
        self.service(
            web::scope("/api/i18n")
                .app_data(web::Data::new(i18n))
                .route("/locales", web::get().to(list_locales))
                .route("/{locale}", web::get().to(get_translations)),
        )
    }
}

async fn list_locales(i18n: web::Data<Arc<I18n>>) -> web::Json<serde_json::Value> {
    web::Json(serde_json::json!({
        "locales": i18n.available_locales(),
        "default": i18n.default_locale(),
    }))
}

async fn get_translations(
    i18n: web::Data<Arc<I18n>>,
    path: web::Path<String>,
) -> web::Json<serde_json::Value> {
    let locale = path.into_inner();
    let translations = serde_json::to_value(i18n.backend().dump(&locale))
        .unwrap_or_else(|_| serde_json::json!({}));
    web::Json(serde_json::json!({
        "locale": locale,
        "translations": translations,
    }))
}

use anycms_i18n::I18nBuilder;
