//! The preserving writer: splices original source lines for regions whose
//! model still matches, canonically emitting only what changed. Comments,
//! blank lines, anchors, aliases and `<<` merge keys survive a save because
//! they live inside the spliced ranges.

use std::collections::HashSet;

use saphyr::{MarkedYamlOwned, YamlDataOwned};

use crate::diagnostic::Pos;
use crate::model::{Document, Node, Value};
use crate::value::{Quoting, is_prop_key};

use super::read;
use super::tree::{self, Scalar, YNode};
use super::write::{self, WriteOptions};

/// Writes `doc` preserving original source for unchanged regions; `None` when
/// the document was not read from YAML text.
pub(crate) fn preserving(doc: &Document, options: &WriteOptions) -> Option<String> {
    let raw = doc.raw.as_ref()?;
    let YamlDataOwned::Mapping(top) = &raw.tree.data else {
        return None;
    };
    let lines: Vec<&str> = raw.source.lines().collect();
    let ctx = Ctx {
        src: raw.source.as_str(),
        lines: &lines,
        options,
    };

    let mut out = String::new();
    // Next source line (1-based) not yet emitted. Everything before each
    // region is copied verbatim, which is what keeps comments and blank lines.
    let mut cursor = 1usize;
    let mut seen: HashSet<String> = HashSet::new();
    let mut covered_dialogs = vec![false; doc.dialogs.len()];
    // Anchored `name:` blocks already in source — pending refs emit only the
    // names the model has and the source does not (new refnames).
    let existing_refs: HashSet<String> = top
        .iter()
        .filter_map(|(_, v)| anchor_name(&ctx, v))
        .collect();

    for (k, v) in top {
        let a = k.span.start.line();
        let b = end_line(v);
        emit_gap(&mut out, ctx.lines, &mut cursor, a);
        let Some((key, _)) = scalar_text(k) else {
            emit(&mut out, ctx.lines, a, b);
            cursor = b + 1;
            continue;
        };
        let key = key.as_str();
        match key {
            "root" => {
                emit_pending_top(&mut out, doc, &mut seen, &mut covered_dialogs, &existing_refs, &ctx);
                seen.insert("root".to_owned());
                splice_named_node(&ctx, &mut out, k, v, &doc.root, false);
            }
            "$schema" | "forge" | "base" | "use" | "meta" => {
                seen.insert(key.to_owned());
                splice_top_field(&ctx, &mut out, k, v, key, doc);
            }
            _ => match read::element_key(key).filter(|(el, _)| el.name == "Dialog") {
                Some((el, id)) => {
                    let index = doc
                        .dialogs
                        .iter()
                        .position(|d| d.element == el && d.id == id);
                    if let Some(i) = index {
                        covered_dialogs[i] = true;
                        splice_named_node(&ctx, &mut out, k, v, &doc.dialogs[i], true);
                    }
                    // Dialog removed from the model — drop its region.
                }
                None => {
                    // `name: &name` ref blocks splice against `doc.refs` so
                    // in-place edits land in the block, not at `<<` sites.
                    let spliced = anchor_name(&ctx, v).and_then(|name| {
                        doc.refs
                            .iter()
                            .find(|(n, _)| n == &name)
                            .map(|(_, fragment)| (name, fragment))
                    });
                    match spliced {
                        Some((name, fragment)) => {
                            seen.insert(name);
                            emit(&mut out, ctx.lines, a, a);
                            let baseline =
                                reparse(v).and_then(|y| {
                                    read::reparse_fragment(
                                        ctx.src,
                                        y,
                                        tree::merge_names(v, ctx.lines),
                                    )
                                });
                            if baseline.as_ref() == Some(fragment) {
                                emit(&mut out, ctx.lines, a + 1, b);
                            } else {
                                splice_map(
                                    &ctx,
                                    &mut out,
                                    v,
                                    fragment,
                                    baseline.as_ref().unwrap_or(&Node::default()),
                                    false,
                                );
                            }
                        }
                        None => emit(&mut out, ctx.lines, a, b),
                    }
                }
            },
        }
        cursor = b + 1;
    }
    emit_gap(&mut out, ctx.lines, &mut cursor, usize::MAX);
    emit_pending_top(&mut out, doc, &mut seen, &mut covered_dialogs, &existing_refs, &ctx);
    if !seen.contains("root") {
        write::line(&mut out, 0, "root:");
        write::write_node(&mut out, &doc.root, 2, false, ctx.options);
    }
    Some(out)
}

