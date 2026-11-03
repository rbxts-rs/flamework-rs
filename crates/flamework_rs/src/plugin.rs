//! The `Flamework` plugin (DESIGN §10.1): the factory, the per-checker instance and the per-file part.
//!
//! - [`Flamework`]: `setup` ([`crate::setup`]) builds the read-only [`FwShared`] from the persistent [`FwState`];
//!   `finish` ([`crate::finish`]) assigns the new ids in transform order (roblox-ts's `getChangedFilePaths` order),
//!   resolves the late ids, expands the globs and queues the include JSON and `flamework.build`, which it leaves in
//!   the state too.
//! - [`FwInstance`], one per checker: the class hook ([`crate::class_hook`]), the call hook for user macros
//!   ([`crate::macros`]), and the assignment and property-key hooks of components ([`crate::components`]), with this
//!   checker's metadata and decorator caches.
//! - [`FwFilePart`], one per file: the ids it asked for first, its glob and path uses, and the variables it inlined.
//!   Only parts cross threads, so the output is the same for any checker count (DESIGN §10.4).
//!
//! The persistent state ([`FwState`]) is the last committed `flamework.build`: a clean build starts a new one, every
//! other build reuses it (its id table, salt and seeds), unless the id mode or the code-shaping config changed, when
//! it starts a new one where the TS transformer refuses the build ([`crate::setup`]).

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

use rbxts_plugin::ts::{Node, P, Symbol};
use rbxts_plugin::{
    AccessSite, AssignSite, CallSite, ChangeSet, CheckerInfo, Claim, ClassFilter, ClassSite, Cx, Fatal, FileIndex,
    FinishCx, Interest, KeyAnswer, LuauExpr, LuauStmt, PResult, Plugin, PluginInfo, PluginInstance, RebuildPlan,
    SetupCx, SymbolRole, Tag,
};

use crate::build_info::{GlobUse, PathUse};
use crate::ids::NewId;
use crate::macro_type::{MacroType, MacroTypes};
use crate::metadata::NodeMetadata;
pub use crate::setup::{FwShared, FwState};

/// The tag of a call whose callee is a user macro (`@metadata macro`).
pub const TAG_MACRO: Tag = Tag(1);

/// The tsconfig `plugins` entry that turns Flamework on.
pub const TSCONFIG_TRANSFORM: &str = "@flamework-experimental/transformer";

/// Flamework's transformer, compiled in, with the user's macro types ([`crate::macro_type`]).
#[derive(Debug, Default)]
pub struct Flamework {
    macro_types: Arc<MacroTypes>,
}

impl Flamework {
    /// The plugin with no macro types.
    pub fn new() -> Flamework {
        Flamework::default()
    }

    /// Registers a macro type under `id` (`registerMacroType`), for `Modding.Intrinsic<"plugin", [id, T], R>`
    /// parameters. Panics when `id` is registered already, as the TS plugin host refuses it.
    pub fn macro_type(mut self, id: impl Into<String>, macro_type: impl MacroType) -> Flamework {
        let types = Arc::make_mut(&mut self.macro_types);
        if let Err(message) = types.insert(id.into(), Arc::new(macro_type)) {
            panic!("{message}");
        }
        self
    }

    /// The registered macro types.
    pub fn macro_types(&self) -> &MacroTypes {
        &self.macro_types
    }
}

/// A glob or path use, in the order the file's transform met them (they key `metadata.globs` and
/// `metadata.paths` in `flamework.build` in first-use order).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathEvent {
    /// A `pathglob` intrinsic: the glob relative to the package root, and where it was written.
    Glob(GlobUse),
    /// A `path` intrinsic.
    Path(PathUse),
}

/// What one file contributes to `finish` (DESIGN §10.1). Owned data only: no handles, no `FileIndex`. Persisted for
/// incremental builds by `crate::part_codec` (bump its format version when a field changes).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FwFilePart {
    /// The file relative to the package root (`getFileId`).
    pub(crate) file_id: String,
    /// Ids the file asked for that the build info did not have, in first-request order within the file.
    pub(crate) new_ids: Vec<NewId>,
    /// The late keys the file used (internal ids), for the late-string contract.
    pub(crate) late_keys: Vec<String>,
    /// The glob and path uses, in the order the hooks met them.
    pub(crate) events: Vec<PathEvent>,
    /// The variables `Flamework.env` inlined, in the order the hooks read them: `(name, hash of the value)`
    /// (`inputs.env` in `flamework.build`).
    pub(crate) env_inputs: Vec<(String, String)>,
}

/// The per-file state of an instance, reset in `begin_file`. Per file only: a cache across files would make the
/// parts depend on which checker saw a file first.
#[derive(Debug, Default)]
pub(crate) struct FileState {
    pub part: FwFilePart,
    pub late: HashSet<String>,
    /// Every new id asked for, in request order; deduplicated into `part.new_ids` by `end_file`.
    pub id_requests: Vec<NewId>,
    /// Macro type results of this file, by `(id, type)` (`executeMacroType`'s per-file cache).
    pub macro_cache: HashMap<(String, P<rbxts_plugin::ts::Type>), LuauExpr>,
    /// How many shuffles each macro call made (`crate::macros::shuffle`).
    pub shuffles: HashMap<P<Node>, usize>,
}

