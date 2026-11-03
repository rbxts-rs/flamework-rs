//! The estimate of a generated function's locals and registers that decides when it is built compactly
//! (`util/luauLimits.ts`: `measurePressure`, `holdsLocals` and the `Temporaries` class), ported onto the generated
//! TypeScript ([`super::ir`]) it measured, so the decisions are TS5's.
//!
//! DESIGN §10.2 has the compaction thresholds read exact counts off the Luau instead, which may change the code
//! shape (never the wire); keeping TS5's estimate keeps the code byte for byte.

use std::collections::{HashMap, HashSet};

use super::ir::{AssignOp, Bind, Body, CallKind, E, Else, Ids, Op, S, Ty};

pub(crate) const LUAU_MAX_LOCALS: u32 = 200;
pub(crate) const LUAU_MAX_REGISTERS: u32 = 255;
const COMPACT_LOCALS: u32 = 150;
const COMPACT_REGISTERS: u32 = 200;

/// A function's most locals and registers at once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Pressure {
    pub locals: u32,
    pub registers: u32,
}

impl Pressure {
    /// `needsCompacting`.
    pub fn needs_compacting(self) -> bool {
        self.locals > COMPACT_LOCALS || self.registers > COMPACT_REGISTERS
    }

    /// `exceedsLimits`.
    pub fn exceeds_limits(self) -> bool {
        self.locals > LUAU_MAX_LOCALS || self.registers > LUAU_MAX_REGISTERS
    }
}

/// `measurePressure(statements, parameters, outer)`.
pub(crate) fn measure_pressure(ids: &Ids, statements: &[S], parameters: &[String], outer: u32) -> Pressure {
    Temporaries::new(ids, statements, parameters).block(statements, parameters.len() as u32 + outer)
}

/// `holdsLocals(statements)`: a declaration, or a temporary roblox-ts declares ahead of a statement.
pub(crate) fn holds_locals(ids: &Ids, statements: &[S]) -> bool {
    let temporaries = Temporaries::new(ids, statements, &[]);
    statements.iter().any(|s| declared_names(s) > 0 || temporaries.of_statement(s) > 0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Collection {
    Array,
    Set,
    Map,
}

#[derive(Clone, Copy)]
struct Cost {
    count: u32,
    prereqs: bool,
}

const FREE: Cost = Cost { count: 0, prereqs: false };

const MACRO_METHODS: &[&str] = &[
    "add", "byte", "clear", "delete", "every", "filter", "filterUndefined", "find", "findIndex", "forEach", "format",
    "get", "gmatch", "gsub", "has", "includes", "indexOf", "insert", "isEmpty", "join", "lower", "map", "mapFiltered",
    "match", "move", "pop", "push", "reduce", "remove", "rep", "reverse", "set", "shift", "size", "some", "sort",
    "split", "sub", "unorderedRemove", "unshift", "upper",
];
const LIBRARIES: &[&str] = &["buffer", "math", "string", "table", "bit32", "utf8", "vector", "coroutine", "os", "debug"];
const MACRO_FUNCTIONS: &[&str] = &["typeIs", "typeOf", "classIs"];
const GLOBAL_FUNCTIONS: &[&str] =
    &["error", "typeof", "type", "select", "tostring", "tonumber", "rawget", "rawequal", "rawlen", "next", "pairs", "ipairs", "print", "warn"];

fn is_global_name(name: &str) -> bool {
    LIBRARIES.contains(&name) || GLOBAL_FUNCTIONS.contains(&name) || name.starts_with(|c: char| c.is_ascii_uppercase())
}

/// `skipOuter`.
fn skip_outer(e: &E) -> &E {
    let mut e = e;
    while let E::Paren(inner) | E::As(inner, _) = e {
        e = inner;
    }
    e
}

/// `isPrimitive`.
fn is_primitive(e: &E) -> bool {
    match e {
        E::Ext(value) => value.literal.is_some(),
        _ => matches!(e, E::Str(_) | E::Num(_) | E::Bool(_) | E::Undefined),
    }
}

/// The text of an identifier node.
fn identifier<'e>(ids: &Ids, e: &'e E) -> Option<std::borrow::Cow<'e, str>> {
    match e {
        E::Id(id) => Some(std::borrow::Cow::Owned(ids.hint(*id).to_owned())),
        E::Global(name) => Some(std::borrow::Cow::Borrowed(name.as_str())),
        E::Ext(value) if value.literal.is_some() => None,
        E::Ext(_) | E::Guard(_) | E::Luau(_) => Some(std::borrow::Cow::Borrowed("$value")),
        _ => None,
    }
}

