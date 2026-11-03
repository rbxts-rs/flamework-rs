//! Placeholder locals for generated Luau, named once the code is complete.
//!
//! Generated code is built with placeholder identifiers ([`Names::fresh`]), and [`Names::assign`] names them when the
//! code is handed out, walking the Luau in render order and asking the compiler for each name
//! ([`rbxts_plugin::Cx::is_usable_local_name`], which records the names it approves for the whole file): `base` when
//! it is free, else `base_1`, `base_2`, … ([`unique_name`], TypeScript's `makeUniqueName` with `Optimistic`). So a
//! local made deep inside the code (a recursive guard's `depth`) is named where it renders first, and code that is
//! built and dropped consumes no names. [`Names::substitute`] replaces placeholders by other expressions first (the
//! `guards.Name` fields of a guard holding more locals than Luau allows).
//!
//! Guards (`crate::guards`) use it today; the serializer's direct emission is to (DESIGN §2.1).

use std::collections::HashMap;

use rbxts_luau::{AnyIdentifier, ElseBody, Expr, InterpolatedStringPart, Stmt, TableField, create};
use rbxts_plugin::Cx;

/// What a placeholder's name starts with: no Luau or TypeScript identifier contains it, so a placeholder can never
/// be mistaken for a real name (and the renderer would refuse one that was left unnamed).
const MARK: char = '\u{1}';

/// The placeholders of one piece of generated code (a guard), by index: the base name each one asks for.
#[derive(Debug, Default)]
pub(crate) struct Names {
    bases: Vec<String>,
}

impl Names {
    /// A new local, named after `base` (as given: [`crate::guards::names::local_name`] it first when it comes from the
    /// project).
    pub(crate) fn fresh(&mut self, base: impl Into<String>) -> Expr {
        let index = self.bases.len();
        self.bases.push(base.into());
        create::id(format!("{MARK}{index}"))
    }

    /// The base name a placeholder asks for (`local.text` of the TS identifier), if `expr` is one.
    pub(crate) fn base_of(&self, expr: &Expr) -> Option<&str> {
        placeholder_index(expr).map(|index| self.bases[index].as_str())
    }

    /// Replaces each placeholder in `replacements` (by index) with its expression, everywhere in `statements`.
    pub(crate) fn substitute(statements: &mut [Stmt], replacements: &HashMap<usize, Expr>) {
        let mut visit = |expr: &mut Expr| {
            if let Some(replacement) = placeholder_index(expr).and_then(|index| replacements.get(&index)) {
                *expr = replacement.clone();
            }
        };
        for statement in statements {
            walk_stmt(statement, &mut visit, &mut |_| {});
        }
    }

    /// Names every placeholder in `statements` and then `expr`, in render order (see the module docs).
    pub(crate) fn assign(&self, cx: &Cx<'_>, statements: &mut [Stmt], expr: Option<&mut Expr>) {
        self.assign_with(statements, expr, |base| unique_name(cx, base));
    }

    /// [`Names::assign`] with the names given by `unique` (a base name in, the local's name out), in render order.
    pub(crate) fn assign_with(
        &self,
        statements: &mut [Stmt],
        expr: Option<&mut Expr>,
        mut unique: impl FnMut(&str) -> String,
    ) {
        let mut assigned: Vec<Option<String>> = vec![None; self.bases.len()];
        let mut name_of = |index: usize| -> String {
            assigned[index].get_or_insert_with(|| unique(&self.bases[index])).clone()
        };
        let named = std::cell::RefCell::new(&mut name_of);
        let mut on_expr = |expr: &mut Expr| {
            if let Some(index) = placeholder_index(expr) {
                let name = (named.borrow_mut())(index);
                *expr = create::id(name);
            }
        };
        let mut on_id = |id: &mut AnyIdentifier| {
            if let AnyIdentifier::Identifier(identifier) = id
                && let Some(index) = index_of_name(&identifier.name)
            {
                identifier.name = (named.borrow_mut())(index);
            }
        };
        for statement in statements {
            walk_stmt(statement, &mut on_expr, &mut on_id);
        }
        if let Some(expr) = expr {
            walk_expr(expr, &mut on_expr, &mut on_id);
        }
    }
}

