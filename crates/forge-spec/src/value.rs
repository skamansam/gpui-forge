//! The Forge value micro-syntax.
//!
//! Forge reads every YAML scalar as text and types it here, so YAML's own typing
//! (`0.1`, `on`, `no`) never changes a value. The quoting selects the rules:
//! plain scalars are fully typed, double-quoted scalars are text (with `{$path}`
//! interpolation) or a color, and single-quoted scalars are always literal text.

use crate::model::{BindingMode, Color, HandlerRef, Path, PathSegment, Segment, Unit, Value};

/// How a scalar was written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quoting {
    Plain,
    Single,
    Double,
}

/// Binding root that refers to theme tokens instead of view state.
pub const THEME_ROOT: &str = "theme";

pub fn parse_value(text: &str, quoting: Quoting) -> Result<Value, String> {
    match quoting {
        Quoting::Single => Ok(Value::String(text.to_owned())),
        Quoting::Double => match Color::parse_hex(text) {
            Some(color) => Ok(Value::Color(color)),
            None => parse_text(text),
        },
        Quoting::Plain => parse_plain(text),
    }
}

fn parse_plain(text: &str) -> Result<Value, String> {
    match text {
        "true" => return Ok(Value::Bool(true)),
        "false" => return Ok(Value::Bool(false)),
        _ => {}
    }
    if let Some(rest) = text.strip_prefix("bind ") {
        let path = rest
            .trim_start()
            .strip_prefix('$')
            .ok_or("`bind` must be followed by a binding, e.g. `bind $form.email`")?;
        return Ok(Value::Binding {
            path: parse_path(path)?,
            mode: BindingMode::TwoWay,
        });
    }
    if let Some(path) = text.strip_prefix('$') {
        return binding_or_token(path);
    }
    if let Some(number) = parse_number(text) {
        return Ok(number);
    }
    if is_ident(text) {
        return Ok(Value::Enum(text.to_owned()));
    }
    parse_text(text)
}

fn binding_or_token(text: &str) -> Result<Value, String> {
    let path = parse_path(text)?;
    if path.0.first() == Some(&PathSegment::Field(THEME_ROOT.to_owned())) {
        if path.0.len() == 1 {
            return Err("`$theme` needs a token name, e.g. `$theme.colors.primary`".into());
        }
        return Ok(Value::Token(Path(path.0[1..].to_vec())));
    }
    Ok(Value::Binding {
        path,
        mode: BindingMode::OneWay,
    })
}

/// Text with optional `{$path}` placeholders; `{{` is a literal `{`.
fn parse_text(text: &str) -> Result<Value, String> {
    let mut segments = Vec::new();
    let mut literal = String::new();
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        literal.push_str(&rest[..open]);
        rest = &rest[open + 1..];
        if let Some(after) = rest.strip_prefix('{') {
            literal.push('{');
            rest = after;
            continue;
        }
        let close = rest
            .find('}')
            .ok_or("unclosed `{`; write `{{` for a literal brace")?;
        let inner = rest[..close].trim();
        let Some(path) = inner.strip_prefix('$') else {
            return Err(format!(
                "`{{{inner}}}` is not a binding; placeholders look like `{{$user.name}}` and `{{{{` is a literal brace"
            ));
        };
        if !literal.is_empty() {
            segments.push(Segment::Text(std::mem::take(&mut literal)));
        }
        segments.push(Segment::Binding(parse_path(path)?));
        rest = &rest[close + 1..];
    }
    literal.push_str(rest);
    if segments.is_empty() {
        return Ok(Value::String(literal));
    }
    if !literal.is_empty() {
        segments.push(Segment::Text(literal));
    }
    Ok(Value::Interpolated(segments))
}

fn parse_number(text: &str) -> Option<Value> {
    let split = text
        .find(|c: char| c.is_ascii_alphabetic() || c == '%')
        .unwrap_or(text.len());
    let (number, unit) = text.split_at(split);
    let unit = if unit.is_empty() {
        None
    } else {
        Some(Unit::parse(unit)?)
    };
    let digits = number.strip_prefix('-').unwrap_or(number);
    let mut parts = digits.split('.');
    let int = parts.next()?;
    let frac = parts.next();
    if parts.next().is_some() || !all_digits(int) || frac.is_some_and(|f| !all_digits(f)) {
        return None;
    }
    Some(Value::Number {
        value: number.parse().ok()?,
        unit,
    })
}

