//! What lands at the top of the file: hoisted types' functions, the file's table, guards, literal and enum
//! tables, the varint and check helpers; the globals the code names; the serializer's version option.
//!
//! Everything is built as Luau ([`super::luau`]), its locals placeholders of the file's [`crate::luau_names::Names`]
//! ([`Id`]), named when a hook hands its output out.

use std::rc::Rc;

use rbxts_luau::{AssignmentOperator, BinaryOperator, Expr, Stmt, create};
use rbxts_plugin::ts::{Kind as SyntaxKind, Node, P, Symbol, Type};
use rbxts_plugin::{Cx, PResult, PluginDiagnostic};
use tsrs_ast::SymbolFlags;
use tsrs_checker::TypeFlags;

use super::classify::datatype;
use super::codegen::{Ctx, Place, function};
use super::lit::Lit;
use super::luau::{self, Part};
use super::typeinfo;
use super::{GResult, Gen, Hoisted, Id, Kind, Shape, Slot, Varint};

const VARINT_MAX_BYTES: u32 = 5;
/// The types Luau's `type()` tells; any other is `typeof()`'s.
const PRIMITIVE_LUAU_TYPES: [&str; 10] =
    ["nil", "boolean", "string", "number", "table", "userdata", "function", "thread", "vector", "buffer"];
pub(crate) const MALFORMED: &str = "malformed payload";

