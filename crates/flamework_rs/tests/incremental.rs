//! Incremental builds and watch rebuilds reuse the last committed `flamework.build` (DESIGN §14; `setup`'s module
//! docs), through the real compiler's `BuildSession` on `tests/fixture`:
//! - a build in a new process keeps the salt, the seeds and the ids, and transforms only what changed (the W1 harness
//!   runs are in `incremental_harness.rs`, which pins the seed);
//! - `Modding.Hash` literals no longer mix across processes (the `hashprobe.cjs` scenario of the sfile validation);
//! - the project's `flamework.build` deleted, edited, made invalid or written by another version: the TS texts,
//!   naming rbxtsc-fw's incremental state, and following them works (changed inputs, which TS refuses, start a new
//!   `flamework.build`: `incremental_harness.rs`, which pins the seed to compare with a clean build);
//! - 1 and 4 checkers give the same bytes.
//!
//! The fixture is copied with a `node_modules` as `tests/e2e.rs` makes it (`support::project`); without the monorepo
//! and the `@rbxts` packages the tests are skipped, saying so.

#![cfg(windows)]

mod support;

use std::collections::BTreeMap;
use std::path::Path;

use flamework_rs::Flamework;
use flamework_rs::json::{self, JsValue};
use rbxts_plugin::BuildKind;
use rbxts_plugin::erased::{DynPlugin, erase};
use rbxts_project::{ProjectOptionLayer, STACK_SIZE};
use rbxts_transform::{BuildSession, BuildSummary, CompileOptions, CompileReport, PluginOptions};
use support::project::{Project, errors};

/// rbxtsc-fw's incremental state of the fixture once it is incremental, the file the remedies name (`rootDir` `src`
/// and `outDir` `out` put it beside the tsconfig).
const STATE_FILE: &str = "tsconfig.rbxts-rs.buildinfo";

/// Runs `f` on a thread with the compiler's stack.
fn big_stack<T: Send>(f: impl FnOnce() -> T + Send) -> T {
    std::thread::scope(|scope| {
        std::thread::Builder::new().stack_size(STACK_SIZE).spawn_scoped(scope, f).unwrap().join().unwrap()
    })
}

fn compile_options(root: &Path, checkers: Option<usize>) -> CompileOptions {
    CompileOptions {
        tsconfig: "tsconfig.json".into(),
        cwd: root.to_string_lossy().into_owned(),
        project_options: ProjectOptionLayer {
            optimized_loops: Some(true),
            allow_comment_directives: Some(true),
            ..ProjectOptionLayer::default()
        },
        out_dir_override: None,
        write_partial: false,
        no_write: false,
        checkers,
    }
}

/// What a build did, as text (the report's diagnostics live only as long as its session).
#[derive(Debug)]
struct Outcome {
    /// Errors and fatal messages.
    errors: Vec<String>,
    fatal: Vec<String>,
    warnings: Vec<String>,
    /// The files transformed, relative to `src`.
    transformed: Vec<String>,
    summary: BuildSummary,
}

impl Outcome {
    fn of(report: &CompileReport, summary: BuildSummary) -> Outcome {
        let mut transformed: Vec<String> = report
            .files
            .iter()
            .map(|f| {
                let name = f.file_name.replace('\\', "/");
                name.rsplit_once("/src/").map_or(name.clone(), |(_, rest)| rest.to_owned())
            })
            .collect();
        transformed.sort();
        Outcome {
            errors: errors(report),
            fatal: report.fatal.clone(),
            warnings: report.warnings.clone(),
            transformed,
            summary,
        }
    }

    fn assert_ok(&self) -> &Outcome {
        assert!(self.errors.is_empty(), "{:#?}", self.errors);
        self
    }

    fn kind(&self) -> BuildKind {
        self.summary.kind.expect("a build kind")
    }

    /// The one fatal message, which must be Flamework's.
    fn flamework_fatal(&self) -> &str {
        assert_eq!(self.fatal.len(), 1, "{:#?}", self.errors);
        self.fatal[0].strip_prefix("plugin `flamework`: ").unwrap_or_else(|| panic!("{}", self.fatal[0]))
    }
}

