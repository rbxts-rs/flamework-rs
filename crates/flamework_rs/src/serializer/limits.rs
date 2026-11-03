//! Luau's limits for the serializer's generated functions (DESIGN §3): at most 200 locals and 255 registers per
//! function. `Gen::fitted` builds a function plainly, and again one field at a time (compact) when the plain form does
//! not fit with [`MARGIN`] to spare.
//!
//! - **Locals** are counted exactly on the generated Luau ([`Function::locals`]): parameters, the locals of every
//!   enclosing block and the loop variables, nested functions apart. Luau folds constant locals at optimisation level
//!   1+ with debug level 1, which only makes the count high, the safe side; generated code binds no constants.
//! - **Registers** are Luau's own answer: the function is compiled ([`Function::probe`]) by the compiler the load
//!   check uses (`rbxts_luau_check`, at its [`ROBLOX_LEVELS`]), behind what the file hoisted so far ahead of its
//!   functions (aliases, the `codec` table, then the helpers, guards and tables):
//!
//!   ```lua
//!   local __fwprobe = function(__fwpad1, …, __fwpadK, <parameters>) <body> end
//!   ```
//!
//!   The K padding parameters stand for K more live locals around the code. Luau's two messages ("Out of local
//!   registers", "Out of registers") are told apart; any other refusal is an internal error. A cheap bound
//!   ([`Function::surely_fits`]) skips the compile for nearly every function.
//!
//! The probe names each placeholder after its index (`__fwl<n>`), asking the compiler for no names; a call site's
//! arguments, placeholders never declared, are globals there (never `nil`, so Luau cannot fold the code reading them).

use std::rc::Rc;

use rbxts_luau::{AnyIdentifier, ElseBody, Expr, InterpolatedStringPart, Stmt, TableField, create};
use rbxts_luau_check::{CompileError, ROBLOX_LEVELS, Severity};

use super::Id;

/// Luau's most locals in one function (`kMaxLocalCount`).
pub(crate) const LUAU_MAX_LOCALS: u32 = 200;
/// Luau's most registers in one function.
pub(crate) const LUAU_MAX_REGISTERS: u32 = 255;
/// The locals and registers a plainly built function keeps to spare (DESIGN §3.2): Roblox runs a newer Luau than the
/// one compiled in, and a register allocator change must not turn a passing build into a failing game.
pub(crate) const MARGIN: u32 = 20;

/// The levels a probe compiles at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Levels {
    /// Every level of [`ROBLOX_LEVELS`].
    All,
    /// The levels whose refusal is an error.
    Errors,
}

/// What a probe found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Fit {
    Fits,
    /// More locals than Luau allows: `count` of the function's own (the padding not included).
    Locals { count: u32 },
    /// More registers than Luau allows: Luau's message.
    Registers { message: String },
}

impl Fit {
    pub fn fits(&self) -> bool {
        matches!(self, Fit::Fits)
    }
}

/// A generated function as Luau, for [`Function::probe`].
pub(crate) struct Function {
    /// What the file hoisted ahead of its functions (declarations, then tables; empty until a probe needs them).
    pub prelude: Rc<Vec<Stmt>>,
    pub parameters: Vec<AnyIdentifier>,
    pub body: Vec<Stmt>,
}

impl Function {
    /// The function of `parameters` with `body`.
    pub fn new(parameters: &[Id], body: Vec<Stmt>) -> Function {
        let parameters = parameters.iter().map(|p| crate::luau_names::declared(&p.e())).collect();
        Function { prelude: Rc::new(Vec::new()), parameters, body }
    }

    /// Its counts: the most locals live at once (the parameters, the enclosing blocks' locals and the loop
    /// variables), and a bound on the registers (the pre-count): at each statement, its live locals, the hidden
    /// registers of its enclosing loops, and twice its expression nodes plus 16 (calls, table constructors, Luau's
    /// inlining of the prelude's helpers). The bound is not a proof: the load check, which compiles every file, is the
    /// backstop.
    fn walk(&self) -> Walk {
        let mut walk = Walk::default();
        walk.block(&self.body, self.parameters.len() as u32, 0);
        walk
    }

    /// The most locals live at once.
    pub fn locals(&self) -> u32 {
        self.walk().locals
    }

