use crate::catalog::DECORATORS_BY_NAME;
use crate::parser::LineDocument;
use tower_lsp::lsp_types::*;

pub fn get_hover(doc: &LineDocument, position: Position) -> Option<Hover> {
    let line_text = doc.line_at(position.line as usize);
    let trimmed = line_text.trim();

    if !trimmed.starts_with('#') {
        return None;
    }

    let char_pos = position.character as usize;
    if char_pos > line_text.len() {
        return None;
    }

    let word_re = regex_lite::Regex::new(r"@?[a-zA-Z0-9]+").unwrap();

    let mut best_start = None;
    let mut best_end = None;

    for caps in word_re.captures_iter(line_text) {
        let m = caps.get(0).unwrap();
        if m.start() <= char_pos && m.end() >= char_pos {
            best_start = Some(m.start());
            best_end = Some(m.end());
        }
    }

    let (start, end) = match (best_start, best_end) {
        (Some(s), Some(e)) => (s, e),
        _ => return None,
    };

    let hovered_text = &line_text[start..end];

    if !hovered_text.starts_with('@') {
        return None;
    }

    let dec_name = &hovered_text[1..];
    let dec = DECORATORS_BY_NAME.get(dec_name)?;

    let content = format!(
        "{}\n\n{}{}",
        dec.summary,
        dec.documentation,
        dec.deprecated
            .map(|d| format!("\n\n**Deprecated:** {}", d))
            .unwrap_or_default()
    );

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: content,
        }),
        range: Some(Range::new(
            Position::new(position.line, start as u32),
            Position::new(position.line, end as u32),
        )),
    })
}