/// One checker's instance.
pub struct FwInstance {
    pub(crate) shared: Arc<FwShared>,
    /// `NodeMetadata.fromCache`: per checker, by node.
    pub(crate) metadata: RefCell<HashMap<P<Node>, Rc<NodeMetadata>>>,
    /// `getFlameworkDecorators`: per checker, by class declaration.
    pub(crate) decorators: RefCell<HashMap<P<Node>, crate::class_hook::DecoratorKinds>>,
    /// `isUserMacro`, keyed by symbol: kept by the compiler's classification memo too, this one serves the class
    /// hook.
    pub(crate) macro_symbols: RefCell<HashMap<P<Symbol>, bool>>,
    pub(crate) file: RefCell<FileState>,
    pub(crate) macro_types: Arc<MacroTypes>,
    /// The serializer generator of the current file ([`crate::serializer`]).
    pub(crate) serializer: RefCell<crate::serializer::FileSerializer>,
}

impl FwInstance {
    /// The metadata of a node, cached on this checker.
    pub(crate) fn metadata(&self, cx: &mut Cx<'_>, node: P<Node>) -> Rc<NodeMetadata> {
        if let Some(found) = self.metadata.borrow().get(&node) {
            return found.clone();
        }
        let parsed = Rc::new(NodeMetadata::parse(cx, node));
        self.metadata.borrow_mut().insert(node, parsed.clone());
        parsed
    }

    /// `isUserMacro`: a declaration of the symbol requests `macro`.
    pub(crate) fn is_user_macro(&self, cx: &mut Cx<'_>, symbol: P<Symbol>) -> bool {
        if let Some(&known) = self.macro_symbols.borrow().get(&symbol) {
            return known;
        }
        let is_macro = symbol
            .declarations()
            .iter()
            .any(|&declaration| self.metadata(cx, declaration).is_requested("macro"));
        self.macro_symbols.borrow_mut().insert(symbol, is_macro);
        is_macro
    }

}

impl Plugin for Flamework {
    type Shared = FwShared;
    type State = FwState;
    type FilePart = FwFilePart;
    type Instance = FwInstance;

    fn info(&self) -> PluginInfo {
        PluginInfo {
            name: "flamework",
            version: crate::VERSION,
            tsconfig_transform: Some(TSCONFIG_TRANSFORM),
        }
    }

    /// The TS transformer's diagnostics carry the code `` ` ${TRANSFORMER_PACKAGE}` `` (`error TS
    /// @flamework-experimental/transformer: …`).
    fn diagnostic_source(&self) -> Option<&'static str> {
        Some(crate::packages::TRANSFORMER_PACKAGE)
    }

    /// `transformer.ts`: unless `noSemanticDiagnostics` is `true`, a file whose `getPreEmitDiagnostics` holds an error
    /// is left untransformed and its located diagnostics are reported through `context.addDiagnostic`, explained
    /// ([`FwInstance::explain_checker_diagnostic`]).
    fn forwards_checker_diagnostics(&self, shared: &FwShared) -> bool {
        shared.config.no_semantic_diagnostics != Some(true)
    }

    fn setup(&self, cx: &mut SetupCx<'_>, state: &mut FwState) -> Result<Arc<FwShared>, Fatal> {
        crate::setup::setup(cx, state).map(Arc::new)
    }

    /// A watch rebuild whose program reaches a package directory setup's build-info scan did not look in (a package
    /// installed and imported while watching, in either order) sets up again, so the rebuild reads that package's
    /// `flamework.build` as a clean build does. Other rebuilds keep the `Shared`.
    fn rebuild_start(&self, shared: &FwShared, _state: &FwState, changes: &ChangeSet) -> RebuildPlan {
        let added = changes.added.iter().map(String::as_str);
        match shared.package_directories.first_new_with(&mut crate::inputs::FsInputs, added) {
            Some(_) => RebuildPlan::RerunSetup,
            None => RebuildPlan::KeepShared,
        }
    }

    fn instance(&self, shared: Arc<FwShared>, _checker: CheckerInfo) -> FwInstance {
        FwInstance {
            shared,
            metadata: RefCell::default(),
            decorators: RefCell::default(),
            macro_symbols: RefCell::default(),
            file: RefCell::default(),
            macro_types: self.macro_types.clone(),
            serializer: RefCell::default(),
        }
    }

    fn finish(
        &self,
        shared: &FwShared,
        state: &mut FwState,
        parts: &[(FileIndex, &FwFilePart)],
        cx: &mut FinishCx<'_>,
    ) -> Result<(), Fatal> {
        crate::finish::finish(shared, state, parts, cx)
    }

    fn encode_file_part(&self, part: &FwFilePart) -> Option<Vec<u8>> {
        crate::part_codec::encode(part)
    }

    fn decode_file_part(&self, bytes: &[u8]) -> Option<FwFilePart> {
        crate::part_codec::decode(bytes)
    }
}

