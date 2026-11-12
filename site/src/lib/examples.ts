export type Example = {
	slug: string;
	title: string;
	summary: string;
	yaml: string;
	/** Sample view state the bindings read from. */
	state: Record<string, unknown>;
	/** Roughly what the same view looks like written by hand with gpui-kit. */
	rust: string;
};

// One demo per widget set — the same `examples/{demos,showcases}/*.forge.yaml` files the
// Welcome tab in Forge Studio opens (#129). gpui-kit docs render on the
// web canvas (libraries the wasm build can't run degrade to a placeholder
// element); `base: ratatui` docs draw a terminal frame via TuiCanvas, and
// `native` bases have no web renderer, so the source stands alone.
import elyDemo from '../../../examples/demos/ely-demo.forge.yaml?raw';
import gcShowcase from '../../../examples/showcases/gc-showcase.forge.yaml?raw';
import gflowDemo from '../../../examples/demos/gflow-demo.forge.yaml?raw';
import baseDemo from '../../../examples/demos/base-demo.forge.yaml?raw';
import videoDemo from '../../../examples/demos/video-demo.forge.yaml?raw';
import higDemo from '../../../examples/demos/hig-demo.forge.yaml?raw';
import pdfDemo from '../../../examples/demos/pdf-demo.forge.yaml?raw';
import whiteboardDemo from '../../../examples/demos/whiteboard-demo.forge.yaml?raw';
import mathDemo from '../../../examples/demos/math-demo.forge.yaml?raw';
import ratatuiShowcase from '../../../examples/showcases/ratatui-showcase.forge.yaml?raw';
import ratatuiDashboard from '../../../examples/demos/ratatui-dashboard.forge.yaml?raw';
import ratatuiForms from '../../../examples/demos/ratatui-forms.forge.yaml?raw';
import gpuiKitShowcase from '../../../examples/showcases/gpui-kit-showcase.forge.yaml?raw';
import icedShowcase from '../../../examples/showcases/iced-showcase.forge.yaml?raw';
import bezelShowcase from '../../../examples/showcases/bezel-showcase.forge.yaml?raw';
import gpui02Showcase from '../../../examples/showcases/gpui02-showcase.forge.yaml?raw';
import adabrakaShowcase from '../../../examples/showcases/adabraka-showcase.forge.yaml?raw';

export type Showcase = {
	slug: string;
	/** The `use:` alias the document imports, if any. */
	alias?: string;
	/** The canvas the doc previews on: defaults to gpui-kit; `native` means no
	 *  web renderer exists, so the source stands alone. */
	base?: 'ratatui' | 'iced' | 'bezel' | 'native';
	title: string;
	summary: string;
	yaml: string;
};

