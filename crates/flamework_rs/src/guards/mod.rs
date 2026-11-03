//! Guards (F3): `t` checks generated from types, a port of the TS transformer's `util/functions/buildGuardFromType.ts`
//! (with `propertyKey.ts`, `identifierName.ts`, `getInstanceTypeFromType.ts` and the guard parts of
//! `macros/intrinsics/guards.ts`), emitting Luau instead of TypeScript for roblox-ts to compile.
//!
//! **Entry points.** Each builds one guard with a generator of its own, as each TS call does:
//! - [`guard_for`] (`buildGuardFromType`): class metadata (`flamework:guard`, `flamework:parameter_guards`,
//!   `flamework:return_guard`), component instance guards;
//! - [`guard_for_with_dedup`] (`buildGuardFromTypeWithDedup`): `Modding.Generic<T, "guard">` (`createGuard<T>()`).
//!   With `transformer.optimizations.guardGenerationDedupLimit` set, types met that many times are declared once
//!   as locals: in the main chunk through the compiler's spill, else ahead of the TS statement around the call (the
//!   TS's `prereqList`), or kept with the guard in a function of their own when the function has no room for them;
//! - [`guard_fields_in_context`] (`buildGuardsFromType`): one guard per property (component attributes);
//! - [`tuple_guards`] (`buildTupleGuardsIntrinsic`): the `tuple-guards` intrinsic, `{ { guards }, rest, { after } }`.
//!
//! **Text.** The Luau is what roblox-ts makes of the TS builder's output, so it matches a TS5 build but for the order
//! of union members and the locals' names: TS7 orders a union's constituents its own way (deterministically), and the
//! guard follows it (DESIGN §10.4, behaviour-neutral); locals are named in the guard's render order with the
//! compiler's approval ([`crate::luau_names`]), not by TypeScript's printer over the whole file. Strings are
//! the builder's synthesized literals ([`rbxts_plugin::synthesized_string`]); properties come in TypeScript 7.1's
//! order ([`rbxts_plugin::Cx::properties_of_type`]), not TS5's (README, "Differences").
//!
//! **Not the TS's.**
//! - The Luau-limits budget: the TS lets Flamework's code claim 16 locals in a file's main chunk and 64 in a
//!   function, shared by every call site there. Here dedup locals whose statement is in the main chunk (not in a
//!   function or a class static block) go ahead of the top-level statement through the compiler's spill
//!   (`Cx::hoist_spillable_named`), which moves them into one table when the main chunk passes Luau's limits, so they
//!   need no budget; a dedup guard in a function checks its own count against the function's cap (64) and against
//!   the locals live where its locals would go ([`rbxts_plugin::Cx::live_before_statement`]), which must stay within
//!   180 (Luau's 200, less a margin of 20), else it keeps them in a function with the guard.

mod generator;
mod library;
pub(crate) mod names;
pub mod types;

use rbxts_luau::{Expr, Stmt, create};
use rbxts_plugin::ts::{Kind, Node, P, Type};
use rbxts_plugin::{Cx, LuauExpr, PResult};
use tsrs_checker::ElementFlags;

use generator::Generator;
use crate::luau_names::declared;

use crate::setup::FwShared;

/// What the generator needs of the project: the package name (the guard library) and the dedup limit.
#[derive(Clone, Copy, Debug)]
pub struct GuardConfig<'a> {
    /// `package.json`'s name: inside the Flamework packages the guard library is `@rbxts/t` itself.
    pub package_name: &'a str,
    /// The package root (`rootDirectory`), for the `Reflect` import's specifier inside core.
    pub root_dir: &'a str,
    /// `transformer.optimizations.guardGenerationDedupLimit`.
    pub dedup_limit: Option<u32>,
}

impl FwShared {
    /// The guard generator's view of the project.
    pub fn guard_config(&self) -> GuardConfig<'_> {
        GuardConfig {
            package_name: &self.package_name,
            root_dir: &self.root_dir,
            dedup_limit: self.config.guard_generation_dedup_limit,
        }
    }
}

