//! `Plugin::setup` (the TS transformer's `TransformState` constructor and `setupBuildInfo`): the package, the config
//! and environment, the package build infos with the version check, the id settings, and the fresh `flamework.build`
//! with its keys created in DESIGN §10.4's order.
//!
//! Everything is read through the compiler ([`SetupCx::read_input`] / [`SetupCx::env`]), so every file and variable
//! read here is a build input.
//!
//! **The previous `flamework.build`** (the TS transformer's `buildInfoRead`). A clean build starts from a new one, as
//! a TS5 full build does: it rotates the salt and the build seed. Every other build reuses the last committed one, so
//! the files it does not transform keep their ids, salt, build seed and `Modding.Hash` seed:
//! - a watch rebuild that runs setup again: the committed [`FwState`], handed on in memory;
//! - an incremental build in a new process, and a watch rebuild whose state file was deleted or edited (the compiler
//!   hands a fresh state then): the file, read with [`SetupCx::read_persisted`] (`fromDirectory`, the project's own
//!   file only) and refused when it cannot be used, with the TS texts;
//! - a clean build the W1 harness seeds with a watcher's committed state (`rbxts_transform`'s
//!   `BuildSession::with_seed`).
//!
//! A reused one whose id mode or code-shaping config differs from this build's ([`build_info::changed_inputs`]) is
//! dropped for a new one, as a clean build makes it (new salt, build seed and hash seed). **This deviates from TS**,
//! whose `refuseChangedInputs` stops the build because it cannot make roblox-ts recompile every file; under rbxts-rs
//! those inputs are build inputs of setup, so a change already makes the compiler transform every file, and no file
//! keeps what the old inputs emitted. A reused one of another version stops at the project version check
//! ([`build_info::check_project_version`]). The `Shared` this makes holds a snapshot of it for the transform;
//! `finish` works from the state, which a watch rebuild that keeps the `Shared` has advanced since (DESIGN §14).

use std::sync::atomic::{AtomicBool, Ordering};

use rbxts_plugin::{BuildKind, Cx, Fatal, SetupCx};

use crate::build_info::{self, BUILD_INFO_NAME, BuildInfo, CandidateScan};
use crate::config::{self, LoadedProjectConfig, TransformerConfig};
use crate::ids::{IdConfig, IdMode, Ids};
use crate::inputs::{Inputs, strip_bom};
use crate::json::{self, JsObject, JsValue};
use crate::packages::{CORE_PACKAGE, FLAMEWORK_SCOPE};
use crate::{ProjectError, node_path, uuid};

/// The test-only switch that pins the build seed (DESIGN §10.4), so obfuscated builds can be compared: the build seed,
/// the `Modding.Hash` seed and the salt (derived from it) of a new `flamework.build`; a reused one keeps its own.
pub const FIXED_SEED_VAR: &str = "FLAMEWORK_RS_FIXED_SEED";

/// The plugin's persistent state (`Plugin::State`, DESIGN §14): what the last committed build left in
/// `flamework.build`, and the `Modding.Hash` seed. The compiler hands a copy to `setup` and `finish` and commits it
/// with the build; a new process starts from `Default` and setup reloads the file.
#[derive(Debug, Clone, Default)]
pub struct FwState {
    /// The project's `flamework.build` as an object, without the package build infos. `None` until a build has made
    /// one in this process (or a seeded clean build was handed one).
    pub(crate) build_info: Option<JsObject>,
    /// The seed `Modding.Hash` derives from outside obfuscation (under obfuscation it is the build seed). It goes into
    /// `flamework.build` as `buildSeed` only once a file has used one, as the TS transformer adds `stringHashes`
    /// only then, so it is kept here as well for the watch rebuilds before that.
    pub(crate) hash_seed: Option<String>,
}

