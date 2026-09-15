# Locus

A fast, open-source tool for managing iOS/macOS localization files.

Supports both classic `.strings` files (inside `*.lproj` directories) and
modern `.xcstrings` String Catalogs (JSON-based, single-file).

Built around a Rust core with a CLI for dev workflows and a future macOS app
for a more visual experience.

## Features

- **Dual format support** — `.strings` (classic) and `.xcstrings` (String Catalog)
- **Search** by key substring or value substring across all locales
- **Missing detection** — find keys absent for a given locale, with CSV/JSON export
- **Interactive editing** — fill all locales in one pass, arrow-right to copy source
- **Atomic saves** — temp file + rename, with optional `.bak` backup
- **Fast** — loads a 155k-line, 39-language file in under 30ms
- **Preserves metadata** — translation states (`new`, `translated`, `stale`) and `extractionState`

## Status

**V1 (core + CLI)** — stable and functional.

| Component | Status |
|-----------|--------|
| Core engine | ✅ Stable |
| CLI | ✅ Stable |
| macOS app (SwiftUI) | 🚧 Planned (V2) |
| CSV/JSON import-export | 🚧 Planned (V3) |

## Install

### Homebrew (recommended)

```bash
brew install luca/tap/locus
```

### Cargo

```bash
cargo install locus-cli
```

### From source

```bash
git clone https://github.com/luca/locus.git
cd locus
cargo build --release
# Binary is at target/release/locus
```

## Usage

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

## Supported formats

| Format | Extension | Structure |
|--------|-----------|-----------|
| Classic strings | `.strings` | One file per locale inside `xx.lproj/` directories |
| String Catalog | `.xcstrings` | Single JSON file containing all locales |

Both formats can coexist in the same project. The scanner detects them
automatically.

## Architecture

```
locus/
├── crates/
│   ├── core/    — locus-core: the engine (parsing, search, editing, saving)
│   └── cli/     — locus-cli: thin CLI wrapper (clap)
└── CDC.md      — specification (French)
```

The `core` crate is pure logic with no I/O dependencies beyond std (plus
serde for JSON). It's designed to be reused by both the CLI and a future
macOS app via FFI.

### Core modules

- **model** — in-memory representation (`Project`, `LocalizationFile`, `StringsFile`, `Key`)
- **parser** — `.strings` file parser (comments, escapes, duplicates)
- **xcstrings** — `.xcstrings` parser and serializer (JSON-based)
- **project** — recursive scanner detecting `*.lproj` and `.xcstrings` files
- **search** — search by key substring or by value substring
- **missing** — detect keys absent for a given locale
- **edit** — in-memory mutations (add key, set value)
- **save** — atomic disk persistence with optional backups

## Roadmap

- [x] V1 — Core engine + CLI
- [ ] V2 — macOS app (SwiftUI via FFI)
- [ ] V3 — CSV/JSON import-export, advanced filtering, Xcode build phase integration

## Contributing

Contributions are welcome. The project is MIT-licensed and built in the open.

1. Fork the repository
2. Create a feature branch (`git checkout -b feat/my-feature`)
3. Run tests (`cargo test`)
4. Submit a pull request

## License

[MIT](LICENSE)
