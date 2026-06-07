# anycms-i18n 设计方案

## 1. 需求分析

### 1.1 PRD 原始需求

| 需求项 | 描述 | 优先级 |
|--------|------|--------|
| TOML 多语言配置 | 从 toml 文件读取多语言设置 | P0 |
| `t!()` 宏 | 提供类型安全的翻译宏 | P0 |
| anycms 生态集成 | 服务于 anycms-rs 各子 crate | P0 |
| anycms-config 集成 | 利用已有的配置加载体系 | P0 |
| Actix-web 支持 | 中间件 + 请求级 locale 管理 | P1 |
| Axum 支持 | 中间件 + 请求级 locale 管理 | P1 |
| 前端 i18n 返回 | API 返回翻译资源给前端使用 | P1 |

### 1.2 隐含需求分析

从 anycms 生态的使用场景出发，还挖掘出以下隐含需求：

| 隐含需求 | 说明 |
|----------|------|
| **编译时嵌入** | 翻译资源编译时嵌入二进制，避免运行时文件 I/O |
| **Fallback 链** | `zh-Hans-CN` → `zh-CN` → `zh` → `en` 自动回退 |
| **插值支持** | `t!("hello", name = "world")` → `"Hello, world"` |
| **复数支持** | `t!("items", count = 5)` → `"5 items"` |
| **运行时扩展** | CMS 场景需要从数据库动态加载翻译（用户自定义翻译） |
| **lib crate** | 当前是 bin crate（main.rs），需改为 lib crate |
| **no_std 兼容** | 部分 anycms 子 crate 可能需要嵌入场景 |

### 1.3 技术选型决策

**方案：自研轻量 i18n，借鉴 rust-i18n 设计理念**

| 方案 | 优势 | 劣势 | 结论 |
|------|------|------|------|
| 直接依赖 `rust-i18n` | 成熟、功能全 | 侵入性强、无法深度集成 anycms-config、外部依赖重 | ❌ 不采用 |
| 自研，借鉴 rust-i18n | 完全可控、深度集成生态、轻量 | 需要自己维护 | ✅ 采用 |
| `fluent-rs` | 强大的复数/语法处理 | .ftl 格式非标准、学习曲线高、过重 | ❌ 不采用 |
| `gettext-rs` | GNU 标准 | 需要 C FFI、不适合 Web 场景 | ❌ 不采用 |

**选择自研的核心原因：**
1. anycms-config 已有成熟的 TOML 配置加载、profile 合并、环境变量覆盖体系
2. 需要与 anycms-web 的 `CrudContext`、中间件深度集成
3. CMS 场景需要自定义 Backend（数据库翻译），这是通用库难以满足的
4. 保持生态内部一致性，减少外部依赖

---

## 2. 架构设计

### 2.1 Crate 结构

```
anycms-i18n/
├── Cargo.toml
├── src/
│   ├── lib.rs              # 公共 API 导出
│   ├── core.rs             # 核心 trait 和类型定义
│   ├── backend.rs          # Backend trait + 默认 TOML 实现
│   ├── locale.rs           # Locale 解析、协商、Fallback
│   ├── plural.rs           # 复数规则
│   ├── interpolate.rs      # 字符串插值 %{name}
│   ├── config.rs           # 集成 anycms-config 的 I18nConfig
│   ├── macros.rs           # t!() 宏 (declarative macro)
│   ├── builder.rs          # I18nBuilder 构建器
│   └── ext/
│       ├── mod.rs           # Web 框架集成入口
│       ├── actix.rs         # Actix-web 中间件 + Extractor
│       └── axum.rs          # Axum 中间件 + Extractor
├── anycms-i18n-macros/      # 过程宏 crate (可选，预留)
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs
├── tests/
│   ├── test_toml_backend.rs
│   ├── test_fallback.rs
│   ├── test_plural.rs
│   └── test_interpolate.rs
└── examples/
    ├── basic.rs
    ├── actix_integration.rs
    └── axum_integration.rs
```

