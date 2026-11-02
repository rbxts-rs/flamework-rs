//! Wire-diff harvest (the `wire-harvest` cargo feature, off in every normal build; `tools/wirediff/README.md`).
//!
//! With `FLAMEWORK_RS_WIRE_DIR` set, every serializer, network decoder and packed call site of a file is also written
//! as a standalone Luau module, `<dir>/<file id>/<ordinal>.luau` (ordinals in source order): the file's hoisted
//! statements, then `return { kind, layout, key, ... }`. What the build emits is taken as it was lowered; what a
//! module needs besides (a call site's decoder, a decoder's encoder, every layout text) comes from a second generator
//! of the file's own, the companion, which never names through the compiler, warns nothing and builds
//! `__fwguard(<key>)` in place of a guard, so the build's output is the same with the feature on.
//!
//! **What B2 (direct Luau emission) must rewrite here.** The companion's lowering ([`Namer`] through
//! [`super::lower::LowerCx`]), [`relower`] and `networking::calls::harvest` work on the IR (`ir.rs`, `lower.rs`), which
//! B2 deletes. On the Luau builders they become:
//! - a site's packing: the build's own emitted statements, its argument placeholders bound to `__fwargs[i]` (instead
//!   of lowering the encoding again with `ExtValue`s swapped);
//! - the companion's names: a namer through the new shared `Names` that never asks the compiler (as [`Namer`] does);
//! - [`guard`]: `__fwguard("<key>")` built as an `rbxts_luau` call;
//! - the module contract unchanged (`kind`, `layout`, `key`, `serializer` / `deserialize` / `encode` / `pack`,
//!   `fixed`, `arity`; ordinals in source order), so that harvests of builds before and after B2 still pair up.

use rbxts_luau::{LateStrings, TempIds, TemporaryIdentifier, create, render_ast, render_expr};
use rbxts_plugin::ts::{Node, P, Type};
use rbxts_plugin::{Cx, LuauExpr, LuauStmt, text_literal};

use super::codegen::SiteValues;
use super::ir::{self, Ty};
use super::lower::{LowerCx, Lowerer};
use super::{Checks, EncodingSite, GResult, Gen, Generator, Shape};
use crate::plugin::FwInstance;

/// One file's harvest: the hoisted statements handed out so far, the modules' entries, the companion.
#[derive(Default)]
pub struct Harvest {
    top: Vec<LuauStmt>,
    companion: Option<Generator>,
    companion_top: Vec<LuauStmt>,
    temps: TempIds,
    named: u32,
    entries: Vec<(u32, String)>,
}

/// Names for harvest lowering: never asks the compiler, so nothing it names takes a name from the file.
struct Namer<'h>(&'h mut TempIds, &'h mut u32);

impl LowerCx for Namer<'_> {
    fn temp_id(&mut self, hint: &str) -> TemporaryIdentifier {
        self.0.temp_id(hint)
    }
    fn local_name(&mut self, hint: &str) -> String {
        *self.1 += 1;
        format!("{hint}_h{}", self.1)
    }
}

pub(crate) fn on() -> bool {
    std::env::var_os("FLAMEWORK_RS_WIRE_DIR").is_some()
}

/// Records statements a hook hoists, in the order they are placed.
pub(crate) fn hoisted(fw: &FwInstance, top: &[LuauStmt]) {
    if on() {
        fw.serializer.borrow_mut().harvest.top.extend_from_slice(top);
    }
}

