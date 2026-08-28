//! Path token definitions, tokenization, parsing, and syntax validation.

use crate::*;

#[derive(Clone, Debug)]
pub(crate) enum TokenKind {
    Root,
    RootMap,
    RootIndex {
        index: isize,
    },
    RootSlice {
        start: Option<isize>,
        end: Option<isize>,
    },
    RootFilter {
        field: String,
        operator: String,
        value: String,
    },
    Wildcard,
    DeepWildcard,
    Map(String),
    Get(String),
    Index {
        key: String,
        index: isize,
    },
    Slice {
        key: String,
        start: Option<isize>,
        end: Option<isize>,
    },
    Filter {
        list_key: String,
        field: String,
        operator: String,
        value: String,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct ParsedToken {
    pub(crate) raw: String,
    pub(crate) kind: TokenKind,
}

pub(crate) static INDEX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(.+)\[(-?\d+)\]$").expect("valid regex"));
pub(crate) static SLICE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(.+)\[(-?\d*):(-?\d*)\]$").expect("valid regex"));
pub(crate) static ROOT_INDEX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\[(-?\d+)\]$").expect("valid regex"));
pub(crate) static ROOT_SLICE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\[(-?\d*):(-?\d*)\]$").expect("valid regex"));
pub(crate) static PATH_FILTER_SEGMENT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\$([a-zA-Z_]\w*)(?:\((.*)\))?(\[\])?$").expect("valid regex"));

pub(crate) fn split_raw_path_tokens(path: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut bracket_depth = 0i32;

    for ch in path.chars() {
        if ch == '[' {
            bracket_depth += 1;
            current.push(ch);
            continue;
        }
        if ch == ']' {
            bracket_depth = (bracket_depth - 1).max(0);
            current.push(ch);
            continue;
        }
        if ch == '.' && bracket_depth == 0 {
            tokens.push(current);
            current = String::new();
            continue;
        }
        current.push(ch);
    }
    tokens.push(current);
    tokens
}

pub(crate) fn split_path_and_transform(path: &str) -> (String, Option<String>) {
    let mut bracket_depth = 0i32;
    let chars: Vec<char> = path.chars().collect();

    let mut i = 0usize;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '[' {
            bracket_depth += 1;
            i += 1;
            continue;
        }
        if ch == ']' {
            bracket_depth = (bracket_depth - 1).max(0);
            i += 1;
            continue;
        }
        if ch == '|' && bracket_depth == 0 && i + 1 < chars.len() && chars[i + 1] == '$' {
            let base = chars[..i].iter().collect::<String>();
            let transform = chars[i + 1..].iter().collect::<String>();
            return (base, Some(transform));
        }
        i += 1;
    }
    (path.to_string(), None)
}

pub(crate) fn parse_filter_expression_parts(expression: &str) -> Option<(String, String, String)> {
    let mut bracket_depth = 0i32;
    let mut paren_depth = 0i32;
    let mut brace_depth = 0i32;
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    let chars: Vec<char> = expression.chars().collect();
    let mut split: Option<(usize, &'static str)> = None;

    let mut i = 0usize;
    while i < chars.len() {
        let ch = chars[i];
        if escaped {
            escaped = false;
            i += 1;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            i += 1;
            continue;
        }

        if in_single {
            if ch == '\'' {
                in_single = false;
            }
            i += 1;
            continue;
        }
        if in_double {
            if ch == '"' {
                in_double = false;
            }
            i += 1;
            continue;
        }

        match ch {
            '\'' => {
                in_single = true;
                i += 1;
                continue;
            }
            '"' => {
                in_double = true;
                i += 1;
                continue;
            }
            '[' => {
                bracket_depth += 1;
                i += 1;
                continue;
            }
            ']' => {
                bracket_depth -= 1;
                if bracket_depth < 0 {
                    return None;
                }
                i += 1;
                continue;
            }
            '(' => {
                paren_depth += 1;
                i += 1;
                continue;
            }
            ')' => {
                paren_depth -= 1;
                if paren_depth < 0 {
                    return None;
                }
                i += 1;
                continue;
            }
            '{' => {
                brace_depth += 1;
                i += 1;
                continue;
            }
            '}' => {
                brace_depth -= 1;
                if brace_depth < 0 {
                    return None;
                }
                i += 1;
                continue;
            }
            _ => {}
        }

        if bracket_depth == 0 && paren_depth == 0 && brace_depth == 0 {
            if i + 1 < chars.len() {
                match (chars[i], chars[i + 1]) {
                    ('=', '=') => {
                        split = Some((i, "=="));
                        break;
                    }
                    ('!', '=') => {
                        split = Some((i, "!="));
                        break;
                    }
                    ('>', '=') => {
                        split = Some((i, ">="));
                        break;
                    }
                    ('<', '=') => {
                        split = Some((i, "<="));
                        break;
                    }
                    _ => {}
                }
            }
            if ch == '>' {
                split = Some((i, ">"));
                break;
            }
            if ch == '<' {
                split = Some((i, "<"));
                break;
            }
        }

        i += 1;
    }

    if in_single
        || in_double
        || escaped
        || bracket_depth != 0
        || paren_depth != 0
        || brace_depth != 0
    {
        return None;
    }

    if let Some((index, operator)) = split {
        let field = expression[..index].trim().to_string();
        let value = expression[index + operator.len()..].trim().to_string();
        if field.is_empty() || value.is_empty() {
            return None;
        }
        return Some((field, operator.to_string(), value));
    }

    let field = expression.trim().to_string();
    if field.is_empty() {
        return None;
    }
    Some((field, "==".to_string(), "$bool".to_string()))
}

pub(crate) fn parse_filter_token_parts(
    raw_token: &str,
) -> Option<(String, String, String, String)> {
    let start = raw_token.find("[?")?;
    if !raw_token.ends_with(']') {
        return None;
    }

    let list_key = raw_token[..start].to_string();
    if list_key.is_empty() {
        return None;
    }

    let expression = &raw_token[start + 2..raw_token.len() - 1];
    let (field, operator, value) = parse_filter_expression_parts(expression)?;
    Some((list_key, field, operator, value))
}

pub(crate) fn parse_root_selector_suffix(suffix: &str) -> Result<TokenKind, String> {
    if suffix == "[]" {
        return Ok(TokenKind::RootMap);
    }

    if let Some(captures) = ROOT_INDEX_RE.captures(suffix) {
        let index = captures
            .get(1)
            .and_then(|m| m.as_str().parse::<isize>().ok())
            .ok_or("Failed to parse list index.")?;
        return Ok(TokenKind::RootIndex { index });
    }

    if let Some(captures) = ROOT_SLICE_RE.captures(suffix) {
        let start = captures
            .get(1)
            .map(|m| m.as_str())
            .filter(|s| !s.is_empty())
            .and_then(|s| s.parse::<isize>().ok());
        let end = captures
            .get(2)
            .map(|m| m.as_str())
            .filter(|s| !s.is_empty())
            .and_then(|s| s.parse::<isize>().ok());
        return Ok(TokenKind::RootSlice { start, end });
    }

    if suffix.starts_with("[?") {
        if !suffix.ends_with(']') {
            return Err("Failed to parse filter token.".to_string());
        }
        let expression = &suffix[2..suffix.len() - 1];
        let Some((field, operator, value)) = parse_filter_expression_parts(expression) else {
            return Err("Failed to parse filter token.".to_string());
        };
        return Ok(TokenKind::RootFilter {
            field,
            operator,
            value,
        });
    }

    Err("Failed to parse root selector token.".to_string())
}

pub(crate) fn parse_token(raw_token: &str) -> Result<TokenKind, String> {
    if raw_token == "$$root" {
        return Ok(TokenKind::Root);
    }
    if let Some(suffix) = raw_token.strip_prefix("$$root") {
        if !suffix.is_empty() && suffix.starts_with('[') {
            return parse_root_selector_suffix(suffix);
        }
    }
    if raw_token.starts_with(".[") {
        return parse_root_selector_suffix(&raw_token[1..]);
    }
    if raw_token == "*" {
        return Ok(TokenKind::Wildcard);
    }
    if raw_token == "**" {
        return Ok(TokenKind::DeepWildcard);
    }
    if raw_token.ends_with("[]") {
        return Ok(TokenKind::Map(raw_token[..raw_token.len() - 2].to_string()));
    }

    if let Some(captures) = INDEX_RE.captures(raw_token) {
        let key = captures
            .get(1)
            .map(|m| m.as_str().to_string())
            .ok_or("Failed to parse index key.")?;
        let index = captures
            .get(2)
            .and_then(|m| m.as_str().parse::<isize>().ok())
            .ok_or("Failed to parse list index.")?;
        return Ok(TokenKind::Index { key, index });
    }

    if let Some(captures) = SLICE_RE.captures(raw_token) {
        let key = captures
            .get(1)
            .map(|m| m.as_str().to_string())
            .ok_or("Failed to parse slice key.")?;
        let start = captures
            .get(2)
            .map(|m| m.as_str())
            .filter(|s| !s.is_empty())
            .and_then(|s| s.parse::<isize>().ok());
        let end = captures
            .get(3)
            .map(|m| m.as_str())
            .filter(|s| !s.is_empty())
            .and_then(|s| s.parse::<isize>().ok());
        return Ok(TokenKind::Slice { key, start, end });
    }

    if raw_token.contains("[?") {
        let Some((list_key, field, operator, value)) = parse_filter_token_parts(raw_token) else {
            return Err("Failed to parse filter token.".to_string());
        };
        return Ok(TokenKind::Filter {
            list_key,
            field,
            operator,
            value,
        });
    }

    Ok(TokenKind::Get(raw_token.to_string()))
}

pub(crate) fn validate_filter_token(
    py: Python<'_>,
    _module: &Bound<'_, PyModule>,
    _registry: &Bound<'_, PyAny>,
    list_key: &str,
    field: &str,
    operator: &str,
    value: &str,
) -> PyResult<()> {
    if !field.starts_with('.') {
        return Err(make_parse_error(
            py,
            &format!("{list_key}[?{field}{operator}{value}]"),
            Some(field),
            "Predicate field expressions must start with '.' (for example: '[?.id==1]' or '[?.|$len>3]').",
        ));
    }

    if field == "." {
        // Valid root-field expression.
    } else if let Some(field_transform) = field.strip_prefix(".|") {
        if compile_builtin_pipeline(py, field_transform, None).is_none() {
            return Err(make_parse_error(
                py,
                &format!("{list_key}[?{field}{operator}{value}]"),
                Some(field),
                &format!("Invalid left-hand predicate expression '{field}'."),
            ));
        }
    } else if field.len() == 1 {
        return Err(make_parse_error(
            py,
            &format!("{list_key}[?{field}{operator}{value}]"),
            Some(field),
            "Predicate field expression cannot be empty. Use '[?.field ...]' or '[?.|$filter ...]'.",
        ));
    }

    // Validate right-side predicate expression/filter syntax.
    if let Err(message) = compile_builtin_or_boolean_predicate(py, value) {
        return Err(make_parse_error(
            py,
            &format!("{list_key}[?{field}{operator}{value}]"),
            Some(value),
            &message,
        ));
    }

    Ok(())
}

pub(crate) fn parse_path(
    py: Python<'_>,
    module: &Bound<'_, PyModule>,
    registry: &Bound<'_, PyAny>,
    path: &str,
) -> PyResult<Vec<ParsedToken>> {
    if path.is_empty() {
        return Err(make_parse_error(py, path, None, "Path cannot be empty."));
    }

    let mut raw_tokens = split_raw_path_tokens(path);
    if raw_tokens.len() >= 2
        && raw_tokens[0].is_empty()
        && raw_tokens[1].starts_with('[')
        && raw_tokens[1].ends_with(']')
    {
        raw_tokens[1] = format!(".{}", raw_tokens[1]);
        raw_tokens.remove(0);
    }

    let mut tokens: Vec<ParsedToken> = Vec::new();
    for raw_token in raw_tokens {
        let kind = match parse_token(&raw_token) {
            Ok(parsed) => parsed,
            Err(message) => return Err(make_parse_error(py, path, Some(&raw_token), &message)),
        };

        match &kind {
            TokenKind::Filter {
                list_key,
                field,
                operator,
                value,
            } => {
                validate_filter_token(py, module, registry, list_key, field, operator, value)?;
            }
            TokenKind::RootFilter {
                field,
                operator,
                value,
            } => {
                let root_key = if raw_token.starts_with("$$root") {
                    "$$root"
                } else {
                    "."
                };
                validate_filter_token(py, module, registry, root_key, field, operator, value)?;
            }
            _ => {}
        }

        tokens.push(ParsedToken {
            raw: raw_token,
            kind,
        });
    }
    Ok(tokens)
}
