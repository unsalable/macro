# FlowMacro

Windows keyboard and mouse automation with precise timing — a Tauri 2 desktop app
with a Rust input engine and a React front end.

## Download

Grab the latest build from the [Releases page](https://github.com/unsalable/macro/releases/latest):

- **`FlowMacro-<version>-portable.exe`** — no installer, just double-click and run.
- `FlowMacro_<version>_x64-setup.exe` — installer with a Start menu shortcut.
- `FlowMacro_<version>_x64_en-US.msi` — MSI for managed deployments.

The builds are unsigned, so SmartScreen may warn on first launch: *More info -> Run anyway*.

FlowMacro checks this Releases feed on startup and can install a new version by itself
(Settings -> Updates, where the check can also be turned off). Update packages carry a
minisign signature and are rejected unless they were signed with this project's key.

## Requirements

- Windows 10/11
- Node.js 20+
- Rust 1.77+ with the `x86_64-pc-windows-msvc` toolchain
- MSVC C++ build tools + Windows SDK (Visual Studio "Desktop development with C++")
- WebView2 runtime (preinstalled on Windows 11)

## Development

```bash
npm install
npm run tauri dev
```

Run the checks the project gates on:

```bash
npm run typecheck
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

The suite has 86 unit tests. One more is ignored by default because it moves the
real cursor to prove `SendInput` reaches the system:

```bash
cargo test --manifest-path src-tauri/Cargo.toml -- --ignored
```

> Run `cargo` from PowerShell, not Git Bash: Git Bash ships a GNU `link` that
> shadows the MSVC linker and the build fails with "extra operand".

## Packaging

```bash
npm run tauri build
```

Produces an NSIS `.exe` and an `.msi` under `src-tauri/target/release/bundle/`.

## How it works

| Layer | Where | Note |
|---|---|---|
| Input injection | `src-tauri/src/input/inject.rs` | `SendInput` with **scan codes**, so DirectInput/RawInput games see the events |
| Global capture | `src-tauri/src/input/hook.rs` | `WH_KEYBOARD_LL` / `WH_MOUSE_LL` on a dedicated thread; event driven, ~0% CPU at idle |
| Timing | `src-tauri/src/input/timing.rs` | `timeBeginPeriod(1)` while running, hybrid sleep + spin, absolute deadlines so long runs do not drift |
| Execution | `src-tauri/src/macros/` | Worker thread, atomic state machine, condvar pause, sub-millisecond stop |
| Storage | `%APPDATA%\FlowMacro\` | `settings.json`, `profiles.json`, `history.jsonl`; every write is temp-file + rename |

### Safety net

`F6` start · `F7` stop · `F8` pause · `F12` emergency stop, all reassignable.
Independently of those, **three taps of `Esc` within 500 ms stops the engine
unconditionally** — so a mis-assigned hotkey can never leave the machine
clicking. It can be turned off in Settings → Hotkeys.

## Known limitations

- **Antivirus warnings.** A program that installs a global hook and synthesises
  input looks like what it is. Unsigned builds may be flagged; code signing is
  the fix and is not set up here.
- **Elevated windows.** Windows UIPI blocks input from a lower integrity process,
  so macros do nothing over an app running as administrator. FlowMacro reports
  this rather than failing silently — run it as administrator to match.
- **Anti-cheat.** Some games filter injected input regardless of scan codes.
  Nothing in the app tries to evade that.
- **Timing above ~200 CPS.** The Windows scheduler stops being reliable; the UI
  shows the *measured* CPS, not the requested one, so the gap is visible.

## Layout

```
src/                React front end (pages, stores, design tokens, i18n)
src-tauri/src/
  input/            SendInput, hooks, key tables, timing
  macros/           model, validation, scheduler, executor, engine
  hotkeys/          matcher (pure, tested) and the hook registry
  recorder/         live capture and recording → macro conversion
  config/           settings and profiles
  storage/          atomic JSON and the history log
  commands/         Tauri command surface
```