/// Runs `build` on the file's companion at `node`, then lowers its hoisted statements and what `lower` makes of the
/// result. `None` when the build failed or there is nothing to test.
fn companion<T>(
    fw: &FwInstance,
    cx: &mut Cx<'_>,
    node: P<Node>,
    build: impl FnOnce(&mut Gen<'_, '_, '_>) -> GResult<Option<T>>,
    lower: impl FnOnce(&mut Lowerer<'_>, T) -> Vec<(&'static str, LuauExpr)>,
) -> Option<Vec<(&'static str, LuauExpr)>> {
    let mut state = fw.serializer.borrow_mut();
    let state = &mut *state;
    let h = &mut state.harvest;
    let g = h.companion.get_or_insert_with(|| {
        let mut g = Generator::new(cx.file().source, Checks::from_config(&fw.shared));
        g.companion = true;
        g
    });
    let mut gn = Gen { g, cx, shared: &fw.shared };
    gn.use_node(node);
    let built = build(&mut gn);
    let hoisted = gn.take_hoisted();
    let built = if gn.g.sticky.take().is_some() { None } else { built.ok().flatten() };
    let mut lowerer = Lowerer { ids: &g.ids, names: &mut g.names, cx: &mut Namer(&mut h.temps, &mut h.named) };
    h.companion_top.extend(lowerer.statements(&hoisted));
    Some(lower(&mut lowerer, built?))
}

/// The layout text and wire key of an argument list (a function type's result with `result`).
fn list_texts(gn: &mut Gen<'_, '_, '_>, ty: P<Type>, node: P<Node>, result: bool) -> GResult<(Shape, String, String)> {
    let list = if result { gn.result_of(ty, node)? } else { gn.list_of(ty, None)? };
    let layout = gn.layout_text(&list)?;
    let key = gn.wire_key(&list, &mut Vec::new(), false)?.0;
    Ok((list, layout, key))
}

fn texts(layout: String, key: String) -> Vec<(&'static str, LuauExpr)> {
    vec![("layout", text_literal(&layout)), ("key", text_literal(&key))]
}

/// `Flamework.createSerializer<T>()`'s object.
pub(crate) fn serializer(fw: &FwInstance, cx: &mut Cx<'_>, node: P<Node>, ty: P<Type>, object: &LuauExpr) {
    if !on() {
        return;
    }
    let written = super::written_type_argument(node);
    let fields = companion(fw, cx, node, |gn| {
        let value = gn.unwrap_promise(ty);
        let shape = gn.spell(written, value)?;
        Ok(Some((gn.layout_text(&shape)?, gn.wire_key(&shape, &mut Vec::new(), false)?.0)))
    }, |_, (layout, key)| texts(layout, key));
    let Some(mut fields) = fields else { return };
    fields.push(("serializer", object.clone()));
    entry(fw, cx, node, "serializer", fields);
}

/// A network decoder, with the companion's encoder of its list (`function(args) ... return { payload, blobs } end`).
pub(crate) fn decoder(fw: &FwInstance, cx: &mut Cx<'_>, node: P<Node>, ty: P<Type>, result: bool, decoder: &LuauExpr) {
    if !on() {
        return;
    }
    let fields = companion(fw, cx, node, |gn| {
        let (list, layout, key) = list_texts(gn, ty, node, result)?;
        let args = gn.parameter("args", Ty::Array);
        let site = EncodingSite { result, ..EncodingSite::default() };
        let encoding = gn.encode_elements(list, SiteValues::Table(ir::cast(ir::id(args), Ty::Array)), site)?;
        Ok(encoding.payload.is_some().then_some((layout, key, args, encoding)))
    }, |lowerer, (layout, key, args, encoding)| {
        let mut body = lowerer.statements(&encoding.statements);
        let ids = encoding.payload.into_iter().chain(encoding.blobs);
        let packed = ids.map(|id| create::id(lowerer.name(id))).collect();
        body.push(create::return_stmt(create::array(packed)));
        let mut fields = texts(layout, key);
        fields.push(("encode", create::function(vec![create::any_id(lowerer.name(args))], false, body)));
        fields
    });
    let Some(mut fields) = fields else { return };
    fields.push(("deserialize", decoder.clone()));
    entry(fw, cx, node, "decoder", fields);
}

/// A call site's packing (`pack`, from [`crate::networking::calls`]), with the companion's decoder of its list.
pub(crate) fn site(fw: &FwInstance, cx: &mut Cx<'_>, node: P<Node>, ty: P<Type>, result: bool, mut extra: Vec<(&'static str, LuauExpr)>) {
    let fields = companion(fw, cx, node, |gn| {
        let (list, layout, key) = list_texts(gn, ty, node, result)?;
        Ok(gn.build_decoder(list)?.map(|decoder| (layout, key, decoder)))
    }, |lowerer, (layout, key, decoder)| {
        let mut fields = texts(layout, key);
        fields.push(("deserialize", lowerer.expr(&decoder, &mut Vec::new())));
        fields
    });
    let Some(mut fields) = fields else { return };
    fields.append(&mut extra);
    entry(fw, cx, node, if result { "callback" } else { "site" }, fields);
}

/// Lowers IR the build lowered already, with its names (copied: nothing named here reaches the build).
pub(crate) fn relower<T>(fw: &FwInstance, f: impl FnOnce(&mut Lowerer<'_>) -> T) -> Option<T> {
    if !on() {
        return None;
    }
    let mut state = fw.serializer.borrow_mut();
    let state = &mut *state;
    let generator = state.generator.as_ref()?;
    let mut names = generator.names.clone();
    let h = &mut state.harvest;
    Some(f(&mut Lowerer { ids: &generator.ids, names: &mut names, cx: &mut Namer(&mut h.temps, &mut h.named) }))
}

fn entry(fw: &FwInstance, cx: &mut Cx<'_>, node: P<Node>, kind: &str, fields: Vec<(&'static str, LuauExpr)>) {
    let mut text = format!("return {{\n\tkind = \"{kind}\",\n");
    for (name, value) in fields {
        text += &format!("\t{name} = {},\n", render_expr(&value, &LateStrings::default()));
    }
    text += "}\n";
    let at = cx.utf16_pos(node).offset;
    fw.serializer.borrow_mut().harvest.entries.push((at, text));
}

/// Writes the file's modules (from `end_file`).
pub(crate) fn end_file(fw: &FwInstance, cx: &mut Cx<'_>) {
    let Some(dir) = std::env::var_os("FLAMEWORK_RS_WIRE_DIR") else { return };
    let mut h = std::mem::take(&mut fw.serializer.borrow_mut().harvest);
    if h.entries.is_empty() {
        return;
    }
    let id = fw.shared.file_id(&cx.file().path);
    let prelude = render_ast(&h.top) + &render_ast(&h.companion_top);
    let dir = std::path::Path::new(&dir).join(&id);
    let _ = std::fs::create_dir_all(&dir);
    h.entries.sort_by_key(|(at, _)| *at);
    for (ordinal, (_, text)) in h.entries.iter().enumerate() {
        let module = format!("-- wire-harvest: {id} #{ordinal}\n{prelude}{text}");
        let _ = std::fs::write(dir.join(format!("{ordinal}.luau")), module);
    }
}

/// What a companion builds for a guard (`guardFor`): `__fwguard("<wire key>")`, a test of the key's values that the
/// harness provides, so that no guard (and no `t` import) is made for the file.
pub(crate) fn guard(gn: &mut Gen<'_, '_, '_>, ty: P<Type>) -> GResult<ir::E> {
    let key = gn.wire_key(&Shape::Type(ty), &mut Vec::new(), false)?.0;
    Ok(ir::call(ir::global("__fwguard"), vec![ir::string(&key)]))
}
