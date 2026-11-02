//! User macros (the TS transformer's `transformations/transformUserMacro.ts` and `macros/intrinsics/*`).
//!
//! A user macro is a call whose callee's declaration has `@metadata macro`. Each parameter whose argument is missing
//! or `undefined` and whose type carries a marker gets generated metadata; missing arguments before it are padded
//! with `nil`; everything else is compiled by roblox-ts as written (`compile_call` with argument slots). `super()`
//! is skipped.
//!
//! **Markers** ([`UserMacro`]): `Modding.Generic<T, "id" | "guard" | "text" | "dependency" | "dependencyConcise">`,
//! `Caller` (line, character, width, uuid, text, in UTF-16), `Hash` / `Obfuscate`, `Labels`, `Many` / `Emit`,
//! `Caller.Constant` (hoisted), and `Intrinsic`s: `path`, `pathglob`, `env`, `obfuscate-obj`, `shuffle-array` and
//! `plugin` ([`crate::macro_type`]).
//!
//! **Parameter intrinsics** on the macro's declaration: `intrinsic-inline` (the call becomes one generated value),
//! `intrinsic-flamework-rewrite` (the callee becomes `core.<Namespace>.<name>`), `intrinsic-const` (a literal
//! argument check) and `intrinsic-component-config` (a class decorator's config, [`crate::components::config`]).
//!
//! **Guards.** `Generic<T, "guard">` is [`crate::guards::guard_for_with_dedup`] at the call (its dedup locals go
//! ahead of the statement, before any argument is compiled, as the TS's `prereqList` puts them), and the
//! `tuple-guards` intrinsic is [`crate::guards::tuple_guards`].
//!
//! **Serializers.** The `serializer` intrinsic is [`crate::serializer::build_serializer`], versioned by the argument of
//! the `intrinsic-serializer-options` parameter, built at the call. `Many<(A | B)[]>` lists its values as the call
//! (or the union's alias) writes them, else by key ([`crate::serializer::union_member_order`]).
//!
//! **Call shapes.** A call is a macro only when its callee is an identifier or a property access (TypeScript's
//! `getSymbolAtLocation` has no symbol for any other callee): `o["m"]()`, `(o.m)()` and `o.m!()` are plain calls,
//! and `super()` is skipped. `super.m()`, `super.m?.()`, `o.m?.()`, `o?.m()` and `o?.m?.()` generate their
//! parameters as any call does (`compile_call_with` keeps roblox-ts's `super.m(self, …)` and the chain's nil checks,
//! the generated arguments inside them). An inlined or rewritten macro call that is the last link of an optional chain
//! is taken when the compiler offers the chain's outermost call, before anything of the chain is compiled
//! (`Interest::calls_chain_outer`), as TS5 replaced the call node: an inlined call's value stands for the whole chain
//! (`maybe?.inlined<T>()` is the value, `maybe` is not evaluated), and a rewritten call keeps only its own `?.`
//! (`o.m?.<T>(x)` nil-checks `Flamework._implements` itself, `o?.m<T>(x)` is a plain call to it). One followed by
//! more of its chain (`maybe?.inlined<T>().size()`, mostly a type error in TS5's output) is refused.
//!
//! **Argument order.** The generated values are `Arg::Deferred` slots of `compile_call_with`, built when the
//! lowering reaches them among the written arguments, so that what they put ahead of the statement lands in argument
//! order.
//!
//! **Networking.** The `network-decoder` / `network-result-decoder` intrinsics are decoders built with the file's
//! serializers ([`crate::serializer::build_decoder`]), `network-unreliable` reads the member's markers, and
//! `intrinsic-middleware` obfuscates a network's middleware names ([`crate::networking::middleware`]).

use rbxts_luau::create;
use rbxts_plugin::ts::{Kind, Node, P, Symbol, Type};
use rbxts_plugin::{
    Arg, CallKind, CallSite, Callee, Claim, Cx, LuauExpr, PResult, PluginDiagnostic, synthesized_string,
};
use tsrs_ast::SymbolFlags;
use tsrs_checker::{ElementFlags, LiteralValue, TypeFlags};

use crate::build_info::{GlobUse, PathUse};
use crate::macro_type::{Literal as TypeLiteral, literal_of};
use crate::node_path;
use crate::order::OrderKey;
use crate::packages::CORE_PACKAGE;
use crate::plugin::{FwInstance, PathEvent};

/// A key of a generated table: the one the type declares (`getPropertyKey`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum TableKey {
    Str(String),
    Num(f64),
}

/// A literal value.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum LiteralMacro {
    Str(String),
    Num(f64),
    Bool(bool),
    Undefined,
}

/// `Modding.Generic`'s metadata kind.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum GenericKind {
    Id,
    Guard,
    Text,
    /// `dependency` / `dependencyConcise`, with the `Modding.Injectable` metadata's macros.
    Dependency { concise: bool, metadata: Option<Vec<UserMacro>> },
    /// Any other string: no value (`nil`), as in the TS.
    Other,
}

/// `Modding.Caller`'s metadata kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CallerKind {
    Line,
    Character,
    Width,
    Uuid,
    Text,
    Other,
}

/// An intrinsic (`Modding.Intrinsic<id, inputs>`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Intrinsic {
    Path(P<Type>),
    PathGlob(P<Type>),
    Env { name: P<Type>, fallback: Option<P<Type>> },
    ObfuscateObject { inner: Box<UserMacro>, context: Option<String> },
    ShuffleArray(Box<UserMacro>),
    Plugin { id: Option<String>, input: Option<P<Type>> },
    /// `tuple-guards`: the guards of a tuple's elements.
    TupleGuards(P<Type>),
    /// `serializer`: `T`'s serializer.
    Serializer(P<Type>),
    /// `network-decoder`: the decoder of the argument list `list` of the member `member` (named `key`), when it is
    /// packed.
    NetworkDecoder { list: P<Type>, member: Option<P<Type>>, key: Option<P<Type>> },
    /// `network-result-decoder`: the decoder of the function type's result, when it is packed.
    NetworkResultDecoder { function: P<Type>, key: Option<P<Type>> },
    /// `network-unreliable`: `true` for a member declared unreliable.
    NetworkUnreliable { member: Option<P<Type>>, key: Option<P<Type>> },
    /// An unusable intrinsic, with the TS transformer's message.
    Invalid(String),
}

/// The TS transformer's `UserMacro`.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum UserMacro {
    Generic { target: P<Type>, kind: GenericKind },
    Caller(CallerKind),
    Many(Vec<UserMacro>),
    Map(Vec<(TableKey, UserMacro)>),
    Literal(LiteralMacro),
    Intrinsic(Intrinsic),
    SharedRef(Box<UserMacro>),
}

/// `obfuscateArray` under obfuscation: a shuffle seeded by the build seed, the file, the call's place, what is
/// shuffled and how many shuffles the call made before (DESIGN §10.4), so it is the same on every checker.
pub(crate) fn shuffle<T>(fw: &FwInstance, cx: &Cx<'_>, call: P<Node>, items: &mut [T], context: &str, count: &mut usize) {
    let shared = &fw.shared;
    if shared.config.obfuscation != Some(true) {
        return;
    }
    let Some(seed) = &shared.build_seed else { return };
    *count += 1;
    let offset = cx.utf16_pos(call).offset;
    let file_id = shared.file_id(&cx.file().path);
    crate::shuffle::shuffle(items, &format!("{}\0{file_id}\0{offset}\0{context}\0{count}", seed.as_str()));
}

