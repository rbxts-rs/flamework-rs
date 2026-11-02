//! `flamework.build` (the TS transformer's `classes/buildInfo.ts` and the build-info half of `transformState.ts`):
//! reading with the TS refusals, writing byte for byte as `JSON.stringify(info, undefined, "\t")` does, the package
//! build infos found above the program's files, and the version checks.
//!
//! The build info is kept as the JSON object itself, so a file read and written back keeps every byte, field order
//! included. A fresh build info gets its fields in the order the TS transformer adds them.

use std::collections::{HashMap, HashSet};

use crate::inputs::{FsInputs, Inputs};
use crate::json::{self, JsObject, JsValue};
use crate::packages::{FLAMEWORK_SCOPE, TRANSFORMER_PACKAGE};
use crate::{ProjectError, VERSION, node_path, schema, uuid};

/// The file name.
pub const BUILD_INFO_NAME: &str = "flamework.build";

/// One use of a glob in a file, kept so that a glob matching nothing can be reported where it is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobUse {
    /// The glob relative to the package root, as `paths` keys it.
    pub glob: String,
    /// The glob as written, which differs from `glob` for a relative (`./`) one.
    pub text: String,
    /// The macro the glob was given to, such as `registerProvidersGlob`.
    pub macro_name: String,
    /// Where the macro is called, one-based.
    pub line: u32,
    /// The column of the call, one-based.
    pub column: u32,
}

impl GlobUse {
    fn to_value(&self) -> JsValue {
        JsValue::Object(JsObject::from_iter([
            ("glob".to_owned(), JsValue::String(self.glob.clone())),
            ("text".to_owned(), JsValue::String(self.text.clone())),
            ("macro".to_owned(), JsValue::String(self.macro_name.clone())),
            ("line".to_owned(), JsValue::Number(self.line as f64)),
            ("column".to_owned(), JsValue::Number(self.column as f64)),
        ]))
    }
}

/// One use of a path macro in a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathUse {
    /// The source path as written, such as `src/server/services`.
    pub path: String,
    /// The call the path was given to: `registerProviders`, `ComponentPlugin.fromPath`.
    pub macro_name: String,
    /// The line of the call, one-based.
    pub line: u32,
    /// The column of the call, one-based.
    pub column: u32,
    /// Core's `requireModules`, which raises after five seconds rather than waiting.
    pub raises: bool,
}

impl PathUse {
    fn to_value(&self) -> JsValue {
        let mut o = JsObject::from_iter([
            ("path".to_owned(), JsValue::String(self.path.clone())),
            ("macro".to_owned(), JsValue::String(self.macro_name.clone())),
            ("line".to_owned(), JsValue::Number(self.line as f64)),
            ("column".to_owned(), JsValue::Number(self.column as f64)),
        ]);
        if self.raises {
            o.insert("raises", JsValue::Bool(true));
        }
        JsValue::Object(o)
    }
}

/// What is wrong with a `flamework.build`'s text, as a phrase (`it is empty`), or `None` when it is a build info.
pub fn find_problem(text: Option<&str>) -> Option<String> {
    let Some(text) = text else {
        return Some("it could not be read".into());
    };
    if crate::js::trim(text).is_empty() {
        return Some("it is empty".into());
    }
    let value = match json::parse(text) {
        Ok(v) => v,
        Err(e) => return Some(format!("it is not valid JSON ({e})")),
    };
    if let Err(errors) = schema::validate(
        schema::flamework_schema(),
        schema::build_info_schema(),
        &value,
    ) {
        let lines: Vec<String> = errors.iter().map(schema::SchemaError::describe).collect();
        return Some(format!(
            "it does not have the shape Flamework writes ({})",
            lines.join("; ")
        ));
    }
    // Not a TS refusal: the TS transformer reads any string and fails later, with uuid's bare "Invalid UUID", when an
    // obfuscated build derives a remote name from it. Only a hand-edited file has one.
    if let Some(seed) = value.get("buildSeed").and_then(JsValue::as_str)
        && !uuid::validate(seed)
    {
        let mut quoted = String::new();
        json::write_string(&mut quoted, seed);
        return Some(format!("its buildSeed {quoted} is not a uuid"));
    }
    None
}

/// A `flamework.build` (`BuildInfo`): the id table, salt, build seed, recorded inputs and glob/path uses, kept as
/// the JSON object it reads and writes, with the package build infos it reads ids from.
#[derive(Debug, Clone)]
pub struct BuildInfo {
    /// Where it is read from and written to.
    pub path: String,
    data: JsObject,
    /// The package build infos this one reads ids from (`addBuildInfo`).
    children: Vec<BuildInfo>,
}

impl BuildInfo {
    /// A new, empty build info: `{ version, flameworkVersion, identifiers }`.
    pub fn new(path: impl Into<String>) -> Self {
        let data = JsObject::from_iter([
            ("version".to_owned(), JsValue::Number(1.0)),
            (
                "flameworkVersion".to_owned(),
                JsValue::String(VERSION.to_owned()),
            ),
            ("identifiers".to_owned(), JsValue::Object(JsObject::new())),
        ]);
        BuildInfo {
            path: path.into(),
            data,
            children: Vec::new(),
        }
    }