    /// Whether the function surely fits with `padding` more locals live around it, without compiling it.
    pub fn surely_fits(&self, padding: u32) -> bool {
        let walk = self.walk();
        walk.locals + padding <= LUAU_MAX_LOCALS && walk.registers + padding <= LUAU_MAX_REGISTERS
    }

    /// The probe chunk: the prelude, then the function with `padding` parameters ahead of its own.
    pub fn probe_source(&self, padding: u32) -> String {
        let mut parameters: Vec<AnyIdentifier> =
            (1..=padding).map(|i| create::any_id(format!("__fwpad{i}"))).collect();
        parameters.extend(self.parameters.iter().cloned());
        let mut chunk: Vec<Stmt> = self.prelude.as_ref().clone();
        let probe = create::function(parameters, false, self.body.clone());
        chunk.push(create::local(create::any_id("__fwprobe"), Some(probe)));
        crate::luau_names::name_placeholders(&mut chunk, None, |index| format!("__fwl{index}"));
        rbxts_luau::render_ast(&chunk)
    }

    /// Whether the function fits with `padding` more locals live around it, at `levels`: the exact local count first,
    /// then the pre-count, then Luau's compiler. `Err` when Luau refuses the chunk for another reason (a bug in the
    /// generator).
    pub fn probe(&self, padding: u32, levels: Levels) -> Result<Fit, CompileError> {
        let walk = self.walk();
        if walk.locals + padding > LUAU_MAX_LOCALS {
            return Ok(Fit::Locals { count: walk.locals });
        }
        if walk.registers + padding <= LUAU_MAX_REGISTERS {
            return Ok(Fit::Fits);
        }
        let source = self.probe_source(padding);
        for &(optimization_level, debug_level, severity) in ROBLOX_LEVELS {
            if levels == Levels::Errors && severity != Severity::Error {
                continue;
            }
            if let Err(error) = rbxts_luau_check::compile(&source, optimization_level, debug_level) {
                return classify(error, walk.locals);
            }
        }
        Ok(Fit::Fits)
    }
}

/// Luau's refusal as a [`Fit`], or the error itself when it is not about a limit.
fn classify(error: CompileError, locals: u32) -> Result<Fit, CompileError> {
    if error.message.starts_with("Out of local registers") {
        Ok(Fit::Locals { count: locals })
    } else if error.message.starts_with("Out of registers") {
        Ok(Fit::Registers { message: error.message })
    } else {
        Err(error)
    }
}

/// Which build `fitted` keeps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Choice {
    Plain,
    Compact,
    /// Neither fits Luau's limits at the error levels, padded only by `outer`: why, about the compact form.
    Refused(String),
}

/// Whether the plain build is kept without building the compact one (DESIGN §3.2, step 1): it fits with
/// [`MARGIN`] + `outer` to spare at every level.
pub(crate) fn keeps_plain(plain: &Function, outer: u32) -> Result<bool, CompileError> {
    Ok(plain.probe(MARGIN + outer, Levels::All)?.fits())
}

/// The choice once the plain build did not fit with the margin (DESIGN §3.2, steps 2 to 4): the compact build when
/// it fits with the margin; else whichever fits without it at the error levels, plain first; else a refusal.
pub(crate) fn choose(plain: &Function, compact: &Function, outer: u32) -> Result<Choice, CompileError> {
    if compact.probe(MARGIN + outer, Levels::All)?.fits() {
        return Ok(Choice::Compact);
    }
    if plain.probe(outer, Levels::Errors)?.fits() {
        return Ok(Choice::Plain);
    }
    Ok(match compact.probe(outer, Levels::Errors)? {
        Fit::Fits => Choice::Compact,
        Fit::Locals { count } => {
            Choice::Refused(format!("needs {count} locals in one function (Luau allows {LUAU_MAX_LOCALS})"))
        }
        Fit::Registers { message } => Choice::Refused(format!(
            "needs more than {LUAU_MAX_REGISTERS} registers in one function (Luau: \"{message}\")"
        )),
    })
}

// --- counting --------------------------------------------------------------------------------------------------

/// The walk behind [`Function::locals`] and [`Function::surely_fits`].
#[derive(Default)]
struct Walk {
    locals: u32,
    registers: u32,
}

