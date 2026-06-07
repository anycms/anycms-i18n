CREATE TABLE IF NOT EXISTS i18n_translations (
    locale  TEXT NOT NULL,
    key     TEXT NOT NULL,
    value   TEXT NOT NULL,
    PRIMARY KEY (locale, key)
);
CREATE INDEX IF NOT EXISTS idx_i18n_translations_locale ON i18n_translations(locale);
