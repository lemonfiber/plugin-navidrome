# Navidrome for lemonfiber

**Plays your music library in a browser, on a phone, and in any Subsonic client.**
Navidrome is one of the parts the stack lemonfiber ships is built from.
Without it: Music arrives and only the thing that fetched it ever looks at it.

## How lemonfiber carries it

The default bundle in
[lemonfiber-plugins](https://github.com/lemonfiber/lemonfiber-plugins) pins this
repository by a revision and the digests of its manifest and its tree, and
lemonfiber compiles that tree in. lemonfiber installs Navidrome with the adapter
this repository holds, which speaks lemonfiber's `owner.claim` and `media.serve`
contracts in front of Navidrome: a Rust crate built on the core's adapter kit,
into the image `ghcr.io/lemonfiber/navidrome-adapter`, published from every
commit on `main` and from each tag the release train cuts. It makes Navidrome's
first administrator the one lemonfiber mints before anybody else can, reads what
albums it holds and what is playing, and starts a scan of its library. lemonfiber
reaches Navidrome through this adapter alone. lemonfiber mints the
administrator's name and password and writes them into the adapter's
`upstream.json`, and the adapter's claim makes Navidrome's first run make that
administrator.

## What is here

| Path | What it is |
| --- | --- |
| `plugin.toml` | The part's manifest |
| `config/navidrome/` | The files Navidrome starts with, written into its configuration directory |
| `recordings/navidrome/` | The responses its proofs are held to |
| `src/` | The adapter |
| `conformance/owner.claim@1/` | The adapter's answer to each conformance case, in front of the pinned Navidrome |
| `conformance/media.serve@1/` | The adapter's answer to each conformance case, in front of the pinned Navidrome |
| `Dockerfile` | The adapter's image |
| `targets.toml` | The lemonfiber its checks run against, and the core the adapter is built on |
| `proofs.json` | What those checks said, committed |

## Checks

The plugin verdict is one job of spec's
[`first-party-plugin`](https://github.com/lemonfiber/spec/blob/main/.github/workflows/first-party-plugin.yml)
workflow: the manifest as the lemonfiber `targets.toml` names reads it, every
proof against its recording, and every pinned digest. The adapter is held to its
format, its lints, its tests, full line coverage and its dependency policy, its
image runs as lemonfiber runs it, and `recordings` takes every recording again
against the pinned Navidrome. `just ci` runs the gates that need nothing but
this tree.

## Licence

The code and data in this repository are under the licence in `LICENSE`.
Navidrome itself is under GPL-3.0-only and is not distributed here: this
repository names an image, it does not contain one.
