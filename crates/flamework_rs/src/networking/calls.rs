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
//! **Where it goes** (`Route`, `place`; `emitWithStatements`): a send that is a whole statement goes in a `do`
//! block with its statement (`do <packing>; target:_fire(...) end`), so its locals end with it; a returned send ahead
//! of its `return`, which ends the block anyway. A send elsewhere in a statement goes ahead of the statement when
//! that runs its code exactly when the call runs (`hoistsCleanly`) and the locals it leaves live there stay within
//! [`scope::SEND_LEAK`] per send and its scope's cap ([`scope::Scope::leak_cap`]), counted against the compiler's
//! live locals ([`scope::AHEAD_LOCALS`]); in a function called where the call is otherwise. The packing is fitted to
//! Luau's limits among the locals live where it lands: the compiler's count (`Cx::live_before_statement`, asked once
//! the arguments are compiled, so that whatever their hooks put ahead of the statement is in it), what the compiler
//! cannot see yet ([`scope::unseen_locals`], [`scope::MAIN_CHUNK_LATE`]), and the site's own statements ahead of
//! the packing; only those in a function of its own, where a packing with no room in either form where the call is
//! goes instead. A call reached through `?.` (`a?.b.fire(x)`) is
//! lowered by the compiler inside the chain's nil check, its statements there: the chain's offer of its outermost
//! call (`Interest::calls_chain_outer`) is declined, so the compiler evaluates each operand ahead of a `?.` once,
//! into its own local, before the arguments, and the send goes through those values, as JavaScript does. (The TS
//! transformer's `guardOptionalChain` checked the operands, packed, then read the chain again for the send: an
//! argument that clears or replaces an operand changed or broke the send; README, Differences.)
//!
//! **Names.** The site's locals are placeholders of the file's serializer names, named when the hook hands its
//! statements out ([`crate::serializer::name_output`]). An argument read where it is (an identifier or a literal) is a
//! slot in the packing ([`Gen::slot`]), replaced by the argument's Luau once it is compiled.

use std::collections::HashMap;

use rbxts_luau::{AssignmentOperator, BinaryOperator, Expr, Stmt, UnaryOperator, create};
use rbxts_plugin::ts::{Kind, Node, P, Type};
use tsrs_checker::{ObjectFlags, TypeFlags};
use rbxts_plugin::{CallSite, Claim, Cx, LuauExpr, PResult};

pub(crate) use super::analysis::CallShape;
use super::scope::{self, Scope};
use crate::luau_names::Names;
use crate::plugin::FwInstance;
use crate::serializer::codegen::SiteValues;
use crate::serializer::lit::Lit;
use crate::serializer::luau;
use crate::serializer::{EncodingSite, GResult, Gen, Id, Shape, name_output};

/// Where a call site's statements go (`emitWithStatements`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Placement {
    /// No statements: the call alone.
    Bare,
    /// Ahead of the statement around the call (`prereqList`).
    Ahead,
    /// In a `do` block with its statement, the call's whole expression statement (`scopedStatements`).
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
    pub read: Read,
}

/// How the packing reads a [`Value`].
#[derive(Clone, Copy, Debug)]
pub(crate) enum Read {
    /// `undefined`: `nil`.
    Nil,
    /// Bound to a local (`bindArgument`).
    Local(Id),
    /// Read where it is, an identifier or a `literal`: a slot the hook fills with the argument's Luau.
    #[cfg_attr(not(feature = "wire-harvest"), allow(dead_code))]
    Slot { id: Id, literal: bool },
}

impl Value {
    /// The value as the packing reads it.
    fn as_e(&self) -> Expr {
        match self.read {
            Read::Nil => create::nil(),
            Read::Local(id) | Read::Slot { id, .. } => id.e(),
        }
    }
}

/// A networking call site's code, built at its hook ([`crate::serializer::call_site`]).
#[derive(Debug)]
pub struct BuiltCall {
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
    /// A send's packed list, the values the packing reads and the member's name, for [`pack`].
    list: Option<(Shape, SiteValues, Option<String>)>,
    /// The packing, built by [`pack`].
    encoding: Vec<Stmt>,
    payload: Option<Id>,
    blobs: Option<Id>,
    /// `setCallback`'s packer: `function(value) <pack>; return { payload, blobs? } end`.
    pack: Option<Expr>,
}