### 2.2 模块依赖关系

```
┌─────────────────────────────────────────────────────┐
│                    lib.rs (公共 API)                 │
├──────────┬──────────┬──────────┬────────────────────┤
│  macros  │  config  │   ext    │   builder          │
│  (t!())  │(I18nCfg) │(actix/   │  (I18nBuilder)     │
│          │          │ axum)    │                    │
├──────────┴──────────┴──────────┴────────────────────┤
│               core.rs (trait 定义)                   │
├──────────┬──────────┬──────────┬────────────────────┤
│ backend  │  locale  │  plural  │  interpolate       │
│(Backend) │(Locale/  │(Plural   │(%{name}            │
│          │ Fallback)│  Rules)  │  解析)             │
└──────────┴──────────┴──────────┴────────────────────┘
```

### 2.3 核心 Trait 设计

```rust
// === core.rs ===

/// 翻译后端 trait —— 抽象翻译来源
pub trait Backend: Send + Sync + 'static {
    /// 获取指定 locale 和 key 的翻译
    fn get(&self, locale: &str, key: &str) -> Option<String>;

    /// 获取支持的所有 locale
    fn available_locales(&self) -> Vec<String>;

    /// 是否包含指定 locale
    fn has_locale(&self, locale: &str) -> bool;
}

/// 翻译器 —— 核心运行时
pub struct I18n {
    backend: Arc<dyn Backend>,
    config: I18nConfig,
    fallback_chain: FallbackChain,
}

impl I18n {
    /// 翻译 key
    pub fn t(&self, key: &str) -> String { ... }

    /// 翻译 key 并指定 locale
    pub fn t_with_locale(&self, key: &str, locale: &str) -> String { ... }

    /// 翻译 key 带插值参数
    pub fn t_with_args(&self, key: &str, args: &[(&str, &str)]) -> String { ... }

    /// 完整翻译：locale + 插值 + 复数
    pub fn translate(
        &self,
        key: &str,
        locale: Option<&str>,
        args: &[(&str, &str)],
        count: Option<i64>,
    ) -> String { ... }

    /// 获取当前 locale（线程本地）
    pub fn locale(&self) -> &str { ... }

    /// 设置当前 locale（线程本地）
    pub fn set_locale(&self, locale: &str) { ... }
}
```

### 2.4 配置集成设计

```rust
// === config.rs ===

use anycms_config::Config;
use serde::Deserialize;

/// i18n 配置结构
#[derive(Debug, Default, Deserialize)]
#[config(path = "config/i18n.toml")]
pub struct I18nConfig {
    /// 默认 locale，默认 "en"
    #[serde(default = "default_locale")]
    pub default_locale: String,

    /// 支持的 locale 列表
    #[serde(default)]
    pub available_locales: Vec<String>,

    /// 翻译文件目录，默认 "locales"
    #[serde(default = "default_locale_dir")]
    pub locale_dir: String,

    /// Fallback locale 列表（按优先级排列）
    #[serde(default)]
    pub fallback_locales: Vec<String>,

    /// 是否启用运行时热加载
    #[serde(default)]
    pub hot_reload: bool,
}

fn default_locale() -> String { "en".into() }
fn default_locale_dir() -> String { "locales".into() }
```

对应 TOML 配置文件示例：

```toml
# config/i18n.toml
default_locale = "zh-CN"
available_locales = ["zh-CN", "zh-TW", "en", "ja"]
locale_dir = "locales"
fallback_locales = ["en"]

[hot_reload]
enabled = false
```

### 2.5 Backend 设计

#### 2.5.1 默认 Backend：编译时 TOML 嵌入

