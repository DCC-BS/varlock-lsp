use crate::catalog::*;
use crate::parser::*;
use tower_lsp::lsp_types::*;

fn build_markdown(summary: &str, documentation: &str, extra_line: Option<&str>) -> MarkupContent {
    let mut md = format!("{}\n\n{}", summary, documentation);
    if let Some(extra) = extra_line {
        md.push_str(&format!("\n\n{}", extra));
    }
    MarkupContent {
        kind: MarkupKind::Markdown,
        value: md,
    }
}

struct MatchContext {
    replace_range: Range,
}

fn create_decorator_item(info: &DecoratorInfo, ctx: &MatchContext) -> CompletionItem {
    CompletionItem {
        label: format!("@{}", info.name),
        kind: Some(CompletionItemKind::PROPERTY),
        text_edit: Some(CompletionTextEdit::Edit(TextEdit {
            range: ctx.replace_range,
            new_text: info.insert_text.to_string(),
        })),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        detail: Some(if info.scope == "root" {
            "Root decorator".to_string()
        } else {
            "Item decorator".to_string()
        }),
        documentation: Some(Documentation::MarkupContent(build_markdown(
            info.summary,
            info.documentation,
            info.deprecated,
        ))),
        deprecated: if info.deprecated.is_some() {
            Some(true)
        } else {
            None
        },
        sort_text: if info.deprecated.is_some() {
            Some(format!("z-{}", info.name))
        } else {
            None
        },
        ..Default::default()
    }
}

fn create_data_type_item(info: &DataTypeInfo, ctx: &MatchContext) -> CompletionItem {
    CompletionItem {
        label: info.name.to_string(),
        kind: Some(CompletionItemKind::CLASS),
        text_edit: Some(CompletionTextEdit::Edit(TextEdit {
            range: ctx.replace_range,
            new_text: info.insert_text.unwrap_or(info.name).to_string(),
        })),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        detail: Some("@type data type".to_string()),
        documentation: Some(Documentation::MarkupContent(build_markdown(
            info.summary,
            info.documentation,
            None,
        ))),
        ..Default::default()
    }
}

fn create_data_type_option_item(option: &DataTypeOption, ctx: &MatchContext) -> CompletionItem {
    CompletionItem {
        label: option.name.to_string(),
        kind: Some(CompletionItemKind::FIELD),
        text_edit: Some(CompletionTextEdit::Edit(TextEdit {
            range: ctx.replace_range,
            new_text: option.insert_text.to_string(),
        })),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        detail: Some("@type option".to_string()),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: option.documentation.to_string(),
        })),
        ..Default::default()
    }
}

fn create_resolver_item(info: &ResolverInfo, ctx: &MatchContext) -> CompletionItem {
    CompletionItem {
        label: format!("{}()", info.name),
        kind: Some(CompletionItemKind::FUNCTION),
        text_edit: Some(CompletionTextEdit::Edit(TextEdit {
            range: ctx.replace_range,
            new_text: info.insert_text.to_string(),
        })),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        detail: Some("Resolver function".to_string()),
        documentation: Some(Documentation::MarkupContent(build_markdown(
            info.summary,
            info.documentation,
            None,
        ))),
        ..Default::default()
    }
}

fn create_reference_items(doc: &LineDocument, ctx: &MatchContext) -> Vec<CompletionItem> {
    let env_key_re = regex_lite::Regex::new(r"^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=").unwrap();
    let mut keys: Vec<String> = vec!["VARLOCK_ENV".to_string()];

    for i in 0..doc.line_count() {
        if let Some(caps) = env_key_re.captures(doc.line_at(i)) {
            keys.push(caps[1].to_string());
        }
    }

    keys.sort();
    keys.dedup();

    keys.into_iter()
        .map(|key| CompletionItem {
            label: key.clone(),
            kind: Some(CompletionItemKind::VARIABLE),
            text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                range: ctx.replace_range,
                new_text: key.clone(),
            })),
            detail: Some("Config item reference".to_string()),
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!("Reference `{}` with `${}`.", key, key),
            })),
            ..Default::default()
        })
        .collect()
}