/// The placeholder index of `expr`, if it is a placeholder identifier.
pub(crate) fn placeholder_index(expr: &Expr) -> Option<usize> {
    match expr {
        Expr::Identifier(identifier) => index_of_name(&identifier.name),
        _ => None,
    }
}

fn index_of_name(name: &str) -> Option<usize> {
    name.strip_prefix(MARK)?.parse().ok()
}

/// A placeholder as a declared identifier (`local x`, a parameter, a loop variable).
pub(crate) fn declared(expr: &Expr) -> AnyIdentifier {
    match expr {
        Expr::Identifier(identifier) => AnyIdentifier::Identifier(identifier.clone()),
        other => panic!("declared a local that is not an identifier: {other:?}"),
    }
}

/// TypeScript's `makeUniqueName` with the `Optimistic` flag, each name approved by the compiler
/// (`is_usable_local_name`): `base` itself when it is free, else the first free `base_1`, `base_2`, … (`base1`, …
/// when `base` ends in `_`).
pub(crate) fn unique_name(cx: &Cx<'_>, base: &str) -> String {
    if cx.is_usable_local_name(base) {
        return base.to_owned();
    }
    let prefix = if base.ends_with('_') { base.to_owned() } else { format!("{base}_") };
    (1u32..)
        .map(|i| format!("{prefix}{i}"))
        .find(|name| cx.is_usable_local_name(name))
        .expect("a free name")
}

// --- the walker: every expression and declared identifier, in render order ---

pub(crate) fn walk_stmt(
    statement: &mut Stmt,
    on_expr: &mut impl FnMut(&mut Expr),
    on_id: &mut impl FnMut(&mut AnyIdentifier),
) {
    match statement {
        Stmt::Assignment(assignment) => {
            for expr in &mut assignment.left {
                walk_expr(expr, on_expr, on_id);
            }
            for expr in &mut assignment.right {
                walk_expr(expr, on_expr, on_id);
            }
        }
        Stmt::Break | Stmt::Continue | Stmt::Comment(_) => {}
        Stmt::Call(expr) => walk_expr(expr, on_expr, on_id),
        Stmt::Do(block) => walk_block(&mut block.statements, on_expr, on_id),
        Stmt::While(node) => {
            walk_expr(&mut node.condition, on_expr, on_id);
            walk_block(&mut node.statements, on_expr, on_id);
        }
        Stmt::Repeat(node) => {
            walk_block(&mut node.statements, on_expr, on_id);
            walk_expr(&mut node.condition, on_expr, on_id);
        }
        Stmt::If(node) => walk_if(node, on_expr, on_id),
        Stmt::NumericFor(node) => {
            on_id(&mut node.id);
            walk_expr(&mut node.start, on_expr, on_id);
            walk_expr(&mut node.end, on_expr, on_id);
            if let Some(step) = &mut node.step {
                walk_expr(step, on_expr, on_id);
            }
            walk_block(&mut node.statements, on_expr, on_id);
        }
        Stmt::For(node) => {
            for id in &mut node.ids {
                on_id(id);
            }
            walk_expr(&mut node.expression, on_expr, on_id);
            walk_block(&mut node.statements, on_expr, on_id);
        }
        Stmt::FunctionDeclaration(node) => {
            walk_expr(&mut node.name, on_expr, on_id);
            for id in &mut node.parameters {
                on_id(id);
            }
            walk_block(&mut node.statements, on_expr, on_id);
        }
        Stmt::MethodDeclaration(node) => {
            walk_expr(&mut node.expression, on_expr, on_id);
            for id in &mut node.parameters {
                on_id(id);
            }
            walk_block(&mut node.statements, on_expr, on_id);
        }
        Stmt::VariableDeclaration(node) => {
            for id in &mut node.left {
                on_id(id);
            }
            for expr in &mut node.right {
                walk_expr(expr, on_expr, on_id);
            }
        }
        Stmt::Return(node) => {
            for expr in &mut node.expressions {
                walk_expr(expr, on_expr, on_id);
            }
        }
    }
}

fn walk_if(
    node: &mut rbxts_luau::IfStatement,
    on_expr: &mut impl FnMut(&mut Expr),
    on_id: &mut impl FnMut(&mut AnyIdentifier),
) {
    walk_expr(&mut node.condition, on_expr, on_id);
    walk_block(&mut node.statements, on_expr, on_id);
    match &mut node.else_body {
        ElseBody::If(inner) => walk_if(inner, on_expr, on_id),
        ElseBody::Block(statements) => walk_block(statements, on_expr, on_id),
    }
}

