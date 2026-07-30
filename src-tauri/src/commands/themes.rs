use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

const MAX_INCLUDE_DEPTH: usize = 12;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VsCodeThemeSummary {
    pub id: String,
    pub label: String,
    pub extension_name: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedVsCodeTheme {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub colors: HashMap<String, String>,
    pub token_colors: HashMap<String, String>,
}

#[derive(Debug, Clone)]
struct ThemeDescriptor {
    summary: VsCodeThemeSummary,
    path: PathBuf,
    extension_root: PathBuf,
}

#[tauri::command]
pub fn list_vscode_themes() -> Vec<VsCodeThemeSummary> {
    discover_themes()
        .into_iter()
        .map(|theme| theme.summary)
        .collect()
}

#[tauri::command]
pub fn load_vscode_theme(theme_id: String) -> Result<ResolvedVsCodeTheme, String> {
    let descriptor = discover_themes()
        .into_iter()
        .find(|theme| theme.summary.id == theme_id)
        .ok_or_else(|| "The selected VS Code theme is no longer installed".to_string())?;

    let mut colors = HashMap::new();
    let mut token_colors = HashMap::new();
    let mut visited = HashSet::new();
    resolve_theme_file(
        &descriptor.path,
        &descriptor.extension_root,
        0,
        &mut visited,
        &mut colors,
        &mut token_colors,
    )?;

    Ok(ResolvedVsCodeTheme {
        id: descriptor.summary.id,
        label: descriptor.summary.label,
        kind: descriptor.summary.kind,
        colors,
        token_colors,
    })
}

fn extension_roots() -> Vec<PathBuf> {
    let Some(home) = std::env::var_os("USERPROFILE").map(PathBuf::from) else {
        return Vec::new();
    };
    [".vscode", ".vscode-insiders", ".vscode-oss", ".cursor"]
        .into_iter()
        .map(|directory| home.join(directory).join("extensions"))
        .filter(|path| path.is_dir())
        .collect()
}

fn discover_themes() -> Vec<ThemeDescriptor> {
    let mut themes = Vec::new();
    let mut ids = HashSet::new();
    for extensions_root in extension_roots() {
        let Ok(entries) = fs::read_dir(&extensions_root) else {
            continue;
        };
        for entry in entries.flatten() {
            let extension_root = entry.path();
            let package_path = extension_root.join("package.json");
            let Ok(raw) = fs::read_to_string(package_path) else {
                continue;
            };
            let Ok(package) = serde_json::from_str::<Value>(&raw) else {
                continue;
            };
            let Some(contributions) = package
                .pointer("/contributes/themes")
                .and_then(Value::as_array)
            else {
                continue;
            };
            let publisher = package
                .get("publisher")
                .and_then(Value::as_str)
                .unwrap_or("local");
            let name = package
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("theme");
            let extension_name = package
                .get("displayName")
                .and_then(Value::as_str)
                .unwrap_or(name);
            for contribution in contributions {
                let Some(relative_path) = contribution.get("path").and_then(Value::as_str) else {
                    continue;
                };
                let label = contribution
                    .get("label")
                    .and_then(Value::as_str)
                    .or_else(|| contribution.get("id").and_then(Value::as_str))
                    .unwrap_or(name);
                let local_id = contribution
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or(label);
                let id = format!("{publisher}.{name}:{local_id}");
                if !ids.insert(id.clone()) {
                    continue;
                }
                let path = extension_root.join(relative_path);
                if !path.is_file() || !path_is_inside(&path, &extension_root) {
                    continue;
                }
                themes.push(ThemeDescriptor {
                    summary: VsCodeThemeSummary {
                        id,
                        label: label.to_string(),
                        extension_name: extension_name.to_string(),
                        kind: normalize_theme_kind(
                            contribution
                                .get("uiTheme")
                                .and_then(Value::as_str)
                                .unwrap_or("vs-dark"),
                        ),
                    },
                    path,
                    extension_root: extension_root.clone(),
                });
            }
        }
    }
    themes.sort_by(|left, right| {
        left.summary
            .label
            .to_lowercase()
            .cmp(&right.summary.label.to_lowercase())
            .then_with(|| left.summary.id.cmp(&right.summary.id))
    });
    themes
}

fn normalize_theme_kind(ui_theme: &str) -> String {
    match ui_theme {
        "vs" | "hc-light" => "light",
        "hc-black" => "high-contrast",
        _ => "dark",
    }
    .to_string()
}

fn path_is_inside(path: &Path, root: &Path) -> bool {
    match (path.canonicalize(), root.canonicalize()) {
        (Ok(path), Ok(root)) => path.starts_with(root),
        _ => false,
    }
}

fn resolve_theme_file(
    path: &Path,
    extension_root: &Path,
    depth: usize,
    visited: &mut HashSet<PathBuf>,
    colors: &mut HashMap<String, String>,
    token_colors: &mut HashMap<String, String>,
) -> Result<(), String> {
    if depth > MAX_INCLUDE_DEPTH {
        return Err("VS Code theme include chain is too deep".to_string());
    }
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("Could not open VS Code theme: {error}"))?;
    let root = extension_root
        .canonicalize()
        .map_err(|error| format!("Could not access VS Code extension: {error}"))?;
    if !canonical.starts_with(&root) {
        return Err("VS Code theme include escapes its extension directory".to_string());
    }
    if !visited.insert(canonical.clone()) {
        return Ok(());
    }
    let raw = fs::read_to_string(&canonical)
        .map_err(|error| format!("Could not read VS Code theme: {error}"))?;
    let value: Value = json5::from_str(raw.trim_start_matches('\u{feff}'))
        .map_err(|error| format!("Could not parse VS Code theme: {error}"))?;

    if let Some(include) = value.get("include").and_then(Value::as_str) {
        let include_path = canonical.parent().unwrap_or(&root).join(include);
        resolve_theme_file(
            &include_path,
            &root,
            depth + 1,
            visited,
            colors,
            token_colors,
        )?;
    }

    if let Some(theme_colors) = value.get("colors").and_then(Value::as_object) {
        for (key, value) in theme_colors {
            if let Some(color) = value.as_str().filter(|color| is_css_color(color)) {
                colors.insert(key.clone(), color.to_string());
            }
        }
    }

    match value.get("tokenColors") {
        Some(Value::Array(rules)) => collect_token_colors(rules, token_colors),
        Some(Value::String(relative_path)) => {
            let token_path = canonical.parent().unwrap_or(&root).join(relative_path);
            if matches!(
                token_path
                    .extension()
                    .and_then(|extension| extension.to_str()),
                Some("json" | "jsonc")
            ) {
                collect_token_file(&token_path, &root, token_colors)?;
            }
        }
        _ => {}
    }
    if let Some(semantic_colors) = value.get("semanticTokenColors").and_then(Value::as_object) {
        collect_semantic_token_colors(semantic_colors, token_colors);
    }
    Ok(())
}