impl Walk {
    /// `live` locals and `hidden` loop registers around the block.
    fn block(&mut self, statements: &[Stmt], live: u32, hidden: u32) {
        self.locals = self.locals.max(live);
        let mut live = live;
        for statement in statements {
            live += self.statement(statement, live, hidden);
            self.locals = self.locals.max(live);
        }
    }

    /// A statement's own expressions, evaluated with `live` locals and `hidden` loop registers.
    fn evaluates(&mut self, nodes: u32, live: u32, hidden: u32) {
        self.registers = self.registers.max(live + hidden + 2 * nodes + 16);
    }

    /// Walks a statement; returns the locals it declares in its block.
    fn statement(&mut self, statement: &Stmt, live: u32, hidden: u32) -> u32 {
        match statement {
            Stmt::Assignment(node) => {
                self.evaluates(nodes_of(&node.left) + nodes_of(&node.right), live, hidden);
                0
            }
            Stmt::Call(call) => {
                self.evaluates(nodes(call), live, hidden);
                0
            }
            Stmt::Break | Stmt::Continue | Stmt::Comment(_) => 0,
            Stmt::Do(node) => {
                self.block(&node.statements, live, hidden);
                0
            }
            Stmt::While(node) => {
                self.evaluates(nodes(&node.condition), live, hidden);
                self.block(&node.statements, live, hidden);
                0
            }
            Stmt::Repeat(node) => {
                // The condition sees the body's locals.
                let mut inner = live;
                self.locals = self.locals.max(live);
                for statement in &node.statements {
                    inner += self.statement(statement, inner, hidden);
                    self.locals = self.locals.max(inner);
                }
                self.evaluates(nodes(&node.condition), inner, hidden);
                0
            }
            Stmt::If(node) => {
                self.if_statement(node, live, hidden);
                0
            }
            Stmt::NumericFor(node) => {
                let ends = nodes(&node.start) + nodes(&node.end) + node.step.as_ref().map_or(0, nodes);
                self.evaluates(ends, live, hidden);
                // limit, step and index, and a copy of the index when the body writes the variable
                self.block(&node.statements, live + 1, hidden + 3);
                0
            }
            Stmt::For(node) => {
                self.evaluates(nodes(&node.expression), live, hidden);
                // generator, state and index, and a register no local names when the loop has one variable
                let extra = if node.ids.len() == 1 { 4 } else { 3 };
                self.block(&node.statements, live + node.ids.len() as u32, hidden + extra);
                0
            }
            Stmt::FunctionDeclaration(node) => {
                self.evaluates(1 + nodes(&node.name), live, hidden);
                u32::from(node.localize)
            }
            Stmt::MethodDeclaration(node) => {
                self.evaluates(1 + nodes(&node.expression), live, hidden);
                0
            }
            Stmt::VariableDeclaration(node) => {
                self.evaluates(nodes_of(&node.right) + node.left.len() as u32, live, hidden);
                node.left.len() as u32
            }
            Stmt::Return(node) => {
                self.evaluates(nodes_of(&node.expressions), live, hidden);
                0
            }
        }
    }

    fn if_statement(&mut self, node: &rbxts_luau::IfStatement, live: u32, hidden: u32) {
        self.evaluates(nodes(&node.condition), live, hidden);
        self.block(&node.statements, live, hidden);
        match &node.else_body {
            ElseBody::If(inner) => self.if_statement(inner, live, hidden),
            ElseBody::Block(statements) => self.block(statements, live, hidden),
        }
    }
}

fn nodes_of(list: &[Expr]) -> u32 {
    list.iter().map(nodes).sum()
}

