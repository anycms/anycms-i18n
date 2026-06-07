//! Axum integration for anycms-i18n.
//!
//! Provides a Layer (middleware) for automatic locale detection and an
//! extractor for accessing the current locale in handlers.
//!
//! # Example
//!
//! ```rust,ignore
//! use axum::{routing::get, Router};
//! use anycms_i18n::I18nBuilder;
//! use anycms_i18n_axum::{I18nLayer, Locale};
//! use std::sync::Arc;
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
//! let app = Router::new()
//!     .route("/greet", get(greet))
//!     .layer(I18nLayer::new(Arc::new(i18n)));
//!
//! async fn greet(locale: Locale) -> String {
//!     format!("Your locale: {}", locale.as_str())
//! }
//! ```

use std::sync::Arc;

use axum::{
    extract::FromRequestParts,
    http::request::Parts,
    response::{IntoResponse, Response},
};
use tower::{Layer, Service};

use anycms_i18n::{negotiate_locale, I18n};

// ---- I18nState ----

/// Shared i18n state, stored in request extensions by the middleware.
#[derive(Clone, Debug)]
pub struct I18nState {
    locale: String,
    i18n: Arc<I18n>,
}

impl I18nState {
    /// Get the resolved locale string.
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

    /// Translate a key with interpolation.
    pub fn t_with_args(&self, key: &str, args: &[(&str, &str)]) -> String {
        self.i18n.t_with_args(key, &self.locale, args)
    }
}

// ---- I18nLayer ----

/// Axum Layer (middleware) that detects locale from the request.
///
/// Detection order:
/// 1. Query parameter `?lang=zh-CN`
/// 2. Cookie `locale`
/// 3. `Accept-Language` header
/// 4. Default locale
#[derive(Clone)]
pub struct I18nLayer {
    i18n: Arc<I18n>,
}

impl I18nLayer {
    /// Create a new layer with the given i18n instance.
    pub fn new(i18n: Arc<I18n>) -> Self {
        Self { i18n }
    }
}

impl<S> Layer<S> for I18nLayer {
    type Service = I18nMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        I18nMiddleware {
            inner,
            i18n: self.i18n.clone(),
        }
    }
}

/// The middleware service.
#[derive(Clone)]
pub struct I18nMiddleware<S> {
    inner: S,
    i18n: Arc<I18n>,
}

impl<S> Service<axum::http::Request<axum::body::Body>> for I18nMiddleware<S>
where
    S: Service<axum::http::Request<axum::body::Body>> + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Send + 'static,
    S::Response: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = futures::future::BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: axum::http::Request<axum::body::Body>) -> Self::Future {
        let locale = self.detect_locale(&req);
        let state = I18nState {
            locale: locale.clone(),
            i18n: self.i18n.clone(),
        };
        req.extensions_mut().insert(state);

        let fut = self.inner.call(req);

        // Wrap the request in CURRENT_LOCALE scope so t!() auto-detects locale
        Box::pin(async move {
            anycms_i18n::CURRENT_LOCALE.scope(locale, fut).await
        })
    }
}

impl<S> I18nMiddleware<S> {
    fn detect_locale(&self, req: &axum::http::Request<axum::body::Body>) -> String {
        let available = self.i18n.available_locales();
        let available_refs: Vec<&str> = available.iter().map(|s| s.as_str()).collect();

        // 1. Query parameter
        if let Some(lang) = req.uri().query().and_then(|q| {
            q.split('&')
                .find_map(|pair| {
                    let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
                    if k == "lang" { Some(v) } else { None }
                })
        }) {
            if self.i18n.backend().has_locale(lang) {
                return lang.to_string();
            }
        }

        // 2. Cookie
        if let Some(cookie) = req
            .headers()
            .get("cookie")
            .and_then(|v| v.to_str().ok())
            .and_then(|cookies| {
                cookies.split(';').find_map(|c| {
                    let c = c.trim();
                    if let Some(v) = c.strip_prefix("locale=") {
                        Some(v.to_string())
                    } else {
                        None
                    }
                })
            })
        {
            if self.i18n.backend().has_locale(&cookie) {
                return cookie;
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

// ---- Locale Extractor ----

/// Axum extractor for the resolved locale.
///
/// ```rust,ignore
/// async fn handler(locale: Locale) -> String {
///     locale.t("welcome")
/// }
/// ```
#[derive(Clone, Debug)]
pub struct Locale {
    state: I18nState,
}

impl Locale {
    /// Get the locale string.
    pub fn as_str(&self) -> &str {
        self.state.as_str()
    }

    /// Get the underlying i18n state.
    pub fn state(&self) -> &I18nState {
        &self.state
    }

    /// Translate a key using the request's locale.
    pub fn t(&self, key: &str) -> String {
        self.state.t(key)
    }

    /// Translate with interpolation.
    pub fn t_with_args(&self, key: &str, args: &[(&str, &str)]) -> String {
        self.state.t_with_args(key, args)
    }
}

impl FromRequestParts<()> for Locale {
    type Rejection = LocaleRejection;

    fn from_request_parts(
        parts: &mut Parts,
        _state: &(),
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        let result = parts
            .extensions
            .get::<I18nState>()
            .cloned()
            .map(|state| Locale { state })
            .ok_or(LocaleRejection::MiddlewareNotRegistered);

        std::future::ready(result)
    }
}

/// Rejection type for the Locale extractor.
#[derive(Debug)]
pub enum LocaleRejection {
    /// The I18nLayer middleware was not registered.
    MiddlewareNotRegistered,
}

impl IntoResponse for LocaleRejection {
    fn into_response(self) -> Response {
        match self {
            Self::MiddlewareNotRegistered => {
                (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "i18n middleware not registered").into_response()
            }
        }
    }
}

impl std::fmt::Display for LocaleRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MiddlewareNotRegistered => write!(f, "i18n middleware not registered"),
        }
    }
}

impl std::error::Error for LocaleRejection {}

// ---- Router extension ----

/// Extension trait for adding i18n API routes to an Axum router.
pub trait I18nRouterExt {
    /// Add i18n API routes:
    /// - `GET /api/i18n/locales` — list available locales
    /// - `GET /api/i18n/{locale}` — get translations for a locale
    fn i18n_routes(self, i18n: Arc<I18n>) -> Self;
}

impl I18nRouterExt for axum::Router<()> {
    fn i18n_routes(self, i18n: Arc<I18n>) -> Self {
        self.route(
            "/api/i18n/locales",
            axum::routing::get({
                let i18n = i18n.clone();
                move || {
                    let i18n = i18n.clone();
                    async move {
                        axum::Json(serde_json::json!({
                            "locales": i18n.available_locales(),
                            "default": i18n.default_locale(),
                        }))
                    }
                }
            }),
        )
        .route(
            "/api/i18n/{locale}",
            axum::routing::get({
                let i18n = i18n.clone();
                move |axum::extract::Path(locale): axum::extract::Path<String>| {
                    let i18n = i18n.clone();
                    async move {
                        let _available = i18n.available_locales();
                        axum::Json(serde_json::json!({
                            "locale": &locale,
                            "translations": format!("Translations for {locale}"),
                        }))
                    }
                }
            }),
        )
    }
}