impl PluginInstance for FwInstance {
    type FilePart = FwFilePart;

    fn interest(&self, interest: &mut Interest) {
        interest.calls_by_symbol();
        // An optional chain's outermost call, before the chain is compiled: inlined and rewritten macros take the
        // chain's place (networking declines it: its sends are packed inside the compiler's nil check).
        interest.calls_chain_outer();
        // Metadata is decided by JSDoc, not by a decorator, so every class is offered.
        interest.classes(ClassFilter::All);
        // Components (F6): attribute setters and obfuscated keys.
        interest.assignments_by_receiver_symbol();
        interest.accesses_by_marker(crate::components::KEY_OBFUSCATION_MARKER, crate::components::TAG_KEY_OBFUSCATION);
        // Networking (F5): sends and callbacks on packed members.
        crate::networking::interest(interest);
    }

    fn classify_symbol(&self, cx: &mut Cx<'_>, symbol: P<Symbol>, role: SymbolRole) -> Option<Tag> {
        match role {
            SymbolRole::Callee => self.is_user_macro(cx, symbol).then_some(TAG_MACRO),
            SymbolRole::AssignReceiver => {
                self.is_attributes_symbol(cx, symbol).then_some(crate::components::TAG_ATTRIBUTES)
            }
        }
    }

    fn begin_file(&self, cx: &mut Cx<'_>) -> PResult<()> {
        let file_id = self.shared.file_id(&cx.file().path);
        *self.file.borrow_mut() = FileState {
            part: FwFilePart { file_id, ..FwFilePart::default() },
            ..FileState::default()
        };
        *self.serializer.borrow_mut() = crate::serializer::FileSerializer::default();
        Ok(())
    }

    fn call(&self, cx: &mut Cx<'_>, call: &CallSite, tag: Tag) -> PResult<Claim<LuauExpr>> {
        match tag {
            TAG_MACRO => crate::macros::user_macro(self, cx, call),
            crate::networking::TAG_RECEIVER | crate::networking::TAG_CALLEE => {
                crate::networking::calls::call_hook(self, cx, call)
            }
            _ => Ok(Claim::Decline),
        }
    }

    fn property_key_answer(&self, cx: &mut Cx<'_>, access: &AccessSite, tag: Tag) -> PResult<Claim<KeyAnswer>> {
        if tag != crate::components::TAG_KEY_OBFUSCATION {
            return Ok(Claim::Decline);
        }
        crate::components::obfuscation::obfuscated_key(self, cx, access)
    }

    fn assignment(&self, cx: &mut Cx<'_>, site: &AssignSite, tag: Tag) -> PResult<Claim<LuauExpr>> {
        if tag != crate::components::TAG_ATTRIBUTES {
            return Ok(Claim::Decline);
        }
        crate::components::setter::attribute_setter(self, cx, site)
    }

    fn class(&self, cx: &mut Cx<'_>, class: &ClassSite) -> PResult<Vec<LuauStmt>> {
        crate::class_hook::class_metadata(self, cx, class)
    }

    /// `explainRemovedNetworkMarker(state, diag) ?? explainRenamed(state, diag) ?? diag`: a dropped networking marker's
    /// error names the marker replacing it, a name renamed or removed before beta its replacement.
    fn explain_checker_diagnostic(
        &self,
        cx: &mut Cx<'_>,
        diagnostic: P<rbxts_plugin::ts::Diagnostic>,
    ) -> Option<rbxts_plugin::PluginDiagnostic> {
        crate::networking::mode::explain_removed_marker(cx, diagnostic)
            .or_else(|| crate::renamed::explain_renamed(cx, diagnostic))
    }

    fn end_file(&self, cx: &mut Cx<'_>) -> PResult<FwFilePart> {
        crate::serializer::check_output(self, cx);
        #[cfg(feature = "wire-harvest")]
        crate::serializer::wire::end_file(self, cx);
        let mut file = std::mem::take(&mut *self.file.borrow_mut());
        let mut seen = HashSet::new();
        for new in file.id_requests {
            if seen.insert(new.internal_id.clone()) {
                file.part.new_ids.push(new);
            }
        }
        Ok(file.part)
    }
}

#[cfg(test)]
mod tests {
    use rbxts_plugin::Plugin;

    #[test]
    fn diagnostics_name_the_transformer_as_stock_does() {
        // TS: `code: ` ${TRANSFORMER_PACKAGE}``, printed `error TS @flamework-experimental/transformer: …`.
        assert_eq!(super::Flamework::new().diagnostic_source(), Some("@flamework-experimental/transformer"));
    }
}
