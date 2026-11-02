//! F2 end to end: user macros of every marker kind, the intrinsics, the parameter intrinsics and macro types, through
//! the real compiler on `tests/fixture_f2` (its plugin is `tests/support/fixture_macros.rs`).
//!
//! `tests/fixture_f2/expected` is a full-mode build, checked against the TS5 pipeline (roblox-ts master with the
//! monorepo transformer at 1a72fa1, `transformer.plugins` = the fixture's `fieldInfoPlugin.cjs`) on the same
//! sources (`seams.ts`: synthesized strings, property order, the hoisted-metadata comment ahead of the TS
//! statement, `{@link}` text; `ünï.ts`: an id with non-ASCII characters). It equals TS5's output but for
//! - `seams.luau`: properties in TypeScript 7.1's order, so a mapped type over an interface (`Readonly<Child>`) lists
//!   the inherited `base` first, as declared;
//! - `hashes.luau`: `Modding.Hash` is a fresh hash every clean build (TS5: a random uuid; here: derived from a
//!   per-build seed), and `flamework.build` has no `stringHashes` table (DESIGN §10.3) but keeps that seed as
//!   `buildSeed`, so that a build that reuses it derives the same hashes.

#![cfg(windows)]

mod support;

use flamework_rs::Flamework;
use support::fixture_macros::flamework_with_fixture_macros;
use support::project::{Project, assert_clean, compare_expected, diagnostics, errors, luau};

const FIXTURE: &str = "fixture_f2";

fn uuids_in(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    for start in 0..text.len().saturating_sub(36) {
        let candidate = &text[start..start + 36];
        if bytes[start] != b'"' && start > 0 && bytes[start - 1] == b'"' && flamework_rs::uuid::validate(candidate) {
            found.push(candidate.to_owned());
        }
    }
    found
}

/// The full-mode build equals `expected`: every Luau file (but the per-build hashes), `flamework.build` and the
/// include JSON.
#[test]
fn full_mode_matches_the_expected_outputs() {
    let Some(project) = Project::new(FIXTURE, "full") else { return };
    let report = project.build_with(flamework_with_fixture_macros());
    assert_clean(&report);
    let failures: Vec<String> = compare_expected(&project, &report, FIXTURE)
        .into_iter()
        .filter(|f| !f.starts_with("shared/hashes.luau") && !f.starts_with("flamework.build:"))
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));

    // `Modding.Hash`: two different uuids, derived from this build's seed, which `flamework.build` keeps as `buildSeed`
    // (before `metadata`) for the builds that reuse it; but for that line it is the expected file.
    let hashes = luau(&report, "shared/hashes.luau");
    let found = uuids_in(hashes);
    assert_eq!(found.len(), 2, "{hashes}");
    assert_ne!(found[0], found[1]);
    let build = project.read("flamework.build");
    let seed_line = build.lines().find(|l| l.starts_with("\t\"buildSeed\": ")).expect("the hash seed is kept");
    let seed = seed_line.trim_start_matches("\t\"buildSeed\": \"").trim_end_matches("\",");
    let seed = flamework_rs::uuid::Namespace::parse(seed).unwrap();
    assert_eq!(found, [seed.v5("@:playerJoined"), seed.v5("remotes:playerJoined")]);
    let expected = std::fs::read_to_string(support::project::fixture_dir(FIXTURE).join("expected/flamework.build"))
        .unwrap()
        .replace("\r\n", "\n");
    let without_seed: Vec<&str> = build.lines().filter(|l| *l != seed_line).collect();
    assert_eq!(without_seed.join("\n"), expected);
    assert!(build.find("\"buildSeed\"").unwrap() < build.find("\"metadata\"").unwrap());

    // Nothing is left to a later milestone in this fixture.
    let warnings = report.warnings.join("\n");
    assert!(!warnings.contains("need features rbxts-rs does not have yet"), "{warnings}");
}