/// The call hook for a callee classified as a user macro (`transformCallExpression` / `transformNewExpression` and
/// `transformUserMacro`).
pub(crate) fn user_macro(fw: &FwInstance, cx: &mut Cx<'_>, call: &CallSite) -> PResult<Claim<LuauExpr>> {
    // `super()` is skipped, as there is not enough information to evaluate it.
    if call.kind == CallKind::SuperCall {
        return Ok(Claim::Decline);
    }
    // The node the TS transformer sees: the call expression (a decorator's own call).
    let expression = match call.kind {
        CallKind::Decorator => match call.node.expression() {
            Some(e) if e.kind() == Kind::CallExpression => e,
            _ => return Ok(Claim::Decline),
        },
        _ => call.node,
    };
    if !has_symbol_callee(expression) {
        return Ok(Claim::Decline);
    }

    let signature = cx.checker().get_resolved_signature_exported(expression);
    let Some(declaration) = signature.declaration() else { return Ok(Claim::Decline) };
    let metadata = fw.metadata(cx, declaration);
    // The chain's outermost call, offered before the chain is compiled: only an inlined or rewritten call takes the
    // chain's place (TS5 replaced the call node); any other is lowered with the chain, offered again inside it.
    if call.chain_outer
        && !matches!(metadata.symbols("intrinsic-inline"), Some([_]))
        && metadata.symbols("intrinsic-flamework-rewrite").and_then(|s| s.first()).is_none()
    {
        return Ok(Claim::Decline);
    }

    // `intrinsic-middleware`: the middleware objects whose names are obfuscated (none without obfuscation).
    let middleware = match metadata.symbols("intrinsic-middleware") {
        Some(parameters) => crate::networking::middleware::targets(fw, cx, expression, signature, parameters)?,
        None => Vec::new(),
    };

    // What each parameter generates, decided from the types alone.
    let analysis = Analysis { fw, node: expression };
    let mut parameters: Vec<(usize, UserMacro)> = Vec::new();
    for index in 0..parameter_count(cx, signature) {
        if !is_undefined_argument(call.args.get(index).copied()) {
            continue;
        }
        let parameter_type = cx.checker().get_type_at_position_exported(signature, index);
        let target = cx.checker().get_non_nullable_type(parameter_type);
        if let Some(found) = analysis.of_union(cx, target)? {
            validate_dependency_target(fw, cx, expression, declaration, &found)?;
            parameters.push((index, found));
        }
    }

    let inline = match metadata.symbols("intrinsic-inline") {
        Some([parameter]) => Some(parameter_index(signature, *parameter).ok_or_else(|| {
            cx.error(expression, "The intrinsic-inline parameter is not a parameter of this signature.")
        })?),
        _ => None,
    };
    if inline.is_none() {
        let constants = metadata.symbols("intrinsic-const").map(<[_]>::to_vec).unwrap_or_default();
        validate_const(cx, expression, signature, &constants)?;
    }
    let rewrite = metadata.symbols("intrinsic-flamework-rewrite").and_then(|s| s.first().copied());
    // Not the chain's last link (see the module docs): refused, before anything is compiled.
    if call.in_optional_chain && !call.chain_outer && (inline.is_some() || rewrite.is_some()) {
        return Err(cx.error(expression, optional_chain_gap(inline.is_some())));
    }
    // `intrinsic-component-config`: the config parameters of a class decorator (`transformComponentConfig`).
    let component = crate::components::config::decorated_class(expression).and_then(|class| {
        let symbols = metadata.symbols("intrinsic-component-config")?;
        let indices: Vec<usize> = symbols.iter().filter_map(|&s| parameter_index(signature, s)).collect();
        Some((class, indices))
    })
    .filter(|(_, indices)| !indices.is_empty());

    if parameters.is_empty() && inline.is_none() && rewrite.is_none() && component.is_none() && middleware.is_empty() {
        // Nothing to generate or change: the call as written, which is what roblox-ts compiles anyway.
        return Ok(Claim::Decline);
    }

    // Taken: from here on, build and compile.
    // The generated values by argument index: a parameter's metadata, a component config (which replaces its
    // argument, one that is not an object literal too, as the TS does), a network's middleware.
    let mut generated: Vec<(usize, Generated<'_>)> =
        parameters.iter().map(|(index, found)| (*index, Generated::Macro(found))).collect();
    if let Some((class, indices)) = &component {
        for &index in indices {
            if !generated.iter().any(|(i, _)| *i == index) {
                generated.push((index, Generated::Config(*class)));
            }
        }
    }
    for (index, object) in middleware {
        if !generated.iter().any(|(i, _)| *i == index) {
            generated.push((index, Generated::Middleware(object)));
        }
    }
    generated.sort_by_key(|(i, _)| *i);
    let site = GeneratedSite { fw, call, expression };

    if let Some(index) = inline {
        // `inlineMacroIntrinsic`: the call is the value of that parameter (the TS cast goes: Luau has none). The
        // callee is not compiled, so its import may go, as the TS transformer's output drops it.
        elide_callee_import(cx, expression);
        let mut value = None;
        for (at, what) in &generated {
            let built = site.build(cx, *at, what)?;
            if *at == index {
                value = Some(built);
            }
        }
        if let Some(value) = value {
            return Ok(Claim::Take(value));
        }
        return match call.args.get(index) {
            Some(&argument) => cx.compile_expr(argument).map(Claim::Take),
            None => Ok(Claim::Take(create::nil())),
        };
    }

    // Each generated value in its slot, built when the lowering reaches it (`Arg::Deferred`); missing arguments before
    // the last one padded with `nil`.
    let highest = generated.last().map(|(i, _)| *i);
    let count = call.args.len().max(highest.map_or(0, |h| h + 1));
    let mut args = Vec::with_capacity(count);
    for index in 0..count {
        if let Some(slot) = generated.iter().position(|(i, _)| *i == index) {
            args.push(Arg::Deferred(slot));
        } else {
            match call.args.get(index) {
                Some(&arg) => args.push(Arg::Original(arg)),
                None => args.push(Arg::Nil),
            }
        }
    }

    // `intrinsic-flamework-rewrite`: a `declare`d macro reaches its runtime implementation in core.
    let callee = match rewrite {
        Some(target) => {
            let Some(namespace) = target.parent() else {
                return Err(cx.error(
                    expression,
                    format!("The rewrite target '{}' is not declared inside a namespace.", target.name()),
                ));
            };
            let reflect = fw.shared.reflect_specifier(&cx.file().path);
            let namespace = crate::class_hook::import_named(cx, &reflect, CORE_PACKAGE, namespace.name(), expression)?;
            elide_callee_import(cx, expression);
            Callee::Function(create::property(namespace, target.name()))
        }
        None => Callee::Original,
    };
    cx.compile_call_with(call, callee, args, |cx, slot| {
        let (index, what) = &generated[slot];
        site.build(cx, *index, what)
    })
    .map(Claim::Take)
}

/// A value [`user_macro`] generates for an argument.
enum Generated<'a> {
    /// A parameter's metadata.
    Macro(&'a UserMacro),
    /// `intrinsic-component-config`: the config of the decorated class.
    Config(P<Node>),
    /// `intrinsic-middleware`: the middleware object, its names obfuscated.
    Middleware(P<Node>),
}

/// The call [`user_macro`] builds generated values for.
struct GeneratedSite<'a> {
    fw: &'a FwInstance,
    call: &'a CallSite,
    expression: P<Node>,
}

impl GeneratedSite<'_> {
    /// The value of argument `index`.
    fn build(&self, cx: &mut Cx<'_>, index: usize, what: &Generated<'_>) -> PResult<LuauExpr> {
        let (fw, expression) = (self.fw, self.expression);
        match what {
            Generated::Macro(found) => {
                let key = fw.order_key(cx, expression, (0, index as u32));
                fw.file.borrow_mut().current_key = key.clone();
                Builder { fw, node: expression, key }.build(cx, found)
            }
            Generated::Config(class) => {
                let key = fw.order_key(cx, expression, (0, index as u32));
                let argument = self.call.args.get(index).copied();
                crate::components::config::component_config(fw, cx, *class, argument, key)
            }
            Generated::Middleware(object) => crate::networking::middleware::compile(fw, cx, expression, *object),
        }
    }
}