struct Ctx<'a> {
    src: &'a str,
    lines: &'a [&'a str],
    options: &'a WriteOptions,
}

/// Copies source lines `[a, b]` (1-based, inclusive) verbatim.
fn emit(out: &mut String, lines: &[&str], a: usize, b: usize) {
    let end = b.min(lines.len());
    for i in a.max(1)..=end {
        out.push_str(lines[i - 1]);
        out.push('\n');
    }
}

/// Emits everything between the cursor and `upto` (exclusive) verbatim.
fn emit_gap(out: &mut String, lines: &[&str], cursor: &mut usize, upto: usize) {
    if upto > *cursor {
        emit(out, lines, *cursor, upto - 1);
    }
    *cursor = upto.max(*cursor);
}

/// The last source line a node covers. A container's end marker can point at
/// the next sibling, so the true last line comes from the deepest last child.
/// Alias-site clones carry foreign (anchor-site) child spans — anything
/// outside the node's own span falls back to the span's last line.
fn end_line(node: &MarkedYamlOwned) -> usize {
    let end = node.span.end;
    let own_last = if end.col() == 0 && end.line() > node.span.start.line() {
        end.line() - 1
    } else {
        end.line()
    }
    .max(node.span.start.line());
    let inner = match &node.data {
        YamlDataOwned::Mapping(m) => m.back().map(|(k, v)| end_line(v).max(k.span.start.line())),
        YamlDataOwned::Sequence(s) => s.last().map(end_line),
        _ => None,
    };
    match inner {
        Some(inner) if inner <= own_last && inner >= node.span.start.line() => inner,
        _ => own_last,
    }
}

fn scalar_text(node: &MarkedYamlOwned) -> Option<(String, Quoting)> {
    match &node.data {
        YamlDataOwned::Representation(text, style, _) => {
            Some((text.clone(), tree::quoting(*style)))
        }
        YamlDataOwned::Value(scalar) => Some((tree::value_text(scalar.clone()), Quoting::Plain)),
        _ => None,
    }
}

/// The `&name` anchor on a node's span — anchored spans start at or just
/// inside the token (saphyr marks them at the name, after `&`).
fn anchor_name(ctx: &Ctx, node: &MarkedYamlOwned) -> Option<String> {
    let line = ctx
        .lines
        .get(node.span.start.line().saturating_sub(1))?;
    let col = node.span.start.col();
    let text: String = line.chars().skip(col).collect();
    let text = text.trim_start();
    let rest = if let Some(rest) = text.strip_prefix('&') {
        rest
    } else if col > 0 && line.chars().nth(col - 1) == Some('&') {
        text
    } else {
        return None;
    };
    let name: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .collect();
    (!name.is_empty()).then_some(name)
}

/// True when the node's span is an `*alias` token rather than a real subtree —
/// the loader resolves aliases into clones whose outer span still covers the
/// alias site. Only whole-verbatim or whole-canonical output is safe there;
/// descending would splice text from the anchor's lines at the alias site.
fn is_alias_site(ctx: &Ctx, node: &MarkedYamlOwned) -> bool {
    let line = ctx.lines.get(node.span.start.line() - 1).unwrap_or(&"");
    line.chars()
        .skip(node.span.start.col())
        .collect::<String>()
        .trim_start()
        .starts_with('*')
}

fn reparse(node: &MarkedYamlOwned) -> Option<YNode> {
    tree::convert_marked(node).ok()
}

fn reparse_node(ctx: &Ctx, node: &MarkedYamlOwned) -> Option<Node> {
    read::reparse_node(ctx.src, reparse(node)?, tree::merge_names(node, ctx.lines))
}

fn reparse_value(ctx: &Ctx, node: &MarkedYamlOwned, allow_two_way: bool) -> Option<Value> {
    read::reparse_value(ctx.src, reparse(node)?, allow_two_way)
}

