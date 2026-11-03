//! The TypeScript the TS serializer generator builds, as a small tree: the generator is ported node for node onto
//! it, and [`crate::serializer::lower`] compiles it to Luau the way roblox-ts compiled the TypeScript.
//!
//! The TS transformer emitted TypeScript, which roblox-ts printed, parsed again and compiled. Three things of that
//! round trip show in the Luau and are kept here:
//! - **Parentheses.** `ts.factory` parenthesizes operands as it builds nodes (`parenthesizerRules`), and roblox-ts
//!   keeps a parenthesized expression whose inside is not simple (`#(v.name)` from `(v.name as string).size()`).
//!   The builders below apply the same rules.
//! - **Names.** Generated identifiers are `createUniqueName`s; an [`Id`] carries its hint until
//!   [`crate::serializer::lower`] names it, when it first lowers it (the names follow build order, not TS5's printer).
//! - **Types.** roblox-ts picks macros by type (`size()` of a string is `#s`, of a set a counting loop); every
//!   identifier records the type its declaration gives it ([`Ty`]), and casts carry theirs.

/// A generated identifier: an index into the generator's [`Ids`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Id(pub u32);

/// The types the generated code is written with, as far as roblox-ts's choices depend on them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Ty {
    Unknown,
    Number,
    String,
    Boolean,
    Buffer,
    /// `defined`
    Defined,
    /// `unknown[]`
    Array,
    /// `defined[]` (the blob list, a literal group's values)
    Blobs,
    /// `{ [key: string]: unknown }`
    Record,
    /// `Map<unknown, unknown>`
    Map,
    /// `Map<defined, number>` (a literal group's index)
    IndexMap,
    /// `Set<defined>`
    Set,
    EnumItem,
    /// `{ [value: number]: EnumItem }`
    EnumTable,
    CFrame,
    /// A Roblox datatype with a layout (`Vector3`, …).
    Datatype,
    /// A function type, or the file's table.
    Function,
    /// `LuaTuple<…>`
    Tuple,
}

impl Ty {
    /// roblox-ts's `isArrayType`: indexes are 0-based in TypeScript, 1-based in Luau.
    pub fn is_array(self) -> bool {
        matches!(self, Ty::Array | Ty::Blobs)
    }
}

/// The generator's identifiers: their hints, the types their declarations give them, and whether they are a
/// generated function's parameters (which roblox-ts treats as mutable; see `bind`).
#[derive(Debug, Default, Clone)]
pub struct Ids {
    pub hints: Vec<String>,
    pub types: Vec<Ty>,
}

impl Ids {
    pub fn new_id(&mut self, hint: &str, ty: Ty) -> Id {
        self.hints.push(hint.to_owned());
        self.types.push(ty);
        Id(self.hints.len() as u32 - 1)
    }

    pub fn hint(&self, id: Id) -> &str {
        &self.hints[id.0 as usize]
    }

    pub fn ty(&self, id: Id) -> Ty {
        self.types[id.0 as usize]
    }
}

/// A binary operator, as the TypeScript token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Lt,
    Le,
    Gt,
    Ge,
    /// `===`
    Eq,
    /// `!==`
    Ne,
    And,
}

impl Op {
    /// `getBinaryOperatorPrecedence`.
    fn precedence(self) -> u8 {
        match self {
            Op::And => 6,
            Op::Eq | Op::Ne => 10,
            Op::Lt | Op::Le | Op::Gt | Op::Ge => 11,
            Op::Add | Op::Sub => 13,
            Op::Mul | Op::Div | Op::Mod => 14,
        }
    }
}

/// An assignment's operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssignOp {
    Assign,
    Add,
}

/// How roblox-ts compiles a call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallKind {
    /// A function call: `f(a)`, `buffer.writeu8(...)`, `codec.s_X(v)`.
    Plain,
    /// A method of a Roblox object: `Enum.X:GetEnumItems()`, `cf:GetComponents()`.
    Method,
    /// A macro chosen by the receiver's type: `size`, `push`, `get`, `set`, `add`.
    Macro,
}

/// An object literal's property name.
#[derive(Clone, Debug, PartialEq)]
pub enum Key {
    /// `"name": v` (an identifier-like one renders `name = v`).
    Str(String),
    /// `10: v`
    Num(f64),
    /// `[-1]: v`
    Computed(E),
}