/// The error for an inlined or rewritten macro call followed by more of its optional chain (see the module docs).
fn optional_chain_gap(inlined: bool) -> String {
    let what = if inlined { "an inlined" } else { "a rewritten" };
    format!(
        "flamework-rs cannot compile {what} macro call followed by more of its optional chain as Flamework's TS \
         transformer does. Call it without `?.`, or end the chain at the call."
    )
}

/// The call's callee is not compiled (inlined or rewritten): asks the compiler to elide the import its root name
/// comes from (`inlinedText` in `inlinedText<T>()`, `Flamework` in `Flamework.env("X")`), which it does unless
/// the file's Luau still uses it. Not a namespace import (`ns` in `ns.inlined<T>()`, `ns?.inlined<T>()`), which
/// TS5 keeps (TypeScript counts the reference whatever replaced the call).
fn elide_callee_import(cx: &mut Cx<'_>, call: P<Node>) {
    let mut root = call.expression();
    while let Some(node) = root {
        match node.kind() {
            Kind::PropertyAccessExpression | Kind::ElementAccessExpression | Kind::ParenthesizedExpression => {
                root = node.expression();
            }
            _ => break,
        }
    }
    let Some(root) = root.filter(|n| n.kind() == Kind::Identifier) else { return };
    let Some(symbol) = cx.checker().get_symbol_at_location_exported(root) else { return };
    if !symbol.flags().intersects(SymbolFlags::Alias) {
        return;
    }
    if let Some(&declaration) = symbol.declarations().first()
        && matches!(
            declaration.kind(),
            Kind::ImportSpecifier | Kind::ImportClause | Kind::ImportEqualsDeclaration
        )
    {
        cx.request_import_elision(declaration);
    }
}

/// Whether the TS transformer can see `call` as a macro at all: it asks `state.getSymbol(node.expression)`, and
/// TypeScript's `getSymbolAtLocation` gives a callee a symbol only when it is an identifier or a property access. So
/// `o["m"]()`, `o?.["m"]()`, `(o.m)()` and `o.m!()` are plain calls there whatever they call, and `super()` is
/// skipped. Known unsupported shapes: TypeScript also finds a symbol for a `this` callee and, through
/// `isNamedDeclaration`, for a named class or function expression callee; a macro there would need `@metadata macro`
/// on a `this` parameter or a class expression, which never happens in practice. A parenthesized decorator call
/// (`@(deco<T>())`) is offered as an ordinary call, as TS5 transforms it.
pub(crate) fn has_symbol_callee(call: P<Node>) -> bool {
    call.expression().is_some_and(|callee| matches!(callee.kind(), Kind::Identifier | Kind::PropertyAccessExpression))
}

/// `state.isUserMacro(state.getSymbol(node.expression))`: whether the TS transformer takes `call` through
/// `transformUserMacro`, decided from the callee's own symbol (aliases skipped), as the call hook's offer is, not from
/// the resolved signature's declaration: `const alias = wrap; alias<T>(x)` and `holder.inner.wrap<T>(x)` (with
/// `inner = { wrap }`) resolve to `wrap`'s signature but are plain calls. The pre-scan that must agree with the hook
/// (`crate::order`) asks this.
pub(crate) fn is_macro_callee(fw: &FwInstance, cx: &mut Cx<'_>, call: P<Node>) -> bool {
    if !has_symbol_callee(call) {
        return false;
    }
    let Some(callee) = call.expression() else { return false };
    crate::metadata::get_symbol(cx, callee).is_some_and(|symbol| fw.is_user_macro(cx, symbol))
}

/// `isUndefinedArgument`: missing, or the identifier `undefined`.
fn is_undefined_argument(argument: Option<P<Node>>) -> bool {
    match argument {
        None => true,
        Some(node) => node.kind() == Kind::Identifier && node.text() == "undefined",
    }
}

/// `getParameterCount`: a rest parameter typed as a tuple counts its fixed elements.
fn parameter_count(cx: &mut Cx<'_>, signature: P<tsrs_checker::Signature>) -> usize {
    let parameters = signature.parameters();
    let length = parameters.len();
    if signature.has_rest_parameter()
        && let Some(&last) = parameters.last()
    {
        let rest = cx.checker().get_type_of_symbol(last);
        if tsrs_checker::is_tuple_type_exported(rest)
            && let Some(target) = rest.target()
        {
            let tuple = target.as_tuple_type();
            let has_rest_element = tuple.combined_flags.get().intersects(ElementFlags::Variable);
            return length + tuple.fixed_length() - usize::from(!has_rest_element);
        }
    }
    length
}

/// The index of the signature parameter a `{@link}` names (`v.valueDeclaration?.symbol === parameter`).
fn parameter_index(signature: P<tsrs_checker::Signature>, parameter: P<Symbol>) -> Option<usize> {
    signature
        .parameters()
        .iter()
        .position(|p| p.value_declaration().and_then(|d| d.symbol()) == Some(parameter))
}

/// `validateParameterConstIntrinsic`: the arguments of the linked parameters must be literals, or object or array
/// literals without spreads.
fn validate_const(
    cx: &mut Cx<'_>,
    call: P<Node>,
    signature: P<tsrs_checker::Signature>,
    parameters: &[P<Symbol>],
) -> PResult<()> {
    for &parameter in parameters {
        let Some(index) = parameter_index(signature, parameter) else { continue };
        let Some(&argument) = call.arguments().get(index) else { continue };
        if tsrs_ast::is_literal_expression(argument) {
            continue;
        }
        let elements = match argument.kind() {
            Kind::ObjectLiteralExpression => argument.properties(),
            Kind::ArrayLiteralExpression => argument.elements(),
            _ => {
                let diagnostic =
                    PluginDiagnostic::error(argument, "Flamework expected this argument to be a literal expression.")
                        .with_related(
                            parameter.value_declaration().unwrap_or(argument),
                            "Required because this parameter must be known at compile-time.",
                        );
                return Err(cx.report(diagnostic).expect("an error"));
            }
        };
        if let Some(&spread) =
            elements.iter().find(|e| matches!(e.kind(), Kind::SpreadElement | Kind::SpreadAssignment))
        {
            let diagnostic =
                PluginDiagnostic::error(spread, "Flamework does not support spread expressions in this location.")
                    .with_related(
                        parameter.value_declaration().unwrap_or(argument),
                        "Required because this parameter must be known at compile-time.",
                    );
            return Err(cx.report(diagnostic).expect("an error"));
        }
    }
    Ok(())
}

/// `validateDependencyTarget`: `Dependency<T>()` / `resolveDependency<T>()` of a component that is not a provider can
/// only fail at runtime.
fn validate_dependency_target(
    fw: &FwInstance,
    cx: &mut Cx<'_>,
    call: P<Node>,
    declaration: P<Node>,
    found: &UserMacro,
) -> PResult<()> {
    let UserMacro::Generic { target, kind: GenericKind::Dependency { .. } } = found else { return Ok(()) };
    if !is_core_dependency_resolver(cx, declaration) {
        return Ok(());
    }
    let Some(component) = fw.component_only_class(cx, *target) else { return Ok(()) };
    let name = match component.name() {
        Some(name) => name.text().to_owned(),
        None => cx.checker().type_to_string_exported(*target),
    };
    let callee = call.expression().map_or("Dependency", |c| cx.source_text(c));
    let diagnostic = PluginDiagnostic::error(
        call,
        format!(
            "'{name}' is a component (@Component), not a provider: {callee}<{name}>() resolves providers, and a module never constructs a component, so this fails at runtime."
        ),
    )
    .with_hint(format!(
        "Make '{name}' a @Provider(), or get the component from the instance it is attached to: Dependency<Components>().getComponent<{name}>(instance)."
    ));
    Err(cx.report(diagnostic).expect("an error"))
}

