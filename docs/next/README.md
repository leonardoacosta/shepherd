# shepherd


<p align="center">
  <img src="assets/logo.png" alt="shepherd" width="100" />
</p>

<p align="center">
  <a href="https://shepherd.dev">shepherd.dev</a> · <a href="#install">install</a> · <a href="https://shepherd.dev/docs/quick-start/">quick start</a> · <a href="https://shepherd.dev/docs/">docs</a> · <a href="#sponsors">sponsors</a>
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-666666?labelColor=333333" alt="Apache 2.0 license" /></a>
  <a href="https://github.com/leonardoacosta/shepherd/releases"><img src="https://img.shields.io/github/downloads/leonardoacosta/shepherd/total?labelColor=333333&color=666666" alt="total GitHub release downloads" /></a>
  <a href="https://github.com/leonardoacosta/shepherd/stargazers"><img src="https://img.shields.io/github/stars/leonardoacosta/shepherd?labelColor=333333&color=666666&logo=github" alt="GitHub stars" /></a>
  <a href="https://github.com/leonardoacosta/shepherd/releases/latest"><img src="https://img.shields.io/github/v/release/leonardoacosta/shepherd?label=release&labelColor=333333&color=666666" alt="latest stable release" /></a>
  <a href="https://formulae.brew.sh/formula/shepherd"><img src="https://img.shields.io/homebrew/v/shepherd?label=homebrew&labelColor=333333&color=666666" alt="Homebrew version" /></a>
  <a href="https://x.com/shepherddev"><img src="https://img.shields.io/badge/follow-%40shepherddev-000000?logo=x&logoColor=white" alt="follow @shepherddev on X" /></a>
</p>

---

https://github.com/user-attachments/assets/043ec09f-4bdd-41d5-aee0-8fda6b83e267

**agent multiplexer that lives in your terminal.**

- **every agent at a glance** — blocked, working, done. real terminal views, not a wrapped interpretation.
- **detach, agents keep running** — reattach from any terminal, or over ssh. sessions survive restarts.
- **agents can use shepherd too** — a pure socket api: agents spawn panes, read output, wait on each other. [agent skill →](https://shepherd.dev/docs/agent-skill/)
- **keyboard and mouse, both first-class** — tmux-style prefix keys *and* click, drag, split. pick per moment, not per tool.
- **plugins** — extend panes and workflows. [browse the marketplace →](https://shepherd.dev/plugins/)
- **one rust binary, no electron** — runs in whatever terminal you already use.

---

## install

```bash
curl -fsSL https://shepherd.dev/install.sh | sh
```

or `brew install shepherd` · `mise use -g shepherd` · windows beta: `powershell -ExecutionPolicy Bypass -c "irm https://shepherd.dev/install.ps1 | iex"` · [binaries](https://github.com/leonardoacosta/shepherd/releases)

then start it where the work lives:

```bash
shepherd
```

run your agents, split panes, walk away. `ctrl+b q` detaches, `shepherd` reattaches. [quick start →](https://shepherd.dev/docs/quick-start/)

## docs

everything lives at [shepherd.dev/docs](https://shepherd.dev/docs/): [quick start](https://shepherd.dev/docs/quick-start/) · [concepts](https://shepherd.dev/docs/concepts/) · [supported agents](https://shepherd.dev/docs/agents/) · [keyboard](https://shepherd.dev/docs/keyboard/) · [configuration](https://shepherd.dev/docs/configuration/) · [session state](https://shepherd.dev/docs/session-state/) · [remote](https://shepherd.dev/docs/persistence-remote/) · [integrations](https://shepherd.dev/docs/integrations/) · [plugins](https://shepherd.dev/docs/plugins/) · [socket api](https://shepherd.dev/docs/socket-api/)

## sponsors

shepherd is built full-time, in the open. sponsoring directly funds development, stability, and the path to a real agent runtime.

### gold

<a href="https://terminaltrove.com/"><img src="assets/sponsors/terminal-trove.png" alt="Terminal Trove" width="200" /></a>

[**→ become a sponsor**](https://github.com/sponsors/ogulcancelik) · enterprise / partnership: hey@shepherd.dev · see [SPONSORS.md](./SPONSORS.md) for tiers. thank you 🐑

## agent instructions

if you are an ai agent helping with this repository, read [`AGENTS.md`](./AGENTS.md) before making changes and read [`CONTRIBUTING.md`](./CONTRIBUTING.md) before opening issues or PRs.

## development

```bash
git clone https://github.com/leonardoacosta/shepherd
cd shepherd
cargo build --release

just test        # unit tests
just check       # formatting, tests, and maintenance checks
```

## license

Shepherd is licensed under the [Apache License 2.0](LICENSE).
