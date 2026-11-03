//! Networking call sites (`transformNetworkingCall.ts`): `transformSend`, `transformReceiverCallback`,
//! `bindArgument` and `emitWithStatements` (`guardOptionalChain` is not ported: see [`call_hook`]).
//!
//! **What a site becomes.** The TS transformer emitted TypeScript statements that roblox-ts compiled ahead of the
//! call: the target bound to a local when it is more than a plain read, each argument that is more than an identifier
//! or a literal bound to a local (`bindArgument`), the encoding of the packed values (the serializer's
//! `buildInlineEncoding`), and then `target._fire(lead…, payload, blobs?)`. `setCallback(cb)` becomes
//! `target._setCallback(callback, function(value) <pack> end)`. The same statements are built here, the encoding by
//! the serializer's generator ([`crate::serializer`]) and the bindings as roblox-ts compiled them, from the arguments
//! compiled by the compiler in the TS statements' order.
//!
//! **Where it goes** (`emitWithStatements`, decided when the site is built at its hook, with its `LocalBudget`
//! claim): ahead of the statement around the call when that runs them exactly when the call runs
//! (`hoistsCleanly`) and the scope's budget of Flamework locals has room; in a block with the statement past the
//! budget; in a function called where the call is otherwise. A call reached through `?.` (`a?.b.fire(x)`) is
//! lowered by the compiler inside the chain's nil check, its statements there: the chain's offer of its outermost
//! call (`Interest::calls_chain_outer`) is declined, so the compiler evaluates each operand ahead of a `?.` once,
//! into its own local, before the arguments, and the send goes through those values, as JavaScript does. (The TS
//! transformer's `guardOptionalChain` checked the operands, packed, then read the chain again for the send: an
//! argument that clears or replaces an operand changed or broke the send; README, Differences.)
//!
//! **Names.** Each local is named when the hook first emits it ([`crate::serializer::local_name`]).

use rbxts_luau::{AssignmentOperator, BinaryOperator, Expr, Stmt, UnaryOperator, create};
use rbxts_plugin::ts::{Kind, Node, P, Type};
use tsrs_checker::{ObjectFlags, TypeFlags};
use rbxts_plugin::{CallSite, Claim, Cx, LuauExpr, PResult};

pub(crate) use super::analysis::CallShape;
use super::scope::{self, COMPACT_LOCALS, Scope};
use crate::plugin::FwInstance;
use crate::serializer::ir::{self, Body, E, ExtLiteral, ExtValue, Id, S, Ty};
use crate::serializer::{EncodingSite, GResult, Gen, local_name};

/// Where a call site's statements go (`emitWithStatements`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Placement {
    /// No statements: the call alone.
    Bare,
    /// Ahead of the statement around the call (`prereqList`).
    Ahead,
    /// Ahead of the statement, in a block with it (`scopedStatements`).
    Scoped,
    /// In a function called where the call is.
    Function,
    /// Reached through `?.`: where the call is evaluated, inside the compiler's nil check.
    Chain,
}

/// A value the site reads: an argument, bound to a local unless it is an identifier or a literal.
#[derive(Debug)]
pub(crate) struct Value {
    pub node: P<Node>,
    /// The local (`bindArgument`), or `None` when the value is read where it is.
    pub local: Option<Id>,
    /// The value as the packing reads it when it has no local: its Luau, filled in at the hook.
    pub ext: Option<ExtValue>,
}

/// A networking call site's code, built at its hook ([`crate::serializer::call_site`]).
#[derive(Debug)]
pub struct BuiltCall {
    pub(crate) shape: CallShape,
    pub(crate) placement: Placement,
    /// The target's local, when it is bound.
    target: Option<Id>,
    /// The arguments ahead of the packed ones (players, a timeout), passed through.
    leading: Vec<Value>,
    /// The packed arguments, one by one.
    packed: Vec<Value>,
    /// The packed arguments' table (`args`), when a spread among them makes their number a runtime matter.
    spread: Option<(Id, Vec<P<Node>>)>,
    /// `setCallback`'s callback argument and its local.
    callback: Option<(P<Node>, Id)>,
    /// The packing.
    encoding: Vec<S>,
    payload: Option<Id>,
    blobs: Option<Id>,
    /// `setCallback`'s packer: `(value) => { <pack>; return [payload, blobs?] }`.
    pack: Option<E>,
}

