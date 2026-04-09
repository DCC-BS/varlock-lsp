use crate::catalog::DECORATORS_BY_NAME;
use crate::parser::*;
use tower_lsp::lsp_types::*;

#[derive(Debug, Clone)]
struct CoreDiagnostic {
    line: usize,
    start: usize,
    end: usize,
    message: String,
}

fn to_range(diag: &CoreDiagnostic) -> Range {
    Range::new(
        Position::new(diag.line as u32, diag.start as u32),
        Position::new(diag.line as u32, diag.end as u32),
    )
}

static INCOMPATIBLE_DECORATOR_PAIRS: &[(&str, &str)] =
    &[("required", "optional"), ("sensitive", "public")];

fn create_decorator_diagnostics(occurrences: &[DecoratorOccurrence]) -> Vec<CoreDiagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut reported_ranges: std::collections::HashSet<String> = std::collections::HashSet::new();

    for occ in occurrences {
        let count = seen_counts.entry(occ.name.clone()).or_insert(0);
        *count += 1;

        let decorator = DECORATORS_BY_NAME.get(occ.name.as_str());
        let is_repeatable_function = decorator
            .map(|d| d.is_function)
            .unwrap_or(occ.is_function_call);

        if !is_repeatable_function && *count > 1 {
            diagnostics.push(CoreDiagnostic {
                line: occ.line,
                start: occ.start,
                end: occ.end,
                message: format!(
                    "@{} can only be used once in the same decorator block.",
                    occ.name
                ),
            });
        }
    }

    for (left, right) in INCOMPATIBLE_DECORATOR_PAIRS {
        let has_left = occurrences.iter().any(|o| o.name == *left);
        let has_right = occurrences.iter().any(|o| o.name == *right);
        if !has_left || !has_right {
            continue;
        }

        let conflicting: Vec<&DecoratorOccurrence> = occurrences
            .iter()
            .filter(|o| o.name == *left || o.name == *right)
            .collect();

        for occ in conflicting {
            let range_key = format!("{}:{}:{}", occ.line, occ.start, occ.end);
            if reported_ranges.contains(&range_key) {
                continue;
            }
            reported_ranges.insert(range_key);

            diagnostics.push(CoreDiagnostic {
                line: occ.line,
                start: occ.start,
                end: occ.end,
                message: format!("@{} and @{} cannot be used together.", left, right),
            });
        }
    }

    diagnostics
}

fn validate_string_value(
    value: &str,
    options: &std::collections::HashMap<String, String>,
) -> Option<String> {
    let allow_empty = options
        .get("allowEmpty")
        .map(|v| v == "true")
        .unwrap_or(false);
    if !allow_empty && value.is_empty() {
        return Some("Value cannot be empty.".to_string());
    }

    if let Some(min) = options.get("minLength") {
        if value.len() < min.parse::<usize>().unwrap_or(0) {
            return Some(format!("Value must be at least {} characters long.", min));
        }
    }

    if let Some(max) = options.get("maxLength") {
        if value.len() > max.parse::<usize>().unwrap_or(usize::MAX) {
            return Some(format!("Value must be at most {} characters long.", max));
        }
    }

    if let Some(is_len) = options.get("isLength") {
        if value.len() != is_len.parse::<usize>().unwrap_or(0) {
            return Some(format!("Value must be exactly {} characters long.", is_len));
        }
    }

    if let Some(starts_with) = options.get("startsWith") {
        if !value.starts_with(starts_with.as_str()) {
            return Some(format!("Value must start with `{}`.", starts_with));
        }
    }

    if let Some(ends_with) = options.get("endsWith") {
        if !value.ends_with(ends_with.as_str()) {
            return Some(format!("Value must end with `{}`.", ends_with));
        }
    }

    if let Some(matches) = options.get("matches") {
        if matches.len() > 200 {
            return None;
        }
        if let Ok(re) = regex_lite::Regex::new(matches) {
            if !re.is_match(value) {
                return Some(format!("Value must match `{}`.", matches));
            }
        }
    }

    None
}

