//! The generated TypeScript ([`super::ir`]) compiled to Luau as roblox-ts compiled it. Each generated identifier is
//! named when it is first lowered ([`LowerCx::local_name`]).
//!
//! Only the shapes the generator builds are handled; the roblox-ts behaviour each one follows is named where it is
//! done (`transformCallExpression`, the property call macros, `addOneIfArrayType`, …).

use std::collections::HashMap;

use rbxts_luau::{
    AnyIdentifier, AssignmentOperator, BinaryOperator, ElseBody, Expr, IfStatement, InterpolatedStringPart,
    ReturnStatement, Stmt, UnaryOperator, create,
};
use rbxts_plugin::Cx;

use super::ir::{AssignOp, Bind, Body, CallKind, E, Else, Id, Ids, Key, Op, S, Ty};
use super::luau;

/// What the lowering asks the compiler for.
pub trait LowerCx {
    /// `luau.tempId(hint)`: one of the compiler's temporaries (`Cx::temp_id`).
    fn temp_id(&mut self, hint: &str) -> rbxts_luau::TemporaryIdentifier;
    /// A generated local's name: `hint`, else the first of `hint_1`, `hint_2`, … (`hint1`, … when the hint ends in
    /// `_`) that the compiler approves (`Cx::is_usable_local_name`, which keeps it from every other name of the file).
    fn local_name(&mut self, hint: &str) -> String;
}

impl LowerCx for Cx<'_> {
    fn temp_id(&mut self, hint: &str) -> rbxts_luau::TemporaryIdentifier {
        Cx::temp_id(self, hint)
    }

    fn local_name(&mut self, hint: &str) -> String {
        crate::luau_names::unique_name(self, hint)
    }
}

/// Compiles IR to Luau, naming each generated identifier (`names`, shared by the file's code) when it first lowers it.
pub struct Lowerer<'l> {
    pub ids: &'l Ids,
    pub names: &'l mut HashMap<Id, String>,
    pub cx: &'l mut dyn LowerCx,
}

/// `skipDownwards`: past parentheses and casts.
fn skip_downwards(e: &E) -> &E {
    let mut e = e;
    loop {
        match e {
            E::Paren(inner) | E::As(inner, _) => e = inner,
            _ => return e,
        }
    }
}

