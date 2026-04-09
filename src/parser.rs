use crate::catalog::{DataTypeInfo, DATA_TYPES};

static CONFIG_ITEM_PATTERN: &str = r"^\s*(?:export\s+)?[A-Za-z_][A-Za-z0-9_.\-]*\s*=";
static DIVIDER_PATTERN: &str = r"^\s*#\s*(?:---+|===+)(?:\s|$)";
static DECORATOR_PATTERN: &str = r"@([A-Za-z][\w\-]*)";

use std::collections::{HashMap, HashSet};

pub struct LineDocument {
    lines: Vec<String>,
}

impl LineDocument {
    pub fn new(text: &str) -> Self {
        Self {
            lines: text.lines().map(|l| l.to_string()).collect(),
        }
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn line_at(&self, line: usize) -> &str {
        self.lines.get(line).map(|s| s.as_str()).unwrap_or("")
    }
}

fn is_config_item(text: &str) -> bool {
    regex_lite::Regex::new(CONFIG_ITEM_PATTERN)
        .unwrap()
        .is_match(text)
}

fn is_divider(text: &str) -> bool {
    regex_lite::Regex::new(DIVIDER_PATTERN)
        .unwrap()
        .is_match(text)
}

pub fn strip_inline_comment(value: &str) -> &str {
    let bytes = value.as_bytes();
    let mut quote: Option<u8> = None;

    for i in 0..bytes.len() {
        let ch = bytes[i];
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            }
            continue;
        }

        if ch == b'"' || ch == b'\'' {
            quote = Some(ch);
            continue;
        }

        if ch == b'#' && (i == 0 || bytes[i - 1].is_ascii_whitespace()) {
            return value[..i].trim_end();
        }
    }

    value.trim_end()
}

pub fn get_decorator_comment_prefix(line_text: &str) -> Option<&str> {
    let trimmed = line_text.trim_start();
    if !trimmed.starts_with('#') {
        return None;
    }
    let after_hash = trimmed[1..].trim_start();
    if !after_hash.starts_with('@') {
        return None;
    }
    Some(strip_inline_comment(after_hash))
}

pub fn is_in_header(doc: &LineDocument, line_number: usize) -> bool {
    let line_text = doc.line_at(line_number);
    let trimmed = line_text.trim();
    if !trimmed.starts_with('#') {
        return false;
    }

    for line in (line_number + 1)..doc.line_count() {
        let text = doc.line_at(line).trim().to_string();
        if text.is_empty() {
            break;
        }
        if is_divider(&text) {
            break;
        }
        if is_config_item(&text) {
            return false;
        }
        if !text.starts_with('#') {
            break;
        }
    }

    for line in 0..line_number {
        if is_config_item(doc.line_at(line)) {
            return false;
        }
    }

    true
}

pub fn get_existing_decorator_names(
    doc: &LineDocument,
    line_number: usize,
    comment_prefix: &str,
) -> HashSet<String> {
    let mut names = HashSet::new();
    let re = regex_lite::Regex::new(DECORATOR_PATTERN).unwrap();

    if is_in_header(doc, line_number) {
        for line in 0..line_number {
            let text = doc.line_at(line).trim();
            if is_config_item(text) {
                break;
            }
            if !text.starts_with('#') {
                continue;
            }
            if let Some(prefix) = get_decorator_comment_prefix(doc.line_at(line)) {
                for cap in re.captures_iter(prefix) {
                    names.insert(cap[1].to_string());
                }
            }
        }
    } else {
        for line in (0..line_number).rev() {
            let text = doc.line_at(line).trim();
            if !text.starts_with('#') {
                break;
            }
            if let Some(prefix) = get_decorator_comment_prefix(doc.line_at(line)) {
                for cap in re.captures_iter(prefix) {
                    names.insert(cap[1].to_string());
                }
            }
        }
    }

    for cap in re.captures_iter(comment_prefix) {
        names.insert(cap[1].to_string());
    }

    names
}

static INCOMPATIBLE_DECORATORS: &[(&str, &[&str])] = &[
    ("required", &["optional"]),
    ("optional", &["required"]),
    ("sensitive", &["public"]),
    ("public", &["sensitive"]),
];

use crate::catalog::DecoratorInfo;

pub fn filter_available_decorators<'a>(
    decorators: &[&'a DecoratorInfo],
    existing_names: &HashSet<String>,
) -> Vec<&'a DecoratorInfo> {
    decorators
        .iter()
        .filter(|decorator| {
            if !decorator.is_function && existing_names.contains(decorator.name) {
                return false;
            }

            if let Some(incompatible) = INCOMPATIBLE_DECORATORS
                .iter()
                .find(|(name, _)| *name == decorator.name)
                .map(|(_, names)| *names)
            {
                if incompatible.iter().any(|n| existing_names.contains(*n)) {
                    return false;
                }
            }

            true
        })
        .copied()
        .collect()
}

pub fn split_enum_args(input: &str) -> Vec<String> {
    split_comma_args(input)
        .into_iter()
        .map(|v| v.trim_matches(|c| c == '\'' || c == '"').trim().to_string())
        .filter(|v| !v.is_empty())
        .collect()
}