/// An expression's nodes; a function expression is one (its body is a frame of its own).
fn nodes(expr: &Expr) -> u32 {
    1 + match expr {
        Expr::Identifier(_)
        | Expr::TemporaryIdentifier(_)
        | Expr::None
        | Expr::Nil
        | Expr::False
        | Expr::True
        | Expr::Number(_)
        | Expr::String(_)
        | Expr::LateString(_)
        | Expr::VarArgs
        | Expr::Function(_) => 0,
        Expr::ComputedIndex(node) => nodes(&node.expression) + nodes(&node.index),
        Expr::PropertyAccess(node) => nodes(&node.expression),
        Expr::Call(node) => nodes(&node.expression) + nodes_of(&node.args),
        Expr::MethodCall(node) => nodes(&node.expression) + nodes_of(&node.args),
        Expr::Parenthesized(node) => nodes(&node.expression),
        Expr::Binary(node) => nodes(&node.left) + nodes(&node.right),
        Expr::Unary(node) => nodes(&node.expression),
        Expr::If(node) => nodes(&node.condition) + nodes(&node.expression) + nodes(&node.alternative),
        Expr::InterpolatedString(node) => node
            .parts
            .iter()
            .map(|part| match part {
                InterpolatedStringPart::Expression(expr) => nodes(expr),
                InterpolatedStringPart::Text(_) => 0,
            })
            .sum(),
        Expr::Array(members) | Expr::Set(members) => nodes_of(members),
        Expr::Map(fields) => fields.iter().map(|field| nodes(&field.index) + nodes(&field.value)).sum(),
        Expr::MixedTable(fields) => fields
            .iter()
            .map(|field| match field {
                TableField::Value(value) => nodes(value),
                TableField::Field(field) => nodes(&field.index) + nodes(&field.value),
            })
            .sum(),
    }
}

/// `holdsLocals(statements)`: whether the statements declare a local in their own block (a `do` block keeps it from
/// the code after them).
pub(crate) fn holds_locals(statements: &[Stmt]) -> bool {
    statements.iter().any(|s| matches!(s, Stmt::VariableDeclaration(_)))
}

// --- the decisions, recorded for the limit tests ---------------------------------------------------------------

/// The decisions `fitted` made, recorded for `tests/limits.rs` while [`record::start`] is on: each probe chunk, so the
/// test can compile them itself. Not part of the crate's API.
#[doc(hidden)]
pub mod record {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// One `fitted` decision.
    #[derive(Clone, Debug)]
    pub struct Decision {
        /// What the function is for (`the type 'Wide40'`), `None` for a call site.
        pub what: Option<String>,
        /// The live locals around the code (`outer`).
        pub outer: u32,
        /// The plain build's probe chunk with the margin and `outer` as padding, then with `outer` only.
        pub plain: [String; 2],
        /// The compact build's, when it was built.
        pub compact: Option<[String; 2]>,
        /// Whether the compact build was kept.
        pub kept_compact: bool,
        /// The plain build's exact local count.
        pub plain_locals: u32,
    }

    static ON: AtomicBool = AtomicBool::new(false);
    static DECISIONS: Mutex<Vec<Decision>> = Mutex::new(Vec::new());

    /// Starts recording (and clears what was recorded).
    pub fn start() {
        DECISIONS.lock().unwrap().clear();
        ON.store(true, Ordering::SeqCst);
    }

    /// Stops recording and returns the decisions.
    pub fn take() -> Vec<Decision> {
        ON.store(false, Ordering::SeqCst);
        std::mem::take(&mut *DECISIONS.lock().unwrap())
    }

    pub(crate) fn on() -> bool {
        ON.load(Ordering::Relaxed)
    }

