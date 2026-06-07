# Anycms-Rs 生态 i18n 支持
- 通用的 rust i18n
- 支持从 toml 文件读取多语言设置
- 充分考虑服务于 anycms-rs 生态 ../
- 考虑集成 anycms-config 加载配置


## 基本需求
- 自动加载多语言配置
- 提供 i18n macro t!()

## web 框架集成
- 支持 actix / axum 框架
- 支持给前端返回 i18n 设置