pub fn split_comma_args(input: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut depth: usize = 0;

    for ch in input.chars() {
        if let Some(q) = quote {
            current.push(ch);
            if ch == q {
                quote = None;
            }
            continue;
        }

        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            current.push(ch);
            continue;
        }

        if ch == '(' {
            depth += 1;
            current.push(ch);
            continue;
        }

        if ch == ')' {
            depth = depth.saturating_sub(1);
            current.push(ch);
            continue;
        }

        if ch == ',' && depth == 0 {
            let value = current.trim().to_string();
            if !value.is_empty() {
                parts.push(value);
            }
            current.clear();
            continue;
        }

        current.push(ch);
    }

    let value = current.trim().to_string();
    if !value.is_empty() {
        parts.push(value);
    }
    parts
}

pub fn get_enum_values_from_preceding_comments(
    doc: &LineDocument,
    line_number: usize,
) -> Option<Vec<String>> {
    let re = regex_lite::Regex::new(r"@type=enum\((.*)\)").unwrap();

    for line in (0..line_number).rev() {
        let text = doc.line_at(line).trim();
        if !text.starts_with('#') {
            break;
        }

        if let Some(prefix) = get_decorator_comment_prefix(doc.line_at(line)) {
            if let Some(caps) = re.captures(prefix) {
                return Some(split_enum_args(&caps[1]));
            }
        }
    }

    None
}

pub fn get_type_option_data_type(comment_prefix: &str) -> Option<&'static DataTypeInfo> {
    let re = regex_lite::Regex::new(r"(^|\s)@type=([A-Za-z][\w\-]*)\([^#)]*$").unwrap();
    let caps = re.captures(comment_prefix)?;
    let type_name = caps.get(2)?.as_str();
    DATA_TYPES.iter().find(|dt| dt.name == type_name)
}

pub fn get_preceding_comment_block(doc: &LineDocument, line_number: usize) -> Vec<String> {
    let mut lines = Vec::new();

    for line in (0..line_number).rev() {
        let text = doc.line_at(line).trim().to_string();
        if !text.starts_with('#') {
            break;
        }
        lines.push(text);
    }

    lines.reverse();
    lines
}

#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub name: String,
    pub args: Vec<String>,
    pub options: HashMap<String, String>,
}

pub fn parse_type_options(input: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();

    for part in split_comma_args(input) {
        if let Some(sep) = part.find('=') {
            let key = part[..sep].trim().to_string();
            let raw_value = part[sep + 1..].trim().to_string();
            if key.is_empty() {
                continue;
            }
            let value = raw_value
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .or_else(|| {
                    raw_value
                        .strip_prefix('\'')
                        .and_then(|s| s.strip_suffix('\''))
                })
                .unwrap_or(&raw_value)
                .to_string();
            result.insert(key, value);
        }
    }

    result
}

pub fn get_type_info_from_preceding_comments(
    doc: &LineDocument,
    line_number: usize,
) -> Option<TypeInfo> {
    let comment_block = get_preceding_comment_block(doc, line_number);
    let re = regex_lite::Regex::new(r"@type=([A-Za-z][\w\-]*)(?:\((.*)\))?").unwrap();

    for text in comment_block.iter().rev() {
        let trimmed = text.trim();
        if !trimmed.starts_with('#') {
            continue;
        }

        let prefix = get_decorator_comment_prefix(trimmed)?;
        let caps = re.captures(prefix)?;

        let type_name = caps[1].to_string();
        if type_name == "enum" {
            return Some(TypeInfo {
                name: type_name,
                args: split_enum_args(caps.get(2).map(|m| m.as_str()).unwrap_or("")),
                options: HashMap::new(),
            });
        }

        return Some(TypeInfo {
            name: type_name,
            args: Vec::new(),
            options: parse_type_options(caps.get(2).map(|m| m.as_str()).unwrap_or("")),
        });
    }

    None
}

pub fn unquote(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| value.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
        .unwrap_or(value)
}

pub fn is_dynamic_value(value: &str) -> bool {
    let re1 = regex_lite::Regex::new(r"\$[A-Za-z_]").unwrap();
    let re2 = regex_lite::Regex::new(r"^[A-Za-z][\w\-]*\(").unwrap();
    re1.is_match(value) || re2.is_match(value)
}

#[derive(Debug, Clone)]
pub struct DecoratorOccurrence {
    pub name: String,
    pub line: usize,
    pub start: usize,
    pub end: usize,
    pub is_function_call: bool,
}

pub fn get_decorator_occurrences(line_text: &str, line_number: usize) -> Vec<DecoratorOccurrence> {
    let mut occurrences = Vec::new();
    let decorator_comment = match get_decorator_comment_prefix(line_text) {
        Some(c) => c,
        None => return occurrences,
    };

    let comment_start = line_text.find(decorator_comment).unwrap_or(0);
    let re = regex_lite::Regex::new(r"@([A-Za-z][\w\-]*)(?:\([^)]*\)|=[^\s#]+)?").unwrap();

    for caps in re.captures_iter(decorator_comment) {
        let full = caps.get(0).unwrap();
        let name = caps[1].to_string();
        let start = comment_start + full.start();
        let end = comment_start + full.end();
        let suffix = &full.as_str()[name.len() + 1..];
        let is_function_call = suffix.starts_with('(');

        occurrences.push(DecoratorOccurrence {
            name,
            line: line_number,
            start,
            end,
            is_function_call,
        });
    }

    occurrences
}
