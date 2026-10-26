//! A minimal YAML tree with source positions, built from parser events.

use saphyr_parser::{Event, Parser, ScalarStyle, ScanError, Span};

use crate::diagnostic::{Diagnostic, Pos};
use crate::value::Quoting;

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

type Item<'a> = Result<(Event<'a>, Span), ScanError>;

fn pos(span: &Span) -> Pos {
    Pos {
        line: span.start.line(),
        col: span.start.col() + 1,
    }
}

fn next<'a>(events: &mut impl Iterator<Item = Item<'a>>) -> Result<(Event<'a>, Span), Diagnostic> {
    match events.next() {
        Some(Ok(event)) => Ok(event),
        Some(Err(e)) => Err(Diagnostic::new(
            Pos {
                line: e.marker().line(),
                col: e.marker().col() + 1,
            },
            e.info().to_owned(),
        )),
        None => Err(Diagnostic::new(Pos::start(), "unexpected end of input")),
    }
}

/// Parses a single YAML document; `None` when the input is empty.
pub(crate) fn parse(src: &str) -> Result<Option<YNode>, Diagnostic> {
    let mut events = Parser::new_from_str(src);
    let mut root = None;
    loop {
        let (event, _) = next(&mut events)?;
        match event {
            Event::StreamEnd => return Ok(root),
            Event::DocumentStart(_) => {
                let first = next(&mut events)?;
                if matches!(first.0, Event::DocumentEnd) {
                    continue;
                }
                if root.is_some() {
                    return Err(Diagnostic::new(
                        pos(&first.1),
                        "a Forge file holds exactly one YAML document",
                    ));
                }
                root = Some(build(first, &mut events)?);
            }
            _ => {}
        }
    }
}

fn build<'a>(
    (event, span): (Event<'a>, Span),
    events: &mut impl Iterator<Item = Item<'a>>,
) -> Result<YNode, Diagnostic> {
    let at = pos(&span);
    match event {
        Event::Scalar(text, style, anchor, tag) => {
            reject_extras(at, anchor, tag.is_some())?;
            let quoting = match style {
                ScalarStyle::Plain => Quoting::Plain,
                ScalarStyle::DoubleQuoted => Quoting::Double,
                // Single-quoted and block scalars are literal text.
                _ => Quoting::Single,
            };
            Ok(YNode::Scalar(Scalar {
                text: text.into_owned(),
                quoting,
                pos: at,
            }))
        }
        Event::SequenceStart(anchor, tag) => {
            reject_extras(at, anchor, tag.is_some())?;
            let mut items = Vec::new();
            loop {
                let item = next(events)?;
                if matches!(item.0, Event::SequenceEnd) {
                    return Ok(YNode::Seq(items, at));
                }
                items.push(build(item, events)?);
            }
        }
        Event::MappingStart(anchor, tag) => {
            reject_extras(at, anchor, tag.is_some())?;
            let mut entries = Vec::new();
            loop {
                let key = next(events)?;
                if matches!(key.0, Event::MappingEnd) {
                    return Ok(YNode::Map(entries, at));
                }
                let key = match build(key, events)? {
                    YNode::Scalar(key) => key,
                    other => {
                        return Err(Diagnostic::new(
                            other.pos(),
                            "mapping keys must be plain text",
                        ));
                    }
                };
                let value = build(next(events)?, events)?;
                entries.push((key, value));
            }
        }
        Event::Alias(_) => Err(Diagnostic::new(
            at,
            "YAML aliases (`*name`) are not supported in Forge documents",
        )),
        _ => Err(Diagnostic::new(at, "unexpected YAML structure")),
    }
}

fn reject_extras(at: Pos, anchor: usize, tagged: bool) -> Result<(), Diagnostic> {
    if anchor != 0 {
        return Err(Diagnostic::new(
            at,
            "YAML anchors (`&name`) are not supported in Forge documents",
        ));
    }
    if tagged {
        return Err(Diagnostic::new(
            at,
            "YAML tags (`!tag`) are not supported in Forge documents",
        ));
    }
    Ok(())
}
