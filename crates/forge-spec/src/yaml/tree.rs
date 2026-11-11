//! A minimal YAML tree with source positions, built from `saphyr` documents.
//!
//! `MarkedYamlOwned` gives us YAML-1.2-compliant parsing (anchors, aliases and
//! every structural edge case) plus source spans; we translate it into `YNode`,
//! which carries the quoting style the write path needs, resolves `<<` merge
//! keys, and rejects YAML features Forge does not support (tags).

use saphyr::{LoadableYamlNode, MarkedYamlOwned, ScalarStyle, YamlDataOwned};

use crate::diagnostic::{Diagnostic, Pos};
use crate::value::Quoting;

#[derive(Clone)]
pub(crate) struct Scalar {
    pub text: String,
    pub quoting: Quoting,
    pub pos: Pos,
}

impl Scalar {
    pub fn is_null(&self) -> bool {
        self.quoting == Quoting::Plain
            && matches!(self.text.as_str(), "" | "~" | "null" | "Null" | "NULL")
    }
}

#[derive(Clone)]
pub(crate) enum YNode {
    Scalar(Scalar),
    Seq(Vec<YNode>, Pos),
    Map(Vec<(Scalar, YNode)>, Pos),
}

impl YNode {
    pub fn pos(&self) -> Pos {
        match self {
            YNode::Scalar(s) => s.pos,
            YNode::Seq(_, pos) | YNode::Map(_, pos) => *pos,
        }
    }
}

fn pos(span: &saphyr_parser::Span) -> Pos {
    Pos {
        line: span.start.line(),
        col: span.start.col() + 1,
    }
}

/// Parses a single YAML document; `None` when the input is empty. Returns the
/// `YNode` tree plus the marked root, which keeps source spans for the
/// lossless writer.
pub(crate) fn parse(src: &str) -> Result<Option<(YNode, MarkedYamlOwned)>, Diagnostic> {
    let docs = MarkedYamlOwned::load_from_str(src).map_err(|e| {
        Diagnostic::new(
            Pos {
                line: e.marker().line(),
                col: e.marker().col() + 1,
            },
            e.info().to_owned(),
        )
    })?;
    let mut root = None;
    for doc in docs {
        if is_empty_document(&doc) {
            continue;
        }
        if root.is_some() {
            return Err(Diagnostic::new(
                pos(&doc.span),
                "a Forge file holds exactly one YAML document",
            ));
        }
        root = Some((convert(doc.clone())?, doc));
    }
    Ok(root)
}

/// Re-parses a raw marked node into a `YNode`, resolving merges like `convert`.
pub(crate) fn convert_marked(node: &MarkedYamlOwned) -> Result<YNode, Diagnostic> {
    convert(node.clone())
}

/// `<<` merge alias names by map position — `<<: *name` and `<<: [*a, *b]`.
/// The loader resolves aliases into clones whose spans still cover the `*name`
/// token at the merge site, so names come from the source text at those spans.
/// What a map's `<<` merges contributed — the alias names (`*name`) and the
/// entry keys spliced in (explicit keys win, so contributed = merged minus
/// explicit). The writer uses `keys` to avoid re-emitting merged entries.
#[derive(Clone, Debug, Default)]
pub(crate) struct MergeInfo {
    pub names: Vec<String>,
    pub keys: Vec<String>,
}

pub(crate) fn merge_names(
    root: &MarkedYamlOwned,
    lines: &[&str],
) -> std::collections::HashMap<Pos, MergeInfo> {
    let mut out = std::collections::HashMap::new();
    walk_merges(root, lines, &mut out);
    out
}

fn key_text(node: &MarkedYamlOwned) -> Option<String> {
    match &node.data {
        YamlDataOwned::Representation(text, _, _) => Some(text.clone()),
        YamlDataOwned::Value(saphyr::ScalarOwned::String(text)) => Some(text.clone()),
        _ => None,
    }
}

