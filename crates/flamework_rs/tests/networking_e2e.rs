//! Networking (F5) end to end, through the real compiler on `tests/fixture_f5`: the packed call sites of every shape
//! (`fire`, `except`, `broadcast`, `invoke`, `invokeWithTimeout`, `setCallback` and the member called itself), every
//! mode (packed by `networking.serialization`, `Unreliable`, `Raw`, `Serialized`), the declarations' decoders and
//! middleware, arguments read where they are or bound, spreads, every place a call can stand (ahead of its statement,
//! in a function, in a `do` block past a function's or the file's budget of locals, including a function or file so
//! full of locals already that the packing is built as first built, `wide.ts` and `wideTop.ts`), nested sends, sends
//! in class members (`members.ts`, which roblox-ts does not compile in source order) and the client and server sides
//! (`crate::networking`).
//!
//! `tests/fixture_f5/expected` holds rbxtsc-fw's output for every file but one, each compared whole
//! (`UPDATE_EXPECTED=1` writes it). It is the TS5 pipeline's output (roblox-ts master with the published 2.0.0-beta.0
//! transformer) but for the generated locals' names (checked with `tools/normalize.cjs`; `chained.ts`: sends in an
//! optional chain, in a function that checks the chain's start, as TS5). The one differs from TS5's in shape only
//! (README, Known limitations) and is checked on its own: `scopedReturn.ts` has a returned send past its function's
//! budget of locals, which flamework-rs packs in a function where TS5 puts the statement in a block.
//!
//! The project is copied to a temporary directory with a `node_modules` as an install has it (`support::project`);
//! without the monorepo and the `@rbxts` packages the tests are skipped, saying so.

#![cfg(windows)]

mod support;

use flamework_rs::Flamework;
use support::project::{Project, assert_clean, errors, fixture_dir, luau};

const FIXTURE: &str = "fixture_f5";

/// Every file builds, and the files in `expected` equal its output byte for byte.
#[test]
fn every_call_shape_builds_and_matches_ts5() {
    let Some(project) = Project::new(FIXTURE, "shapes") else { return };
    let report = project.build_with(Flamework::new());
    assert_clean(&report);

    let expected = fixture_dir(FIXTURE).join("expected");
    let mut failures = Vec::new();
    let mut compared = 0;
    for entry in std::fs::read_dir(&expected).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().into_owned();
        compared += 1;
        failures.extend(support::expected::check(&name, &entry.path(), luau(&report, &name)));
    }
    assert!(compared >= 6, "expected outputs missing ({compared})");
    assert!(failures.is_empty(), "{}", failures.join("\n"));

    // A returned send past its function's budget of locals: in a function of its own, where TS5 puts the return
    // statement in a `do` block with the packing. The sends before it are in `do` blocks, as TS5's.
    let scoped = luau(&report, "scopedReturn.luau");
    assert!(scoped.contains("\tdo\n"), "{scoped}");
    assert!(scoped.contains("\treturn (function()\n\t\tlocal arg_40 = compute()"), "{scoped}");
    assert!(scoped.contains("\t\treturn Events.echo:_fire(buf_22)\n\tend)()\nend"), "{scoped}");
}

const NETWORK: &str = "import { Networking } from \"@flamework-experimental/networking\";\n";

