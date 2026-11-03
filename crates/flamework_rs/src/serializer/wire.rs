//! Wire-diff harvest (the `wire-harvest` cargo feature, off in every normal build; `tools/wirediff/README.md`).
//!
//! With `FLAMEWORK_RS_WIRE_DIR` set, every serializer, network decoder and packed call site of a file is also written
//! as a standalone Luau module, `<dir>/<file id>/<ordinal>.luau` (ordinals in source order): the file's hoisted
//! statements, then `return { kind, layout, key, ... }`. What the build emits is taken as it was named; what a
//! module needs besides (a call site's decoder, a decoder's encoder, every layout text) comes from a second generator
//! of the file's own, the companion, which never names through the compiler, warns nothing and builds
//! `__fwguard(<key>)` in place of a guard, so the build's output is the same with the feature on.
//!
//! Names in a module never come from the compiler: the companion's are `<base>_h<n>`, and a call site's packing keeps
//! the build's names ([`name_copy`]). The module contract (`kind`, `layout`, `key`, `serializer` / `deserialize` /
//! `encode` / `pack`, `fixed`, `arity`; ordinals in source order) is the same as before direct Luau emission, so
//! harvests of builds before and after it pair up.
//!
//! The build's slots (guards, literal and enum tables), which the compiler may spill into one table, are globals of a
//! module (\`guard = …\`, each module runs in an environment of its own), so that no module passes Luau's 200 locals.

use rbxts_luau::{Expr, LateStrings, Stmt, create, render_ast, render_expr};
use rbxts_plugin::ts::{Node, P, Type};
use rbxts_plugin::{Cx, LuauExpr, LuauStmt, text_literal};

use super::codegen::{SiteValues, function};
use super::{Checks, EncodingSite, GResult, Gen, Generator, Id, Shape};
use crate::luau_names::Names;
use crate::plugin::FwInstance;

/// One file's harvest: the hoisted statements handed out so far, the modules' entries, the companion.
#[derive(Default)]
pub struct Harvest {
    top: Vec<LuauStmt>,
    companion: Option<Generator>,
    companion_top: Vec<LuauStmt>,
    named: u32,
    entries: Vec<(u32, String)>,
}

/// Names placeholders with `names`, giving each new one `<base>_h<n>`: never asks the compiler, so nothing named
/// here takes a name from the file.
fn name_harvest(names: &mut Names, named: &mut u32, statements: &mut [Stmt], expr: Option<&mut Expr>) {
    names.assign_with(statements, expr, |base| {
        *named += 1;
        format!("{base}_h{named}")
    });
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

/// Runs `build` on the file's companion at `node`, then names its hoisted statements and the fields `fields` makes of
/// the result. `None` when the build failed or there is nothing to test.
fn companion<T>(
    fw: &FwInstance,
    cx: &mut Cx<'_>,
    node: P<Node>,
    build: impl FnOnce(&mut Gen<'_, '_, '_>) -> GResult<Option<T>>,
    fields: impl FnOnce(T) -> Vec<(&'static str, LuauExpr)>,
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
    let mut hoisted = gn.take_hoisted();
    let built = if gn.g.sticky.take().is_some() { None } else { built.ok().flatten() };
    name_harvest(&mut g.names, &mut h.named, &mut hoisted, None);
    h.companion_top.extend(hoisted);
    let mut fields = fields(built?);
    for (_, value) in &mut fields {
        name_harvest(&mut g.names, &mut h.named, &mut [], Some(value));
    }
    Some(fields)
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
    }, |(layout, key)| texts(layout, key));
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
        let args = gn.uid("args");
        let site = EncodingSite { result, ..EncodingSite::default() };
        let encoding = gn.encode_elements(list, SiteValues::Table(args.e()), site)?;
        Ok(encoding.payload.is_some().then_some((layout, key, args, encoding)))
    }, |(layout, key, args, encoding)| {
        let mut body = encoding.statements;
        let packed = encoding.payload.into_iter().chain(encoding.blobs).map(Id::e).collect();
        body.push(create::return_stmt(create::array(packed)));
        let mut fields = texts(layout, key);
        fields.push(("encode", function(&[args], body)));
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
    }, |(layout, key, decoder)| {
        let mut fields = texts(layout, key);
        fields.push(("deserialize", decoder));
        fields
    });
    let Some(mut fields) = fields else { return };
    fields.append(&mut extra);
    entry(fw, cx, node, if result { "callback" } else { "site" }, fields);
}

/// Names the build's code again for a module, with the build's names (copied: nothing named here reaches the build).
pub(crate) fn name_copy(fw: &FwInstance, expr: &mut Expr) {
    let mut state = fw.serializer.borrow_mut();
    let state = &mut *state;
    let Some(generator) = state.generator.as_ref() else { return };
    let mut names = generator.names.clone();
    name_harvest(&mut names, &mut state.harvest.named, &mut [], Some(expr));
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
pub(crate) fn guard(gn: &mut Gen<'_, '_, '_>, ty: P<Type>) -> GResult<Expr> {
    let key = gn.wire_key(&Shape::Type(ty), &mut Vec::new(), false)?.0;
    Ok(super::luau::call(create::id("__fwguard"), vec![super::luau::str(&key)]))
}