/// Canonical lines for top-level fields the model has but the raw document
/// does not (added programmatically), emitted just before `root:`. Marks
/// what it emits so a second call is a no-op.
fn emit_pending_top(
    out: &mut String,
    doc: &Document,
    seen: &mut HashSet<String>,
    covered_dialogs: &mut [bool],
    existing_refs: &HashSet<String>,
    ctx: &Ctx,
) {
    if let Some(schema) = &doc.schema
        && seen.insert("$schema".to_owned())
    {
        write::line(out, 0, &format!("$schema: {}", write::text_scalar(schema)));
    }
    if seen.insert("forge".to_owned()) {
        write::line(out, 0, &format!("forge: \"{}\"", doc.version));
    }
    if let Some(base) = &doc.base
        && seen.insert("base".to_owned())
    {
        write::line(out, 0, &format!("base: {}", write::text_scalar(base)));
    }
    if !doc.imports.is_empty() && seen.insert("use".to_owned()) {
        write::line(out, 0, "use:");
        for import in &doc.imports {
            let spec = match &import.version {
                Some(version) => format!("{} {version}", import.library),
                None => import.library.clone(),
            };
            write::line(
                out,
                2,
                &format!("{}: {}", import.alias, write::text_scalar(&spec)),
            );
        }
    }
    if !doc.meta.is_empty() && seen.insert("meta".to_owned()) {
        write::line(out, 0, "meta:");
        for (key, text) in &doc.meta {
            write::line(out, 2, &format!("{key}: {}", write::text_scalar(text)));
        }
    }
    // New `name: &name` ref blocks — emitted before `root:`/dialogs so every
    // `<<: *name` resolves (aliases can't precede their anchors).
    for (name, fragment) in &doc.refs {
        if !existing_refs.contains(name) && seen.insert(name.clone()) {
            out.push('\n');
            write::line(out, 0, &format!("{name}: &{name}"));
            write::write_entries(out, fragment, 2, ctx.options);
        }
    }
    for (dialog, covered) in doc.dialogs.iter().zip(covered_dialogs.iter_mut()) {
        if !*covered {
            *covered = true;
            out.push('\n');
            write::write_node(out, dialog, 0, false, ctx.options);
        }
    }
}

/// Top-level scalar/map fields — verbatim when the model field matches.
fn splice_top_field(
    ctx: &Ctx,
    out: &mut String,
    k: &MarkedYamlOwned,
    v: &MarkedYamlOwned,
    key: &str,
    doc: &Document,
) {
    let (a, b) = (k.span.start.line(), end_line(v));
    let unchanged = match key {
        "$schema" => doc.schema.as_deref().is_some_and(|s| {
            reparse(v)
                .and_then(|y| read::reparse_text(ctx.src, y))
                .as_deref()
                == Some(s)
        }),
        "forge" => reparse(v)
            .and_then(|y| read::reparse_text(ctx.src, y))
            .is_some_and(|t| t == doc.version),
        "base" => doc.base.as_deref().is_some_and(|s| {
            reparse(v)
                .and_then(|y| read::reparse_text(ctx.src, y))
                .as_deref()
                == Some(s)
        }),
        "use" => reparse(v)
            .map(|y| read::reparse_imports(ctx.src, y))
            .is_some_and(|imports| imports == doc.imports),
        "meta" => reparse(v)
            .map(|y| read::reparse_meta(ctx.src, y))
            .is_some_and(|meta| meta == doc.meta),
        _ => false,
    };
    if unchanged {
        emit(out, ctx.lines, a, b);
    }
    // Changed or removed top fields are dropped. Canonical replacements for
    // schema/base/imports/meta are rare enough that emit_pending_top-style
    // output is skipped here — a changed scalar still needs emitting though.
    else {
        match key {
            "$schema" => {
                if let Some(s) = &doc.schema {
                    write::line(out, 0, &format!("$schema: {s}"));
                }
            }
            "forge" => write::line(out, 0, &format!("forge: {:?}", doc.version)),
            "base" => {
                if let Some(s) = &doc.base {
                    write::line(out, 0, &format!("base: {s}"));
                }
            }
            "use" => {
                if !doc.imports.is_empty() {
                    write::line(out, 0, "use:");
                    for import in &doc.imports {
                        let spec = match &import.version {
                            Some(version) => format!("{} {version}", import.library),
                            None => import.library.clone(),
                        };
                        write::line(out, 2, &format!("{}: {spec}", import.alias));
                    }
                }
            }
            "meta" if !doc.meta.is_empty() => {
                write::line(out, 0, "meta:");
                for (key, text) in &doc.meta {
                    write::line(out, 2, &format!("{key}: {text}"));
                }
            }
            _ => {}
        }
    }
}