fn match_decorator_name(comment_prefix: &str, position: Position) -> Option<MatchContext> {
    let re = regex_lite::Regex::new(r"(^|\s)(@[\w\-]*)$").unwrap();
    let caps = re.captures(comment_prefix)?;
    let token = caps.get(2)?.as_str();
    Some(MatchContext {
        replace_range: Range::new(
            Position::new(position.line, position.character - token.len() as u32),
            position,
        ),
    })
}

fn match_type_value(comment_prefix: &str, position: Position) -> Option<MatchContext> {
    let re = regex_lite::Regex::new(r"(^|\s)@type=([\w\-]*)$").unwrap();
    let caps = re.captures(comment_prefix)?;
    let typed_value = caps.get(2)?.as_str();
    Some(MatchContext {
        replace_range: Range::new(
            Position::new(position.line, position.character - typed_value.len() as u32),
            position,
        ),
    })
}

fn match_type_option(
    comment_prefix: &str,
    position: Position,
) -> Option<(&DataTypeInfo, MatchContext)> {
    let data_type = get_type_option_data_type(comment_prefix)?;
    if data_type.option_snippets.is_none() {
        return None;
    }

    let re = regex_lite::Regex::new(r"(^|\s)@type=([A-Za-z][\w\-]*)\((?:[^#)]*?,\s*)?([\w\-]*)$")
        .unwrap();
    let caps = re.captures(comment_prefix)?;
    let typed_value = caps.get(3)?.as_str();

    Some((
        data_type,
        MatchContext {
            replace_range: Range::new(
                Position::new(position.line, position.character - typed_value.len() as u32),
                position,
            ),
        },
    ))
}

fn match_reference(line_prefix: &str, position: Position) -> Option<MatchContext> {
    let re = regex_lite::Regex::new(r"\$([A-Za-z0-9_]*)$").unwrap();
    let caps = re.captures(line_prefix)?;
    let typed_value = caps.get(1)?.as_str();
    Some(MatchContext {
        replace_range: Range::new(
            Position::new(position.line, position.character - typed_value.len() as u32),
            position,
        ),
    })
}

fn match_resolver_value(line_prefix: &str, position: Position) -> Option<MatchContext> {
    let re = regex_lite::Regex::new(r"(?:=\s*|[(,]\s*)([A-Za-z][\w\-]*)$").unwrap();
    let caps = re.captures(line_prefix)?;
    let typed_value = caps.get(1)?.as_str();
    Some(MatchContext {
        replace_range: Range::new(
            Position::new(position.line, position.character - typed_value.len() as u32),
            position,
        ),
    })
}

fn match_decorator_value(
    comment_prefix: &str,
    position: Position,
) -> Option<(&'static DecoratorInfo, MatchContext)> {
    let re = regex_lite::Regex::new(r"(^|\s)@([\w\-]+)=([A-Za-z][\w\-]*)$").unwrap();
    let caps = re.captures(comment_prefix)?;
    let dec_name = &caps[2];
    let decorator = DECORATORS_BY_NAME.get(dec_name)?;
    let typed_value = caps.get(3)?.as_str();

    Some((
        decorator,
        MatchContext {
            replace_range: Range::new(
                Position::new(position.line, position.character - typed_value.len() as u32),
                position,
            ),
        },
    ))
}

fn create_keyword_items(values: &[&str], ctx: &MatchContext) -> Vec<CompletionItem> {
    values
        .iter()
        .map(|value| CompletionItem {
            label: value.to_string(),
            kind: Some(CompletionItemKind::VALUE),
            text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                range: ctx.replace_range,
                new_text: value.to_string(),
            })),
            ..Default::default()
        })
        .collect()
}

fn create_decorator_value_items(
    doc: &LineDocument,
    decorator: &DecoratorInfo,
    ctx: &MatchContext,
) -> Option<Vec<CompletionItem>> {
    match decorator.name {
        "currentEnv" => Some(create_reference_items(doc, ctx)),
        "defaultRequired" => Some(create_keyword_items(&["infer", "true", "false"], ctx)),
        "defaultSensitive" => {
            let mut items = create_keyword_items(&["true", "false"], ctx);
            items.extend(
                RESOLVERS
                    .iter()
                    .filter(|r| r.name == "inferFromPrefix")
                    .map(|info| create_resolver_item(info, ctx)),
            );
            Some(items)
        }
        "required" | "optional" | "sensitive" | "public" | "disable" => {
            let mut items = create_keyword_items(&["true", "false"], ctx);
            items.extend(
                RESOLVERS
                    .iter()
                    .filter(|r| ["forEnv", "eq", "if", "not", "isEmpty"].contains(&r.name))
                    .map(|info| create_resolver_item(info, ctx)),
            );
            Some(items)
        }
        _ => None,
    }
}

