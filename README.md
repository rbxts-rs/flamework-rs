# flamework-rs

Flamework's TypeScript transformer (v2, `@flamework-experimental/transformer`) ported to Rust, as a plugin compiled into
[rbxts-rs](#dependencies) (roblox-ts in Rust, on tsrs), plus `rbxtsc-fw`: the `rbxtsc` command line with Flamework
compiled in, the binary a Flamework project builds with.

- `crates/flamework_rs`: the plugin (ids, decorators and metadata, user macros and intrinsics, guards, the serializer,
  components, `flamework.build`, the include JSON) and the byte-exact libraries its output depends on.
- `crates/rbxtsc_fw`: the `rbxtsc-fw` binary. It takes roblox-ts's `build` arguments and returns its exit codes;
  Flamework is on for a project whose tsconfig lists `@flamework-experimental/transformer` in
  `compilerOptions.plugins`.

**Status:** a personal port, provided as is; fork it if it is useful to you. Every part of the TS transformer is ported,
networking included. It follows TypeScript 7.1 (through tsrs) and its own build order rather than reproducing the TS
transformer's output: what a build does at run time (evaluation order, what is reflected, guards, the serializer's wire
format and layouts) is the TS transformer's, except for the items marked On purpose below, while generated text and the
order of some artefacts differ, see [Differences from the TS transformer](#differences-from-the-ts-transformer).

**Licence:** MIT (`LICENSE`). Parts are ports of Flamework, hashids, uuid, glob, minimatch and brace-expansion; their
notices are in `THIRD-PARTY-NOTICES.md`.

## Dependencies

rbxts-rs (the compiler and its plugin API, `rbxts_plugin`) and tsrs are git dependencies pinned by rev in the root
`Cargo.toml`. The tsrs URL and rev must be exactly the ones rbxts-rs's own `Cargo.toml` names at the pinned rbxts-rs
rev: Cargo does not enforce one copy of a git dependency, so a mismatch builds a second tsrs and fails only later, with
E0308 on a tsrs type. `bash tools/check-deps.sh` fails when any `tsrs_*` or `rbxts_*` crate is in the graph twice; run
it after changing a dependency, a `[patch]` or `Cargo.lock`. `Cargo.lock` started as a copy of rbxts-rs's.

`DESIGN §n` in the code and docs refers to rbxts-rs's `docs/design.md`.

## Build and test

The toolchain comes from `rust-toolchain.toml`. Dev builds are optimised (a debug checker is unusably slow).

```sh
cargo build --locked
cargo test --locked
bash tools/check-deps.sh
```

The end-to-end tests (`e2e`, `guards_e2e`, `macros`, `macros_env`, `serializer_e2e`) build fixtures with the real
compiler. They need the built Flamework v2 packages (a checkout of its monorepo with `packages/core/out` built:
`FLAMEWORK_RS_MONOREPO`) and the `@rbxts` packages `compiler-types`, `t` and `types` (a `node_modules/@rbxts` folder:
`FLAMEWORK_RS_RBXTS`; its `compiler-types` must declare `Iterable` with TypeScript 7's three type parameters, and its
`types` must have the `vector` library). Without them they are skipped, saying so; so is the unit test that checks the
embedded JSON schemas against the transformer's (`FLAMEWORK_RS_MONOREPO`). To set both for every `cargo test` on one
machine, put them in an uncommitted `.cargo/config.toml` (git-ignored):

```toml
[env]
FLAMEWORK_RS_MONOREPO = "<monorepo>"
FLAMEWORK_RS_RBXTS = "<project>/node_modules/@rbxts"
```

`diagnostics` is the TS transformer tests' failure assertions as one table (`tests/diagnostics/cases.json`, built on
the TS tests' own fixture in `tests/diagnostics/fixture`): each case's build must fail with the TS transformer's
message. The file says how a case is written, marked or left to another test.

## Install

`rbxtsc-fw` is installed with Cargo from a pinned commit, one folder per Flamework version (the paths below are
examples; `<url>` stands for this repository's git URL):

```sh
cargo install --locked --git <url> --rev <hash> rbxtsc_fw --root E:\Tools\rbxts\fw-2.0.0-beta.0
```

`--locked` makes Cargo use the repository's `Cargo.lock`; without it, `cargo install` ignores the lock and resolves
the dependencies again. `cargo install` builds with the release profile (fat LTO, one codegen unit), so it builds
slowly.

That gives `E:\Tools\rbxts\fw-2.0.0-beta.0\bin\rbxtsc-fw.exe`. The plain compiler from rbxts-rs (no Flamework) goes in a
folder of its own the same way, for example `E:\Tools\rbxts\rbxtsc\`.

- **A project** points its `package.json` build script at the folder that matches its `@flamework-experimental/*`
  version (`"build": "E:/Tools/rbxts/fw-2.0.0-beta.0/bin/rbxtsc-fw.exe build"`).
- **Upgrading** is changing that path and the npm packages together, to the folder of the new version.
- **Cleaning up** an old version is deleting its folder, or `cargo uninstall rbxtsc_fw --root <dir>`.
- **Where the build goes:** `cargo install` builds in a temporary target directory, removed afterwards, unless
  `CARGO_TARGET_DIR` is set. A build in a checkout goes to `target/` (or `--target-dir`, or `[build] target-dir` in
  `.cargo/config.toml`), which `cargo clean` removes. Cargo's git and registry caches under `~/.cargo` (`git`,
  `registry`) are safe to delete; they are downloaded again when needed.
- The dev profile is optimised (`opt-level = 3`), so a checkout's `cargo build` gives a usable compiler too.

## Developing against a local rbxts-rs

To build against a local rbxts-rs checkout instead of the pinned rev, override the git source with path dependencies
in an uncommitted `.cargo/config.toml` (git-ignored), one line per rbxts-rs crate in `Cargo.lock`:

```toml
[patch."file:///E:/Projects/TS/Flamework/rbxts-rs"]
rbxts_cli = { path = "<rbxts-rs>/crates/rbxts_cli" }
rbxts_luau = { path = "<rbxts-rs>/crates/rbxts_luau" }
rbxts_luau_check = { path = "<rbxts-rs>/crates/rbxts_luau_check" }
rbxts_plugin = { path = "<rbxts-rs>/crates/rbxts_plugin" }
rbxts_project = { path = "<rbxts-rs>/crates/rbxts_project" }
rbxts_transform = { path = "<rbxts-rs>/crates/rbxts_transform" }
```

While it is on, Cargo rewrites `Cargo.lock` for the path sources, so `--locked` fails: build without it, and restore
`Cargo.lock` (`git checkout Cargo.lock`) when you remove the patch. The local checkout must name the same tsrs as this
workspace (`tools/check-deps.sh`). When the change lands in rbxts-rs, move the rev in `Cargo.toml` to it and update the
lock with `cargo update -p rbxts_plugin` (which moves every crate of that source).

## Comparing with the TS transformer

The TS transformer's output (TypeScript 5 roblox-ts with Flamework's TS transformer) stays as a test oracle, not a
target: `tools/parity.cjs` and `tools/compare.cjs --normalize` show what a change touched, and every remaining
difference should be one of the [Differences from the TS transformer](#differences-from-the-ts-transformer).
`tools/parity.cjs` builds a whole project with `rbxtsc-fw` and compares the output, file by file and byte for byte,
with a reference build of the same project:

```sh
node tools/parity.cjs <rbxtsc-fw exe> <project dir> <reference out dir> <work dir> [--report FILE] [--diff N] [--save-out DIR]
```

It copies the project into the work dir (without its `out`, `include` and build infos; links are kept as junctions),
builds it, and prints `build exit N` and `identical A, differing B, only ours C, only ref D` (`tools/compare.cjs`, also
usable on its own). The options are described at the top of `tools/parity.cjs`.

With `--normalize` (both tools), Luau files are compared with their generated locals renamed to canonical names
(`tools/normalize.cjs`, also usable on its own: `node tools/normalize.cjs <file.luau> [<source.ts>]`). The generated
locals are named differently from the TS transformer's (Differences), so this is the comparison to make against its
reference; it also checks that a change touched generated names only. What is renamed: a name the file declares
(`local`, a `for` variable, a parameter, `local function`) that is not an identifier of the file's source
(`parity.cjs` finds the sources under the project's `src`, `--src` to change it; `compare.cjs` takes `--src DIR`),
wherever it stands as a name. The compiler never names a generated local after an identifier of the source, so the
user's own locals keep their names. Keywords, field names, table constructors' keys, `self`, globals (names the file
never declares, such as `Vector3`), strings and comments stay as written. A renamed name becomes `<class>#<n>`: its
class is its name without a trailing number (`buf_2` is `buf`), and `n` counts the class's renamed names in the order
they first occur in the file; a renamed `_`-class name (`_`, `_1`, …) that is only declared, never used, becomes `_#`.
Two files normalise alike exactly when one is the other with renamed names consistently swapped within their classes
(and never-used `_`-class names swapped freely): a changed constant, a different order, a global, a renamed user
local, or a use that reads another local still differs. The whole file is one scope, which is stricter than Luau's.
Without a source (`compare.cjs` without `--src`, a file no source maps to), every declared name is renamed, so a
user's local renamed consistently goes unnoticed.

## Differences from the TS transformer

flamework-rs's output differs from the TS transformer's (TypeScript 5) in the ways below. Unless an item says
otherwise, only the generated text or an artefact's order differs, not what the build does.

Text and order:

- **The names of Flamework's generated locals** (`buf`, `value_2`, a guard's `v_Tree`, …): each is named when it is
  built, as the compiler approves it (never a name the file uses), so the numbers after a name differ from the TS
  printer's. `--normalize` (above) hides these names and nothing else.
- **The order things are recorded in within a file:** a file's new ids (`flamework.build`'s `identifiers` and the
  counters of `short` / `tiny` / `obfuscated` ids), its `Flamework.env` inputs and its glob and path uses (their entries
  in `flamework.build` and `include/flamework/globs.json`, and the order of the glob warnings; path warnings stay in
  source position order) follow the order the compiler's hooks run in (roblox-ts lowers a call chain from its innermost
  call, so a module's `.registerProvidersGlob` calls come in source order, and a class's metadata after its members),
  not the TS transformer's walk. Files still come in program order, so the output is the same for any checker count;
  `full` ids are paths and do not move, and a clean build's new salt changes `short` ids anyway.
- **Guard key order of a mapped type over a union of keys** (`Pick<T, "b" | "a">`, `Omit`, `Record`, a key-remapping
  `as` clause, and a rest or spread of one): the TS transformer lists the keys in the order its checker first made
  the keys' literal types (it shifts with unrelated edits). The guard checks the same keys. Serializers of these
  types order their fields by the mapped type itself and are exact.
- **Guard text of a union** (`t.union(...)`, a literal union): the members are listed in the order TypeScript 7 gives
  a union's constituents.
- **The order of a class's hoisted serializer helpers** (the codecs, `createSerializer` locals and helpers that the
  members of one class hoist), and the order in which its networking calls claim the file's budget of locals: they
  follow roblox-ts's compile order of the members (instance properties, constructor, methods, statics), not source
  order. Each helper is still defined before the code that uses it.
- **The order of a network's members.** The keys of a mapped type over `keyof T` (`incomingIds`, `outgoingIds`,
  `namespaceIds`, and the decoders built for them) are listed by a key of each, where the TS transformer lists them in
  the order its checker first made their literal types. Remotes are matched by name, so a server and a client built
  by the two compilers work together.
- **A returned send past its function's budget of locals** (`return events.alpha.fire(x)` in a function that already
  holds many of Flamework's locals): the call and its packing go in a function, where the TS transformer puts the
  `return` statement in a block of its own with the packing.
- **A send in a `for` loop's incrementor** (`for (let i = 0; i < n; events.alpha.fire(i), i++)`): the packing is not
  seen as a closure, so the loop has no per-iteration copies of `i`. The function is called where it is made, so
  nothing can see the difference.
- **A comment at the end of a file** (after its last statement) is kept; the TS transformer drops it.

Order that depends on symbol ids (can reach a serializer's layout): a union that no written type orders (`Box<A | B>`
reaching `value: T`) orders its members by a text of each type, and an object's text spells a unique-symbol key with
the symbol's id (`__@sym@12`). Two keys whose texts agree up to the id (two unique symbols with the same name from
different modules, or a user key `"__@zeta@1"` against a real `[zeta]`) are ordered by ids that differ between the
compilers (and, in the TS transformer, with unrelated edits); give such unions a written order (a type alias of the
union) instead.

Diagnostics: with `noSemanticDiagnostics: true`, a file with both type errors and Flamework errors reports the type
errors, where the TS transformer reports the Flamework error. The build fails either way.

On purpose:

- **A packed send followed by more of its optional chain** (`maybe?.fns.add.invoke(x).andThen(f)`): packed inside the
  compiler's nil check, so the rest of the chain runs only when `maybe` is defined. The TS transformer's output fails
  its own type check there (TS2532). A send that ends its chain (`maybe?.alpha.fire(x)`) is compiled as the TS
  transformer compiles it.
- **Obfuscated keys through an optional chain** (`maybe?.alpha.fire(x)` with `transformer.obfuscation`): the member
  name is obfuscated as outside a chain. The TS transformer leaves it as written there, so its send reaches no remote.
- **`middleware` written as a shorthand property, a method or an accessor** (`createServer({ middleware })`): a build
  error, as for any value that is not an object literal (the TS transformer fails an internal assertion).
- **A package's `flamework.build` is looked for only in the package's directory** (beside the `package.json` of the
  program's files), not in every directory up to four above each file. Every look is a build input the watcher
  watches, so a package installed and imported while watching is read on that rebuild. A `flamework.build` in a
  package's subfolder is not read: that package's ids fall back to unprefixed path ids (`@scope/name:out/...`).

Refused, where the TS transformer builds:

- **Spreading a map, an iterable function or a generator into a packed call's arguments** (`events.list.fire(...map)`):
  arrays, tuples, strings and sets are spread as roblox-ts spreads them; the rest is a build error. Spread the values
  into an array first (`[...map]`).
- **An inlined or rewritten macro call followed by more of its optional chain** (`maybe?.inlined<T>().size()`).