/// A `root:` or `Dialog#name:` entry — the key line stays, the body is
/// spliced entry by entry against `model`. For dialogs the key line doubles
/// as the head (`Dialog#x:` is the element), so a rename rewrites it.
fn splice_named_node(
    ctx: &Ctx,
    out: &mut String,
    k: &MarkedYamlOwned,
    v: &MarkedYamlOwned,
    model: &Node,
    dialog: bool,
) {
    let a = k.span.start.line();
    let baseline = baseline_for(ctx, k, v);
    if baseline.as_ref() == Some(model) {
        emit(out, ctx.lines, a, end_line(v));
        return;
    }
    if v.span.start.line() == a {
        // Inline value (`root: {Window: ...}`) — canonical entry + body.
        write::line(
            out,
            k.span.start.col(),
            &format!(
                "{key}:",
                key = { scalar_text(k).map(|(t, _)| t).unwrap_or_default() }
            ),
        );
        write::write_node(out, model, 2, false, ctx.options);
        return;
    }
    let head_changed = dialog
        && baseline
            .as_ref()
            .is_some_and(|b| b.element != model.element || b.id != model.id);
    if head_changed {
        write::write_head(out, model, k.span.start.col(), false, ctx.options);
    } else {
        emit(out, ctx.lines, a, a);
    }
    let Some(baseline) = baseline else {
        write::write_entries(out, model, 2, ctx.options);
        return;
    };
    splice_map(ctx, out, v, model, &baseline, false);
}

/// The original model node a raw value produced — the merge/alias baseline.
fn baseline_for(ctx: &Ctx, k: &MarkedYamlOwned, v: &MarkedYamlOwned) -> Option<Node> {
    if let Some((key, _)) = scalar_text(k)
        && read::element_key(&key).is_some_and(|(el, _)| el.name == "Dialog")
    {
        // `Dialog#x:` — synthesize `{Dialog#x: null, ..body}` like the reader.
        let YNode::Map(body, at) = reparse(v)? else {
            return None;
        };
        let pos = Pos {
            line: k.span.start.line(),
            col: k.span.start.col() + 1,
        };
        let mut entries = vec![(
            Scalar {
                text: key,
                quoting: Quoting::Plain,
                pos,
            },
            YNode::Scalar(Scalar {
                text: String::new(),
                quoting: Quoting::Plain,
                pos,
            }),
        )];
        entries.extend(body);
        return read::reparse_node(ctx.src, YNode::Map(entries, at), Default::default());
    }
    reparse_node(ctx, v)
}