export const showcases: Showcase[] = [
	{
		slug: 'ely-demo',
		alias: 'ely',
		title: 'Ely GPUI Components',
		summary: '`use: ely: ely-core` — data-display widgets (KPI cards, meters, timelines).',
		yaml: elyDemo
	},
	{
		slug: 'gc-showcase',
		alias: 'gc',
		title: 'gpui-component',
		summary: '`use: gc: gpui-component` — the upstream component suite.',
		yaml: gcShowcase
	},
	{
		slug: 'gflow-demo',
		alias: 'gflow',
		title: 'gpui-flow',
		summary: '`use: gflow: gpui-flow` — node-graph canvas.',
		yaml: gflowDemo
	},
	{
		slug: 'base-demo',
		alias: 'base',
		title: 'base-gpui',
		summary: '`use: base: base-gpui` — gpui-pre primitives ported to gpui-kit.',
		yaml: baseDemo
	},
	{
		slug: 'video-demo',
		alias: 'video',
		title: 'gpui-video-player',
		summary: '`use: video: gpui-video-player` — GStreamer video element.',
		yaml: videoDemo
	},
	{
		slug: 'hig-demo',
		alias: 'hig',
		title: 'gpui-apple-hig',
		summary: '`use: hig: gpui-apple-hig` — macOS-styled widgets.',
		yaml: higDemo
	},
	{
		slug: 'pdf-demo',
		alias: 'pdf',
		title: 'gpui-pdf',
		summary: '`use: pdf: gpui-pdf` — virtualized PDF viewer.',
		yaml: pdfDemo
	},
	{
		slug: 'whiteboard-demo',
		alias: 'wb',
		title: 'gpui-whiteboard',
		summary: '`use: wb: gpui-whiteboard` — infinite canvas.',
		yaml: whiteboardDemo
	},
	{
		slug: 'math-demo',
		alias: 'math',
		title: 'ratex-gpui',
		summary: '`use: math: ratex-gpui` — LaTeX rendering + equation editor.',
		yaml: mathDemo
	},
	{
		slug: 'gpui-kit-showcase',
		title: 'gpui-kit',
		summary: '`base: gpui-kit` — the base widget set itself.',
		yaml: gpuiKitShowcase
	},
	{
		slug: 'ratatui-showcase',
		base: 'ratatui',
		title: 'ratatui',
		summary: '`base: ratatui` — terminal widgets drawn into a text-cell frame.',
		yaml: ratatuiShowcase
	},
	{
		slug: 'ratatui-dashboard',
		base: 'ratatui',
		title: 'ratatui dashboard',
		summary: '`base: ratatui` — tabs, gauges and a table in a tiled layout.',
		yaml: ratatuiDashboard
	},
	{
		slug: 'ratatui-forms',
		base: 'ratatui',
		title: 'ratatui forms',
		summary: '`base: ratatui` — inputs and lists in a two-pane form.',
		yaml: ratatuiForms
	},
	{
		slug: 'iced-showcase',
		base: 'native',
		title: 'Iced',
		summary: '`base: iced` — the same document on the Elm-style Iced runtime; its web canvas does not paint yet.',
		yaml: icedShowcase
	},
	{
		slug: 'bezel-showcase',
		base: 'native',
		title: 'Bezel',
		summary: '`base: bezel` — the gpui-web/bezel render path; its web canvas does not paint yet.',
		yaml: bezelShowcase
	},
	{
		slug: 'gpui02-showcase',
		base: 'native',
		title: 'GPUI 0.2 · guise',
		summary: '`base: gpui02` — Vanilla GPUI 0.2 + guise; native only.',
		yaml: gpui02Showcase
	},
	{
		slug: 'adabraka-showcase',
		base: 'native',
		title: 'adabraka-ui',
		summary: '`base: adabraka` — the adabraka GPUI port; native only.',
		yaml: adabrakaShowcase
	}
];