    /// A build info from its parsed JSON, which should be one [`find_problem`] accepts. Anything but an object is
    /// taken as an empty build info, and a `buildSeed` that is not a uuid is dropped (a new one is made when needed).
    pub fn from_value(path: impl Into<String>, value: JsValue) -> Self {
        let JsValue::Object(mut data) = value else {
            return BuildInfo::new(path);
        };
        if data
            .get("buildSeed")
            .is_some_and(|s| !s.as_str().is_some_and(uuid::validate))
        {
            data.remove("buildSeed");
        }
        BuildInfo {
            path: path.into(),
            data,
            children: Vec::new(),
        }
    }

    /// Reads a `flamework.build`, or starts an empty one when there is no file. A file that cannot be used stops the
    /// build, naming it as `name` and saying what is wrong, followed by `remedy`.
    pub fn from_path(file: &str, name: &str, remedy: &[String]) -> Result<Self, ProjectError> {
        Self::from_path_with(&mut FsInputs, file, name, remedy)
    }

    /// [`BuildInfo::from_path`], reading through `inputs`.
    pub fn from_path_with(
        inputs: &mut dyn Inputs,
        file: &str,
        name: &str,
        remedy: &[String],
    ) -> Result<Self, ProjectError> {
        let Some(text) = inputs.read(file) else {
            return Ok(BuildInfo::new(file));
        };
        BuildInfo::from_text(file, &text, name, remedy)
    }

    /// A `flamework.build` from the text of an existing file (a byte order mark is dropped), refused as
    /// [`BuildInfo::from_path`] refuses it.
    pub fn from_text(file: &str, text: &str, name: &str, remedy: &[String]) -> Result<Self, ProjectError> {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        if let Some(problem) = find_problem(Some(text)) {
            let mut lines = vec![format!("Flamework cannot use {name}: {problem}.")];
            lines.extend(remedy.iter().cloned());
            return Err(ProjectError(lines.join("\n")));
        }
        Ok(BuildInfo::from_value(file, json::parse(text).unwrap_or(JsValue::Null)))
    }

    /// A build info from the object [`BuildInfo::data`] gave (the plugin's persistent state), without children.
    pub fn from_data(path: impl Into<String>, data: JsObject) -> Self {
        BuildInfo::from_value(path, JsValue::Object(data))
    }

    /// The project's own build info: the one in `directory`, else the one at its package root. `None` when neither
    /// exists.
    ///
    /// `ceiling`: the highest directory the package root is looked for in (`None`: up to the file system's root).
    pub fn from_directory(
        directory: &str,
        remedy: &[String],
        ceiling: Option<&str>,
    ) -> Result<Option<Self>, ProjectError> {
        Self::from_directory_with(&mut FsInputs, directory, remedy, ceiling)
    }

    /// [`BuildInfo::from_directory`], reading through `inputs`.
    pub fn from_directory_with(
        inputs: &mut dyn Inputs,
        directory: &str,
        remedy: &[String],
        ceiling: Option<&str>,
    ) -> Result<Option<Self>, ProjectError> {
        let read = |inputs: &mut dyn Inputs, file: &str| {
            let relative = node_path::relative_slash(directory, file);
            let name = if relative.is_empty() {
                file.to_owned()
            } else {
                relative
            };
            BuildInfo::from_path_with(inputs, file, &name, remedy)
        };
        let own = node_path::join(&[directory, BUILD_INFO_NAME]);
        if inputs.exists(&own) {
            return read(inputs, &own).map(Some);
        }
        if let Some(package_json) = find_package_json_with(inputs, directory, ceiling) {
            let candidate = node_path::join(&[&node_path::dirname(&package_json), BUILD_INFO_NAME]);
            if inputs.exists(&candidate) {
                return read(inputs, &candidate).map(Some);
            }
        }
        Ok(None)
    }

    /// The text `save` writes: `JSON.stringify(info, undefined, "\t")`, no final newline.
    pub fn to_json(&self) -> String {
        json::stringify(&JsValue::Object(self.data.clone()), Some("\t")).unwrap()
    }

    /// Writes [`BuildInfo::to_json`] to [`BuildInfo::path`].
    pub fn save(&self) -> std::io::Result<()> {
        std::fs::write(&self.path, self.to_json())
    }

    fn string_field(&self, key: &str) -> Option<&str> {
        self.data.get(key).and_then(JsValue::as_str)
    }

    /// The salt ids are hashed with, made (64 random bytes as hex) and kept on first use.
    pub fn salt(&mut self) -> String {
        if let Some(salt) = self.string_field("salt").filter(|s| !s.is_empty()) {
            return salt.to_owned();
        }
        let salt = crate::random::hex(64);
        self.data.insert("salt", JsValue::String(salt.clone()));
        salt
    }

    /// The recorded salt, if there is one.
    pub fn recorded_salt(&self) -> Option<&str> {
        self.string_field("salt").filter(|s| !s.is_empty())
    }

