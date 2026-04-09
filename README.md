# varlock-lsp

A Language Server Protocol (LSP) implementation for [Varlock](https://varlock.com) environment configuration files (`.env`, `.env.schema`, `.env.*`).

## Features

- **Autocomplete** for decorators, data types, type options, resolver functions, and config item references
- **Diagnostics** that validate decorator conflicts, duplicate usage, and static values against declared types
- **Hover documentation** for all built-in decorators

## Supported Decorators

### Root (Header) Decorators

| Decorator | Description |
|---|---|
| `@currentEnv` | Sets the env var reference used to select environment-specific files |
| `@defaultRequired` | Controls whether items default to required (`true`, `false`, or `infer`) |
| `@defaultSensitive` | Controls whether items default to sensitive |
| `@disable` | Disables the current file, optionally conditionally |
| `@generateTypes` | Generates types from the schema |
| `@import` | Imports schema and values from another file or directory |
| `@plugin` | Loads a plugin that can register decorators, types, and resolvers |
| `@redactLogs` | Controls whether sensitive values are redacted in logs |
| `@preventLeaks` | Controls whether outgoing responses are scanned for secret leaks |
| `@setValuesBulk` | Injects many config values from a single data source |

### Item Decorators

| Decorator | Description |
|---|---|
| `@required` | Marks an item as required |
| `@optional` | Marks an item as optional |
| `@sensitive` | Marks an item as sensitive |
| `@public` | Marks an item as not sensitive |
| `@type` | Sets the item data type |
| `@example` | Adds an example value |
| `@docs` | Attaches documentation URLs to an item |
| `@icon` | Attaches an icon identifier to an item |

## Supported Data Types

`string`, `number`, `boolean`, `url`, `simple-object`, `enum`, `email`, `ip`, `port`, `semver`, `isoDate`, `uuid`, `md5`

Each type supports contextual completion for its options (e.g. `minLength`, `startsWith` for strings; `min`, `max`, `isInt` for numbers).

## Supported Resolvers

`concat`, `fallback`, `exec`, `ref`, `regex`, `remap`, `forEnv`, `eq`, `if`, `not`, `isEmpty`, `inferFromPrefix`

## Example

```env
# @currentEnv=$APP_ENV
# @defaultRequired=infer

# @type=string(minLength=8)
# @required
DATABASE_URL=postgres://localhost:5432/mydb

# @type=enum(development, staging, production)
# @required
NODE_ENV=development

# @type=number(min=1024, max=65535)
PORT=3000
```

## Installation

### Build from source

```bash
cargo build --release
```

The binary will be at `target/release/varlock-lsp`.

### Move the binaries
cp target/release/varlock-lsp ~/.local/bin/

### Editor Configuration

#### Helix

Add the contents of [`helix/languages.toml`](helix/languages.toml) to your Helix languages config (`~/.config/helix/languages.toml` or project-level `.helix/languages.toml`):

```toml
[[language]]
name = "env-spec"
scope = "source.env-spec"
file-types = [
  { glob = "**/.env" },
  { glob = "**/.env.*" },
]
comment-token = "#"
language-servers = ["varlock-lsp"]

[language-server.varlock-lsp]
command = "varlock-lsp"
```

#### VS Code / Other Editors

Configure your editor to use `varlock-lsp` as the language server for `.env` files. The server communicates via stdio.

## Development

```bash
# Build
cargo build

# Run
cargo run
```