impl Lowerer<'_> {
    /// The identifier's name, given now if it has none yet.
    pub fn name(&mut self, id: Id) -> String {
        if let Some(name) = self.names.get(&id) {
            return name.clone();
        }
        let name = self.cx.local_name(self.ids.hint(id));
        self.names.insert(id, name.clone());
        name
    }

    fn any_id(&mut self, id: Id) -> AnyIdentifier {
        create::any_id(self.name(id))
    }

    /// The type TypeScript gives an expression, as far as roblox-ts's choices need it.
    pub fn type_of(&self, e: &E) -> Ty {
        match e {
            E::Id(id) => self.ids.ty(*id),
            E::As(_, ty) => *ty,
            E::Paren(inner) => self.type_of(inner),
            E::Str(_) | E::Template(..) => Ty::String,
            E::Num(_) | E::Neg(_) => Ty::Number,
            E::Bool(_) | E::Not(_) => Ty::Boolean,
            E::New(class, _) => match class.as_str() {
                "Array" => Ty::Array,
                "Map" => Ty::Map,
                "Set" => Ty::Set,
                "CFrame" => Ty::CFrame,
                _ => Ty::Datatype,
            },
            E::Call(callee, _, CallKind::Method) => match skip_downwards(callee) {
                E::Prop(_, name) if name == "GetEnumItems" => Ty::Array,
                _ => Ty::Unknown,
            },
            E::Binary(l, Op::Add, r) => {
                if self.type_of(l) == Ty::String || self.type_of(r) == Ty::String {
                    Ty::String
                } else {
                    Ty::Number
                }
            }
            _ => Ty::Unknown,
        }
    }

    // --- statements ---------------------------------------------------------------------------------------

    pub fn statements(&mut self, list: &[S]) -> Vec<Stmt> {
        let mut out = Vec::new();
        for s in list {
            self.statement(s, &mut out);
        }
        out
    }

    fn statement(&mut self, s: &S, out: &mut Vec<Stmt>) {
        match s {
            S::Const(Bind::One(id), e) => {
                let name = self.any_id(*id);
                let value = self.expr(e, out);
                out.push(create::local(name, Some(value)));
            }
            S::Const(Bind::Tuple(ids), e) => {
                let left = ids.iter().map(|id| self.any_id(*id)).collect();
                let value = self.expr(e, out);
                out.push(create::locals(left, vec![value]));
            }
            S::Let(id, e) => {
                let name = self.any_id(*id);
                let value = e.as_ref().map(|e| self.expr(e, out));
                out.push(create::local(name, value));
            }
            S::Expr(e) => self.expression_statement(e, out),
            S::Assign(target, op, value) => {
                let target = self.writable(target, out);
                let value_expr = self.expr(value, out);
                let operator = match op {
                    AssignOp::Assign => AssignmentOperator::Assign,
                    AssignOp::Add => {
                        if self.type_of(value) == Ty::String {
                            AssignmentOperator::Concat
                        } else {
                            AssignmentOperator::Add
                        }
                    }
                    AssignOp::Mul => AssignmentOperator::Mul,
                };
                out.push(create::assignment(target, operator, value_expr));
            }
            S::If(..) => {
                let statement = self.if_statement(s, out);
                out.push(Stmt::If(Box::new(statement)));
            }
            S::Block(list) => {
                let body = self.statements(list);
                out.push(create::do_stmt(body));
            }
            S::ForOf(bind, iterable, body) => self.for_of(bind, iterable, body, out),
            S::While(condition, body) => {
                let condition = self.expr(condition, out);
                let body = self.statements(body);
                out.push(create::while_stmt(condition, body));
            }
            S::Luau(s) => out.push(s.clone()),
            S::Return(None) => out.push(Stmt::Return(ReturnStatement { expressions: Vec::new() })),
            S::Return(Some(e)) => {
                // `return $tuple(a, b)`: several values.
                if let E::Call(callee, args, CallKind::Plain) = e
                    && matches!(skip_downwards(callee), E::Global(name) if name == "$tuple")
                {
                    let expressions = args.iter().map(|a| self.expr(a, out)).collect();
                    out.push(Stmt::Return(ReturnStatement { expressions }));
                    return;
                }
                let value = self.expr(e, out);
                out.push(create::return_stmt(value));
            }
        }
    }

    fn if_statement(&mut self, s: &S, out: &mut Vec<Stmt>) -> IfStatement {
        let S::If(condition, then, otherwise) = s else { unreachable!() };
        let condition = self.expr(condition, out);
        let statements = self.statements(then);
        let else_body = match otherwise.as_deref() {
            None => ElseBody::Block(Vec::new()),
            Some(Else::Block(list)) => ElseBody::Block(self.statements(list)),
            Some(Else::If(inner)) => {
                let mut prereqs = Vec::new();
                let nested = self.if_statement(inner, &mut prereqs);
                if prereqs.is_empty() {
                    ElseBody::If(Box::new(nested))
                } else {
                    prereqs.push(Stmt::If(Box::new(nested)));
                    ElseBody::Block(prereqs)
                }
            }
        };
        IfStatement { condition, statements, else_body }
    }

    /// `transformForOfStatement`: a `$range` is a numeric loop, an array `for _, v in`, a set `for v in`, a map
    /// `for k, v in`.
    fn for_of(&mut self, bind: &Bind, iterable: &E, body: &[S], out: &mut Vec<Stmt>) {
        if let E::Call(callee, args, CallKind::Plain) = iterable
            && matches!(skip_downwards(callee), E::Global(name) if name == "$range")
        {
            let Bind::One(id) = bind else { panic!("a $range loop binds one value") };
            let start = self.expr(&args[0], out);
            let end = self.expr(&args[1], out);
            let statements = self.statements(body);
            out.push(create::numeric_for(self.any_id(*id), start, end, None, statements));
            return;
        }
        let ty = self.type_of(iterable);
        let expression = self.expr(iterable, out);
        let ids = match (bind, ty) {
            (Bind::One(id), Ty::Array | Ty::Blobs) => vec![create::any_id("_"), self.any_id(*id)],
            (Bind::One(id), _) => vec![self.any_id(*id)],
            (Bind::Tuple(ids), _) => ids.iter().map(|id| self.any_id(*id)).collect(),
        };
        let statements = self.statements(body);
        out.push(create::for_in(ids, expression, statements));
    }

    /// An expression statement: a call, or one of the macros that are statements (`push`, `set`, `add`).
    fn expression_statement(&mut self, e: &E, out: &mut Vec<Stmt>) {
        let value = self.expr_in_statement(e, out);
        match value {
            Expr::None => {}
            call @ (Expr::Call(_) | Expr::MethodCall(_)) => out.push(Stmt::Call(call)),
            other => panic!("not a statement: {other:?}"),
        }
    }

    /// The left side of an assignment.
    fn writable(&mut self, e: &E, out: &mut Vec<Stmt>) -> Expr {
        self.expr(e, out)
    }

    // --- expressions --------------------------------------------------------------------------------------

    pub fn expr(&mut self, e: &E, out: &mut Vec<Stmt>) -> Expr {
        self.expr_used(e, out, true)
    }

    fn expr_in_statement(&mut self, e: &E, out: &mut Vec<Stmt>) -> Expr {
        self.expr_used(e, out, false)
    }

    fn expr_used(&mut self, e: &E, out: &mut Vec<Stmt>, used: bool) -> Expr {
        match e {
            E::Id(id) => create::id(self.name(*id)),
            E::Global(name) => create::id(name.as_str()),
            E::Ext(value) => value.value.borrow().clone(),
            E::Guard(guard) => {
                guard.name(|base| self.cx.local_name(base));
                guard.named()
            }
            E::Num(value) => luau::num(*value),
            E::Str(text) => luau::str(text),
            E::Bool(value) => create::bool(*value),
            E::Undefined => create::nil(),
            E::Prop(object, name) => {
                let object = self.expr(object, out);
                create::property(luau::indexable(object), name.as_str())
            }
            E::Elem(object, index) => {
                let ty = self.type_of(object);
                let object = self.expr(object, out);
                let index = self.expr(index, out);
                let index = if ty.is_array() { luau::add(index, 1.0) } else { index };
                create::computed_index(luau::indexable(object), index)
            }
            E::Call(callee, args, kind) => self.call(callee, args, *kind, out, used),
            E::Binary(left, op, right) => {
                let left_ty = self.type_of(left);
                let right_ty = self.type_of(right);
                let l = self.expr(left, out);
                let r = self.expr(right, out);
                let operator = match op {
                    Op::Add => {
                        if left_ty == Ty::String || right_ty == Ty::String {
                            let l = if left_ty == Ty::String { l } else { create::call(create::id("tostring"), vec![l]) };
                            let r = if right_ty == Ty::String { r } else { create::call(create::id("tostring"), vec![r]) };
                            return create::binary(l, BinaryOperator::Concat, r);
                        }
                        BinaryOperator::Add
                    }
                    Op::Sub => BinaryOperator::Sub,
                    Op::Mul => BinaryOperator::Mul,
                    Op::Div => BinaryOperator::Div,
                    Op::Mod => BinaryOperator::Mod,
                    Op::Lt => BinaryOperator::Lt,
                    Op::Le => BinaryOperator::Le,
                    Op::Gt => BinaryOperator::Gt,
                    Op::Ge => BinaryOperator::Ge,
                    Op::Eq => BinaryOperator::Eq,
                    Op::Ne => BinaryOperator::Ne,
                    Op::And => BinaryOperator::And,
                    Op::Or => BinaryOperator::Or,
                };
                create::binary(l, operator, r)
            }
            E::Not(inner) => {
                let inner = self.expr(inner, out);
                create::unary(UnaryOperator::Not, inner)
            }
            E::Neg(inner) => {
                let inner = self.expr(inner, out);
                create::unary(UnaryOperator::Neg, inner)
            }
            E::Paren(inner) => {
                let inner = self.expr(skip_downwards(inner), out);
                if inner.is_simple() { inner } else { create::paren(inner) }
            }
            E::As(inner, _) => self.expr_used(inner, out, used),
            E::Cond(condition, when_true, when_false) => {
                let condition = self.expr(condition, out);
                let when_true = self.expr(when_true, out);
                let when_false = self.expr(when_false, out);
                create::if_expr(condition, when_true, when_false)
            }
            E::Template(head, spans) => {
                let mut parts = Vec::new();
                if !head.is_empty() {
                    parts.push(InterpolatedStringPart::Text(luau::template_text(head)));
                }
                for (e, text) in spans {
                    parts.push(InterpolatedStringPart::Expression(self.expr(e, out)));
                    if !text.is_empty() {
                        parts.push(InterpolatedStringPart::Text(luau::template_text(text)));
                    }
                }
                create::interpolated_string(parts)
            }
            E::Arrow(arrow) => {
                let parameters = arrow.params.iter().map(|p| self.any_id(p.id)).collect();
                let mut statements = Vec::new();
                for p in &arrow.params {
                    if let Some(default) = &p.default {
                        let mut body = Vec::new();
                        let value = self.expr(default, &mut body);
                        body.push(create::assignment(create::id(self.name(p.id)), AssignmentOperator::Assign, value));
                        statements.push(create::if_stmt(
                            create::binary(create::id(self.name(p.id)), BinaryOperator::Eq, create::nil()),
                            body,
                            ElseBody::Block(Vec::new()),
                        ));
                    }
                }
                match &arrow.body {
                    Body::Block(list) => statements.extend(self.statements(list)),
                    Body::Expr(e) => {
                        let value = self.expr(e, &mut statements);
                        statements.push(create::return_stmt(value));
                    }
                }
                create::function(parameters, false, statements)
            }
            E::Object(fields) => {
                let mut out_fields = Vec::new();
                for (key, value) in fields {
                    let index = match key {
                        Key::Str(s) => luau::str(s),
                        Key::Num(n) => luau::num(*n),
                        Key::Computed(e) => self.expr(e, out),
                    };
                    let value = self.expr(value, out);
                    out_fields.push((index, value));
                }
                create::map(out_fields)
            }
            E::Array(list) => {
                let members = list.iter().map(|e| self.expr(e, out)).collect();
                create::array(members)
            }
            E::New(class, args) => self.construct(class, args, out),
            E::Luau(e) => e.clone(),
        }
    }

    /// `transformNewExpression`: the `Array`, `Map` and `Set` constructor macros, else `Class.new(...)`.
    fn construct(&mut self, class: &str, args: &[E], out: &mut Vec<Stmt>) -> Expr {
        match class {
            "Array" => {
                if args.is_empty() {
                    create::array(Vec::new())
                } else {
                    let args = args.iter().map(|a| self.expr(a, out)).collect();
                    create::call(create::property(create::id("table"), "create"), args)
                }
            }
            "Set" => create::set(Vec::new()),
            "Map" => match args.first() {
                None => create::map(Vec::<(Expr, Expr)>::new()),
                Some(E::Array(pairs)) => {
                    let mut fields = Vec::new();
                    for pair in pairs {
                        let E::Array(kv) = pair else { panic!("a Map pair is an array") };
                        let k = self.expr(&kv[0], out);
                        let v = self.expr(&kv[1], out);
                        fields.push((k, v));
                    }
                    create::map(fields)
                }
                Some(other) => panic!("unsupported Map argument {other:?}"),
            },
            _ => {
                let args = args.iter().map(|a| self.expr(a, out)).collect();
                create::call(create::property(create::id(class), "new"), args)
            }
        }
    }

    fn call(&mut self, callee: &E, args: &[E], kind: CallKind, out: &mut Vec<Stmt>, used: bool) -> Expr {
        let skipped = skip_downwards(callee);
        match kind {
            CallKind::Plain => {
                if let E::Global(name) = skipped {
                    match name.as_str() {
                        // `typeIs(v, name)`: `type(v) == name` for Luau's primitive types, `typeof` otherwise.
                        "typeIs" => {
                            let value = self.expr(&args[0], out);
                            let E::Str(type_name) = &args[1] else { panic!("typeIs takes a literal name") };
                            let function = if PRIMITIVE_LUAU_TYPES.contains(&type_name.as_str()) { "type" } else { "typeof" };
                            return create::binary(
                                create::call(create::id(function), vec![value]),
                                BinaryOperator::Eq,
                                luau::str(type_name),
                            );
                        }
                        "typeOf" => {
                            let args = args.iter().map(|a| self.expr(a, out)).collect();
                            return create::call(create::id("typeof"), args);
                        }
                        _ => {}
                    }
                }
                match skipped {
                    E::Prop(object, name) => {
                        let object = self.expr(object, out);
                        let args = args.iter().map(|a| self.expr(a, out)).collect();
                        create::call(create::property(luau::indexable(object), name.as_str()), args)
                    }
                    other => {
                        let callee = self.expr(other, out);
                        let args = args.iter().map(|a| self.expr(a, out)).collect();
                        create::call(luau::indexable(callee), args)
                    }
                }
            }
            CallKind::Method => {
                let E::Prop(object, name) = skipped else { panic!("a method call is on a property") };
                let object = self.expr(object, out);
                let args = args.iter().map(|a| self.expr(a, out)).collect();
                create::method_call(luau::indexable(object), name.as_str(), args)
            }
            CallKind::Macro => {
                let E::Prop(object, name) = skipped else { panic!("a macro call is on a property") };
                let ty = self.type_of(object);
                let receiver = self.expr(object, out);
                match (name.as_str(), ty) {
                    ("size", Ty::String | Ty::Array | Ty::Blobs) => create::unary(UnaryOperator::Len, receiver),
                    ("size", Ty::Set | Ty::Map) => {
                        let class = if ty == Ty::Set { "ReadonlySet.size" } else { "ReadonlyMap.size" };
                        // `pushToVar(0, "size")` and a counting loop, between the macro comments.
                        let size: Expr = self.cx.temp_id("size").into();
                        out.push(create::comment(format!(" ▼ {class} ▼")));
                        out.push(create::local(AnyIdentifier::try_from(size.clone()).unwrap(), Some(create::number(0.0))));
                        out.push(create::for_in(
                            Vec::new(),
                            receiver,
                            vec![create::assignment(size.clone(), AssignmentOperator::Add, create::number(1.0))],
                        ));
                        out.push(create::comment(format!(" ▲ {class} ▲")));
                        size
                    }
                    ("push", Ty::Array | Ty::Blobs) => {
                        for a in args {
                            let a = self.expr(a, out);
                            out.push(Stmt::Call(create::call(
                                create::property(create::id("table"), "insert"),
                                vec![receiver.clone(), a],
                            )));
                        }
                        if used { create::unary(UnaryOperator::Len, receiver) } else { create::none() }
                    }
                    ("get", Ty::IndexMap | Ty::Map) => {
                        let key = self.expr(&args[0], out);
                        create::computed_index(luau::indexable(receiver), key)
                    }
                    ("set", Ty::Map | Ty::IndexMap) => {
                        let key = self.expr(&args[0], out);
                        let value = self.expr(&args[1], out);
                        out.push(create::assignment(
                            create::computed_index(luau::indexable(receiver.clone()), key),
                            AssignmentOperator::Assign,
                            value,
                        ));
                        if used { receiver } else { create::none() }
                    }
                    ("add", Ty::Set) => {
                        let key = self.expr(&args[0], out);
                        out.push(create::assignment(
                            create::computed_index(luau::indexable(receiver.clone()), key),
                            AssignmentOperator::Assign,
                            create::bool(true),
                        ));
                        if used { receiver } else { create::none() }
                    }
                    (name, ty) => panic!("no macro {name} on {ty:?}"),
                }
            }
        }
    }
}