/// `buildGuardFromType`: the guard of `ty`. Errors (unsupported types) are reported at `node`, with a note for each
/// type the guard was being built inside of, and abort the hook.
pub fn guard_for(cx: &mut Cx<'_>, config: GuardConfig<'_>, node: P<Node>, ty: P<Type>) -> PResult<LuauExpr> {
    guard_for_in_context(cx, config, node, ty, None)
}

/// [`guard_for`] inside `withDiagnosticContext(at, message, …)`: an error becomes a note of an error at `at` with
/// `message` (component links: "Failed to generate a guard for the …link").
pub fn guard_for_in_context(
    cx: &mut Cx<'_>,
    config: GuardConfig<'_>,
    node: P<Node>,
    ty: P<Type>,
    context: Option<(P<Node>, String)>,
) -> PResult<LuauExpr> {
    let mut generator = Generator::new(cx, config, node);
    generator.set_context(context);
    let mut guard = generator.build_guard(ty)?;
    generator.names.assign(generator.cx, &mut [], Some(&mut guard));
    Ok(guard)
}

/// [`guard_for`] with its locals left unnamed: the guard holds placeholders, and `names` gives each its base name
/// ([`crate::luau_names::Names::assign_with`]). For a guard that lands inside other generated code (the
/// serializer's), named with that code.
pub(crate) fn guard_for_unnamed(
    cx: &mut Cx<'_>,
    config: GuardConfig<'_>,
    node: P<Node>,
    ty: P<Type>,
) -> PResult<(LuauExpr, crate::luau_names::Names)> {
    let mut generator = Generator::new(cx, config, node);
    let guard = generator.build_guard(ty)?;
    Ok((guard, generator.names))
}

/// The TS's `FUNCTION_LOCAL_BUDGET` (`luauLimits.ts`): the dedup locals one guard may put ahead of a statement in a
/// function. (In the main chunk the compiler's spill holds them: no budget.)
const FUNCTION_LOCAL_BUDGET: usize = 64;
/// The live locals, the guard's included, past which its dedup locals go in a function with it: Luau's 200, less a
/// margin of 20.
const LIVE_LOCALS: usize = 180;

/// `buildGuardFromTypeWithDedup`: the guard of `ty` for a `Generic<T, "guard">` argument at `node` (the call). With a
/// dedup limit, the shared guards go ahead of the statement around the call: in the main chunk ahead of the top-level
/// statement, each through the compiler's spill (`cx.hoist_spillable_named`, in their order, so each is declared
/// before the ones that read it); in a function where the TS's `prereqList` puts them (`cx.insert_before_statement`),
/// or, when the function has no room for them, into a function with the guard.
pub fn guard_for_with_dedup(cx: &mut Cx<'_>, config: GuardConfig<'_>, node: P<Node>, ty: P<Type>) -> PResult<LuauExpr> {
    let mut generator = Generator::new(cx, config, node);
    if let Some(limit) = config.dedup_limit {
        generator.calculate_dedup(ty, limit.max(1));
    }
    let mut guard = generator.build_guard(ty)?;

    let count = generator.dedup.len();
    if count > 0 && in_main_chunk(node) {
        for dedup in std::mem::take(&mut generator.dedup) {
            let mut value = dedup.value;
            generator.names.assign(generator.cx, &mut [], Some(&mut value));
            let index = crate::luau_names::placeholder_index(&dedup.id).expect("a dedup guard's placeholder");
            let base = generator.names.base(index).to_owned();
            let name = match generator.cx.hoist_spillable_named(value, &base) {
                Expr::Identifier(local) => local.name,
                // refused: the compiler has reported it, and the build fails
                _ => crate::luau_names::unique_name(generator.cx, &base),
            };
            generator.names.name(index, name);
        }
        generator.names.assign(generator.cx, &mut [], Some(&mut guard));
        return Ok(guard);
    }
    if count == 0 || dedup_fits(generator.cx, count) {
        let mut statements: Vec<Stmt> =
            generator.dedup.drain(..).map(|d| create::local(declared(&d.id), Some(d.value))).collect();
        generator.names.assign(generator.cx, &mut statements, Some(&mut guard));
        generator.cx.insert_before_statement(statements);
        return Ok(guard);
    }

    guard = generator.hold_in_function(guard);
    generator.names.assign(generator.cx, &mut [], Some(&mut guard));
    Ok(guard)
}

