//! Internationalization (i18n) module using Fluent

mod loader;

use fluent::{FluentBundle, FluentResource};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use unic_langid::LanguageIdentifier;

/// Supported languages with display names
pub const SUPPORTED_LANGUAGES: &[(&str, &str)] = &[
    ("en", "English"),
    ("zh-CN", "简体中文"),
];

/// Default language
pub const DEFAULT_LANGUAGE: &str = "en";

/// Index of current language in SUPPORTED_LANGUAGES
static CURRENT_LANG_INDEX: AtomicUsize = AtomicUsize::new(0);

/// Get current language code
pub fn current_language() -> String {
    SUPPORTED_LANGUAGES[CURRENT_LANG_INDEX.load(Ordering::Relaxed)].0.to_string()
}

/// Get the display name for a language code
pub fn get_language_name(code: &str) -> &str {
    SUPPORTED_LANGUAGES.iter()
        .find(|(c, _)| *c == code)
        .map(|(_, name)| *name)
        .unwrap_or(code)
}

/// Set language by code
pub fn set_language(lang: &str) -> Result<(), String> {
    let normalized = normalize_lang(lang);
    let index = SUPPORTED_LANGUAGES.iter()
        .position(|(code, _)| *code == normalized)
        .unwrap_or(0);
    CURRENT_LANG_INDEX.store(index, Ordering::Relaxed);
    Ok(())
}

/// Normalize language code
fn normalize_lang(lang: &str) -> String {
    let lang = lang.replace('_', "-");

    // Direct match
    for (code, _) in SUPPORTED_LANGUAGES {
        if *code == lang {
            return lang;
        }
    }

    // Prefix match (e.g., "zh" -> "zh-CN")
    let prefix = lang.split('-').next().unwrap_or(&lang);
    for (code, _) in SUPPORTED_LANGUAGES {
        if code.starts_with(prefix) {
            return code.to_string();
        }
    }

    DEFAULT_LANGUAGE.to_string()
}

/// Create a bundle for the given language
fn create_bundle(lang: &str) -> Result<FluentBundle<FluentResource>, String> {
    let lang_id: LanguageIdentifier = lang.parse()
        .map_err(|e| format!("Invalid language identifier: {}", e))?;

    let mut bundle = FluentBundle::new(vec![lang_id]);

    let ftl_files = loader::load_ftl_files(lang);
    
    #[cfg(debug_assertions)]
    eprintln!("[DEBUG] create_bundle: lang={}, loaded {} files", lang, ftl_files.len());

    for (name, content) in ftl_files {
        #[cfg(debug_assertions)]
        eprintln!("[DEBUG] Processing FTL file: {} ({} bytes)", name, content.len());
        
        let resource = FluentResource::try_new(content)
            .map_err(|(_, errors)| {
                let error_msgs: Vec<String> = errors.iter().map(|e| format!("{:?}", e)).collect();
                format!("Failed to parse FTL file '{}': {}", name, error_msgs.join(", "))
            })?;

        bundle.add_resource(resource)
            .map_err(|e| format!("Failed to add resource '{}': {:?}", name, e))?;
    }

    Ok(bundle)
}

/// Initialize the global localization with the given language
pub fn init(lang: &str) -> Result<(), String> {
    set_language(lang)
}

/// Initialize localization with system language detection
pub fn init_with_system_lang() -> Result<(), String> {
    init(&detect_system_lang())
}

/// Check if localization is initialized (always true now)
pub fn is_initialized() -> bool {
    true
}

/// Detect system language
pub fn detect_system_lang() -> String {
    if let Some(locale) = sys_locale::get_locale() {
        let normalized = normalize_lang(&locale);
        for (code, _) in SUPPORTED_LANGUAGES {
            if *code == normalized {
                return normalized;
            }
        }
    }
    DEFAULT_LANGUAGE.to_string()
}

/// Translate a key to the current language
pub fn t(key: &str) -> String {
    #[cfg(debug_assertions)]
    eprintln!("[DEBUG] t() called with key: {}, current_lang: {}", key, current_language());
    
    let lang = current_language();

    match create_bundle(&lang) {
        Ok(bundle) => {
            #[cfg(debug_assertions)]
            eprintln!("[DEBUG] Bundle created successfully, looking for key: {}", key);
            
            let message = match bundle.get_message(key) {
                Some(msg) => {
                    #[cfg(debug_assertions)]
                    eprintln!("[DEBUG] Message found for key: {}", key);
                    msg
                }
                None => {
                    #[cfg(debug_assertions)]
                    eprintln!("[DEBUG] Message NOT found for key: {}", key);
                    return key.to_string();
                }
            };

            let value = match message.value() {
                Some(v) => v,
                None => {
                    #[cfg(debug_assertions)]
                    eprintln!("[DEBUG] Message has no value for key: {}", key);
                    return key.to_string();
                }
            };

            let mut errors = vec![];
            let result = bundle.format_pattern(value, None, &mut errors).to_string();
            #[cfg(debug_assertions)]
            eprintln!("[DEBUG] Translated '{}' -> '{}'", key, result);
            result
        }
        Err(e) => {
            #[cfg(debug_assertions)]
            eprintln!("[DEBUG] Failed to create bundle: {}", e);
            key.to_string()
        }
    }
}

/// Translate a key with arguments
pub fn t_args(key: &str, args: HashMap<&str, String>) -> String {
    let lang = current_language();

    match create_bundle(&lang) {
        Ok(bundle) => {
            let message = match bundle.get_message(key) {
                Some(msg) => msg,
                None => return key.to_string(),
            };

            let value = match message.value() {
                Some(v) => v,
                None => return key.to_string(),
            };

            let mut fluent_args = fluent::FluentArgs::new();
            for (k, v) in &args {
                fluent_args.set(*k, v.as_str());
            }

            let mut errors = vec![];
            bundle.format_pattern(value, Some(&fluent_args), &mut errors).to_string()
        }
        Err(_) => key.to_string(),
    }
}

/// Macro for convenient translation
#[macro_export]
macro_rules! t {
    ($key:expr) => { $crate::i18n::t($key) };
    ($key:expr, $($k:expr => $v:expr),+ $(,)?) => {{
        let mut args = std::collections::HashMap::new();
        $( args.insert($k, $v.to_string()); )+
        $crate::i18n::t_args($key, args)
    }};
}