const PRIMITIVE_LUAU_TYPES: [&str; 10] =
    ["nil", "boolean", "string", "number", "table", "userdata", "function", "thread", "vector", "buffer"];

#[cfg(test)]
mod tests {
    use super::super::ir::{self, Bind, Body, Ids, Op, S, Ty};
    use super::*;
    use std::collections::HashSet;

    /// The compiler, as far as the lowering asks it: temporaries, and names approved once each, never one of the file's
    /// `identifiers`.
    struct TestCx {
        temps: rbxts_luau::TempIds,
        taken: HashSet<String>,
    }

    impl LowerCx for TestCx {
        fn temp_id(&mut self, hint: &str) -> rbxts_luau::TemporaryIdentifier {
            self.temps.temp_id(hint)
        }

        fn local_name(&mut self, hint: &str) -> String {
            let prefix = if hint.ends_with('_') { hint.to_owned() } else { format!("{hint}_") };
            let name = std::iter::once(hint.to_owned())
                .chain((1u32..).map(|i| format!("{prefix}{i}")))
                .find(|name| !self.taken.contains(name))
                .unwrap();
            self.taken.insert(name.clone());
            name
        }
    }

    /// Renders `statements` at the top of a file whose source uses `identifiers`.
    fn render(ids: &Ids, statements: &[S], identifiers: &[&str]) -> String {
        let mut names = HashMap::new();
        let mut cx =
            TestCx { temps: rbxts_luau::TempIds::new(), taken: identifiers.iter().map(|s| (*s).to_owned()).collect() };
        let mut lowerer = Lowerer { ids, names: &mut names, cx: &mut cx };
        let luau = lowerer.statements(statements);
        rbxts_luau::render_ast(&luau)
    }