/// Whether `count` more locals fit ahead of the statement around the call, in a function (see the module docs).
fn dedup_fits(cx: &mut Cx<'_>, count: usize) -> bool {
    count <= FUNCTION_LOCAL_BUDGET && cx.live_before_statement().locals as usize + count <= LIVE_LOCALS
}

/// The statement `cx.insert_before_statement` would put `node`'s statements ahead of is in the main chunk: in no
/// function or class static block (in a block of the file's top level, its locals go ahead of the top-level statement
/// instead, which is the same for a guard: it reads nothing but `t` and the other dedup guards).
fn in_main_chunk(node: P<Node>) -> bool {
    crate::networking::scope::enclosing_statement(node).is_some_and(|statement| !in_function(statement))
}

/// `enclosingScope(node)` is a function (or a class static block), not the file.
fn in_function(node: P<Node>) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        if tsrs_ast::is_function_like(Some(parent)) || parent.kind() == Kind::ClassStaticBlockDeclaration {
            return true;
        }
        current = parent.parent();
    }
    false
}

/// `buildGuardsFromType`: a guard per property of `ty`, keyed for a Luau table: by the property's table key when
/// `interface` (as `t.interface` checks them), else by its name (a component's attributes). `context`: the
/// `withDiagnosticContext` around it (component attributes: "Failed to generate component attributes").
pub fn guard_fields_in_context(
    cx: &mut Cx<'_>,
    config: GuardConfig<'_>,
    node: P<Node>,
    ty: P<Type>,
    interface: bool,
    context: Option<(P<Node>, String)>,
) -> PResult<Vec<(LuauExpr, LuauExpr)>> {
    let mut generator = Generator::new(cx, config, node);
    generator.set_context(context);
    let fields = generator.build_guards_from_type(ty, interface)?;
    let mut map = create::map(fields);
    generator.names.assign(generator.cx, &mut [], Some(&mut map));
    let Expr::Map(fields) = map else { unreachable!("a map") };
    Ok(fields.into_iter().map(|field| (field.index, field.value)).collect())
}

/// `buildTupleGuardsIntrinsic` (the `tuple-guards` intrinsic): `{ { element guards }, rest guard or nil }`, and a
/// third entry `{ guards after the rest }` for a tuple with elements after its rest. A tuple of only a rest element
/// is an array: `{ {}, element guard }`.
pub fn tuple_guards(cx: &mut Cx<'_>, config: GuardConfig<'_>, node: P<Node>, ty: P<Type>) -> PResult<LuauExpr> {
    if cx.checker().is_array_type_exported(ty) {
        let element = cx.checker().get_type_arguments_exported(ty)[0];
        let guard = guard_for(cx, config, node, element)?;
        return Ok(create::array(vec![create::array(Vec::new()), guard]));
    }
    if !tsrs_checker::is_tuple_type_exported(ty) {
        let text = cx.checker().type_to_string_exported(ty);
        return Err(cx.error(node, format!("Intrinsic encountered non-tuple type: {text}")));
    }

    let arguments = cx.checker().get_type_arguments_exported(ty);
    let infos = ty.target().expect("a tuple reference has a target").as_tuple_type().element_infos();
    let mut guards = Vec::new();
    let mut rest: Option<Expr> = None;
    let mut after = Vec::new();
    for (index, &element) in arguments.iter().enumerate() {
        let info = infos.get(index).copied().unwrap_or_default();
        let guard = guard_for(cx, config, info.labeled_declaration().unwrap_or(node), element)?;
        if info.flags.intersects(ElementFlags::Rest) {
            rest = Some(guard);
        } else if rest.is_some() {
            after.push(guard);
        } else {
            guards.push(guard);
        }
    }

    let mut entries = vec![create::array(guards), rest.unwrap_or_else(create::nil)];
    if !after.is_empty() {
        entries.push(create::array(after));
    }
    Ok(create::array(entries))
}