/// Entry-by-entry splice of a node body map against `model`; `baseline` is the
/// node this map originally produced. Explicit keys win over `<<` merges, so
/// changed merge-derived facets are written as explicit overrides.
fn splice_map(
    ctx: &Ctx,
    out: &mut String,
    map: &MarkedYamlOwned,
    model: &Node,
    baseline: &Node,
    seq_item: bool,
) {
    let YamlDataOwned::Mapping(entries) = &map.data else {
        write::write_entries(out, model, fallback_indent(map), ctx.options);
        return;
    };
    let indent = entries
        .front()
        .map(|(k, _)| k.span.start.col())
        .unwrap_or_else(|| map.span.start.col());
    let explicit: HashSet<String> = entries
        .keys()
        .filter_map(|k| scalar_text(k).map(|(t, _)| t))
        .collect();

    let head_index = entries.keys().position(|k| {
        scalar_text(k)
            .and_then(|(t, _)| read::element_key(&t))
            .is_some_and(|(el, id)| {
                el == baseline.element && id.as_deref() == baseline.id.as_deref()
            })
    });

    // A merged facet cannot be removed by an override — flatten the node.
    // Control flow splices all-or-nothing too: its keys (`if`, `else`,
    // `case`…) rewrite canonically whenever the facet differs. The head
    // (`- Button:` / `- if: …`) lives inside this map for seq items and root
    // nodes, so it is rewritten here. Dialog heads live outside.
    let flattened = baseline
        .props
        .iter()
        .any(|(k, _)| !explicit.contains(k) && !model.props.iter().any(|(mk, _)| mk == k))
        || baseline.events.iter().any(|e| {
            !explicit.contains(&format!("on {}", e.event))
                && !model.events.iter().any(|me| me.event == e.event)
        })
        || baseline.control != model.control
        || baseline.refname != model.refname;
    if flattened {
        if seq_item {
            write::write_node(out, model, indent.saturating_sub(2), true, ctx.options);
        } else if head_index.is_some() || model.is_block() {
            write::write_node(out, model, indent, false, ctx.options);
        } else {
            write::write_entries(out, model, indent, ctx.options);
        }
        return;
    }

    // Head came from a merge and changed — emit an explicit head first.
    let head_differs = model.element != baseline.element
        || model.id != baseline.id
        || model.content != baseline.content;
    if head_index.is_none() && head_differs {
        write::write_head(out, model, indent, seq_item, ctx.options);
    }

    let mut cursor = map.span.start.line();
    for (i, (k, v)) in entries.iter().enumerate() {
        let a = k.span.start.line();
        let b = end_line(v);
        emit_gap(out, ctx.lines, &mut cursor, a);
        let Some((key, _)) = scalar_text(k) else {
            emit(out, ctx.lines, a, b);
            cursor = b + 1;
            continue;
        };
        let col = k.span.start.col();
        let key = key.as_str();
        if Some(i) == head_index {
            if head_differs {
                let dash = col.saturating_sub(2);
                write::write_head(
                    out,
                    model,
                    if seq_item { dash } else { col },
                    seq_item,
                    ctx.options,
                );
            } else {
                emit(out, ctx.lines, a, b);
            }
        } else if key == "<<" {
            emit(out, ctx.lines, a, b);
        } else if is_control_key(key) {
            // Reached only when `control` is unchanged (changes flatten) —
            // `if`/`else`/`is`/`each`/`as` lines keep their source.
            emit(out, ctx.lines, a, b);
        } else if key == "style" {
            splice_style(ctx, out, k, v, model, baseline, a, b);
        } else if key == "children" {
            splice_children(ctx, out, k, v, model, baseline, a, b);
        } else if let Some(event) = key.strip_prefix("on ") {
            match model.events.iter().find(|e| e.event == event.trim()) {
                Some(binding)
                    if baseline.events.iter().find(|e| e.event == binding.event)
                        == Some(binding) =>
                {
                    emit(out, ctx.lines, a, b)
                }
                Some(binding) => write::write_event(out, col, binding),
                None => {}
            }
        } else if is_prop_key(key) {
            match model.props.iter().find(|(mk, _)| mk == key) {
                Some((_, value)) if reparse_value(ctx, v, true).as_ref() == Some(value) => {
                    emit(out, ctx.lines, a, b)
                }
                Some((_, value)) => write::write_entry(out, col, key, value),
                None => {}
            }
        } else {
            emit(out, ctx.lines, a, b);
        }
        cursor = b + 1;
    }
    emit_gap(out, ctx.lines, &mut cursor, end_line(map) + 1);

    // Facets with no explicit entry: emit when they differ from the baseline
    // (new keys, or explicit overrides for changed merge-derived content).
    // Merge-derived facets are owned by the `name:` ref block — they splice
    // there in place and are never rewritten as `<<`-site overrides.
    let merged = |key: &str| model.refname.is_some() && model.merged.iter().any(|k| k == key);
    for (key, value) in &model.props {
        let unchanged = baseline
            .props
            .iter()
            .any(|(bk, bv)| bk == key && bv == value);
        if !explicit.contains(key) && !unchanged && !merged(key) {
            write::write_entry(out, indent, key, value);
        }
    }
    if model.style != baseline.style && !explicit.contains("style") && !merged("style") {
        write::line(out, indent, "style:");
        for (key, value) in &model.style {
            write::write_entry(out, indent + 2, key, value);
        }
    }
    for binding in &model.events {
        let key = format!("on {}", binding.event);
        let same = baseline
            .events
            .iter()
            .any(|e| e.event == binding.event && e == binding);
        if !explicit.contains(&key) && !same && !merged(&key) {
            write::write_event(out, indent, binding);
        }
    }
    if model.children != baseline.children && !explicit.contains("children") && !merged("children") {
        if model.children.is_empty() {
            write::line(out, indent, "children: []");
        } else {
            write::line(out, indent, "children:");
            for child in &model.children {
                write::write_node(out, child, indent + 2, true, ctx.options);
            }
        }
    }
}