/// An arrow function.
#[derive(Clone, Debug, PartialEq)]
pub struct Arrow {
    pub params: Vec<Param>,
    pub body: Body,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    pub id: Id,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Body {
    Block(Vec<S>),
}

/// An expression.
#[derive(Clone, Debug, PartialEq)]
pub enum E {
    Id(Id),
    /// A plain identifier: a global (`buffer`, `math`, `typeIs`, `Enum`, …).
    Global(String),
    /// A value compiled elsewhere (a call site's argument): read like an identifier, or a literal as written.
    Ext(ExtValue),
    Num(f64),
    Str(String),
    Bool(bool),
    /// The identifier `undefined`.
    Undefined,
    Prop(Box<E>, String),
    Elem(Box<E>, Box<E>),
    Call(Box<E>, Vec<E>, CallKind),
    Binary(Box<E>, Op, Box<E>),
    Not(Box<E>),
    Neg(Box<E>),
    Paren(Box<E>),
    As(Box<E>, Ty),
    Cond(Box<E>, Box<E>, Box<E>),
    /// `` `head${e}text${e}text` ``
    Template(String, Vec<(E, String)>),
    Arrow(Box<Arrow>),
    Object(Vec<(Key, E)>),
    Array(Vec<E>),
    New(String, Vec<E>),
    /// Luau built directly ([`super::luau`]), passed through by the lowering, which names the placeholders in it
    /// ([`luau_id`]) as it meets them: the transition to direct emission (DESIGN §4, rounds A to B2).
    Luau(rbxts_luau::Expr),
}

/// A call site's value ([`E::Ext`]): its Luau, filled in when the site is compiled (the generator builds the site's
/// code before the arguments are compiled, see `crate::networking`), and what the TS generator saw of the node.
#[derive(Clone, Debug, PartialEq)]
pub struct ExtValue {
    pub value: std::rc::Rc<std::cell::RefCell<rbxts_plugin::LuauExpr>>,
    /// An identifier that names a parameter of the function around the site (`isParameterReference`).
    pub parameter: bool,
    /// The value of a literal (a string, a number, `true` or `false` as written), which the TS generator read as a
    /// literal (its checks fold it), not as an identifier; the Luau stays the literal as written.
    pub literal: Option<ExtLiteral>,
}

/// A call site's literal value ([`ExtValue::literal`]).
#[derive(Clone, Debug, PartialEq)]
pub enum ExtLiteral {
    Num(f64),
    Str(String),
    Bool(bool),
}

impl ExtValue {
    /// A value to be filled in, `nil` until then.
    pub fn new(parameter: bool, literal: Option<ExtLiteral>) -> ExtValue {
        ExtValue { value: std::rc::Rc::new(std::cell::RefCell::new(rbxts_luau::create::nil())), parameter, literal }
    }


    pub fn set(&self, value: rbxts_plugin::LuauExpr) {
        *self.value.borrow_mut() = value;
    }

