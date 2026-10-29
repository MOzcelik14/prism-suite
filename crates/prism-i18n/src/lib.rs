use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};
use std::sync::Arc;
use unic_langid::{langid, LanguageIdentifier};

pub const LANG_EN: LanguageIdentifier = langid!("en");
pub const LANG_TR: LanguageIdentifier = langid!("tr");

const EN_FTL: &str = include_str!("../locales/en.ftl");
const TR_FTL: &str = include_str!("../locales/tr.ftl");

/// Supported languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    English,
    Turkish,
}

impl Language {
    pub fn as_code(&self) -> &'static str {
        match self {
            Language::English => "en",
            Language::Turkish => "tr",
        }
    }

    pub fn from_code(code: &str) -> Self {
        if code.starts_with("tr") {
            Language::Turkish
        } else {
            Language::English
        }
    }

    pub fn detect_system() -> Self {
        if let Some(locale) = sys_locale::get_locale() {
            Self::from_code(&locale.to_lowercase())
        } else {
            Language::English
        }
    }
}

/// Thread-safe internationalization manager.
pub struct I18n {
    current_lang: Language,
    en_bundle: FluentBundle<Arc<FluentResource>>,
    tr_bundle: FluentBundle<Arc<FluentResource>>,
}

impl I18n {
    /// Creates a new I18n instance with the specified or detected language.
    pub fn new(initial_lang: Option<Language>) -> Self {
        let lang = initial_lang.unwrap_or_else(Language::detect_system);

        let en_res = Arc::new(FluentResource::try_new(EN_FTL.to_string()).expect("valid en.ftl"));
        let mut en_bundle = FluentBundle::new(vec![LANG_EN]);
        en_bundle
            .add_resource(en_res)
            .expect("failed to add en.ftl resource");

        let tr_res = Arc::new(FluentResource::try_new(TR_FTL.to_string()).expect("valid tr.ftl"));
        let mut tr_bundle = FluentBundle::new(vec![LANG_TR]);
        tr_bundle
            .add_resource(tr_res)
            .expect("failed to add tr.ftl resource");

        Self {
            current_lang: lang,
            en_bundle,
            tr_bundle,
        }
    }

    pub fn language(&self) -> Language {
        self.current_lang
    }

    pub fn set_language(&mut self, lang: Language) {
        self.current_lang = lang;
    }

    fn active_bundle(&self) -> &FluentBundle<Arc<FluentResource>> {
        match self.current_lang {
            Language::English => &self.en_bundle,
            Language::Turkish => &self.tr_bundle,
        }
    }

    /// Resolves a localized string by key. Falls back to English, then to the key itself.
    pub fn text(&self, key: &str) -> String {
        self.text_args(key, None)
    }

    /// Resolves a localized string by key with dynamic arguments.
    pub fn text_args(&self, key: &str, args: Option<&FluentArgs>) -> String {
        let bundle = self.active_bundle();
        if let Some(msg) = bundle.get_message(key) {
            if let Some(val) = msg.value() {
                let mut errors = vec![];
                let formatted = bundle.format_pattern(val, args, &mut errors);
                return formatted.to_string();
            }
        }

        // Fallback to English if active is Turkish
        if self.current_lang != Language::English {
            if let Some(msg) = self.en_bundle.get_message(key) {
                if let Some(val) = msg.value() {
                    let mut errors = vec![];
                    let formatted = self.en_bundle.format_pattern(val, args, &mut errors);
                    return formatted.to_string();
                }
            }
        }

        key.to_string()
    }
}

impl Default for I18n {
    fn default() -> Self {
        Self::new(None)
    }
}