export const examples: Example[] = [
	{
		slug: 'login',
		title: 'Login form',
		summary: 'Two-way bindings, interpolated text and a hook on click.',
		state: { user: { name: 'Ada' }, form: { email: '', remember: true } },
		yaml: `forge: "0.1"
base: gpui-kit
meta:
  title: Login
root:
  VStack#root:
  style:
    gap: 12px
    padding: 24px
    max-width: 380px
  children:
    - Text: "Welcome back, {$user.name}"
      style:
        size: xl
        weight: semibold
    - Input#email:
      value: bind $form.email
      placeholder: Email
    - Checkbox: Remember me
      checked: bind $form.remember
    - HStack:
      style:
        gap: 8px
      children:
        - Button: Sign in
          variant: Primary
          icon: log-in
          on click: sign_in()
        - Button: Cancel
          variant: Ghost
`,
		rust: `use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::{checkbox::Checkbox, input::{Input, InputState}, v_flex, h_flex, Icon};
use gpui_kit::{prelude::*, *};

struct Login { email: Entity<InputState>, remember: bool, user: SharedString }

impl Render for Login {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap(px(12.)).p(px(24.)).max_w(px(380.))
            .child(div().text_xl().font_weight(FontWeight::SEMIBOLD)
                .child(format!("Welcome back, {}", self.user)))
            .child(Input::new(&self.email)) // InputState::new(..).placeholder("Email")
            .child(Checkbox::new("remember").label("Remember me").checked(self.remember)
                .on_click(cx.listener(|this, checked, _, _| this.remember = *checked)))
            .child(h_flex().gap(px(8.))
                .child(Button::new("sign-in").label("Sign in").primary()
                    .icon(Icon::empty().path("icons/log-in.svg"))
                    .on_click(cx.listener(|this, _, _, cx| this.sign_in(cx))))
                .child(Button::new("cancel").label("Cancel").ghost()))
    }
}`
	},
	{
		slug: 'settings',
		title: 'Settings panel',
		summary: 'Switches, a radio group and a select, all bound to state.',
		state: { prefs: { notifications: true, theme: 1, language: 'English' } },
		yaml: `forge: "0.1"
root:
  VStack:
  style:
    gap: 14px
    padding: 24px
    max-width: 420px
  children:
    - Text: Settings
      style:
        size: lg
        weight: semibold
    - Switch: Notifications
      checked: bind $prefs.notifications
    - Separator: Appearance
    - RadioGroup:
      selected: bind $prefs.theme
      layout: Horizontal
      children:
        - Radio: Light
        - Radio: Dark
        - Radio: System
    - Label: Language
    - Select:
      value: bind $prefs.language
      options: [English, Deutsch, Français, 日本語]
      placeholder: Pick a language
    - HStack:
      style:
        gap: 6px
      children:
        - Tag: Saved
          variant: Success
        - Text: "Language: {$prefs.language}"
`,
		rust: `use gpui_kit::component::radio::{Radio, RadioGroup};
use gpui_kit::component::select::{Select, SelectState};
use gpui_kit::component::{switch::Switch, separator::Separator, label::Label, tag::Tag, v_flex};
use gpui_kit::{prelude::*, *};

v_flex()
    .gap(px(14.)).p(px(24.)).max_w(px(420.))
    .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child("Settings"))
    .child(Switch::new("notifications").label("Notifications").checked(self.notifications))
    .child(Separator::horizontal().label("Appearance"))
    .child(RadioGroup::horizontal("theme")
        .selected_index(Some(self.theme))
        .children([Radio::new("light").label("Light"), Radio::new("dark").label("Dark"),
                   Radio::new("system").label("System")]))
    .child(Label::new("Language"))
    .child(Select::new(&self.language).placeholder("Pick a language"))
    .child(h_flex().gap(px(6.)).child(Tag::success().child("Saved")))`
	},
	{
		slug: 'tabs',
		title: 'Tabs and badges',
		summary: 'Containers with children, enum variants and theme tokens.',
		state: { inbox: { unread: 4 } },
		yaml: `forge: "0.1"
root:
  VStack:
  style:
    gap: 12px
    padding: 24px
  children:
    - Tabs:
      variant: Underline
      children:
        - Tab: Inbox
          children:
            - HStack:
              style:
                gap: 10px
                align: center
              children:
                - Badge:
                  count: $inbox.unread
                  children:
                    - Icon: inbox
                      size: Large
                - Text: "{$inbox.unread} unread messages"
                  style:
                    color: $theme.muted_foreground
        - Tab: Archive
          children:
            - Text: Nothing archived yet.
        - Tab: Settings
          children:
            - Button: Clear inbox
              variant: Danger
              outline: true
              on click: clear_inbox()
`,
		rust: `use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::component::{badge::Badge, button::{Button, ButtonVariants}, ActiveTheme, Icon, Sizable, Size, v_flex, h_flex};
use gpui_kit::{prelude::*, *};

v_flex().gap(px(12.)).p(px(24.))
    .child(TabBar::new("tabs").underline().selected_index(self.tab)
        .children([Tab::new().label("Inbox"), Tab::new().label("Archive"), Tab::new().label("Settings")])
        .on_click(cx.listener(|this, ix: &usize, _, cx| { this.tab = *ix; cx.notify() })))
    .child(match self.tab {
        0 => h_flex().gap(px(10.)).items_center()
            .child(Badge::new().count(self.unread)
                .child(Icon::empty().path("icons/inbox.svg").with_size(Size::Large)))
            .child(div().text_color(cx.theme().muted_foreground)
                .child(format!("{} unread messages", self.unread)))
            .into_any_element(),
        1 => div().child("Nothing archived yet.").into_any_element(),
        _ => Button::new("clear").label("Clear inbox").danger().outline()
            .on_click(cx.listener(|this, _, _, cx| this.clear_inbox(cx))).into_any_element(),
    })`
	}
];
