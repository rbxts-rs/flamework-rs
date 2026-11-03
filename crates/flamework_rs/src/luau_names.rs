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
//! Guards (`crate::guards`) use it, and so does the serializer (`crate::serializer`), whose file-wide registry names
//! its output hook by hook: a name, once given, is kept ([`Names::assign_with`]), so that a helper one hook hoisted is
//! read by the same name in the next hook's code.

use std::collections::HashMap;

use rbxts_luau::{AnyIdentifier, ElseBody, Expr, InterpolatedStringPart, Stmt, TableField, create};
use rbxts_plugin::Cx;

/// What a placeholder's name starts with: no Luau or TypeScript identifier contains it, so a placeholder can never
/// be mistaken for a real name (and the renderer would refuse one that was left unnamed).
const MARK: char = '\u{1}';

/// The placeholders of one piece of generated code (a guard, or a file's serializer code), by index: the base name each
/// one asks for, and the name it was given.
#[derive(Clone, Debug, Default)]
pub(crate) struct Names {
    bases: Vec<String>,
    named: Vec<Option<String>>,
}

impl Names {
    /// A new local, named after `base` (as given: [`crate::guards::names::local_name`] it first when it comes from the
    /// project).
    pub(crate) fn fresh(&mut self, base: impl Into<String>) -> Expr {
        placeholder(self.fresh_index(base))
    }

    /// [`Names::fresh`], as the placeholder's index.
    pub(crate) fn fresh_index(&mut self, base: impl Into<String>) -> usize {
        self.bases.push(base.into());
        self.named.push(None);
        self.bases.len() - 1
    }

    /// Hands the placeholders in `expr` over to another registry: each becomes the placeholder `adopt` gives for its
    /// base name (asked once per placeholder, in index order).
    pub(crate) fn rebase(self, expr: &mut Expr, mut adopt: impl FnMut(&str) -> usize) {
        let adopted: Vec<String> = self.bases.iter().map(|base| placeholder_name(adopt(base))).collect();
        name_placeholders(&mut [], Some(expr), |index| adopted[index].clone());
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
    pub(crate) fn assign(&mut self, cx: &Cx<'_>, statements: &mut [Stmt], expr: Option<&mut Expr>) {
        self.assign_with(statements, expr, |base| unique_name(cx, base));
    }

    /// [`Names::assign`] with the names given by `unique` (a base name in, the local's name out), in render order. A
    /// placeholder named before keeps its name.
    pub(crate) fn assign_with(
        &mut self,
        statements: &mut [Stmt],
        expr: Option<&mut Expr>,
        mut unique: impl FnMut(&str) -> String,
    ) {
        let (bases, named) = (&self.bases, &mut self.named);
        name_placeholders(statements, expr, |index| named[index].get_or_insert_with(|| unique(&bases[index])).clone());
    }
}

/// The placeholder of index `index` ([`Names::fresh`]'s).
pub(crate) fn placeholder(index: usize) -> Expr {
    create::id(placeholder_name(index))
}

fn placeholder_name(index: usize) -> String {
    format!("{MARK}{index}")
}

/// Replaces every placeholder in `statements` and then `expr`, used or declared, by the name `name_of` gives for its
/// index, in render order. `name_of` is asked at each occurrence: keeping a name is up to it.
pub(crate) fn name_placeholders(
    statements: &mut [Stmt],
    expr: Option<&mut Expr>,
    mut name_of: impl FnMut(usize) -> String,
) {
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

    /// A name given once is kept: a later piece of code reading the same placeholder (a hoisted helper, read by the
    /// next hook's code) gets the same name, without asking for a new one.
    #[test]
    fn assigned_names_persist() {
        let mut names = Names::default();
        let helper = names.fresh("vsize");
        let mut first = vec![create::local(declared(&helper), Some(create::number(1.0)))];
        let mut asked = Vec::new();
        names.assign_with(&mut first, None, |base| {
            asked.push(base.to_owned());
            format!("{base}_1")
        });
        let value = names.fresh("value");
        let mut second = vec![create::local(declared(&value), Some(create::call(helper.clone(), Vec::new())))];
        names.assign_with(&mut second, None, |base| {
            asked.push(base.to_owned());
            base.to_owned()
        });
        assert_eq!(rbxts_luau::render_ast(&first), "local vsize_1 = 1\n");
        assert_eq!(rbxts_luau::render_ast(&second), "local value = vsize_1()\n");
        assert_eq!(asked, ["vsize", "value"]);
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

    #[test]
    fn rebase_moves_used_and_declared_placeholders() {
        let mut names = Names::default();
        let depth = names.fresh("depth");
        let value = names.fresh("value");
        let body = vec![create::local(declared(&depth), Some(value.clone())), create::return_stmt(depth)];
        let mut function = create::function(vec![declared(&value)], false, body);
        let mut adopted = Vec::new();
        names.rebase(&mut function, |base| {
            adopted.push(base.to_owned());
            40 + adopted.len()
        });
        assert_eq!(adopted, ["depth", "value"]);
        name_placeholders(&mut [], Some(&mut function), |index| format!("n{index}"));
        let text = rbxts_luau::render_expr(&function, &rbxts_luau::LateStrings::default());
        assert_eq!(text, "function(n42)\n\tlocal n41 = n42\n\treturn n41\nend");
    }
}
