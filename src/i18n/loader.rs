//! FTL file loader

use std::collections::HashMap;

/// FTL file names to load (in order)
const FTL_FILES: &[&str] = &[
    "common.ftl",
    "menu.ftl",
    "cli.ftl",
    "errors.ftl",
    "help.ftl",
];

/// Load all FTL files for a given language
pub fn load_ftl_files(lang: &str) -> Vec<(String, String)> {
    let mut files = Vec::new();

    for name in FTL_FILES {
        if let Some(content) = load_ftl_file(lang, name) {
            files.push((name.to_string(), content));
        }
    }

    files
}

/// Load a single FTL file
fn load_ftl_file(lang: &str, name: &str) -> Option<String> {
    // Try to load from filesystem first (for development)
    let fs_content = load_from_filesystem(lang, name);
    if fs_content.is_some() {
        return fs_content;
    }

    // Fall back to embedded content (for release builds)
    load_embedded(lang, name)
}

/// Load FTL file from filesystem
fn load_from_filesystem(lang: &str, name: &str) -> Option<String> {
    // Try multiple possible paths
    let paths = [
        format!("i18n/{}/{}", lang, name),
        format!("./i18n/{}/{}", lang, name),
    ];

    for path in &paths {
        if let Ok(content) = std::fs::read_to_string(path) {
            return Some(content);
        }
    }

    None
}

/// Load embedded FTL content (compiled into binary)
fn load_embedded(lang: &str, name: &str) -> Option<String> {
    let key = format!("{}/{}", lang, name);

    // Embedded FTL files
    let embedded: HashMap<&str, &str> = get_embedded_files();

    embedded.get(&key.as_str()).map(|s| s.to_string())
}

/// Get embedded FTL file contents
fn get_embedded_files() -> HashMap<&'static str, &'static str> {
    let mut files = HashMap::new();

    // English files
    files.insert("en/common.ftl", include_str!("../../i18n/en/common.ftl"));
    files.insert("en/menu.ftl", include_str!("../../i18n/en/menu.ftl"));
    files.insert("en/cli.ftl", include_str!("../../i18n/en/cli.ftl"));
    files.insert("en/errors.ftl", include_str!("../../i18n/en/errors.ftl"));
    files.insert("en/help.ftl", include_str!("../../i18n/en/help.ftl"));

    // Chinese (Simplified) files
    files.insert("zh-CN/common.ftl", include_str!("../../i18n/zh-CN/common.ftl"));
    files.insert("zh-CN/menu.ftl", include_str!("../../i18n/zh-CN/menu.ftl"));
    files.insert("zh-CN/cli.ftl", include_str!("../../i18n/zh-CN/cli.ftl"));
    files.insert("zh-CN/errors.ftl", include_str!("../../i18n/zh-CN/errors.ftl"));
    files.insert("zh-CN/help.ftl", include_str!("../../i18n/zh-CN/help.ftl"));

    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_embedded() {
        let files = load_ftl_files("en");
        assert!(!files.is_empty());

        // Check common.ftl is loaded
        let common = files.iter().find(|(name, _)| name == "common.ftl");
        assert!(common.is_some());
        assert!(common.unwrap().1.contains("status-success"));
    }

    #[test]
    fn test_load_zh_cn() {
        let files = load_ftl_files("zh-CN");
        assert!(!files.is_empty());

        // Check Chinese content is loaded
        let common = files.iter().find(|(name, _)| name == "common.ftl");
        assert!(common.is_some());
        assert!(common.unwrap().1.contains("成功"));
    }
}