/// One build in a new session: a new `rbxtsc-fw build` process (incremental once the tsconfig says so and its state
/// is there).
fn build_once(root: &Path, checkers: Option<usize>) -> Outcome {
    let root = root.to_owned();
    big_stack(move || {
        let plugins: Vec<Box<dyn DynPlugin>> = vec![erase(Flamework::new())];
        let options = PluginOptions { plugins: &plugins, ..PluginOptions::default() };
        let (report, summary) = BuildSession::new(compile_options(&root, checkers), &options, false).build(&[]);
        Outcome::of(&report, summary)
    })
}

/// An edit of the watched tree: what to do, returning the paths it touched.
type Edit = Box<dyn FnOnce() -> Vec<String> + Send>;

/// A watch session: the initial build, then one rebuild per edit.
fn watch(root: &Path, checkers: Option<usize>, edits: Vec<Edit>) -> Vec<Outcome> {
    let root = root.to_owned();
    big_stack(move || {
        let plugins: Vec<Box<dyn DynPlugin>> = vec![erase(Flamework::new())];
        let options = PluginOptions { plugins: &plugins, ..PluginOptions::default() };
        let mut session = BuildSession::new(compile_options(&root, checkers), &options, true);
        let (report, summary) = session.build(&[]);
        let mut outcomes = vec![Outcome::of(&report, summary)];
        for edit in edits {
            let touched = edit();
            let (report, summary) = session.build(&touched);
            outcomes.push(Outcome::of(&report, summary));
        }
        outcomes
    })
}

/// Writes `text` into the project's `relative`, returning the touched path (for [`watch`]).
fn write(root: &Path, relative: &str, text: &str) -> Vec<String> {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, text).unwrap();
    vec![path.to_string_lossy().replace('\\', "/")]
}

/// The fixture as an incremental project.
fn incremental(project: &Project) {
    let tsconfig = project.read("tsconfig.json");
    assert!(tsconfig.contains("\"incremental\": false"));
    project.write("tsconfig.json", &tsconfig.replace("\"incremental\": false", "\"incremental\": true"));
}

/// Every file a build writes (`out`, `include`, `flamework.build`), without the incremental state.
fn snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.ends_with(".rbxts-rs.buildinfo") {
                continue;
            }
            if entry.file_type().unwrap().is_dir() {
                walk(root, &path, out);
            } else {
                let relative = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
                out.insert(relative, std::fs::read(&path).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, &root.join("out"), &mut out);
    walk(root, &root.join("include"), &mut out);
    if let Ok(bytes) = std::fs::read(root.join("flamework.build")) {
        out.insert("flamework.build".into(), bytes);
    }
    out
}

/// The files of `a` and `b` that differ, are missing from one, or (with `only`) are not in `only`.
fn differences(a: &BTreeMap<String, Vec<u8>>, b: &BTreeMap<String, Vec<u8>>) -> Vec<String> {
    let mut out = Vec::new();
    for (path, bytes) in a {
        match b.get(path) {
            Some(other) if other == bytes => {}
            Some(_) => out.push(format!("{path}: differs")),
            None => out.push(format!("{path}: only in the first")),
        }
    }
    out.extend(b.keys().filter(|p| !a.contains_key(*p)).map(|p| format!("{p}: only in the second")));
    out
}

/// [`snapshot`] without `flamework.build`.
fn outputs(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut tree = snapshot(root);
    tree.remove("flamework.build");
    tree
}

fn build_info(project: &Project) -> JsValue {
    json::parse(&project.read("flamework.build")).unwrap()
}

fn field<'a>(info: &'a JsValue, key: &str) -> Option<&'a str> {
    info.get(key).and_then(JsValue::as_str)
}

/// `Modding.Hash` in two files: the macro and a use in `hashA.ts`, a use in `hashB.ts`.
const HASH_A: &str = "import { Modding } from \"@flamework-experimental/core\";\n\n/** @metadata macro */\nexport function probeHash(h?: Modding.Target.Hash<\"s2Probe\">) {\n\treturn h;\n}\n\nexport const a = probeHash();\n";
const HASH_B: &str = "import { probeHash } from \"./hashA\";\n\nexport const b = probeHash();\n";