    pub(crate) fn push(decision: Decision) {
        DECISIONS.lock().unwrap().push(decision);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(name: &str, args: Vec<Expr>) -> Expr {
        create::call(create::id(name), args)
    }

    /// `local x<i> = g()` for `i` in `0..count`.
    fn locals(count: u32) -> Vec<Stmt> {
        (0..count).map(|i| create::local(create::any_id(format!("x{i}")), Some(call("g", vec![])))).collect()
    }

    fn function(body: Vec<Stmt>) -> Function {
        Function { prelude: Rc::new(Vec::new()), parameters: vec![create::any_id("v")], body }
    }

    fn compiles_everywhere(source: &str) -> bool {
        ROBLOX_LEVELS.iter().all(|&(o, d, _)| match rbxts_luau_check::compile(source, o, d) {
            Ok(()) => true,
            Err(error) => {
                assert!(error.message.starts_with("Out of"), "{error:?}");
                false
            }
        })
    }

    /// k locals with padding p compile exactly when k + p ≤ 200 (the parameter `v` is one of the k), and the
    /// counter, the probe and Luau agree.
    #[test]
    fn probe_calibration_locals() {
        for k in [150u32, 179, 180, 181, 199, 200] {
            let f = function(locals(k - 1));
            assert_eq!(f.locals(), k);
            for p in [0u32, 1, 19, 20, 21, 50] {
                let fits = k + p <= LUAU_MAX_LOCALS;
                assert_eq!(compiles_everywhere(&f.probe_source(p)), fits, "k {k} p {p}");
                assert_eq!(f.probe(p, Levels::All).unwrap().fits(), fits, "k {k} p {p}");
            }
        }
    }

    /// A call over n values: Luau's registers (`function_stats`) plus the padding decide, and the probe agrees.
    #[test]
    fn probe_calibration_registers() {
        for n in [200u32, 230, 240] {
            let args = (0..n).map(|i| create::id(format!("a{i}"))).collect();
            let f = function(vec![create::return_stmt(call("f", args))]);
            let stats = rbxts_luau_check::function_stats(&f.probe_source(0), 1, 1).unwrap();
            let registers = u32::from(stats[0].max_stack_size);
            for p in [0u32, 1, 10, 20, 30, 60] {
                let fits = registers + p <= LUAU_MAX_REGISTERS;
                assert_eq!(f.probe(p, Levels::Errors).unwrap().fits(), fits, "n {n} p {p} registers {registers}");
            }
        }
    }

    #[test]
    fn counts_blocks_loops_and_parameters() {
        let inner =
            vec![create::local(create::any_id("a"), Some(call("g", vec![]))), create::local(create::any_id("b"), None)];
        let body = vec![
            create::local(create::any_id("x"), None),
            create::do_stmt(inner.clone()),
            create::for_in(vec![create::any_id("k"), create::any_id("w")], create::id("t"), inner.clone()),
            create::numeric_for(create::any_id("i"), create::number(1.0), create::id("n"), None, inner),
            create::local(create::any_id("y"), Some(create::function(vec![], false, locals(150)))),
        ];
        // v, x, then k, w, a, b inside the generic loop
        assert_eq!(function(body).locals(), 6);
    }

    #[test]
    fn other_refusals_are_internal_errors() {
        let mut wide = function(vec![Stmt::Break]);
        // a register-heavy body forces the compile: `break` outside a loop is no limit
        wide.body.insert(0, Stmt::Call(call("f", (0..300).map(|i| create::id(format!("a{i}"))).collect())));
        let error = wide.probe(0, Levels::Errors).unwrap_err();
        assert!(!error.message.starts_with("Out of"), "{error:?}");
    }

    /// A body past the limits in both builds is refused, naming the compact build's exact local count.
    #[test]
    fn refuses_what_fits_in_neither_build() {
        let plain = function(locals(230));
        let compact = function(locals(210));
        assert!(!keeps_plain(&plain, 0).unwrap());
        assert_eq!(
            choose(&plain, &compact, 0).unwrap(),
            Choice::Refused("needs 211 locals in one function (Luau allows 200)".to_owned())
        );
        // registers: Luau's message
        let wide = |n: u32| {
            let args = (0..n).map(|i| create::id(format!("a{i}"))).collect();
            function(vec![create::return_stmt(call("f", args))])
        };
        let Choice::Refused(reason) = choose(&wide(300), &wide(280), 0).unwrap() else { panic!("not refused") };
        let expected = "needs more than 255 registers in one function (Luau: \"Out of registers";
        assert!(reason.starts_with(expected), "{reason}");
    }

    #[test]
    fn keeps_what_fits() {
        // plain fits with the margin
        assert!(keeps_plain(&function(locals(150)), 0).unwrap());
        assert!(!keeps_plain(&function(locals(150)), 40).unwrap());
        // compact fits with the margin
        assert_eq!(choose(&function(locals(190)), &function(locals(100)), 0).unwrap(), Choice::Compact);
        // neither with the margin: plain fits without it
        assert_eq!(choose(&function(locals(190)), &function(locals(195)), 0).unwrap(), Choice::Plain);
        // compact only without it
        assert_eq!(choose(&function(locals(205)), &function(locals(190)), 0).unwrap(), Choice::Compact);
    }
}
