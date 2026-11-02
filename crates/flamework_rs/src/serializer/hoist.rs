//! What lands at the top of the file: hoisted types' functions, the file's table, guards, literal and enum
//! tables, the varint and check helpers; the globals the code names; the serializer's version option.

use std::rc::Rc;

use rbxts_plugin::ts::{Kind as SyntaxKind, Node, P, Symbol, Type};
use rbxts_plugin::{Cx, PResult, PluginDiagnostic};
use tsrs_ast::SymbolFlags;
use tsrs_checker::TypeFlags;

use super::classify::datatype;
use super::codegen::{Ctx, Place};
use super::ir::{self, Body, Bind, E, Else, Id, Op, Param, S, Ty};
use super::typeinfo;
use super::{GResult, Gen, Hoisted, Kind, Shape, Slot, Varint};

const VARINT_MAX_BYTES: u32 = 5;
pub(crate) const MALFORMED: &str = "malformed payload";

/// `GLOBAL_VALUES`: how a hidden global is reached (`alias`), or refused.
fn global_policy(name: &str) -> Option<bool> {
    match name {
        "buffer" | "error" | "math" => Some(true),
        "typeIs" | "$range" | "$tuple" | "Array" | "Map" | "Set" | "Enum" | "CFrame" | "warn" | "Promise" => Some(false),
        _ => None,
    }
}

/// The roles of a hoisted type's functions.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role {
    S,
    W,
    R,
}

impl Role {
    fn prefix(self) -> &'static str {
        match self {
            Role::S => "s",
            Role::W => "w",
            Role::R => "r",
        }
    }
}