impl BuiltCall {
    /// The locals it declares, in the order its statements declare them.
    fn declared(&self) -> Vec<Id> {
        let mut ids: Vec<Id> = self.target.into_iter().collect();
        ids.extend(self.leading.iter().filter_map(|v| v.local));
        ids.extend(self.packed.iter().filter_map(|v| v.local));
        ids.extend(self.spread.as_ref().map(|(id, _)| *id));
        ids.extend(self.callback.map(|(_, id)| id));
        ids
    }
}

/// `state.isUserMacro(symbol)` of the callee: such a call is the user macro's, never networking's.
fn is_user_macro_call(fw: &FwInstance, cx: &mut Cx<'_>, call: P<Node>) -> bool {
    let Some(callee) = call.expression() else { return false };
    let Some(symbol) = cx.checker().get_symbol_at_location_exported(callee) else { return false };
    let symbol = cx.checker().skip_alias(symbol);
    fw.is_user_macro(cx, symbol)
}

// --- building ------------------------------------------------------------------------------------------------------

/// Builds a call site's code with the file's generator: its bindings, its packing, and where it goes.
pub(crate) fn build(gn: &mut Gen<'_, '_, '_>, node: P<Node>, shape: &CallShape) -> GResult<BuiltCall> {
    match shape {
        CallShape::Send { target, list, rest_index, .. } => build_send(gn, node, shape, *target, *list, *rest_index),
        CallShape::Callback { target, function } => build_callback(gn, node, shape, *target, *function),
    }
}

fn build_send(
    gn: &mut Gen<'_, '_, '_>,
    node: P<Node>,
    shape: &CallShape,
    target: P<Node>,
    list: P<rbxts_plugin::ts::Type>,
    rest_index: usize,
) -> GResult<BuiltCall> {
    let arguments = node.arguments();
    let lead = rest_index.min(arguments.len());
    if arguments[..lead].iter().any(|a| a.kind() == Kind::SpreadElement) {
        return Err(gn.cx.error(
            node,
            "Flamework cannot pack this call: a spread argument ahead of the payload is not supported for a packed member (networking.serialization or Serialized).",
        ));
    }
    let target_local = (!scope::is_pure(target)).then(|| gn.uid("target", Ty::Unknown));
    let leading: Vec<Value> = arguments[..lead].iter().map(|&a| value_of(gn, a, "target")).collect();
    let packed_nodes = &arguments[lead..];

    let mut packed = Vec::new();
    let mut spread = None;
    let values = if packed_nodes.iter().any(|a| a.kind() == Kind::SpreadElement) {
        for &element in packed_nodes {
            if element.kind() == Kind::SpreadElement {
                let inner = element.expression().expect("a spread has an expression");
                if spread_kind(gn.cx, inner).is_none() {
                    return Err(gn.cx.error(
                        element,
                        "Flamework cannot pack this spread argument: a packed call (networking.serialization or Serialized) spreads arrays, tuples, strings and sets only.",
                    ));
                }
            }
        }
        // A spread makes the count a runtime matter: gather the list first, as `fire` itself would.
        let table = gn.uid("args", Ty::Array);
        spread = Some((table, packed_nodes.to_vec()));
        crate::serializer::codegen::SiteValues::Table(ir::cast(ir::id(table), Ty::Array))
    } else {
        packed = packed_nodes.iter().map(|&a| value_of(gn, a, "arg")).collect();
        crate::serializer::codegen::SiteValues::List(packed.iter().map(Value::as_e).collect())
    };

    // The packing lands among the locals of the function the call is in: its own, those Flamework's code ahead of it
    // holds there still, and the ones bound above.
    let bound = u32::from(target_local.is_some())
        + leading.iter().filter(|v| v.local.is_some()).count() as u32
        + packed.iter().filter(|v| v.local.is_some()).count() as u32
        + u32::from(spread.is_some());
    let outer = live_locals(gn, node) + bound;
    let name = member_name(gn, target);
    let shape_list = gn.list_of(list, None)?;
    let encoding = gn.encode_elements(shape_list, values, EncodingSite { name, result: false, outer })?;

    let mut call = BuiltCall {
        shape: shape.clone(),
        placement: Placement::Bare,
        target: target_local,
        leading,
        packed,
        spread,
        callback: None,
        encoding: encoding.statements,
        payload: encoding.payload,
        blobs: encoding.blobs,
        pack: None,
    };
    let mut hoisted = vec![target];
    hoisted.extend_from_slice(arguments);
    place(gn, node, &mut call, &hoisted);
    Ok(call)
}

