
publish:
    cargo publish --registry crates-io -panycms-i18n-macro
    cargo publish --registry crates-io -panycms-i18n
    cargo publish --registry crates-io -panycms-i18n-sqlx
    cargo publish --registry crates-io -panycms-i18n-axum
    cargo publish --registry crates-io -panycms-i18n-actix

test:
    cargo test --workspace --all-features

clippy:
    cargo clippy --workspace --all-features -- -D warnings

release-patch:
    cargo release patch --no-publish --execute

release-minor:
    cargo release minor --no-publish --execute

release-major:
    cargo release major --no-publish --execute
