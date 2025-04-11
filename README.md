# Chug

The fastest way to consume Homebrew bottles.

## Installation

To install the latest release:

```sh
curl -fsSL https://chug.bend.nz/install.sh | sh
```

Or, to install from source:

```sh
cargo install chug-cli
```

## Usage

```sh
chug add $formula_name
chug remove $formula_name
chug update
```

## Rationale

[Homebrew](https://brew.sh/) is the de-facto standard package manager for 3rd-party development tools on macOS. Most of these tools are built using "formulae" and their pre-built binaries can be downloaded as "bottles". However, Homebrew still requires that users download a significant portion of the Homebrew toolchain to install bottles. Chug aims to improve on Homebrew in the following ways:

- Single-purpose
  - Chug can only download and manage bottles
- Efficiency
  - Chug is pre-compiled to a single binary (no more [Ruby DSLs](https://docs.brew.sh/Formula-Cookbook))
  - Chug is written in Rust
  - Chug downloads and extracts bottles in parallel
  - Chug extracts bottles as they are being downloaded
  - Chug avoids using external programs where practical
- Cleanliness
  - Chug installs packages on a per-user basis
  - Chug avoids changing permissions for `/usr/local`, `/opt/homebrew`, etc.
  - Chug follows the [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir-spec/latest) and installs binaries to `~/.local/bin` (configured via `$XDG_BIN_HOME`)

## TODO List

- [ ] Issues around patching on macOS (particularly for `python@3.13`)
- [ ] Linux support
- [x] `curl -fsSL https://chug.bend.nz/install.sh | sh`
- [ ] `chug list` and `chug tree`

## Non-goals

To keep this project fast and maintainable, the following are non-goals:

- Casks or building formulae from source
- Non-Homebrew sources
