# wirediff: wire identity of two builds, on behaviour

Checks that two builds of a project put the same bytes on the wire and read them the same way: an old build of the
serializer generator against a new one, once comparing their Luau text no longer means anything. Each build harvests
its serializers, network decoders and packed call sites as standalone Luau modules; `wirediff.cjs` pairs the two
harvests and runs every pair on Lune with the same seeded values.

## Harvesting

Build `rbxtsc-fw` with the `wire-harvest` feature (off in every normal build) and build the project with
`FLAMEWORK_RS_WIRE_DIR` set to an empty folder:

```sh
cargo build -p rbxtsc_fw --features wire-harvest
FLAMEWORK_RS_WIRE_DIR=/tmp/harvest-new rbxtsc-fw build -p <project>
```

The project's own output is the same as without the feature. Each file with a site writes
`<dir>/<file id>/<ordinal>.luau` (`<file id>` the source's path from the root directory, ordinals in source order, so
two builds pair up): the file's hoisted statements, then `return { kind, layout, key, ... }`:

| kind | what the build emitted | added for the test |
|---|---|---|
| `serializer` | `serializer`: `Flamework.createSerializer`'s object | |
| `decoder` | `deserialize`: a `network-decoder` intrinsic | `encode(args)`: an encoder of the list |
| `site` | `pack(args)`: a packed send's packing, its arguments read from `args` | `deserialize`: a decoder of the list |
| `callback` | `pack(value)`: a `setCallback` packer | `deserialize`: a decoder of the result |

`layout` is the layout text, `key` the wire key the values are generated from. A site also has `arity` (its number of
arguments) and `fixed` (the literal or `undefined` arguments its packing was built for). What is added for the test is
built by a second generator of the file (`serializer::wire`), which uses `__fwguard(<key>)` in place of a `t` guard.

## Running

```sh
node tools/wirediff/wirediff.cjs <old harvest> <new harvest> [--seed N] [--random N] [--jobs N] [--redact] ...
```

Options are described at the top of `wirediff.cjs`. Needs `lune` (0.10) on the PATH. Per pair, the values are every
edge value of the key's kinds (0 and the bounds of each width, ±0, NaN and the infinities, empty and 255/256/65536-byte
strings and buffers, empty and one-element collections, nil and non-nil optionals, every union member and the `whole`
path, every literal and enum item) and 200 random ones (back-references followed at most 4 deep). The checks
(`wirediff.luau` has the details):

- **W1** both encoders give the same buffer and blob list, or raise the same message;
- **W2** each decoder reads the other's buffer, and all four readings agree (a reading other than the value itself is
  counted as a lossy round trip, not a failure: an f32, a union member written as another);
- **W3** invalid values (wrong type, out of width, non-integer, a hole, an unknown literal, a missing field): the same
  message, or both accept with the same bytes;
- **W4** malformed buffers (every truncation up to 256, 64 byte flips, an appended byte): the same reading or message;
- **W5** versioned serializers: the version byte leads the buffer; a changed version or hash byte, and a buffer too
  short for the header, are refused with the same message.

W0 counts pairs that cannot be compared: one module loads and the other does not, or their kind, layout, arity or
literal arguments differ. Messages are compared without Luau's `chunk:line:` prefix, and the warnings a call makes are
part of its result. The summary gives pairs, values, and per check the failures with their first examples (`--redact`:
counts only). Exit 0 means no failure.

The Roblox side (`env.luau`) is Lune's `@lune/roblox` datatypes and Instances, `Enum` items kept per item, `t`
(`vendor/t.lua`, @rbxts/t 3.2.1, MIT), `game:GetService("RunService")` for the `--realm`, and `warn` recorded.