/// What every hook reads, made once per build (DESIGN §10.1). Read-only during the transform.
#[derive(Debug)]
pub struct FwShared {
    /// The tsconfig's directory (`currentDirectory`).
    pub(crate) project_dir: String,
    /// The directory of the package.json above it (`rootDirectory`): file ids and globs are relative to it.
    pub(crate) root_dir: String,
    pub(crate) package_name: String,
    /// A game (an unscoped package name), as opposed to a package.
    pub(crate) is_game: bool,
    /// `rootDirs`, else `[rootDir ?? currentDirectory]`: the project's own source roots.
    pub(crate) root_dirs: Vec<String>,
    /// The include folder (`--includePath`), absolute.
    pub(crate) include_dir: String,
    /// `transformer` options, with `idGenerationMode` and (for a package) `hashPrefix` filled in.
    pub(crate) config: TransformerConfig,
    /// The whole config file.
    pub(crate) loaded: LoadedProjectConfig,
    pub(crate) ids: Ids,
    /// This build's `flamework.build` as setup leaves it, with the package build infos as children. A snapshot for the
    /// transform's id lookups: a kept `Shared` misses the ids later builds added, which the transform then asks for as
    /// new ones and `finish` answers from [`FwState`].
    pub(crate) build_info: BuildInfo,
    /// The build seed, under obfuscation.
    pub(crate) build_seed: Option<uuid::Namespace>,
    /// The namespace of `Modding.Hash` values: the build seed under obfuscation, else the reused build info's
    /// `buildSeed` or [`FwState::hash_seed`], else a new seed (pinned by [`FIXED_SEED_VAR`]). The TS transformer made
    /// a random v4 per string and kept them in `flamework.build`'s `stringHashes`; derived ones need only the seed
    /// (DESIGN §10.3, §10.4), which a build that reuses `flamework.build` keeps.
    pub(crate) hash_seed: uuid::Namespace,
    /// A file transformed with this `Shared` used `Modding.Hash` ([`FwShared::hash_string`]): `finish` records the
    /// seed in `flamework.build`.
    pub(crate) hash_used: AtomicBool,
    /// Whether the project has a Rojo project file (Flamework's `rojoResolver` is undefined without one).
    pub(crate) has_rojo: bool,
    /// Flamework's own path translator: roblox-ts's, with `.lua` outputs (`createPathTranslator`).
    pub(crate) path_translator: rbxts_project::path_translator::PathTranslator,
}

impl FwShared {
    /// `obfuscateText`: the text, or under obfuscation its derived hash (DESIGN §10.4).
    pub(crate) fn obfuscate_text(&self, text: &str, context: Option<&str>) -> String {
        match &self.build_seed {
            Some(seed) if self.config.obfuscation == Some(true) => {
                seed.v5(&format!("{}:{text}", context.unwrap_or("@")))
            }
            _ => text.to_owned(),
        }
    }

    /// `Flamework.env(name)` at transform time ([`crate::env::transform_env`]): what setup read from the `.env`
    /// files and the process, else the process environment through [`Cx::env`], recorded as an input of the file.
    pub(crate) fn env(&self, cx: &mut Cx<'_>, name: &str) -> Option<String> {
        crate::env::transform_env(&self.loaded.env, name, |name| cx.env(name))
    }

    /// `buildInfo.hashString`: a derived hash of `context:text` (`Modding.Hash`).
    pub(crate) fn hash_string(&self, text: &str, context: Option<&str>) -> String {
        self.hash_used.store(true, Ordering::Relaxed);
        self.hash_seed.v5(&format!("{}:{text}", context.unwrap_or("@")))
    }

    /// `getFileId`: a file relative to the package root, with forward slashes.
    pub(crate) fn file_id(&self, file_name: &str) -> String {
        node_path::relative(&self.root_dir, file_name).replace('\\', "/")
    }

    /// The `Reflect` import's specifier. Inside core itself, `Reflect` is imported by its relative module path.
    pub(crate) fn reflect_specifier(&self, file_name: &str) -> String {
        reflect_specifier(&self.package_name, &self.root_dir, file_name)
    }
}

