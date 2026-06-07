
publish:
    cargo publish --registry r404 -p anycms-i18n-macro
    cargo publish --registry r404 -p anycms-i18n
    cargo publish --registry r404 -p anycms-i18n-axum
    cargo publish --registry r404 -p anycms-i18n-actix

release-patch:
    cargo release patch --no-publish --execute

release-minor:
    cargo release minor --no-publish --execute

release-major:
    cargo release major --no-publish --execute