fn build_callback(
    gn: &mut Gen<'_, '_, '_>,
    node: P<Node>,
    shape: &CallShape,
    target: P<Node>,
    function: P<rbxts_plugin::ts::Type>,
) -> GResult<BuiltCall> {
    let callback = *node.arguments().first().expect("a callback argument");
    // The target is bound unless it is an identifier (`bindArgument` of the chained target).
    let target_local = (!is_simple(target)).then(|| gn.uid("target", Ty::Unknown));
    let callback_local = gn.uid("callback", Ty::Function);
    let value = gn.parameter("value", Ty::Unknown);
    let name = member_name(gn, target);
    let list = gn.result_of(function, node)?;
    let encoding = gn.encode_elements(
        list,
        crate::serializer::codegen::SiteValues::List(vec![ir::id(value)]),
        EncodingSite { name, result: true, outer: 0 },
    )?;
    // The runtime calls `pack` only with a resolved, successful value.
    let mut body = encoding.statements;
    body.push(S::Return(Some(match encoding.payload {
        Some(payload) => {
            let mut packed = vec![ir::id(payload)];
            packed.extend(encoding.blobs.map(ir::id));
            ir::cast(ir::array(packed), Ty::Array)
        }
        None => ir::nil(),
    })));
    let pack = ir::arrow(vec![ir::param(value)], Body::Block(body));

    let mut call = BuiltCall {
        shape: shape.clone(),
        placement: Placement::Bare,
        target: target_local,
        leading: Vec::new(),
        packed: Vec::new(),
        spread: None,
        callback: Some((callback, callback_local)),
        encoding: Vec::new(),
        payload: None,
        blobs: None,
        pack: Some(pack),
    };
    place(gn, node, &mut call, &[target, callback]);
    Ok(call)
}

impl Value {
    /// The value as the packing reads it.
    fn as_e(&self) -> E {
        match (self.local, &self.ext) {
            (Some(local), _) => ir::id(local),
            (None, Some(ext)) => E::Ext(ext.clone()),
            (None, None) => ir::nil(),
        }
    }
}

/// `bindArgument`: an argument read where it is (an identifier or a literal), else bound to a new local (`hint`).
fn value_of(gn: &mut Gen<'_, '_, '_>, node: P<Node>, hint: &str) -> Value {
    if node.kind() == Kind::Identifier && node.text() == "undefined" {
        return Value { node, local: None, ext: None };
    }
    if is_simple(node) {
        let literal = match node.kind() {
            Kind::Identifier => None,
            Kind::TrueKeyword => Some(ExtLiteral::Bool(true)),
            Kind::FalseKeyword => Some(ExtLiteral::Bool(false)),
            Kind::NumericLiteral => Some(ExtLiteral::Num(crate::js::string_to_number(node.text()))),
            _ => Some(ExtLiteral::Str(node.text().to_owned())),
        };
        let parameter = literal.is_none() && is_parameter_reference(gn.cx, node);
        return Value { node, local: None, ext: Some(ExtValue::new(parameter, literal)) };
    }
    Value { node, local: Some(gn.uid(hint, Ty::Unknown)), ext: None }
}

/// What `bindArgument` leaves as it is: an identifier, a literal, `true` or `false`.
fn is_simple(node: P<Node>) -> bool {
    node.kind() == Kind::Identifier
        || tsrs_ast::is_literal_expression(node)
        || matches!(node.kind(), Kind::TrueKeyword | Kind::FalseKeyword)
}

/// `isParameterReference`: an identifier that names a parameter.
fn is_parameter_reference(cx: &mut Cx<'_>, node: P<Node>) -> bool {
    let Some(symbol) = cx.checker().get_symbol_at_location_exported(node) else { return false };
    symbol.value_declaration().is_some_and(|d| d.kind() == Kind::Parameter)
}

/// `memberName`: the member a call goes through, as the source names it, for a width check's message; none under
/// obfuscation.
fn member_name(gn: &Gen<'_, '_, '_>, target: P<Node>) -> Option<String> {
    if gn.shared.config.obfuscation == Some(true) {
        return None;
    }
    match target.kind() {
        Kind::PropertyAccessExpression => target.name().map(|n| n.text().to_owned()),
        Kind::ElementAccessExpression => {
            let argument = target.as_element_access_expression().argument_expression;
            matches!(argument.kind(), Kind::StringLiteral | Kind::NoSubstitutionTemplateLiteral)
                .then(|| argument.text().to_owned())
        }
        _ => None,
    }
}

/// `liveLocals(budget, node)`: the locals live where code put ahead of the call's statement lands.
fn live_locals(gn: &Gen<'_, '_, '_>, node: P<Node>) -> u32 {
    let around = scope::locals_around(node, &gn.g.net_budget);
    match scope::enclosing_scope(node) {
        Scope::File => around + gn.g.budget_used,
        Scope::Function(_) => around,
    }
}

