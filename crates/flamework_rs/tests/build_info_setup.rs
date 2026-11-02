//! The build-info setup that reads the disk: package build infos above the program's files, their refusals and the
//! version check, the derived string hashes, and late id assignment across file parts.

use std::path::PathBuf;

use flamework_rs::build_info::{
    BuildInfo, CandidateScan, PackageDirectories, check_project_version, load_package_build_infos,
    package_build_info_remedy,
};
use flamework_rs::ids::{Declaration, IdAnswer, IdConfig, IdMode, Ids};
use flamework_rs::inputs::FsInputs;
use flamework_rs::{VERSION, uuid};

/// A tree in a folder of the test's own: `<temp>/fwrs-setup-<label>-<pid>/root`. Nothing is written outside the
/// owned folder, and the upward scans are given the owned folder as their ceiling, so stray files in the temp
/// directory cannot affect them.
struct Tree {
    owned: PathBuf,
    root: PathBuf,
}

impl Tree {
    fn new(label: &str) -> Self {
        let owned = std::env::temp_dir().join(format!("fwrs-setup-{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&owned);
        let root = owned.join("root");
        std::fs::create_dir_all(&root).unwrap();
        Tree { owned, root }
    }

    fn ceiling(&self) -> String {
        self.owned.to_string_lossy().into_owned()
    }

    fn write(&self, relative: &str, text: &str) {
        let file = self.root.join(relative);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, text).unwrap();
    }

    fn path(&self, relative: &str) -> String {
        self.root
            .join(relative.replace('/', std::path::MAIN_SEPARATOR_STR))
            .to_string_lossy()
            .into_owned()
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.owned);
    }
}

fn build_info_text(version: &str, prefix: &str) -> String {
    format!(
        "{{\n\t\"version\": 1,\n\t\"flameworkVersion\": \"{version}\",\n\t\"identifiers\": {{}},\n\t\"identifierPrefix\": \"{prefix}\"\n}}"
    )
}