    #[test]
    fn a_cast_property_keeps_its_parentheses_under_a_length() {
        let mut ids = Ids::default();
        let v = ids.new_id("v", Ty::Unknown);
        let n = ids.new_id("n", Ty::Number);
        let size = ir::macro_call(ir::cast(ir::prop(ir::id(v), "name"), Ty::String), "size", vec![]);
        let plain = ir::macro_call(ir::cast(ir::id(v), Ty::String), "size", vec![]);
        let statements = vec![S::Const(Bind::One(n), ir::binary(size, Op::Add, plain))];
        assert_eq!(render(&ids, &statements, &["v"]), "local n = #(v_1.name) + #v_1\n");
    }

    #[test]
    fn array_indexes_fold_the_offset() {
        let mut ids = Ids::default();
        let blobs = ids.new_id("blobs", Ty::Blobs);
        let index = ids.new_id("index", Ty::Number);
        let item = ids.new_id("item", Ty::Unknown);
        let statements = vec![
            S::Const(Bind::One(item), ir::elem(ir::id(blobs), ir::binary(ir::id(index), Op::Sub, ir::num(1.0)))),
            ir::assign(ir::elem(ir::id(blobs), ir::num(0.0)), ir::id(item)),
            ir::assign(ir::elem(ir::cast(ir::id(item), Ty::Record), ir::num(10.0)), ir::num(-1.0)),
        ];
        assert_eq!(render(&ids, &statements, &[]), "local item = blobs[index]\nblobs[1] = item\nitem[10] = -1\n");
    }