    /// The literal as the generator's own literal node, for what reads its value.
    pub fn as_literal(&self) -> Option<E> {
        Some(match self.literal.as_ref()? {
            ExtLiteral::Num(n) => E::Num(*n),
            ExtLiteral::Str(s) => E::Str(s.clone()),
            ExtLiteral::Bool(b) => E::Bool(*b),
        })
    }
}

/// What a declaration binds.
#[derive(Clone, Debug, PartialEq)]
pub enum Bind {
    One(Id),
    /// `[a, b]`: a LuaTuple's values.
    Tuple(Vec<Id>),
}

/// A statement.
#[derive(Clone, Debug, PartialEq)]
pub enum S {
    Const(Bind, E),
    Let(Id, Option<E>),
    Expr(E),
    Assign(E, AssignOp, E),
    If(E, Vec<S>, Option<Box<Else>>),
    Block(Vec<S>),
    ForOf(Bind, E, Vec<S>),
    Return(Option<E>),
    /// A Luau statement built directly, passed through by the lowering ([`E::Luau`]).
    Luau(rbxts_luau::Stmt),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Else {
    If(S),
    Block(Vec<S>),
}

// --- precedence and the factory's parentheses ------------------------------------------------------------------

/// `getExpressionPrecedence`.
fn precedence(e: &E) -> u8 {
    match e {
        E::Binary(_, op, _) => op.precedence(),
        E::Cond(..) => 4,
        E::As(..) => 11,
        E::Not(_) | E::Neg(_) => 16,
        E::Call(..) => 18,
        E::Prop(..) | E::Elem(..) | E::New(..) => 19,
        _ => 20,
    }
}

/// `isLeftHandSideExpression` (an arrow function is not one).
fn is_left_hand_side(e: &E) -> bool {
    !matches!(e, E::Binary(..) | E::Cond(..) | E::As(..) | E::Not(_) | E::Neg(_) | E::Arrow(_))
}

/// `parenthesizeLeftSideOfAccess`.
pub fn paren_access(e: E) -> E {
    if is_left_hand_side(&e) { e } else { E::Paren(Box::new(e)) }
}

/// `binaryOperandNeedsParentheses` for the operators the generator uses (none is right-associative, and none
/// mixes with `??`).
fn binary_operand(op: Op, operand: E, left_side: bool, left: Option<&E>) -> E {
    if matches!(operand, E::Paren(_)) {
        return operand;
    }
    if !left_side && matches!(operand, E::Arrow(_)) {
        return E::Paren(Box::new(operand));
    }
    let operand_precedence = precedence(&operand);
    let op_precedence = op.precedence();
    let needs = match operand_precedence.cmp(&op_precedence) {
        std::cmp::Ordering::Less => true,
        std::cmp::Ordering::Greater => false,
        std::cmp::Ordering::Equal => {
            if left_side {
                false
            } else {
                if let E::Binary(_, inner, _) = &operand
                    && *inner == op
                {
                    if op == Op::Mul {
                        return operand;
                    }
                    if op == Op::Add {
                        let left_kind = left.map(plus_literal_kind).unwrap_or(LiteralKind::Unknown);
                        if left_kind != LiteralKind::Unknown && left_kind == plus_literal_kind(&operand) {
                            return operand;
                        }
                    }
                }
                // Every operand kind of equal precedence here is left-associative (a conditional's is lower).
                !matches!(operand, E::Cond(..) | E::Not(_) | E::Neg(_))
            }
        }
    };
    if needs { E::Paren(Box::new(operand)) } else { operand }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LiteralKind {
    Unknown,
    Num,
    Str,
}

/// `getLiteralKindOfBinaryPlusOperand`.
fn plus_literal_kind(e: &E) -> LiteralKind {
    match e {
        E::Num(_) => LiteralKind::Num,
        E::Str(_) => LiteralKind::Str,
        E::Binary(l, Op::Add, r) => {
            let left = plus_literal_kind(l);
            if left != LiteralKind::Unknown && left == plus_literal_kind(r) { left } else { LiteralKind::Unknown }
        }
        _ => LiteralKind::Unknown,
    }
}

// --- builders (`f.*` and `ts.factory.*`) ---------------------------------------------------------------------

pub fn id(id: Id) -> E {
    E::Id(id)
}

/// The identifier `id` in Luau built directly ([`E::Luau`]): a placeholder, named as the IR's identifiers are.
pub fn luau_id(id: Id) -> rbxts_luau::Expr {
    crate::luau_names::placeholder(id.0 as usize)
}

pub fn global(name: &str) -> E {
    E::Global(name.to_owned())
}

/// `f.number`: a negative value is `-` of its magnitude.
pub fn num(value: f64) -> E {
    if value < 0.0 { E::Neg(Box::new(E::Num(-value))) } else { E::Num(value) }
}

pub fn string(text: &str) -> E {
    E::Str(text.to_owned())
}

pub fn bool(value: bool) -> E {
    E::Bool(value)
}

/// `f.nil()`: the identifier `undefined`.
pub fn nil() -> E {
    E::Undefined
}

/// `createPropertyAccessExpression`.
pub fn prop(object: E, name: &str) -> E {
    E::Prop(Box::new(paren_access(object)), name.to_owned())
}

/// `createElementAccessExpression`.
pub fn elem(object: E, index: E) -> E {
    E::Elem(Box::new(paren_access(object)), Box::new(index))
}

/// `f.call`: a plain function call.
pub fn call(callee: E, args: Vec<E>) -> E {
    E::Call(Box::new(paren_access(callee)), args, CallKind::Plain)
}

/// `f.call(prop(object, name), args)` for a method of a Roblox object.
pub fn method(object: E, name: &str, args: Vec<E>) -> E {
    E::Call(Box::new(prop(object, name)), args, CallKind::Method)
}

/// `f.call(prop(object, name), args)` for one of roblox-ts's macros on `object`'s type.
pub fn macro_call(object: E, name: &str, args: Vec<E>) -> E {
    E::Call(Box::new(prop(object, name)), args, CallKind::Macro)
}

/// `createBinaryExpression`.
pub fn binary(left: E, op: Op, right: E) -> E {
    let left = binary_operand(op, left, true, None);
    let right = binary_operand(op, right, false, Some(&left));
    E::Binary(Box::new(left), op, Box::new(right))
}

/// `createPrefixUnaryExpression(ExclamationToken, …)`: its operand is parenthesized unless it is a unary or
/// left-hand-side expression.
pub fn not(operand: E) -> E {
    E::Not(Box::new(unary_operand(operand)))
}

/// `createPrefixUnaryExpression(MinusToken, …)`.
pub fn neg(operand: E) -> E {
    E::Neg(Box::new(unary_operand(operand)))
}

fn unary_operand(operand: E) -> E {
    if is_left_hand_side(&operand) || matches!(operand, E::Not(_) | E::Neg(_)) {
        operand
    } else {
        E::Paren(Box::new(operand))
    }
}

/// `createParenthesizedExpression`.
pub fn paren(e: E) -> E {
    E::Paren(Box::new(e))
}

/// `f.as`.
pub fn cast(e: E, ty: Ty) -> E {
    E::As(Box::new(e), ty)
}

/// `createConditionalExpression`.
pub fn cond(condition: E, when_true: E, when_false: E) -> E {
    let condition = if precedence(&condition) <= 4 { paren(condition) } else { condition };
    E::Cond(Box::new(condition), Box::new(when_true), Box::new(when_false))
}

pub fn arrow(params: Vec<Param>, body: Body) -> E {
    E::Arrow(Box::new(Arrow { params, body }))
}

pub fn param(id: Id) -> Param {
    Param { id }
}

pub fn object(fields: Vec<(Key, E)>) -> E {
    E::Object(fields)
}

pub fn array(elements: Vec<E>) -> E {
    E::Array(elements)
}

/// `createNewExpression(globalRef(name), …)`.
pub fn new(class: &str, args: Vec<E>) -> E {
    E::New(class.to_owned(), args)
}

/// `f.statement(f.binary(target, =, value))`.
pub fn assign(target: E, value: E) -> S {
    S::Assign(target, AssignOp::Assign, value)
}

/// `target += value`.
pub fn add_assign(target: E, value: E) -> S {
    S::Assign(target, AssignOp::Add, value)
}

/// `f.statement(e)`.
pub fn stmt(e: E) -> S {
    S::Expr(e)
}

/// `ifStatement(condition, then, otherwise)`.
pub fn if_stmt(condition: E, then: Vec<S>, otherwise: Option<Else>) -> S {
    S::If(condition, then, otherwise.map(Box::new))
}

// --- facts the generator reads off its own nodes -------------------------------------------------------------

impl E {
    /// `f.is.identifier`: an identifier node (a generated one, a global, `undefined`, or a call site's value).
    pub fn is_identifier(&self) -> bool {
        match self {
            E::Ext(value) => value.literal.is_none(),
            _ => matches!(self, E::Id(_) | E::Global(_) | E::Undefined),
        }
    }

    /// `isNilLiteral`: the identifier `undefined`.
    pub fn is_nil_literal(&self) -> bool {
        matches!(self, E::Undefined)
    }

    /// `isLiteral`: a string, number (not a negative one: that is a prefix expression), boolean or `undefined`.
    pub fn is_literal(&self) -> bool {
        match self {
            E::Ext(value) => value.literal.is_some(),
            _ => matches!(self, E::Str(_) | E::Num(_) | E::Bool(_) | E::Undefined),
        }
    }

    /// `literalNumber`: `7` or `-7`.
    pub fn literal_number(&self) -> Option<f64> {
        match self {
            E::Num(n) => Some(*n),
            E::Ext(ExtValue { literal: Some(ExtLiteral::Num(n)), .. }) => Some(*n),
            E::Neg(inner) => match &**inner {
                E::Num(n) => Some(-*n),
                _ => None,
            },
            _ => None,
        }
    }

    /// `skipCasts`.
    pub fn skip_casts(&self) -> &E {
        let mut e = self;
        while let E::As(inner, _) = e {
            e = inner;
        }
        e
    }

    /// `f.is.number(e) && e.text === "0"`.
    pub fn is_zero(&self) -> bool {
        matches!(self, E::Num(n) if *n == 0.0)
    }
}