/// The locals a passed-through Luau statement ([`S::Luau`]) declares at its own level.
fn luau_locals(s: &rbxts_luau::Stmt) -> u32 {
    match s {
        rbxts_luau::Stmt::VariableDeclaration(declaration) => declaration.left.len() as u32,
        _ => 0,
    }
}

/// `castType(node)`: the type of the outermost cast, parentheses aside.
fn cast_type(e: &E) -> Option<Ty> {
    let mut e = e;
    while let E::Paren(inner) = e {
        e = inner;
    }
    if let E::As(_, ty) = e { Some(*ty) } else { None }
}

/// `isMeasurableType`: `string`, an array or a tuple type node.
fn is_measurable_type(ty: Ty) -> bool {
    matches!(ty, Ty::String | Ty::Array | Ty::Blobs)
}

/// `collectionOfType`.
fn collection_of_type(ty: Ty) -> Option<Collection> {
    match ty {
        Ty::Array | Ty::Blobs => Some(Collection::Array),
        Ty::Set => Some(Collection::Set),
        Ty::Map | Ty::IndexMap => Some(Collection::Map),
        _ => None,
    }
}

/// `collectionOf(node)`: by how a value is written.
fn collection_of(e: &E) -> Option<Collection> {
    let mut e = e;
    while let E::Paren(inner) = e {
        e = inner;
    }
    match e {
        E::As(inner, ty) => collection_of_type(*ty).or_else(|| collection_of(inner)),
        E::New(class, _) => match class.as_str() {
            "Array" | "ReadonlyArray" => Some(Collection::Array),
            "Set" | "ReadonlySet" => Some(Collection::Set),
            "Map" | "ReadonlyMap" => Some(Collection::Map),
            _ => None,
        },
        E::Call(callee, ..) => match &**callee {
            E::Global(name) if name == "pairs" => Some(Collection::Map),
            E::Prop(_, name) if name == "GetEnumItems" => Some(Collection::Array),
            _ => None,
        },
        _ => None,
    }
}

/// `isMeasurableValue`.
fn is_measurable_value(e: &E) -> bool {
    if let Some(ty) = cast_type(e) {
        return is_measurable_type(ty);
    }
    let mut e = e;
    while let E::Paren(inner) = e {
        e = inner;
    }
    matches!(e, E::Str(_) | E::Template(..) | E::Array(_))
}

/// `isBooleanTyped`.
fn is_boolean_typed(e: &E) -> bool {
    let mut e = e;
    while let E::Paren(inner) = e {
        e = inner;
    }
    match e {
        E::As(_, ty) => *ty == Ty::Boolean,
        E::Bool(_) | E::Not(_) | E::Call(..) => true,
        E::Binary(l, op, r) => match op {
            Op::Eq | Op::Ne | Op::Lt | Op::Le | Op::Gt | Op::Ge => true,
            Op::And | Op::Or => is_boolean_typed(l) && is_boolean_typed(r),
            _ => false,
        },
        _ => false,
    }
}

/// `isStringValue`.
fn is_string_value(e: &E) -> bool {
    if let Some(ty) = cast_type(e) {
        return ty == Ty::String;
    }
    matches!(skip_outer(e), E::Str(_) | E::Template(..))
}

/// `flatten(expression, operator)`.
fn flatten(e: &E, op: Op) -> Vec<&E> {
    let inner = if let E::Paren(inner) = e { inner } else { e };
    if let E::Binary(l, o, r) = inner
        && *o == op
    {
        let mut out = flatten(l, op);
        out.extend(flatten(r, op));
        return out;
    }
    vec![e]
}

fn last_index_with_prereqs(costs: &[Cost]) -> Option<usize> {
    costs.iter().rposition(|c| c.prereqs)
}

/// `declaredNames(statement)`.
fn declared_names(s: &S) -> u32 {
    match s {
        S::Const(bind, _) => binding_names(bind),
        S::Let(..) => 1,
        _ => 0,
    }
}

fn binding_names(bind: &Bind) -> u32 {
    match bind {
        Bind::One(_) => 1,
        Bind::Tuple(ids) => ids.len() as u32,
    }
}

struct Temporaries<'i> {
    ids: &'i Ids,
    constants: HashSet<String>,
    variables: HashSet<String>,
    measurable: HashSet<String>,
    collections: HashMap<String, Collection>,
    parameters: HashSet<String>,
    many_constants: bool,
}