```rust
// === backend.rs ===

/// 编译时 TOML 翻译后端
///
/// 使用 include_str! 在编译时嵌入翻译文件
pub struct TomlBackend {
    translations: DashMap<String, LocaleTranslations>,
}

struct LocaleTranslations {
    /// 普通翻译 key -> value
    messages: HashMap<String, String>,
    /// 复数翻译 key -> { "zero", "one", "other" } -> value
    plurals: HashMap<String, HashMap<String, String>>,
}

impl TomlBackend {
    /// 从编译时嵌入的字符串构造
    pub fn from_embedded(raw: &[(&str, &str)]) -> Self { ... }

    /// 从目录构造（运行时）
    pub fn from_dir(path: impl AsRef<Path>) -> Result<Self, I18nError> { ... }
}
```

#### 2.5.2 组合 Backend

```rust
/// 支持多 Backend 叠加（CMS 场景：数据库翻译 > 文件翻译）
pub struct ChainedBackend {
    backends: Vec<Arc<dyn Backend>>,
}

impl Backend for ChainedBackend {
    fn get(&self, locale: &str, key: &str) -> Option<String> {
        // 按优先级查找，第一个命中即返回
        for backend in &self.backends {
            if let Some(value) = backend.get(locale, key) {
                return Some(value);
            }
        }
        None
    }
}
```

#### 2.5.3 数据库 Backend（未来扩展）

```rust
/// 数据库翻译后端 —— CMS 编辑器管理的翻译
pub struct DatabaseBackend {
    // 未来集成 anycms-sea-orm
}
```

### 2.6 Locale 与 Fallback 设计

```rust
// === locale.rs ===

/// Locale 标识符解析
///
/// 支持格式:
/// - "en"          -> language only
/// - "zh-CN"       -> language + region
/// - "zh-Hans-CN"  -> language + script + region
pub struct Locale {
    language: String,
    script: Option<String>,
    region: Option<String>,
}

impl Locale {
    /// 从字符串解析
    pub fn parse(input: &str) -> Result<Self, LocaleError> { ... }

    /// 生成 fallback 链
    /// "zh-Hans-CN" → ["zh-Hans-CN", "zh-CN", "zh", "en"]
    pub fn fallback_chain(&self, default: &str) -> Vec<String> { ... }
}

/// Accept-Language 头解析与协商
pub fn negotiate_locale(
    accept_language: &str,
    available: &[&str],
    default: &str,
) -> String { ... }
```

**Fallback 规则：**
```
zh-Hans-CN
  → zh-Hans-CN  (完全匹配)
  → zh-CN       (忽略 script)
  → zh          (仅 language)
  → en          (默认 fallback)
```

### 2.7 字符串插值设计

```rust
// === interpolate.rs ===

/// 解析并替换 %{name} 格式的插值
///
/// 支持:
///   "%{name}"          → 简单替换
///   "%{count}"         → 复数相关
///   "%{name:default}"  → 带默认值 (预留)
pub fn interpolate(template: &str, args: &HashMap<String, String>) -> String { ... }
```

### 2.8 复数规则设计

```rust
// === plural.rs ===

/// 复数类别
#[derive(Debug, Clone, Copy)]
pub enum PluralCategory {
    Zero,
    One,
    Two,
    Few,
    Many,
    Other,
}

/// 根据 locale 和 count 确定复数类别
pub fn plural_category(locale: &str, count: i64) -> PluralCategory { ... }
```

**TOML 中的复数写法：**
```toml
# locales/en.toml
[items]
zero = "No items"
one = "One item"
other = "%{count} items"

# locales/zh-CN.toml
[items]
other = "%{count} 个项目"
```

### 2.9 `t!()` 宏设计

```rust
// === macros.rs ===

/// 核心翻译宏
///
/// 用法:
///   t!("key")                                // 简单翻译
///   t!("key", locale = "zh-CN")              // 指定 locale
///   t!("key", name = "world")                // 带插值
///   t!("key", locale = "zh-CN", count = 5)   // 复数 + locale
///   t!("key", name = "test", count = 3)      // 插值 + 复数
#[macro_export]
macro_rules! t {
    // t!("key")
    ($key:expr) => { ... };

    // t!("key", locale = "zh-CN")
    ($key:expr, locale = $locale:expr) => { ... };

    // t!("key", name = "value", ...)
    ($key:expr, $($name:ident = $value:expr),+ $(,)?) => { ... };
}
```