fn walk_block(statements: &mut [Stmt], on_expr: &mut impl FnMut(&mut Expr), on_id: &mut impl FnMut(&mut AnyIdentifier)) {
    for statement in statements {
        walk_stmt(statement, on_expr, on_id);
    }
}

pub(crate) fn walk_expr(
    expr: &mut Expr,
    on_expr: &mut impl FnMut(&mut Expr),
    on_id: &mut impl FnMut(&mut AnyIdentifier),
) {
    on_expr(expr);
    match expr {
        Expr::Identifier(_)
        | Expr::TemporaryIdentifier(_)
        | Expr::None
        | Expr::Nil
        | Expr::False
        | Expr::True
        | Expr::Number(_)
        | Expr::String(_)
        | Expr::LateString(_)
        | Expr::VarArgs => {}
        Expr::ComputedIndex(node) => {
            walk_expr(&mut node.expression, on_expr, on_id);
            walk_expr(&mut node.index, on_expr, on_id);
        }
        Expr::PropertyAccess(node) => walk_expr(&mut node.expression, on_expr, on_id),
        Expr::Call(node) => {
            walk_expr(&mut node.expression, on_expr, on_id);
            for arg in &mut node.args {
                walk_expr(arg, on_expr, on_id);
            }
        }
        Expr::MethodCall(node) => {
            walk_expr(&mut node.expression, on_expr, on_id);
            for arg in &mut node.args {
                walk_expr(arg, on_expr, on_id);
            }
        }
        Expr::Parenthesized(node) => walk_expr(&mut node.expression, on_expr, on_id),
        Expr::Function(node) => {
            for id in &mut node.parameters {
                on_id(id);
            }
            walk_block(&mut node.statements, on_expr, on_id);
        }
        Expr::Binary(node) => {
            walk_expr(&mut node.left, on_expr, on_id);
            walk_expr(&mut node.right, on_expr, on_id);
        }
        Expr::Unary(node) => walk_expr(&mut node.expression, on_expr, on_id),
        Expr::If(node) => {
            walk_expr(&mut node.condition, on_expr, on_id);
            walk_expr(&mut node.expression, on_expr, on_id);
            walk_expr(&mut node.alternative, on_expr, on_id);
        }
        Expr::InterpolatedString(node) => {
            for part in &mut node.parts {
                if let InterpolatedStringPart::Expression(expr) = part {
                    walk_expr(expr, on_expr, on_id);
                }
            }
        }
        Expr::Array(members) | Expr::Set(members) => {
            for member in members {
                walk_expr(member, on_expr, on_id);
            }
        }
        Expr::Map(fields) => {
            for field in fields {
                walk_expr(&mut field.index, on_expr, on_id);
                walk_expr(&mut field.value, on_expr, on_id);
            }
        }
        Expr::MixedTable(fields) => {
            for field in fields {
                match field {
                    TableField::Value(value) => walk_expr(value, on_expr, on_id),
                    TableField::Field(field) => {
                        walk_expr(&mut field.index, on_expr, on_id);
                        walk_expr(&mut field.value, on_expr, on_id);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_round_trip() {
        let mut names = Names::default();
        let a = names.fresh("value");
        let b = names.fresh("Tree");
        assert_eq!(placeholder_index(&a), Some(0));
        assert_eq!(names.base_of(&b), Some("Tree"));
        assert_eq!(placeholder_index(&create::id("value")), None);
    }

    #[test]
    fn substitute_replaces_every_use() {
        let mut names = Names::default();
        let a = names.fresh("Tree");
        let mut statements = vec![create::assignment(
            a.clone(),
            rbxts_luau::AssignmentOperator::Assign,
            create::call(create::property(create::id("t"), "array"), vec![a.clone()]),
        )];
        let replacement = create::property(create::id("guards"), "Tree");
        Names::substitute(&mut statements, &HashMap::from([(0, replacement)]));
        let text = rbxts_luau::render_ast(&statements);
        assert_eq!(text, "guards.Tree = t.array(guards.Tree)\n");
    }
}