#[test]
fn package_build_infos_are_found_above_the_programs_files_once_each() {
    let t = Tree::new("scan");
    t.write("game/package.json", r#"{"name":"game"}"#);
    t.write("game/flamework.build", &build_info_text(VERSION, "game"));
    t.write(
        "game/node_modules/@flamework-experimental/core/package.json",
        r#"{"name":"@flamework-experimental/core"}"#,
    );
    t.write(
        "game/node_modules/@flamework-experimental/core/flamework.build",
        &build_info_text(VERSION, "$c"),
    );
    t.write(
        "game/node_modules/@s/lib/package.json",
        r#"{"name":"@s/lib"}"#,
    );
    t.write(
        "game/node_modules/@s/lib/flamework.build",
        &build_info_text("2.0.0-alpha.1", "@s/lib"),
    );
    // Five levels below the package: out of the upward scan's reach (the file and four parents).
    t.write(
        "game/node_modules/@s/deep/package.json",
        r#"{"name":"@s/deep"}"#,
    );
    t.write(
        "game/node_modules/@s/deep/flamework.build",
        &build_info_text(VERSION, "@s/deep"),
    );

    let dirs = [
        t.path("game/src/server"),
        t.path("game/node_modules/@flamework-experimental/core/out"),
        t.path("game/node_modules/@flamework-experimental/core/out/sub"),
        t.path("game/node_modules/@s/lib/out"),
        t.path("game/node_modules/@s/deep/a/b/c/d/e"),
    ];
    let owned = t.ceiling();
    let mut scan = CandidateScan::with_ceiling(&owned);
    let ceiling = Some(owned.as_str());
    let own = t.path("game/flamework.build");
    let candidates = scan.candidates(dirs.iter().map(String::as_str), &own.to_uppercase());
    assert_eq!(
        candidates,
        [
            t.path("game/node_modules/@flamework-experimental/core/flamework.build"),
            t.path("game/node_modules/@s/lib/flamework.build"),
        ],
        "the game's own is left out (compared without case on Windows), the core one is found once"
    );

    // A package of another scope may come from any 2.x; Flamework's own may not.
    let (infos, log) = load_package_build_infos(&candidates, &t.path("game"), ceiling).unwrap();
    assert_eq!(infos.len(), 2);
    assert_eq!(
        log[0],
        "Loaded buildInfo at node_modules/@flamework-experimental/core/flamework.build, next id: 1"
    );
    t.write(
        "game/node_modules/@flamework-experimental/core/flamework.build",
        &build_info_text("2.0.0-alpha.7", "$c"),
    );
    let error = load_package_build_infos(&candidates, &t.path("game"), ceiling).unwrap_err();
    assert!(error.0.contains("  @flamework-experimental/core 2.0.0-alpha.7 (node_modules/@flamework-experimental/core/flamework.build)"), "{error}");

    // A package build info that cannot be used names its package.
    t.write("game/node_modules/@s/lib/flamework.build", "{");
    let error = load_package_build_infos(&candidates[1..], &t.path("game"), ceiling).unwrap_err();
    assert_eq!(
        error.0,
        format!(
            "Flamework cannot use node_modules/@s/lib/flamework.build: it is not valid JSON (Expected property name or '}}' in JSON at position 1 (line 1 column 2)).\n{}",
            package_build_info_remedy(&candidates[1], ceiling)[0]
        )
    );
    assert!(error.0.ends_with("It came with the package @s/lib, which wrote it when it was built, and this build reads it for the ids of that package's classes. Reinstall the package, or build it again if it is your own."));

    // The project's own: in the directory, else at the package root.
    let own_info = BuildInfo::from_directory(&t.path("game/src"), &[], ceiling)
        .unwrap()
        .unwrap();
    assert_eq!(own_info.identifier_prefix(), Some("game"));

    // The scans stop at the ceiling: a package.json above it is not found.
    t.write("lone/x/file.ts", "");
    assert_eq!(
        flamework_rs::build_info::find_package_json(&t.path("lone/x"), ceiling),
        None
    );
    std::fs::write(t.owned.join("package.json"), r#"{"name":"owned"}"#).unwrap();
    assert_eq!(
        flamework_rs::build_info::package_name_of(&t.path("lone/x/file.ts"), ceiling).as_deref(),
        Some("owned"),
        "the ceiling itself is still looked in"
    );
}

#[test]
fn the_project_version_check_names_both_versions() {
    let mut info = BuildInfo::new("x");
    assert!(check_project_version(&info, "out", None).is_ok());
    info = BuildInfo::from_value(
        "x",
        flamework_rs::json::parse(&build_info_text("2.0.0-alpha.1", "p")).unwrap(),
    );
    assert_eq!(
        check_project_version(&info, "out", None).unwrap_err(),
        [
            "Project was compiled on different version of Flamework.".to_owned(),
            "Please recompile by deleting the out directory".to_owned(),
            format!("Current Flamework Version: {VERSION}"),
            "Previous Flamework Version: 2.0.0-alpha.1".to_owned(),
        ]
    );
    // An incremental project names its build info instead (`transformer.ts`).
    assert_eq!(
        check_project_version(&info, "out", Some("tsconfig.rbxts-rs.buildinfo")).unwrap_err()[1],
        "This is an incremental build, which recompiles only the files that changed. Delete tsconfig.rbxts-rs.buildinfo and build again: the next build compiles every file."
    );
}

/// Every path the scan reads, with forward slashes.
struct Recorded(Vec<String>);

impl flamework_rs::inputs::Inputs for Recorded {
    fn read(&mut self, path: &str) -> Option<String> {
        self.0.push(path.replace('\\', "/"));
        std::fs::read_to_string(path).ok()
    }
    fn env(&mut self, _: &str) -> Option<String> {
        None
    }
}

/// The scan takes the project's own `flamework.build` as there without reading it (it is the plugin's state file,
/// which setup may not depend on), so it stops at the project directory whether the file exists or not.
#[test]
fn the_scan_never_reads_the_projects_own_build_info() {
    let t = Tree::new("own");
    // One above the game, within the scan's reach from `game/src`: a first build (no own file) would find it in TS.
    t.write("flamework.build", &build_info_text(VERSION, "above"));
    t.write("game/package.json", r#"{"name":"game"}"#);
    let own = t.path("game/flamework.build");
    let owned = t.ceiling();
    for exists in [false, true] {
        if exists {
            t.write("game/flamework.build", &build_info_text(VERSION, "game"));
        }
        let mut inputs = Recorded(Vec::new());
        let mut scan = CandidateScan::with_ceiling(&owned);
        let packages = [t.path("game"), t.path("game/node_modules/@s/p")];
        scan.look_only_in_packages(packages.iter().map(String::as_str));
        let starts = [t.path("game/src"), t.path("game/node_modules/@s/p/out")];
        let candidates = scan.candidates_with(&mut inputs, starts.iter().map(String::as_str), &own);
        assert_eq!(candidates, Vec::<String>::new(), "exists: {exists}");
        // The package's directory; the project's own is taken as there.
        assert_eq!(
            inputs.0,
            [t.path("game/node_modules/@s/p/flamework.build").replace('\\', "/")],
            "exists: {exists}"
        );
    }
}

/// Given the program's package directories, the scan looks only in those: where a package's build writes its
/// `flamework.build`, beside its `package.json`, wherever the package is (below `node_modules`, linked from outside,
/// or a folder of the project's sources with a `package.json`, as the TS transformer's tests make one).
#[test]
fn the_scan_looks_only_in_package_directories() {
    let t = Tree::new("where");
    t.write("game/package.json", r#"{"name":"game"}"#);
    // Outside a package directory: the TS transformer would find these first.
    t.write("game/src/flamework.build", &build_info_text(VERSION, "sources"));
    t.write("game/node_modules/@s/lib/out/flamework.build", &build_info_text(VERSION, "subfolder"));
    t.write("game/src/vendored/flamework.build", &build_info_text(VERSION, "vendored"));
    t.write("game/node_modules/@s/lib/flamework.build", &build_info_text(VERSION, "@s/lib"));
    t.write("linked/core/flamework.build", &build_info_text(VERSION, "core"));
    let own = t.path("game/flamework.build");
    let owned = t.ceiling();
    let mut inputs = Recorded(Vec::new());
    let mut scan = CandidateScan::with_ceiling(&owned);
    let packages = [
        t.path("game"),
        t.path("game/src/vendored"),
        t.path("game/node_modules/@s/lib"),
        t.path("game/node_modules/a"),
        t.path("game/node_modules/a/node_modules/@b/c"),
        t.path("linked/core"),
    ];
    scan.look_only_in_packages(packages.iter().map(String::as_str));
    let starts = [
        t.path("game/src/server/deep"),
        t.path("game/src/vendored"),
        t.path("game/node_modules/@s/lib/out/sub"),
        t.path("game/node_modules/a/node_modules/@b/c/out"),
        t.path("linked/core/out"),
    ];
    let candidates = scan.candidates_with(&mut inputs, starts.iter().map(String::as_str), &own);
    assert_eq!(
        candidates,
        [
            t.path("game/src/vendored/flamework.build"),
            t.path("game/node_modules/@s/lib/flamework.build"),
            t.path("linked/core/flamework.build"),
        ]
    );
    // A nested package's directory, then its parent package's: the scan goes on above a package without one.
    let read: Vec<String> = [
        "game/src/vendored",
        "game/node_modules/@s/lib",
        "game/node_modules/a/node_modules/@b/c",
        "game/node_modules/a",
        "linked/core",
    ]
    .iter()
    .map(|dir| t.path(&format!("{dir}/flamework.build")).replace('\\', "/"))
    .collect();
    assert_eq!(inputs.0, read);
}

/// A watch rebuild sets up again when its added files reach a package directory the scan did not look in: each added
/// file's nearest `package.json` directory, unless a known package directory comes first.
#[test]
fn added_files_in_a_new_package_directory_are_noticed() {
    let t = Tree::new("newpkg");
    t.write("game/package.json", r#"{"name":"game"}"#);
    t.write("game/node_modules/@s/known/package.json", r#"{"name":"@s/known"}"#);
    t.write("game/node_modules/@s/new/package.json", r#"{"name":"@s/new"}"#);
    let known_directories = [t.path("game"), t.path("game/node_modules/@s/known")];
    let known = PackageDirectories::new(known_directories.iter().map(String::as_str));
    let first_new = |files: &[&str]| {
        let files: Vec<String> = files.iter().map(|f| t.path(f).replace('\\', "/")).collect();
        known.first_new_with(&mut FsInputs, files.iter().map(String::as_str))
    };
    assert_eq!(first_new(&[]), None);
    assert_eq!(first_new(&["game/src/server/new.ts", "game/node_modules/@s/known/out/index.d.ts"]), None);
    assert_eq!(
        first_new(&["game/src/a.ts", "game/node_modules/@s/new/out/sub/index.d.ts"]).map(|d| d.replace('\\', "/")),
        Some(t.path("game/node_modules/@s/new").replace('\\', "/"))
    );
    assert!(known.contains(&t.path("game").to_uppercase()) == cfg!(windows));
}

/// Per file, the variables `Flamework.env` inlined there and their values.
type EnvUses<'a> = [(&'a str, &'a [(&'a str, Option<&'a str>)])];

/// A build info with recorded inputs: `mode`, the config hash `config` and `inputs.env`.
fn recorded(mode: &str, config: &str, env: &EnvUses<'_>) -> BuildInfo {
    let mut info = BuildInfo::new("x");
    info.set_config_input(config);
    info.set_id_generation_mode(mode);
    for (origin, uses) in env {
        for (name, value) in *uses {
            info.add_env_input(origin, name, &flamework_rs::env::hash_env_value(*value));
        }
    }
    info
}

/// `refuseChangedInputs`'s comparison, which under rbxts-rs starts a new table instead of refusing: the mode before
/// the config (never both, as TS names them), nothing for unrecorded inputs or another version, and the inlined
/// variables never (the compiler transforms again the files that read a changed one).
#[test]
fn changed_inputs_name_the_mode_before_the_config() {
    use flamework_rs::build_info::changed_inputs;
    let info = recorded("full", "c1", &[("src/b.ts", &[("ZED", Some("1"))])]);
    assert!(changed_inputs(&info, "full", "c1").is_empty());
    assert_eq!(changed_inputs(&info, "short", "c2"), ["idGenerationMode is 'short', and was 'full'"]);
    assert_eq!(
        changed_inputs(&info, "full", "c2"),
        ["an option in flamework.config.json that shapes the emitted code changed (the transformer or serialization section, or networking.serialization)"]
    );
    assert!(changed_inputs(&BuildInfo::new("x"), "short", "c2").is_empty());
    let other = BuildInfo::from_value(
        "x",
        flamework_rs::json::parse(
            "{\"version\":1,\"flameworkVersion\":\"2.0.0-alpha.1\",\"identifiers\":{},\"idGenerationMode\":\"full\"}",
        )
        .unwrap(),
    );
    assert!(changed_inputs(&other, "short", "c2").is_empty());
}

/// The project's own `flamework.build` read for reuse: refused as the TS refuses it, with the remedy of an incremental
/// build or a watcher, and the dirty-environment stop.
#[test]
fn an_unusable_own_build_info_is_refused_with_the_ts_texts() {
    use flamework_rs::build_info::{dirty_environment, own_build_info_remedy};
    let incremental = own_build_info_remedy(Some("tsconfig.rbxts-rs.buildinfo"));
    assert_eq!(
        incremental,
        ["This incremental build reads it to keep the ids of the files it does not recompile. Delete tsconfig.rbxts-rs.buildinfo and build again: a full build does not read flamework.build, and writes a new one."]
    );
    assert_eq!(
        own_build_info_remedy(None),
        ["This watcher reads it on every rebuild, to keep the ids of the files it does not recompile. Restart the watcher: a full build does not read flamework.build, and writes a new one."]
    );
    let refused = |text: &str| BuildInfo::from_text("x", text, "flamework.build", &incremental).unwrap_err().0;
    assert_eq!(
        refused("{\"version\": 1,"),
        // node 23's `JSON.parse` message.
        format!(
            "Flamework cannot use flamework.build: it is not valid JSON (Expected double-quoted property name in JSON at position 14 (line 1 column 15)).\n{}",
            incremental[0]
        )
    );
    assert!(refused("  \n").starts_with("Flamework cannot use flamework.build: it is empty.\n"));
    assert!(refused("{}").starts_with("Flamework cannot use flamework.build: it does not have the shape Flamework writes ("));
    // A byte order mark is dropped, as `ts.sys.readFile` drops it.
    assert!(BuildInfo::from_text("x", &format!("\u{feff}{}", build_info_text(VERSION, "p")), "flamework.build", &[]).is_ok());
    assert_eq!(
        dirty_environment(Some("tsconfig.rbxts-rs.buildinfo")),
        [
            "Flamework cannot be built in a dirty environment.",
            "This incremental build has no flamework.build to match the files it does not recompile. Delete tsconfig.rbxts-rs.buildinfo and build again.",
        ]
    );
}

/// What `finish` makes again from the file parts goes, the rest keeps its place; `env_inputs` reads `inputs.env` back.
#[test]
fn a_reused_build_info_forgets_only_the_per_file_records() {
    let mut info = recorded("short", "c1", &[("src/a.ts", &[("A", Some("1")), ("B", None)])]);
    info.salt();
    info.add_identifier("p:out/a@A", "a@A{x}").unwrap();
    info.add_path_use("src/a.ts", &flamework_rs::build_info::PathUse {
        path: "src/x".into(),
        macro_name: "registerProviders".into(),
        line: 1,
        column: 1,
        raises: false,
    });
    assert_eq!(
        info.env_inputs(),
        [("src/a.ts".to_owned(), vec![
            ("A".to_owned(), flamework_rs::env::hash_env_value(Some("1"))),
            ("B".to_owned(), "unset".to_owned())
        ])]
    );
    info.forget_file_records();
    let data = info.data();
    assert_eq!(
        data.keys().cloned().collect::<Vec<_>>(),
        ["version", "flameworkVersion", "identifiers", "inputs", "idGenerationMode", "salt"]
    );
    assert!(info.env_inputs().is_empty());
    assert_eq!(info.config_input(), Some("c1"));
    assert_eq!(info.identifier_from_internal("p:out/a@A").as_deref(), Some("a@A{x}"));
    // The object round-trips through the persistent state.
    let again = BuildInfo::from_data("x", data.clone());
    assert_eq!(again.to_json(), info.to_json());
}

#[test]
fn string_hashes_derive_from_the_build_seed() {
    let mut info = BuildInfo::new("x");
    info.set_build_seed("6f4c1d2e-8b3a-4e5f-9c7d-2a1b0e9f8d7c")
        .unwrap();
    assert_eq!(
        info.set_build_seed("not a uuid"),
        Err("Invalid UUID".to_owned())
    );
    let a = info.hash_string("src/glob/**", Some("addPaths"));
    assert_eq!(
        a,
        uuid::v5(
            "addPaths:src/glob/**",
            "6f4c1d2e-8b3a-4e5f-9c7d-2a1b0e9f8d7c"
        )
        .unwrap()
    );
    assert_eq!(
        info.hash_string("src/glob/**", Some("addPaths")),
        a,
        "the same within a build"
    );
    assert_eq!(
        info.hash_string("x", None),
        uuid::v5("@:x", "6f4c1d2e-8b3a-4e5f-9c7d-2a1b0e9f8d7c").unwrap(),
        "the context defaults to @"
    );
    assert!(!info.to_json().contains("stringHashes"), "no table is kept");

    let mut fresh = BuildInfo::new("y");
    let seed = fresh.build_seed();
    assert!(uuid::validate(seed.as_str()));
    assert_ne!(
        fresh.hash_string("src/glob/**", Some("addPaths")),
        a,
        "a clean build rotates them"
    );
    assert_eq!(fresh.salt().len(), 128);

    // A seed that is not a uuid: refused when read from a file, dropped from a value, never a panic.
    let bad =
        r#"{ "version": 1, "flameworkVersion": "x", "identifiers": {}, "buildSeed": "nope" }"#;
    assert_eq!(
        flamework_rs::build_info::find_problem(Some(bad)).as_deref(),
        Some("its buildSeed \"nope\" is not a uuid")
    );
    let mut from_value = BuildInfo::from_value("z", flamework_rs::json::parse(bad).unwrap());
    let replaced = from_value.build_seed();
    assert!(uuid::validate(replaced.as_str()));
    let _ = from_value.hash_string("a", None);
    assert_eq!(
        flamework_rs::ids::callsite_uuid("p", "src/a.ts", "A", 3, Some(&replaced)),
        replaced.v5("p:src/a.ts@A+3")
    );
}

/// Ids are the same whatever order the checkers transformed the files in, as long as the parts are merged in transform
/// order (roblox-ts's `getChangedFilePaths` order).
#[test]
fn late_ids_follow_transform_order_not_checker_order() {
    let ids = Ids::new(IdConfig {
        mode: IdMode::Short,
        hash_prefix: None,
        package_name: "game".into(),
        is_game: true,
        salt: "salt".into(),
    });
    let decl = |file: &str, name: &str| Declaration {
        internal_id: format!("game:out/{file}@{name}"),
        is_package: false,
        name: name.into(),
        source_file: format!("C:\\p\\src\\{file}.ts"),
        file_name: file.into(),
    };
    let files = [
        vec![decl("a", "A"), decl("a", "B")],
        vec![decl("b", "C"), decl("a", "A")],
    ];

    let run = |checker_order: &[usize]| {
        let mut info = BuildInfo::new("x");
        let mut parts: Vec<(usize, Vec<_>)> = Vec::new();
        let mut answers: Vec<(usize, Vec<IdAnswer>)> = Vec::new();
        for &f in checker_order {
            let mut part = Vec::new();
            let mut file_answers = Vec::new();
            for d in &files[f] {
                let (answer, new) = ids.request(&info, d);
                if let Some(new) = new {
                    part.push(new);
                }
                file_answers.push(answer);
            }
            parts.push((f, part));
            answers.push((f, file_answers));
        }
        parts.sort_by_key(|(f, _)| *f);
        let resolved = ids.assign(&mut info, parts.iter().map(|(_, p)| p.as_slice()));
        answers.sort_by_key(|(f, _)| *f);
        let ids: Vec<String> = answers
            .into_iter()
            .flat_map(|(_, a)| a)
            .map(|a| match a {
                IdAnswer::Now(id) => id,
                IdAnswer::Late(key) => resolved[&key].clone(),
            })
            .collect();
        (ids, info.to_json())
    };
    let forward = run(&[0, 1]);
    assert_eq!(run(&[1, 0]), forward);
    assert!(forward.0[0].starts_with("a@A{") && forward.0[3] == forward.0[0]);
}