/// The hash literal in an output.
fn hash_literal(project: &Project, output: &str) -> String {
    let text = project.read(output);
    let start = text.find("probeHash(\"").unwrap_or_else(|| panic!("{text}")) + "probeHash(\"".len();
    text[start..start + text[start..].find('"').unwrap()].to_owned()
}

/// A second process with nothing changed transforms nothing and writes the same bytes; an edit transforms that file
/// only, under the same salt and ids (short mode, whose ids hash a counter with the salt).
#[test]
fn a_new_process_reuses_the_build_info() {
    let Some(project) = Project::new("fixture", "s2-reuse") else { return };
    incremental(&project);
    project.write(".env", "F1_IDMODE=short\n");

    let first = build_once(&project.root, None);
    first.assert_ok();
    assert_eq!(first.kind(), BuildKind::Clean);
    let after_first = snapshot(&project.root);
    let info = build_info(&project);
    let salt = field(&info, "salt").expect("a salt in short mode").to_owned();

    let second = build_once(&project.root, None);
    second.assert_ok();
    assert_eq!(second.kind(), BuildKind::Incremental);
    assert_eq!(second.transformed, Vec::<String>::new());
    assert_eq!(differences(&after_first, &snapshot(&project.root)), Vec::<String>::new());

    let alpha = "src/server/services/alpha.ts";
    project.write(alpha, &project.read(alpha).replace("onInit() {}", "onInit() {\n\t\tprint(\"edited\");\n\t}"));
    let third = build_once(&project.root, None);
    third.assert_ok();
    assert_eq!(third.kind(), BuildKind::Incremental);
    assert_eq!(third.transformed, ["server/services/alpha.ts"]);
    let after_third = snapshot(&project.root);
    let changed: Vec<String> = differences(&after_first, &after_third);
    assert_eq!(changed, ["out/server/services/alpha.luau: differs"]);
    let again = build_info(&project);
    assert_eq!(field(&again, "salt"), Some(salt.as_str()));
    assert_eq!(again.get("identifiers"), info.get("identifiers"));
    let identifiers = info.get("identifiers").and_then(JsValue::as_object).unwrap();
    let alpha_id = identifiers.iter().find(|(k, _)| k.ends_with("@Alpha")).unwrap().1.as_str().unwrap();
    assert!(project.read("out/server/services/alpha.luau").contains(&format!("\"identifier\", \"{alpha_id}\"")));
}

/// The sfile validation's probe (`hashprobe.cjs`): two files share one `Modding.Hash` macro; once one of them is edited
/// in a later process, both still hold the same literal, which `flamework.build` keeps the seed of.
#[test]
fn modding_hash_literals_do_not_mix_across_processes() {
    let Some(project) = Project::new("fixture", "s2-hash") else { return };
    incremental(&project);
    project.write("src/shared/hashA.ts", HASH_A);
    project.write("src/shared/hashB.ts", HASH_B);

    build_once(&project.root, None).assert_ok();
    let literal = hash_literal(&project, "out/shared/hashA.luau");
    assert_eq!(hash_literal(&project, "out/shared/hashB.luau"), literal);
    let info = build_info(&project);
    let seed = field(&info, "buildSeed").expect("the hash seed is kept");
    assert_eq!(literal, flamework_rs::uuid::Namespace::parse(seed).unwrap().v5("@:s2Probe"));
    // Kept where a fresh build info would have it: after the id settings, before `metadata`.
    let keys: Vec<String> = info.as_object().unwrap().keys().cloned().collect();
    assert_eq!(keys, ["version", "flameworkVersion", "identifiers", "inputs", "idGenerationMode", "buildSeed", "metadata"]);

    for _ in 0..2 {
        let unchanged = build_once(&project.root, None);
        unchanged.assert_ok();
        assert_eq!(unchanged.transformed, Vec::<String>::new());
    }
    project.write("src/shared/hashB.ts", &format!("{HASH_B}\n// edit\n"));
    let edited = build_once(&project.root, None);
    edited.assert_ok();
    assert_eq!(edited.transformed, ["shared/hashB.ts"]);
    assert_eq!(hash_literal(&project, "out/shared/hashA.luau"), literal);
    assert_eq!(hash_literal(&project, "out/shared/hashB.luau"), literal);
}