/// `GLOBAL_VALUES`: how a hidden global is reached (`alias`), or refused.
fn global_policy(name: &str) -> Option<bool> {
    match name {
        "buffer" | "error" | "math" => Some(true),
        "typeIs" | "$range" | "Array" | "Set" | "Enum" | "CFrame" | "warn" | "Promise" => Some(false),
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
    /// `uid(hint)`: a generated local, named after `hint` when its code is handed out.
    pub(crate) fn uid(&mut self, hint: &str) -> Id {
        Id(self.g.names.fresh_index(hint) as u32)
    }

    /// A call site's argument, read by the packing before it is compiled: a placeholder the site replaces with the
    /// argument's Luau (`crate::networking::calls`), with the value of a literal argument, which checks fold.
    pub(crate) fn slot(&mut self, literal: Option<Lit>) -> Id {
        let id = self.uid("arg");
        if let Some(literal) = literal {
            self.g.slot_literals.insert(id, literal);
        }
        id
    }

    /// The literal a value is (`isLiteral`, with its value): `nil`, a boolean or a number, or a call site's literal
    /// argument ([`Self::slot`]).
    pub(crate) fn literal(&self, value: &Expr) -> Option<Lit> {
        match crate::luau_names::placeholder_index(value) {
            Some(index) => self.g.slot_literals.get(&Id(index as u32)).cloned(),
            None => Lit::of(value),
        }
    }

    /// A value code can read again and index as it is (`isIdentifier`): a name ([`luau::is_name`]), but not a call
    /// site's literal argument, whose Luau, a literal, is bound to a local first wherever a name is not enough.
    pub(crate) fn is_name(&self, value: &Expr) -> bool {
        luau::is_name(value) && self.literal(value).is_none()
    }

    /// [`Self::literal`]'s number.
    pub(crate) fn literal_number(&self, value: &Expr) -> Option<f64> {
        match self.literal(value)? {
            Lit::Num(n) => Some(n),
            _ => None,
        }
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

    /// The module-level alias that stands for the global `name` here (a hidden `buffer`, `math` or `error`, below
    /// the file level), declared once; `None` for the global itself. Records the refusal where no alias may stand for
    /// a hidden global (`globalRef`).
    fn global_alias(&mut self, name: &str) -> Option<Id> {
        let hidden = self.hiding_declaration(name, SymbolFlags::Value)?;
        let alias_allowed = global_policy(name).unwrap_or(false);
        if !alias_allowed || self.g.file_level > 0 {
            self.refuse_hidden(name, hidden, false);
            return None;
        }
        if let Some(&alias) = self.g.global_aliases.get(name) {
            return Some(alias);
        }
        let module_level = self.at_file_level(|gn| gn.hiding_declaration(name, SymbolFlags::Value));
        if let Some(module_level) = module_level {
            self.refuse_hidden(name, module_level, false);
            return None;
        }
        let alias = self.uid(name);
        self.g.declarations.push(luau::local(&alias.e(), Some(create::id(name))));
        self.g.global_aliases.insert(name.to_owned(), alias);
        Some(alias)
    }

    /// `globalRef(name)`: the global, or the module-level alias of a hidden `buffer`, `math` or `error`.
    pub(crate) fn global_ref(&mut self, name: &str) -> Expr {
        match self.global_alias(name) {
            Some(alias) => alias.e(),
            None => create::id(name),
        }
    }

    /// Refuses a declaration hiding `name` where the TS generator's code named the global `name` (the Luau itself
    /// may not).
    pub(crate) fn uses_global(&mut self, name: &str) {
        self.global_alias(name);
    }

    /// `checkGlobalsIn(node)` for the globals a literal names: an enum item's `Enum` (refused when a declaration hides
    /// it).
    pub(crate) fn check_globals_in(&mut self, literal: &Lit) {
        if let Lit::EnumItem { .. } = literal {
            self.global_alias("Enum");
        }
    }

    /// `buffer.<method>(args)`.
    pub(crate) fn buffer_call(&mut self, method: &str, args: Vec<Expr>) -> Expr {
        let buffer = self.global_ref("buffer");
        luau::call(luau::prop(buffer, method), args)
    }

    /// `math.<method>(args)`.
    pub(crate) fn math_call(&mut self, method: &str, args: Vec<Expr>) -> Expr {
        let math = self.global_ref("math");
        luau::call(luau::prop(math, method), args)
    }

    /// `for index = from, to do body end` (the TS generator's `for (const index of $range(from, to))`).
    pub(crate) fn for_range(&mut self, index: Id, from: Expr, to: Expr, body: Vec<Stmt>) -> Stmt {
        self.uses_global("$range");
        luau::for_range(&index.e(), from, to, body)
    }

    /// `typeIs(v, name)`: `type(v) == name` for Luau's primitive types, `typeof(v) == name` otherwise.
    pub(crate) fn type_of_is(&mut self, value: Expr, name: &str) -> Expr {
        self.uses_global("typeIs");
        let function = if PRIMITIVE_LUAU_TYPES.contains(&name) { "type" } else { "typeof" };
        luau::bin(luau::call(create::id(function), vec![value]), BinaryOperator::Eq, luau::str(name))
    }

    /// `new Name(args)`: an empty table for an `Array` (`table.create(n)` with a length) or a `Set`, else
    /// `Name.new(args)`.
    pub(crate) fn construct(&mut self, name: &str, args: Vec<Expr>) -> Expr {
        let class = self.global_ref(name);
        match name {
            "Array" if !args.is_empty() => luau::call(luau::prop(create::id("table"), "create"), args),
            "Array" | "Set" => create::map(Vec::<(Expr, Expr)>::new()),
            _ => luau::call(luau::prop(class, "new"), args),
        }
    }

    /// `error("…")`.
    pub(crate) fn raise(&mut self, message: &str) -> Stmt {
        self.raise_with(luau::str(message))
    }

    /// `raiseWith(message)`: `error(message)`, or `assert(false, message)` where a `catch (error)` hides `error`.
    pub(crate) fn raise_with(&mut self, message: Expr) -> Stmt {
        if self.g.file_level == 0
            && self.hiding_declaration("error", SymbolFlags::Value).is_some()
            && self.hiding_declaration("assert", SymbolFlags::Value).is_none()
        {
            return Stmt::Call(luau::call(create::id("assert"), vec![create::bool(false), message]));
        }
        let error = self.global_ref("error");
        Stmt::Call(luau::call(error, vec![message]))
    }

    // --- the file's table ------------------------------------------------------------------------------------

    /// `hoistedTable()`: `local codec = {}`, declared once.
    pub(crate) fn hoisted_table(&mut self) -> Id {
        if let Some(table) = self.g.function_table {
            return table;
        }
        let table = self.uid("codec");
        self.g.declarations.push(luau::local(&table.e(), Some(create::map(Vec::<(Expr, Expr)>::new()))));
        self.g.function_table = Some(table);
        table
    }

    /// `codec.field`.
    fn codec_field(&mut self, field: &str) -> Expr {
        let table = self.hoisted_table();
        luau::prop(table.e(), field)
    }

    /// `define(field, value)`: `codec.field = value`, recorded for the self-check.
    fn define(&mut self, field: &str, value: Expr) -> Stmt {
        luau::assign(self.codec_field(field), value)
    }

    fn push_table(&mut self, statement: Stmt, field: Option<String>) {
        self.g.tables.push(statement);
        self.g.table_defines.push(field);
    }

    fn push_definition(&mut self, statement: Stmt, field: String) {
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
    pub(crate) fn take_hoisted(&mut self) -> Vec<Stmt> {
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

        let value = self.uid("v");
        let size_where = if info.size_checks { Some(self.uid("where")) } else { None };
        let size_params: Vec<Id> = std::iter::once(value).chain(size_where).collect();
        let size_body = self.fitted(
            &mut |gn| {
                let body = super::codegen::out();
                let place = Place { path: Some(String::new()), owner: Some(display.clone()), where_: size_where, ..Place::default() };
                let size = gn.emit_size(&kind_shape, value.e(), &body, &place)?;
                body.borrow_mut().push(create::return_stmt(size));
                Ok(super::codegen::take(&body))
            },
            &size_params,
            Some(&named),
            None,
            0,
        )?;
        let definition = self.define(&format!("s_{}", info.name), function(&size_params, size_body));
        self.push_definition(definition, format!("s_{}", info.name));

        let buf = self.uid("buf");
        let o = self.uid("o");
        let blobs = if layout.blobs { Some(self.uid("blobs")) } else { None };
        let where_ = if info.checks { Some(self.uid("where")) } else { None };

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
                gn.emit_write(&kind_shape, value.e(), &ctx)?;
                gn.sync(&ctx);
                body.borrow_mut().push(create::return_stmt(o.e()));
                Ok(super::codegen::take(&body))
            },
            &write_params,
            Some(&named),
            None,
            0,
        )?;
        let definition = self.define(&format!("w_{}", info.name), function(&write_params, write_body));
        self.push_definition(definition, format!("w_{}", info.name));

        let read_params: Vec<Id> = [buf, o].into_iter().chain(blobs).collect();
        let read_body = self.fitted(
            &mut |gn| {
                let body = super::codegen::out();
                let ctx = Ctx::new(buf, blobs, Some(o), 0, body.clone(), Place::default());
                let result = gn.emit_read(&kind_shape, &ctx)?;
                let bound = if luau::is_name(&result) || luau::is_literal(&result) {
                    result
                } else {
                    gn.bind(&body, result, "value")
                };
                gn.sync(&ctx);
                body.borrow_mut().push(luau::returns(vec![bound, o.e()]));
                Ok(super::codegen::take(&body))
            },
            &read_params,
            Some(&named),
            None,
            0,
        )?;
        let definition = self.define(&format!("r_{}", info.name), function(&read_params, read_body));
        self.push_definition(definition, format!("r_{}", info.name));
        Ok(())
    }

    /// `callHoisted(info, role, args)`.
    pub(crate) fn call_hoisted(&mut self, info: &Hoisted, role: Role, args: Vec<Expr>) -> Expr {
        let field = format!("{}_{}", role.prefix(), info.name);
        self.note_call(&field, &info.owner.clone());
        luau::call(self.codec_field(&field), args)
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
    fn file_slots(&mut self, hints: &[&str]) -> Vec<Slot> {
        if self.g.claim_locals(hints.len() as u32) {
            return hints.iter().map(|hint| Slot::Local(self.uid(hint))).collect();
        }
        hints
            .iter()
            .map(|hint| {
                let count = self.g.table_fields.get(*hint).copied().unwrap_or(0);
                self.g.table_fields.insert((*hint).to_owned(), count + 1);
                Slot::Field(if count == 0 { (*hint).to_owned() } else { format!("{hint}_{count}") })
            })
            .collect()
    }

    /// `readSlot(slot)`.
    fn read_slot(&mut self, slot: &Slot) -> Expr {
        match slot {
            Slot::Local(id) => id.e(),
            Slot::Field(field) => self.codec_field(field),
        }
    }

    /// `declareSlot(slot, value)`.
    fn declare_slot(&mut self, slot: &Slot, value: Expr) -> Stmt {
        match slot {
            Slot::Local(id) => luau::local(&id.e(), Some(value)),
            Slot::Field(field) => luau::assign(self.codec_field(field), value),
        }
    }

    /// `guardFor(type)`: the member's `t` guard (F3's `buildGuardFromType`, [`crate::guards::guard_for`]), kept in a
    /// slot. Its errors are reported at the site being built.
    pub(crate) fn guard_for(&mut self, ty: P<Type>) -> GResult<Expr> {
        #[cfg(feature = "wire-harvest")]
        if self.g.companion {
            return super::wire::guard(self, ty);
        }
        let slot = match self.g.guards.get(&ty) {
            Some(slot) => slot.clone(),
            None => {
                let slot = self.file_slots(&["guard"]).remove(0);
                let expression = self.at_file_level(|gn| -> GResult<Expr> {
                    let node = gn.g.diagnostic_node;
                    let (mut guard, names) =
                        crate::guards::guard_for_unnamed(gn.cx, gn.shared.guard_config(), node, ty)?;
                    // `checkGlobalsIn(buildGuardFromType(…))`
                    for name in crate::guards::names::ts_globals_in(&guard) {
                        gn.global_alias(name);
                    }
                    // the guard's locals become the generator's, named with the code that holds the guard
                    names.rebase(&mut guard, |base| gn.uid(base).0 as usize);
                    Ok(guard)
                })?;
                let statement = self.declare_slot(&slot, expression);
                self.push_table(statement, None);
                self.g.guards.insert(ty, slot.clone());
                slot
            }
        };
        Ok(self.read_slot(&slot))
    }

    /// `enumTableFor(name)`: `Enum.<name>` items by `Value`, built once.
    pub(crate) fn enum_table_for(&mut self, name: &str) -> Expr {
        let slot = match self.g.enum_tables.get(name) {
            Some(slot) => slot.clone(),
            None => {
                let hint = format!("enum_{name}");
                let slot = self.file_slots(&[&hint]).remove(0);
                let id = match &slot {
                    Slot::Local(id) => *id,
                    Slot::Field(_) => self.uid(&hint),
                };
                let item = self.uid("item");
                self.at_file_level(|gn| {
                    let group = luau::prop(gn.global_ref("Enum"), name);
                    let items = luau::method(group, "GetEnumItems", Vec::new());
                    let (table, item) = (id.e(), item.e());
                    let by_value = luau::assign(luau::index(table.clone(), luau::prop(item.clone(), "Value")), item.clone());
                    let mut build = vec![
                        luau::local(&table, Some(create::map(Vec::<(Expr, Expr)>::new()))),
                        luau::for_in(&[create::id("_"), item], items, vec![by_value]),
                    ];
                    if matches!(slot, Slot::Local(_)) {
                        for s in build {
                            gn.push_table(s, None);
                        }
                    } else {
                        build.push(gn.declare_slot(&slot, table));
                        gn.push_table(luau::do_(build), None);
                    }
                });
                self.g.enum_tables.insert(name.to_owned(), slot.clone());
                slot
            }
        };
        self.read_slot(&slot)
    }

    /// `literalTablesFor(kind)`: the values as a list (decode) and as a lookup to their index (encode).
    pub(crate) fn literal_tables_for(&mut self, kind: &Rc<Kind>) -> (Expr, Expr) {
        let key = super::kind_key(kind);
        let (list, index) = match self.g.literal_tables.get(&key) {
            Some(entry) => entry.clone(),
            None => {
                let Kind::Literals(values) = &**kind else { panic!("literal tables of a literal group") };
                let slots = self.file_slots(&["literals", "literalIndex"]);
                let (list, index) = (slots[0].clone(), slots[1].clone());
                self.at_file_level(|gn| {
                    for v in values {
                        gn.check_globals_in(v);
                    }
                    let list_value = create::array(values.iter().map(Lit::to_luau).collect());
                    let pairs: Vec<(Expr, Expr)> =
                        values.iter().enumerate().map(|(i, v)| (v.to_luau(), luau::num(i as f64))).collect();
                    let a = gn.declare_slot(&list, list_value);
                    gn.push_table(a, None);
                    let b = gn.declare_slot(&index, create::map(pairs));
                    gn.push_table(b, None);
                });
                self.g.literal_tables.insert(key, (list.clone(), index.clone()));
                (list, index)
            }
        };
        let list = self.read_slot(&list);
        let index = self.read_slot(&index);
        (list, index)
    }

    // --- varints -----------------------------------------------------------------------------------------------

    /// `varintHelpers()`: `vsize`, `vwrite`, `vread`, built once per file.
    pub(crate) fn varint_helpers(&mut self) -> Varint {
        if let Some(varint) = self.g.varint {
            return varint;
        }
        let helpers = Varint {
            size: self.uid("vsize"),
            write: self.uid("vwrite"),
            read: self.uid("vread"),
        };
        self.at_file_level(|gn| gn.build_varint_helpers(helpers));
        self.g.varint = Some(helpers);
        helpers
    }

    fn build_varint_helpers(&mut self, varint: Varint) {
        use BinaryOperator::{Add, Div, Ge, Gt, Lt, Mod, Mul};
        let below = |value: Expr, limit: f64| luau::bin(value, Lt, luau::num(limit));
        let one = || luau::num(1.0);
        let byte_size = || luau::num(128.0);

        let n = self.uid("n").e();
        let mut size = luau::num(f64::from(VARINT_MAX_BYTES));
        for bytes in (1..VARINT_MAX_BYTES).rev() {
            size = luau::if_expr(below(n.clone(), 128f64.powi(bytes as i32)), luau::num(f64::from(bytes)), size);
        }
        let vsize = luau::function(&[n], vec![create::return_stmt(size)]);
        self.push_table(luau::local(&varint.size.e(), Some(vsize)), None);

        let buf = self.uid("buf").e();
        let o = self.uid("o").e();
        let n = self.uid("n").e();
        let low_bits = luau::bin(luau::bin(n.clone(), Mod, byte_size()), Add, byte_size());
        let write_byte = self.buffer_call("writeu8", vec![buf.clone(), o.clone(), low_bits]);
        let math = self.global_ref("math");
        let floor = luau::call(luau::prop(math, "floor"), vec![luau::bin(n.clone(), Div, byte_size())]);
        let last = self.buffer_call("writeu8", vec![buf.clone(), o.clone(), n.clone()]);
        let vwrite = luau::function(&[buf, o.clone(), n.clone()], vec![
            luau::while_(luau::bin(n.clone(), Ge, byte_size()), vec![
                Stmt::Call(write_byte),
                luau::compound(o.clone(), AssignmentOperator::Add, one()),
                luau::assign(n, floor),
            ]),
            Stmt::Call(last),
            create::return_stmt(luau::add(o, 1.0)),
        ]);
        self.push_table(luau::local(&varint.write.e(), Some(vwrite)), None);

        let buf = self.uid("buf").e();
        let o = self.uid("o").e();
        let n = self.uid("n").e();
        let byte = self.uid("b").e();
        let scale = self.uid("scale").e();
        let read_byte = self.buffer_call("readu8", vec![buf.clone(), o.clone()]);
        let raise = self.raise(MALFORMED);
        let both = luau::returns(vec![n.clone(), o.clone()]);
        let last_scale = luau::num(128f64.powi(VARINT_MAX_BYTES as i32 - 1));
        let vread = luau::function(&[buf, o.clone()], vec![
            luau::local(&n, Some(luau::num(0.0))),
            luau::local(&scale, Some(one())),
            luau::while_(create::bool(true), vec![
                luau::local(&byte, Some(read_byte)),
                luau::compound(o, AssignmentOperator::Add, one()),
                luau::compound(n, AssignmentOperator::Add, luau::bin(luau::bin(byte.clone(), Mod, byte_size()), Mul, scale.clone())),
                luau::if_chain(vec![(below(byte, 128.0), vec![both])], Vec::new()),
                luau::compound(scale.clone(), AssignmentOperator::Mul, byte_size()),
                luau::if_chain(vec![(luau::bin(scale, Gt, last_scale), vec![raise])], Vec::new()),
            ]),
        ]);
        self.push_table(luau::local(&varint.read.e(), Some(vread)), None);
    }

    // --- check helpers -----------------------------------------------------------------------------------------

    /// `checkHelper()`: `codec.checkWidth`, built once.
    pub(crate) fn check_helper(&mut self) -> Expr {
        if !self.g.check_function {
            self.at_file_level(|gn| gn.build_check_helper());
            self.g.check_function = true;
        }
        self.note_call("checkWidth", "a width check");
        self.codec_field("checkWidth")
    }

    fn realm_test(&mut self, body: &mut Vec<Stmt>) {
        if self.g.checks.side == "both" {
            return;
        }
        let run_service = luau::method(create::id("game"), "GetService", vec![luau::str("RunService")]);
        let in_realm = luau::method(run_service, if self.g.checks.side == "server" { "IsServer" } else { "IsClient" }, Vec::new());
        body.push(luau::if_chain(vec![(luau::not(in_realm), vec![create::return_stmt(create::bool(false))])], Vec::new()));
    }

    fn build_check_helper(&mut self) {
        let [width, value, where_, unit, message] =
            ["width", "value", "where", "unit", "message"].map(|hint| self.uid(hint).e());
        // `unit = ""`
        let unset = luau::bin(unit.clone(), BinaryOperator::Eq, create::nil());
        let mut body = vec![luau::if_chain(vec![(unset, vec![luau::assign(unit.clone(), luau::str(""))])], Vec::new())];
        self.realm_test(&mut body);
        let text = luau::interp(vec![
            Part::Text("[Flamework] ".into()),
            Part::Expr(width.clone()),
            Part::Text(" cannot hold ".into()),
            Part::Expr(value.clone()),
            Part::Expr(unit.clone()),
            Part::Text(", at ".into()),
            Part::Expr(where_.clone()),
        ]);
        body.push(luau::local(&message, Some(text)));
        if self.g.checks.mode == "assert" {
            let error = self.global_ref("error");
            body.push(Stmt::Call(luau::call(error, vec![message, luau::num(2.0)])));
        } else {
            let warn = self.global_ref("warn");
            body.push(Stmt::Call(luau::call(warn, vec![message])));
            body.push(create::return_stmt(create::bool(true)));
        }
        let definition = self.define("checkWidth", luau::function(&[width, value, where_, unit], body));
        self.push_table(definition, Some("checkWidth".into()));
    }

    /// `typeCheckHelper()`: `codec.checkType`, built once.
    pub(crate) fn type_check_helper(&mut self) -> Expr {
        if !self.g.type_check_function {
            self.at_file_level(|gn| gn.build_type_check_helper());
            self.g.type_check_function = true;
        }
        self.note_call("checkType", "a type check");
        self.codec_field("checkType")
    }

    fn build_type_check_helper(&mut self) {
        let [expected, value, where_, show, got, message] = ["expected", "value", "where", "show", "got", "message"].map(|hint| self.uid(hint).e());
        let mut body = Vec::new();
        self.realm_test(&mut body);
        let is = |name: &str| luau::bin(got.clone(), BinaryOperator::Eq, luau::str(name));
        let type_of = create::id("typeof");
        body.push(luau::local(&got, Some(luau::call(type_of, vec![value.clone()]))));
        let quoted = luau::interp(vec![Part::Text("\"".into()), Part::Expr(value.clone()), Part::Text("\"".into())]);
        let plain = luau::interp(vec![Part::Expr(value.clone())]);
        let shown = luau::bin(luau::bin(is("number"), BinaryOperator::Or, is("boolean")), BinaryOperator::Or, is("EnumItem"));
        let shows = luau::if_chain(
            vec![(is("string"), vec![luau::assign(got.clone(), quoted)]), (shown, vec![luau::assign(got.clone(), plain)])],
            Vec::new(),
        );
        body.push(luau::if_chain(vec![(show.clone(), vec![shows])], Vec::new()));
        let text = luau::interp(vec![
            Part::Text("[Flamework] ".into()),
            Part::Expr(expected.clone()),
            Part::Text(" expected, got ".into()),
            Part::Expr(got),
            Part::Text(", at ".into()),
            Part::Expr(where_.clone()),
        ]);
        body.push(luau::local(&message, Some(text)));
        if self.g.checks.mode == "warn" {
            let warn = self.global_ref("warn");
            let boolean = luau::bin(expected.clone(), BinaryOperator::Eq, luau::str("boolean"));
            let warns = vec![Stmt::Call(luau::call(warn, vec![message.clone()])), create::return_stmt(create::bool(true))];
            body.push(luau::if_chain(vec![(boolean, warns)], Vec::new()));
        }
        let error = self.global_ref("error");
        body.push(Stmt::Call(luau::call(error, vec![message, luau::num(2.0)])));
        let definition = self.define("checkType", luau::function(&[expected, value, where_, show], body));
        self.push_table(definition, Some("checkType".into()));
    }

    /// `zeroTally()`: the file's zero-size element tally.
    pub(crate) fn zero_tally(&mut self) -> Id {
        if let Some(zeros) = self.g.zeros {
            return zeros;
        }
        let zeros = self.uid("zeros");
        self.push_table(luau::local(&zeros.e(), Some(luau::num(0.0))), None);
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