/// [`FwShared::reflect_specifier`] from the package name and root directory.
pub(crate) fn reflect_specifier(package_name: &str, root_dir: &str, file_name: &str) -> String {
    if package_name == CORE_PACKAGE {
        let module = node_path::join(&[root_dir, "src", "reflect"]);
        let relative = node_path::relative(&node_path::dirname(file_name), &module).replace('\\', "/");
        // `"./" + relative || "."`: never empty.
        format!("./{relative}")
    } else {
        CORE_PACKAGE.to_owned()
    }
}

/// [`Inputs`] over the compiler's recorded reads.
struct SetupInputs<'c, 'a> {
    cx: &'c mut SetupCx<'a>,
}

impl Inputs for SetupInputs<'_, '_> {
    fn read(&mut self, path: &str) -> Option<String> {
        self.cx.read_input(path).ok().flatten()
    }

    fn env(&mut self, name: &str) -> Option<String> {
        self.cx.env(name)
    }
}

/// A project mistake, said as the TS transformer says it: the first line as the message, the rest as the hint.
pub fn fatal(error: ProjectError) -> Fatal {
    let mut lines = error.0.splitn(2, '\n');
    let headline = lines.next().unwrap_or_default().to_owned();
    match lines.next() {
        Some(rest) => Fatal::new(headline).with_hint(rest),
        None => Fatal::new(headline),
    }
}

fn entry_object(cx: &SetupCx<'_>) -> JsObject {
    let Some(entry) = cx.tsconfig_entry() else {
        return JsObject::new();
    };
    // serde_json keeps the file's order (`preserve_order`); JSON text round-trips into JavaScript's order.
    let text = serde_json::to_string(entry).unwrap_or_else(|_| "{}".into());
    match json::parse(&text) {
        Ok(JsValue::Object(object)) => object,
        _ => JsObject::new(),
    }
}

/// The salt a pinned seed ([`FIXED_SEED_VAR`]) gives a new build info: 128 hex digits, as a random one has.
fn pinned_salt(seed: &str) -> String {
    let part = |n: u8| crate::sha::hex(&crate::sha::sha256(format!("flamework-rs salt {n}:{seed}").as_bytes()));
    format!("{}{}", part(0), part(1))
}

/// The previous `flamework.build` this build reuses, if any (`setupBuildInfo`'s first half; module docs), and whether
/// it was read from the disk.
fn previous_build_info(
    cx: &mut SetupCx<'_>,
    state: &mut FwState,
    path: &str,
    state_file: Option<&str>,
) -> Result<(Option<BuildInfo>, bool), Fatal> {
    if let Some(data) = state.build_info.take() {
        return Ok((Some(BuildInfo::from_data(path, data)), false));
    }
    // A full build does not reuse the previous flamework.build, so it does not read it either: one that cannot be
    // used is replaced, not refused.
    if cx.build_kind() == BuildKind::Clean {
        return Ok((None, false));
    }
    let remedy = build_info::own_build_info_remedy(state_file);
    let read = match cx.read_persisted(path) {
        Ok(Some(bytes)) => String::from_utf8(bytes).ok(),
        Ok(None) => {
            // Only the files this build does not transform need the old table; the compiler transforms them all when
            // the file it left is gone. A defensive stop: rbxts-rs's build session never reaches it (a missing
            // flamework.build is a lost state file), only another `SetupBackend` host could.
            if !cx.transforms_all_files() {
                let mut lines = build_info::dirty_environment(state_file).into_iter();
                let headline = lines.next().unwrap_or_default();
                return Err(Fatal::new(headline).with_hint(lines.collect::<Vec<_>>().join("\n")));
            }
            return Ok((None, false));
        }
        Err(_) => None,
    };
    let info = match read {
        Some(text) => BuildInfo::from_text(path, &text, BUILD_INFO_NAME, &remedy),
        None => {
            // `ts.sys.readFile` gave nothing: TS's "it could not be read".
            let mut lines = vec![format!("Flamework cannot use {BUILD_INFO_NAME}: it could not be read.")];
            lines.extend(remedy);
            Err(ProjectError(lines.join("\n")))
        }
    }
    .map_err(fatal)?;
    Ok((Some(info), true))
}