/// `emitWithStatements`' decision, with the scope's claim.
fn place(gn: &mut Gen<'_, '_, '_>, node: P<Node>, call: &mut BuiltCall, hoisted: &[P<Node>]) {
    let statement = scope::enclosing_statement(node);
    let locals = call.declared().len() as u32 + declared_locals(&call.encoding);
    let has_statements = !call.declared().is_empty() || !call.encoding.is_empty();
    call.placement = if !has_statements {
        Placement::Bare
    } else if in_optional_chain(node, &call.shape) {
        Placement::Chain
    } else if !scope::hoists_cleanly(node, hoisted) {
        Placement::Function
    } else if locals > 0 && !claims(gn, node, statement, locals) {
        match statement.map(|s| s.kind()) {
            Some(Kind::ExpressionStatement | Kind::ReturnStatement) => Placement::Scoped,
            _ => Placement::Function,
        }
    } else {
        Placement::Ahead
    };
}

/// `liveLocals + locals <= COMPACT_LOCALS && budget.claim(scope, locals, statement)`.
fn claims(gn: &mut Gen<'_, '_, '_>, node: P<Node>, statement: Option<P<Node>>, locals: u32) -> bool {
    let Some(statement) = statement else { return false };
    if live_locals(gn, node) + locals > COMPACT_LOCALS {
        return false;
    }
    match scope::enclosing_scope(node) {
        Scope::File => gn.g.claim_file_locals(locals, statement),
        Scope::Function(function) => gn.g.net_budget.claim_function(function, locals, statement),
    }
}

/// `declaredLocals` of generated statements.
fn declared_locals(statements: &[S]) -> u32 {
    statements
        .iter()
        .map(|s| match s {
            S::Const(ir::Bind::One(_), _) | S::Let(..) => 1,
            S::Const(ir::Bind::Tuple(ids), _) => ids.len() as u32,
            _ => 0,
        })
        .sum()
}

/// The call is reached through `?.` (`optionalOperands` is not empty): the target's chain or the call's own.
fn in_optional_chain(node: P<Node>, shape: &CallShape) -> bool {
    let (target, own) = match shape {
        CallShape::Send { target, target_is_callee: true, .. } => (*target, node.question_dot_token().is_some()),
        CallShape::Send { target, .. } | CallShape::Callback { target, .. } => {
            let callee = node.expression().expect("a call has a callee");
            (*target, callee.question_dot_token().is_some())
        }
    };
    if own {
        return true;
    }
    let mut current = target;
    loop {
        match current.kind() {
            Kind::PropertyAccessExpression | Kind::ElementAccessExpression | Kind::CallExpression => {
                if current.question_dot_token().is_some() {
                    return true;
                }
            }
            Kind::NonNullExpression => {}
            _ => return false,
        }
        current = current.expression().expect("a link has an expression");
    }
}

// --- the hook ------------------------------------------------------------------------------------------------------