/// `state.isUserMacro(symbol)` of the callee: such a call is the user macro's, never networking's.
fn is_user_macro_call(fw: &FwInstance, cx: &mut Cx<'_>, call: P<Node>) -> bool {
    let Some(callee) = call.expression() else { return false };
    let Some(symbol) = cx.checker().get_symbol_at_location_exported(callee) else { return false };
    let symbol = cx.checker().skip_alias(symbol);
    fw.is_user_macro(cx, symbol)
}

// --- building ------------------------------------------------------------------------------------------------------

/// Builds a call site's code with the file's generator, ahead of its arguments: its bindings, and a callback's packer.
/// A send's packing is built by [`pack`] once the arguments are compiled.
pub(crate) fn build(gn: &mut Gen<'_, '_, '_>, node: P<Node>, shape: &CallShape) -> GResult<BuiltCall> {
    match shape {
        CallShape::Send { target, list, rest_index, .. } => build_send(gn, node, *target, *list, *rest_index),
        CallShape::Callback { target, function } => build_callback(gn, node, *target, *function),
    }
}

fn build_send(
    gn: &mut Gen<'_, '_, '_>,
    node: P<Node>,
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
    let target_local = (!scope::is_pure(target)).then(|| gn.uid("target"));
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
        let table = gn.uid("args");
        spread = Some((table, packed_nodes.to_vec()));
        SiteValues::Table(table.e())
    } else {
        packed = packed_nodes.iter().map(|&a| value_of(gn, a, "arg")).collect();
        SiteValues::List(packed.iter().map(Value::as_e).collect())
    };

    let name = member_name(gn, target);
    let shape_list = gn.list_of(list, None)?;
    Ok(BuiltCall {
        target: target_local,
        leading,
        packed,
        spread,
        callback: None,
        list: Some((shape_list, values, name)),
        encoding: Vec::new(),
        payload: None,
        blobs: None,
        pack: None,
    })
}

/// A send's packing, fitted to Luau's limits among `outer` live locals (a callback's packer is built with it, a
/// function of its own); `false` when it fits in neither form there, its compact form kept.
pub(crate) fn pack(gn: &mut Gen<'_, '_, '_>, call: &mut BuiltCall, outer: u32) -> GResult<bool> {
    let Some((list, values, name)) = call.list.take() else { return Ok(true) };
    gn.g.site_refused = false;
    let encoding = gn.encode_elements(list, values, EncodingSite { name, result: false, outer })?;
    call.encoding = encoding.statements;
    call.payload = encoding.payload;
    call.blobs = encoding.blobs;
    Ok(!std::mem::take(&mut gn.g.site_refused))
}

fn build_callback(
    gn: &mut Gen<'_, '_, '_>,
    node: P<Node>,
    target: P<Node>,
    function: P<rbxts_plugin::ts::Type>,
) -> GResult<BuiltCall> {
    let callback = *node.arguments().first().expect("a callback argument");
    // The target is bound unless it is an identifier (`bindArgument` of the chained target).
    let target_local = (!is_simple(target)).then(|| gn.uid("target"));
    let callback_local = gn.uid("callback");
    let value = gn.uid("value");
    let name = member_name(gn, target);
    let list = gn.result_of(function, node)?;
    let encoding =
        gn.encode_elements(list, SiteValues::List(vec![value.e()]), EncodingSite { name, result: true, outer: 0 })?;
    // The runtime calls `pack` only with a resolved, successful value.
    let mut body = encoding.statements;
    body.push(create::return_stmt(match encoding.payload {
        Some(payload) => create::array(std::iter::once(payload).chain(encoding.blobs).map(Id::e).collect()),
        None => create::nil(),
    }));
    let pack = crate::serializer::codegen::function(&[value], body);

    Ok(BuiltCall {
        target: target_local,
        leading: Vec::new(),
        packed: Vec::new(),
        spread: None,
        callback: Some((callback, callback_local)),
        list: None,
        encoding: Vec::new(),
        payload: None,
        blobs: None,
        pack: Some(pack),
    })
}