/// `Modding.Generic`: ids, type text, dependency info (with a `Modding.Injectable`'s metadata), padding, unions,
/// `new`, methods and decorators.
#[test]
fn generic_markers() {
    let Some(project) = Project::new(FIXTURE, "generic") else { return };
    let report = project.build_with(flamework_with_fixture_macros());
    assert_clean(&report);
    let generic = luau(&report, "shared/generic.luau");
    for line in [
        "local economyId = idOf(\"shared/generic@Economy\")",
        "local shopText = textOf(\"Shop | undefined\")",
        "local economyConcise = conciseOf(\"shared/generic@Economy\")",
        "\tmetadata = { 33, \"logger\", \"shared/generic@Shop\" },",
        "local inlined = \"shared/generic@Bank\"",
        "local inlinedOwn = \"[Economy, string]\"",
        "local passedThrough = \"explicit\"",
        "\treturn Flamework._implements(value, \"shared/generic@Shop\")",
        "local explicit = either(\"mine\")",
        "local padded = second(nil, \"shared/generic@Economy\")",
        "local paddedUndefined = second(nil, \"shared/generic@Economy\")",
        "local registry = Registry.new():add(1, \"shared/generic@Economy\"):add(2, \"shared/generic@Shop\")",
        "local tagged = Tagged.new(\"shared/generic@Bank\")",
        "\tMarked = Mark(\"shared/generic@Economy\")(Marked) or Marked",
    ] {
        assert!(generic.contains(line), "missing {line:?} in\n{generic}");
    }
    // A provider's injected `Modding.Injectable` carries its metadata, made at the parameter.
    assert!(generic.contains("\t\t\tmetadata = { 24, \"logger\", \"shared/generic@Shop\" },"), "{generic}");
}

/// `Modding.Caller`: positions and widths in UTF-16, the source text, and callsite uuids as TS5 makes them.
#[test]
fn caller_markers() {
    let Some(project) = Project::new(FIXTURE, "caller") else { return };
    let report = project.build_with(flamework_with_fixture_macros());
    assert_clean(&report);
    let caller = luau(&report, "shared/caller.luau");
    // The key of the first: `<package>:<file id>@<declaration>+<offset>`, `atRoot = where()` has the call at +9.
    let uuid = flamework_rs::ids::callsite_uuid("f2-fixture", "src/shared/caller.ts", "atRoot", 9, None);
    assert!(caller.contains(&format!("local atRoot = where(4, 23, 7, \"{uuid}\", \"where()\")")), "{caller}");
    assert!(caller.contains("where(18, 26, 32, \"f4dcb6d8-18b4-594a-a04a-b259eb859214\", \"where(\\n\\tundefined,\\n\\tundefined,\\n)\")"), "{caller}");
    let unicode = luau(&report, "shared/unicode.luau");
    // "héllo 😀" is 8 UTF-16 units: the call after it starts at character 57 (one-based), not at its UTF-8 column.
    assert!(unicode.contains("local here = where(4, 57, 7, \"f0702dac-c6be-52c3-988f-4b7749644ed9\", \"where()\")"), "{unicode}");
    assert!(unicode.contains("\tlocal at = where(9, 49, 7, \"8e30b7d2-0b29-595f-93b5-9e89216edc07\", \"where()\")"), "{unicode}");
}

/// `Modding.Emit` / `Many`, `Labels` and `Caller.Constant`.
#[test]
fn emit_labels_and_constants() {
    let Some(project) = Project::new(FIXTURE, "emit") else { return };
    let report = project.build_with(flamework_with_fixture_macros());
    assert_clean(&report);
    let emit = luau(&report, "shared/emit.luau");
    for line in [
        "local tuple = emit({ \"a\", 1, true, false, nil })",
        "local empty = emit({})",
        "\t[10] = \"ten\",",
        "\t[\"two words\"] = 2,",
        "local labels = labelsOf({ \"name\", \"age\" })",
        "local parameterNames = labelsOf({ \"first\", \"second\", \"rest\" })",
        "local unlabelled = labelsOf({ \"\", \"\" })",
        "-- Flamework user macro metadata (line 25)",
        "\t-- Flamework hoisted this macro's metadata (constant_25) to the file root.",
        "local name = obfuscated(\"playerJoined\")",
        "local order = shuffled({ \"a\", \"b\", \"c\" })",
    ] {
        assert!(emit.contains(line), "missing {line:?} in\n{emit}");
    }
    // A literal is not hoisted.
    assert!(emit.contains("constant(\"plain\")"), "{emit}");
}

/// `path`, `pathglob` and `env`, with their records in `flamework.build`.
#[test]
fn path_glob_and_env_intrinsics() {
    let Some(project) = Project::new(FIXTURE, "intrinsics") else { return };
    let report = project.build_with(flamework_with_fixture_macros());
    assert_clean(&report);
    let intrinsics = luau(&report, "shared/intrinsics.luau");
    for line in [
        "local commands = pathOf(\"src/server/commands\", { \"ServerScriptService\", \"TS\", \"commands\" })",
        "local relativeGlob = globOf(\"../server/commands/*.ts\", \"src/server/commands/*.ts\")",
        "local channel = \"beta\"",
        "local region = \"eu\"",
        "local missing = nil",
    ] {
        assert!(intrinsics.contains(line), "missing {line:?} in\n{intrinsics}");
    }
    let build = project.read("flamework.build");
    assert!(build.contains("\t\t\"env\": {\n\t\t\t\"src/shared/intrinsics.ts\": {\n\t\t\t\t\"F2_FIXTURE_CHANNEL\": \"a295e0bdde1938d1fbfd343e5a3e569e868e1465\",\n\t\t\t\t\"F2_FIXTURE_REGION\": \"unset\",\n\t\t\t\t\"F2_FIXTURE_MISSING\": \"unset\"\n"), "{build}");
}

