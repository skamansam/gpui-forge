//! `forge-preview-iced` — the out-of-process preview host for the iced base
//! (#113). Same JSON wire protocol; the host is an `iced::application` whose
//! update loop bridges the channel into the view's Elm messages.

use gpui_forge_runtime::check;
use gpui_forge_runtime::render::{ForgeView, Message as ForgeMessage};
use iced::{Element, Task};
const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
use serde_json::json;
use std::sync::mpsc;
use std::time::Duration;

mod protocol;
use protocol::{HostMsg, emit, spawn_reader};

struct Host {
    view: ForgeView,
    rx: mpsc::Receiver<HostMsg>,
    ticks: u64,
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
        Self { view, rx, ticks: 0 }
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
                                diags.iter().map(|d| d.message.clone()).collect::<Vec<_>>()}));
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
                        HostMsg::Theme { name, dark } => {
                            if let Some(name) = &name {
                                eprintln!(
                                    "forge-preview-iced: named theme `{name}` not mapped (dark={dark})"
                                );
                            }
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
                let bytes = shot.as_ref();
                let mut b64 = String::with_capacity(bytes.len() / 3 * 4);
                for c in bytes.chunks(3) {
                    let mut v = (c[0] as u32) << 16;
                    if c.len() > 1 {
                        v |= (c[1] as u32) << 8;
                    }
                    if c.len() > 2 {
                        v |= c[2] as u32;
                    }
                    b64.push(B64[(v >> 18) as usize & 63] as char);
                    b64.push(B64[(v >> 12) as usize & 63] as char);
                    b64.push(if c.len() > 1 {
                        B64[(v >> 6) as usize & 63] as char
                    } else {
                        '='
                    });
                    b64.push(if c.len() > 2 {
                        B64[v as usize & 63] as char
                    } else {
                        '='
                    });
                }
                emit(json!({
                    "event": "frame_rgba",
                    "base": "iced",
                    "w": shot.size.width,
                    "h": shot.size.height,
                    "data": b64,
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
    iced::application("Forge Preview · iced", Host::update, Host::view).run_with(|| {
        (
            Host::new(rx),
            Task::perform(
                async { std::thread::sleep(Duration::from_millis(50)) },
                |_| Message::Tick,
            ),
        )
    })
}