fn collect_semantic_token_colors(
    rules: &serde_json::Map<String, Value>,
    target: &mut HashMap<String, String>,
) {
    for (selector, value) in rules {
        let color = value
            .as_str()
            .or_else(|| value.get("foreground").and_then(Value::as_str));
        let Some(color) = color.filter(|color| is_css_color(color)) else {
            continue;
        };
        let token_type = selector
            .split(':')
            .next()
            .unwrap_or(selector)
            .split('.')
            .next()
            .unwrap_or(selector);
        let category = match token_type {
            "comment" => Some("comment"),
            "keyword" | "modifier" => Some("keyword"),
            "string" => Some("string"),
            "regexp" => Some("regexp"),
            "number" => Some("number"),
            "function" | "method" => Some("function"),
            "variable" | "parameter" => Some("variable"),
            "type" | "class" | "interface" | "enum" | "struct" | "typeParameter" | "namespace" => {
                Some("type")
            }
            "property" => Some("property"),
            "enumMember" | "event" | "label" => Some("literal"),
            "decorator" | "macro" => Some("meta"),
            "operator" => Some("operator"),
            _ => None,
        };
        if let Some(category) = category {
            target.insert(category.to_string(), color.to_string());
        }
    }
}

fn collect_token_file(
    path: &Path,
    extension_root: &Path,
    token_colors: &mut HashMap<String, String>,
) -> Result<(), String> {
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("Could not open token color file: {error}"))?;
    if !canonical.starts_with(extension_root) {
        return Err("Token color file escapes its extension directory".to_string());
    }
    let raw = fs::read_to_string(canonical)
        .map_err(|error| format!("Could not read token color file: {error}"))?;
    let value: Value = json5::from_str(raw.trim_start_matches('\u{feff}'))
        .map_err(|error| format!("Could not parse token color file: {error}"))?;
    if let Some(rules) = value
        .as_array()
        .or_else(|| value.get("tokenColors").and_then(Value::as_array))
        .or_else(|| value.get("settings").and_then(Value::as_array))
    {
        collect_token_colors(rules, token_colors);
    }
    Ok(())
}