/// Macro types: results cached per file and hoisted when they are not literals, warnings and errors at the call,
/// and an id nothing registered.
#[test]
fn macro_types() {
    let Some(project) = Project::new(FIXTURE, "plugins") else { return };
    project.write(
        "src/shared/counter.ts",
        "import { Modding } from \"@flamework-experimental/core\";\n\n/** @metadata macro */\nexport function counter<T>(meta?: Modding.Intrinsic<\"plugin\", [\"counter\", T], (n: number) => number>) {\n\treturn meta!;\n}\n\nexport const increment = counter<\"x\">();\n",
    );
    let report = project.build_with(flamework_with_fixture_macros());
    assert_clean(&report);
    let intrinsics = luau(&report, "shared/intrinsics.luau");
    // One hoisted table, used three times.
    assert_eq!(intrinsics.matches("name = \"fxcoins\"").count(), 1, "{intrinsics}");
    assert!(intrinsics.contains("local fields = fieldInfo(fieldInfo_1)"), "{intrinsics}");
    assert!(intrinsics.contains("local fieldsAgain = fieldInfo(fieldInfo_1)"), "{intrinsics}");
    assert!(intrinsics.contains("\treturn fieldInfo(fieldInfo_1)"), "{intrinsics}");
    assert!(intrinsics.contains("\tkind = \"tuple[string,number]\","), "{intrinsics}");
    assert!(intrinsics.contains("local members = memberCount(3)"), "{intrinsics}");
    let counter = luau(&report, "shared/counter.luau");
    assert!(counter.contains("local counter_1 = function(increment)\n\treturn increment + 1\nend"), "{counter}");
    let all = diagnostics(&report);
    assert!(all.iter().any(|d| d == "warning counter.ts: counter macro used with \"x\""), "{all:#?}");

    // Without the macro type, or with a type the macro refuses.
    let project = Project::new(FIXTURE, "plugins-missing").expect("sources");
    project.write("src/shared/bad.ts", "import { memberCount } from \"./markers\";\n\nexport const one = memberCount<\"a\">();\n");
    let report = project.build_with(Flamework::new());
    let errors = errors(&report);
    assert!(
        errors.iter().any(|e| e.starts_with("error intrinsics.ts: The macro type 'fieldInfo' requires a plugin, but no macro types are registered.\nRegister it in the build's main.rs")),
        "{errors:#?}"
    );
    let report = project.build_with(flamework_with_fixture_macros());
    let errors = support::project::errors(&report);
    assert_eq!(errors, vec!["error bad.ts: mustBeUnion requires a union type".to_owned()]);
}

/// A unique-symbol key is left out where TS5 finds no type for it (`getTypeOfPropertyOfType(type, property.name)`):
/// a `Modding.Many` object map (`Modding.inspect`, TS5's bytes) and a macro type's `fields` (TS5's `getFields` skips
/// a field without a type). A user key that starts with U+007F but is no internal name (`"\u007f@x"`) stays.
#[test]
fn symbol_keys_are_left_out() {
    let Some(project) = Project::new(FIXTURE, "symbols") else { return };
    project.write(
        "src/shared/symbols.ts",
        "import { Modding } from \"@flamework-experimental/core\";\n\
         import { fieldInfo } from \"./markers\";\n\n\
         declare const sym: unique symbol;\n\
         interface Keyed {\n\ta: string;\n\t[sym]: number;\n\t\"\\u007f@x\": boolean;\n\tb: number;\n}\n\n\
         export const m1 = Modding.inspect<{ a: \"x\"; [sym]: \"y\"; b: 1 }>();\n\
         export const fields = fieldInfo<Keyed>();\n",
    );
    let report = project.build_with(flamework_with_fixture_macros());
    assert_clean(&report);
    let out = luau(&report, "shared/symbols.luau");
    assert!(out.contains("local m1 = Modding.inspect({\n\ta = \"x\",\n\tb = 1,\n})"), "{out}");
    assert!(!out.contains("@sym@"), "{out}");
    let names: Vec<&str> = out.lines().filter_map(|line| line.trim().strip_prefix("name = ")).collect();
    assert_eq!(names.len(), 3, "{out}");
    assert!(names[0] == "\"fxa\"," && names[1].ends_with("@x\",") && names[2] == "\"fxb\",", "{out}");
}