/// The call hook for calls on networking members ([`super::TAG_RECEIVER`], [`super::TAG_CALLEE`]): packs a call to a
/// packed member, declines anything else before compiling anything.
pub(crate) fn call_hook(fw: &FwInstance, cx: &mut Cx<'_>, call: &CallSite) -> PResult<Claim<LuauExpr>> {
    // A chain's outermost call, offered before the chain is compiled: declined, so the compiler binds the operands
    // ahead of each `?.` once and offers the call again inside the chain's nil check (`Placement::Chain`).
    if call.chain_outer || call.node.kind() != Kind::CallExpression || is_user_macro_call(fw, cx, call.node) {
        return Ok(Claim::Decline);
    }
    let node = call.node;
    let mut key_of = |cx: &mut Cx<'_>, ty, result| crate::serializer::packing_key(fw, cx, node, ty, result);
    let Some(shape) = super::analysis::analyze(fw, cx, node, &mut key_of)? else { return Ok(Claim::Decline) };

    let built = crate::serializer::call_site(fw, cx, node, &shape)?;

    let mut statements: Vec<Stmt> = Vec::new();
    let value = match &built.shape {
        CallShape::Send { target, target_is_callee, method, .. } => {
            let receiver = target_value(fw, cx, call, &built, *target, *target_is_callee, &mut statements)?;
            let mut arguments = Vec::with_capacity(built.leading.len() + 2);
            for value in &built.leading {
                arguments.push(bind_value(fw, cx, value, &mut statements)?);
            }
            match &built.spread {
                Some((table, nodes)) => {
                    let (array, prereqs) = cx.capture(|cx| spread_array(cx, nodes))?;
                    statements.extend(prereqs);
                    statements.push(create::local(create::any_id(local_name(fw, cx, *table)), Some(array)));
                }
                None => {
                    for value in &built.packed {
                        bind_value(fw, cx, value, &mut statements)?;
                    }
                }
            }
            let (encoding, payload, blobs) = lower_encoding(fw, cx, &built);
            statements.extend(encoding);
            arguments.extend(payload);
            arguments.extend(blobs);
            create::method_call(receiver, *method, arguments)
        }
        CallShape::Callback { target, .. } => {
            let receiver = target_value(fw, cx, call, &built, *target, false, &mut statements)?;
            let (callback_node, callback) = built.callback.expect("a callback site has its callback");
            let (value, prereqs) = cx.capture(|cx| cx.compile_expr(callback_node))?;
            statements.extend(prereqs);
            let callback_name = local_name(fw, cx, callback);
            statements.push(create::local(create::any_id(callback_name.clone()), Some(value)));
            let pack = built.pack.as_ref().expect("a callback site has its packer");
            let pack = crate::serializer::with_lowerer(fw, cx, |lowerer| {
                let mut prereqs = Vec::new();
                lowerer.expr(pack, &mut prereqs)
            });
            create::method_call(receiver, "_setCallback", vec![create::id(callback_name), pack])
        }
    };
    #[cfg(feature = "wire-harvest")]
    harvest(fw, cx, node, &built);

    Ok(Claim::Take(match built.placement {
        Placement::Bare | Placement::Chain => {
            statements.into_iter().for_each(|s| cx.prereq(s));
            value
        }
        Placement::Ahead => {
            cx.insert_before_statement(statements);
            value
        }
        Placement::Scoped if !call.value_used && is_whole_statement(node) => {
            // The statement goes in a block of its own with its packing, which ends their locals with it.
            statements.push(create::call_statement(value));
            cx.prereq(create::do_stmt(statements));
            create::none()
        }
        Placement::Scoped | Placement::Function => {
            statements.push(create::return_stmt(value));
            create::call(create::paren(create::function(Vec::new(), false, statements)), Vec::new())
        }
    }))
}

/// The call is its expression statement's whole expression.
fn is_whole_statement(node: P<Node>) -> bool {
    let mut current = node;
    while let Some(parent) = current.parent() {
        match parent.kind() {
            Kind::ParenthesizedExpression => current = parent,
            Kind::ExpressionStatement => return true,
            _ => return false,
        }
    }
    false
}

/// The target, compiled ahead of the arguments as the call would evaluate it, and bound to its local when the TS
/// transformer bound it.
fn target_value(
    fw: &FwInstance,
    cx: &mut Cx<'_>,
    call: &CallSite,
    built: &BuiltCall,
    target: P<Node>,
    target_is_callee: bool,
    statements: &mut Vec<Stmt>,
) -> PResult<Expr> {
    let (value, prereqs) = cx.capture(|cx| {
        if !target_is_callee {
            return cx.compile_receiver(call);
        }
        if is_chain_start(call.node) {
            return cx.compile_expr(target);
        }
        // The callee's object is compiled already: read the member off it.
        let receiver = cx.compile_receiver(call)?;
        match target.name().filter(|_| target.kind() == Kind::PropertyAccessExpression) {
            Some(name) => Ok(create::property(receiver, obfuscated_key(fw, cx, target, name.text()))),
            None => Err(cx.error(target, "rbxts-rs's Flamework cannot pack a call through this member access yet.")),
        }
    })?;
    statements.extend(prereqs);
    Ok(match built.target {
        Some(local) => {
            let name = local_name(fw, cx, local);
            statements.push(create::local(create::any_id(name.clone()), Some(value)));
            create::id(name)
        }
        None => value,
    })
}

/// A member key as the key-obfuscation hook gives it (networking's handlers carry `_flamework_key_obfuscation`),
/// through an optional chain too ([`crate::components::obfuscation`]: the receiver's non-nullable type).
fn obfuscated_key(fw: &FwInstance, cx: &mut Cx<'_>, access: P<Node>, name: &str) -> String {
    let Some(object) = access.expression() else { return name.to_owned() };
    let ty = cx.checker().get_type_at_location(object);
    let ty = cx.checker().get_non_nullable_type(ty);
    let context = cx
        .checker()
        .get_type_of_property_of_type_exported(ty, crate::components::KEY_OBFUSCATION_MARKER)
        .and_then(crate::macros::string_literal);
    match context {
        Some(context) => fw.shared.obfuscate_text(name, Some(context)),
        None => name.to_owned(),
    }
}