    /// Records the salt (a pinned one: `FLAMEWORK_RS_FIXED_SEED`, DESIGN §10.4).
    pub fn set_salt(&mut self, salt: &str) {
        self.data.insert("salt", JsValue::String(salt.to_owned()));
    }

    /// A random seed that lives as long as this build info does: obfuscated callsite uuids and string hashes are
    /// derived from it. Made (a random v4 uuid) on first use, and also when the recorded one is not a uuid.
    pub fn build_seed(&mut self) -> uuid::Namespace {
        if let Some(seed) = self.string_field("buildSeed")
            && let Ok(namespace) = uuid::Namespace::parse(seed)
        {
            return namespace;
        }
        let seed = uuid::random_v4();
        self.data.insert("buildSeed", JsValue::String(seed.clone()));
        uuid::Namespace::parse(&seed)
            .unwrap_or_else(|_| unreachable!("random_v4 makes a valid uuid"))
    }

    /// The recorded build seed, if there is one.
    pub fn recorded_build_seed(&self) -> Option<&str> {
        self.string_field("buildSeed")
    }

    /// Pins the build seed (a test or `FLAMEWORK_RS_FIXED_SEED` build, DESIGN §10.4), or records the seed
    /// `Modding.Hash` derived from (outside obfuscation, once a file has used one). Refuses one that is not a uuid,
    /// with npm uuid's message.
    pub fn set_build_seed(&mut self, seed: &str) -> Result<(), String> {
        uuid::Namespace::parse(seed)?;
        self.data
            .insert("buildSeed", JsValue::String(seed.to_owned()));
        Ok(())
    }

    /// The version of Flamework that wrote this build info.
    pub fn flamework_version(&self) -> &str {
        self.string_field("flameworkVersion").unwrap_or("")
    }

    /// A new build info at the same path, with the same package build infos: what a clean build would have.
    pub fn renewed(&self) -> BuildInfo {
        let mut renewed = BuildInfo::new(self.path.clone());
        renewed.children = self.children.clone();
        renewed
    }

    /// Registers a package build info, which ids are read from.
    pub fn add_build_info(&mut self, child: BuildInfo) {
        self.children.push(child);
    }

    /// The package build infos registered with [`BuildInfo::add_build_info`], in registration order.
    pub fn children(&self) -> &[BuildInfo] {
        &self.children
    }

    fn identifiers(&self) -> &JsObject {
        self.data
            .get("identifiers")
            .and_then(JsValue::as_object)
            .expect("identifiers")
    }

    fn identifiers_mut(&mut self) -> &mut JsObject {
        self.data
            .get_mut("identifiers")
            .and_then(JsValue::as_object_mut)
            .expect("identifiers")
    }

    /// Records an identifier. Rewriting one is a bug (the TS transformer throws a plain `Error`).
    pub fn add_identifier(&mut self, internal_id: &str, id: &str) -> Result<(), String> {
        if let Some(existing) = self.identifier_from_internal(internal_id) {
            return Err(format!(
                "Attempt to rewrite identifier {internal_id} -> {id} (from {existing})"
            ));
        }
        self.identifiers_mut()
            .insert(internal_id, JsValue::String(id.to_owned()));
        Ok(())
    }

    /// The id recorded for an internal id, here or in a package build info. An empty id counts as none.
    pub fn identifier_from_internal(&self, internal_id: &str) -> Option<String> {
        if let Some(id) = self
            .identifiers()
            .get(internal_id)
            .and_then(JsValue::as_str)
            .filter(|s| !s.is_empty())
        {
            return Some(id.to_owned());
        }
        self.children
            .iter()
            .find_map(|c| c.identifier_from_internal(internal_id))
    }

    /// The package build info whose directory holds `file`.
    pub fn build_info_from_file(&self, file: &str) -> Option<&BuildInfo> {
        self.children
            .iter()
            .find(|b| node_path::is_path_descendant_of(file, &node_path::dirname(&b.path)))
    }

    /// The package build info with this identifier prefix, searched depth first.
    pub fn build_info_from_prefix(&self, prefix: &str) -> Option<&BuildInfo> {
        for child in &self.children {
            if child.identifier_prefix() == Some(prefix) {
                return Some(child);
            }
            if let Some(found) = child.build_info_from_prefix(prefix) {
                return Some(found);
            }
        }
        None
    }

    /// `metadata[key]`.
    pub fn metadata(&self, key: &str) -> Option<&JsValue> {
        self.data.get("metadata")?.get(key)
    }

    /// `metadata[key] = value`.
    pub fn set_metadata(&mut self, key: &str, value: JsValue) {
        let metadata = self
            .data
            .entry_or_insert("metadata", || JsValue::Object(JsObject::new()));
        metadata.as_object_mut().unwrap().insert(key, value);
    }

    fn metadata_section(&mut self, key: &str) -> &mut JsObject {
        let metadata = self
            .data
            .entry_or_insert("metadata", || JsValue::Object(JsObject::new()));
        metadata
            .as_object_mut()
            .unwrap()
            .entry_or_insert(key, || JsValue::Object(JsObject::new()))
            .as_object_mut()
            .unwrap()
    }