/// `isCoreDependencyResolver`: core's `Dependency`, or `resolveDependency` on its `Module`.
fn is_core_dependency_resolver(cx: &Cx<'_>, declaration: P<Node>) -> bool {
    let identifier = |node: Option<P<Node>>| node.filter(|n| n.kind() == Kind::Identifier).map(|n| n.text());
    let name = match declaration.kind() {
        Kind::FunctionDeclaration | Kind::MethodDeclaration | Kind::MethodSignature => identifier(declaration.name()),
        Kind::FunctionType | Kind::ArrowFunction | Kind::FunctionExpression => match declaration.parent() {
            Some(parent)
                if matches!(
                    parent.kind(),
                    Kind::PropertySignature | Kind::PropertyDeclaration | Kind::VariableDeclaration
                ) =>
            {
                identifier(parent.name())
            }
            _ => None,
        },
        _ => None,
    };
    if !matches!(name, Some("Dependency" | "resolveDependency")) {
        return false;
    }
    crate::class_hook::declaring_package(cx, declaration).as_deref() == Some(CORE_PACKAGE)
}

/// `getSerializerVersion`: the version the call's `intrinsic-serializer-options` argument gives (`None` without one).
fn serializer_version_at(fw: &FwInstance, cx: &mut Cx<'_>, call: P<Node>) -> PResult<Option<u32>> {
    let signature = cx.checker().get_resolved_signature_exported(call);
    let options = signature.declaration().and_then(|declaration| {
        fw.metadata(cx, declaration).symbols("intrinsic-serializer-options").and_then(|s| s.first().copied())
    });
    crate::serializer::serializer_version(cx, call, options)
}

fn property(cx: &mut Cx<'_>, ty: P<Type>, name: &str) -> Option<P<Type>> {
    cx.checker().get_type_of_property_of_type_exported(ty, name)
}

/// The text of a string literal type.
pub(crate) fn string_literal(ty: P<Type>) -> Option<&'static str> {
    if !ty.flags().intersects(TypeFlags::StringLiteral) {
        return None;
    }
    match ty.as_literal_type().value() {
        Some(LiteralValue::String(s)) => Some(s),
        _ => None,
    }
}

/// The union type node among the type arguments written at `call` (nested ones too, first in source order) whose
/// type is `union`: `({ z: 1 } | { a: 1 })` in `Modding.inspect<({ z: 1 } | { a: 1 })[]>()`.
fn written_union(cx: &mut Cx<'_>, call: P<Node>, union: P<Type>) -> Option<P<Node>> {
    fn visit(cx: &mut Cx<'_>, node: P<Node>, union: P<Type>, found: &mut Option<P<Node>>) {
        if found.is_some() {
            return;
        }
        if node.kind() == Kind::UnionType && cx.checker().get_type_from_type_node_exported(node) == union {
            *found = Some(node);
            return;
        }
        node.for_each_child(&mut |child| {
            visit(cx, child, union, found);
            found.is_some()
        });
    }
    let mut found = None;
    for &argument in call.type_arguments() {
        visit(cx, argument, union, &mut found);
    }
    found
}

/// Reads macro types (`getUserMacroOf*`): types only, nothing compiled or recorded.
pub(crate) struct Analysis<'f> {
    pub fw: &'f FwInstance,
    /// The macro call, where analysis errors are reported.
    pub node: P<Node>,
}