/// `intrinsic-const` refuses an argument that is not a literal, naming the parameter.
#[test]
fn const_parameters_need_literals() {
    let Some(project) = Project::new(FIXTURE, "const") else { return };
    project.write(
        "src/shared/bad.ts",
        "import { withOptions } from \"./markers\";\n\nconst options = { name: \"a\", size: 1 };\nexport const a = withOptions<string>(options);\nexport const b = withOptions<string>({ ...options });\n",
    );
    let report = project.build_with(flamework_with_fixture_macros());
    let errors = errors(&report);
    assert!(errors.iter().any(|e| e == "error bad.ts: Flamework expected this argument to be a literal expression."), "{errors:#?}");
    assert!(errors.iter().any(|e| e == "error bad.ts: Flamework does not support spread expressions in this location."), "{errors:#?}");
}

/// A parameter whose union members carry different macros cannot pick one: an error, where TS5 took the first.
#[test]
fn unions_of_different_macros_are_refused() {
    let Some(project) = Project::new(FIXTURE, "union") else { return };
    project.write(
        "src/shared/bad.ts",
        "import { Modding } from \"@flamework-experimental/core\";\n\n/** @metadata macro */\nexport function either<T>(value?: Modding.Target.Id<T> | Modding.Caller.Line) {\n\treturn value;\n}\n\nexport const a = either<string>();\n",
    );
    let report = project.build_with(flamework_with_fixture_macros());
    let errors = errors(&report);
    assert_eq!(errors.len(), 1, "{errors:#?}");
    assert!(errors[0].contains("carry different Flamework macros"), "{errors:#?}");
}

/// The later milestones' intrinsics are all built, none left to a later milestone: a network decoder (F5), a
/// serializer (F4) and a guard (F3).
#[test]
fn later_milestones_are_built() {
    let Some(project) = Project::new(FIXTURE, "later") else { return };
    project.write(
        "src/shared/later.ts",
        "import { Modding } from \"@flamework-experimental/core\";\n\n/** @metadata macro */\nexport function decoder<T>(meta?: Modding.Intrinsic<\"network-decoder\", [T]>) {\n\treturn meta;\n}\n\nexport const d = decoder<[number]>();\n",
    );
    let report = project.build_with(flamework_with_fixture_macros());
    assert_clean(&report);
    assert!(report.warnings.iter().all(|w| !w.contains("need features")), "{:?}", report.warnings);
    let decoder = luau(&report, "shared/later.luau");
    // Packed only under `networking.serialization`, which this fixture leaves off: nil, as TS5 builds it.
    assert!(decoder.contains("local d = decoder(nil)"), "{decoder}");

    project.write(
        "src/shared/serialized.ts",
        "import { Flamework } from \"@flamework-experimental/core\";\n\nexport const serializer = Flamework.createSerializer<{ a: number }>();\n",
    );
    project.write(
        "src/shared/guarded.ts",
        "import { Flamework } from \"@flamework-experimental/core\";\n\nexport const guard = Flamework.createGuard<string>();\n",
    );
    let report = project.build_with(flamework_with_fixture_macros());
    assert_clean(&report);
    let guarded = luau(&report, "shared/guarded.luau");
    assert!(guarded.contains("Flamework.createGuard(t.string)"), "{guarded}");
    let serialized = luau(&report, "shared/serialized.luau");
    assert!(serialized.contains("buffer.writef64(buf, 0, v.a)"), "{serialized}");
    assert!(report.warnings.iter().all(|w| !w.contains("need features")), "{:?}", report.warnings);
}

/// `tuple-guards` (F3's `tuple_guards`): `{ { guards }, rest or nil, { after }? }`, as TS5 builds it.
#[test]
fn tuple_guards_intrinsic() {
    let Some(project) = Project::new(FIXTURE, "tupleguards") else { return };
    project.write(
        "src/shared/tupleGuards.ts",
        "import { Modding } from \"@flamework-experimental/core\";\n\n/** @metadata macro */\nexport function tupleGuards<T extends unknown[]>(meta?: Modding.Intrinsic<\"tuple-guards\", [T], unknown>) {\n\treturn meta;\n}\n\nexport const labeled = tupleGuards<[id: number, name?: string]>();\nexport const after = tupleGuards<[number, ...string[], boolean]>();\nexport const array = tupleGuards<string[]>();\n",
    );
    let report = project.build_with(flamework_with_fixture_macros());
    assert_clean(&report);
    let luau = luau(&report, "shared/tupleGuards.luau");
    for line in [
        "local labeled = tupleGuards({ { t.number, t.optional(t.string) }, nil })",
        "local after = tupleGuards({ { t.number }, t.string, { t.boolean } })",
        "local array = tupleGuards({ {}, t.string })",
    ] {
        assert!(luau.contains(line), "{line}\n{luau}");
    }
}
