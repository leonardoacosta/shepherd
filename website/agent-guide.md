# Shepherd agent guide

You are reading this because a human asked you to help them understand, set up, or troubleshoot Shepherd. This file gives you the concept model, the setup path, and the diagnosis recipes so you can guide them accurately. Canonical documentation lives at https://shepherd.dev/docs/ — link the human there for depth, and verify any command you are unsure about against those pages instead of guessing.

If you are running *inside* a Shepherd pane (the environment variable `SHEPHERD_ENV=1` is set), Shepherd also ships a skill file that teaches you to control Shepherd yourself through the `shepherd` CLI: https://raw.githubusercontent.com/leonardoacosta/shepherd/master/SKILL.md. That file is about you operating Shepherd; this file is about you teaching a human.

## What Shepherd is

Shepherd is a terminal workspace manager for AI coding agents. Like tmux, it is a multiplexer: a background server owns real terminal processes, and clients attach to render them. Panes keep running when the human detaches, closes the terminal, or disconnects SSH.

Unlike tmux, Shepherd is mouse-first and agent-aware. The whole UI is clickable — panes, tabs, workspaces, split borders, right-click menus. Shepherd detects coding agents running inside panes and shows each one's state in a sidebar, so the human can see across all their projects which agent is `working`, which is `blocked` waiting for input, and which is `done`. A CLI and a local socket API let scripts and agents drive Shepherd programmatically.

## Concept model

Teach these in this order:

- **Session** — a persistent background server namespace. Running `shepherd` attaches to the default session. Named sessions (`shepherd session attach work`) are fully separate runtime namespaces; most people only need the default.
- **Workspace** — the project-level container. One per repo, task, or investigation. Owns tabs and panes. The sidebar rolls agent states up per workspace.
- **Tab** — a layout inside a workspace, for separating views like `agents`, `logs`, `server`.
- **Pane** — a real terminal. Splittable right or down. Survives client detach.
- **Agent** — a process Shepherd recognizes inside a pane. States: `working`, `blocked`, `done`, `idle`, `unknown`.
- **Modes** — terminal mode sends keys to the focused pane; prefix mode (`ctrl+b`, then one action key) sends one command to Shepherd; navigate mode is a persistent navigation surface.

Full concepts page: https://shepherd.dev/docs/concepts/

## Install

Linux and macOS:

```bash
curl -fsSL https://shepherd.dev/install.sh | sh
shepherd
```

Windows preview beta:

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://shepherd.dev/install.ps1 | iex"
shepherd
```

Homebrew, mise, and Nix installs, verification, and manual downloads: https://shepherd.dev/docs/install/. Updating later is `shepherd update`. Check the version with `shepherd --version`.

## First-run walkthrough

First check where you are. If `SHEPHERD_ENV=1` is set in your environment, you are already running inside a Shepherd pane — the human is already attached, so skip step 1 entirely and never tell them to run `shepherd` from your pane. Shepherd blocks nested launches by design. Start from step 2, and consider the skill file below.

Walk the human through this sequence:

1. `cd` into a project and run `shepherd`. It launches or attaches to the default background session and creates a workspace automatically. First run shows an onboarding flow.
2. Start their coding agent in the pane — `claude`, `codex`, or any supported agent (full list: https://shepherd.dev/docs/agents/). Shepherd detects it automatically; the sidebar shows its state. Installing the matching integration improves detection: `shepherd integration install claude` (and similarly for other agents).
3. Show them the mouse first: click panes and tabs to focus, drag split borders, right-click for menus, drag-select to copy. No keybindings are required to use Shepherd.
4. Split panes: right-click menu, or `prefix+v` (right) / `prefix+minus` (down). New tab: `prefix+c`.
5. Detach with `prefix+q` (press `ctrl+b`, release, press `q`) or simply close the terminal window. Everything keeps running. Reattach later with `shepherd`.
6. To actually stop everything: `shepherd server stop`.

## The keyboard story

Important framing for new users: Shepherd does not require learning keybindings. The mouse covers everything. When the human wants keyboard control:

- The prefix key is `ctrl+b` by default. `prefix+?` shows every active binding live.
- The guided keyboard page — what the prefix is, which bindings to learn first, and a vetted prefix-free setup using `ctrl+alt` chords — is https://shepherd.dev/docs/keyboard/. Recommend it over improvising.
- Every binding, including the prefix itself, is configurable under `[keys]` in the config file.
- If a direct chord does nothing, the OS or the outer terminal consumed it before Shepherd could see it. The keyboard page explains which chords are safe and why.

## Install the Shepherd skill into yourself

Shepherd ships `SKILL.md` (https://raw.githubusercontent.com/leonardoacosta/shepherd/master/SKILL.md), an instruction file that teaches a coding agent to control Shepherd from inside a pane — splitting panes, running commands without stealing focus, reading output, waiting on other agents.

Once the human is set up, offer to install it into your own harness so future sessions know Shepherd natively. For agents supported by the open skills CLI, use `npx skills add leonardoacosta/shepherd --skill shepherd -g`. Agents without a skill system can paste the GitHub copy above into global custom instructions. Ask the human before writing to their config locations, and use the GitHub copy above as the source of truth.

## Configuration

- Config file: `~/.config/shepherd/config.toml`. Shepherd works without one.
- Print the full default config: `shepherd --default-config`.
- Apply edits to a running server: `shepherd server reload-config` (or the global menu → reload config).
- Main areas: `[keys]` keybindings, `[theme]` themes, `[ui]` sidebar and UI behavior, `[terminal]` shell defaults, `[update]` channel.
- Full reference: https://shepherd.dev/docs/configuration/

## Diagnosis recipes

- **Agent not detected or wrong state:** `shepherd agent list` to see what Shepherd sees, `shepherd agent explain <target> --json` to see why the detector classified a pane the way it did. Installing the agent's integration (`shepherd integration install <name>`, status via `shepherd integration status`) gives Shepherd authoritative state instead of screen detection. Details: https://shepherd.dev/docs/agents/ and https://shepherd.dev/docs/integrations/
- **A keybinding does nothing:** the outer terminal or desktop environment owns that chord. Point the human to https://shepherd.dev/docs/keyboard/ to pick a safe one or free the chord in their terminal settings.
- **Something looks wrong at startup or with the socket API:** logs are at `~/.config/shepherd/shepherd.log`, `~/.config/shepherd/shepherd-client.log`, and `~/.config/shepherd/shepherd-server.log`. `shepherd status`, `shepherd status server`, and `shepherd status client` summarize the runtime.
- **Remote questions:** SSH to the machine and run `shepherd` there (works like tmux), or attach as a thin local client with `shepherd --remote <host>`. Trade-offs: https://shepherd.dev/docs/how-to-work/
- **What survives a detach, restart, or update:** https://shepherd.dev/docs/session-state/

## Rules for you

- Do not invent keybindings, config keys, or CLI flags. The ones in this file are accurate as of writing; for anything else, read the linked docs page first.
- Teach mouse before keyboard for humans new to multiplexers.
- Shepherd is not tmux: do not give tmux commands, tmux config syntax, or `.tmux.conf` advice for Shepherd questions.
- For automation, scripting, or controlling Shepherd from code, point to the CLI reference (https://shepherd.dev/docs/cli-reference/) and socket API (https://shepherd.dev/docs/socket-api/).