/// A chain's first call reached through plain accesses only, which the compiler offers before compiling anything
/// of it (so its callee may be compiled).
fn is_chain_start(call: P<Node>) -> bool {
    if call.question_dot_token().is_some() {
        return false;
    }
    let mut current = call.expression();
    while let Some(node) = current {
        match node.kind() {
            Kind::PropertyAccessExpression | Kind::ElementAccessExpression => {
                if node.question_dot_token().is_some() {
                    return false;
                }
                current = node.expression();
            }
            Kind::CallExpression | Kind::ParenthesizedExpression | Kind::NonNullExpression => return false,
            _ => return true,
        }
    }
    true
}

/// An argument compiled in the TS statements' order: bound to its local, or kept for the packing to read.
fn bind_value(fw: &FwInstance, cx: &mut Cx<'_>, value: &Value, statements: &mut Vec<Stmt>) -> PResult<Expr> {
    let (compiled, prereqs) = cx.capture(|cx| cx.compile_expr(value.node))?;
    statements.extend(prereqs);
    Ok(match (value.local, &value.ext) {
        (Some(local), _) => {
            let name = local_name(fw, cx, local);
            statements.push(create::local(create::any_id(name.clone()), Some(compiled)));
            create::id(name)
        }
        (None, Some(ext)) => {
            ext.set(compiled.clone());
            compiled
        }
        (None, None) => compiled,
    })
}

/// Lowers the packing: its statements, payload and blob list.
fn lower_encoding(fw: &FwInstance, cx: &mut Cx<'_>, built: &BuiltCall) -> (Vec<Stmt>, Option<Expr>, Option<Expr>) {
    crate::serializer::with_lowerer(fw, cx, |lowerer| {
        let statements = lowerer.statements(&built.encoding);
        let mut prereqs = Vec::new();
        let payload = built.payload.map(|id| lowerer.expr(&ir::id(id), &mut prereqs));
        let blobs = built.blobs.map(|id| lowerer.expr(&ir::id(id), &mut prereqs));
        (statements, payload, blobs)
    })
}

/// `wire-harvest` ([`crate::serializer::wire`]): the site's packing as `pack(args)`, which returns `{ payload, blobs }`
/// (a callback's packer as it is), its arguments read from `args` where the build reads them; `fixed` boxes the
/// literal (or `undefined`) arguments the packing was built for.
#[cfg(feature = "wire-harvest")]
fn harvest(fw: &FwInstance, cx: &mut Cx<'_>, node: P<Node>, built: &BuiltCall) {
    use crate::serializer::wire;
    if !wire::on() {
        return;
    }
    let args = || create::id("__fwargs");
    let at = |i: usize| create::computed_index(args(), create::number((i + 1) as f64));
    let (ty, result, pack, fixed) = match &built.shape {
        CallShape::Send { list, .. } => {
            let Some(payload) = built.payload else { return };
            let (mut fixed, mut saved) = (Vec::new(), Vec::new());
            for (i, value) in built.packed.iter().enumerate() {
                match (value.local, &value.ext) {
                    (Some(_), _) => {}
                    (None, Some(ext)) => {
                        let compiled = ext.value.replace(at(i));
                        if ext.literal.is_some() {
                            fixed.push((create::number((i + 1) as f64), create::array(vec![compiled.clone()])));
                        }
                        saved.push((ext, compiled));
                    }
                    (None, None) => fixed.push((create::number((i + 1) as f64), create::array(Vec::new()))),
                }
            }
            let pack = wire::relower(fw, |lowerer| {
                let mut body = Vec::new();
                for (i, value) in built.packed.iter().enumerate() {
                    if let Some(local) = value.local {
                        body.push(create::local(create::any_id(lowerer.name(local)), Some(at(i))));
                    }
                }
                if let Some((table, _)) = &built.spread {
                    body.push(create::local(create::any_id(lowerer.name(*table)), Some(args())));
                }
                body.extend(lowerer.statements(&built.encoding));
                let packed = std::iter::once(payload).chain(built.blobs).map(|id| create::id(lowerer.name(id))).collect();
                body.push(create::return_stmt(create::array(packed)));
                create::function(vec![create::any_id("__fwargs")], false, body)
            });
            for (ext, compiled) in saved {
                ext.set(compiled);
            }
            (*list, false, pack, fixed)
        }
        CallShape::Callback { function, .. } => {
            let pack = built.pack.as_ref().expect("a callback site has its packer");
            (*function, true, wire::relower(fw, |lowerer| lowerer.expr(pack, &mut Vec::new())), Vec::new())
        }
    };
    let Some(pack) = pack else { return };
    let mut extra = vec![("pack", pack), ("fixed", create::map(fixed))];
    if built.spread.is_none() && !result {
        extra.push(("arity", create::number(built.packed.len() as f64)));
    }
    wire::site(fw, cx, node, ty, result, extra);
}