impl Gen<'_, '_, '_> {
    /// `uid(hint)`: a generated identifier, with the type its declaration gives it.
    pub(crate) fn uid(&mut self, hint: &str, ty: Ty) -> Id {
        self.g.ids.new_id(hint, ty)
    }

    /// `parameter(hint)`: an identifier declared as a parameter of a generated function.
    pub(crate) fn parameter(&mut self, hint: &str, ty: Ty) -> Id {
        let id = self.uid(hint, ty);
        self.g.parameters.insert(id);
        id
    }

    /// `atFileLevel(build)`.
    pub(crate) fn at_file_level<T>(&mut self, build: impl FnOnce(&mut Self) -> T) -> T {
        self.g.file_level += 1;
        let result = build(self);
        self.g.file_level -= 1;
        result
    }

    // --- globals -----------------------------------------------------------------------------------------------

    /// `hidingDeclaration(name, meaning)`.
    fn hiding_declaration(&mut self, name: &str, meaning: SymbolFlags) -> Option<P<Symbol>> {
        let key = format!("{}:{}:{name}", if self.g.file_level > 0 { "file" } else { "site" }, meaning.bits());
        if let Some(found) = self.g.hidden_here.get(&key) {
            return *found;
        }
        let location = if self.g.file_level > 0 { self.g.file.as_node() } else { self.g.diagnostic_node };
        let found = self.cx.checker().resolve_name_exported(name, Some(location), meaning, false);
        let global = self.cx.checker().resolve_name_exported(name, None, meaning, false);
        let hidden = if found.is_some() && found != global { found } else { None };
        self.g.hidden_here.insert(key, hidden);
        hidden
    }

    /// `refuseHidden(name, declared)`: records the error (once per value) and keeps building.
    fn refuse_hidden(&mut self, name: &str, declared: P<Symbol>, is_type: bool) {
        let mut at = String::new();
        if let Some(&declaration) = declared.declarations().first()
            && let Some(source) = tsrs_ast::get_source_file_of_node(declaration)
        {
            let line = self.cx.utf16_pos(declaration).line + 1;
            if source == self.g.file {
                at = format!(" on line {line}");
            } else {
                let id = self.shared.file_id(source.file_name());
                at = format!(" in {id}, line {line}");
            }
        }
        let message = format!(
            "Flamework's generated code here uses the global {}'{name}', which the declaration of '{name}'{at} hides. Rename that declaration.",
            if is_type { "type " } else { "" }
        );
        if self.g.sticky.is_none() {
            let abort = self.cx.error(self.g.diagnostic_node, message);
            self.g.sticky = Some(abort);
        }
    }

    /// `globalRef(name)`: the global, or the module-level alias of a hidden `buffer`, `math` or `error`.
    pub(crate) fn global_ref(&mut self, name: &str) -> E {
        let Some(hidden) = self.hiding_declaration(name, SymbolFlags::Value) else { return ir::global(name) };
        let alias_allowed = global_policy(name).unwrap_or(false);
        if !alias_allowed || self.g.file_level > 0 {
            self.refuse_hidden(name, hidden, false);
            return ir::global(name);
        }
        if let Some(&alias) = self.g.global_aliases.get(name) {
            return ir::id(alias);
        }
        let module_level = self.at_file_level(|gn| gn.hiding_declaration(name, SymbolFlags::Value));
        if let Some(module_level) = module_level {
            self.refuse_hidden(name, module_level, false);
            return ir::global(name);
        }
        let alias = self.uid(name, Ty::Unknown);
        self.g.declarations.push(S::Const(Bind::One(alias), ir::global(name)));
        self.g.global_aliases.insert(name.to_owned(), alias);
        ir::id(alias)
    }

    /// `checkGlobalsIn(node)` for the globals a literal names (`Enum`, a datatype).
    pub(crate) fn check_globals_in(&mut self, e: &E) {
        match e {
            E::Global(name) => {
                if global_policy(name).is_some() || datatype(name).is_some() {
                    self.global_ref(name);
                }
            }
            E::Prop(object, _) => self.check_globals_in(object),
            E::Elem(object, index) => {
                self.check_globals_in(object);
                self.check_globals_in(index);
            }
            _ => {}
        }
    }

    pub(crate) fn buffer_call(&mut self, method: &str, args: Vec<E>) -> E {
        let buffer = self.global_ref("buffer");
        ir::call(ir::prop(buffer, method), args)
    }

    pub(crate) fn math_call(&mut self, method: &str, args: Vec<E>) -> E {
        let math = self.global_ref("math");
        ir::call(ir::prop(math, method), args)
    }

    pub(crate) fn range(&mut self, from: E, to: E) -> E {
        let range = self.global_ref("$range");
        ir::call(range, vec![from, to])
    }

    pub(crate) fn tuple(&mut self, values: Vec<E>) -> E {
        let tuple = self.global_ref("$tuple");
        ir::call(tuple, values)
    }

    /// `typeIs(v, name)`.
    pub(crate) fn type_of_is(&mut self, value: E, name: &str) -> E {
        let type_is = self.global_ref("typeIs");
        ir::call(type_is, vec![value, ir::string(name)])
    }

    /// `new Name(args)`.
    pub(crate) fn construct(&mut self, name: &str, args: Vec<E>) -> E {
        self.global_ref(name);
        ir::new(name, args)
    }

    /// `error("…")`.
    pub(crate) fn raise(&mut self, message: &str) -> S {
        self.raise_with(ir::string(message))
    }

    /// `raiseWith(message)`: `error(message)`, or `assert(false, message)` where a `catch (error)` hides `error`.
    pub(crate) fn raise_with(&mut self, message: E) -> S {
        if self.g.file_level == 0
            && self.hiding_declaration("error", SymbolFlags::Value).is_some()
            && self.hiding_declaration("assert", SymbolFlags::Value).is_none()
        {
            return ir::stmt(ir::call(ir::global("assert"), vec![ir::bool(false), message]));
        }
        let error = self.global_ref("error");
        ir::stmt(ir::call(error, vec![message]))
    }

    // --- the file's table ------------------------------------------------------------------------------------

    /// `hoistedTable()`: `local codec = {}`, declared once.
    pub(crate) fn hoisted_table(&mut self) -> Id {
        if let Some(table) = self.g.function_table {
            return table;
        }
        let table = self.uid("codec", Ty::Function);
        self.g.declarations.push(S::Const(Bind::One(table), ir::object(Vec::new())));
        self.g.function_table = Some(table);
        table
    }

    /// `define(field, value)`: `codec.field = value`, recorded for the self-check.
    fn define(&mut self, field: &str, value: E) -> S {
        let table = self.hoisted_table();
        ir::assign(ir::prop(ir::id(table), field), value)
    }

    fn push_table(&mut self, statement: S, field: Option<String>) {
        self.g.tables.push(statement);
        self.g.table_defines.push(field);
    }

    fn push_definition(&mut self, statement: S, field: String) {
        self.g.definitions.push(statement);
        self.g.definition_defines.push(Some(field));
    }

    /// `noteCall(field, owner)`.
    pub(crate) fn note_call(&mut self, field: &str, owner: &str) {
        if !self.g.called.iter().any(|(f, ..)| f == field) {
            self.g.called.push((field.to_owned(), self.g.diagnostic_node, owner.to_owned()));
        }
    }

    /// `takeHoisted()`: what was hoisted since the last call, in an order that keeps every reference in scope.
    pub(crate) fn take_hoisted(&mut self) -> Vec<S> {
        let [d, t, f] = self.g.emitted;
        let mut statements = Vec::new();
        statements.extend(self.g.declarations[d..].iter().cloned());
        statements.extend(self.g.tables[t..].iter().cloned());
        statements.extend(self.g.definitions[f..].iter().cloned());
        let defined: Vec<String> = self.g.table_defines[t..]
            .iter()
            .chain(&self.g.definition_defines[f..])
            .flatten()
            .cloned()
            .collect();
        self.g.handed_out.extend(defined);
        self.g.emitted = [self.g.declarations.len(), self.g.tables.len(), self.g.definitions.len()];
        statements
    }

    // --- hoisting ----------------------------------------------------------------------------------------------

    /// `hoist(type)`: the type's hoisted functions, built on first use; `None` when it stays in place.
    pub(crate) fn hoist(&mut self, ty: P<Type>) -> GResult<Option<Rc<Hoisted>>> {
        if let Some((_, existing)) = self.g.hoisted.iter().find(|(t, _)| *t == ty) {
            return Ok(Some(existing.clone()));
        }
        if !self.can_hoist(ty)? {
            return Ok(None);
        }
        let shape = Shape::Type(ty);
        let layout = self.layout_of(&shape)?;
        let structure = self.describe(&shape)?;
        let named = self.hoist_name(ty);
        let is_collection = matches!(&*structure, Kind::Array(_) | Kind::Set(_) | Kind::Map(..));
        let name = match named {
            Some(named) if !is_collection => Some(named),
            _ => {
                if self.g.uses.get(&shape.key()).copied().unwrap_or(0) > 1 {
                    Some(self.generated_name(ty, structure.name()))
                } else {
                    None
                }
            }
        };
        let Some(name) = name else { return Ok(None) };

        let mut unique = name.clone();
        let mut suffix = 1;
        while self.g.hoisted_names.contains(&unique) {
            unique = format!("{name}_{suffix}");
            suffix += 1;
        }
        self.g.hoisted_names.insert(unique.clone());

        let owner = format!("the type '{}'", self.type_text(ty));
        let checks = self.has_checks(&shape)?;
        let size_checks = self.has_type_checks(&shape, true)?;
        let info = Rc::new(Hoisted { name: unique, owner, layout, checks, size_checks });

        let before = self.g.hoisted.len();
        self.g.hoisted.push((ty, info.clone()));
        let built = self.at_file_level(|gn| gn.build_hoisted(ty, &info));
        if let Err(error) = built {
            self.g.hoisted.truncate(before);
            return Err(error);
        }
        Ok(Some(info))
    }

    fn build_hoisted(&mut self, ty: P<Type>, info: &Hoisted) -> GResult<()> {
        self.g.trail.push(ty);
        let result = self.build_hoisted_functions(ty, info);
        self.g.trail.pop();
        result
    }

    fn build_hoisted_functions(&mut self, ty: P<Type>, info: &Hoisted) -> GResult<()> {
        let layout = info.layout;
        let kind = self.describe(&Shape::Type(ty))?;
        let kind_shape = Shape::Kind(kind.clone());
        let display = self.display_name(ty);
        let named = format!("the type '{}'", self.type_text(ty));

        let value = self.parameter("v", Ty::Unknown);
        let size_where = if info.size_checks { Some(self.uid("where", Ty::String)) } else { None };
        let size_params: Vec<Id> = std::iter::once(value).chain(size_where).collect();
        let size_body = self.fitted(
            &mut |gn| {
                let body = super::codegen::out();
                let place = Place { path: Some(String::new()), owner: Some(display.clone()), where_: size_where, ..Place::default() };
                let size = gn.emit_size(&kind_shape, ir::id(value), &body, &place)?;
                body.borrow_mut().push(S::Return(Some(size)));
                Ok(super::codegen::take(&body))
            },
            &size_params,
            Some(&named),
            None,
            0,
        )?;
        let definition = self.define(&format!("s_{}", info.name), ir::arrow(params(&size_params), Body::Block(size_body)));
        self.push_definition(definition, format!("s_{}", info.name));

        let buf = self.uid("buf", Ty::Buffer);
        let o = self.uid("o", Ty::Number);
        let blobs = if layout.blobs { Some(self.uid("blobs", Ty::Blobs)) } else { None };
        let where_ = if info.checks { Some(self.uid("where", Ty::String)) } else { None };

        let write_params: Vec<Id> = [buf, o, value].into_iter().chain(blobs).chain(where_).collect();
        let write_body = self.fitted(
            &mut |gn| {
                let body = super::codegen::out();
                let ctx = Ctx::new(buf, blobs, Some(o), 0, body.clone(), Place {
                    path: Some(String::new()),
                    where_,
                    owner: Some(display.clone()),
                    ..Place::default()
                });
                gn.emit_write(&kind_shape, ir::id(value), &ctx)?;
                gn.sync(&ctx);
                body.borrow_mut().push(S::Return(Some(ir::id(o))));
                Ok(super::codegen::take(&body))
            },
            &write_params,
            Some(&named),
            None,
            0,
        )?;
        let definition = self.define(&format!("w_{}", info.name), ir::arrow(params(&write_params), Body::Block(write_body)));
        self.push_definition(definition, format!("w_{}", info.name));

        let read_params: Vec<Id> = [buf, o].into_iter().chain(blobs).collect();
        let read_body = self.fitted(
            &mut |gn| {
                let body = super::codegen::out();
                let ctx = Ctx::new(buf, blobs, Some(o), 0, body.clone(), Place::default());
                let result = gn.emit_read(&kind_shape, &ctx)?;
                let bound = if result.is_identifier() || result.is_literal() {
                    result
                } else {
                    gn.bind(&body, result, "value", None)
                };
                gn.sync(&ctx);
                let tuple = gn.tuple(vec![bound, ir::id(o)]);
                body.borrow_mut().push(S::Return(Some(tuple)));
                Ok(super::codegen::take(&body))
            },
            &read_params,
            Some(&named),
            None,
            0,
        )?;
        let definition = self.define(&format!("r_{}", info.name), ir::arrow(params(&read_params), Body::Block(read_body)));
        self.push_definition(definition, format!("r_{}", info.name));
        Ok(())
    }

    /// `callHoisted(info, role, args)`.
    pub(crate) fn call_hoisted(&mut self, info: &Hoisted, role: Role, args: Vec<E>) -> E {
        let field = format!("{}_{}", role.prefix(), info.name);
        self.note_call(&field, &info.owner.clone());
        let table = self.hoisted_table();
        let callee = ir::cast(ir::prop(ir::id(table), &field), if role == Role::R { Ty::Tuple } else { Ty::Function });
        let call = ir::call(callee, args);
        if role == Role::R { ir::cast(call, Ty::Tuple) } else { call }
    }

    /// `generatedName(type, structure)`.
    fn generated_name(&mut self, ty: P<Type>, structure: &str) -> String {
        let printed = match typeinfo::alias_symbol(ty) {
            Some(alias) => alias.name().to_owned(),
            None => self.type_to_string(ty),
        };
        let text = if printed.contains('{') {
            self.g.hints.get(&ty).cloned().unwrap_or_else(|| structure.to_owned())
        } else {
            printed
        };
        let text = text.replace("[]", "Array");
        let mut words = String::new();
        let mut in_run = false;
        for c in text.chars() {
            if c.is_ascii_alphanumeric() || c == '_' {
                words.push(c);
                in_run = false;
            } else if !in_run {
                words.push('_');
                in_run = true;
            }
        }
        let words = words.trim_matches('_').to_owned();
        let name = if words.len() > 40 {
            let cut = &words[..40];
            match cut.rfind('_') {
                Some(at) => cut[..at].to_owned(),
                None => cut.to_owned(),
            }
        } else {
            words
        };
        if name.is_empty() { structure.to_owned() } else { name }
    }

    /// `hoistName(type)`.
    pub(crate) fn hoist_name(&mut self, ty: P<Type>) -> Option<String> {
        if !ty.flags().intersects(TypeFlags::Object | TypeFlags::UnionOrIntersection) {
            return None;
        }
        if typeinfo::is_instance_type(self.cx, ty) || typeinfo::get_literal(self.cx, ty, false).is_some() {
            return None;
        }
        // TS5's spelling of an internal name (a class expression's `__class`), from which the local is named.
        let name = tsrs_ast::escape_internal_symbol_name(typeinfo::alias_symbol(ty).or_else(|| ty.symbol())?.name());
        let name = name.as_str();
        if typeinfo::is_anonymous_name(name) || matches!(name, "Array" | "ReadonlyArray" | "CFrame") || datatype(name).is_some() {
            return None;
        }
        Some(name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' { c.to_string() } else { "_".repeat(c.len_utf16()) }).collect())
    }

    // --- slots -------------------------------------------------------------------------------------------------

    /// `fileSlots(hints)`.
    fn file_slots(&mut self, hints: &[(&str, Ty)]) -> Vec<Slot> {
        if self.g.claim_locals(hints.len() as u32) {
            return hints.iter().map(|(hint, ty)| Slot::Local(self.uid(hint, *ty))).collect();
        }
        hints
            .iter()
            .map(|(hint, _)| {
                let count = self.g.table_fields.get(*hint).copied().unwrap_or(0);
                self.g.table_fields.insert((*hint).to_owned(), count + 1);
                Slot::Field(if count == 0 { (*hint).to_owned() } else { format!("{hint}_{count}") })
            })
            .collect()
    }

    /// `readSlot(slot, type)`.
    fn read_slot(&mut self, slot: &Slot, ty: Ty) -> E {
        match slot {
            Slot::Local(id) => ir::id(*id),
            Slot::Field(field) => {
                let table = self.hoisted_table();
                ir::cast(ir::prop(ir::id(table), field), ty)
            }
        }
    }

    /// `declareSlot(slot, value)`.
    fn declare_slot(&mut self, slot: &Slot, value: E) -> S {
        match slot {
            Slot::Local(id) => S::Const(Bind::One(*id), value),
            Slot::Field(field) => {
                let table = self.hoisted_table();
                ir::assign(ir::prop(ir::id(table), field), value)
            }
        }
    }

    /// `guardFor(type)`: the member's `t` guard (F3's `buildGuardFromType`, [`crate::guards::guard_for`]), kept in a
    /// slot. Its errors are reported at the site being built.
    pub(crate) fn guard_for(&mut self, ty: P<Type>) -> GResult<E> {
        #[cfg(feature = "wire-harvest")]
        if self.g.companion {
            return super::wire::guard(self, ty);
        }
        let slot = match self.g.guards.get(&ty) {
            Some(slot) => slot.clone(),
            None => {
                let slot = self.file_slots(&[("guard", Ty::Function)]).remove(0);
                let expression = self.at_file_level(|gn| -> GResult<E> {
                    let node = gn.g.diagnostic_node;
                    let (guard, names) =
                        crate::guards::guard_for_unnamed(gn.cx, gn.shared.guard_config(), node, ty)?;
                    // `checkGlobalsIn(buildGuardFromType(…))`
                    for name in crate::guards::names::ts_globals_in(&guard) {
                        gn.global_ref(name);
                    }
                    Ok(E::Guard(std::rc::Rc::new(ir::Guard::new(guard, names))))
                })?;
                let statement = self.declare_slot(&slot, expression);
                self.push_table(statement, None);
                self.g.guards.insert(ty, slot.clone());
                slot
            }
        };
        Ok(self.read_slot(&slot, Ty::Function))
    }

    /// `enumTableFor(name)`: `Enum.<name>` items by `Value`, built once.
    pub(crate) fn enum_table_for(&mut self, name: &str) -> E {
        let slot = match self.g.enum_tables.get(name) {
            Some(slot) => slot.clone(),
            None => {
                let hint = format!("enum_{name}");
                let slot = self.file_slots(&[(&hint, Ty::EnumTable)]).remove(0);
                let id = match &slot {
                    Slot::Local(id) => *id,
                    Slot::Field(_) => self.uid(&hint, Ty::EnumTable),
                };
                let item = self.uid("item", Ty::EnumItem);
                self.at_file_level(|gn| {
                    let group = ir::prop(gn.global_ref("Enum"), name);
                    let items = ir::method(group, "GetEnumItems", Vec::new());
                    let build = vec![
                        S::Const(Bind::One(id), ir::object(Vec::new())),
                        S::ForOf(Bind::One(item), items, vec![ir::assign(
                            ir::elem(ir::id(id), ir::prop(ir::id(item), "Value")),
                            ir::id(item),
                        )]),
                    ];
                    if matches!(slot, Slot::Local(_)) {
                        for s in build {
                            gn.push_table(s, None);
                        }
                    } else {
                        let mut block = build;
                        block.push(gn.declare_slot(&slot, ir::id(id)));
                        gn.push_table(S::Block(block), None);
                    }
                });
                self.g.enum_tables.insert(name.to_owned(), slot.clone());
                slot
            }
        };
        self.read_slot(&slot, Ty::EnumTable)
    }

    /// `literalTablesFor(kind)`: the values as a list (decode) and as a lookup to their index (encode).
    pub(crate) fn literal_tables_for(&mut self, kind: &Rc<Kind>) -> (E, E) {
        let key = super::kind_key(kind);
        let (list, index) = match self.g.literal_tables.get(&key) {
            Some(entry) => entry.clone(),
            None => {
                let Kind::Literals(values) = &**kind else { panic!("literal tables of a literal group") };
                let slots = self.file_slots(&[("literals", Ty::Blobs), ("literalIndex", Ty::IndexMap)]);
                let (list, index) = (slots[0].clone(), slots[1].clone());
                self.at_file_level(|gn| {
                    for v in values {
                        gn.check_globals_in(v);
                    }
                    let pairs = values.iter().enumerate().map(|(i, v)| ir::array(vec![v.clone(), ir::num(i as f64)])).collect();
                    let list_value = ir::array(values.clone());
                    let map = gn.construct("Map", vec![ir::array(pairs)]);
                    let a = gn.declare_slot(&list, list_value);
                    gn.push_table(a, None);
                    let b = gn.declare_slot(&index, map);
                    gn.push_table(b, None);
                });
                self.g.literal_tables.insert(key, (list.clone(), index.clone()));
                (list, index)
            }
        };
        let list = self.read_slot(&list, Ty::Blobs);
        let index = self.read_slot(&index, Ty::IndexMap);
        (list, index)
    }

    // --- varints -----------------------------------------------------------------------------------------------

    /// `varintHelpers()`: `vsize`, `vwrite`, `vread`, built once per file.
    pub(crate) fn varint_helpers(&mut self) -> Varint {
        if let Some(varint) = self.g.varint {
            return varint;
        }
        let helpers = Varint {
            size: self.uid("vsize", Ty::Function),
            write: self.uid("vwrite", Ty::Function),
            read: self.uid("vread", Ty::Function),
        };
        self.at_file_level(|gn| gn.build_varint_helpers(helpers));
        self.g.varint = Some(helpers);
        helpers
    }

    fn build_varint_helpers(&mut self, varint: Varint) {
        let below = |value: E, limit: f64| ir::binary(value, Op::Lt, ir::num(limit));

        let size_of = self.uid("n", Ty::Number);
        let mut size = ir::num(f64::from(VARINT_MAX_BYTES));
        for bytes in (1..VARINT_MAX_BYTES).rev() {
            size = ir::cond(below(ir::id(size_of), 128f64.powi(bytes as i32)), ir::num(f64::from(bytes)), size);
        }
        let vsize = ir::arrow(vec![ir::param(size_of)], Body::Expr(size));
        self.push_table(S::Const(Bind::One(varint.size), vsize), None);

        let wbuf = self.uid("buf", Ty::Buffer);
        let wo = self.uid("o", Ty::Number);
        let wn = self.uid("n", Ty::Number);
        let write_byte = self.buffer_call(
            "writeu8",
            vec![
                ir::id(wbuf),
                ir::id(wo),
                ir::binary(ir::binary(ir::id(wn), Op::Mod, ir::num(128.0)), Op::Add, ir::num(128.0)),
            ],
        );
        let floor = self.math_call("floor", vec![ir::binary(ir::id(wn), Op::Div, ir::num(128.0))]);
        let last = self.buffer_call("writeu8", vec![ir::id(wbuf), ir::id(wo), ir::id(wn)]);
        let vwrite = ir::arrow(
            vec![ir::param(wbuf), ir::param(wo), ir::param(wn)],
            Body::Block(vec![
                S::While(ir::binary(ir::id(wn), Op::Ge, ir::num(128.0)), vec![
                    ir::stmt(write_byte),
                    ir::add_assign(ir::id(wo), ir::num(1.0)),
                    ir::assign(ir::id(wn), floor),
                ]),
                ir::stmt(last),
                S::Return(Some(super::codegen::add(ir::id(wo), 1.0))),
            ]),
        );
        self.push_table(S::Const(Bind::One(varint.write), vwrite), None);

        let rbuf = self.uid("buf", Ty::Buffer);
        let ro = self.uid("o", Ty::Number);
        let rn = self.uid("n", Ty::Number);
        let byte = self.uid("b", Ty::Number);
        let scale = self.uid("scale", Ty::Number);
        let read_byte = self.buffer_call("readu8", vec![ir::id(rbuf), ir::id(ro)]);
        let tuple = self.tuple(vec![ir::id(rn), ir::id(ro)]);
        let raise = self.raise(MALFORMED);
        let vread = ir::arrow(
            vec![ir::param(rbuf), ir::param(ro)],
            Body::Block(vec![
                S::Let(rn, Some(ir::num(0.0))),
                S::Let(scale, Some(ir::num(1.0))),
                S::While(ir::bool(true), vec![
                    S::Const(Bind::One(byte), read_byte),
                    ir::add_assign(ir::id(ro), ir::num(1.0)),
                    ir::add_assign(
                        ir::id(rn),
                        ir::binary(ir::binary(ir::id(byte), Op::Mod, ir::num(128.0)), Op::Mul, ir::id(scale)),
                    ),
                    ir::if_stmt(below(ir::id(byte), 128.0), vec![S::Return(Some(tuple))], None),
                    S::Assign(ir::id(scale), ir::AssignOp::Mul, ir::num(128.0)),
                    ir::if_stmt(
                        ir::binary(ir::id(scale), Op::Gt, ir::num(128f64.powi(VARINT_MAX_BYTES as i32 - 1))),
                        vec![raise],
                        None,
                    ),
                ]),
            ]),
        );
        self.push_table(S::Const(Bind::One(varint.read), vread), None);
    }

    // --- check helpers -----------------------------------------------------------------------------------------

    /// `checkHelper()`: `codec.checkWidth`, built once.
    pub(crate) fn check_helper(&mut self) -> E {
        if !self.g.check_function {
            self.at_file_level(|gn| gn.build_check_helper());
            self.g.check_function = true;
        }
        self.note_call("checkWidth", "a width check");
        let table = self.hoisted_table();
        ir::cast(ir::prop(ir::id(table), "checkWidth"), Ty::Function)
    }

    fn realm_test(&mut self, body: &mut Vec<S>) {
        if self.g.checks.side == "both" {
            return;
        }
        let run_service = ir::method(ir::global("game"), "GetService", vec![ir::string("RunService")]);
        let in_realm = ir::method(run_service, if self.g.checks.side == "server" { "IsServer" } else { "IsClient" }, Vec::new());
        body.push(ir::if_stmt(ir::not(in_realm), vec![S::Return(Some(ir::bool(false)))], None));
    }

    fn build_check_helper(&mut self) {
        let width = self.uid("width", Ty::String);
        let value = self.uid("value", Ty::Number);
        let where_ = self.uid("where", Ty::String);
        let unit = self.uid("unit", Ty::String);
        let message = self.uid("message", Ty::String);
        let mut body = Vec::new();
        self.realm_test(&mut body);
        let text = E::Template(
            "[Flamework] ".into(),
            vec![
                (ir::id(width), " cannot hold ".into()),
                (ir::id(value), String::new()),
                (ir::id(unit), ", at ".into()),
                (ir::id(where_), String::new()),
            ],
        );
        body.push(S::Const(Bind::One(message), text));
        if self.g.checks.mode == "assert" {
            let error = self.global_ref("error");
            body.push(ir::stmt(ir::call(error, vec![ir::id(message), ir::num(2.0)])));
        } else {
            let warn = self.global_ref("warn");
            body.push(ir::stmt(ir::call(warn, vec![ir::id(message)])));
            body.push(S::Return(Some(ir::bool(true))));
        }
        let params = vec![
            ir::param(width),
            ir::param(value),
            ir::param(where_),
            Param { id: unit, default: Some(ir::string("")) },
        ];
        let definition = self.define("checkWidth", ir::arrow(params, Body::Block(body)));
        self.push_table(definition, Some("checkWidth".into()));
    }

    /// `typeCheckHelper()`: `codec.checkType`, built once.
    pub(crate) fn type_check_helper(&mut self) -> E {
        if !self.g.type_check_function {
            self.at_file_level(|gn| gn.build_type_check_helper());
            self.g.type_check_function = true;
        }
        self.note_call("checkType", "a type check");
        let table = self.hoisted_table();
        ir::cast(ir::prop(ir::id(table), "checkType"), Ty::Function)
    }

    fn build_type_check_helper(&mut self) {
        let expected = self.uid("expected", Ty::String);
        let value = self.uid("value", Ty::Unknown);
        let where_ = self.uid("where", Ty::String);
        let show = self.uid("show", Ty::Boolean);
        let got = self.uid("got", Ty::String);
        let message = self.uid("message", Ty::String);
        let mut body = Vec::new();
        self.realm_test(&mut body);
        let is = |name: &str| ir::binary(ir::id(got), Op::Eq, ir::string(name));
        let type_of = self.global_ref("typeOf");
        body.push(S::Let(got, Some(ir::call(type_of, vec![ir::id(value)]))));
        let quoted = super::codegen::interpolate(vec![
            super::codegen::Part::Text("\"".into()),
            super::codegen::Part::Expr(ir::id(value)),
            super::codegen::Part::Text("\"".into()),
        ]);
        let plain = super::codegen::interpolate(vec![super::codegen::Part::Text(String::new()), super::codegen::Part::Expr(ir::id(value))]);
        let shown = ir::binary(ir::binary(is("number"), Op::Or, is("boolean")), Op::Or, is("EnumItem"));
        body.push(ir::if_stmt(ir::id(show), vec![ir::if_stmt(
            is("string"),
            vec![ir::assign(ir::id(got), quoted)],
            Some(Else::If(ir::if_stmt(shown, vec![ir::assign(ir::id(got), plain)], None))),
        )], None));
        let text = super::codegen::interpolate(vec![
            super::codegen::Part::Text("[Flamework] ".into()),
            super::codegen::Part::Expr(ir::id(expected)),
            super::codegen::Part::Text(" expected, got ".into()),
            super::codegen::Part::Expr(ir::id(got)),
            super::codegen::Part::Text(", at ".into()),
            super::codegen::Part::Expr(ir::id(where_)),
        ]);
        body.push(S::Const(Bind::One(message), text));
        if self.g.checks.mode == "warn" {
            let warn = self.global_ref("warn");
            body.push(ir::if_stmt(
                ir::binary(ir::id(expected), Op::Eq, ir::string("boolean")),
                vec![ir::stmt(ir::call(warn, vec![ir::id(message)])), S::Return(Some(ir::bool(true)))],
                None,
            ));
        }
        let error = self.global_ref("error");
        body.push(ir::stmt(ir::call(error, vec![ir::id(message), ir::num(2.0)])));
        let params = vec![ir::param(expected), ir::param(value), ir::param(where_), ir::param(show)];
        let definition = self.define("checkType", ir::arrow(params, Body::Block(body)));
        self.push_table(definition, Some("checkType".into()));
    }

    /// `zeroTally()`: the file's zero-size element tally.
    pub(crate) fn zero_tally(&mut self) -> Id {
        if let Some(zeros) = self.g.zeros {
            return zeros;
        }
        let zeros = self.uid("zeros", Ty::Number);
        self.push_table(S::Let(zeros, Some(ir::num(0.0))), None);
        self.g.zeros = Some(zeros);
        zeros
    }

    /// `displayName(type)`: the type's name, parenthesized when TypeScript prints it as more than a name.
    pub(crate) fn display_name(&mut self, ty: P<Type>) -> String {
        let text = self.type_to_string(ty);
        if is_plain_name(&text) { text } else { format!("({text})") }
    }

    /// `rootPath(shape)`.
    pub(crate) fn root_path(&mut self, shape: &Shape) -> GResult<String> {
        let Shape::Type(ty) = shape else { return Ok("value".into()) };
        if self.hoist_name(*ty).is_none() {
            return Ok("value".into());
        }
        let kind = self.describe(shape)?;
        Ok(if matches!(&*kind, Kind::Object { .. } | Kind::Union { .. } | Kind::List { .. }) {
            self.display_name(*ty)
        } else {
            "value".into()
        })
    }

}