/// `bindArgument`: an argument read where it is (an identifier or a literal, through a slot), else bound to a new
/// local (`hint`).
fn value_of(gn: &mut Gen<'_, '_, '_>, node: P<Node>, hint: &str) -> Value {
    if node.kind() == Kind::Identifier && node.text() == "undefined" {
        return Value { node, read: Read::Nil };
    }
    if is_simple(node) {
        let literal = match node.kind() {
            Kind::Identifier => None,
            Kind::TrueKeyword => Some(Lit::Bool(true)),
            Kind::FalseKeyword => Some(Lit::Bool(false)),
            Kind::NumericLiteral => Some(Lit::Num(crate::js::string_to_number(node.text()))),
            _ => Some(Lit::Str(node.text().to_owned())),
        };
        let literal_read = literal.is_some();
        return Value { node, read: Read::Slot { id: gn.slot(literal), literal: literal_read } };
    }
    Value { node, read: Read::Local(gn.uid(hint)) }
}

/// What `bindArgument` leaves as it is: an identifier, a literal, `true` or `false`.
fn is_simple(node: P<Node>) -> bool {
    node.kind() == Kind::Identifier
        || tsrs_ast::is_literal_expression(node)
        || matches!(node.kind(), Kind::TrueKeyword | Kind::FalseKeyword)
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

/// Where a site's statements go, as far as the source tells before they are built ([`place`] decides).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Route {
    /// Reached through `?.`: [`Placement::Chain`].
    Chain,
    /// Not evaluated exactly when its statement starts (`hoistsCleanly`), where the compiler's count is short (a
    /// class expression's static block), or a packing with no room where the call is: [`Placement::Function`].
    Function,
    /// The call is its expression statement: [`Placement::Scoped`].
    Whole,
    /// In a `return` statement, which ends its block: [`Placement::Ahead`], which leaves nothing live after it.
    Returned,
    /// Elsewhere in a statement: [`Placement::Ahead`] within the caps, else [`Placement::Function`].
    Ahead,
}

/// The site's [`Route`] (`emitWithStatements`' tests on the source).
fn route(node: P<Node>, shape: &CallShape) -> Route {
    if in_optional_chain(node, shape) {
        return Route::Chain;
    }
    let target = match shape {
        CallShape::Send { target, .. } | CallShape::Callback { target, .. } => *target,
    };
    let mut hoisted = vec![target];
    hoisted.extend_from_slice(node.arguments());
    if !scope::hoists_cleanly(node, &hoisted) || scope::in_class_expression_static_block(node) {
        Route::Function
    } else if is_whole_statement(node) {
        Route::Whole
    } else if scope::enclosing_statement(node).is_some_and(|s| s.kind() == Kind::ReturnStatement) {
        Route::Returned
    } else {
        Route::Ahead
    }
}

/// The locals live where the site's statements land, before them; none in a function of its own: the compiler's count
/// (`Cx::live_before_statement`), plus what it cannot count yet ([`scope::unseen_locals`], and
/// [`scope::MAIN_CHUNK_LATE`] in the main chunk); in a chain, also the chain's own locals ahead of the call
/// ([`chain_locals`]). The probe's padding stands for each as a local and a register; the registers the loops around
/// hold besides their variables (2 to 4 a loop, against Luau's 255 registers to its 200 locals) are left to the margin.
fn live_locals(cx: &mut Cx<'_>, node: P<Node>, route: Route) -> u32 {
    if route == Route::Function {
        return 0;
    }
    let mut live = cx.live_before_statement().locals + scope::unseen_locals(node);
    if scope::enclosing_scope(node) == Scope::File {
        live += scope::MAIN_CHUNK_LATE;
    }
    if route == Route::Chain {
        live += chain_locals(node);
    }
    live
}