impl Analysis<'_> {
    /// `getUserMacroOfUnion`. A union whose members carry different macros is an error (DESIGN §10.4): the TS
    /// transformer took the first, which depended on the union's order.
    pub(crate) fn of_union(&self, cx: &mut Cx<'_>, target: P<Type>) -> PResult<Option<UserMacro>> {
        if !target.flags().intersects(TypeFlags::Union) {
            return self.of_type(cx, target);
        }
        let mut found: Option<UserMacro> = None;
        for &constituent in target.types() {
            let Some(next) = self.of_type(cx, constituent)? else { continue };
            match &found {
                None => found = Some(next),
                Some(first) if *first == next => {}
                Some(_) => {
                    let shown = cx.checker().type_to_string_exported(target);
                    return Err(cx.error(
                        self.node,
                        format!(
                            "The members of '{shown}' carry different Flamework macros, and a parameter can only generate one."
                        ),
                    ));
                }
            }
        }
        Ok(found)
    }

    /// `getUserMacroOfType`.
    fn of_type(&self, cx: &mut Cx<'_>, target: P<Type>) -> PResult<Option<UserMacro>> {
        // `Constant<Emit<T>>` carries both markers: the shared-ref one is looked at first.
        if property(cx, target, "_flamework_macro_shared_ref").is_some() {
            return self.of_many(cx, target).map(Some);
        }
        if let Some(many) = property(cx, target, "_flamework_macro_many") {
            return self.of_many(cx, many).map(Some);
        }
        self.basic(cx, target)
    }

    /// `getUserMacroOfMany`.
    pub(crate) fn of_many(&self, cx: &mut Cx<'_>, target: P<Type>) -> PResult<UserMacro> {
        // A basic macro is a constant already, so `Constant` around it changes nothing.
        if let Some(basic) = self.basic(cx, target)? {
            return Ok(basic);
        }
        if let Some(shared) = property(cx, target, "_flamework_macro_shared_ref") {
            return Ok(UserMacro::SharedRef(Box::new(self.of_many(cx, shared)?)));
        }
        if let Some(many) = property(cx, target, "_flamework_macro_many") {
            return self.of_many(cx, many);
        }

        if tsrs_checker::is_tuple_type_exported(target) {
            let members = cx.checker().get_type_arguments_exported(target).to_vec();
            let mut macros = Vec::with_capacity(members.len());
            for member in members {
                macros.push(self.of_many(cx, member)?);
            }
            return Ok(UserMacro::Many(macros));
        }
        if cx.checker().is_array_type_exported(target) {
            let element = cx.checker().get_type_arguments_exported(target).first().copied();
            // The union's members as written (in the call's type arguments, else in the alias it was declared as),
            // which is TS5's order (`targetType.types`, the order its checker made them in) for a union the call
            // writes out of new types; the rest by a key of each member (DESIGN §10.4).
            let constituents = match element {
                Some(e) if e.flags().intersects(TypeFlags::Union) => {
                    let written = written_union(cx, self.node, e);
                    crate::serializer::union_member_order(self.fw, cx, self.node, written, e)?
                }
                Some(e) => vec![e],
                None => Vec::new(),
            };
            let mut macros = Vec::new();
            for member in constituents {
                // `never` is what a union without members is: an empty array.
                if member.flags().intersects(TypeFlags::Never) {
                    break;
                }
                macros.push(self.of_many(cx, member)?);
            }
            return Ok(UserMacro::Many(macros));
        }
        if is_object_type(target) {
            // Keyed as the type declares its properties, in TS5's order.
            let properties = cx.ts5_properties_of_type(target);
            let mut entries = Vec::with_capacity(properties.len());
            for member in properties {
                // TS5 finds no type for a symbol key or a private name, and leaves the member out.
                let Some(member_type) = crate::guards::types::ts5_type_of_property(cx, target, member) else {
                    continue;
                };
                let value = self.of_many(cx, member_type)?;
                entries.push((property_key(cx, member), value));
            }
            return Ok(UserMacro::Map(entries));
        }
        match literal_of(target) {
            Some(TypeLiteral::Str(s)) if target.flags().intersects(TypeFlags::StringLiteral) => {
                return Ok(UserMacro::Literal(LiteralMacro::Str(s)));
            }
            Some(TypeLiteral::Num(n)) if target.flags().intersects(TypeFlags::NumberLiteral) => {
                return Ok(UserMacro::Literal(LiteralMacro::Num(n)));
            }
            _ => {}
        }
        if target.flags().intersects(TypeFlags::Undefined) {
            return Ok(UserMacro::Literal(LiteralMacro::Undefined));
        }
        if target.flags().intersects(TypeFlags::BooleanLiteral) {
            // TS5 compared the regular type with `true`; a literal type's value says it for fresh and regular alike.
            return Ok(UserMacro::Literal(LiteralMacro::Bool(literal_of(target) == Some(TypeLiteral::Bool(true)))));
        }

        let shown = cx.checker().type_to_string_exported(target);
        Err(cx.error(self.node, format!("Unknown type '{shown}' encountered")))
    }

    /// `getBasicUserMacro`.
    fn basic(&self, cx: &mut Cx<'_>, target: P<Type>) -> PResult<Option<UserMacro>> {
        if let Some(generic) = property(cx, target, "_flamework_macro_generic") {
            let Some(target_type) = property(cx, generic, "0") else { return Ok(None) };
            let Some(metadata_type) = property(cx, generic, "1") else { return Ok(None) };
            let Some(metadata) = string_literal(metadata_type) else {
                let shown = cx.checker().type_to_string_exported(metadata_type);
                return Err(cx.error(self.node, format!("Flamework encountered invalid metadata: '{shown}'")));
            };
            let kind = match metadata {
                "id" => GenericKind::Id,
                "guard" => GenericKind::Guard,
                "text" => GenericKind::Text,
                "dependency" | "dependencyConcise" => GenericKind::Dependency {
                    concise: metadata == "dependencyConcise",
                    metadata: self.injectable_metadata(cx, target_type)?,
                },
                _ => GenericKind::Other,
            };
            return Ok(Some(UserMacro::Generic { target: target_type, kind }));
        }

        if let Some(caller) = property(cx, target, "_flamework_macro_caller") {
            let Some(metadata) = string_literal(caller) else { return Ok(None) };
            let kind = match metadata {
                "line" => CallerKind::Line,
                "character" => CallerKind::Character,
                "width" => CallerKind::Width,
                "uuid" => CallerKind::Uuid,
                "text" => CallerKind::Text,
                _ => CallerKind::Other,
            };
            return Ok(Some(UserMacro::Caller(kind)));
        }

        if let Some(hash) = property(cx, target, "_flamework_macro_hash") {
            let text = property(cx, hash, "0");
            let context = property(cx, hash, "1");
            let is_obfuscation = property(cx, hash, "2").is_some();
            let Some(text) = text.and_then(string_literal) else { return Ok(None) };
            let Some(context) = context else { return Ok(None) };
            let context = string_literal(context).unwrap_or("@");
            let shared = &self.fw.shared;
            let value = if is_obfuscation {
                shared.obfuscate_text(text, Some(context))
            } else {
                shared.hash_string(text, Some(context))
            };
            return Ok(Some(UserMacro::Literal(LiteralMacro::Str(value))));
        }

        let non_nullable = cx.checker().get_non_nullable_type(target);
        if let Some(labels) = property(cx, non_nullable, "_flamework_macro_tuple_labels") {
            return Ok(Some(labels_of(labels)));
        }

        let Some(intrinsic) = property(cx, non_nullable, "_flamework_intrinsic") else { return Ok(None) };
        if !tsrs_checker::is_tuple_type_exported(intrinsic) {
            return Ok(None);
        }
        let arguments = cx.checker().get_type_arguments_exported(intrinsic).to_vec();
        let Some((&id, inputs)) = arguments.split_first() else { return Ok(None) };
        let Some(id) = string_literal(id) else { return Ok(None) };
        Ok(Some(UserMacro::Intrinsic(self.intrinsic(cx, id, inputs)?)))
    }

    /// The inputs of an intrinsic, checked as `buildIntrinsicMacro` checks them.
    fn intrinsic(&self, cx: &mut Cx<'_>, id: &str, inputs: &[P<Type>]) -> PResult<Intrinsic> {
        let invalid = || Intrinsic::Invalid("Invalid intrinsic usage".into());
        Ok(match id {
            "pathglob" => inputs.first().map_or_else(invalid, |&t| Intrinsic::PathGlob(t)),
            "path" => inputs.first().map_or_else(invalid, |&t| Intrinsic::Path(t)),
            "env" => match inputs.first() {
                Some(&name) => Intrinsic::Env { name, fallback: inputs.get(1).copied() },
                None => invalid(),
            },
            "obfuscate-obj" => match (inputs.first(), inputs.get(1)) {
                (Some(&inner), Some(&hash)) => Intrinsic::ObfuscateObject {
                    inner: Box::new(self.of_many(cx, inner)?),
                    context: string_literal(hash).map(str::to_owned),
                },
                _ => invalid(),
            },
            "shuffle-array" => match inputs.first() {
                Some(&inner) => Intrinsic::ShuffleArray(Box::new(self.of_many(cx, inner)?)),
                None => invalid(),
            },
            "plugin" => Intrinsic::Plugin {
                id: inputs.first().and_then(|&t| string_literal(t)).map(str::to_owned),
                input: inputs.get(1).copied(),
            },
            "tuple-guards" => inputs.first().map_or_else(invalid, |&t| Intrinsic::TupleGuards(t)),
            "serializer" => inputs.first().map_or_else(invalid, |&t| Intrinsic::Serializer(t)),
            // `[T, F?, K?]` for an argument list, `[F, K?]` for a result and for `network-unreliable`.
            "network-decoder" => match inputs.first() {
                Some(&list) => Intrinsic::NetworkDecoder { list, member: inputs.get(1).copied(), key: inputs.get(2).copied() },
                None => invalid(),
            },
            "network-result-decoder" => match inputs.first() {
                Some(&function) => Intrinsic::NetworkResultDecoder { function, key: inputs.get(1).copied() },
                None => invalid(),
            },
            "network-unreliable" => {
                Intrinsic::NetworkUnreliable { member: inputs.first().copied(), key: inputs.get(1).copied() }
            }
            _ => Intrinsic::Invalid(format!("Unexpected intrinsic ID '{id}' with {} inputs", inputs.len())),
        })
    }

    /// `getInjectableMetadata`: the macros of `Modding.Injectable<{ metadata: [...] }>`.
    pub(crate) fn injectable_metadata(&self, cx: &mut Cx<'_>, ty: P<Type>) -> PResult<Option<Vec<UserMacro>>> {
        let Some(config) = property(cx, ty, "_flamework_injectable") else { return Ok(None) };
        let Some(reflect) = property(cx, config, "metadata") else { return Ok(None) };
        if !tsrs_checker::is_tuple_type_exported(reflect) {
            return Ok(None);
        }
        let members = cx.checker().get_type_arguments_exported(reflect).to_vec();
        let mut macros = Vec::with_capacity(members.len());
        for member in members {
            macros.push(self.of_many(cx, member)?);
        }
        Ok(Some(macros))
    }
}

/// `isObjectType`: an object type, or an intersection of them.
fn is_object_type(ty: P<Type>) -> bool {
    if ty.flags().intersects(TypeFlags::Intersection) {
        return ty.types().iter().all(|&t| is_object_type(t));
    }
    ty.flags().intersects(TypeFlags::Object)
}

/// `getLabels`: a tuple's element names (`""` for an unnamed one); `nil` for anything else.
fn labels_of(ty: P<Type>) -> UserMacro {
    if !tsrs_checker::is_tuple_type_exported(ty) {
        return UserMacro::Literal(LiteralMacro::Undefined);
    }
    let Some(target) = ty.target() else { return UserMacro::Literal(LiteralMacro::Undefined) };
    let names = target
        .as_tuple_type()
        .element_infos()
        .iter()
        .map(|info| match info.labeled_declaration() {
            Some(declaration) => match declaration.name() {
                Some(name) if name.kind() == Kind::Identifier => UserMacro::Literal(LiteralMacro::Str(name.text().into())),
                _ => UserMacro::Literal(LiteralMacro::Undefined),
            },
            // An unnamed element (TypeScript 5.0+ allows them).
            None => UserMacro::Literal(LiteralMacro::Str(String::new())),
        })
        .collect();
    UserMacro::Many(names)
}