fn validate_number_value(
    value: &str,
    options: &std::collections::HashMap<String, String>,
) -> Option<String> {
    let numeric_value: f64 = value.parse().ok()?;
    if !numeric_value.is_finite() {
        return Some("Value must be a valid number.".to_string());
    }

    if let Some(min) = options.get("min") {
        let min_val: f64 = min.parse().unwrap_or(f64::MIN);
        if numeric_value < min_val {
            return Some(format!("Value must be greater than or equal to {}.", min));
        }
    }

    if let Some(max) = options.get("max") {
        let max_val: f64 = max.parse().unwrap_or(f64::MAX);
        if numeric_value > max_val {
            return Some(format!("Value must be less than or equal to {}.", max));
        }
    }

    if options.get("isInt").map(|v| v == "true").unwrap_or(false) {
        if numeric_value.fract() != 0.0 {
            return Some("Value must be an integer.".to_string());
        }
    }

    if let Some(div) = options.get("isDivisibleBy") {
        let div_val: f64 = div.parse().unwrap_or(1.0);
        if div_val != 0.0 && (numeric_value % div_val).abs() > f64::EPSILON {
            return Some(format!("Value must be divisible by {}.", div));
        }
    }

    if let Some(prec) = options.get("precision") {
        let prec_val: usize = prec.parse().unwrap_or(0);
        let decimals: Vec<&str> = value.split('.').collect();
        if decimals.len() > 1 && decimals[1].len() > prec_val {
            return Some(format!("Value must have at most {} decimal places.", prec));
        }
    }

    None
}

fn validate_url_value(
    value: &str,
    options: &std::collections::HashMap<String, String>,
) -> Option<String> {
    let prepend_https = options
        .get("prependHttps")
        .map(|v| v == "true")
        .unwrap_or(false);
    let has_protocol = value.starts_with("http://") || value.starts_with("https://");

    if prepend_https && has_protocol {
        return Some("URL should omit the protocol when prependHttps=true.".to_string());
    }

    if !prepend_https && !has_protocol {
        return Some("URL must include a protocol unless prependHttps=true.".to_string());
    }

    let url_str = if prepend_https {
        format!("https://{}", value)
    } else {
        value.to_string()
    };

    let url = url::Url::parse(&url_str).ok()?;

    if let Some(domains_str) = options.get("allowedDomains") {
        let allowed: Vec<String> = split_enum_args(domains_str);
        if !allowed.is_empty() {
            let host = url.host_str().unwrap_or("").to_lowercase();
            if !allowed.iter().any(|d| d.to_lowercase() == host) {
                return Some(format!("URL host must be one of: {}.", allowed.join(", ")));
            }
        }
    }

    None
}

fn is_ip_v(value: &str) -> u8 {
    if value.parse::<std::net::Ipv4Addr>().is_ok() {
        return 4;
    }
    if value.parse::<std::net::Ipv6Addr>().is_ok() {
        return 6;
    }
    0
}

fn validate_static_value(type_info: &TypeInfo, value: &str) -> Option<String> {
    match type_info.name.as_str() {
        "string" => validate_string_value(value, &type_info.options),
        "number" => validate_number_value(value, &type_info.options),
        "boolean" => {
            let re = regex_lite::Regex::new(r"(?i)^(true|false|1|0|yes|no|on|off)$").unwrap();
            if re.is_match(value) {
                None
            } else {
                Some("Value must be a boolean.".to_string())
            }
        }
        "email" => {
            let re = regex_lite::Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$").unwrap();
            if re.is_match(value) {
                None
            } else {
                Some("Value must be a valid email address.".to_string())
            }
        }
        "url" => validate_url_value(value, &type_info.options),
        "ip" => {
            let version = type_info
                .options
                .get("version")
                .and_then(|v| v.parse::<u8>().ok());
            let detected = is_ip_v(value);
            if detected == 0 {
                return Some("Value must be a valid IPv4 or IPv6 address.".to_string());
            }
            if let Some(v) = version {
                if (v == 4 || v == 6) && detected != v {
                    return Some(format!("Value must be a valid IPv{} address.", v));
                }
            }
            None
        }
        "port" => {
            let numeric_value: f64 = value.parse().ok()?;
            if numeric_value < 0.0 || numeric_value > 65535.0 || numeric_value.fract() != 0.0 {
                return Some("Value must be a valid port number.".to_string());
            }
            if let Some(min) = type_info.options.get("min") {
                let min_val: f64 = min.parse().unwrap_or(0.0);
                if numeric_value < min_val {
                    return Some(format!("Port must be greater than or equal to {}.", min));
                }
            }
            if let Some(max) = type_info.options.get("max") {
                let max_val: f64 = max.parse().unwrap_or(65535.0);
                if numeric_value > max_val {
                    return Some(format!("Port must be less than or equal to {}.", max));
                }
            }
            None
        }
        "semver" => {
            let re = regex_lite::Regex::new(
                r"^\d+\.\d+\.\d+(?:-[0-9A-Za-z.\-]+)?(?:\+[0-9A-Za-z.\-]+)?$",
            )
            .unwrap();
            if re.is_match(value) {
                None
            } else {
                Some("Value must be a valid semantic version.".to_string())
            }
        }
        "isoDate" => {
            let re = regex_lite::Regex::new(r"^\d{4}-\d{2}-\d{2}(?:[T ][0-9:.+\-Z]*)?$").unwrap();
            if re.is_match(value) && value.parse::<chrono::NaiveDateTime>().is_ok()
                || re.is_match(value)
            {
                None
            } else {
                Some("Value must be a valid ISO date.".to_string())
            }
        }
        "uuid" => {
            let re = regex_lite::Regex::new(
                r"(?i)^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$",
            )
            .unwrap();
            if re.is_match(value) {
                None
            } else {
                Some("Value must be a valid UUID.".to_string())
            }
        }
        "md5" => {
            let re = regex_lite::Regex::new(r"(?i)^[0-9a-f]{32}$").unwrap();
            if re.is_match(value) {
                None
            } else {
                Some("Value must be a valid MD5 hash.".to_string())
            }
        }
        "enum" => {
            if type_info.args.iter().any(|a| a == value) {
                None
            } else {
                Some(format!(
                    "Value must be one of: {}.",
                    type_info.args.join(", ")
                ))
            }
        }
        _ => None,
    }
}