/// An upper bound of the locals an optional chain binds ahead of a call in it: one per link, and its result.
fn chain_locals(node: P<Node>) -> u32 {
    let mut count = 1;
    let mut current = node.expression();
    while let Some(link) = current {
        match link.kind() {
            Kind::PropertyAccessExpression
            | Kind::ElementAccessExpression
            | Kind::CallExpression
            | Kind::NonNullExpression
            | Kind::ParenthesizedExpression => {
                count += 1;
                current = link.expression();
            }
            _ => break,
        }
    }
    count
}

/// `emitWithStatements`' decision, once the site's statements are built; `live` is [`live_locals`]. A send put ahead
/// of its statement claims the locals it leaves there from its scope's cap.
fn place(fw: &FwInstance, node: P<Node>, route: Route, live: u32, statements: &[Stmt]) -> Placement {
    if statements.is_empty() {
        return Placement::Bare;
    }
    match route {
        Route::Chain => Placement::Chain,
        Route::Function => Placement::Function,
        Route::Whole => Placement::Scoped,
        Route::Returned => Placement::Ahead,
        Route::Ahead => {
            let leaked = declared_locals(statements);
            let fits = leaked <= scope::SEND_LEAK && live + leaked <= scope::AHEAD_LOCALS;
            if fits && claim_leak(fw, scope::enclosing_scope(node), leaked) {
                Placement::Ahead
            } else {
                Placement::Function
            }
        }
    }
}

/// Claims `count` locals left live ahead of a statement in `scope`, within [`Scope::leak_cap`].
fn claim_leak(fw: &FwInstance, scope: Scope, count: u32) -> bool {
    let mut state = fw.serializer.borrow_mut();
    let generator = state.generator.as_mut().expect("a generator");
    let used = generator.net_leaked.entry(scope).or_default();
    if *used + count > scope.leak_cap() {
        return false;
    }
    *used += count;
    true
}

