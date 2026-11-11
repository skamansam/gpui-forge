//! Canonical Forge YAML writer (the formatter).

use crate::model::{
    BindingMode, Case, Compare, Control, Document, ElementRef, HandlerRef, Node, Segment, Value,
};
use crate::value::{self, Quoting};

/// How node ids are written. Both forms are always accepted when reading.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IdStyle {
    /// `Input#email`
    #[default]
    Hash,
    /// `Input .email`
    Dot,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WriteOptions {
    pub id_style: IdStyle,
}

/// Writes `doc` preserving the original source wherever the model did not
/// change — comments, anchors, aliases, `<<` merges and formatting survive.
/// Falls back to canonical output for documents built programmatically.
pub fn to_string_preserving(doc: &Document, options: &WriteOptions) -> String {
    super::preserve::preserving(doc, options).unwrap_or_else(|| to_string(doc, options))
}

/// Writes `doc` as canonical Forge YAML.
///
/// Key order: element key, props, `style`, events, `children`.
pub fn to_string(doc: &Document, options: &WriteOptions) -> String {
    let mut out = String::new();
    if let Some(schema) = &doc.schema {
        line(&mut out, 0, &format!("$schema: {}", text_scalar(schema)));
    }
    line(
        &mut out,
        0,
        &format!("forge: {}", double_quoted(&doc.version, false)),
    );
    if let Some(base) = &doc.base {
        line(&mut out, 0, &format!("base: {}", text_scalar(base)));
    }
    if !doc.imports.is_empty() {
        line(&mut out, 0, "use:");
        for import in &doc.imports {
            let spec = match &import.version {
                Some(version) => format!("{} {version}", import.library),
                None => import.library.clone(),
            };
            line(
                &mut out,
                2,
                &format!("{}: {}", import.alias, text_scalar(&spec)),
            );
        }
    }
    if !doc.state.is_empty() {
        line(&mut out, 0, "state:");
        for (key, text) in &doc.state {
            line(&mut out, 1, &format!("{key}: {text}"));
        }
    }
    if !doc.meta.is_empty() {
        line(&mut out, 0, "meta:");
        for (key, text) in &doc.meta {
            line(&mut out, 2, &format!("{key}: {}", text_scalar(text)));
        }
    }
    // Ref blocks precede every `<<: *name` use — anchors must be defined
    // before their aliases resolve (dialogs and root both live below).
    for (name, fragment) in &doc.refs {
        out.push('\n');
        line(&mut out, 0, &format!("{name}: &{name}"));
        write_entries(&mut out, fragment, 2, options);
    }
    for dialog in &doc.dialogs {
        out.push('\n');
        write_node(&mut out, dialog, 0, false, options);
    }
    out.push('\n');
    line(&mut out, 0, "root:");
    write_node(&mut out, &doc.root, 2, false, options);
    out
}

pub(crate) fn line(out: &mut String, indent: usize, text: &str) {
    out.extend(std::iter::repeat_n(' ', indent));
    out.push_str(text);
    out.push('\n');
}

pub(crate) fn write_node(
    out: &mut String,
    node: &Node,
    indent: usize,
    list_item: bool,
    options: &WriteOptions,
) {
    let indent = write_head(out, node, indent, list_item, options);
    write_entries(out, node, indent, options);
}

/// Writes the `Element#id: content` head line — or the `if:`/`for:`/`with:`/
/// `match:` control line for bare blocks. Returns the body indent.
pub(crate) fn write_head(
    out: &mut String,
    node: &Node,
    indent: usize,
    list_item: bool,
    options: &WriteOptions,
) -> usize {
    if node.element.name.is_empty() {
        // Ref-block fragments have no element head — `name: &name` is the line.
        return indent;
    }
    let head = match &node.control {
        Some(control) if node.is_block() => {
            format!("{}: {}", control.keyword(), inline(control.source(), false))
        }
        _ => {
            let mut head = element_key(&node.element, node.id.as_deref(), options.id_style);
            head.push(':');
            if let Some(content) = &node.content {
                head.push(' ');
                head.push_str(&inline(content, false));
            }
            head
        }
    };
    if list_item {
        line(out, indent, &format!("- {head}"));
        indent + 2
    } else {
        line(out, indent, &head);
        indent
    }
}

