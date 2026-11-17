# Do we need `forge.toml`?

Now that a document's `meta:` block carries app identity (title, app id, icon —
doc wins, manifest is the fallback), it's fair to ask what the project manifest
is still *for*. This is the honest inventory and the trade-off.

## What `forge.toml` actually does today

| Field | Consumer | Covered by the doc? |
|---|---|---|
| `package.id` | `WindowOptions.app_id` → WM class / `CFBundleIdentifier` | yes — `meta.id` wins |
| `package.title` | window titlebar, `.desktop` `Name=`, NSIS | yes — `meta.title` wins |
| `package.icon` | `.desktop` `Icon=` in `forge package` | yes — `meta.icon` wins |
| `package.name` | binary / crate / AppDir name | partially — still manifest-or-file-stem |
| `package.version` | macOS `CFBundleShortVersionString` only | `meta.version` exists but isn't wired here yet |
| `package.binary` | `forge package` wraps an *existing workspace binary* (this is how Studio itself ships) | no — can't be; it's about the build target, not a UI doc |
| `forge.document` | entry doc when pointing `forge` at a dir; skeleton entry | no — only meaningful for multi-doc projects |
| `forge.documents` | explicit doc set for `forge skeleton` | no — the doc set is a project property |
| `forge.base` | expected base — `skeleton` skips docs targeting other bases | no — it's a project-wide *policy*, docs only declare their own `base:` |
| `libraries` | extra dependency pins → generated `Cargo.toml` | mostly — `use:` imports already carry versions (`ely: ely-core ^0.2`) |
| `hooks.krate` | **declared, never consumed** | dead field today |
| `assets.dir` | `forge package` copies it next to the binary; Studio's Assets tab browses it | no — a directory tree can't live in a doc |
| `themes.dir` | project theme JSONs merge into the bundled catalog (project wins name collisions); Studio loads them into the live registry | no — theme files, not markup |

## Case for dropping it

**Single-doc is the norm.** A Forge app is one document, one window. Everything
a user actually cares about — title, app id, icon, base, libraries — already
lives in the doc via `meta:` and `use:`. For that app shape, `forge.toml` is a
ceremony file: `forge new` scaffolds it, nobody edits it, and it silently
contributes nothing.

**cwd-based discovery is a footgun.** `load_manifest` walks up from your
*current working directory*, not the doc's directory. The same doc builds with
a different identity depending on where you ran `forge` — it picks up whatever
`forge.toml` happens to sit above cwd. Doc `meta:` is hermetic: the doc *is*
the app, and builds identically from anywhere.

**Two sources of truth.** With meta-in-doc, title/id/icon can be set in two
places with a precedence rule users have to learn ("doc wins, manifest
fallback"). Every duplicated key is a support question waiting to happen.

**Redundancy is already real.** `forge.base` duplicates `base:`;
`libraries` duplicates `use:` versions; `document` duplicates just passing the
path. Several fields are dead weight.

## Case for keeping it

**Build-target concerns aren't document concerns.** `package.binary`,
`hooks.krate`, `assets.dir`, `themes.dir`, and the entry point of a multi-doc
app describe
*the crate*, not a UI document. A doc shouldn't get to declare "I am the entry
point" — that's a property of the project, decided by whoever wires the docs
together.

**Multi-doc apps need a home for project policy.** `forge skeleton` builds an
explicit `documents` list and needs to know which one is the entry and which
base is expected. That could move to CLI flags, but a manifest is the
conventional place for it (Cargo.toml, package.json).

**Packaging metadata.** `forge package` builds AppImages, `.app` bundles, NSIS
installers — `version`, icon, and binary naming are real inputs there, and a
manifest is the natural place for OS-packaging data users wouldn't think to put
in a UI doc.

**Familiarity.** `forge new` writing `forge.toml + app.forge.yaml` matches the
Cargo.toml/package.json mental model — "here's my project file."

## The honest middle

The question isn't really *manifest vs no manifest* — it's **optional vs
mandatory**, and that's already decided: the manifest is *always* optional.
Every field has a doc-meta or file-stem fallback; `ForgeToml::default()` kicks
in whenever no `forge.toml` is found.

So the practical breakdown is:

- **Doc concerns → the doc.** Identity (`meta`), base, libraries (`use:`).
  Hermetic, versioned with the UI, no precedence puzzle. Done.
- **App-crate concerns → the manifest, when it exists.** `binary`, `document`,
  `forge.base` (policy), `hooks`, `assets`, OS-packaging extras. These only
  matter when an app outgrows one window — which is exactly when a project file
  earns its place.

**Recommendation: keep the file, shrink its role.** Stop scaffolding it in
`forge new` — emit a doc with a `meta:` block instead, and let a project grow a
`forge.toml` only when it needs a manifest-level knob (multi-doc skeleton,
`forge package` on a binary, hook crates). The manifest becomes what it should
have been all along: an opt-in for *project* configuration, never required for
a document to run.

Remaining work that would make the split clean:

- Wire `meta.version` into `bundle`'s plist/`CFBundleShortVersionString` so
  doc-meta fully covers packaging identity.
- Either consume `hooks.krate` or delete it — `assets.dir` is now wired.
- Consider honoring `meta.name` for the generated binary name — or document
  that binary naming stays manifest/file-stem only.
- Fix `load_manifest` to also (or only) search from the *doc's* directory, so
  a stray `forge.toml` above cwd can't silently rebrand a build.