/// The locals generated statements declare at their top level.
fn declared_locals(statements: &[Stmt]) -> u32 {
    statements
        .iter()
        .map(|s| match s {
            Stmt::VariableDeclaration(declaration) => declaration.left.len() as u32,
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

    let mut built = crate::serializer::call_site(fw, cx, node, &shape)?;
    let mut route = route(node, &shape);

    let mut statements: Vec<Stmt> = Vec::new();
    // each slot's Luau, by placeholder index
    let mut slots: HashMap<usize, Expr> = HashMap::new();
    let (mut value, live) = match &shape {
        CallShape::Send { target, target_is_callee, method, .. } => {
            let receiver = target_value(fw, cx, call, &built, *target, *target_is_callee, &mut statements)?;
            let mut arguments = Vec::with_capacity(built.leading.len() + 2);
            for value in &built.leading {
                arguments.push(bind_value(cx, value, &mut statements, &mut slots)?);
            }
            match &built.spread {
                Some((table, nodes)) => {
                    let (array, prereqs) = cx.capture(|cx| spread_array(cx, nodes))?;
                    statements.extend(prereqs);
                    statements.push(luau::local(&table.e(), Some(array)));
                }
                None => {
                    for value in &built.packed {
                        bind_value(cx, value, &mut statements, &mut slots)?;
                    }
                }
            }
            // The arguments are compiled: what their hooks put ahead of the statement is in the compiler's count.
            let live = live_locals(cx, node, route);
            let outer = live + declared_locals(&statements);
            if !crate::serializer::call_site_packing(fw, cx, node, &mut built, outer)? {
                // No room where the call is, even for the compact packing: away from the live locals.
                route = Route::Function;
            }
            let mut encoding = built.encoding.clone();
            Names::substitute(&mut encoding, &slots);
            statements.extend(encoding);
            arguments.extend(built.payload.map(Id::e));
            arguments.extend(built.blobs.map(Id::e));
            (create::method_call(receiver, *method, arguments), live)
        }
        CallShape::Callback { target, .. } => {
            let receiver = target_value(fw, cx, call, &built, *target, false, &mut statements)?;
            let (callback_node, callback) = built.callback.expect("a callback site has its callback");
            let (value, prereqs) = cx.capture(|cx| cx.compile_expr(callback_node))?;
            statements.extend(prereqs);
            statements.push(luau::local(&callback.e(), Some(value)));
            let pack = built.pack.clone().expect("a callback site has its packer");
            let live = if route == Route::Ahead { live_locals(cx, node, route) } else { 0 };
            (create::method_call(receiver, "_setCallback", vec![callback.e(), pack]), live)
        }
    };
    let placement = place(fw, node, route, live, &statements);
    name_output(fw, cx, &mut statements, Some(&mut value));
    #[cfg(feature = "wire-harvest")]
    harvest(fw, cx, node, &shape, &built, &slots);

    Ok(Claim::Take(match placement {
        Placement::Bare | Placement::Chain => {
            statements.into_iter().for_each(|s| cx.prereq(s));
            value
        }
        Placement::Ahead => {
            cx.insert_before_statement(statements);
            value
        }
        Placement::Scoped => {
            debug_assert!(!call.value_used, "a whole statement's call");
            // The statement goes in a block of its own with its packing, which ends their locals with it.
            statements.push(create::call_statement(value));
            cx.prereq(create::do_stmt(statements));
            create::none()
        }
        Placement::Function => {
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
            statements.push(luau::local(&local.e(), Some(value)));
            local.e()
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

/// An argument compiled in the TS statements' order: bound to its local, or kept for its slot in the packing (`slots`,
/// by placeholder index).
fn bind_value(
    cx: &mut Cx<'_>,
    value: &Value,
    statements: &mut Vec<Stmt>,
    slots: &mut HashMap<usize, Expr>,
) -> PResult<Expr> {
    let (compiled, prereqs) = cx.capture(|cx| cx.compile_expr(value.node))?;
    statements.extend(prereqs);
    Ok(match value.read {
        Read::Local(local) => {
            statements.push(luau::local(&local.e(), Some(compiled)));
            local.e()
        }
        Read::Slot { id, .. } => {
            slots.insert(id.0 as usize, compiled.clone());
            compiled
        }
        Read::Nil => compiled,
    })
}

/// `wire-harvest` ([`crate::serializer::wire`]): the site's packing as `pack(args)`, which returns `{ payload, blobs }`
/// (a callback's packer as it is), its arguments read from `args` where the build reads them, named as the build named
/// them; `fixed` boxes the literal (or `undefined`) arguments the packing was built for (`slots` holds their Luau).
#[cfg(feature = "wire-harvest")]
fn harvest(
    fw: &FwInstance,
    cx: &mut Cx<'_>,
    node: P<Node>,
    shape: &CallShape,
    built: &BuiltCall,
    slots: &HashMap<usize, Expr>,
) {
    use crate::serializer::wire;
    if !wire::on() {
        return;
    }
    let args = || create::id("__fwargs");
    let at = |i: usize| create::computed_index(args(), create::number((i + 1) as f64));
    let position = |i: usize| create::number((i + 1) as f64);
    let (ty, result, mut pack, fixed) = match shape {
        CallShape::Send { list, .. } => {
            let Some(payload) = built.payload else { return };
            let (mut fixed, mut read_args, mut body) = (Vec::new(), HashMap::new(), Vec::new());
            for (i, value) in built.packed.iter().enumerate() {
                match value.read {
                    Read::Local(local) => body.push(luau::local(&local.e(), Some(at(i)))),
                    Read::Slot { id, literal } => {
                        read_args.insert(id.0 as usize, at(i));
                        if literal {
                            fixed.push((position(i), create::array(vec![slots[&(id.0 as usize)].clone()])));
                        }
                    }
                    Read::Nil => fixed.push((position(i), create::array(Vec::new()))),
                }
            }
            if let Some((table, _)) = &built.spread {
                body.push(luau::local(&table.e(), Some(args())));
            }
            body.extend(built.encoding.iter().cloned());
            Names::substitute(&mut body, &read_args);
            let packed = std::iter::once(payload).chain(built.blobs).map(Id::e).collect();
            body.push(create::return_stmt(create::array(packed)));
            (*list, false, create::function(vec![create::any_id("__fwargs")], false, body), fixed)
        }
        CallShape::Callback { function, .. } => {
            (*function, true, built.pack.clone().expect("a callback site has its packer"), Vec::new())
        }
    };
    wire::name_copy(fw, &mut pack);
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