/// Runs setup (module docs): `state` is the working copy of the persistent state, which setup leaves holding this
/// build's `flamework.build` as `finish` starts from it.
pub fn setup(cx: &mut SetupCx<'_>, state: &mut FwState) -> Result<FwShared, Fatal> {
    let services = cx.project();
    let project_dir = services.data.project_path.clone();
    // The compiler's incremental state file, as the remedies name it: relative to the project, with forward slashes.
    let state_file = cx
        .incremental_state_path()
        .map(|path| node_path::relative_slash(&project_dir, &path.to_string_lossy()));
    let include_dir = services.data.options.include_path.clone();
    let has_rojo = services.data.rojo_config_path.is_some();
    let path_translator = rbxts_project::path_translator::PathTranslator::new(
        services.path_translator.root_dir.clone(),
        services.path_translator.out_dir.clone(),
        None,
        services.path_translator.declaration,
        false,
    );
    let options = cx.options();
    let root_dir_option = options.root_dir.clone();
    let root_dirs_option = options.root_dirs.clone();
    let out_dir_option = options.out_dir.clone();
    let entry = entry_object(cx);
    let program = cx.program();

    // getPackageJson(currentDirectory)
    let (root_dir, package_name) = {
        let mut inputs = SetupInputs { cx };
        let package_json = build_info::find_package_json_with(&mut inputs, &project_dir, None)
            .ok_or_else(|| Fatal::new(format!("package.json not found in {project_dir}")))?;
        let text = inputs.read(&package_json).map(strip_bom).unwrap_or_default();
        let name = json::parse(&text)
            .ok()
            .and_then(|v| v.get("name").and_then(JsValue::as_str).map(|s| s.trim().to_owned()))
            .filter(|n| !n.is_empty())
            .ok_or_else(|| Fatal::new(format!("{package_json} has no package name")))?;
        (node_path::dirname(&package_json), name)
    };
    let is_game = !package_name.starts_with('@');

    // The `$schema` line the TS transformer adds to a game's flamework.config.json is not written: it would edit a
    // user's source file from a build (Q-F5, the user's call).

    let loaded = config::load_project_config_with(
        &mut SetupInputs { cx },
        &project_dir,
        &root_dir,
        &entry,
    )
    .map_err(fatal)?;
    if let Some(path) = &loaded.config_path {
        let shown = node_path::relative(&project_dir, path);
        cx.info(&format!(
            "Loaded project config from {}",
            if shown.is_empty() { path.as_str() } else { shown.as_str() }
        ));
    }

    let mut config = loaded.transformer();
    let build_info_path = node_path::join(&[&project_dir, BUILD_INFO_NAME]);
    let (previous, read_from_disk) = previous_build_info(cx, state, &build_info_path, state_file.as_deref())?;
    // `buildInfoRead`, and `buildInfoReused` (read as the process starts: an incremental build's first compilation).
    let build_info_read = previous.is_some();
    let mut build_info_reused = read_from_disk && cx.build_kind() == BuildKind::Incremental;
    let mut build_info = previous.unwrap_or_else(|| BuildInfo::new(&build_info_path));

    // The package build infos above the program's files, once per directory, looked for in the program's package
    // directories only (`CandidateScan`).
    {
        let mut directories: Vec<String> = Vec::new();
        let mut packages: Vec<&str> = Vec::new();
        for file in program.source_files() {
            let file = file.get();
            let directory = node_path::dirname(file.file_name());
            if directories.last() != Some(&directory) {
                directories.push(directory);
            }
            let package = program.get_source_file_meta_data(file.path()).package_json_directory.as_str();
            if !package.is_empty() && packages.last() != Some(&package) {
                packages.push(package);
            }
        }
        let mut inputs = SetupInputs { cx };
        let mut scan = CandidateScan::new();
        scan.look_only_in_packages(packages);
        let candidates = scan.candidates_with(
            &mut inputs,
            directories.iter().map(String::as_str),
            &build_info_path,
        );
        let (infos, log) = build_info::load_package_build_infos_with(
            &mut inputs,
            &candidates,
            &project_dir,
            None,
        )
        .map_err(fatal)?;
        for line in log {
            cx.info(&line);
        }
        for info in infos {
            build_info.add_build_info(info);
        }
    }

    let mode = config.effective_id_mode();
    config.id_generation_mode = Some(mode);

    let config_input = config::fingerprint_emit_config(&loaded);
    // `refuseChangedInputs`, deviating from TS (module docs): a reused table made from another id mode or code-shaping
    // config is dropped for a new one, with a new salt and new seeds, as a clean build would make it. Both inputs are
    // build inputs of setup, so the compiler transforms every file whenever they change: no file keeps what the old
    // inputs emitted, where the TS transformer, which cannot make roblox-ts recompile them, has to stop the build.
    let changes =
        if build_info_read { build_info::changed_inputs(&build_info, mode.as_str(), &config_input) } else { Vec::new() };
    if !changes.is_empty() {
        if !cx.transforms_all_files() {
            // Unreachable under rbxts-rs (the mode and the config are setup inputs); a host that breaks that stops
            // here, as the TS transformer does, rather than mixing outputs of two inputs.
            return Err(Fatal::new(format!(
                "Flamework's inputs changed since flamework.build was written: {}.",
                changes.join("; ")
            ))
            .with_hint(format!(
                "This is an incremental build, which recompiles only the files that changed, so the others would keep \
                 what the old inputs emitted. Delete {} and build again: the next build compiles every file.",
                state_file.as_deref().unwrap_or("the incremental state file")
            )));
        }
        cx.info(&format!(
            "flamework.build was written from other inputs: {}. Every file is transformed, and flamework.build starts \
             anew (new ids, salt and seeds).",
            changes.join("; ")
        ));
        build_info = build_info.renewed();
        build_info_reused = false;
        state.hash_seed = None;
    }

    // In the TS transformer's order (DESIGN §10.4): inputs, idGenerationMode, identifierPrefix, then salt and
    // buildSeed, which TS adds at their first use and setup makes here; metadata comes last, in finish. A reused
    // build info keeps its keys where they are.
    build_info.set_config_input(&config_input);
    build_info.set_id_generation_mode(mode.as_str());

    let obfuscation = config.obfuscation == Some(true);
    if obfuscation && config.salt.is_some() {
        cx.warn(
            "transformer.salt is set with obfuscation on",
            Some(
                "A fixed salt gives every build the same obfuscated ids; leave it unset so each build gets fresh ones.",
            ),
        );
    }
    if obfuscation && build_info_reused {
        cx.warn(
            "obfuscated ids were kept from the previous build",
            Some(&format!(
                "This is an incremental build, so files that do not recompile keep their ids and the rest have to match.\nDelete {} before a release build to rotate every id.",
                state_file.as_deref().unwrap_or("the tsbuildinfo")
            )),
        );
    }

    if is_game
        && !out_dir_option.is_empty()
        && node_path::is_path_descendant_of(&include_dir, &out_dir_option)
    {
        let include = node_path::relative(&project_dir, &include_dir).replace('\\', "/");
        cx.warn(
            &format!(
                "The include folder {} is inside outDir, which roblox-ts cleans of files that have no source",
                if include.is_empty() { "." } else { include.as_str() }
            ),
            Some(
                "include/flamework (globs.json, paths.json, config.json) is written again only after a build that succeeds, so a build that fails can leave the place without it. Move the include folder outside outDir.",
            ),
        );
    }

    if !is_game && config.hash_prefix.is_none() {
        config.hash_prefix = Some(package_name.clone());
    }
    build_info.set_identifier_prefix(config.hash_prefix.as_deref());

    if config.hash_prefix.as_deref().is_some_and(|p| p.starts_with('$'))
        && !package_name.starts_with(FLAMEWORK_SCOPE)
    {
        return Err(Fatal::new("The hashPrefix $ is used internally by Flamework"));
    }

    // transformer.plugins: JavaScript plugins cannot run here; macro types are registered in the build's main.rs
    // (Q-F8; the macro types themselves are F2).
    if !config.plugins.is_empty() {
        let names: Vec<String> = config.plugins.iter().map(|p| format!("'{}'", p.path)).collect();
        cx.warn(
            &format!(
                "transformer.plugins ({}) is ignored: rbxts-rs does not load JavaScript plugins",
                names.join(", ")
            ),
            Some("Register macro types in the build's main.rs instead (flamework_rs::Flamework::macro_type)."),
        );
    }

    // Salt and seeds: the reused build info's, else new ones. The pinned seed pins new ones only: the seeds, and a salt
    // derived from it, so that a clean build with it equals an incremental build that started from one.
    let pinned = cx.env(FIXED_SEED_VAR);
    let pinned_error = |e: String| Fatal::new(format!("{FIXED_SEED_VAR} is not a uuid: {e}"));
    let salt = match (&config.salt, mode) {
        (Some(salt), _) => salt.clone(),
        (None, IdMode::Full) => String::new(),
        (None, _) => {
            if build_info.recorded_salt().is_none()
                && let Some(seed) = &pinned
            {
                build_info.set_salt(&pinned_salt(seed));
            }
            build_info.salt()
        }
    };
    let build_seed = if obfuscation {
        if build_info.recorded_build_seed().is_none()
            && let Some(seed) = &pinned
        {
            build_info.set_build_seed(seed).map_err(pinned_error)?;
        }
        Some(build_info.build_seed())
    } else {
        None
    };

    let hash_seed = match &build_seed {
        Some(seed) => seed.clone(),
        None => match build_info.recorded_build_seed().map(str::to_owned).or_else(|| state.hash_seed.clone()) {
            // Recorded by this plugin: a uuid (`find_problem` refuses a `buildSeed` that is not one).
            Some(kept) => uuid::Namespace::parse(&kept)
                .map_err(|e| Fatal::new(format!("flamework: the kept hash seed is not a uuid: {e}")))?,
            None => uuid::Namespace::parse(&pinned.unwrap_or_else(uuid::random_v4)).map_err(pinned_error)?,
        },
    };
    state.hash_seed = Some(hash_seed.as_str().to_owned());

    // The project's flamework.build came from another version: only a reused one can, and only one written by hand or
    // by another toolchain, since rbxts-rs builds clean when the plugin version changes.
    let out_relative = node_path::relative(&project_dir, &out_dir_option).replace('\\', "/");
    if let Err(lines) = build_info::check_project_version(&build_info, &out_relative, state_file.as_deref()) {
        let mut lines = lines.into_iter();
        let headline = lines.next().unwrap_or_default();
        return Err(Fatal::new(headline).with_hint(lines.collect::<Vec<_>>().join("\n")));
    }

    let src_dir = if root_dir_option.is_empty() { project_dir.clone() } else { root_dir_option };
    let root_dirs = root_dirs_option.unwrap_or_else(|| vec![src_dir]);

    let ids = Ids::new(IdConfig {
        mode,
        hash_prefix: config.hash_prefix.clone(),
        package_name: package_name.clone(),
        is_game,
        salt,
    });

    state.build_info = Some(build_info.data().clone());

    Ok(FwShared {
        project_dir,
        root_dir,
        package_name,
        is_game,
        root_dirs,
        include_dir,
        config,
        loaded,
        ids,
        build_info,
        build_seed,
        hash_seed,
        hash_used: AtomicBool::new(false),
        has_rojo,
        path_translator,
    })
}