    /// Every package build info's `metadata[key]`, by identifier prefix, children's children included (a later one
    /// with the same prefix replaces the value in the earlier one's place, as a `Map` does).
    pub fn children_metadata(&self, key: &str) -> Vec<(String, JsValue)> {
        let mut out: Vec<(String, JsValue)> = Vec::new();
        let mut set = |k: String, v: JsValue| match out.iter_mut().find(|(e, _)| *e == k) {
            Some(entry) => entry.1 = v,
            None => out.push((k, v)),
        };
        for child in &self.children {
            let (Some(prefix), Some(metadata)) = (child.identifier_prefix(), child.metadata(key))
            else {
                continue;
            };
            set(prefix.to_owned(), metadata.clone());
            for (k, v) in child.children_metadata(key) {
                set(k, v);
            }
        }
        out
    }

    /// Records a glob used in `origin`, with where it is used.
    pub fn add_glob(&mut self, glob: &str, origin: &str, glob_use: Option<&GlobUse>) {
        let globs = self.metadata_section("globs");
        let paths = globs
            .entry_or_insert("paths", || JsValue::Object(JsObject::new()))
            .as_object_mut()
            .unwrap();
        paths.insert(glob, JsValue::Array(Vec::new()));
        let origins = globs
            .entry_or_insert("origins", || JsValue::Object(JsObject::new()))
            .as_object_mut()
            .unwrap();
        let list = origins.entry_or_insert(origin, || JsValue::Array(Vec::new()));
        if let JsValue::Array(list) = list {
            list.push(JsValue::String(glob.to_owned()));
        }
        if let Some(glob_use) = glob_use {
            let uses = globs
                .entry_or_insert("uses", || JsValue::Object(JsObject::new()))
                .as_object_mut()
                .unwrap();
            if let JsValue::Array(list) =
                uses.entry_or_insert(origin, || JsValue::Array(Vec::new()))
            {
                list.push(glob_use.to_value());
            }
        }
    }

    /// Records a use of a path macro in `origin`.
    pub fn add_path_use(&mut self, origin: &str, path_use: &PathUse) {
        let paths = self.metadata_section("paths");
        let uses = paths
            .entry_or_insert("uses", || JsValue::Object(JsObject::new()))
            .as_object_mut()
            .unwrap();
        if let JsValue::Array(list) = uses.entry_or_insert(origin, || JsValue::Array(Vec::new())) {
            list.push(path_use.to_value());
        }
    }

    /// Forgets the path uses of a file about to be compiled again.
    pub fn invalidate_path_uses(&mut self, origin: &str) {
        if let Some(uses) = self
            .data
            .get_mut("metadata")
            .and_then(|m| m.as_object_mut())
            .and_then(|m| m.get_mut("paths"))
            .and_then(|p| p.as_object_mut())
            .and_then(|p| p.get_mut("uses"))
            .and_then(|u| u.as_object_mut())
        {
            uses.remove(origin);
        }
    }

    /// Forgets the globs of a file about to be compiled again, and every glob no file uses any more.
    pub fn invalidate_globs(&mut self, origin: &str) {
        let Some(globs) = self
            .data
            .get_mut("metadata")
            .and_then(|m| m.as_object_mut())
            .and_then(|m| m.get_mut("globs"))
            .and_then(|g| g.as_object_mut())
        else {
            return;
        };
        if let Some(uses) = globs.get_mut("uses").and_then(JsValue::as_object_mut) {
            uses.remove(origin);
        }
        if globs.contains_key("paths") && globs.contains_key("origins") {
            let origins = globs
                .get_mut("origins")
                .and_then(JsValue::as_object_mut)
                .unwrap();
            origins.remove(origin);
            let used: Vec<String> = origins
                .iter()
                .flat_map(|(_, list)| {
                    list.as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(JsValue::as_str)
                })
                .map(str::to_owned)
                .collect();
            let paths = globs
                .get_mut("paths")
                .and_then(JsValue::as_object_mut)
                .unwrap();
            let unused: Vec<String> = paths
                .keys()
                .filter(|p| !used.contains(p))
                .cloned()
                .collect();
            for path in unused {
                paths.remove(&path);
            }
        }
    }