    #[test]
    fn nested_conditionals_render_as_elseif() {
        let mut ids = Ids::default();
        let vsize = ids.new_id("vsize", Ty::Function);
        let n = ids.new_id("n", Ty::Number);
        let mut size = ir::num(5.0);
        for bytes in (1..5).rev() {
            size = ir::cond(ir::binary(ir::id(n), Op::Lt, ir::num(128f64.powi(bytes))), ir::num(f64::from(bytes)), size);
        }
        let statements = vec![S::Const(Bind::One(vsize), ir::arrow(vec![ir::param(n)], Body::Expr(size)))];
        assert_eq!(
            render(&ids, &statements, &["n"]),
            "local vsize = function(n_1)\n\treturn if n_1 < 128 then 1 elseif n_1 < 16384 then 2 elseif n_1 < 2097152 then 3 elseif n_1 < 268435456 then 4 else 5\nend\n"
        );
    }

    #[test]
    fn type_is_picks_type_or_typeof() {
        let mut ids = Ids::default();
        let v = ids.new_id("v", Ty::Unknown);
        let ok = ids.new_id("ok", Ty::Boolean);
        let test = ir::binary(
            ir::call(ir::global("typeIs"), vec![ir::id(v), ir::string("table")]),
            Op::And,
            ir::call(ir::global("typeIs"), vec![ir::id(v), ir::string("Vector3")]),
        );
        let statements = vec![S::Const(Bind::One(ok), test)];
        assert_eq!(render(&ids, &statements, &[]), "local ok = type(v) == \"table\" and typeof(v) == \"Vector3\"\n");
    }