fn all_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// Parses a binding path (without the leading `$`), e.g. `form.items[0].name`.
pub fn parse_path(text: &str) -> Result<Path, String> {
    let invalid = || format!("invalid binding path `${text}`");
    let mut segments = Vec::new();
    for piece in text.split('.') {
        let (name, mut indexes) = piece.split_at(piece.find('[').unwrap_or(piece.len()));
        if !is_path_ident(name) {
            return Err(invalid());
        }
        segments.push(PathSegment::Field(name.to_owned()));
        while let Some(rest) = indexes.strip_prefix('[') {
            let close = rest.find(']').ok_or_else(invalid)?;
            let index = rest[..close].parse().map_err(|_| invalid())?;
            segments.push(PathSegment::Index(index));
            indexes = &rest[close + 1..];
        }
        if !indexes.is_empty() {
            return Err(invalid());
        }
    }
    Ok(Path(segments))
}

/// Parses an event handler: `sign_in`, `sign_in()` or `go_to($wizard.id, 2, 'next')`.
pub fn parse_handler(text: &str) -> Result<HandlerRef, String> {
    let text = text.trim();
    let (name, args) = match text.split_once('(') {
        None => (text, ""),
        Some((name, rest)) => (
            name.trim(),
            rest.strip_suffix(')')
                .ok_or("handler call is missing a closing `)`")?,
        ),
    };
    if !is_path_ident(name) {
        return Err(format!(
            "invalid handler `{text}`; handlers are hook ids like `sign_in` or `go_to($wizard.id, 2)`"
        ));
    }
    let args = split_args(args)?
        .iter()
        .map(|arg| parse_arg(arg))
        .collect::<Result<_, _>>()?;
    Ok(HandlerRef {
        name: name.to_owned(),
        args,
    })
}

fn parse_arg(arg: &str) -> Result<Value, String> {
    let value = if let Some(inner) = arg.strip_prefix('\'').and_then(|a| a.strip_suffix('\'')) {
        parse_value(&inner.replace("''", "'"), Quoting::Single)?
    } else if let Some(inner) = arg.strip_prefix('"').and_then(|a| a.strip_suffix('"')) {
        parse_value(inner, Quoting::Double)?
    } else {
        parse_value(arg, Quoting::Plain)?
    };
    if is_two_way(&value) {
        return Err("`bind` (two-way binding) is only allowed on props".into());
    }
    Ok(value)
}

fn split_args(args: &str) -> Result<Vec<String>, String> {
    if args.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    for c in args.chars() {
        match (quote, c) {
            (None, '\'' | '"') => {
                quote = Some(c);
                current.push(c);
            }
            (Some(q), _) if c == q => {
                quote = None;
                current.push(c);
            }
            (None, ',') => out.push(std::mem::take(&mut current).trim().to_owned()),
            _ => current.push(c),
        }
    }
    if quote.is_some() {
        return Err("unterminated string in handler arguments".into());
    }
    out.push(current.trim().to_owned());
    if out.iter().any(String::is_empty) {
        return Err("empty handler argument".into());
    }
    Ok(out)
}

pub fn is_two_way(value: &Value) -> bool {
    matches!(
        value,
        Value::Binding {
            mode: BindingMode::TwoWay,
            ..
        }
    )
}