fn walk_merges(
    node: &MarkedYamlOwned,
    lines: &[&str],
    out: &mut std::collections::HashMap<Pos, MergeInfo>,
) {
    match &node.data {
        YamlDataOwned::Mapping(map) => {
            let mut info = MergeInfo::default();
            let explicit: std::collections::HashSet<String> = map
                .keys()
                .filter_map(|k| key_text(k).filter(|t| t != "<<"))
                .collect();
            for (key, value) in map {
                if key_text(key).as_deref() == Some("<<") {
                    alias_names(value, lines, &mut info.names);
                    merge_keys(value, &explicit, &mut info.keys);
                }
                walk_merges(key, lines, out);
                walk_merges(value, lines, out);
            }
            if !info.names.is_empty() {
                out.insert(pos(&node.span), info);
            }
        }
        YamlDataOwned::Sequence(items) => {
            for item in items {
                walk_merges(item, lines, out);
            }
        }
        _ => {}
    }
}

/// Entry keys a `<<` value contributes — mapping keys not shadowed by
/// explicit entries (and, for merge lists, not already contributed).
fn merge_keys(
    node: &MarkedYamlOwned,
    explicit: &std::collections::HashSet<String>,
    keys: &mut Vec<String>,
) {
    match &node.data {
        YamlDataOwned::Mapping(map) => {
            for key in map.keys() {
                if let Some(text) = key_text(key)
                    && !explicit.contains(&text)
                    && !keys.contains(&text)
                {
                    keys.push(text);
                }
            }
        }
        YamlDataOwned::Sequence(items) => {
            for item in items {
                merge_keys(item, explicit, keys);
            }
        }
        _ => {}
    }
}

/// `&name` anchor definitions — Forge requires unique ref names even though
/// the YAML spec permits shadowing (nearest wins): refs are addressed by name.
pub(crate) fn anchor_names(root: &MarkedYamlOwned, lines: &[&str]) -> Vec<(String, Pos)> {
    let mut out = Vec::new();
    walk_anchors(root, lines, &mut out);
    out
}

fn walk_anchors(node: &MarkedYamlOwned, lines: &[&str], out: &mut Vec<(String, Pos)>) {
    if let Some(name) = anchor_at(&node.span, lines) {
        out.push((name, pos(&node.span)));
    }
    match &node.data {
        YamlDataOwned::Mapping(map) => {
            for (key, value) in map {
                walk_anchors(key, lines, out);
                walk_anchors(value, lines, out);
            }
        }
        YamlDataOwned::Sequence(items) => {
            for item in items {
                walk_anchors(item, lines, out);
            }
        }
        _ => {}
    }
}

/// The `&name` token at a node's own span, if it carries an anchor.
fn anchor_at(span: &saphyr_parser::Span, lines: &[&str]) -> Option<String> {
    let line = lines.get(span.start.line().saturating_sub(1))?;
    let text: String = line.chars().skip(span.start.col()).collect();
    let rest = text.trim_start().strip_prefix('&')?;
    let name: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .collect();
    (!name.is_empty()).then_some(name)
}

/// The `*name` token at a node's own span — recursing into sequences for
/// `<<: [*a, *b]` merge lists.
fn alias_names(node: &MarkedYamlOwned, lines: &[&str], names: &mut Vec<String>) {
    if let YamlDataOwned::Sequence(items) = &node.data {
        for item in items {
            alias_names(item, lines, names);
        }
        return;
    }
    let Some(line) = lines.get(node.span.start.line().saturating_sub(1)) else {
        return;
    };
    let text: String = line.chars().skip(node.span.start.col()).collect();
    let text = text.trim_start();
    if let Some(rest) = text.strip_prefix('*') {
        let name: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .collect();
        if !name.is_empty() {
            names.push(name);
        }
    }
}

/// `---` separators and comment-only documents load as a bare scalar.
fn is_empty_document(node: &MarkedYamlOwned) -> bool {
    match &node.data {
        YamlDataOwned::Representation(text, _, _) => text.trim().is_empty(),
        YamlDataOwned::Value(v) => matches!(v, saphyr::ScalarOwned::Null),
        _ => false,
    }
}