    /// The globs recorded here: `metadata.globs.paths` keys, in order.
    pub fn glob_patterns(&self) -> Vec<String> {
        self.metadata("globs")
            .and_then(|g| g.get("paths"))
            .and_then(JsValue::as_object)
            .map(|p| p.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Sets what a glob matched (`calculateGlobs`): paths relative to the package root, with forward slashes.
    pub fn set_glob_results(&mut self, glob: &str, results: Vec<String>) {
        if let Some(paths) = self
            .metadata_section("globs")
            .get_mut("paths")
            .and_then(JsValue::as_object_mut)
        {
            paths.insert(
                glob,
                JsValue::Array(results.into_iter().map(JsValue::String).collect()),
            );
        }
    }

    /// The next id for sequential generation: the number of ids in the table, plus one.
    pub fn latest_id(&self) -> u64 {
        self.identifiers().len() as u64 + 1
    }

    /// An obfuscated string's replacement: a uuid v5 of `context:text` in the build seed's namespace (DESIGN §10.4).
    /// The TS transformer made a random v4 per `(context, text)` and kept them in `stringHashes`; derived hashes
    /// are as unpredictable across clean builds, need no table, and agree on every checker. `context` defaults to
    /// `"@"`, as in the TS.
    pub fn hash_string(&mut self, text: &str, context: Option<&str>) -> String {
        let context = context.unwrap_or("@");
        self.build_seed().v5(&format!("{context}:{text}"))
    }

    /// `identifierPrefix`, kept in its place even when set to `undefined` (a game without one).
    pub fn set_identifier_prefix(&mut self, prefix: Option<&str>) {
        self.data.insert(
            "identifierPrefix",
            prefix.map_or(JsValue::Undefined, |p| JsValue::String(p.to_owned())),
        );
    }

    /// The recorded `identifierPrefix`; `None` when it is unset or empty.
    pub fn identifier_prefix(&self) -> Option<&str> {
        self.string_field("identifierPrefix")
            .filter(|s| !s.is_empty())
    }

    /// Records the id generation mode. The identifier table is kept even when the mode differs from the recorded one
    /// (`setIdGenerationMode` since 1a72fa1): a table of another mode is never kept, as setup starts a new one first
    /// ([`changed_inputs`]; the TS transformer refuses it in `refuseChangedInputs`).
    pub fn set_id_generation_mode(&mut self, mode: &str) {
        self.data
            .insert("idGenerationMode", JsValue::String(mode.to_owned()));
    }

    /// The recorded id generation mode (`idGenerationMode`), if there is one.
    pub fn id_generation_mode(&self) -> Option<&str> {
        self.string_field("idGenerationMode")
    }

    fn inputs_section(&mut self) -> &mut JsObject {
        self.data
            .entry_or_insert("inputs", || JsValue::Object(JsObject::new()))
            .as_object_mut()
            .expect("inputs is an object")
    }

    /// `inputs.config`: the hash of the code-shaping options ([`crate::config::fingerprint_emit_config`]).
    pub fn set_config_input(&mut self, hash: &str) {
        self.inputs_section()
            .insert("config", JsValue::String(hash.to_owned()));
    }

    /// The recorded `inputs.config`, if any.
    pub fn config_input(&self) -> Option<&str> {
        self.data.get("inputs")?.get("config")?.as_str()
    }

    /// Records that `Flamework.env` inlined `name` in the file `origin`, with the hash of its value
    /// ([`crate::env::hash_env_value`]): `inputs.env[origin][name]`.
    pub fn add_env_input(&mut self, origin: &str, name: &str, hash: &str) {
        let env = self
            .inputs_section()
            .entry_or_insert("env", || JsValue::Object(JsObject::new()))
            .as_object_mut()
            .expect("inputs.env is an object");
        env.entry_or_insert(origin, || JsValue::Object(JsObject::new()))
            .as_object_mut()
            .expect("inputs.env[origin] is an object")
            .insert(name, JsValue::String(hash.to_owned()));
    }

    /// `inputs.env`: per file, each variable `Flamework.env` inlined there with the hash of its value, in the
    /// recorded order. For tests and tools: setup no longer compares the recorded variables (a changed one is a build
    /// input of the files that read it, so the compiler transforms them again).
    pub fn env_inputs(&self) -> Vec<(String, Vec<(String, String)>)> {
        let Some(env) = self.data.get("inputs").and_then(|i| i.get("env")).and_then(JsValue::as_object) else {
            return Vec::new();
        };
        env.iter()
            .map(|(origin, uses)| {
                let uses = uses
                    .as_object()
                    .into_iter()
                    .flat_map(JsObject::iter)
                    .filter_map(|(name, hash)| Some((name.clone(), hash.as_str()?.to_owned())))
                    .collect();
                (origin.clone(), uses)
            })
            .collect()
    }

    /// Forgets what `finish` makes again from the file parts of every current file: `metadata` (globs and path uses)
    /// and `inputs.env`. A reused build info keeps its keys in their places otherwise; `metadata` comes back last.
    pub fn forget_file_records(&mut self) {
        self.data.remove("metadata");
        if let Some(inputs) = self.data.get_mut("inputs").and_then(JsValue::as_object_mut) {
            inputs.remove("env");
        }
    }

    /// The whole object, for anything this API does not cover.
    pub fn data(&self) -> &JsObject {
        &self.data
    }
}

/// Whether an upward scan at `current` may go on to its parent: not past `ceiling` (nor outside it).
fn below_ceiling(current: &str, ceiling: Option<&str>) -> bool {
    ceiling.is_none_or(|c| {
        !are_paths_equal(current, c) && node_path::is_path_descendant_of(current, c)
    })
}

/// The nearest `package.json` from `directory` up (`ts.findPackageJson`), looking no higher than `ceiling`.
pub fn find_package_json(directory: &str, ceiling: Option<&str>) -> Option<String> {
    find_package_json_with(&mut FsInputs, directory, ceiling)
}

/// [`find_package_json`], reading through `inputs`.
pub fn find_package_json_with(
    inputs: &mut dyn Inputs,
    directory: &str,
    ceiling: Option<&str>,
) -> Option<String> {
    let mut current = node_path::resolve(&[directory]);
    loop {
        let candidate = node_path::join(&[&current, "package.json"]);
        if inputs.exists(&candidate) {
            return Some(candidate);
        }
        let parent = node_path::dirname(&current);
        if parent == current || !below_ceiling(&current, ceiling) {
            return None;
        }
        current = parent;
    }
}

/// The name in the `package.json` nearest to `file`, if it names a package.
pub fn package_name_of(file: &str, ceiling: Option<&str>) -> Option<String> {
    package_name_of_with(&mut FsInputs, file, ceiling)
}

/// [`package_name_of`], reading through `inputs`.
pub fn package_name_of_with(
    inputs: &mut dyn Inputs,
    file: &str,
    ceiling: Option<&str>,
) -> Option<String> {
    let package_json = find_package_json_with(inputs, &node_path::dirname(file), ceiling)?;
    let text = inputs.read(&package_json)?;
    json::parse(text.strip_prefix('\u{feff}').unwrap_or(&text))
        .ok()?
        .get("name")?
        .as_str()
        .map(str::to_owned)
}

/// Finds the package build infos above the program's files (`findCandidateUpper`): from each file's directory,
/// the first `flamework.build` in it or in one of its four parents. One cached answer per directory.
///
/// The project's own `flamework.build` ([`CandidateScan::candidates`]' `own_path`) is taken as there without being
/// read. It is the plugin's state file, which `Shared` may not depend on (`rbxts_plugin::Plugin::Shared`), and a
/// recorded read of it would make the build after the one that first writes it run setup and transform every file
/// again. It is there in every build but a project's first (and one after it was deleted), where TS's
/// `fs.existsSync` lets the scan go on above the project directory.
///
/// With the program's package directories ([`CandidateScan::look_only_in_packages`]), the scan looks only in those
/// (TS looks in every directory on the way; each look is a recorded input of setup, and a directory the watcher
/// watches). A package's build writes its `flamework.build` beside its `package.json` (`currentDirectory`), and the
/// build info is the package's (`packageNameOf` names it from the nearest `package.json`), so a package directory is
/// where one can be: the candidates are TS's but for a `flamework.build` in a package's subfolder.
#[derive(Debug, Default)]
pub struct CandidateScan {
    cache: HashMap<String, Option<String>>,
    ceiling: Option<String>,
    own: Option<String>,
    packages: Option<HashSet<String>>,
}

/// A directory as [`are_paths_equal`] compares it.
fn path_key(path: &str) -> String {
    if cfg!(windows) { node_path::normalize(&path.to_lowercase()) } else { node_path::normalize(path) }
}

impl CandidateScan {
    /// A scan with no ceiling (up to the file system's root) and an empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// A scan that looks no higher than `ceiling` (tests keep it inside a folder of their own).
    pub fn with_ceiling(ceiling: &str) -> Self {
        CandidateScan {
            cache: HashMap::new(),
            ceiling: Some(ceiling.to_owned()),
            own: None,
            packages: None,
        }
    }

    /// Makes the scan look only in these directories: the package directories of the program's files (each file's
    /// nearest `package.json`, which the compiler finds as it loads the program). Without it, every directory.
    pub fn look_only_in_packages<'a>(&mut self, package_directories: impl IntoIterator<Item = &'a str>) {
        self.packages = Some(package_directories.into_iter().map(path_key).collect());
    }

    /// The first `flamework.build` in `start` or up to `depth` parents of it, among the directories the scan looks in
    /// ([`CandidateScan::look_only_in_packages`]).
    pub fn find_upper(&mut self, start: &str, depth: u32) -> Option<String> {
        self.find_upper_with(&mut FsInputs, start, depth)
    }

    /// [`CandidateScan::find_upper`], reading through `inputs`.
    pub fn find_upper_with(
        &mut self,
        inputs: &mut dyn Inputs,
        start: &str,
        depth: u32,
    ) -> Option<String> {
        let found = match self.cache.get(start) {
            Some(cached) => cached.clone(),
            None => {
                let candidate = node_path::join(&[start, BUILD_INFO_NAME]);
                let own = self.own.as_deref().is_some_and(|own| are_paths_equal(&candidate, own));
                let looked_in = self.packages.as_ref().is_none_or(|packages| packages.contains(&path_key(start)));
                let found = (own || (looked_in && inputs.exists(&candidate))).then_some(candidate);
                self.cache.insert(start.to_owned(), found.clone());
                found
            }
        };
        if found.is_some() {
            return found;
        }
        let parent = node_path::dirname(start);
        if depth > 0 && parent != start && below_ceiling(start, self.ceiling.as_deref()) {
            return self.find_upper_with(inputs, &parent, depth - 1);
        }
        None
    }

    /// The candidates for a program, in its files' order, without duplicates and without the project's own
    /// (`own_path`, compared as `arePathsEqual` does).
    pub fn candidates<'a>(
        &mut self,
        file_directories: impl IntoIterator<Item = &'a str>,
        own_path: &str,
    ) -> Vec<String> {
        self.candidates_with(&mut FsInputs, file_directories, own_path)
    }

