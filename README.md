# Upstream

[![build][bb]][bu]
[![dist][db]][dd]

Upstream is a cross-platform desktop client for the radicle code collaboration
protocol.

At the moment we support Linux and macOS. Latest packages for these platforms
are available on the [Radicle website][rw].

Windows support is considered experimental at this stage -- we don't provide
packages for this platform, so you'll have to build it from source.

The [UI][ui] is written in JavaScript using [Svelte][sv] and Electron and the
node [proxy][pr] logic is implemented in [Rust][ru].

A good entry point for exploration is [`development.md`][de], where you'll find
information on how to build Upstream from source.

If you're looking to contribute, take a look at [`contributing.md`][co] to
learn about the different ways that we accept contributions.

If you have questions or would like to get in touch, check out
[radicle.community][rc].

## Building and running Upstream

Prerequisites

* [NodeJS](https://nodejs.org/en/)
* [Yarn](https://yarnpkg.com/getting-started/install)
* [Rustup](https://github.com/rust-lang/rustup)

To build upstream run

```bash
yarn run dist
```

This command will create an application package in the `./dist` folder which
you can then run.

### Attribution

Upstream uses:
  - [Twemoji by Twitter][tw]
  - [The Inter typeface family by Rasmus Andersson][ra]
  - [Source Code Pro font family by Adobe][so]


[bb]: https://github.com/radicle-dev/radicle-upstream/actions/workflows/build.yaml/badge.svg
[bu]: https://github.com/radicle-dev/radicle-upstream/actions/workflows/build.yaml
[co]: docs/contributing.md
[db]: https://github.com/radicle-dev/radicle-upstream/actions/workflows/dist.yaml/badge.svg
[dd]: https://github.com/radicle-dev/radicle-upstream/actions/workflows/dist.yaml
[de]: docs/development.md
[pr]: proxy
[ra]: https://rsms.me/inter
[rc]: https://radicle.community
[ru]: https://www.rust-lang.org
[rw]: https://radicle.xyz/downloads.html
[so]: https://adobe-fonts.github.io/source-code-pro
[sv]: https://svelte.dev
[tw]: https://twemoji.twitter.com
[ui]: ui