fn convert(node: MarkedYamlOwned) -> Result<YNode, Diagnostic> {
    let at = pos(&node.span);
    match node.data {
        YamlDataOwned::Representation(text, style, tag) => {
            reject_tag(at, tag.is_some())?;
            Ok(YNode::Scalar(Scalar {
                text,
                quoting: quoting(style),
                pos: at,
            }))
        }
        YamlDataOwned::Value(scalar) => Ok(YNode::Scalar(Scalar {
            text: value_text(scalar),
            // Resolved values have no quoting distinction left.
            quoting: Quoting::Plain,
            pos: at,
        })),
        YamlDataOwned::Sequence(items) => Ok(YNode::Seq(
            items.into_iter().map(convert).collect::<Result<_, _>>()?,
            at,
        )),
        YamlDataOwned::Mapping(map) => {
            let mut entries = Vec::new();
            let mut merged = Vec::new();
            for (key, value) in map {
                let key = match convert(key)? {
                    YNode::Scalar(key) => key,
                    other => {
                        return Err(Diagnostic::new(
                            other.pos(),
                            "mapping keys must be plain text",
                        ));
                    }
                };
                let value = convert(value)?;
                // `<<` merge key — splice the aliased mapping's entries in.
                if key.quoting == Quoting::Plain && key.text == "<<" {
                    collect_merge(value, &key, &mut merged)?;
                    continue;
                }
                entries.push((key, value));
            }
            // Explicit keys win; merged keys dedup against them and each other,
            // first occurrence taking precedence per the YAML spec.
            for (k, v) in merged {
                if !entries.iter().any(|(ek, _)| ek.text == k.text) {
                    entries.push((k, v));
                }
            }
            Ok(YNode::Map(entries, at))
        }
        YamlDataOwned::Tagged(_, _) => Err(Diagnostic::new(
            at,
            "YAML tags (`!tag`) are not supported in Forge documents",
        )),
        YamlDataOwned::Alias(_) => Err(Diagnostic::new(
            at,
            "alias references an anchor that has not been defined yet",
        )),
        YamlDataOwned::BadValue => Err(Diagnostic::new(at, "invalid YAML value")),
    }
}

pub(crate) fn quoting(style: ScalarStyle) -> Quoting {
    match style {
        ScalarStyle::Plain => Quoting::Plain,
        ScalarStyle::DoubleQuoted => Quoting::Double,
        // Single-quoted and block scalars are literal text.
        _ => Quoting::Single,
    }
}

/// A resolved scalar, rendered back to its text form.
pub(crate) fn value_text(scalar: saphyr::ScalarOwned) -> String {
    match scalar {
        saphyr::ScalarOwned::Null => String::new(),
        saphyr::ScalarOwned::Boolean(b) => b.to_string(),
        saphyr::ScalarOwned::Integer(i) => i.to_string(),
        saphyr::ScalarOwned::FloatingPoint(f) => f.to_string(),
        saphyr::ScalarOwned::String(s) => s,
    }
}

/// Splices `<<` merge values — a mapping or a sequence of mappings — into `out`.
fn collect_merge(
    node: YNode,
    key: &Scalar,
    out: &mut Vec<(Scalar, YNode)>,
) -> Result<(), Diagnostic> {
    match node {
        YNode::Map(entries, _) => {
            out.extend(entries);
            Ok(())
        }
        YNode::Seq(items, _) => {
            // Earlier maps take precedence over later ones per the spec.
            for item in items {
                collect_merge(item, key, out)?;
            }
            Ok(())
        }
        other => Err(Diagnostic::new(
            key.pos,
            format!(
                "merge key `<<` requires a mapping or a list of mappings, found {}",
                match other {
                    YNode::Scalar(_) => "a scalar",
                    _ => "a non-mapping value",
                }
            ),
        )),
    }
}

fn reject_tag(at: Pos, tagged: bool) -> Result<(), Diagnostic> {
    if tagged {
        return Err(Diagnostic::new(
            at,
            "YAML tags (`!tag`) are not supported in Forge documents",
        ));
    }
    Ok(())
}