/// `getPropertyKey`: the key a type declares for a property, as `keyof` works it out
/// (`getLiteralTypeFromProperty`): the literal type TypeScript keeps on the symbols it makes (a mapped type's,
/// `Record<10, V>`), else the declaration's name (a numeric literal or a computed number is a number).
fn property_key(cx: &mut Cx<'_>, property: P<Symbol>) -> TableKey {
    let key = cx.checker().get_literal_type_from_property(property, TypeFlags::NumberLiteral, true);
    if let Some(TypeLiteral::Num(n)) = literal_of(key) {
        return TableKey::Num(n);
    }
    // TS's `getPropertyKey` reads `valueDeclaration ?? declarations[0]`, the checker only the value declaration. A
    // symbol TypeScript copies without one (a readonly property spread into an object literal, a reverse-mapped
    // property) keeps its declarations: read the first, as TS does, unless the symbol carries a key type (which the
    // checker read: TS returns the name then).
    let has_name_type = property.flags().intersects(SymbolFlags::Transient)
        && cx.checker().get_name_type_of_symbol(property).is_some();
    if property.value_declaration().is_some() || has_name_type {
        return TableKey::Str(property.name().to_owned());
    }
    if let Some(name) = property.declarations().first().copied().and_then(tsrs_ast::get_name_of_declaration) {
        match name.kind() {
            Kind::NumericLiteral => {
                if let Ok(value) = name.text().parse::<f64>() {
                    return TableKey::Num(value);
                }
            }
            Kind::ComputedPropertyName => {
                let expression = name.as_computed_property_name().expression;
                let ty = cx.checker().get_type_at_location(expression);
                if let Some(TypeLiteral::Num(n)) = literal_of(ty)
                    && ty.flags().intersects(TypeFlags::NumberLiteral)
                {
                    return TableKey::Num(n);
                }
            }
            _ => {}
        }
    }
    TableKey::Str(property.name().to_owned())
}

/// Builds macro values (`buildUserMacro`) at one generated argument: ids, paths and globs, and env reads are
/// recorded with that argument's place in the TS transformer's walk.
pub(crate) struct Builder<'f> {
    pub fw: &'f FwInstance,
    /// The macro call.
    pub node: P<Node>,
    pub key: OrderKey,
}

impl Builder<'_> {
    /// `buildUserMacro`.
    pub(crate) fn build(&self, cx: &mut Cx<'_>, found: &UserMacro) -> PResult<LuauExpr> {
        let fw = self.fw;
        match found {
            UserMacro::Generic { target, kind } => match kind {
                GenericKind::Id => {
                    let id = fw.type_uid(cx, *target, self.node)?;
                    Ok(fw.id_expr(cx, id))
                }
                GenericKind::Text => Ok(synthesized_string(&cx.checker().type_to_string_exported(*target))),
                GenericKind::Dependency { concise, metadata } => {
                    self.dependency(cx, *target, *concise, metadata.as_deref())
                }
                // `buildGuardFromTypeWithDedup` at the call, its dedup locals ahead of the statement (`prereqList`).
                GenericKind::Guard => {
                    crate::guards::guard_for_with_dedup(cx, fw.shared.guard_config(), self.node, *target)
                }
                GenericKind::Other => Ok(create::nil()),
            },
            UserMacro::Caller(kind) => self.caller(cx, *kind),
            UserMacro::Many(items) => {
                let mut values = Vec::with_capacity(items.len());
                for item in items {
                    values.push(self.build(cx, item)?);
                }
                Ok(create::array(values))
            }
            UserMacro::Map(entries) => {
                let mut fields = Vec::with_capacity(entries.len());
                for (key, item) in entries {
                    let value = self.build(cx, item)?;
                    if matches!(value, LuauExpr::Nil) {
                        continue;
                    }
                    let key = match key {
                        TableKey::Str(s) => synthesized_string(s),
                        TableKey::Num(n) => create::number(*n),
                    };
                    fields.push((key, value));
                }
                Ok(create::map(fields))
            }
            UserMacro::Literal(literal) => Ok(match literal {
                LiteralMacro::Str(s) => synthesized_string(s),
                LiteralMacro::Num(n) => create::number(*n),
                LiteralMacro::Bool(b) => create::bool(*b),
                LiteralMacro::Undefined => create::nil(),
            }),
            UserMacro::Intrinsic(intrinsic) => self.intrinsic(cx, intrinsic),
            UserMacro::SharedRef(inner) => self.shared_ref(cx, inner),
        }
    }

    /// `getDependencyInjectionMetadata`: the id, as a string when concise and there is no injectable metadata, else
    /// `{ id, metadata? }`.
    pub(crate) fn dependency(
        &self,
        cx: &mut Cx<'_>,
        ty: P<Type>,
        concise: bool,
        metadata: Option<&[UserMacro]>,
    ) -> PResult<LuauExpr> {
        let id = self.fw.type_uid(cx, ty, self.node)?;
        let id = self.fw.id_expr(cx, id);
        let Some(metadata) = metadata else {
            return Ok(if concise { id } else { create::map(vec![(create::string("id"), id)]) });
        };
        let mut values = Vec::with_capacity(metadata.len());
        for item in metadata {
            values.push(self.build(cx, item)?);
        }
        Ok(create::map(vec![(create::string("id"), id), (create::string("metadata"), create::array(values))]))
    }

    /// `getCallerMetadata`: where the call is written, in UTF-16.
    fn caller(&self, cx: &mut Cx<'_>, kind: CallerKind) -> PResult<LuauExpr> {
        let pos = cx.utf16_pos(self.node);
        Ok(match kind {
            CallerKind::Line => create::number(f64::from(pos.line + 1)),
            CallerKind::Character => create::number(f64::from(pos.character + 1)),
            CallerKind::Width => create::number(f64::from(cx.utf16_width(self.node))),
            CallerKind::Uuid => synthesized_string(&self.callsite_uuid(cx)),
            CallerKind::Text => synthesized_string(cx.source_text(self.node)),
            CallerKind::Other => create::nil(),
        })
    }

    /// `getCallsiteUuid`: v5 of `<package>:<fileId>@<declaration>+<offset>`, the offset in UTF-16 from the
    /// enclosing named declaration's start.
    fn callsite_uuid(&self, cx: &mut Cx<'_>) -> String {
        let shared = &self.fw.shared;
        let mut declaration = None;
        let mut current = Some(self.node);
        while let Some(node) = current {
            if matches!(
                node.kind(),
                Kind::VariableDeclaration
                    | Kind::FunctionDeclaration
                    | Kind::MethodDeclaration
                    | Kind::PropertyDeclaration
                    | Kind::ClassDeclaration
                    | Kind::GetAccessor
                    | Kind::SetAccessor
            ) {
                declaration = Some(node);
                break;
            }
            current = node.parent();
        }
        let start = cx.utf16_pos(self.node).offset;
        let (name, offset) = match declaration {
            Some(declaration) => {
                let name = declaration.name().filter(|n| n.kind() == Kind::Identifier).map_or("", |n| n.text());
                (name, start - cx.utf16_pos(declaration).offset)
            }
            None => ("", start),
        };
        let file_id = shared.file_id(&cx.file().path);
        let seed = if shared.config.obfuscation == Some(true) { shared.build_seed.as_ref() } else { None };
        crate::ids::callsite_uuid(&shared.package_name, &file_id, name, offset, seed)
    }

    /// `sharedRef`: the value, hoisted to a local before the root statement unless it is a literal or already at the
    /// file's root.
    fn shared_ref(&self, cx: &mut Cx<'_>, inner: &UserMacro) -> PResult<LuauExpr> {
        let result = self.build(cx, inner)?;
        if is_simple_inlineable(&result) || is_at_root(self.node) {
            return Ok(result);
        }
        let line = cx.utf16_pos(self.node).line + 1;
        let debug = debug_name(cx, self.node);
        let name = local_name(cx, &format!("{debug}_{line}"));
        cx.insert_before_root_statement([create::comment(format!(" Flamework user macro metadata (line {line})"))]);
        // A spillable hoist named as the TS's `f.identifier(name, true)` (`name`, else `name_1`, …): past the main
        // chunk's local budget the compiler makes it a field of one table, which is what the TS transformer's
        // `macroMetadata` table was for.
        let local = cx.hoist_spillable_named(result, &name);
        // The TS's `state.prereq(comment)`: ahead of the TS statement around the call.
        let comment = format!(" Flamework hoisted this macro's metadata ({name}) to the file root.");
        cx.insert_before_statement([create::comment(comment)]);
        Ok(local)
    }

    fn intrinsic(&self, cx: &mut Cx<'_>, intrinsic: &Intrinsic) -> PResult<LuauExpr> {
        let fw = self.fw;
        match intrinsic {
            Intrinsic::Path(ty) => build_path(fw, cx, self.node, *ty, self.key.clone()),
            Intrinsic::PathGlob(ty) => build_path_glob(fw, cx, self.node, *ty, self.key.clone()),
            Intrinsic::Env { name, fallback } => self.env(cx, *name, *fallback),
            Intrinsic::ObfuscateObject { inner, context } => {
                let mut inner = (**inner).clone();
                if let UserMacro::Map(entries) = &mut inner {
                    let mut shuffled = std::mem::take(entries);
                    self.obfuscate_order(cx, &mut shuffled, "obfuscate-obj");
                    for (key, _) in &mut shuffled {
                        // A number key is no name the runtime looks up.
                        if let TableKey::Str(name) = key {
                            *name = fw.shared.obfuscate_text(name, context.as_deref());
                        }
                    }
                    *entries = shuffled;
                }
                self.build(cx, &inner)
            }
            Intrinsic::ShuffleArray(inner) => {
                let mut inner = (**inner).clone();
                if let UserMacro::Many(items) = &mut inner {
                    self.obfuscate_order(cx, items, "shuffle-array");
                }
                self.build(cx, &inner)
            }
            Intrinsic::Plugin { id, input } => {
                let (Some(id), Some(input)) = (id, input) else {
                    return Err(cx.error(
                        self.node,
                        "A plugin macro must be declared as `Modding.Intrinsic<\"plugin\", [id, T], R>` where `id` is a string literal.",
                    ));
                };
                crate::macro_type::execute(fw, cx, self.node, id, *input)
            }
            Intrinsic::TupleGuards(ty) => crate::guards::tuple_guards(cx, fw.shared.guard_config(), self.node, *ty),
            Intrinsic::Serializer(ty) => {
                let version = serializer_version_at(fw, cx, self.node)?;
                crate::serializer::build_serializer(fw, cx, self.node, *ty, version)
            }
            // Networking metadata: a decoder only for a packed member, `nil` (values pass through) otherwise.
            Intrinsic::NetworkDecoder { list, member, key } => {
                if !crate::networking::mode::check_packed(fw, cx, *member, self.node, *key)? {
                    return Ok(create::nil());
                }
                crate::serializer::build_decoder(fw, cx, self.node, *list, false)
            }
            Intrinsic::NetworkResultDecoder { function, key } => {
                if !crate::networking::mode::check_packed(fw, cx, Some(*function), self.node, *key)? {
                    return Ok(create::nil());
                }
                crate::serializer::build_decoder(fw, cx, self.node, *function, true)
            }
            // Whether an event is unreliable, and the one place every member of an event network passes through, so
            // its markers are checked.
            Intrinsic::NetworkUnreliable { member, key } => {
                crate::networking::mode::check_packed(fw, cx, *member, self.node, *key)?;
                let unreliable = crate::networking::mode::has_marker(cx, *member, "_flamework_unreliable");
                Ok(if unreliable { create::bool(true) } else { create::nil() })
            }
            Intrinsic::Invalid(message) => Err(cx.error(self.node, message.clone())),
        }
    }

    /// `obfuscateArray`: under obfuscation, a shuffle seeded by the build seed, the file, the call's place and what
    /// is shuffled (DESIGN §10.4), so it is the same on every checker.
    fn obfuscate_order<T>(&self, cx: &mut Cx<'_>, items: &mut [T], context: &str) {
        let shared = &self.fw.shared;
        if shared.config.obfuscation != Some(true) {
            return;
        }
        if shared.build_seed.is_none() {
            return;
        }
        let mut count = self.fw.file.borrow().shuffles.get(&self.node).copied().unwrap_or(0);
        shuffle(self.fw, cx, self.node, items, context, &mut count);
        self.fw.file.borrow_mut().shuffles.insert(self.node, count);
    }

    /// `buildEnvIntrinsic`: the variable's value as a string literal, else the fallback, else `nil`; recorded in
    /// `flamework.build`'s `inputs.env`.
    fn env(&self, cx: &mut Cx<'_>, name: P<Type>, fallback: Option<P<Type>>) -> PResult<LuauExpr> {
        let Some(name) = string_literal(name) else {
            let shown = cx.checker().type_to_string_exported(name);
            return Err(cx.error(
                self.node,
                format!("Flamework.env expects the variable's name as a string literal, got: {shown}"),
            ));
        };
        let mut fallback_value = None;
        if let Some(fallback) = fallback
            && !fallback.flags().intersects(TypeFlags::Undefined)
        {
            let Some(text) = string_literal(fallback) else {
                let shown = cx.checker().type_to_string_exported(fallback);
                let diagnostic = PluginDiagnostic::error(
                    self.node,
                    format!("Flamework.env expects the fallback as a string literal, got: {shown}"),
                )
                .with_hint(
                    "The fallback is inlined at compile time, which is what makes the result a string rather than string | undefined.",
                );
                return Err(cx.report(diagnostic).expect("an error"));
            };
            fallback_value = Some(text);
        }
        let variable = self.fw.shared.env(cx, name);
        let hash = crate::env::hash_env_value(variable.as_deref());
        let mut file = self.fw.file.borrow_mut();
        let seq = file.next();
        file.env_inputs.push((self.key.clone(), seq, (name.to_owned(), hash)));
        drop(file);
        Ok(match variable.as_deref().or(fallback_value) {
            Some(value) => synthesized_string(value),
            None => create::nil(),
        })
    }
}