    #[test]
    fn set_sizes_count_in_a_loop_between_comments() {
        let mut ids = Ids::default();
        let set = ids.new_id("set", Ty::Set);
        let n = ids.new_id("n", Ty::Number);
        let statements = vec![S::Const(Bind::One(n), ir::macro_call(ir::id(set), "size", vec![]))];
        assert_eq!(
            render(&ids, &statements, &[]),
            "-- ▼ ReadonlySet.size ▼\nlocal _size = 0\nfor _ in set do\n\t_size += 1\nend\n-- ▲ ReadonlySet.size ▲\nlocal n = _size\n"
        );
    }

    #[test]
    fn luau_passes_through() {
        let mut ids = Ids::default();
        let n = ids.new_id("n", Ty::Number);
        let length = rbxts_luau::create::unary(UnaryOperator::Len, rbxts_luau::create::id("s"));
        let statements = vec![
            S::Luau(rbxts_luau::create::local(rbxts_luau::create::any_id("s"), Some(luau::str("ab")))),
            S::Const(Bind::One(n), ir::binary(E::Luau(length), Op::Add, ir::num(1.0))),
            S::Luau(rbxts_luau::create::call_statement(luau::call(
                rbxts_luau::create::id("print"),
                vec![luau::num(-2.0)],
            ))),
        ];
        assert_eq!(render(&ids, &statements, &[]), "local s = \"ab\"\nlocal n = #s + 1\nprint(-2)\n");
    }
}