/// Enum values, ids, props and event names: `[A-Za-z_][A-Za-z0-9_-]*`.
pub fn is_ident(s: &str) -> bool {
    let mut chars = s.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

pub(crate) fn is_path_ident(s: &str) -> bool {
    let mut chars = s.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub(crate) fn is_prop_key(s: &str) -> bool {
    is_ident(s) && s.starts_with(|c: char| c.is_ascii_lowercase() || c == '_')
}

pub(crate) fn is_namespace(s: &str) -> bool {
    let mut chars = s.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

pub(crate) fn is_element_name(s: &str) -> bool {
    let mut chars = s.chars();
    chars.next().is_some_and(|c| c.is_ascii_uppercase())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(text: &str) -> Value {
        parse_value(text, Quoting::Plain).unwrap()
    }

    fn path(text: &str) -> Path {
        parse_path(text).unwrap()
    }

    fn number(value: f64, unit: Option<Unit>) -> Value {
        Value::Number { value, unit }
    }

    #[test]
    fn numbers_and_units() {
        assert_eq!(plain("12"), number(12.0, None));
        assert_eq!(plain("12px"), number(12.0, Some(Unit::Px)));
        assert_eq!(plain("-1.5rem"), number(-1.5, Some(Unit::Rem)));
        assert_eq!(plain("50%"), number(50.0, Some(Unit::Percent)));
        assert_eq!(plain("1fr"), number(1.0, Some(Unit::Fr)));
        assert_eq!(plain("12em"), Value::String("12em".into()));
        assert_eq!(plain("1.2.3"), Value::String("1.2.3".into()));
    }

    #[test]
    fn words_bools_and_text() {
        assert_eq!(plain("true"), Value::Bool(true));
        assert_eq!(plain("space-between"), Value::Enum("space-between".into()));
        assert_eq!(plain("on"), Value::Enum("on".into()));
        assert_eq!(plain("Sign in"), Value::String("Sign in".into()));
        assert_eq!(
            parse_value("primary", Quoting::Double).unwrap(),
            Value::String("primary".into())
        );
        assert_eq!(
            parse_value("{$not.interpolated}", Quoting::Single).unwrap(),
            Value::String("{$not.interpolated}".into())
        );
    }

    #[test]
    fn bindings_and_tokens() {
        assert_eq!(
            plain("$items[0].name"),
            Value::Binding {
                path: Path(vec![
                    PathSegment::Field("items".into()),
                    PathSegment::Index(0),
                    PathSegment::Field("name".into()),
                ]),
                mode: BindingMode::OneWay,
            }
        );
        assert_eq!(
            plain("bind $form.email"),
            Value::Binding {
                path: path("form.email"),
                mode: BindingMode::TwoWay
            }
        );
        assert_eq!(
            plain("$theme.colors.primary"),
            Value::Token(path("colors.primary"))
        );
        assert!(parse_value("$theme", Quoting::Plain).is_err());
        assert!(parse_value("$form..x", Quoting::Plain).is_err());
        assert!(parse_value("bind form.x", Quoting::Plain).is_err());
    }

    #[test]
    fn interpolation() {
        assert_eq!(
            parse_value("Hi {$user.name}, {{literal}", Quoting::Double).unwrap(),
            Value::Interpolated(vec![
                Segment::Text("Hi ".into()),
                Segment::Binding(path("user.name")),
                Segment::Text(", {literal}".into()),
            ])
        );
        assert_eq!(
            parse_value("{{x}", Quoting::Double).unwrap(),
            Value::String("{x}".into())
        );
        assert!(parse_value("Hi {user}", Quoting::Double).is_err());
        assert!(parse_value("Hi {$user", Quoting::Double).is_err());
    }

    #[test]
    fn colors() {
        let white = Color {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        };
        assert_eq!(
            parse_value("#fff", Quoting::Double).unwrap(),
            Value::Color(white)
        );
        assert_eq!(
            Color::parse_hex("#00000080").unwrap().to_string(),
            "#00000080"
        );
        assert_eq!(
            parse_value("#ggg", Quoting::Double).unwrap(),
            Value::String("#ggg".into())
        );
    }

    #[test]
    fn handlers() {
        let h = parse_handler("sign_in").unwrap();
        assert_eq!((h.name.as_str(), h.args.len()), ("sign_in", 0));
        assert_eq!(parse_handler("sign_in()").unwrap(), h);
        let h = parse_handler("go_to($wizard.id, 2, 'a, b')").unwrap();
        assert_eq!(h.args[1], number(2.0, None));
        assert_eq!(h.args[2], Value::String("a, b".into()));
        assert!(parse_handler("bad name()").is_err());
        assert!(parse_handler("save(bind $x)").is_err());
        assert!(parse_handler("save($x").is_err());
    }
}