/// `style:` block — verbatim when unchanged, per-entry splice otherwise.
#[allow(clippy::too_many_arguments)]
fn splice_style(
    ctx: &Ctx,
    out: &mut String,
    k: &MarkedYamlOwned,
    v: &MarkedYamlOwned,
    model: &Node,
    baseline: &Node,
    a: usize,
    b: usize,
) {
    if model.style == baseline.style {
        emit(out, ctx.lines, a, b);
        return;
    }
    emit(out, ctx.lines, a, a);
    let YamlDataOwned::Mapping(entries) = &v.data else {
        for (key, value) in &model.style {
            write::write_entry(out, k.span.start.col() + 2, key, value);
        }
        return;
    };
    let indent = entries
        .front()
        .map(|(sk, _)| sk.span.start.col())
        .unwrap_or(k.span.start.col() + 2);
    let explicit: HashSet<String> = entries
        .keys()
        .filter_map(|sk| scalar_text(sk).map(|(t, _)| t))
        .collect();
    let mut cursor = a + 1;
    for (sk, sv) in entries {
        let (sa, sb) = (sk.span.start.line(), end_line(sv));
        emit_gap(out, ctx.lines, &mut cursor, sa);
        if let Some((key, _)) = scalar_text(sk) {
            match model.style.iter().find(|(mk, _)| mk == &key) {
                Some((_, value)) if reparse_value(ctx, sv, false).as_ref() == Some(value) => {
                    emit(out, ctx.lines, sa, sb)
                }
                Some((_, value)) => write::write_entry(out, sk.span.start.col(), &key, value),
                None => {}
            }
        } else {
            emit(out, ctx.lines, sa, sb);
        }
        cursor = sb + 1;
    }
    emit_gap(out, ctx.lines, &mut cursor, end_line(v) + 1);
    for (key, value) in &model.style {
        let unchanged = baseline
            .style
            .iter()
            .any(|(bk, bv)| bk == key && bv == value);
        if !explicit.contains(key) && !unchanged {
            write::write_entry(out, indent, key, value);
        }
    }
}

/// `children:` block — verbatim when unchanged, per-item splice when the
/// length matches, canonical replacement otherwise.
#[allow(clippy::too_many_arguments)]
fn splice_children(
    ctx: &Ctx,
    out: &mut String,
    k: &MarkedYamlOwned,
    v: &MarkedYamlOwned,
    model: &Node,
    baseline: &Node,
    a: usize,
    b: usize,
) {
    if model.children == baseline.children {
        emit(out, ctx.lines, a, b);
        return;
    }
    emit(out, ctx.lines, a, a);
    let item_indent = k.span.start.col() + 2;
    let YamlDataOwned::Sequence(items) = &v.data else {
        // `children:` was already emitted; a bare key reads as an empty list.
        for child in &model.children {
            write::write_node(out, child, item_indent, true, ctx.options);
        }
        return;
    };
    if items.len() != model.children.len() {
        for child in &model.children {
            write::write_node(out, child, item_indent, true, ctx.options);
        }
        return;
    }
    let mut cursor = a + 1;
    for (i, (item, child)) in items.iter().zip(&model.children).enumerate() {
        let (ia, ib) = (item.span.start.line(), end_line(item));
        emit_gap(out, ctx.lines, &mut cursor, ia);
        let baseline_child = &baseline.children[i];
        if baseline_child == child {
            emit(out, ctx.lines, ia, ib);
        } else if is_alias_site(ctx, item) || !matches!(item.data, YamlDataOwned::Mapping(_)) {
            write::write_node(
                out,
                child,
                item.span.start.col().saturating_sub(2),
                true,
                ctx.options,
            );
        } else {
            splice_map(ctx, out, item, child, baseline_child, true);
        }
        cursor = ib + 1;
    }
    emit_gap(out, ctx.lines, &mut cursor, end_line(v) + 1);
}

fn fallback_indent(node: &MarkedYamlOwned) -> usize {
    node.span.start.col() + 2
}

/// Control-flow keys inside a node map — prop-shaped, so they must be
/// intercepted before the prop branch. `case <v>` keys contain a space and
/// never reach the prop branch anyway.
fn is_control_key(key: &str) -> bool {
    matches!(
        key,
        "if" | "for" | "with" | "match" | "else" | "is" | "not" | "each" | "index" | "as" | "key"
    )
}
