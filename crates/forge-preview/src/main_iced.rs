//! `forge-preview-iced` — the out-of-process preview host for the iced base
//! (#113). Same JSON wire protocol; the host is an `iced::application` whose
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
}

#[derive(Debug, Clone)]
enum Message {
    Forge(ForgeMessage),
    Tick,
    Quit,
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
        Self { view, rx }
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
                        HostMsg::Quit => return Task::done(Message::Quit),
                    }
                }
                return Task::perform(
                    async { std::thread::sleep(Duration::from_millis(50)) },
                    |_| Message::Tick,
                );
            }
            Message::Quit => return iced::exit(),
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