/// `ts.isSimpleInlineableExpression` on what the value lowers to: a string, number or boolean literal.
fn is_simple_inlineable(expression: &LuauExpr) -> bool {
    matches!(
        expression,
        LuauExpr::String(_) | LuauExpr::LateString(_) | LuauExpr::Number(_) | LuauExpr::True | LuauExpr::False
    )
}

/// The TS transformer's "already at the next root" test: the nearest expression statement around `node` is one of
/// the file's (`findAncestor(node, f.is.statement)`, where `f.is.statement` is `isExpressionStatement`). A macro
/// in a variable statement at the root is hoisted all the same.
fn is_at_root(node: P<Node>) -> bool {
    let mut current = Some(node);
    while let Some(n) = current {
        if n.kind() == Kind::ExpressionStatement {
            return n.parent().is_some_and(|p| p.kind() == Kind::SourceFile);
        }
        current = n.parent();
    }
    false
}

/// `getNodeDebugName`: the callee's name for a call, else `macro`.
fn debug_name(cx: &mut Cx<'_>, node: P<Node>) -> String {
    if node.kind() == Kind::CallExpression
        && let Some(callee) = node.expression()
        && let Some(symbol) = crate::metadata::get_symbol(cx, callee)
    {
        return symbol.name().to_owned();
    }
    "macro".to_owned()
}

/// Names a generated local cannot take (`identifierName.ts`): JavaScript's reserved words, strict mode's, Luau's
/// keywords and the Luau globals roblox-ts reserves.
const UNUSABLE: &[&str] = &[
    "break", "case", "catch", "class", "const", "continue", "debugger", "default", "delete", "do", "else", "enum",
    "export", "extends", "false", "finally", "for", "function", "if", "import", "in", "instanceof", "new", "null",
    "return", "super", "switch", "this", "throw", "true", "try", "typeof", "var", "void", "while", "with", "await",
    "implements", "interface", "let", "package", "private", "protected", "public", "static", "yield", "arguments",
    "eval", "and", "elseif", "end", "local", "nil", "not", "or", "repeat", "then", "until", "_G", "TS", "assert",
    "bit32", "coroutine", "error", "exports", "game", "getmetatable", "ipairs", "math", "next", "pairs", "pcall",
    "require", "script", "select", "self", "setmetatable", "string", "table", "tostring", "type", "unpack", "utf8",
];

