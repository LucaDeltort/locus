# Locus

A fast, open-source tool for managing iOS/macOS localization files.

Supports both classic `.strings` files (inside `*.lproj` directories) and
modern `.xcstrings` String Catalogs (JSON-based, single-file).

Built around a Rust core with a CLI for dev workflows and a native macOS app
(SwiftUI) for a more visual experience.

## Features

- **Dual format support** — `.strings` (classic) and `.xcstrings` (String Catalog)
- **Search** by key substring or value substring across all locales
- **Missing detection** — find keys absent for a given locale, with CSV/JSON export
- **Interactive editing** — fill all locales in one pass, arrow-right to copy source
- **Atomic saves** — temp file + rename, with optional `.bak` backup
- **Fast** — loads a 155k-line, 39-language file in under 30ms
- **Preserves metadata** — translation states (`new`, `translated`, `stale`) and `extractionState`
- **macOS app** — native SwiftUI app with 2-column layout, search, inline editing,
  undo, locale filtering, key sorting, persistence

## Status

| Component | Status |
|-----------|--------|
| Core engine | ✅ Stable |
| CLI | ✅ Stable |
| macOS app (SwiftUI) | ✅ Functional |
| CSV/JSON import-export | 🚧 Planned (V3) |

## Install

### Homebrew (recommended)

```bash
brew install LucaDeltort/tap/locus
```

### Cargo

```bash
cargo install locus-cli
```

### From source

```bash
git clone https://github.com/LucaDeltort/locus.git
cd locus
cargo build --release
# Binary is at target/release/locus
```

## Usage (CLI)

```text
locus <PATH> <COMMAND>
```

`PATH` is required — either a project directory (scanned recursively) or a
direct `.xcstrings` file.

### search

Search by substring in key names or in translation values across all locales.

```bash
# Search by key substring
locus ./MyProject.xcstrings search --key "login"

# Search by value substring (all languages)
locus ./MyProject.xcstrings search --text "Connexion"

# Filter output to specific locales only
locus ./MyProject.xcstrings search --key "speed" --langs en,fr

# Search a .strings-based project directory
locus ./MyXcodeProject search --text "Welcome"
```

Output format:

```text
login.button
  ✓ en  "Sign In"
  ✓ fr  "Connexion"
  ✗ de  (missing)

logout.button
  ✓ en  "Log Out"
  ✗ fr  (missing)

2 key(s) found in MyProject.xcstrings
```

Colors: keys in yellow, `✓` and values in green, `✗` and `(missing)` in red,
header in cyan. Colors are automatically disabled when piped.

### missing

List keys present in other locales but absent for the given locale.

```bash
# Plain text output
locus ./MyProject.xcstrings missing --lang fr

# JSON export
locus ./MyProject.xcstrings missing --lang fr --output json

# CSV export
locus ./MyProject.xcstrings missing --lang fr --output csv
```

Plain text output:

```text
Localizable
  ✗ logout.button   en  "Log Out"
  ✗ welcome.title    en  "Welcome"

2 missing key(s) for [fr] in MyProject.xcstrings
```

JSON output:

```json
[
  {"file": "Localizable", "key": "logout.button", "refLang": "en", "refValue": "Log Out", "presentIn": ["en"]}
]
```

### set

Set a translation value for a specific key and locale. Creates the locale
entry if it doesn't exist. Saves atomically (temp file + rename).

```bash
# Set a single locale (--file auto-detected)
locus ./MyProject.xcstrings set --key "logout.button" --lang fr --value "Déconnexion"

# With backup
locus ./MyProject.xcstrings set --key "BEST" --lang fr --value "MEILLEUR" --backup
```

Output:

```text
✓ Set Localizable :: logout.button
  ✓ fr  "Déconnexion"
```

#### Interactive mode

Omit `--lang` and `--value` to fill all locales interactively. The source
language is shown first (skipped if already has a value), then each other
locale prompts for input.

```bash
locus ./MyProject.xcstrings set --key "new.key"
```

```text
new.key — fill all locales
Press Enter to skip, → to copy source.

  ✓ en  "Hello World"
  ar: >
  bg: 
    (skipped)
  ...

✓ Saved — 2 locale(s) updated in MyProject.xcstrings
```

Press **→** (arrow-right) or `>` to copy the source value into the input,
then **Enter** to confirm. Or type your own translation.

You can also set a single locale interactively by specifying `--lang`
without `--value`:

```bash
locus ./MyProject.xcstrings set --key "new.key" --lang pl
```

### add-key

Add a new key with a base-locale value. Other locales are left empty
(detectable via `missing`). Refuses to overwrite an existing key.

```bash
# Add a new key with an English base value
locus ./MyProject.xcstrings add-key --key "new.feature.title" --lang en --value "New Feature"

# With backup, and --file omitted (auto-detected)
locus ./MyProject.xcstrings add-key --key "new.feature.title" --lang en --value "New Feature" --backup
```

Output:

```text
✓ Added Localizable :: new.feature.title
  ✓ en  "New Feature"
```

## macOS App (SwiftUI)

The project includes a native macOS app built with SwiftUI, using the Rust
core via UniFFI bindings.

### Structure

```
app/
├── Sources/
│   ├── App/              # LocusApp.swift, Info.plist
│   ├── Views/            # ContentView, KeyListView, KeyDetailView, AddKeySheet, LocaleFilterPopover
│   ├── Models/           # ProjectViewModel, ProjectPersistence
│   ├── Theme/            # Theme.swift
│   └── Generated/        # locus.swift — UniFFI-generated bindings (auto)
├── Lib/                  # liblocus_core.a, locusFFI.h, locusFFI.modulemap (gitignored)
├── AppIcons/             # Assets.xcassets (app icon + accent colors)
├── Locus.xcodeproj/      # Xcode project (generated by XcodeGen)
└── project.yml           # XcodeGen configuration
```