/// The build errors of a call site, with the TS transformer's texts: a target that may be members packed differently,
/// and a member marked both Raw and Serialized.
#[test]
fn errors_name_the_call() {
    let Some(project) = Project::new(FIXTURE, "errors") else { return };
    std::fs::remove_dir_all(project.root.join("src")).unwrap();
    project.write(
        "src/errors.ts",
        &format!(
            "{NETWORK}\
             interface Events {{\n\
             \tpacked(value: number): void;\n\
             \traw: Networking.Raw<(value: number) => void>;\n\
             \tboth: Networking.Raw<Networking.Serialized<(value: number) => void>>;\n\
             }}\n\
             const events = Networking.createEvent<{{}}, Events>().createServer({{}});\n\
             declare const name: \"packed\" | \"raw\";\n\
             export function send(player: Player) {{\n\
             \tevents[name].fire(player, 1);\n\
             \tevents.both.fire(player, 2);\n\
             }}\n"
        ),
    );
    let report = project.build_with(Flamework::new());
    let errors = errors(&report);
    let has = |text: &str| errors.iter().any(|e| e.contains(text));
    // TS5's three errors, word for word: the declaration's, the mixed target's and the call's.
    assert_eq!(errors.len(), 3, "{errors:#?}");
    let both = " is declared both Raw and Serialized. Raw sends its values as they are; Serialized packs them into a buffer.\nKeep one of the two markers.";
    assert!(has(&format!("The networking member 'both'{both}")), "{errors:#?}");
    assert!(has(&format!("The networking member 'events.both'{both}")), "{errors:#?}");
    assert!(
        has("The call 'events[name].fire(...)' may reach networking members that are packed differently: plain members \
             are packed, as networking.serialization is on; Raw members are sent as they are.\nOne call site packs for all \
             of them or for none, so some would drop what it sends as malformed.\nMake the call where the member's own type \
             is known, such as in each case of a switch over the name, or declare these members the same way."),
        "{errors:#?}"
    );
}

/// `explainRemovedNetworkMarker`: TypeScript's error for a marker removed before beta is explained, naming the type
/// that replaces it.
#[test]
fn a_removed_marker_is_explained() {
    let Some(project) = Project::new(FIXTURE, "removed") else { return };
    std::fs::remove_dir_all(project.root.join("src")).unwrap();
    project.write(
        "src/removed.ts",
        &format!(
            "{NETWORK}\
             export type Old = Networking.SerializedReliable<(value: number) => void>;\n\
             export type Raw = Networking.RawReliable<(value: number) => void>;\n"
        ),
    );
    let report = project.build_with(Flamework::new());
    let errors = errors(&report);
    let has = |text: &str| errors.iter().any(|e| e.contains(text));
    assert!(
        has("'Networking.SerializedReliable' was removed before beta. It was the same type as 'Networking.Serialized'"),
        "{errors:#?}"
    );
    assert!(has("Write 'Networking.Raw<...>'. 'RawUnreliable' would make the event unreliable."), "{errors:#?}");
}

/// `explainRenamed`: TypeScript's error for a name renamed or removed before beta names the replacement, where
/// Flamework's package declares it; a user's own name that matches keeps TypeScript's message. With
/// `noSemanticDiagnostics`, nothing is explained (TS5 output of the same source: fwapi round, `v2/ren-src`).
#[test]
fn a_renamed_name_is_explained() {
    let Some(project) = Project::new(FIXTURE, "renamed") else { return };
    std::fs::remove_dir_all(project.root.join("src")).unwrap();
    project.write(
        "src/renamed.ts",
        "import { BaseComponent, Component, ComponentStreamingMode } from \"@flamework-experimental/components\";\n\
         export const mode = ComponentStreamingMode.Default;\n\
         @Component({ tag: \"Ren\", ancestorWhitelist: [] })\n\
         export class Ren extends BaseComponent {}\n\
         export const y = ({ x: 1 } as { x: number }).onPostIgnite;\n",
    );
    let report = project.build_with(Flamework::new());
    let errors = errors(&report);
    let has = |text: &str| errors.iter().any(|e| e.contains(text));
    assert_eq!(errors.len(), 3, "{errors:#?}");
    assert!(
        has("'ComponentStreamingMode.Default' was removed before beta. It stood for \
             'ComponentStreamingMode.Contextual', which replaces it.\nWrite 'Contextual'."),
        "{errors:#?}"
    );
    assert!(
        has("'ancestorWhitelist' was renamed 'ancestorAllowList' before beta.\nWrite 'ancestorAllowList'."),
        "{errors:#?}"
    );
    assert!(has("Property 'onPostIgnite' does not exist on type '{ x: number; }'."), "{errors:#?}");
}