/// The project's `flamework.build` deleted, edited, cut short or written by another version before an incremental
/// build: the compiler transforms every file then (its state file changed); a deleted one is made again, an edited
/// one is read as the TS transformer reads it (an edited id is used), and one that cannot be used stops the build with
/// the TS texts, naming rbxtsc-fw's incremental state, whose deletion makes the next build a clean one.
#[test]
fn the_build_info_changed_by_hand_before_an_incremental_build() {
    let Some(project) = Project::new("fixture", "s2-hand-inc") else { return };
    incremental(&project);
    build_once(&project.root, None).assert_ok();
    let built = snapshot(&project.root);
    let original = project.read("flamework.build");
    assert!(project.exists(STATE_FILE), "the remedies name a file that is there");

    // Deleted: made again, as a clean build makes it (full mode: the same bytes).
    std::fs::remove_file(project.root.join("flamework.build")).unwrap();
    let rebuilt = build_once(&project.root, None);
    rebuilt.assert_ok();
    assert!(rebuilt.summary.all);
    assert_eq!(rebuilt.summary.lost_state_files.len(), 1, "{:?}", rebuilt.summary);
    assert_eq!(differences(&built, &snapshot(&project.root)), Vec::<String>::new());

    // Edited: read, its id used.
    let id = "\"f1-fixture:out/server/services/alpha@Alpha\": \"server/services/alpha@Alpha\"";
    assert!(original.contains(id), "{original}");
    project.write("flamework.build", &original.replace(id, "\"f1-fixture:out/server/services/alpha@Alpha\": \"edited@Alpha\""));
    let edited = build_once(&project.root, None);
    edited.assert_ok();
    assert!(edited.summary.all);
    assert!(project.read("out/server/services/alpha.luau").contains("\"identifier\", \"edited@Alpha\""));
    project.write("flamework.build", &original);
    build_once(&project.root, None).assert_ok();
    assert_eq!(differences(&built, &snapshot(&project.root)), Vec::<String>::new());

    // Cut short: refused, nothing written; the remedy (delete the incremental state) gives a clean build.
    project.write("flamework.build", &original[..original.len() / 2]);
    let before = snapshot(&project.root);
    let cut = build_once(&project.root, None);
    let message = cut.flamework_fatal();
    assert!(message.starts_with("Flamework cannot use flamework.build: it is not valid JSON ("), "{message}");
    assert!(message.ends_with(&format!(").\nThis incremental build reads it to keep the ids of the files it does not recompile. Delete {STATE_FILE} and build again: a full build does not read flamework.build, and writes a new one.")), "{message}");
    assert_eq!(differences(&before, &snapshot(&project.root)), Vec::<String>::new(), "nothing written");
    std::fs::remove_file(project.root.join(STATE_FILE)).unwrap();
    let followed = build_once(&project.root, None);
    followed.assert_ok();
    assert_eq!(followed.kind(), BuildKind::Clean);
    assert_eq!(differences(&built, &snapshot(&project.root)), Vec::<String>::new());

    // Another version: refused; the remedy works too.
    project.write("flamework.build", &original.replace(flamework_rs::VERSION, "2.0.0-alpha.1"));
    let before = snapshot(&project.root);
    let other = build_once(&project.root, None);
    assert_eq!(
        other.flamework_fatal(),
        format!("Project was compiled on different version of Flamework.\nThis is an incremental build, which recompiles only the files that changed. Delete {STATE_FILE} and build again: the next build compiles every file.\nCurrent Flamework Version: {}\nPrevious Flamework Version: 2.0.0-alpha.1", flamework_rs::VERSION)
    );
    assert_eq!(differences(&before, &snapshot(&project.root)), Vec::<String>::new(), "nothing written");
    std::fs::remove_file(project.root.join(STATE_FILE)).unwrap();
    build_once(&project.root, None).assert_ok();
    assert_eq!(differences(&built, &snapshot(&project.root)), Vec::<String>::new());
}

