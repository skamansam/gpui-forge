//! Adapter for `gpui-video-player` (`use: video: gpui-video-player` →
//! `video.*` elements) — #77.
//!
//! Two build variants, selected by feature to match the runtime's base:
//! `base-gpui-kit` (default) links the crate's `gpui-pre` feature, whose
//! `gpui_pre =0.3.9` pin is the gpui-kit base's own. `base-gpui02` links its
//! `gpui` (crates.io 0.2) feature for the Vanilla GPUI 0.2 base.
//!
//! GStreamer 1.14+ and its plugins must be installed — see
//! <https://github.com/sdroege/gstreamer-rs#installation>.
//!
//! `Video` handles are whole GStreamer pipelines, so the adapter caches one
//! per node key and only rebuilds it when `src` changes. Control props
//! (`playing`/`volume`/`muted`/`looping`) are applied every render — they're
//! cheap setter calls on the handle.

// `gpui_video_player::gpui` re-exports whichever GPUI build the selected
// `base-*` feature links — always the same crate the runtime's types come
// from, on both variants.
use gpui_forge_runtime::forge_spec::catalog::{self, Catalog};
use gpui_forge_runtime::forge_spec::{Node, Value};
use gpui_forge_runtime::render::{ForgeLibrary, ForgeView, get_path};
use gpui_video_player::gpui::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::OnceLock;

/// `video.*` adapter — register with `view.register_library(VideoPlayer::new())`.
#[derive(Default)]
pub struct VideoPlayer {
    /// `key → (src, Video)` — the pipeline is rebuilt only when `src` changes.
    players: RefCell<HashMap<String, (String, gpui_video_player::Video)>>,
}

impl VideoPlayer {
    pub fn new() -> Self {
        Self::default()
    }

    /// The `src`'s cached pipeline, creating or replacing it on change.
    /// `src` may be an absolute URL or a doc-relative file path.
    fn player(&self, key: &str, src: &str, view: &ForgeView) -> Option<gpui_video_player::Video> {
        let url = url::Url::parse(src)
            .ok()
            .or_else(|| url::Url::from_file_path(view.resolve_path(src)).ok())?;
        let mut players = self.players.borrow_mut();
        match players.entry(key.to_owned()) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                if entry.get().0 != src {
                    entry.insert((src.to_owned(), gpui_video_player::Video::new(&url).ok()?));
                }
                Some(entry.into_mut().1.clone())
            }
            std::collections::hash_map::Entry::Vacant(entry) => Some(
                entry
                    .insert((src.to_owned(), gpui_video_player::Video::new(&url).ok()?))
                    .1
                    .clone(),
            ),
        }
    }
}

impl ForgeLibrary for VideoPlayer {
    fn alias(&self) -> &'static str {
        "video"
    }

    fn catalog(&self) -> Option<&Catalog> {
        static CAT: OnceLock<Catalog> = OnceLock::new();
        Some(CAT.get_or_init(|| {
            catalog::from_yaml(include_str!("../../../catalogs/video.catalog.yaml"))
                .expect("video catalog parses")
        }))
    }

    fn render(
        &self,
        node: &Node,
        key: &str,
        view: &ForgeView,
        _window: &mut Window,
        _cx: &mut Context<ForgeView>,
    ) -> Option<AnyElement> {
        match node.element.name.as_str() {
            "Player" => {
                let src = prop_str(node, "src", view)?;
                let video = self.player(key, &src, view)?;
                video.set_paused(!prop_bool(node, "playing", view, true));
                if let Some(volume) = prop_num(node, "volume", view) {
                    video.set_volume(volume.clamp(0.0, 1.0));
                }
                video.set_muted(prop_bool(node, "muted", view, false));
                video.set_looping(prop_bool(node, "looping", view, false));
                let mut el = gpui_video_player::video(video).id(SharedString::from(key.to_owned()));
                if let Some(w) = prop_num(node, "width", view) {
                    el = el.width(px(w as f32));
                }
                if let Some(h) = prop_num(node, "height", view) {
                    el = el.height(px(h as f32));
                }
                if let Some(cap) = prop_num(node, "buffer_capacity", view) {
                    el = el.buffer_capacity(cap as usize);
                }
                Some(el.into_any_element())
            }
            _ => None,
        }
    }
}

fn prop<'a>(node: &'a Node, name: &str) -> Option<&'a Value> {
    node.props.iter().find(|(k, _)| k == name).map(|(_, v)| v)
}

/// A `string`/`enum`/literal prop — `{{binding}}` values resolve against the
/// view's state so `playing: {{stream.playing}}` works one-way.
fn prop_str(node: &Node, name: &str, view: &ForgeView) -> Option<String> {
    match prop(node, name)? {
        Value::String(s) | Value::Enum(s) => Some(s.clone()),
        Value::Number { value, .. } => Some(value.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Binding { path, .. } => get_path(view.state(), path).map(|j| match j {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        }),
        _ => None,
    }
}

fn prop_bool(node: &Node, name: &str, view: &ForgeView, fallback: bool) -> bool {
    match prop(node, name) {
        Some(Value::Bool(b)) => *b,
        Some(Value::Binding { path, .. }) => get_path(view.state(), path)
            .and_then(|j| j.as_bool())
            .unwrap_or(fallback),
        _ => fallback,
    }
}

fn prop_num(node: &Node, name: &str, view: &ForgeView) -> Option<f64> {
    match prop(node, name) {
        Some(Value::Number { value, .. }) => Some(*value),
        Some(Value::Binding { path, .. }) => get_path(view.state(), path).and_then(|j| j.as_f64()),
        _ => None,
    }
}
