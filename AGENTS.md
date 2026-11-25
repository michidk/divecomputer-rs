# Agent instructions

## Project overview

divecomputer-rs downloads dive logs from dive computers and exports them to open
formats. It ships two things:

- **An open protocol specification** in `docs/spec/`, published with mdBook from `docs/`
  to GitHub Pages. It describes about 40 protocol families: transports, commands,
  framing, memory layouts and dive data formats.
- **Rust crates** implementing that specification as a library that runs natively and in
  the browser (WebAssembly) from the same code. The design is in
  [ARCHITECTURE.md](ARCHITECTURE.md); read it before writing or reviewing Rust code.

Everything is licensed MIT OR Apache-2.0. The project must stay free of GPL and LGPL
code; see [Clean-room rules](#clean-room-rules).

## Repository map

- `docs/`: mdBook root (`book.toml`, `SUMMARY.md`, `README.md` landing page, `theme/`)
- `docs/spec/`: the specification
  - `README.md` layout and clean-room policy, `conventions.md` notation and units,
    `TEMPLATE.md` family document structure
  - `common/`: checksums (`checksums.md`), date/time encodings, common data model
  - `transports/`: serial and USB-serial chips, USB HID, raw USB, Bluetooth classic, BLE,
    IrDA, USB storage and MTP
  - `families/`: one document per protocol family, plus `README.md` index
- `crates/`: Cargo workspace members. Today `divecomputer-core` (checksums, SLIP,
  errors, transport traits, raw dives, the parsed dive model and units),
  `divecomputer-protocol` (Shearwater download and log parser),
  `divecomputer-transport` (Web Bluetooth and a browser clock), `divecomputer-export`
  (DiveJSON export of the dive model, through the `divejson` crate) and
  `divecomputer-wasm` (JavaScript bindings and a test page in `www/`). All but core are
  `publish = false` until they are ready; the other crates in ARCHITECTURE.md are added
  as they are built. `divejson` and its `uddf` dependency use published crates.io
  releases; no sibling checkouts are required.
  Browser-only code is gated on `all(target_arch = "wasm32", target_os = "unknown")`.
  Protocol integration tests live in `crates/<crate>/tests/`. Workspace-wide settings
  live in the root `Cargo.toml` (`[workspace.package]`, `[workspace.dependencies]`,
  `[workspace.lints]`), `clippy.toml`, `rust-toolchain.toml` and `deny.toml`.
- `.github/workflows/`:
  - Managed by Orcastrate from `michidk/workflows` (frontmatter at the top; never edit
    below it by hand, change the template or its `params` instead): `ci.yml` (Test &
    Lint), `pr-title.yml`, `bump-version.yml`, `release.yml`, `publish-cratesio.yml`.
    `cliff.toml` is managed the same way.
  - Repository-owned: `checks.yml` (WebAssembly and `no_std` builds, tests on
    `wasm32-wasip1`, `cargo-deny`) and `docs-pages.yml` (mdBook build, link check,
    GitHub Pages deploy).

## Releases

1. Merge work into `main` with Conventional Commit titles; git-cliff derives the next
   version from them.
2. Run **Bump Version & Tag** (`bump-version.yml`). It updates the versions of the
   crates listed in its `manifest_paths`, commits through GitHub's signing API and tags
   `vX.Y.Z`.
3. The tag triggers **Release** (`release.yml`), which verifies the tag and publishes a
   GitHub release with git-cliff notes.
4. The published release triggers **Publish to crates.io** (`publish-cratesio.yml`)
   through crates.io Trusted Publishing.

When a new crate becomes publishable, add its manifest to `manifest_paths` in the
frontmatter of `bump-version.yml`, `release.yml` and `publish-cratesio.yml`, in
dependency order (libraries before the crates that depend on them), and configure its
Trusted Publisher on crates.io.

## Working style

- Work on a feature branch. Do not push to `main`; push the branch and give the user a
  compare link for the pull request.
- Keep each change focused. A pull request changes the spec, the code, or both for one
  family or one concern.
- When code and spec disagree, fix whichever is wrong in the same pull request. Code
  never silently diverges from the spec.
- Prefer the existing structure and naming over new patterns. If a new pattern is needed,
  update ARCHITECTURE.md or this file in the same pull request.

## Commit conventions

[Conventional Commits](https://www.conventionalcommits.org/): `type(scope): description`.
Pull request titles are checked by CI against the same format.

- Types: `feat`, `fix`, `refactor`, `perf`, `docs`, `style`, `test`, `build`, `ci`, `chore`
- Scopes: a crate without its prefix (`core`, `protocol`, `transport`, `export`, `wasm`,
  `cli`), a family module or document name (`shearwater`, `suunto-eonsteel`,
  `pelagic-atom2`), or `spec` for cross-cutting specification changes.
- Examples: `docs(shearwater): document log version 14 sample fields`,
  `feat(protocol): add Mares Icon HD download`, `fix(core): reject truncated SLIP frames`

## Clean-room rules

These apply to the specification and to code, and they are not negotiable:

- Learn **facts** from any source (vendor documents, device captures and dumps, other
  open-source projects including libdivecomputer). Facts are byte values, offsets, sizes,
  scaling, UUIDs, USB IDs, timings, checksum parameters, model codes and behaviour.
- **Never copy** code, comments, identifiers, prose or table layouts from other projects,
  whatever their licence. Do not translate C into Rust line by line. Write from the spec.
- Never add a dependency on GPL or LGPL code, including bindings to libdivecomputer.
- In the spec, name sources only in each document's "References" section, as public
  sources. No line references into other projects.

## Specification rules (`docs/spec/`)

- Family documents follow `docs/spec/TEMPLATE.md` section by section. A section that does
  not apply says so in one line instead of being removed.
- Follow `docs/spec/conventions.md` for byte notation (`1B 2C FF`, `H→D`/`D→H`), integer
  types (`u16le`), units and confidence levels.
- Confidence: **verified** only when vendor documentation or captured device traffic
  confirms the fact; **likely** for one reliable source; **uncertain** otherwise. Never
  upgrade confidence because two implementations agree; they may share a mistake.
- Every checksum is defined once in `common/checksums.md` (anchor = lowercase RevEng name)
  and linked from family documents. Do not redefine an algorithm inside a family
  document. The same applies to transports, date/time encodings and the data model: link
  to the shared document, describe only what is family-specific.
- Where sources disagree, document every variant and add an open question. Do not pick
  one silently.
- Give at least one worked example frame per protocol, computed with a script, and state
  the check value for every CRC used.
- Every page is listed in `docs/SUMMARY.md` and, for families, in
  `docs/spec/families/README.md`.
- Write plainly: short sentences, tables for structured data, no marketing language.

## Rust rules

Read [ARCHITECTURE.md](ARCHITECTURE.md) first. The rules below make it concrete.

### Structure and duplication

- Each concern exists exactly once. Before writing a helper, search `divecomputer-core`
  for it. If two families need the same logic, move it to core rather than copying it.
  Typical shared pieces: CRCs and sums, SLIP/HDLC/COBS framing, BCD and packed date
  decoding, ring-buffer walking, paged memory reads with caching, BLE chunking.
- Use the `crc` crate's algorithms for CRCs instead of hand-written loops, and name them
  after the spec's anchors.
- Protocol code depends only on transport **traits** from core. It must not import
  `divecomputer-transport`, `web-sys`, `nusb`, `btleplug` or any runtime.
- Download (async, transport-bound) and parsing (sync, pure) are separate functions. A
  parser takes bytes and returns the dive model; it never performs I/O.
- Model-specific differences are data (tables of layouts, offsets and scale factors), not
  copies of functions with small edits.
- Constants carry names that say what they mean (`CMD_READ_PAGE`, `PAGE_SIZE`) and cite
  the spec section in their doc comment.

### Idiomatic Rust

- Edition 2024, stable toolchain, `#![forbid(unsafe_code)]` in every crate except where a
  platform backend genuinely needs `unsafe` (justify it in a `// SAFETY:` comment).
- `core`, `protocol` and `export` are `no_std` + `alloc`. Do not add `std` to them.
- Make invalid states unrepresentable: newtypes for units and identifiers, enums instead
  of booleans and magic numbers, builders or typed configs for transport settings.
- No `unwrap`, `expect`, indexing that can panic, or integer overflow on device input.
  Use checked arithmetic and `get(..)`; return an error that names the field or offset.
- Errors: one `thiserror` enum per crate, `#[non_exhaustive]`, variants that let callers
  decide (timeout vs checksum vs protocol violation vs unsupported model).
- Public items have doc comments that state units, ranges and failure cases. Each public
  module and each family module has a top-level doc comment linking its spec document.
- Keep public APIs small. Default to `pub(crate)`; make an item public only when a user
  needs it. Mark public enums and structs that will grow `#[non_exhaustive]`.
- Async code is runtime-agnostic and must compile for `wasm32-unknown-unknown`. No
  `std::thread`, blocking I/O or `std::time::Instant` in protocol code; use the `Clock`
  trait.
- Prefer iterators and slices over index loops, `impl Trait` arguments for single-use
  generics, and borrowing over cloning. Avoid `Box<dyn Error>` and stringly-typed errors.
- Dependencies: check licence and maintenance before adding one (see the dependency policy
  in ARCHITECTURE.md). Add it to the workspace `[workspace.dependencies]`, never with a
  per-crate version.

### Tests

- Every checksum, codec and byte decoder has unit tests, including the test vectors from
  the spec.
- Every protocol has a session-replay test from a recorded session and parser snapshot
  tests (`insta`). New fixtures go under the protocol's test directory with a short
  README naming device, firmware and what the session covers.
- Every parser and framing decoder has a fuzz target.
- Tests for `core`, `protocol` and `export` also run on `wasm32-unknown-unknown`.

## Development checks

Run before every commit that touches the respective area.

```bash
# Rust
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo clippy --workspace --target wasm32-unknown-unknown -- -D warnings  # .cargo/config.toml adds --cfg=web_sys_unstable_apis
CARGO_TARGET_WASM32_WASIP1_RUNNER=wasmtime cargo test --workspace --target wasm32-wasip1
cargo build -p divecomputer-core -p divecomputer-protocol -p divecomputer-export --target thumbv7em-none-eabihf
cargo deny check

# Documentation
mdbook build docs
lychee --offline --include-fragments --exclude-path docs/SUMMARY.md 'docs/**/*.md'
```

Use the shared Cargo target directory configured on the machine. Do not set
`--target-dir` or create per-project build directories.

## README and documentation conventions

- The root `README.md` is for people who have never heard of the project: what it does,
  supported devices (link to the spec), quick start for Rust and for the browser, crate
  map, licence. Keep protocol detail in `docs/spec` and link to it.
- Each crate has a short `README.md` (used on crates.io): purpose, feature flags, a
  minimal example, link to the docs.
- Keep `ARCHITECTURE.md` current. It describes the code as it is, plus clearly marked
  open decisions.