/// Middleware has to be an object literal the call site can rename: written as a variable, or as a shorthand property
/// (where the TS transformer fails an assertion instead).
#[test]
fn middleware_is_an_object() {
    let Some(project) = Project::new(FIXTURE, "middleware") else { return };
    std::fs::remove_dir_all(project.root.join("src")).unwrap();
    project.write(
        "flamework.config.json",
        "{\n\t\"transformer\": { \"hashPrefix\": \"$t\", \"obfuscation\": true },\n\t\"networking\": { \"serialization\": true }\n}\n",
    );
    project.write(
        "src/middleware.ts",
        &format!(
            "{NETWORK}\
             interface Events {{\n\talpha(value: number): void;\n}}\n\
             declare const middleware: {{ alpha?: Networking.EventMiddleware<[number]>[] }};\n\
             export const events = Networking.createEvent<{{}}, Events>().createServer({{ middleware: middleware }});\n\
             export const shorthand = Networking.createEvent<{{}}, Events>().createServer({{ middleware }});\n\
             export const getter = Networking.createEvent<{{}}, Events>().createServer({{ get middleware() {{ return {{}}; }} }});\n"
        ),
    );
    let report = project.build_with(Flamework::new());
    let errors = errors(&report);
    let refused = errors.iter().filter(|e| e.contains("Networking middleware must be an object.")).count();
    let accessor = errors.iter().filter(|e| e.contains("not as a method or an accessor")).count();
    assert_eq!((refused, accessor, errors.len()), (2, 1, 3), "{errors:#?}");
}

/// Under obfuscation, a member reached through an optional chain is obfuscated as it is outside a chain (a deliberate
/// difference from the TS transformer, which leaves its name as written, so that the send reaches no remote): the
/// file's sends name no member as written, through `?.` on a nullable value, a call's result, or a nested namespace,
/// and the chain's key is the plain send's.
#[test]
fn obfuscated_keys_through_an_optional_chain() {
    let Some(project) = Project::new(FIXTURE, "obfuscated-chain") else { return };
    std::fs::remove_dir_all(project.root.join("src")).unwrap();
    project.write(
        "flamework.config.json",
        "{\n\t\"transformer\": { \"hashPrefix\": \"$t\", \"obfuscation\": true },\n\t\"networking\": { \"serialization\": true }\n}\n",
    );
    project.write(
        "src/net.ts",
        &format!(
            "{NETWORK}\
             interface Events {{\n\tzulu(value: number): void;\n\tns: {{ deeper: {{ leaf(text: string): void }} }};\n}}\n\
             interface Functions {{\n\tadd(a: number, b: number): number;\n}}\n\
             export const events = Networking.createEvent<Events, {{}}>().createClient({{}});\n\
             export const functions = Networking.createFunction<Functions, {{}}>().createClient({{}});\n"
        ),
    );
    project.write(
        "src/send.ts",
        "import { events, functions } from \"./net\";\n\
         declare function pick(): typeof events | undefined;\n\
         export function send(maybe: typeof events | undefined, flag: boolean) {\n\
         \tevents.zulu.fire(1);\n\
         \tmaybe?.zulu.fire(2);\n\
         \tmaybe?.zulu(3);\n\
         \tpick()?.ns.deeper.leaf.fire(\"x\");\n\
         \treturn (flag ? functions : undefined)?.add.invoke(1, 2);\n\
         }\n",
    );
    let report = project.build_with(Flamework::new());
    assert_clean(&report);
    let out = luau(&report, "send.luau");
    for name in ["zulu", "ns", "deeper", "leaf", "add"] {
        assert!(!out.contains(&format!(".{name}")), "{name} as written: {out}");
    }
    // `events["<key>"]:_fire(` and `_result = _result["<key>"]:_fire(`: the same key
    let key = |line: &str, from: &str| {
        let rest = &line[line.find(from)? + from.len()..];
        Some(rest[..rest.find("\"]")?].to_owned())
    };
    let plain = out.lines().find_map(|l| key(l, "events[\""));
    let chained = out.lines().find_map(|l| key(l, "_result = _result[\""));
    assert!(plain.is_some() && plain == chained, "{plain:?} {chained:?}: {out}");
}