impl<'i> Temporaries<'i> {
    fn new(ids: &'i Ids, statements: &[S], parameters: &[String]) -> Temporaries<'i> {
        let mut t = Temporaries {
            ids,
            constants: HashSet::new(),
            variables: HashSet::new(),
            measurable: HashSet::new(),
            collections: HashMap::new(),
            parameters: parameters.iter().cloned().collect(),
            many_constants: false,
        };
        let mut constants = HashSet::new();
        let mut names = HashSet::new();
        for s in statements {
            t.visit_statement(s, &mut constants, &mut names);
        }
        for name in names {
            if !t.constants.contains(&name) && !t.variables.contains(&name) && !t.parameters.contains(&name) {
                constants.insert(format!("s{name}"));
            }
        }
        t.many_constants = constants.len() > 255;
        t
    }

    fn declare(&mut self, id: super::ir::Id, constant: bool, initializer: Option<&E>) {
        let name = self.ids.hint(id).to_owned();
        let ty = self.ids.ty(id);
        if constant {
            self.constants.insert(name.clone());
        } else {
            self.variables.insert(name.clone());
        }
        if is_measurable_type(ty) || initializer.is_some_and(is_measurable_value) {
            self.measurable.insert(name.clone());
        }
        if let Some(c) = collection_of_type(ty).or_else(|| initializer.and_then(collection_of)) {
            self.collections.insert(name, c);
        }
    }

    fn visit_statement(&mut self, s: &S, constants: &mut HashSet<String>, names: &mut HashSet<String>) {
        match s {
            S::Const(bind, e) => {
                match bind {
                    Bind::One(id) => {
                        names.insert(self.ids.hint(*id).to_owned());
                        self.declare(*id, true, Some(e));
                    }
                    Bind::Tuple(list) => {
                        for id in list {
                            names.insert(self.ids.hint(*id).to_owned());
                            let name = self.ids.hint(*id).to_owned();
                            self.constants.insert(name);
                        }
                    }
                }
                self.visit_expr(e, constants, names);
            }
            S::Let(id, e) => {
                names.insert(self.ids.hint(*id).to_owned());
                self.declare(*id, false, e.as_ref());
                if let Some(e) = e {
                    self.visit_expr(e, constants, names);
                }
            }
            S::Expr(e) | S::Return(Some(e)) => self.visit_expr(e, constants, names),
            S::Luau(_) => {}
            S::Assign(t, _, v) => {
                self.visit_expr(t, constants, names);
                self.visit_expr(v, constants, names);
            }
            S::If(c, then, otherwise) => {
                self.visit_expr(c, constants, names);
                for s in then {
                    self.visit_statement(s, constants, names);
                }
                match otherwise.as_deref() {
                    Some(Else::If(s)) => self.visit_statement(s, constants, names),
                    Some(Else::Block(list)) => {
                        for s in list {
                            self.visit_statement(s, constants, names);
                        }
                    }
                    None => {}
                }
            }
            S::Block(list) => {
                for s in list {
                    self.visit_statement(s, constants, names);
                }
            }
            S::While(c, body) => {
                self.visit_expr(c, constants, names);
                for s in body {
                    self.visit_statement(s, constants, names);
                }
            }
            S::ForOf(bind, e, body) => {
                // `for (const x of ...)`: a `const` declaration list.
                match bind {
                    Bind::One(id) => {
                        names.insert(self.ids.hint(*id).to_owned());
                        self.declare(*id, true, None);
                    }
                    Bind::Tuple(list) => {
                        for id in list {
                            names.insert(self.ids.hint(*id).to_owned());
                            self.constants.insert(self.ids.hint(*id).to_owned());
                        }
                    }
                }
                self.visit_expr(e, constants, names);
                for s in body {
                    self.visit_statement(s, constants, names);
                }
            }
            S::Return(None) => {}
        }
    }

    fn visit_expr(&mut self, e: &E, constants: &mut HashSet<String>, names: &mut HashSet<String>) {
        match e {
            E::Num(n) => {
                constants.insert(format!("n{}", crate::js::number_to_string(*n)));
            }
            E::Str(s) => {
                constants.insert(format!("s{s}"));
            }
            E::Template(head, spans) => {
                constants.insert(format!("s{head}"));
                for (e, text) in spans {
                    self.visit_expr(e, constants, names);
                    constants.insert(format!("s{text}"));
                }
            }
            E::Prop(object, name) => {
                constants.insert(format!("s{name}"));
                self.visit_expr(object, constants, names);
            }
            E::Id(_) | E::Global(_) | E::Ext(..) | E::Guard(_) | E::Luau(_) | E::Undefined => {
                if let Some(name) = identifier(self.ids, e) {
                    names.insert(name.into_owned());
                } else {
                    names.insert("undefined".into());
                }
            }
            E::Bool(_) => {}
            E::Elem(o, i) => {
                self.visit_expr(o, constants, names);
                self.visit_expr(i, constants, names);
            }
            E::Call(callee, args, _) => {
                self.visit_expr(callee, constants, names);
                for a in args {
                    self.visit_expr(a, constants, names);
                }
            }
            E::Binary(l, _, r) => {
                self.visit_expr(l, constants, names);
                self.visit_expr(r, constants, names);
            }
            E::Not(e) | E::Neg(e) | E::Paren(e) | E::As(e, _) => self.visit_expr(e, constants, names),
            E::Cond(c, a, b) => {
                self.visit_expr(c, constants, names);
                self.visit_expr(a, constants, names);
                self.visit_expr(b, constants, names);
            }
            E::Arrow(arrow) => {
                for p in &arrow.params {
                    names.insert(self.ids.hint(p.id).to_owned());
                }
                match &arrow.body {
                    Body::Block(list) => {
                        for s in list {
                            self.visit_statement(s, constants, names);
                        }
                    }
                    Body::Expr(e) => self.visit_expr(e, constants, names),
                }
            }
            E::Object(fields) => {
                for (key, value) in fields {
                    match key {
                        super::ir::Key::Str(s) => {
                            constants.insert(format!("s{s}"));
                        }
                        super::ir::Key::Num(n) => {
                            constants.insert(format!("n{}", crate::js::number_to_string(*n)));
                        }
                        super::ir::Key::Computed(k) => self.visit_expr(k, constants, names),
                    }
                    self.visit_expr(value, constants, names);
                }
            }
            E::Array(list) | E::New(_, list) => {
                if let E::New(class, _) = e {
                    names.insert(class.clone());
                }
                for x in list {
                    self.visit_expr(x, constants, names);
                }
            }
        }
    }

    fn block(&self, statements: &[S], live: u32) -> Pressure {
        let mut live = live;
        let mut locals = live;
        let mut registers = live;
        for s in statements {
            let temporaries = self.of_statement(s);
            let inner = self.statement(s, live, temporaries);
            live += declared_names(s) + temporaries;
            locals = locals.max(inner.locals).max(live);
            registers = registers.max(inner.registers).max(live);
        }
        Pressure { locals, registers }
    }

    fn statement(&self, s: &S, live: u32, temporaries: u32) -> Pressure {
        let before = live;
        let live = live + temporaries;
        let flat = |registers: u32| Pressure { locals: live, registers: live + registers };
        let most = |pressures: &[Pressure]| Pressure {
            locals: pressures.iter().map(|p| p.locals).max().unwrap_or(0).max(live),
            registers: pressures.iter().map(|p| p.registers).max().unwrap_or(0).max(live),
        };
        match s {
            S::Const(bind, e) => {
                let names = binding_names(bind);
                let registers = self.registers(e);
                Pressure { locals: live + names, registers: live + registers.max(names) }
            }
            S::Let(_, e) => {
                let registers = e.as_ref().map_or(0, |e| self.registers(e));
                Pressure { locals: live + 1, registers: live + registers.max(1) }
            }
            S::Expr(e) => flat(self.registers(e)),
            S::Luau(s) => {
                let names = luau_locals(s);
                Pressure { locals: live + names, registers: live + names }
            }
            S::Assign(t, op, v) => flat(self.registers_of_assignment(t, *op, v)),
            S::Return(e) => flat(e.as_ref().map_or(0, |e| self.registers(e))),
            S::Block(list) => self.block(list, before),
            S::If(c, then, otherwise) => {
                let mut branches = vec![flat(self.registers(c)), self.block(then, live)];
                match otherwise.as_deref() {
                    Some(Else::If(s)) => branches.push(self.statement(s, live, self.of_statement(s))),
                    Some(Else::Block(list)) => branches.push(self.block(list, live)),
                    None => {}
                }
                most(&branches)
            }
            S::ForOf(bind, e, body) => {
                let loop_ = self.loop_of(bind, e);
                let inner = self.block(body, live + loop_.locals);
                most(&[flat(self.registers(e) + 3), Pressure {
                    locals: inner.locals,
                    registers: inner.registers + loop_.registers,
                }])
            }
            S::While(c, body) => most(&[flat(self.registers(c)), self.block(body, live)]),
        }
    }

    /// `loop(statement)`.
    fn loop_of(&self, bind: &Bind, iterable: &E) -> Pressure {
        if let E::Call(callee, ..) = skip_outer(iterable)
            && matches!(&**callee, E::Global(name) if name == "$range")
            && matches!(bind, Bind::One(_))
        {
            return Pressure { locals: 1, registers: 2 };
        }
        let generic = |variables: u32, inner: u32| Pressure { locals: variables + inner, registers: 3 + variables.max(2) - variables };
        let as_array = generic(2, 0);
        let as_set = generic(1, 0);
        let as_map = match bind {
            Bind::One(_) => generic(2, 1),
            Bind::Tuple(list) => generic(list.len() as u32, 0),
        };
        match self.collection_of(iterable) {
            Some(Collection::Array) => as_array,
            Some(Collection::Set) => as_set,
            Some(Collection::Map) => as_map,
            None => {
                let loops = [as_array, as_set, as_map];
                let locals = loops.iter().map(|l| l.locals).max().unwrap();
                let registers = loops.iter().map(|l| l.locals + l.registers).max().unwrap();
                Pressure { locals, registers: registers - locals }
            }
        }
    }

    fn collection_of(&self, e: &E) -> Option<Collection> {
        collection_of(e).or_else(|| {
            identifier(self.ids, skip_outer(e)).and_then(|name| self.collections.get(name.as_ref()).copied())
        })
    }

    /// `ofStatement(statement)`.
    fn of_statement(&self, s: &S) -> u32 {
        match s {
            S::Const(bind, e) => {
                let mut count = self.of(e).count;
                if matches!(bind, Bind::Tuple(_)) && !matches!(skip_outer(e), E::Array(_) | E::Call(..)) {
                    count += 1;
                }
                count
            }
            S::Let(_, e) => e.as_ref().map_or(0, |e| self.of(e).count),
            S::Expr(e) => self.of(e).count,
            S::Assign(t, _, v) => self.assignment(t, v, true).count,
            S::If(c, ..) | S::While(c, _) => self.condition(c).count,
            S::Block(_) | S::Luau(_) => 0,
            S::ForOf(_, e, _) => self.of(e).count,
            S::Return(e) => e.as_ref().map_or(0, |e| self.of(e).count),
        }
    }

    /// `of(node)`.
    fn of(&self, node: &E) -> Cost {
        let e = skip_outer(node);
        if is_primitive(e) || identifier(self.ids, e).is_some() {
            return FREE;
        }
        match e {
            E::Prop(object, _) => self.of(object),
            E::Elem(object, index) => {
                let object = self.of(object);
                let index = self.of(index);
                Cost { count: object.count + index.count + u32::from(index.prereqs), prereqs: object.prereqs || index.prereqs }
            }
            E::Call(callee, args, _) => self.call(callee, args),
            E::New(_, args) => self.ordered(&args.iter().collect::<Vec<_>>()),
            E::Binary(l, op, r) => match op {
                Op::And | Op::Or => self.logical(e, *op),
                _ => self.ordered(&[l, r]),
            },
            E::Cond(c, a, b) => {
                let condition = self.condition(c);
                let a = self.of(a);
                let b = self.of(b);
                if !a.prereqs && !b.prereqs {
                    return condition;
                }
                Cost { count: condition.count + 1 + a.count.max(b.count), prereqs: true }
            }
            E::Not(operand) => self.condition(operand),
            E::Neg(operand) => self.of(operand),
            E::Object(fields) => self.object(fields),
            E::Array(list) => self.ordered(&list.iter().collect::<Vec<_>>()),
            E::Template(_, spans) => self.ordered(&spans.iter().map(|(e, _)| e).collect::<Vec<_>>()),
            E::Arrow(_) => FREE,
            _ => FREE,
        }
    }

    fn ordered(&self, operands: &[&E]) -> Cost {
        let costs: Vec<Cost> = operands.iter().map(|o| self.of(o)).collect();
        let last = last_index_with_prereqs(&costs);
        let mut count = 0;
        for (index, cost) in costs.iter().enumerate() {
            count += cost.count;
            if last.is_some_and(|l| index < l) && self.is_copied_ahead(operands[index]) {
                count += 1;
            }
        }
        Cost { count, prereqs: last.is_some() }
    }

    fn is_copied_ahead(&self, operand: &E) -> bool {
        if let Some(name) = identifier(self.ids, operand)
            && self.constants.contains(name.as_ref())
        {
            return false;
        }
        !is_primitive(skip_outer(operand))
    }

    fn call(&self, callee: &E, args: &[E]) -> Cost {
        let property = if let E::Prop(object, name) = callee { Some((object, name)) } else { None };
        let object: &E = property.map_or(callee, |(o, _)| o);
        let head = self.of(object);
        let costs: Vec<Cost> = args.iter().map(|a| self.of(a)).collect();
        let last = last_index_with_prereqs(&costs);
        let mut count = head.count;
        let mut prereqs = head.prereqs;
        let mut argument_prereqs = last.is_some();
        let copied: Vec<bool> =
            args.iter().enumerate().map(|(i, a)| last.is_some_and(|l| i < l) && self.is_copied_ahead(a)).collect();
        for (i, cost) in costs.iter().enumerate() {
            count += cost.count + u32::from(copied[i]);
        }
        if let Some((object, name)) = property
            && MACRO_METHODS.contains(&name.as_str())
            && !is_library(self.ids, object)
        {
            for (i, a) in args.iter().enumerate() {
                if !copied[i] && self.might_change(a, true) {
                    count += 1;
                    argument_prereqs = true;
                }
            }
            if name == "size" && args.is_empty() && !self.is_measurable(object) {
                count += 1;
                prereqs = true;
            }
        }
        if argument_prereqs && self.might_change(object, false) {
            count += 1;
        }
        Cost { count, prereqs: prereqs || argument_prereqs }
    }

    fn assignment(&self, target: &E, value: &E, statement: bool) -> Cost {
        let target = skip_outer(target);
        let written = match target {
            E::Elem(o, i) => self.ordered(&[o, i]),
            E::Prop(o, _) => self.of(o),
            _ => FREE,
        };
        let value = self.of(value);
        let read = u32::from(!statement);
        Cost { count: written.count + value.count + read, prereqs: written.prereqs || value.prereqs || !statement }
    }

    fn logical(&self, e: &E, op: Op) -> Cost {
        let operands = flatten(e, op);
        let costs: Vec<Cost> = operands.iter().map(|o| self.of(o)).collect();
        let count: u32 = costs.iter().map(|c| c.count).sum();
        let inline = costs.iter().all(|c| !c.prereqs) && operands[..operands.len() - 1].iter().all(|o| is_boolean_typed(o));
        if inline { Cost { count, prereqs: false } } else { Cost { count: count + 1, prereqs: true } }
    }

    fn condition(&self, condition: &E) -> Cost {
        let cost = self.of(condition);
        let inner = skip_outer(condition);
        if is_boolean_typed(condition) || identifier(self.ids, inner).is_some() || is_primitive(inner) {
            return cost;
        }
        Cost { count: cost.count + 1, prereqs: true }
    }

    fn object(&self, fields: &[(super::ir::Key, E)]) -> Cost {
        let mut count = 0;
        let mut prereqs = false;
        for (key, value) in fields {
            let key = if let super::ir::Key::Computed(k) = key { self.of(k) } else { FREE };
            let value = self.of(value);
            let cost = Cost { count: key.count + value.count, prereqs: key.prereqs || value.prereqs };
            count += cost.count;
            if !cost.prereqs {
                continue;
            }
            if !prereqs {
                count += 1;
            }
            count += 1;
            prereqs = true;
        }
        Cost { count, prereqs }
    }

    fn might_change(&self, node: &E, unknown_names: bool) -> bool {
        let e = skip_outer(node);
        if is_primitive(e) || matches!(e, E::Arrow(_)) {
            return false;
        }
        if let Some(name) = identifier(self.ids, e) {
            if self.constants.contains(name.as_ref()) {
                return false;
            }
            return self.variables.contains(name.as_ref()) || unknown_names;
        }
        match e {
            E::Cond(c, a, b) => [c, a, b].iter().any(|p| self.might_change(p, unknown_names)),
            E::Binary(l, _, r) => self.might_change(l, unknown_names) || self.might_change(r, unknown_names),
            E::Not(o) | E::Neg(o) => self.might_change(o, unknown_names),
            E::Array(list) => list.iter().any(|x| self.might_change(x, unknown_names)),
            _ => true,
        }
    }

    fn is_measurable(&self, node: &E) -> bool {
        if is_measurable_value(node) {
            return true;
        }
        if cast_type(node).is_some() {
            return false;
        }
        let Some(name) = identifier(self.ids, skip_outer(node)) else { return false };
        let declared = self.constants.contains(name.as_ref()) || self.variables.contains(name.as_ref());
        !declared || self.measurable.contains(name.as_ref())
    }

    // --- registers ------------------------------------------------------------------------------------------

    fn registers(&self, e: &E) -> u32 {
        self.value(e).max(if self.counts_in_loop(e) { 5 } else { 0 })
    }

    fn registers_of_assignment(&self, target: &E, op: AssignOp, value: &E) -> u32 {
        let assigned = self.assigned(target, op, value);
        assigned.max(if self.counts_in_loop(target) || self.counts_in_loop(value) { 5 } else { 0 })
    }

    fn value(&self, node: &E) -> u32 {
        let e = skip_outer(node);
        if is_primitive(e) || identifier(self.ids, e).is_some() {
            return 1;
        }
        match e {
            E::Prop(object, _) => {
                if self.is_import(e) {
                    return 1;
                }
                1 + self.peak(object)
            }
            E::Elem(object, index) => self.index_registers(object, index),
            E::Call(callee, args, kind) => self.call_registers(callee, args, *kind),
            E::New(_, args) => self.call_of(1, args),
            E::Binary(..) => self.binary_registers(e),
            E::Cond(c, a, b) => (1 + self.tested(c)).max(self.value(a)).max(self.value(b)),
            E::Not(o) | E::Neg(o) => 1 + self.peak(o),
            E::Object(fields) => 1 + fields.iter().map(|(_, v)| self.value(v)).max().unwrap_or(0),
            E::Array(list) => 1 + (list.len() as u32).min(16) + list.iter().map(|x| self.value(x)).max().unwrap_or(0),
            E::Template(_, spans) => {
                let args: Vec<E> = spans.iter().map(|(e, _)| e.clone()).collect();
                1 + self.call_of(2, &args)
            }
            E::Arrow(_) => 1,
            _ => 1,
        }
    }

    fn is_local(&self, node: &E) -> bool {
        identifier(self.ids, skip_outer(node)).is_some_and(|name| {
            matches!(skip_outer(node), E::Id(_) | E::Ext(..) | E::Guard(_) | E::Global(_))
                && (self.constants.contains(name.as_ref())
                    || self.variables.contains(name.as_ref())
                    || self.parameters.contains(name.as_ref()))
        })
    }

    fn peak(&self, node: &E) -> u32 {
        if self.is_local(node) { 0 } else { self.value(node) }
    }

    fn held(&self, node: &E) -> u32 {
        u32::from(!self.is_local(node))
    }

    fn constant_peak(&self, node: &E) -> u32 {
        if is_primitive(skip_outer(node)) { 0 } else { self.peak(node) }
    }

    fn constant_held(&self, node: &E) -> u32 {
        if is_primitive(skip_outer(node)) { 0 } else { self.held(node) }
    }

    fn is_import(&self, e: &E) -> bool {
        let E::Prop(object, _) = e else { return false };
        let mut object = skip_outer(object);
        let mut names = 2;
        while let E::Prop(inner, _) = object {
            object = skip_outer(inner);
            names += 1;
        }
        names <= 3
            && !self.is_local(object)
            && identifier(self.ids, object).is_some_and(|name| is_global_name(name.as_ref()))
    }

    fn index_registers(&self, object: &E, index: &E) -> u32 {
        if is_primitive(skip_outer(index)) {
            return 1 + self.peak(object);
        }
        1 + self.peak(object).max(self.held(object) + 1 + self.peak(index))
    }

    fn call_registers(&self, node: &E, args: &[E], _kind: CallKind) -> u32 {
        let callee = skip_outer(node);
        if let E::Prop(object, name) = callee {
            if MACRO_METHODS.contains(&name.as_str()) && !is_library(self.ids, object) {
                if name == "size" && args.is_empty() {
                    return if self.is_measurable(object) { 1 + self.peak(object) } else { 1 };
                }
                if name == "get" && args.len() == 1 {
                    return 1 + self.peak(object).max(self.held(object) + self.constant_peak(&args[0]));
                }
                let mut all = vec![(**object).clone()];
                all.extend(args.iter().cloned());
                return 1 + self.call_of(1, &all);
            }
            if name.starts_with(|c: char| c.is_ascii_uppercase()) {
                return (1 + self.value(object)).max(self.call_of(2, args));
            }
        }
        if let E::Global(name) = callee
            && MACRO_FUNCTIONS.contains(&name.as_str())
        {
            return 1 + self.call_of(1, args);
        }
        self.value(callee).max(self.call_of(1, args))
    }

    fn call_of(&self, head: u32, args: &[E]) -> u32 {
        let mut registers = head + args.len() as u32;
        for (index, argument) in args.iter().enumerate() {
            registers = registers.max(head + index as u32 + self.value(argument));
        }
        registers
    }

    fn binary_registers(&self, e: &E) -> u32 {
        let E::Binary(left, op, right) = e else { unreachable!() };
        match op {
            Op::And | Op::Or => self.value(left).max(self.value(right)),
            Op::Eq | Op::Ne => 1 + self.tested(e),
            Op::Lt | Op::Le | Op::Gt | Op::Ge => 1 + self.peak(left).max(self.held(left) + self.peak(right)),
            _ => {
                if *op == Op::Add && (is_string_value(left) || is_string_value(right)) {
                    let operands: Vec<E> = flatten(e, Op::Add).into_iter().cloned().collect();
                    return self.call_of(1, &operands);
                }
                let right_peak =
                    if matches!(skip_outer(right), E::Num(_)) && !self.many_constants { 0 } else { self.peak(right) };
                1 + self.peak(left).max(self.held(left) + right_peak)
            }
        }
    }

    fn tested(&self, node: &E) -> u32 {
        let e = skip_outer(node);
        if is_primitive(e) || self.is_local(e) {
            return 0;
        }
        if let E::Not(o) = e {
            return self.tested(o);
        }
        if let E::Binary(l, op, r) = e {
            match op {
                Op::And | Op::Or => return self.tested(l).max(self.tested(r)),
                Op::Eq | Op::Ne => return self.constant_peak(l).max(self.constant_held(l) + self.constant_peak(r)),
                _ => {}
            }
        }
        self.value(e)
    }

    fn assigned(&self, target: &E, op: AssignOp, value: &E) -> u32 {
        let target = skip_outer(target);
        let compound = op != AssignOp::Assign;
        let value = if compound {
            let right = if matches!(skip_outer(value), E::Num(_)) && !self.many_constants { 0 } else { self.peak(value) };
            right + u32::from(!self.is_local(target))
        } else {
            self.value(value)
        };
        match target {
            E::Prop(object, _) => self.peak(object).max(self.held(object) + value),
            E::Elem(object, index) => {
                let constant = is_primitive(skip_outer(index));
                let index_peak = if constant { 0 } else { 1 + self.peak(index) };
                let index_held = u32::from(!constant);
                self.peak(object).max(self.held(object) + index_peak).max(self.held(object) + index_held + value)
            }
            _ => value,
        }
    }

    fn counts_in_loop(&self, e: &E) -> bool {
        match e {
            E::Arrow(_) => false,
            E::Call(callee, args, _) => {
                if args.is_empty()
                    && let E::Prop(object, name) = &**callee
                    && name == "size"
                    && !is_library(self.ids, object)
                    && !self.is_measurable(object)
                {
                    return true;
                }
                self.counts_in_loop(callee) || args.iter().any(|a| self.counts_in_loop(a))
            }
            E::Prop(o, _) | E::Not(o) | E::Neg(o) | E::Paren(o) | E::As(o, _) => self.counts_in_loop(o),
            E::Elem(o, i) => self.counts_in_loop(o) || self.counts_in_loop(i),
            E::Binary(l, _, r) => self.counts_in_loop(l) || self.counts_in_loop(r),
            E::Cond(c, a, b) => self.counts_in_loop(c) || self.counts_in_loop(a) || self.counts_in_loop(b),
            E::Array(list) | E::New(_, list) => list.iter().any(|x| self.counts_in_loop(x)),
            E::Object(fields) => fields.iter().any(|(_, v)| self.counts_in_loop(v)),
            E::Template(_, spans) => spans.iter().any(|(e, _)| self.counts_in_loop(e)),
            _ => false,
        }
    }
}

fn is_library(ids: &Ids, e: &E) -> bool {
    identifier(ids, skip_outer(e)).is_some_and(|name| LIBRARIES.contains(&name.as_ref()))
}