fn collect_token_colors(rules: &[Value], target: &mut HashMap<String, String>) {
    const CATEGORIES: [(&str, &[&str]); 15] = [
        ("comment", &["comment"]),
        ("keyword", &["keyword", "storage", "control"]),
        ("string", &["string"]),
        ("regexp", &["string.regexp", "regexp"]),
        ("number", &["constant.numeric", "number"]),
        ("literal", &["constant.language", "constant.character"]),
        ("function", &["entity.name.function", "support.function"]),
        ("variable", &["variable", "identifier"]),
        (
            "type",
            &["entity.name.type", "support.type", "storage.type"],
        ),
        (
            "property",
            &["variable.other.property", "support.type.property-name"],
        ),
        ("attribute", &["entity.other.attribute-name"]),
        ("tag", &["entity.name.tag"]),
        (
            "selector",
            &[
                "entity.other.attribute-name.class",
                "entity.other.attribute-name.id",
            ],
        ),
        (
            "meta",
            &[
                "meta.preprocessor",
                "keyword.control.import",
                "keyword.control.export",
            ],
        ),
        ("operator", &["keyword.operator"]),
    ];
    for rule in rules {
        let Some(foreground) = rule
            .pointer("/settings/foreground")
            .and_then(Value::as_str)
            .filter(|color| is_css_color(color))
        else {
            continue;
        };
        let scopes: Vec<&str> = match rule.get("scope") {
            Some(Value::String(scope)) => scope.split(',').map(str::trim).collect(),
            Some(Value::Array(scopes)) => scopes.iter().filter_map(Value::as_str).collect(),
            _ => Vec::new(),
        };
        for (category, needles) in CATEGORIES {
            if scopes
                .iter()
                .any(|scope| needles.iter().any(|needle| scope.contains(needle)))
            {
                target.insert(category.to_string(), foreground.to_string());
            }
        }
    }
}

fn is_css_color(value: &str) -> bool {
    let value = value.trim();
    (value.starts_with('#') && matches!(value.len(), 4 | 5 | 7 | 9))
        || value.starts_with("rgb(")
        || value.starts_with("rgba(")
        || value == "transparent"
}

#[cfg(test)]
mod tests {
    use super::{
        collect_semantic_token_colors, collect_token_colors, normalize_theme_kind,
        resolve_theme_file,
    };
    use std::collections::{HashMap, HashSet};
    use std::fs;

    #[test]
    fn normalizes_vscode_ui_theme_kinds() {
        assert_eq!(normalize_theme_kind("vs"), "light");
        assert_eq!(normalize_theme_kind("vs-dark"), "dark");
        assert_eq!(normalize_theme_kind("hc-black"), "high-contrast");
    }

    #[test]
    fn extracts_semantic_syntax_colors() {
        let rules = serde_json::json!([{
            "scope": ["comment.line", "comment.block"],
            "settings": { "foreground": "#667788" }
        }, {
            "scope": "entity.name.function",
            "settings": { "foreground": "#112233" }
        }]);
        let mut colors = HashMap::new();
        collect_token_colors(rules.as_array().unwrap(), &mut colors);
        assert_eq!(colors.get("comment").map(String::as_str), Some("#667788"));
        assert_eq!(colors.get("function").map(String::as_str), Some("#112233"));
    }

    #[test]
    fn semantic_tokens_override_matching_syntax_categories() {
        let rules = serde_json::json!({
            "class.declaration": "#778899",
            "method:java": { "foreground": "#aabbcc", "bold": true }
        });
        let mut colors = HashMap::new();
        collect_semantic_token_colors(rules.as_object().unwrap(), &mut colors);
        assert_eq!(colors.get("type").map(String::as_str), Some("#778899"));
        assert_eq!(colors.get("function").map(String::as_str), Some("#aabbcc"));
    }

    #[test]
    fn resolves_jsonc_include_with_child_overrides() {
        let root = std::env::temp_dir().join(format!("aaai-theme-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("base.json"),
            r##"{ // JSONC comment
                "colors": { "editor.background": "#111111", "editor.foreground": "#aaaaaa", },
            }"##,
        )
        .unwrap();
        fs::write(
            root.join("theme.json"),
            r##"{
                "include": "./base.json",
                "colors": { "editor.foreground": "#eeeeee" },
            }"##,
        )
        .unwrap();

        let mut colors = HashMap::new();
        resolve_theme_file(
            &root.join("theme.json"),
            &root,
            0,
            &mut HashSet::new(),
            &mut colors,
            &mut HashMap::new(),
        )
        .unwrap();

        assert_eq!(
            colors.get("editor.background").map(String::as_str),
            Some("#111111")
        );
        assert_eq!(
            colors.get("editor.foreground").map(String::as_str),
            Some("#eeeeee")
        );
        fs::remove_dir_all(root).unwrap();
    }
}