/// `/^[\w$.]+(<.*>)?$/`.
fn is_plain_name(text: &str) -> bool {
    let word = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '$' || c == '.';
    let end = text.find(|c: char| !word(c)).unwrap_or(text.len());
    if end == 0 {
        return false;
    }
    let rest = &text[end..];
    rest.is_empty() || (rest.starts_with('<') && rest.ends_with('>'))
}

fn params(ids: &[Id]) -> Vec<Param> {
    ids.iter().map(|&id| ir::param(id)).collect()
}

// --- the serializer's options --------------------------------------------------------------------------------------

const VERSION_MAX: f64 = 255.0;

/// `getSerializerVersion` (see [`super::serializer_version`]).
pub(crate) fn serializer_version(cx: &mut Cx<'_>, node: P<Node>, parameter: Option<P<Symbol>>) -> PResult<Option<u32>> {
    if !matches!(node.kind(), SyntaxKind::CallExpression | SyntaxKind::NewExpression) {
        return Ok(None);
    }
    let Some(parameter) = parameter else { return Ok(None) };
    let signature = cx.checker().get_resolved_signature_exported(node);
    let parameters = signature.parameters();
    let index = parameters.iter().position(|p| p.value_declaration().and_then(|d| d.symbol()) == Some(parameter));
    let arguments = node.arguments();
    let Some(argument) = index.and_then(|i| arguments.get(i).copied()) else { return Ok(None) };

    let options = tsrs_ast::skip_outer_expressions(argument, tsrs_ast::OuterExpressionKinds::All);
    if options.kind() != SyntaxKind::ObjectLiteralExpression {
        return Err(cx.error(
            argument,
            "Flamework reads a serializer's options when the project builds, so they have to be written here as an object literal, such as `{ version: 1 }`.",
        ));
    }

    let mut version: Option<(P<Node>, P<Type>)> = None;
    for &property in options.as_object_literal_expression().properties.nodes() {
        if property.kind() == SyntaxKind::SpreadAssignment {
            return Err(cx.error(
                property,
                "Flamework reads a serializer's options when the project builds, and cannot read a spread in them: write each option out.",
            ));
        }
        if option_name(cx, property.name())?.as_deref() != Some("version") {
            continue;
        }
        match property.kind() {
            SyntaxKind::PropertyAssignment => {
                let initializer = property.initializer().unwrap();
                let ty = cx.checker().get_type_at_location(initializer);
                version = Some((initializer, ty));
            }
            SyntaxKind::ShorthandPropertyAssignment => {
                let symbol = cx.checker().get_shorthand_assignment_value_symbol(Some(property));
                let ty = match symbol {
                    Some(s) => cx.checker().get_type_of_symbol_at_location(s, Some(property)),
                    None => None,
                };
                let ty = match ty {
                    Some(t) => t,
                    None => cx.checker().get_type_at_location(property.name().unwrap()),
                };
                version = Some((property, ty));
            }
            _ => return Err(cx.error(property, "A serializer's `version` has to be a number, written as `version: 1`.")),
        }
    }
    let Some((version_node, ty)) = version else {
        return Err(cx.error(argument, "Flamework found no `version` in these serializer options."));
    };
    let Some(value) = typeinfo::number_literal(ty) else {
        let text = cx.checker().type_to_string_exported(ty);
        let diagnostic = PluginDiagnostic::error(
            version_node,
            format!("Flamework writes a serializer's version into the code it generates, so it has to be known when the project builds, and the type of this one is '{text}'."),
        )
        .with_hint("Write the version as a number literal (`{ version: 3 }`), or as a `const` or an enum member whose type is that number.");
        return Err(cx.report(diagnostic).unwrap());
    };
    if value.fract() != 0.0 || !(0.0..=VERSION_MAX).contains(&value) {
        return Err(cx.error(
            version_node,
            format!(
                "A serializer's version is written as one byte: a whole number from 0 to 255, not {}.",
                crate::js::number_to_string(value)
            ),
        ));
    }
    Ok(Some(value as u32))
}

/// `optionName(name)`.
fn option_name(cx: &mut Cx<'_>, name: Option<P<Node>>) -> PResult<Option<String>> {
    let Some(name) = name else { return Ok(None) };
    match name.kind() {
        SyntaxKind::Identifier | SyntaxKind::StringLiteral => Ok(Some(name.text().to_owned())),
        SyntaxKind::ComputedPropertyName => {
            let expression = name.as_computed_property_name().expression;
            let ty = cx.checker().get_type_at_location(expression);
            if let Some(s) = typeinfo::string_literal(ty) {
                return Ok(Some(s.to_owned()));
            }
            if let Some(n) = typeinfo::number_literal(ty) {
                return Ok(Some(crate::js::number_to_string(n)));
            }
            let text = cx.checker().type_to_string_exported(ty);
            let diagnostic = PluginDiagnostic::error(
                name,
                format!("Flamework reads a serializer's options when the project builds, and cannot tell which option this key names: its type is '{text}'."),
            )
            .with_hint("Write the option's name out, as `version: 1`.");
            Err(cx.report(diagnostic).unwrap())
        }
        _ => Ok(None),
    }
}