### 2.10 Builder 模式

```rust
// === builder.rs ===

/// I18n 构建器
///
/// ```rust
/// let i18n = I18nBuilder::new()
///     .config(I18nConfig::load())
///     .embedded_translations(&[
///         ("en", include_str!("../locales/en.toml")),
///         ("zh-CN", include_str!("../locales/zh-CN.toml")),
///     ])
///     .fallback_locale("en")
///     .build()?;
/// ```
pub struct I18nBuilder {
    config: I18nConfig,
    backends: Vec<Arc<dyn Backend>>,
    fallback_locale: Option<String>,
}

impl I18nBuilder {
    pub fn new() -> Self { ... }
    pub fn config(mut self, config: I18nConfig) -> Self { ... }
    pub fn embedded_translations(mut self, translations: &[(&str, &str)]) -> Self { ... }
    pub fn add_backend(mut self, backend: Arc<dyn Backend>) -> Self { ... }
    pub fn fallback_locale(mut self, locale: &str) -> Self { ... }
    pub fn build(self) -> Result<I18n, I18nError> { ... }
}
```

---

## 3. Web 框架集成设计

### 3.1 Actix-web 集成

```rust
// === ext/actix.rs ===

/// Actix-web i18n 中间件
///
/// 从以下来源自动检测 locale:
/// 1. URL 查询参数 `?lang=zh-CN`
/// 2. Cookie `locale=zh-CN`
/// 3. Accept-Language 请求头
/// 4. 配置中的默认 locale
pub struct I18nMiddleware {
    i18n: Arc<I18n>,
}

/// Locale 提取器 —— 在 handler 中直接获取当前请求的 locale
///
/// ```rust
/// async fn handler(locale: LocaleExtractor) -> String {
///     t!("welcome", locale = locale.as_str())
/// }
/// ```
pub struct LocaleExtractor {
    locale: String,
    i18n: Arc<I18n>,
}

impl FromRequest for LocaleExtractor { ... }