/// The same in a watch session: the watcher reads the file again once it changed (the compiler hands a fresh state);
/// a refused rebuild writes nothing.
#[test]
fn the_build_info_changed_by_hand_under_a_watcher() {
    let Some(project) = Project::new("fixture", "s2-hand-watch") else { return };
    let root = project.root.clone();
    let path = root.join("flamework.build");
    // Before each edit: the file, and every output but it.
    type Seen = (String, BTreeMap<String, Vec<u8>>);
    let seen: std::sync::Arc<std::sync::Mutex<Vec<Seen>>> = Default::default();
    let rewrite = |change: fn(&str) -> Option<String>| -> Edit {
        let (root, path, seen) = (root.clone(), path.clone(), seen.clone());
        Box::new(move || {
            let text = std::fs::read_to_string(&path).unwrap();
            seen.lock().unwrap().push((text.clone(), outputs(&root)));
            match change(&text) {
                Some(text) => std::fs::write(&path, text).unwrap(),
                None => std::fs::remove_file(&path).unwrap(),
            }
            vec![path.to_string_lossy().replace('\\', "/")]
        })
    };
    let outcomes = watch(
        &root,
        None,
        vec![
            rewrite(|_| None),
            rewrite(|text| Some(text[..text.len() / 2].to_owned())),
            rewrite(|text| Some(text.to_owned())),
        ],
    );
    let mut seen = seen.lock().unwrap().clone();
    seen.push((std::fs::read_to_string(&path).unwrap(), outputs(&root)));
    outcomes[0].assert_ok();
    // Deleted: made again (full mode: as it was).
    outcomes[1].assert_ok();
    assert!(outcomes[1].summary.all);
    assert_eq!(seen[1].0, seen[0].0);
    // Cut short: the watcher's remedy, and nothing written.
    let message = outcomes[2].flamework_fatal();
    assert!(message.starts_with("Flamework cannot use flamework.build: it is not valid JSON ("), "{message}");
    assert!(message.ends_with(").\nThis watcher reads it on every rebuild, to keep the ids of the files it does not recompile. Restart the watcher: a full build does not read flamework.build, and writes a new one."), "{message}");
    assert_eq!(seen[2].0, seen[1].0[..seen[1].0.len() / 2], "the cut file is left as it is");
    assert_eq!(differences(&seen[1].1, &seen[2].1), Vec::<String>::new(), "nothing written");
    // Still cut short (the failed build wrote nothing): refused again, nothing written.
    assert!(outcomes[3].flamework_fatal().starts_with("Flamework cannot use flamework.build"));
    assert_eq!(seen[3].0, seen[2].0);
    assert_eq!(differences(&seen[2].1, &seen[3].1), Vec::<String>::new(), "nothing written");
}

/// Under obfuscation an incremental build keeps the ids and says so, as the TS transformer does, naming the file to
/// delete before a release build; deleting it rotates every id.
#[test]
fn an_incremental_obfuscated_build_says_it_kept_the_ids() {
    let Some(project) = Project::new("fixture", "s2-obf") else { return };
    incremental(&project);
    project.write(".env", "F1_OBFUSCATE=true\nF1_IDMODE=obfuscated\n");
    let first = build_once(&project.root, None);
    first.assert_ok();
    let built = snapshot(&project.root);
    let second = build_once(&project.root, None);
    second.assert_ok();
    let kept = format!(
        "flamework: obfuscated ids were kept from the previous build\nThis is an incremental build, so files that do not recompile keep their ids and the rest have to match.\nDelete {STATE_FILE} before a release build to rotate every id."
    );
    let warnings = |outcome: &Outcome| -> Vec<String> {
        outcome.warnings.iter().filter(|w| w.contains("obfuscated ids")).cloned().collect()
    };
    assert_eq!(warnings(&first), Vec::<String>::new());
    assert_eq!(warnings(&second), [kept]);
    assert_eq!(differences(&built, &snapshot(&project.root)), Vec::<String>::new());

    // The release advice: with the incremental state deleted, the next build is a clean one with new ids.
    std::fs::remove_file(project.root.join(STATE_FILE)).unwrap();
    let release = build_once(&project.root, None);
    release.assert_ok();
    assert_eq!(release.kind(), BuildKind::Clean);
    assert_eq!(warnings(&release), Vec::<String>::new());
    let alpha = "out/server/services/alpha.luau";
    assert_ne!(built.get(alpha).unwrap(), &std::fs::read(project.root.join(alpha)).unwrap(), "the ids rotated");
}

