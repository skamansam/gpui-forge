//! The style & layout model (`docs/spec/style-layout.md`): which keys a `style:` map may hold and
//! what values each accepts. Renderers map each key onto GPUI `Styled` methods.

use crate::model::{Unit, Value};

/// The kind of value a style key accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StyleType {
    Length,
    /// A length, `auto` or `full`.
    Size,
    /// One length, `[y, x]` or `[top, right, bottom, left]`.
    Edges,
    Color,
    Bool,
    /// A number, or `true`/`false` for 1/0.
    Factor,
    /// A number from 0 to 1.
    Fraction,
    Keyword(&'static [&'static str]),
    LengthOrKeyword(&'static [&'static str]),
}

const ALIGN: &[&str] = &["start", "center", "end", "stretch", "baseline"];

/// Every key the 0.1 model defines.
pub const KEYS: &[(&str, StyleType)] = &[
    (
        "direction",
        StyleType::Keyword(&["row", "column", "row-reverse", "column-reverse"]),
    ),
    ("gap", StyleType::Length),
    ("gap-x", StyleType::Length),
    ("gap-y", StyleType::Length),
    ("align", StyleType::Keyword(ALIGN)),
    (
        "justify",
        StyleType::Keyword(&["start", "center", "end", "between", "around", "evenly"]),
    ),
    ("wrap", StyleType::Bool),
    ("grow", StyleType::Factor),
    ("shrink", StyleType::Factor),
    ("basis", StyleType::Size),
    ("align-self", StyleType::Keyword(ALIGN)),
    ("width", StyleType::Size),
    ("height", StyleType::Size),
    ("min-width", StyleType::Size),
    ("max-width", StyleType::Size),
    ("min-height", StyleType::Size),
    ("max-height", StyleType::Size),
    ("padding", StyleType::Edges),
    ("margin", StyleType::Edges),
    (
        "overflow",
        StyleType::Keyword(&["visible", "hidden", "scroll"]),
    ),
    ("border", StyleType::Length),
    ("border-color", StyleType::Color),
    ("border-style", StyleType::Keyword(&["solid", "dashed"])),
    (
        "radius",
        StyleType::LengthOrKeyword(&["sm", "md", "lg", "full"]),
    ),
    ("background", StyleType::Color),
    ("opacity", StyleType::Fraction),
    (
        "shadow",
        StyleType::Keyword(&["none", "sm", "md", "lg", "xl"]),
    ),
    ("color", StyleType::Color),
    (
        "size",
        StyleType::LengthOrKeyword(&["xs", "sm", "base", "lg", "xl", "2xl", "3xl"]),
    ),
    (
        "weight",
        StyleType::Keyword(&[
            "thin", "light", "normal", "medium", "semibold", "bold", "black",
        ]),
    ),
    (
        "text-align",
        StyleType::Keyword(&["left", "center", "right"]),
    ),
    ("line-height", StyleType::Length),
    ("italic", StyleType::Bool),
    ("underline", StyleType::Bool),
    ("truncate", StyleType::Bool),
];

pub fn key_type(key: &str) -> Option<StyleType> {
    KEYS.iter().find(|(k, _)| *k == key).map(|(_, t)| *t)
}

/// The defined key closest to `key`, for "did you mean" hints.
pub fn closest_key(key: &str) -> Option<&'static str> {
    KEYS.iter()
        .map(|(k, _)| (distance(key, k), *k))
        .filter(|(d, k)| *d <= 2.max(k.len() / 3))
        .min()
        .map(|(_, k)| k)
}

impl StyleType {
    /// Whether `value` fits. One-way bindings are checked at runtime.
    pub fn accepts(self, value: &Value) -> bool {
        if matches!(value, Value::Binding { .. }) {
            return true;
        }
        // Words like `2xl` don't parse as bare enums, so plain strings count too.
        let word = |words: &[&str]| matches!(value, Value::Enum(w) | Value::String(w) if words.contains(&w.as_str()));
        match self {
            StyleType::Length => is_length(value),
            StyleType::Size => is_length(value) || word(&["auto", "full"]),
            StyleType::Edges => match value {
                Value::List(items) => {
                    matches!(items.len(), 1 | 2 | 4) && items.iter().all(is_length)
                }
                other => is_length(other),
            },
            StyleType::Color => matches!(value, Value::Color(_) | Value::Token(_)),
            StyleType::Bool => matches!(value, Value::Bool(_)),
            StyleType::Factor => {
                matches!(
                    value,
                    Value::Bool(_)
                        | Value::Number {
                            unit: None,
                            value: 0.0..
                        }
                )
            }
            StyleType::Fraction => {
                matches!(value, Value::Number { unit: None, value } if (0.0..=1.0).contains(value))
            }
            StyleType::Keyword(words) => word(words),
            StyleType::LengthOrKeyword(words) => is_length(value) || word(words),
        }
    }
}

impl std::fmt::Display for StyleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StyleType::Length => f.write_str("a length (`12px`, `1.5rem`, `50%`)"),
            StyleType::Size => f.write_str("a length, `auto` or `full`"),
            StyleType::Edges => f.write_str("a length, `[y, x]` or `[top, right, bottom, left]`"),
            StyleType::Color => f.write_str("a quoted color (`\"#3b82f6\"`) or `$theme.<field>`"),
            StyleType::Bool => f.write_str("`true` or `false`"),
            StyleType::Factor => f.write_str("a number, `true` or `false`"),
            StyleType::Fraction => f.write_str("a number from 0 to 1"),
            StyleType::Keyword(words) => write!(f, "one of {}", words.join(", ")),
            StyleType::LengthOrKeyword(words) => {
                write!(f, "a length or one of {}", words.join(", "))
            }
        }
    }
}

fn is_length(value: &Value) -> bool {
    matches!(
        value,
        Value::Number {
            unit: None | Some(Unit::Px | Unit::Rem | Unit::Percent),
            ..
        }
    )
}

fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cur = row[j + 1];
            row[j + 1] = (prev + usize::from(ca != *cb)).min(row[j] + 1).min(cur + 1);
            prev = cur;
        }
    }
    row[b.len()]
}