    /// [`CandidateScan::candidates`], reading through `inputs`.
    pub fn candidates_with<'a>(
        &mut self,
        inputs: &mut dyn Inputs,
        file_directories: impl IntoIterator<Item = &'a str>,
        own_path: &str,
    ) -> Vec<String> {
        self.own = Some(own_path.to_owned());
        let mut out: Vec<String> = Vec::new();
        for directory in file_directories {
            let Some(candidate) = self.find_upper_with(inputs, directory, 4) else {
                continue;
            };
            if !are_paths_equal(&candidate, own_path) && !out.contains(&candidate) {
                out.push(candidate);
            }
        }
        out
    }
}

/// `arePathsEqual`: normalised, and without case on Windows.
pub fn are_paths_equal(a: &str, b: &str) -> bool {
    path_key(a) == path_key(b)
}

/// What to do when a package's `flamework.build` cannot be used.
pub fn package_build_info_remedy(file: &str, ceiling: Option<&str>) -> Vec<String> {
    package_build_info_remedy_with(&mut FsInputs, file, ceiling)
}

/// [`package_build_info_remedy`], reading through `inputs`.
pub fn package_build_info_remedy_with(
    inputs: &mut dyn Inputs,
    file: &str,
    ceiling: Option<&str>,
) -> Vec<String> {
    let owner = match package_name_of_with(inputs, file, ceiling) {
        Some(name) => format!("the package {name}"),
        None => "the package it is in".to_owned(),
    };
    vec![format!(
        "It came with {owner}, which wrote it when it was built, and this build reads it for the ids of that package's classes. Reinstall the package, or build it again if it is your own."
    )]
}