/// 1 and 4 checkers: the same bytes for a clean build seeded with the same state, its watch rebuild after an edit, and
/// a later incremental build after another edit.
#[test]
fn one_and_four_checkers_give_the_same_bytes() {
    let Some(one) = Project::new("fixture", "s2-checkers-1") else { return };
    let Some(four) = Project::new("fixture", "s2-checkers-4") else { return };
    for project in [&one, &four] {
        incremental(project);
        project.write(".env", "F1_IDMODE=short\n");
        project.write("src/shared/hashA.ts", HASH_A);
        project.write("src/shared/hashB.ts", HASH_B);
    }
    let alpha = "src/server/services/alpha.ts";
    let edit = |project: &Project, from: &str, to: &str| {
        let text = project.read(alpha);
        assert!(text.contains(from));
        write(&project.root, alpha, &text.replace(from, to))
    };
    let (a, b) = (one.root.clone(), four.root.clone());
    big_stack(|| {
        let plugins: Vec<Box<dyn DynPlugin>> = vec![erase(Flamework::new())];
        let options = PluginOptions { plugins: &plugins, ..PluginOptions::default() };
        let mut session_one = BuildSession::new(compile_options(&a, Some(1)), &options, true);
        let (report, _) = session_one.build(&[]);
        assert!(errors(&report).is_empty(), "{:#?}", errors(&report));
        // The second copy starts from the first one's state (salt, seeds), as the harness seeds a clean build.
        let mut session_four =
            BuildSession::new(compile_options(&b, Some(4)), &options, true).with_seed(session_one.committed_states());
        let (report, _) = session_four.build(&[]);
        assert!(errors(&report).is_empty(), "{:#?}", errors(&report));
        assert_eq!(differences(&snapshot(&a), &snapshot(&b)), Vec::<String>::new(), "clean");

        let touched = edit(&one, "onInit() {}", "onInit() {\n\t\tprint(\"edited\");\n\t}");
        let (report, _) = session_one.build(&touched);
        assert!(errors(&report).is_empty(), "{:#?}", errors(&report));
        let touched = edit(&four, "onInit() {}", "onInit() {\n\t\tprint(\"edited\");\n\t}");
        let (report, _) = session_four.build(&touched);
        assert!(errors(&report).is_empty(), "{:#?}", errors(&report));
        assert_eq!(differences(&snapshot(&a), &snapshot(&b)), Vec::<String>::new(), "watch rebuild");
    });
    for (project, checkers) in [(&one, 1), (&four, 4)] {
        edit(project, "\"edited\"", "\"again\"");
        let outcome = build_once(&project.root, Some(checkers));
        outcome.assert_ok();
        assert_eq!(outcome.kind(), BuildKind::Incremental);
        assert_eq!(outcome.transformed, ["server/services/alpha.ts"]);
    }
    assert_eq!(differences(&snapshot(&one.root), &snapshot(&four.root)), Vec::<String>::new(), "incremental");
}

/// A file of the fixture that imports the package's type (`IMPORTS`) or not yet (`NO_IMPORT`).
const PKG_USE: &str = "src/shared/pkguse.ts";
const NO_IMPORT: &str = "export const libThingId = \"none\";\n";
const IMPORTS: &str = "import { Flamework } from \"@flamework-experimental/core\";\nimport type { LibThing } from \"@fwtest/lib\";\n\nexport const libThingId = Flamework.id<LibThing>();\n";