/// Writes control sub-keys, props, `style:`, events, `case` arms, `children:`
/// and `else:` below the head line.
pub(crate) fn write_entries(out: &mut String, node: &Node, indent: usize, options: &WriteOptions) {
    // `<<` merges splice the named ref block's entries in below the head.
    if let Some(refname) = &node.refname {
        line(out, indent, &format!("<<: *{refname}"));
    }
    // Entries the merges contributed live in the ref block — never re-emit.
    let merged = |key: &str| node.merged.iter().any(|k| k == key);
    if let Some(control) = &node.control {
        // Modifiers repeat the control key under the element head. Bare
        // blocks already wrote it as their head line.
        if !node.is_block() {
            line(
                out,
                indent,
                &format!("{}: {}", control.keyword(), inline(control.source(), false)),
            );
        }
        match control {
            Control::If { test, .. } => {
                if let Some(test) = test {
                    let (key, operand) = match test {
                        Compare::Is(v) => ("is", v),
                        Compare::Not(v) => ("not", v),
                    };
                    line(out, indent, &format!("{key}: {}", inline(operand, false)));
                }
            }
            Control::For {
                each, index, key, ..
            } => {
                line(out, indent, &format!("each: {each}"));
                if let Some(index) = index {
                    line(out, indent, &format!("index: {index}"));
                }
                if let Some(key) = key {
                    let path = key
                        .0
                        .iter()
                        .map(|s| match s {
                            crate::model::PathSegment::Field(name) => name.clone(),
                            crate::model::PathSegment::Index(i) => format!("[{i}]"),
                        })
                        .collect::<Vec<_>>()
                        .join(".");
                    line(out, indent, &format!("key: {path}"));
                }
            }
            Control::With { name, .. } => line(out, indent, &format!("as: {name}")),
            Control::Match { .. } => {}
        }
    }
    for (key, value) in &node.props {
        if !merged(key) {
            write_entry(out, indent, key, value);
        }
    }
    if !node.style.is_empty() && !merged("style") {
        line(out, indent, "style:");
        for (key, value) in &node.style {
            write_entry(out, indent + 2, key, value);
        }
    }
    for event in &node.events {
        if !merged(&format!("on {}", event.event)) {
            write_event(out, indent, event);
        }
    }
    if let Some(Control::Match { cases, .. }) = &node.control {
        for Case { value, children } in cases {
            if merged(&format!("case {}", inline(value, false))) {
                continue;
            }
            line(out, indent, &format!("case {}:", inline(value, false)));
            for child in children {
                write_node(out, child, indent + 2, true, options);
            }
        }
    }
    if !node.children.is_empty() && !merged("children") {
        line(out, indent, "children:");
        for child in &node.children {
            write_node(out, child, indent + 2, true, options);
        }
    }
    if let Some(otherwise) = node.control.as_ref().map(|c| c.otherwise())
        && !otherwise.is_empty()
        && !merged("else")
    {
        line(out, indent, "else:");
        for child in otherwise {
            write_node(out, child, indent + 2, true, options);
        }
    }
}

pub(crate) fn write_event(out: &mut String, indent: usize, event: &crate::model::EventBinding) {
    line(
        out,
        indent,
        &format!("on {}: {}", event.event, handler(&event.handler)),
    );
}

pub(crate) fn element_key(element: &ElementRef, id: Option<&str>, style: IdStyle) -> String {
    match (id, style) {
        (None, _) => element.to_string(),
        (Some(id), IdStyle::Hash) => format!("{element}#{id}"),
        (Some(id), IdStyle::Dot) => format!("{element} .{id}"),
    }
}

pub(crate) fn write_entry(out: &mut String, indent: usize, key: &str, value: &Value) {
    if let Value::List(items) = value {
        let flow: Vec<String> = items.iter().map(|item| inline(item, true)).collect();
        if flow.iter().all(|item| flow_safe(item)) {
            line(out, indent, &format!("{key}: [{}]", flow.join(", ")));
        } else {
            line(out, indent, &format!("{key}:"));
            for item in items {
                line(out, indent + 2, &format!("- {}", inline(item, false)));
            }
        }
        return;
    }
    line(out, indent, &format!("{key}: {}", inline(value, false)));
}