fn match_item_value(line_prefix: &str, position: Position) -> Option<MatchContext> {
    let re =
        regex_lite::Regex::new(r"^\s*[A-Za-z_][A-Za-z0-9_]*\s*=\s*([A-Za-z0-9._\-]*)$").unwrap();
    let caps = re.captures(line_prefix)?;
    let typed_value = caps.get(1)?.as_str();
    Some(MatchContext {
        replace_range: Range::new(
            Position::new(position.line, position.character - typed_value.len() as u32),
            position,
        ),
    })
}

fn get_enum_value_context(
    doc: &LineDocument,
    position: Position,
    line_prefix: &str,
) -> Option<(Vec<String>, MatchContext)> {
    let item_ctx = match_item_value(line_prefix, position)?;
    let enum_values = get_enum_values_from_preceding_comments(doc, position.line as usize)?;
    Some((enum_values, item_ctx))
}

fn create_enum_value_items(enum_values: &[String], ctx: &MatchContext) -> Vec<CompletionItem> {
    enum_values
        .iter()
        .map(|value| CompletionItem {
            label: value.clone(),
            kind: Some(CompletionItemKind::ENUM_MEMBER),
            text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                range: ctx.replace_range,
                new_text: value.clone(),
            })),
            detail: Some("@type=enum value".to_string()),
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!("Allowed enum value `{}`.", value),
            })),
            ..Default::default()
        })
        .collect()
}

pub fn get_completions(doc: &LineDocument, position: Position) -> Option<Vec<CompletionItem>> {
    let line_text = doc.line_at(position.line as usize);
    let line_prefix = &line_text[..position.character as usize];

    let comment_start = line_prefix.find('#');
    let comment_prefix = comment_start.and_then(|_| get_decorator_comment_prefix(line_prefix));

    if let Some(ref_ctx) = match_reference(line_prefix, position) {
        return Some(create_reference_items(doc, &ref_ctx));
    }

    if let Some((enum_values, ctx)) = get_enum_value_context(doc, position, line_prefix) {
        return Some(create_enum_value_items(&enum_values, &ctx));
    }

    if let Some(comment_prefix) = comment_prefix {
        let existing_names =
            get_existing_decorator_names(doc, position.line as usize, comment_prefix);

        if let Some((data_type, ctx)) = match_type_option(comment_prefix, position) {
            if let Some(snippets) = data_type.option_snippets {
                return Some(
                    snippets
                        .iter()
                        .map(|opt| create_data_type_option_item(opt, &ctx))
                        .collect(),
                );
            }
        }

        if let Some(ctx) = match_type_value(comment_prefix, position) {
            return Some(
                DATA_TYPES
                    .iter()
                    .map(|info| create_data_type_item(info, &ctx))
                    .collect(),
            );
        }

        if let Some((decorator, ctx)) = match_decorator_value(comment_prefix, position) {
            return create_decorator_value_items(doc, decorator, &ctx);
        }

        if let Some(ctx) = match_decorator_name(comment_prefix, position) {
            let decorators = if is_in_header(doc, position.line as usize) {
                ROOT_DECORATORS.iter().collect::<Vec<_>>()
            } else {
                ITEM_DECORATORS.iter().collect::<Vec<_>>()
            };
            let available = filter_available_decorators(&decorators, &existing_names);
            return Some(
                available
                    .iter()
                    .map(|info| create_decorator_item(info, &ctx))
                    .collect(),
            );
        }
    }

    if let Some(ctx) = match_resolver_value(line_prefix, position) {
        return Some(
            RESOLVERS
                .iter()
                .map(|info| create_resolver_item(info, &ctx))
                .collect(),
        );
    }

    None
}