/// Actix-web App 扩展 trait
pub trait I18nAppExt {
    /// 注册 i18n 中间件
    fn i18n(self, i18n: Arc<I18n>) -> Self;
}
```

**使用示例：**

```rust
use actix_web::{web, App, HttpServer};
use anycms_i18n::{I18nBuilder, I18nConfig, ext::actix::I18nAppExt};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let i18n = I18nBuilder::new()
        .config(I18nConfig::load())
        .embedded_translations(&[
            ("en", include_str!("../locales/en.toml")),
            ("zh-CN", include_str!("../locales/zh-CN.toml")),
        ])
        .build()
        .unwrap();

    let i18n = Arc::new(i18n);

    HttpServer::new(move || {
        App::new()
            .i18n(i18n.clone())  // 注册中间件
            .route("/api/greet", web::get().to(greet))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}

async fn greet(locale: LocaleExtractor) -> String {
    t!("welcome", locale = locale.as_str())
}
```

### 3.2 Axum 集成

```rust
// === ext/axum.rs ===

/// Axum i18n State
pub struct I18nState {
    i18n: Arc<I18n>,
}

/// Axum Layer（中间件）
pub struct I18nLayer {
    i18n: Arc<I18n>,
}

/// Axum Extractor
#[derive(Clone)]
pub struct Locale {
    locale: String,
    i18n: Arc<I18n>,
}

// 实现 FromRequestParts for Locale
#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for Locale { ... }
```

### 3.3 前端 i18n 资源 API

```rust
/// 返回指定 locale 的所有翻译（JSON 格式），供前端使用
///
/// GET /api/i18n/{locale} → { "welcome": "Welcome", ... }
///
/// 或作为 Web 框架的路由注册 helper
pub fn i18n_routes(i18n: Arc<I18n>) -> Router {
    // 提供:
    // GET /api/i18n/locales          → 返回支持的语言列表
    // GET /api/i18n/{locale}         → 返回指定语言的全部翻译
    // GET /api/i18n/{locale}/{key}   → 返回单个翻译
}
```

---

## 4. 翻译文件格式设计

### 4.1 目录结构

```
locales/
├── en.toml          # 英文翻译
├── zh-CN.toml       # 简体中文翻译
├── zh-TW.toml       # 繁体中文翻译
└── ja.toml          # 日文翻译
```

### 4.2 TOML 文件格式

```toml
# locales/en.toml

# 顶层简单 key-value
app.title = "Anycms"
app.description = "A modern CMS built with Rust"

# 表格式（支持嵌套）
[welcome]
message = "Welcome, %{name}!"

[errors]
not_found = "Page not found"
unauthorized = "Please login first"
forbidden = "Access denied"

[items]
zero = "No items"
one = "%{count} item"
other = "%{count} items"

[navigation]
home = "Home"
about = "About"
contact = "Contact"
```

```toml
# locales/zh-CN.toml

app.title = "Anycms"
app.description = "基于 Rust 构建的现代化 CMS"

[welcome]
message = "欢迎，%{name}！"

[errors]
not_found = "页面未找到"
unauthorized = "请先登录"
forbidden = "访问被拒绝"

[items]
other = "%{count} 个项目"

[navigation]
home = "首页"
about = "关于"
contact = "联系我们"
```

### 4.3 Key 命名约定

| 层级 | 格式 | 示例 |
|------|------|------|
| 模块 | `module.` 前缀 | `auth.login`, `user.profile` |
| 错误 | `errors.` 前缀 | `errors.not_found` |
| 验证 | `validations.` 前缀 | `validations.required` |
| 通用 | `common.` 前缀 | `common.save`, `common.cancel` |
| 复数 | 表 + `zero/one/other` 键 | `[items]` + `zero/one/other` |

---

## 5. Cargo.toml 设计

```toml
[package]
name = "anycms-i18n"
version = "0.1.0"
edition = "2024"
description = "Internationalization support for the anycms-rs ecosystem"

[features]
default = ["toml-backend"]

# Backend 特性
toml-backend = ["dep:toml"]

# Web 框架集成
actix = ["dep:actix-web"]
axum = ["dep:axum", "dep:tower", "dep:futures"]

# 配置系统集成
config = ["dep:anycms-config"]

# 运行时文件加载（非编译时嵌入）
fs-loader = []

# 热加载
hot-reload = ["fs-loader", "dep:notify"]

[dependencies]
# 核心
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
thiserror = { workspace = true }
parking_lot = { workspace = true }
dashmap = { workspace = true }

# 可选依赖
toml = { workspace = true, optional = true }
anycms-config = { path = "../anycms-config", optional = true }

# Actix-web 集成
actix-web = { workspace = true, optional = true }

# Axum 集成
axum = { workspace = true, optional = true }
tower = { workspace = true, optional = true }
futures = { workspace = true, optional = true }

# 热加载
notify = { version = "7", optional = true }
```

---

## 6. 公共 API 总览

```rust
// === lib.rs ===

// 核心类型
pub use crate::core::{I18n, Backend};
pub use crate::config::I18nConfig;
pub use crate::builder::I18nBuilder;
pub use crate::locale::{Locale, negotiate_locale};
pub use crate::error::I18nError;

// Backend 实现
pub use crate::backend::TomlBackend;
pub use crate::backend::ChainedBackend;

// 宏
pub use crate::macros::t;

// Web 框架扩展 (feature-gated)
#[cfg(feature = "actix")]
pub mod ext_actix {
    pub use crate::ext::actix::{I18nMiddleware, LocaleExtractor, I18nAppExt};
}

#[cfg(feature = "axum")]
pub mod ext_axum {
    pub use crate::ext::axum::{I18nLayer, I18nState, Locale};
}

// 前端 API 路由
pub mod api;
```

---

## 7. 实现计划

### Phase 1: 核心基础（P0）

**目标：** 可用的 t!() 宏 + TOML Backend

| 步骤 | 内容 | 预计工作量 |
|------|------|-----------|
| 1 | 将 bin crate 改为 lib crate | 0.5h |
| 2 | 实现 `I18nError` 错误类型 | 0.5h |
| 3 | 实现 `Locale` 解析 + fallback chain | 1h |
| 4 | 实现 `TomlBackend` | 1.5h |
| 5 | 实现字符串插值 `interpolate()` | 1h |
| 6 | 实现复数规则 `plural_category()` | 1h |
| 7 | 实现 `t!()` 宏 | 1h |
| 8 | 实现 `I18nBuilder` + `I18n` 核心 | 1h |
| 9 | 编写单元测试 | 1.5h |

### Phase 2: 生态集成（P0-P1）

| 步骤 | 内容 | 预计工作量 |
|------|------|-----------|
| 10 | 集成 anycms-config (`I18nConfig`) | 1h |
| 11 | 实现 `ChainedBackend` | 0.5h |
| 12 | 编写集成测试 | 1h |

### Phase 3: Web 框架集成（P1）

| 步骤 | 内容 | 预计工作量 |
|------|------|-----------|
| 13 | Actix-web 中间件 + Extractor | 2h |
| 14 | Axum Layer + Extractor | 2h |
| 15 | 前端 i18n 资源 API | 1h |
| 16 | Web 集成测试 + 示例 | 2h |

### Phase 4: 高级特性（P2，后续迭代）

- 热加载支持（`notify` 文件监听）
- 数据库 Backend（集成 `anycms-sea-orm`）
- CLI 工具（`cargo anycms-i18n extract` 提取未翻译 key）
- 翻译合并与覆盖策略
- 更多复数规则（斯拉夫语系、阿拉伯语等）

---

## 8. 设计决策记录

| # | 决策 | 原因 | 备选方案 |
|---|------|------|----------|
| D1 | 自研而非依赖 rust-i18n | 深度集成 anycms-config/web，保持生态一致性 | 直接使用 rust-i18n |
| D2 | TOML 为唯一翻译格式 | 与 anycms-config 一致，Rust 生态标准 | 同时支持 YAML/JSON |
| D3 | 编译时嵌入 + 可选运行时加载 | 性能优先，CMS 场景按需扩展 | 纯运行时加载 |
| D4 | Feature flag 控制 web 集成 | 按需引入，避免不必要的依赖 | 单独的 crate |
| D5 | 声明宏 `t!()` 而非过程宏 | 简单够用、编译快 | 过程宏（编译时 key 校验） |
| D6 | Arc + DashMap 存储 | 高并发读、线程安全 | RwLock + HashMap |
| D7 | `%{name}` 插值语法 | 与 rust-i18n 一致，开发者熟悉 | `{name}` (Fluent 风格) |
| D8 | 单 lib crate + feature flags | 简单直接，减少 crate 数量 | 拆分为 -actix/-axum 子 crate |

---

## 9. 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 复数规则覆盖不全 | 部分语言复数形式不正确 | Phase 1 先覆盖中英日，后续迭代补充 CLDR 规则 |
| 宏 API 灵活性不足 | 无法满足复杂翻译场景 | 预留 `I18n::translate()` 完整 API 作为 escape hatch |
| Actix/Axum API 变更 | 集成层需要跟随升级 | 通过 feature flag 隔离，版本对齐 |
| 性能未达预期 | 高并发下翻译成为瓶颈 | 编译时嵌入 + DashMap 无锁读，理论 < 50ns/op |