/// A value as a single-line YAML scalar (or flow sequence) that reads back identically.
pub(crate) fn inline(value: &Value, flow: bool) -> String {
    match value {
        Value::String(s) => string_scalar(s, flow),
        Value::Interpolated(segments) => {
            let mut text = String::new();
            for segment in segments {
                match segment {
                    Segment::Text(t) => text.push_str(&t.replace('{', "{{")),
                    Segment::Binding(path) => text.push_str(&format!("{{${path}}}")),
                }
            }
            double_quoted(&text, false)
        }
        Value::Number { value, unit } => {
            format!("{value}{}", unit.map_or("", |u| u.as_str()))
        }
        Value::Bool(b) => b.to_string(),
        Value::Enum(word) if matches!(word.as_str(), "null" | "Null" | "NULL") => {
            single_quoted(word)
        }
        Value::Enum(word) => word.clone(),
        Value::Color(color) => format!("\"{color}\""),
        Value::Token(path) => format!("${}.{path}", value::THEME_ROOT),
        Value::Binding {
            path,
            mode: BindingMode::OneWay,
        } => format!("${path}"),
        Value::Binding {
            path,
            mode: BindingMode::TwoWay,
        } => format!("bind ${path}"),
        Value::List(items) => {
            let items: Vec<String> = items.iter().map(|item| inline(item, true)).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Map(entries) => {
            let items: Vec<String> = entries
                .iter()
                .map(|(key, val)| format!("{}: {}", string_scalar(key, true), inline(val, true)))
                .collect();
            format!("{{{}}}", items.join(", "))
        }
    }
}

fn string_scalar(s: &str, flow: bool) -> String {
    if plain_safe(s, flow)
        && value::parse_value(s, Quoting::Plain) == Ok(Value::String(s.to_owned()))
    {
        s.to_owned()
    } else if s.contains(['\n', '\r', '\t']) {
        double_quoted(s, true)
    } else {
        single_quoted(s)
    }
}

pub(crate) fn handler(handler: &HandlerRef) -> String {
    let args: Vec<String> = handler.args.iter().map(handler_arg).collect();
    let text = format!("{}({})", handler.name, args.join(", "));
    if plain_safe(&text, false) {
        text
    } else {
        single_quoted(&text)
    }
}

fn handler_arg(value: &Value) -> String {
    match value {
        Value::String(s)
            if plain_safe(s, true)
                && !s.contains(['(', ')', '\'', '"'])
                && value::parse_value(s, Quoting::Plain) == Ok(Value::String(s.clone())) =>
        {
            s.clone()
        }
        Value::String(s) => single_quoted(s),
        other => inline(other, true),
    }
}

/// Raw text (version, base, imports, meta): plain when YAML-safe, otherwise quoted.
pub(crate) fn text_scalar(s: &str) -> String {
    if plain_safe(s, false) {
        s.to_owned()
    } else if s.contains(['\n', '\r', '\t']) {
        double_quoted(s, false)
    } else {
        single_quoted(s)
    }
}

/// True when `s` can be written as a plain YAML scalar that no YAML 1.1/1.2 tool re-types.
fn plain_safe(s: &str, flow: bool) -> bool {
    let Some(first) = s.chars().next() else {
        return false;
    };
    !"-?:,[]{}#&*!|>'\"%@`".contains(first)
        && !first.is_whitespace()
        && !s.ends_with(char::is_whitespace)
        && !s.ends_with(':')
        && !s.contains(": ")
        && !s.contains(" #")
        && !s.chars().any(char::is_control)
        && !(flow && s.contains([',', '[', ']', '{', '}']))
        && !matches!(
            s.to_ascii_lowercase().as_str(),
            "~" | "null" | "true" | "false" | "yes" | "no" | "on" | "off" | "y" | "n"
        )
        && !s
            .trim_start_matches(['-', '+', '.'])
            .starts_with(|c: char| c.is_ascii_digit())
}

fn flow_safe(item: &str) -> bool {
    item.starts_with(['"', '\'', '[']) || !item.contains([',', '[', ']', '{', '}'])
}

fn single_quoted(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

fn double_quoted(s: &str, escape_braces: bool) -> String {
    let mut out = String::from('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '{' if escape_braces => out.push_str("{{"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