/// A Flamework package built by another version of the transformer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MixedVersion {
    /// The package's name.
    pub name: String,
    /// The Flamework version its `flamework.build` records (`flameworkVersion`).
    pub version: String,
    /// The `flamework.build`, relative to the project, with forward slashes.
    pub file: String,
}

/// Refuses Flamework's own packages built by another version (`refuseMixedVersions`). Packages of other scopes may
/// come from any 2.x.
pub fn refuse_mixed_versions(packages: &[MixedVersion]) -> Result<(), ProjectError> {
    if packages.is_empty() {
        return Ok(());
    }
    let mut versions: Vec<&str> = Vec::new();
    for p in packages {
        if !versions.contains(&p.version.as_str()) {
            versions.push(&p.version);
        }
    }
    let one = packages.len() == 1;
    let mut lines = vec![format!(
        "Flamework's packages are not all the same version: {TRANSFORMER_PACKAGE} is {VERSION}, and {} built by another version.",
        if one {
            "this package was"
        } else {
            "these packages were"
        }
    )];
    lines.extend(
        packages
            .iter()
            .map(|p| format!("  {} {} ({})", p.name, p.version, p.file)),
    );
    lines.push(format!(
        "Install the same version of every {FLAMEWORK_SCOPE} package: update {} to {VERSION}, or install {TRANSFORMER_PACKAGE}@{} instead, then build again.",
        if one { "it" } else { "them" },
        if versions.len() == 1 { versions[0] } else { "<their version>" }
    ));
    Err(ProjectError(lines.join("\n")))
}

/// Reads the package build infos (`setupBuildInfo`'s second half): each candidate, refused when it cannot be used,
/// with Flamework's own packages checked for their version. Returns them in order, with the verbose log lines.
///
/// `ceiling`: the highest directory a candidate's `package.json` is looked for in (`None`: no limit).
pub fn load_package_build_infos(
    candidates: &[String],
    current_directory: &str,
    ceiling: Option<&str>,
) -> Result<(Vec<BuildInfo>, Vec<String>), ProjectError> {
    load_package_build_infos_with(&mut FsInputs, candidates, current_directory, ceiling)
}

/// [`load_package_build_infos`], reading through `inputs`.
pub fn load_package_build_infos_with(
    inputs: &mut dyn Inputs,
    candidates: &[String],
    current_directory: &str,
    ceiling: Option<&str>,
) -> Result<(Vec<BuildInfo>, Vec<String>), ProjectError> {
    let mut infos = Vec::new();
    let mut log = Vec::new();
    let mut mixed = Vec::new();
    for candidate in candidates {
        let relative = node_path::relative_slash(current_directory, candidate);
        let remedy = package_build_info_remedy_with(inputs, candidate, ceiling);
        let info = BuildInfo::from_path_with(inputs, candidate, &relative, &remedy)?;
        log.push(format!(
            "Loaded buildInfo at {relative}, next id: {}",
            info.latest_id()
        ));
        if let Some(name) = package_name_of_with(inputs, candidate, ceiling)
            && name.starts_with(&format!("{FLAMEWORK_SCOPE}/"))
            && info.flamework_version() != VERSION
        {
            mixed.push(MixedVersion {
                name,
                version: info.flamework_version().to_owned(),
                file: relative.clone(),
            });
        }
        infos.push(info);
    }
    refuse_mixed_versions(&mixed)?;
    Ok((infos, log))
}

