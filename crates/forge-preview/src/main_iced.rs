//! `forge-preview-iced` — the out-of-process preview host for the iced base
//! (#113). Same JSON wire protocol. The host is an `iced::application` whose
//! update loop bridges the channel into the view's Elm messages.

use gpui_forge_runtime::check;
use gpui_forge_runtime::render::{ForgeView, Message as ForgeMessage};
use iced::{Element, Task};

use serde_json::json;
use std::sync::mpsc;
use std::time::Duration;

mod protocol;
use protocol::{HostMsg, emit, spawn_reader};

struct Host {
    view: ForgeView,
    rx: mpsc::Receiver<HostMsg>,
    ticks: u64,
    theme: iced::Theme,
}

#[derive(Debug, Clone)]
enum Message {
    Forge(ForgeMessage),
    Tick,
    Quit,
    /// #120 Phase 2 — a captured frame from `iced::window::screenshot`.
    Screenshot(iced::window::Screenshot),
}

impl Host {
    fn new(rx: mpsc::Receiver<HostMsg>) -> Self {
        let mut view = ForgeView::new(check::catalogs());
        view.set_sinks(
            std::rc::Rc::new(|call, _| {
                emit(json!({"event":"call","call": call}));
            }),
            std::rc::Rc::new(|state, _| {
                emit(json!({"event":"state","state": state}));
            }),
        );
        Self {
            view,
            rx,
            ticks: 0,
            theme: iced::Theme::Dark,
        }
    }

    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::Forge(m) => self.view.update(m),
            Message::Tick => {
                while let Ok(msg) = self.rx.try_recv() {
                    match msg {
                        HostMsg::Source { src, dir } => {
                            if let Some(dir) = dir {
                                self.view.set_base_dir(dir);
                            }
                            let catalogs = self.view.catalogs.clone();
                            let (doc, diags) =
                                check::check(&src, gpui_forge_runtime::base::BASE_ID, &catalogs);
                            emit(json!({"event":"diagnostics","items":
                                check::diagnostics_json(&diags)}));
                            if let Some(doc) = doc {
                                self.view.set_document(doc);
                            }
                        }
                        HostMsg::Pick(on) => self.view.set_pick(on),
                        HostMsg::Place(on) => self.view.set_placing(on),
                        HostMsg::Dnd(on) => self.view.set_dnd(on),
                        HostMsg::Select(path) => self.view.update_state(
                            &gpui_forge_runtime::forge_spec::value::parse_path("document.selected")
                                .unwrap(),
                            serde_json::Value::String(path),
                        ),
                        HostMsg::Theme { dark, colors, .. } => {
                            // Forge theme names don't resolve in iced — the
                            // wire palette does: build a custom Theme from
                            // Studio's resolved colors over a dark/light base.
                            let c =
                                |k: &str| {
                                    protocol::theme_color(&colors, k)
                                        .map(|(r, g, b, a)| iced::Color { r, g, b, a })
                                };
                            let mut p = if dark {
                                iced::theme::Palette::DARK
                            } else {
                                iced::theme::Palette::LIGHT
                            };
                            if let Some(v) = c("background") {
                                p.background = v;
                            }
                            if let Some(v) = c("foreground") {
                                p.text = v;
                            }
                            if let Some(v) = c("accent") {
                                p.primary = v;
                            }
                            if let Some(v) = c("success") {
                                p.success = v;
                            }
                            if let Some(v) = c("danger") {
                                p.danger = v;
                            }
                            self.theme = iced::Theme::custom("forge".to_owned(), p);
                        }
                        HostMsg::Raise => {
                            return iced::window::get_latest().then(|id| {
                                id.map(|id| iced::window::gain_focus(id))
                                    .unwrap_or(Task::none())
                            });
                        }
                        HostMsg::FrameRect { x, y, w, h } => {
                            // #120 Phase 1 — iced can do the full frame_rect:
                            // `winit` position + size.
                            return iced::window::get_latest().then(move |id| {
                                id.map(|id| {
                                    Task::batch([
                                        iced::window::move_to(
                                            id,
                                            iced::Point {
                                                x: x as f32,
                                                y: y as f32,
                                            },
                                        ),
                                        iced::window::resize(
                                            id,
                                            iced::Size {
                                                width: w as f32,
                                                height: h as f32,
                                            },
                                        ),
                                    ])
                                })
                                .unwrap_or(Task::none())
                            });
                        }
                        HostMsg::Quit => return Task::done(Message::Quit),
                    }
                }
                self.ticks += 1;
                if self.ticks % 20 == 0 {
                    // #120 Phase 2 — stream real pixels: screenshot →
                    // frame_rgba (base64) for Studio's embedded card.
                    return Task::batch([
                        iced::window::get_latest().then(|id| {
                            id.map(|id| iced::window::screenshot(id).map(Message::Screenshot))
                                .unwrap_or(Task::none())
                        }),
                        Task::perform(
                            async { std::thread::sleep(Duration::from_millis(50)) },
                            |_| Message::Tick,
                        ),
                    ]);
                }
                return Task::perform(
                    async { std::thread::sleep(Duration::from_millis(50)) },
                    |_| Message::Tick,
                );
            }
            Message::Quit => return iced::exit(),
            Message::Screenshot(shot) => {
                // #120 Phase 2 — downsample the rgba to a luminance text
                // grid. `frame` events share ratatui's wire + renderer
                // (`preview.frame` → `{$preview.frame}` Text in the card).
                let bytes = shot.as_ref();
                let (w, h) = (shot.size.width as usize, shot.size.height as usize);
                let (tw, th) = (80usize, 24usize);
                const RAMP: &[u8; 10] = b" .:-=+*#%@";
                let mut rows = Vec::with_capacity(th);
                for ty in 0..th {
                    let mut line = String::with_capacity(tw);
                    for tx in 0..tw {
                        let (sx, sy) = (tx * w / tw, ty * h / th);
                        let i = (sy * w + sx) * 4;
                        let lum = if i + 2 < bytes.len() {
                            (bytes[i] as u32 + bytes[i + 1] as u32 + bytes[i + 2] as u32) / 3
                        } else {
                            0
                        };
                        line.push(RAMP[(lum * 9 / 255) as usize] as char);
                    }
                    rows.push(line);
                }
                emit(json!({
                    "event": "frame",
                    "base": "iced",
                    "cols": tw,
                    "rows": rows,
                }));
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        // The ForgeView's Elm tree gets wrapped so host messages route back.
        let el: Element<ForgeMessage> = self.view.view();
        el.map(Message::Forge)
    }
}

fn main() -> iced::Result {
    let (tx, rx) = mpsc::channel::<HostMsg>();
    spawn_reader(tx);
    iced::application("Forge Preview · iced", Host::update, Host::view)
        .theme(|host: &Host| host.theme.clone())
        .run_with(|| {
            (
                Host::new(rx),
                Task::perform(
                    async { std::thread::sleep(Duration::from_millis(50)) },
                    |_| Message::Tick,
                ),
            )
        })
}