/// What a spread argument spreads, as roblox-ts's `getAddIterableToArrayBuilder` tells it: the kinds a packed call
/// takes.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SpreadKind {
    Array,
    String,
    Set,
}

/// The kind of `node`'s type (`isDefinitelyType` with `isArrayType`, `isStringType`, `isSetType`, in that order);
/// `None` for the iterables a packed call does not take (maps, iterable functions, generators) and anything else.
fn spread_kind(cx: &mut Cx<'_>, node: P<Node>) -> Option<SpreadKind> {
    let ty = cx.checker().get_type_at_location(node);
    let checks: [(SpreadKind, TypeCheck); 3] =
        [(SpreadKind::Array, is_array_type), (SpreadKind::String, is_string_type), (SpreadKind::Set, is_set_type)];
    checks.into_iter().find(|(_, check)| is_definitely_type(cx, ty, *check)).map(|(kind, _)| kind)
}

/// A type predicate (`TypeCheck`).
type TypeCheck = fn(&mut Cx<'_>, P<Type>) -> bool;

/// `isDefinitelyType`: the type's constraint (or the type) passes `check` for every member of a union, some member of
/// an intersection, or through a base type.
fn is_definitely_type(cx: &mut Cx<'_>, ty: P<Type>, check: TypeCheck) -> bool {
    let ty = cx.checker().get_base_constraint_of_type_exported(ty).unwrap_or(ty);
    definitely(cx, ty, check)
}

fn definitely(cx: &mut Cx<'_>, ty: P<Type>, check: TypeCheck) -> bool {
    if ty.flags().intersects(TypeFlags::Union) {
        return ty.types().iter().all(|&member| definitely(cx, member, check));
    }
    if ty.flags().intersects(TypeFlags::Intersection) {
        return ty.types().iter().any(|&member| definitely(cx, member, check));
    }
    if ty.object_flags().intersects(ObjectFlags::ClassOrInterface) {
        let bases = cx.checker().get_base_types_exported(ty);
        if bases.iter().any(|&base| definitely(cx, base, check)) {
            return true;
        }
    }
    check(cx, ty)
}

/// `isArrayType`: a tuple, an array-like type, or one of the array types by their global symbols.
fn is_array_type(cx: &mut Cx<'_>, ty: P<Type>) -> bool {
    if ty.flags().intersects(TypeFlags::Any) {
        return false;
    }
    if ty.is_tuple_type() || cx.checker().is_array_like_type_exported(ty) {
        return true;
    }
    is_global(cx, ty, &["ReadonlyArray", "Array", "ReadVoxelsArray", "TemplateStringsArray"])
}

/// `isStringType`.
fn is_string_type(_: &mut Cx<'_>, ty: P<Type>) -> bool {
    ty.flags().intersects(TypeFlags::String | TypeFlags::StringLike | TypeFlags::StringLiteral)
}

/// `isSetType`.
fn is_set_type(cx: &mut Cx<'_>, ty: P<Type>) -> bool {
    is_global(cx, ty, &["Set", "ReadonlySet", "WeakSet"])
}

/// The type's symbol is one of these global types.
fn is_global(cx: &mut Cx<'_>, ty: P<Type>, names: &[&str]) -> bool {
    let Some(symbol) = ty.symbol() else { return false };
    names.iter().any(|name| crate::guards::types::global_type(cx, name) == Some(symbol))
}

/// roblox-ts's array literal with spreads (`transformArrayLiteralExpression`), for the packed arguments' table: a
/// spread of an array goes in with `table.move` (`addArray`), a string's characters and a set's values one by one in
/// a loop (`addString`, `addSet`), other elements one by one.
fn spread_array(cx: &mut Cx<'_>, elements: &[P<Node>]) -> PResult<Expr> {
    let mut inline: Option<Vec<Expr>> = Some(Vec::new());
    let mut pointer = create::nil();
    let length = rbxts_luau::AnyIdentifier::from(cx.temp_id("length"));
    let length_expr = Expr::from(length.clone());
    let mut since_update = 0u32;
    let disable = |cx: &mut Cx<'_>, members: Vec<Expr>| -> Expr {
        let array = cx.push_to_var(create::array(members), "array");
        cx.prereq(create::local(length.clone(), Some(create::unary(UnaryOperator::Len, array.clone()))));
        array
    };
    for (index, &element) in elements.iter().enumerate() {
        if element.kind() == Kind::SpreadElement {
            if let Some(members) = inline.take() {
                pointer = disable(cx, members);
                since_update = 0;
            }
            let expression = element.expression().expect("a spread has an expression");
            let kind = spread_kind(cx, expression).expect("a packed call's spreads are checked when it is built");
            let spread = cx.compile_expr(expression)?;
            if kind != SpreadKind::Array {
                // `createForLoopArrayBuilder`: the elements since the last update counted in, then one at a time.
                if since_update > 0 {
                    cx.prereq(create::assignment(
                        length_expr.clone(),
                        AssignmentOperator::Add,
                        create::number(f64::from(since_update)),
                    ));
                }
                let (hint, iterated) = match kind {
                    SpreadKind::String => (
                        "char",
                        create::call(
                            create::property(create::id("string"), "gmatch"),
                            vec![spread, create::property(create::id("utf8"), "charpattern")],
                        ),
                    ),
                    _ => ("v", spread),
                };
                let value = rbxts_luau::AnyIdentifier::from(cx.temp_id(hint));
                cx.prereq(create::for_in(
                    vec![value.clone()],
                    iterated,
                    vec![
                        create::assignment(length_expr.clone(), AssignmentOperator::Add, create::number(1.0)),
                        create::assignment(
                            create::computed_index(pointer.clone(), length_expr.clone()),
                            AssignmentOperator::Assign,
                            Expr::from(value),
                        ),
                    ],
                ));
                continue;
            }
            let input = if matches!(spread, Expr::Identifier(_) | Expr::TemporaryIdentifier(_)) {
                spread
            } else {
                cx.push_to_var(spread, "array")
            };
            let update = index < elements.len() - 1;
            let mut input_length = create::unary(UnaryOperator::Len, input.clone());
            if update {
                let hint = format!("{}Length", value_to_id_str(&input));
                input_length = cx.push_to_var(input_length, &hint);
            }
            let start = create::binary(length_expr.clone(), BinaryOperator::Add, create::number(f64::from(since_update + 1)));
            let table_move = create::property(create::id("table"), "move");
            cx.prereq(create::call_statement(create::call(
                table_move,
                vec![input, create::number(1.0), input_length.clone(), start, pointer.clone()],
            )));
            if update {
                cx.prereq(create::assignment(length_expr.clone(), AssignmentOperator::Add, input_length));
            }
        } else {
            let (expression, prereqs) = cx.capture(|cx| cx.compile_expr(element))?;
            if !prereqs.is_empty()
                && let Some(members) = inline.take()
            {
                pointer = disable(cx, members);
                since_update = 0;
            }
            match &mut inline {
                Some(members) => members.push(expression),
                None => {
                    prereqs.into_iter().for_each(|s| cx.prereq(s));
                    let index =
                        create::binary(length_expr.clone(), BinaryOperator::Add, create::number(f64::from(since_update + 1)));
                    cx.prereq(create::assignment(
                        create::computed_index(pointer.clone(), index),
                        AssignmentOperator::Assign,
                        expression,
                    ));
                }
            }
            since_update += 1;
        }
    }
    Ok(match inline {
        Some(members) => create::array(members),
        None => pointer,
    })
}

/// roblox-ts's `valueToIdStr` (`util/valueToIdStr.ts`): `foo` for `Foo`, `a.foo`, `Foo.new()`; else nothing
/// (temporaries included).
fn value_to_id_str(value: &Expr) -> String {
    fn expression_to_str(expression: &Expr) -> Option<&str> {
        match expression {
            Expr::Identifier(id) => Some(&id.name),
            Expr::PropertyAccess(access) => Some(&access.name),
            Expr::Call(call) => match &call.expression {
                Expr::PropertyAccess(access) if access.name == "new" => expression_to_str(&access.expression),
                _ => None,
            },
            _ => None,
        }
    }
    match expression_to_str(value) {
        Some(name) if rbxts_luau::is_valid_identifier(name) => {
            let mut chars = name.chars();
            let first = chars.next().map(|c| c.to_lowercase().collect::<String>()).unwrap_or_default();
            first + chars.as_str()
        }
        _ => String::new(),
    }
}