/// A local named `hint` (made usable with [`local_name`]) holding `value`, declared before the current root
/// statement: `f.identifier(name, true)`'s optimistic unique name, the hint itself when the file has no such name,
/// else `hint_1`, `hint_2`, …
pub(crate) fn hoist_named(cx: &mut Cx<'_>, value: LuauExpr, hint: &str) -> LuauExpr {
    let base = local_name(cx, hint);
    let mut name = base.clone();
    let stem = if base.ends_with('_') { base.clone() } else { format!("{base}_") };
    let mut suffix = 1;
    while !cx.is_usable_local_name(&name) {
        name = format!("{stem}{suffix}");
        suffix += 1;
    }
    cx.insert_before_root_statement([create::local(create::any_id(name.clone()), Some(value))]);
    create::id(name)
}

/// `localName`: a name from the project's code made into one a generated local can have: other characters as `_`,
/// and `v_` ahead of it when it is no usable name or names a global.
pub(crate) fn local_name(cx: &mut Cx<'_>, hint: &str) -> String {
    match usable_name(hint) {
        Ok(name) => {
            let meaning = SymbolFlags::Value | SymbolFlags::Namespace;
            let global = cx.checker().resolve_name_exported(&name, None, meaning, false);
            if global.is_some() { format!("v_{name}") } else { name }
        }
        Err(name) => name,
    }
}

/// [`local_name`] without the global check: `Ok` with a name that may still name a global, `Err` with a final
/// `v_` name.
fn usable_name(hint: &str) -> Result<String, String> {
    let mut name = String::with_capacity(hint.len());
    for c in hint.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            name.push(c);
        } else {
            // `\W` without the `u` flag matches each UTF-16 unit.
            for _ in 0..c.len_utf16() {
                name.push('_');
            }
        }
    }
    let valid = name.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
    if !valid || UNUSABLE.contains(&name.as_str()) { Err(format!("v_{name}")) } else { Ok(name) }
}

/// `getMacroCall`: the macro's name and where it is, one-based, in UTF-16. A method call is placed at the method's
/// name (a chain of registrations is one expression, and every call in it starts where the chain does).
/// `qualified` also names the receiver when it is a plain name (`ComponentPlugin.fromPath`).
pub(crate) fn macro_call_shape(cx: &Cx<'_>, call: P<Node>, qualified: bool) -> (String, u32, u32) {
    let mut name = "a macro".to_owned();
    let mut anchor = call;
    if matches!(call.kind(), Kind::CallExpression | Kind::NewExpression)
        && let Some(callee) = call.expression()
    {
        match callee.kind() {
            Kind::PropertyAccessExpression => {
                let access = callee.as_property_access_expression();
                let method = access.name.text();
                let object = access.expression;
                name = if qualified && object.kind() == Kind::Identifier {
                    format!("{}.{method}", object.text())
                } else {
                    method.to_owned()
                };
                anchor = access.name;
            }
            Kind::Identifier => name = callee.text().to_owned(),
            _ => {}
        }
    }
    let pos = cx.utf16_pos(anchor);
    (name, pos.line + 1, pos.character + 1)
}

/// A path literal's text, or the TS transformer's error.
fn path_literal(cx: &mut Cx<'_>, node: P<Node>, ty: P<Type>) -> PResult<&'static str> {
    match string_literal(ty) {
        Some(value) => Ok(value),
        None => {
            let shown = cx.checker().type_to_string_exported(ty);
            Err(cx.error(node, format!("Path is invalid, expected string literal and got: {shown}")))
        }
    }
}

/// `buildPathIntrinsic`: the Rojo path of a source path, as an array of names, with the use recorded.
fn build_path(fw: &FwInstance, cx: &mut Cx<'_>, call: P<Node>, ty: P<Type>, key: OrderKey) -> PResult<LuauExpr> {
    let shared = &fw.shared;
    let value = path_literal(cx, call, ty)?;
    // Source paths are relative to the working directory the TS transformer runs in: the project's.
    let source = node_path::resolve(&[&shared.project_dir, value]);
    let output = shared.path_translator.get_output_path(&source);
    let rbx_path = if shared.has_rojo { cx.project().rojo_resolver.get_rbx_path_from_file_path(&output) } else { None };
    let Some(rbx_path) = rbx_path else {
        let shown = node_path::relative(&shared.project_dir, &output).replace('\\', "/");
        let hint = if shared.has_rojo {
            format!(
                "It compiles to '{shown}', and no $path in your Rojo project covers that. Give the source path of a folder the project maps, relative to the project, such as \"src/server/commands\"."
            )
        } else {
            "No Rojo project file was found, so a source path cannot be turned into a Rojo path.".to_owned()
        };
        let diagnostic =
            PluginDiagnostic::error(call, format!("Could not find Rojo data for '{value}'")).with_hint(hint);
        return Err(cx.report(diagnostic).expect("an error"));
    };

    let (macro_name, line, column) = macro_call_shape(cx, call, true);
    let raises = is_core_require_modules(cx, call);
    let mut file = fw.file.borrow_mut();
    let seq = file.next();
    file.events.push((key, seq, PathEvent::Path(PathUse { path: value.to_owned(), macro_name, line, column, raises })));
    drop(file);
    Ok(create::array(rbx_path.iter().map(|name| synthesized_string(name)).collect()))
}

/// `buildPathGlobIntrinsic`: the glob relative to the package root (obfuscated under obfuscation), recorded for
/// `globs.json`.
fn build_path_glob(
    fw: &FwInstance,
    cx: &mut Cx<'_>,
    call: P<Node>,
    ty: P<Type>,
    key: OrderKey,
) -> PResult<LuauExpr> {
    let shared = &fw.shared;
    let glob = path_literal(cx, call, ty)?;
    let file_name = cx.file().path.clone();
    let absolute = if glob.starts_with('.') {
        let resolved = node_path::resolve(&[&node_path::dirname(&file_name), glob]);
        node_path::relative(&shared.root_dir, &resolved).replace('\\', "/")
    } else {
        glob.to_owned()
    };
    let (macro_name, line, column) = macro_call_shape(cx, call, false);
    let mut file = fw.file.borrow_mut();
    let seq = file.next();
    let glob_use = GlobUse { glob: absolute.clone(), text: glob.to_owned(), macro_name, line, column };
    file.events.push((key, seq, PathEvent::Glob(glob_use)));
    drop(file);
    Ok(synthesized_string(&shared.obfuscate_text(&absolute, Some("addPaths"))))
}

/// `isCoreRequireModules`: the call is core's `requireModules`, which raises after five seconds rather than waiting.
fn is_core_require_modules(cx: &mut Cx<'_>, call: P<Node>) -> bool {
    if call.kind() != Kind::CallExpression {
        return false;
    }
    let signature = cx.checker().get_resolved_signature_exported(call);
    let Some(declaration) = signature.declaration() else { return false };
    if declaration.kind() != Kind::FunctionDeclaration
        || declaration.name().map(|n| n.text()) != Some("requireModules")
    {
        return false;
    }
    crate::class_hook::declaring_package(cx, declaration).as_deref() == Some(CORE_PACKAGE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_names_are_made_usable_as_the_ts_transformer_makes_them() {
        assert_eq!(usable_name("withEmit_17"), Ok("withEmit_17".into()));
        assert_eq!(usable_name("my-plugin.id"), Ok("my_plugin_id".into()));
        // `W` without the `u` flag: one `_` per UTF-16 unit.
        assert_eq!(usable_name("é😀"), Ok("___".into()));
        assert_eq!(usable_name("10x"), Err("v_10x".into()));
        assert_eq!(usable_name("end"), Err("v_end".into()));
        assert_eq!(usable_name("script"), Err("v_script".into()));
        assert_eq!(usable_name(""), Err("v_".into()));
    }

}
