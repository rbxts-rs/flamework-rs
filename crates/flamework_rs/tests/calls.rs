//! Macro calls through `super.m()`, `super.m?.()`, `o.m?.()`, `o?.m()` and `o?.m?.()` end to end, through the real
//! compiler on `tests/fixture_calls`: macro methods of every marker kind (`src/shared/macros.ts`) called in each shape
//! (`optcall.ts`, `optrecv.ts`, `optboth.ts`, `super.ts`, the same marker list in each), longer chains, function
//! and core macros in chains, callees that are not macros in TS5 (`o["m"]()`, `(o.m)()`, `o.m!()`), and guards in a
//! written argument ahead of a generated serializer (`argguard.ts`), and calls TS5
//! leaves alone because the callee's own symbol is not the macro, whose serializers must not be made (`al.ts`, `ob.ts`,
//! `vleak.ts`: an alias, a property holding the macro, a callee holding a call).
//!
//! `tests/fixture_calls/expected/out` holds rbxtsc-fw's output, each file compared whole (`UPDATE_EXPECTED=1` writes
//! it). It is the TS5 pipeline's output (roblox-ts master with the published 2.0.0-beta.0 transformer,
//! `fieldInfoPlugin.cjs` as its plugin) but for the generated locals' names (checked with `tools/normalize.cjs`) and
//! the serializers' code, which flamework-rs builds as Luau of its own (README, Differences).
//!
//! `tests/fixture_chains` holds inlined and rewritten macro calls that end optional chains, which TS5 compiles by
//! replacing the call node (the `crate::macros` docs): `ts5-reference/chaingaps.luau` is TS5's output for the source,
//! compared with the generated locals' names normalised (`support::expected::normalize`, the names of the source kept).

#![cfg(windows)]

mod support;

use support::fixture_macros::flamework_with_fixture_macros;
use support::expected::{check, first_difference, normalize, source_identifiers};
use support::project::{Project, assert_clean, fixture_dir, luau};

const FIXTURE: &str = "fixture_calls";

fn expected_files(dir: &std::path::Path, relative: &str, out: &mut Vec<(String, std::path::PathBuf)>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let name = format!("{relative}{}", entry.file_name().to_string_lossy());
        if entry.file_type().unwrap().is_dir() {
            expected_files(&entry.path(), &format!("{name}/"), out);
        } else {
            out.push((name, entry.path()));
        }
    }
}

/// Every call shape and marker equals the expected output byte for byte.
#[test]
fn every_shape_matches_expected() {
    let Some(project) = Project::new(FIXTURE, "shapes") else { return };
    let report = project.build_with(flamework_with_fixture_macros());
    assert_clean(&report);

    let mut files = Vec::new();
    expected_files(&fixture_dir(FIXTURE).join("expected/out"), "", &mut files);
    assert!(files.len() >= 10, "expected outputs missing ({})", files.len());
    let failures: Vec<String> =
        files.iter().filter_map(|(name, path)| check(name, path, luau(&report, name))).collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Inlined and rewritten macro calls that end an optional chain, compiled as TS5 replaced the call node (41 calls: 28
/// inlined, 13 rewritten): `fixture_chains/ts5-reference/chaingaps.luau` is TS5's output for the same source, compared
/// whole with the generated locals' names normalised (the names of `chaingaps.ts` kept as written).
#[test]
fn inlined_and_rewritten_calls_in_optional_chains_match_ts5() {
    let Some(project) = Project::new("fixture_chains", "chains") else { return };
    let report = project.build_with(flamework_with_fixture_macros());
    assert_clean(&report);
    let dir = fixture_dir("fixture_chains");
    let keep = source_identifiers(&std::fs::read_to_string(dir.join("src/shared/chaingaps.ts")).unwrap());
    let want = std::fs::read_to_string(dir.join("ts5-reference/chaingaps.luau")).unwrap().replace("\r\n", "\n");
    let want = normalize(&want, &keep);
    let got = normalize(luau(&report, "shared/chaingaps.luau"), &keep);
    assert!(got == want, "{}", first_difference("chaingaps.luau (normalised)", &got, &want).unwrap_or_default());
}

/// The normaliser the TS5 reference is compared with hides a consistent renaming of generated locals, and nothing
/// else.
#[test]
fn normalize_hides_renamed_locals_only() {
    let none = std::collections::HashSet::new();
    let plain = |text: &str| normalize(text, &none);
    let a = "local buf = 1\nlocal buf_1 = { buf = buf }\nprint(buf_1.buf, `{buf}`, \"buf\")\nfor _ = 1, 2 do end\n";
    let b = "local buf_3 = 1\nlocal buf = { buf = buf_3 }\nprint(buf.buf, `{buf_3}`, \"buf\")\nfor _2 = 1, 2 do end\n";
    assert_eq!(plain(a), plain(b));
    assert_eq!(
        plain(a),
        "local buf#0 = 1\nlocal buf#1 = { buf = buf#0 }\nprint(buf#1.buf, `{buf#0}`, \"buf\")\nfor _# = 1, 2 do end\n"
    );
    // A use that reads the other local, or another constant, still differs.
    let c = "local buf = 1\nlocal buf_1 = { buf = buf_1 }\nprint(buf_1.buf, `{buf}`, \"buf\")\nfor _ = 1, 2 do end\n";
    assert_ne!(plain(a), plain(c));
    assert_ne!(plain(a), plain(&a.replace("= 1\n", "= 2\n")));
    // Globals are never renamed; nor are the keys after an if-expression; a `_` that is read is a local like any other.
    assert_ne!(plain("local p = Vector3.new(1)\n"), plain("local p = Vector2.new(1)\n"));
    assert_ne!(plain("local b = buffer.create(4)\n"), plain("local b = buffer1.create(4)\n"));
    let t = "local t = { a = if c then 1 else 2, p1 = f(), p2 = g() }\nreturn t.p1\n";
    assert_ne!(plain(t), plain(&t.replace("p1 = f(), p2 = g()", "p2 = f(), p1 = g()")));
    let u = "local _ = 1\nlocal _1 = 2\nprint(_)\n";
    assert_ne!(plain(u), plain(&u.replace("print(_)", "print(_1)")));
    // The names of the source stay as written, so a user's local renamed still differs.
    let keep = source_identifiers("const player = Players.LocalPlayer; // `x`\nprint(`${player} ${\"y\"}`);\n");
    assert!(keep.contains("player") && keep.contains("print") && !keep.contains("x") && !keep.contains("y"));
    let user = "local player = game.Players.LocalPlayer\nprint(player)\n";
    assert_ne!(normalize(user, &keep), normalize(&user.replace("player", "player_1"), &keep));
    assert_eq!(plain(user), plain(&user.replace("player", "player_1")));
}