/// The project's `flamework.build` came from another version (`transformer.ts`): the lines to print, the first in
/// red. Only a build that reuses the previous `flamework.build` can meet one (rbxts-rs starts a clean build itself
/// when the plugin version changes, so in practice a file put there by hand or by another toolchain).
///
/// `out_directory` is the output directory relative to the project, with forward slashes; `state_file` is the
/// compiler's incremental state file (`SetupCx::incremental_state_path`, relative to the project, with forward
/// slashes) of an incremental project (TS: `getTsBuildInfoPath(options) !== undefined`), which picks the remedy.
pub fn check_project_version(
    build_info: &BuildInfo,
    out_directory: &str,
    state_file: Option<&str>,
) -> Result<(), Vec<String>> {
    let previous = build_info.flamework_version();
    if previous == VERSION {
        return Ok(());
    }
    Err(vec![
        "Project was compiled on different version of Flamework.".to_owned(),
        match state_file {
            Some(name) => format!(
                "This is an incremental build, which recompiles only the files that changed. Delete {name} and build again: the next build compiles every file."
            ),
            None => format!("Please recompile by deleting the {out_directory} directory"),
        },
        format!("Current Flamework Version: {VERSION}"),
        format!("Previous Flamework Version: {previous}"),
    ])
}

/// `ownBuildInfoRemedy`: what to do when the project's own `flamework.build`, read to keep the ids of the files a
/// build does not transform, cannot be used. `state_file`: the compiler's incremental state file
/// (`SetupCx::incremental_state_path`, relative to the project, with forward slashes).
pub fn own_build_info_remedy(state_file: Option<&str>) -> Vec<String> {
    vec![match state_file {
        Some(name) => format!(
            "This incremental build reads it to keep the ids of the files it does not recompile. Delete {name} and build again: a full build does not read flamework.build, and writes a new one."
        ),
        None => "This watcher reads it on every rebuild, to keep the ids of the files it does not recompile. Restart the watcher: a full build does not read flamework.build, and writes a new one.".to_owned(),
    }]
}

/// The stop of an incremental build that has no `flamework.build` for the files it does not transform
/// (`setupBuildInfo`, said as the version check says it): the headline (red) and the remedy. `state_file`:
/// the compiler's incremental state file (`SetupCx::incremental_state_path`, relative to the project, with forward
/// slashes; `"the tsbuildinfo"` without one, as the TS names it).
pub fn dirty_environment(state_file: Option<&str>) -> Vec<String> {
    vec![
        "Flamework cannot be built in a dirty environment.".to_owned(),
        format!(
            "This incremental build has no flamework.build to match the files it does not recompile. Delete {} and build again.",
            state_file.unwrap_or("the tsbuildinfo")
        ),
    ]
}

/// The inputs a reused `flamework.build`'s table was made from that differ from this build's, as `refuseChangedInputs`
/// names them: the id generation mode (`mode`), else the code-shaping options of flamework.config.json (`config_input`,
/// [`crate::config::fingerprint_emit_config`]). Empty when they agree, when the table did not record them, and for a
/// table of another version (left to [`check_project_version`]).
///
/// **Not a refusal, unlike TS.** The TS transformer refuses such a table because "roblox-ts chooses which files to
/// recompile before it loads the transformer, and nothing in these inputs is part of what it compares, so a full
/// recompile cannot be started from here" (`transformState.ts`, `refuseChangedInputs`). Under rbxts-rs both are
/// build inputs of setup (the config file and `.env` files through `SetupCx::read_input`, variables through
/// `SetupCx::env`): a change makes the compiler transform every file, so setup starts a new `flamework.build`
/// instead, as a clean build does (`crate::setup`).
///
/// TS's third input, the variables `Flamework.env` inlined, is not compared: their values change no table entry, and
/// the compiler transforms again exactly the files that read a changed one (`Cx::env` reads are recorded per file),
/// whose new parts make `inputs.env` again.
pub fn changed_inputs(build_info: &BuildInfo, mode: &str, config_input: &str) -> Vec<String> {
    if build_info.flamework_version() != VERSION {
        return Vec::new();
    }
    let mut changes = Vec::new();
    if let Some(recorded) = build_info.id_generation_mode().filter(|recorded| *recorded != mode) {
        changes.push(format!("idGenerationMode is '{mode}', and was '{recorded}'"));
    } else if build_info.config_input().is_some_and(|recorded| recorded != config_input) {
        changes.push(
            "an option in flamework.config.json that shapes the emitted code changed (the transformer or serialization section, or networking.serialization)"
                .to_owned(),
        );
    }
    changes
}