static ENV_ASSIGNMENT_PATTERN: &str = r"^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.*?)\s*$";

fn flush_decorator_block(block: &mut Vec<DecoratorOccurrence>, diagnostics: &mut Vec<Diagnostic>) {
    if block.is_empty() {
        return;
    }
    let core_diags = create_decorator_diagnostics(block);
    for cd in core_diags {
        diagnostics.push(Diagnostic::new(
            to_range(&cd),
            Some(DiagnosticSeverity::ERROR),
            None,
            None,
            cd.message,
            None,
            None,
        ));
    }
    block.clear();
}

pub fn validate_document(doc: &LineDocument) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let env_re = regex_lite::Regex::new(ENV_ASSIGNMENT_PATTERN).unwrap();

    let mut header_decorator_block: Vec<DecoratorOccurrence> = Vec::new();
    let mut decorator_block: Vec<DecoratorOccurrence> = Vec::new();
    let mut has_seen_config_item = false;

    for line_number in 0..doc.line_count() {
        let line_text = doc.line_at(line_number);
        let trimmed = line_text.trim();

        if trimmed.starts_with('#') {
            let scope = if !has_seen_config_item && is_in_header(doc, line_number) {
                "header"
            } else {
                "item"
            };

            let occurrences = get_decorator_occurrences(line_text, line_number);
            if scope == "header" {
                header_decorator_block.extend(occurrences);
            } else {
                decorator_block.extend(occurrences);
            }
        } else if trimmed.is_empty() && !has_seen_config_item {
            continue;
        } else {
            flush_decorator_block(&mut header_decorator_block, &mut diagnostics);
            flush_decorator_block(&mut decorator_block, &mut diagnostics);
        }

        let caps = match env_re.captures(line_text) {
            Some(c) => c,
            None => continue,
        };
        has_seen_config_item = true;

        let raw_value = strip_inline_comment(caps.get(2).unwrap().as_str());
        if raw_value.is_empty() {
            continue;
        }
        if is_dynamic_value(raw_value) {
            continue;
        }

        let type_info = match get_type_info_from_preceding_comments(doc, line_number) {
            Some(t) => t,
            None => continue,
        };

        let unquoted = unquote(raw_value);
        let message = match validate_static_value(&type_info, unquoted) {
            Some(m) => m,
            None => continue,
        };

        let value_start = line_text.find(raw_value).unwrap_or(0);
        diagnostics.push(Diagnostic::new(
            Range::new(
                Position::new(line_number as u32, value_start as u32),
                Position::new(line_number as u32, (value_start + raw_value.len()) as u32),
            ),
            Some(DiagnosticSeverity::ERROR),
            None,
            None,
            message,
            None,
            None,
        ));
    }

    flush_decorator_block(&mut header_decorator_block, &mut diagnostics);
    flush_decorator_block(&mut decorator_block, &mut diagnostics);

    diagnostics
}