### Building the app

**Prerequisites:** Xcode 15+, Rust toolchain, [XcodeGen](https://github.com/yonaskolb/XcodeGen).

The Xcode project includes a Run Script Phase that automatically compiles
the Rust core and regenerates Swift bindings before each build. Just open
`app/Locus.xcodeproj` in Xcode and press ⌘R.

To regenerate the project after structural changes:

```bash
cd app && xcodegen generate
```

### Opening the app on other Macs

The app is **not notarized** (no Apple Developer license), so macOS will
show a security warning the first time you open it:

1. Right-click `Locus.app` → **Open**
2. In the dialog, click **Open** again

After this one-time confirmation, macOS remembers the app and opens it
normally on subsequent launches.

### Features

- **2-column layout** — key list on the left, translation editor on the right
- **Search** — filter keys by name or by value content (segmented control)
- **Sort** — click column headers to sort by key name or completion status
- **Inline editing** — type directly in each locale's TextField; changes are
  pushed to the Rust model immediately
- **Copy from source** — one-click button to copy the source language value
  into any missing locale
- **Add / Delete keys** — ⌘N to add a key with a sheet dialog; ⌘⌫ to delete
  the selected key (with confirmation alert)
- **Undo** — ⌘Z to undo the last edit (grouped per field); undo add-key
  deletes the key
- **Save** — ⌘S to flush all changes to disk atomically (temp file + rename)
- **Locale filter** — popover to show/hide specific locales in the editor
- **Persistence** — last-opened project is restored on app launch via UserDefaults
- **CLI launcher** — `locus /path/to/project` opens the app with a project loaded

### Architecture

```
┌─────────────────────────────────────────────┐
│            SwiftUI App (macOS)               │
│         NavigationSplitView (2 cols)         │
│                                             │
│         │  @Observable ProjectViewModel       │
│         ▼                                    │
│    locus.swift (UniFFI bindings)             │
│         │  FFI calls                         │
│         ▼                                    │
│    liblocus_core.a                           │
│    ├─ scan_project()                         │
│    ├─ search_by_key/text()                   │
│    ├─ find_missing()                          │
│    ├─ set_value() / add_key() / delete_key() │
│    └─ save_project()                        │
└─────────────────────────────────────────────┘
```

The ViewModel caches key data to avoid repeated FFI round-trips on every
view re-render. Editing writes directly to the Rust model; the Save button
(⌘S) flushes changes to disk atomically. Undo is grouped per field so
consecutive keystrokes in the same TextField produce a single undo step.

Accent colors are defined in the asset catalog (`AppIcons/Assets.xcassets`):
Violet `#7C5CFC`, PurpleDark `#4E3AA8`, Slate `#8B8B95`.

### CLI Launcher

A shell script (`locus`) at the project root launches the macOS app from
the command line. It finds the built `.app` (either in `app/Locus.app` or
in Xcode's DerivedData) and opens it with an optional path argument.

**Install** (from the project root):

```bash
chmod +x locus
ln -s "$(pwd)/locus" /usr/local/bin/locus
```

**Usage:**

```bash
# Open a specific project folder or .xcstrings file
locus /path/to/project

# Open with the last-used project (via persistence)
locus
```

The app declares `CFBundleDocumentTypes` for folders and `.xcstrings`
files, so `open -a Locus /path` triggers `.onOpenURL` which loads the
project automatically.

Note: if the app is already running, macOS may not pass the path argument.
Quit the app first (`⌘Q`) or use the in-app "Open…" button.

## Supported formats

| Format | Extension | Structure |
|--------|-----------|-----------|
| Classic strings | `.strings` | One file per locale inside `xx.lproj/` directories |
| String Catalog | `.xcstrings` | Single JSON file containing all locales |

Both formats can coexist in the same project. The scanner detects them
automatically.

## Architecture (Rust core)

```
crates/
├── core/    — locus-core: the engine (parsing, search, editing, saving, FFI)
├── cli/     — locus-cli: thin CLI wrapper (clap)
└── bindgen/ — locus-bindgen: Swift binding generator (UniFFI)
```

The `core` crate exposes a UniFFI interface (`ffi.rs`) consumed by the macOS
app. It compiles as `rlib`, `staticlib`, and `cdylib`.

### Core modules

- **model** — in-memory representation (`Project`, `LocalizationFile`, `StringsFile`, `Key`)
- **parser** — `.strings` file parser (comments, escapes, duplicates)
- **xcstrings** — `.xcstrings` parser and serializer (JSON-based)
- **project** — recursive scanner detecting `*.lproj` and `.xcstrings` files
- **search** — search by key substring or by value substring
- **missing** — detect keys absent for a given locale
- **edit** — in-memory mutations (add key, set value, delete key)
- **save** — atomic disk persistence with optional backups
- **ffi** — UniFFI interface for Swift consumption

## Roadmap

- [x] V1 — Core engine + CLI
- [x] V2 — macOS app (SwiftUI via UniFFI)
- [ ] V3 — CSV/JSON import-export, advanced filtering, Xcode build phase integration

## Contributing

Contributions are welcome. The project is MIT-licensed and built in the open.

1. Fork the repository
2. Create a feature branch (`git checkout -b feat/my-feature`)
3. Run tests (`cargo test`)
4. Submit a pull request

## License

[MIT](LICENSE)