/// Installs `@fwtest/lib`, a package whose `flamework.build` names `LibThing`'s id `fl:lib@LibThing` (without it,
/// the id falls back to `@fwtest/lib:out/init@LibThing`), returning the touched paths (for [`watch`]).
fn install_package(root: &Path) -> Vec<String> {
    let build = format!(
        "{{\n\t\"version\": 1,\n\t\"flameworkVersion\": \"{}\",\n\t\"identifiers\": {{\n\t\t\"@fwtest/lib:out/init@LibThing\": \"fl:lib@LibThing\"\n\t}},\n\t\"identifierPrefix\": \"fl\"\n}}",
        flamework_rs::VERSION
    );
    let package = "node_modules/@fwtest/lib";
    let mut touched = Vec::new();
    for (file, text) in [
        ("package.json", "{ \"name\": \"@fwtest/lib\", \"version\": \"1.0.0\", \"main\": \"out/init.luau\", \"types\": \"out/index.d.ts\" }"),
        ("flamework.build", build.as_str()),
        ("out/index.d.ts", "export interface LibThing {\n\tthing(): void;\n}\n"),
        ("out/init.luau", "return {}\n"),
    ] {
        touched.extend(write(root, &format!("{package}/{file}"), text));
    }
    touched
}

/// A package installed and imported while watching, in either order: the rebuild reads the package's
/// `flamework.build` (set up again, as the program reaches a package directory setup did not look in) and writes what a
/// clean build of the same tree writes. Edits that reach no new package keep the setup.
#[test]
fn a_package_installed_while_watching_is_read() {
    for order in ["install-then-import", "import-then-install"] {
        let Some(project) = Project::new("fixture", &format!("night2-{order}")) else { return };
        project.write(PKG_USE, NO_IMPORT);
        let root = project.root.clone();
        let alpha = "src/server/services/alpha.ts";
        let alpha_text = project.read(alpha);
        let edited = alpha_text.replace("onInit() {}", "onInit() {\n\t\tprint(\"edited\");\n\t}");
        let at = || root.clone();
        let (r1, r2, r3, r4, r5) = (at(), at(), at(), at(), at());
        let mut edits: Vec<Edit> = vec![
            Box::new(move || install_package(&r1)),
            Box::new(move || write(&r2, PKG_USE, IMPORTS)),
            // a plain edit, and a new source file of the project: no new package
            Box::new(move || write(&r3, alpha, &edited)),
            Box::new(move || write(&r4, "src/shared/extra.ts", "export const extra = 1;\n")),
            Box::new(move || write(&r5, "src/shared/extra.ts", "export const extra = 2;\n")),
        ];
        if order == "import-then-install" {
            edits.swap(0, 1);
        }
        let outcomes = watch(&root, None, edits);
        let (first, second) = (&outcomes[1], &outcomes[2]);
        if order == "install-then-import" {
            first.assert_ok();
            assert_eq!(first.transformed, Vec::<String>::new(), "{order}: the install alone");
            second.assert_ok();
            assert!(second.summary.all, "{order}: the import sets up again");
        } else {
            assert!(!first.errors.is_empty(), "{order}: the import of a package not installed fails");
            second.assert_ok();
            assert!(second.summary.all, "{order}: the install sets up again");
        }
        let output = project.read("out/shared/pkguse.luau");
        assert!(output.contains("\"fl:lib@LibThing\""), "{order}:\n{output}");
        let later = ["server/services/alpha.ts", "shared/extra.ts", "shared/extra.ts"];
        for (file, outcome) in later.into_iter().zip(&outcomes[3..]) {
            outcome.assert_ok();
            assert!(!outcome.summary.all, "{order}: {file} keeps the setup");
            assert_eq!(outcome.transformed, [file], "{order}");
        }
        let watched = snapshot(&root);
        let clean = build_once(&root, None);
        clean.assert_ok();
        assert_eq!(clean.kind(), BuildKind::Clean);
        let differences = differences(&watched, &snapshot(&root));
        assert_eq!(differences, Vec::<String>::new(), "{order}: the watcher's tree against a clean build");
    }
}
