//! Where a call site's packing goes (`transformNetworkingCall.ts`: `emitWithStatements`, `hoistsCleanly`,
//! `evaluatedBefore`; `util/luauLimits.ts`: `LocalBudget`, `localsAround`, `enclosingScope`, `enclosingStatement`,
//! `declaredLocals`, `sureLocalsOf`). AST facts only, read off the nodes as written.

use std::collections::HashMap;

use rbxts_plugin::ts::{Kind, Node, P};

/// `FUNCTION_LOCAL_BUDGET`: Flamework locals a function may take ahead of its statements.
pub(crate) const FUNCTION_LOCAL_BUDGET: u32 = 64;
/// `COMPACT_LOCALS`.
pub(crate) const COMPACT_LOCALS: u32 = 150;

/// The scope a call site's code lands in (`enclosingScope`): the function around it (a class static block counts),
/// or the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Scope {
    Function(P<Node>),
    File,
}

/// `enclosingScope(node)`.
pub(crate) fn enclosing_scope(node: P<Node>) -> Scope {
    let mut current = node.parent();
    while let Some(parent) = current {
        if tsrs_ast::is_function_like(Some(parent)) || parent.kind() == Kind::ClassStaticBlockDeclaration {
            return Scope::Function(parent);
        }
        current = parent.parent();
    }
    Scope::File
}

/// `enclosingStatement(node)`: the statement `node` is in (itself when it is one).
pub(crate) fn enclosing_statement(node: P<Node>) -> Option<P<Node>> {
    let mut current = Some(node);
    while let Some(node) = current {
        if tsrs_ast::is_statement(node) {
            return Some(node);
        }
        current = node.parent();
    }
    None
}

/// Flamework's locals in each scope (`LocalBudget`), for the networking call sites' claims. The file's budget is the
/// serializer's (`claim_file` asks it), shared with the file's hoisted helpers.
#[derive(Debug, Default)]
pub(crate) struct Budget {
    used: HashMap<P<Node>, u32>,
    /// Claims by the statement, as written, that their locals went ahead of.
    placed: HashMap<P<Node>, u32>,
}

impl Budget {
    /// `claim(scope, count, statement)` for a function scope.
    pub(crate) fn claim_function(&mut self, scope: P<Node>, count: u32, statement: P<Node>) -> bool {
        let used = self.used.get(&scope).copied().unwrap_or(0);
        if used + count > FUNCTION_LOCAL_BUDGET {
            return false;
        }
        self.used.insert(scope, used + count);
        *self.placed.entry(statement).or_default() += count;
        true
    }

    /// Records a claim the file's budget granted ahead of `statement`.
    pub(crate) fn place(&mut self, count: u32, statement: P<Node>) {
        *self.placed.entry(statement).or_default() += count;
    }

    /// `ahead(statement)`.
    pub(crate) fn ahead(&self, statement: P<Node>) -> u32 {
        self.placed.get(&statement).copied().unwrap_or(0)
    }
}

/// `localsAround(node, budget)`: the locals live where code put ahead of `node`'s statement runs, as far as the
/// source tells, Flamework's claims ahead of the statements of each block around it included.
pub(crate) fn locals_around(node: P<Node>, budget: &Budget) -> u32 {
    let mut child = node;
    let mut current = node.parent();
    let mut count = 0;
    while let Some(parent) = current {
        if matches!(
            parent.kind(),
            Kind::Block | Kind::SourceFile | Kind::ModuleBlock | Kind::CaseClause | Kind::DefaultClause
        ) {
            let file = parent.kind() == Kind::SourceFile;
            for &statement in parent.statements() {
                count += budget.ahead(statement);
                if statement == child {
                    break;
                }
                count += if file { sure_locals_of(statement) } else { declared_names(statement) };
            }
            if file {
                return count;
            }
        }
        if matches!(parent.kind(), Kind::ForOfStatement | Kind::ForInStatement | Kind::ForStatement)
            && child == parent.statement()
            && let Some(initializer) = parent.initializer()
            && initializer.kind() == Kind::VariableDeclarationList
        {
            for &declaration in initializer.as_variable_declaration_list().declarations().nodes().iter() {
                count += binding_names(declaration.name().expect("a declaration has a name"));
            }
        }
        if tsrs_ast::is_function_like(Some(parent)) {
            for &parameter in parent.parameters() {
                let name = parameter.name().expect("a parameter has a name");
                // `this` is a type annotation, not a parameter.
                if !(name.kind() == Kind::Identifier && name.text() == "this") {
                    count += binding_names(name);
                }
            }
            if matches!(
                parent.kind(),
                Kind::MethodDeclaration | Kind::GetAccessor | Kind::SetAccessor | Kind::Constructor
            ) {
                count += 1;
            }
            return count;
        }
        if parent.kind() == Kind::ClassStaticBlockDeclaration {
            return count;
        }
        child = parent;
        current = parent.parent();
    }
    count
}

/// `declaredNames(statement)`: the locals a statement declares in its block.
pub(crate) fn declared_names(statement: P<Node>) -> u32 {
    match statement.kind() {
        Kind::VariableStatement => statement
            .as_variable_statement()
            .declaration_list()
            .as_variable_declaration_list()
            .declarations()
            .nodes()
            .iter()
            .map(|d| binding_names(d.name().expect("a declaration has a name")))
            .sum(),
        Kind::FunctionDeclaration | Kind::ClassDeclaration => u32::from(statement.name().is_some()),
        _ => 0,
    }
}

/// `bindingNames(name)`.
fn binding_names(name: P<Node>) -> u32 {
    match name.kind() {
        Kind::Identifier => 1,
        Kind::ObjectBindingPattern | Kind::ArrayBindingPattern => name
            .elements()
            .iter()
            .filter(|e| e.kind() != Kind::OmittedExpression)
            .map(|e| binding_names(e.name().expect("a binding element has a name")))
            .sum(),
        _ => 0,
    }
}

/// `sureLocalsOf(statement)`: the main-chunk locals a top-level statement is sure to keep.
fn sure_locals_of(statement: P<Node>) -> u32 {
    let flags = statement.modifier_flags();
    if flags.intersects(tsrs_ast::ModifierFlags::Ambient) {
        return 0;
    }
    let exported = flags.intersects(tsrs_ast::ModifierFlags::Export);
    match statement.kind() {
        Kind::VariableStatement => {
            let list = statement.as_variable_statement().declaration_list();
            // An exported `let` lives in the module's exports table rather than a local.
            if exported && !list.flags().intersects(tsrs_ast::NodeFlags::Const) {
                return 0;
            }
            list.as_variable_declaration_list()
                .declarations()
                .nodes()
                .iter()
                .filter(|d| d.initializer().is_some_and(is_sure_value))
                .map(|d| binding_names(d.name().expect("a declaration has a name")))
                .sum()
        }
        Kind::FunctionDeclaration => u32::from(statement.name().is_some() && statement.body().is_some()),
        Kind::ClassDeclaration => u32::from(statement.name().is_some()),
        Kind::EnumDeclaration => u32::from(!flags.intersects(tsrs_ast::ModifierFlags::Const)),
        _ => 0,
    }
}

/// Libraries whose calls on constants Luau folds (`FOLDABLE_LIBRARIES`).
const FOLDABLE_LIBRARIES: [&str; 9] = ["math", "bit32", "string", "vector", "buffer", "type", "typeof", "tostring", "tonumber"];

/// `isSureValue`: an initializer Luau keeps in a register.
fn is_sure_value(expression: P<Node>) -> bool {
    let mut expression = expression;
    while matches!(
        expression.kind(),
        Kind::ParenthesizedExpression
            | Kind::AsExpression
            | Kind::NonNullExpression
            | Kind::TypeAssertionExpression
            | Kind::SatisfiesExpression
    ) {
        expression = expression.expression().expect("a wrapper has an expression");
    }
    if expression.kind() == Kind::CallExpression
        && expression.arguments().iter().all(|&a| tsrs_ast::is_literal_expression(a))
        && let Some(callee) = expression.expression()
    {
        let library = if callee.kind() == Kind::PropertyAccessExpression { callee.expression() } else { Some(callee) };
        if let Some(library) = library
            && library.kind() == Kind::Identifier
            && FOLDABLE_LIBRARIES.contains(&library.text())
        {
            return false;
        }
    }
    matches!(
        expression.kind(),
        Kind::CallExpression
            | Kind::NewExpression
            | Kind::ObjectLiteralExpression
            | Kind::ArrayLiteralExpression
            | Kind::ArrowFunction
            | Kind::FunctionExpression
            | Kind::ClassExpression
            | Kind::TaggedTemplateExpression
            | Kind::AwaitExpression
    )
}

/// `hoistsCleanly(node, hoisted)`: evaluating `hoisted` ahead of the statement that holds `node` is the same as
/// evaluating it where `node` is.
pub(crate) fn hoists_cleanly(node: P<Node>, hoisted: &[P<Node>]) -> bool {
    let mut ahead: Vec<P<Node>> = Vec::new();
    let mut child = node;
    let mut current = node.parent();
    while let Some(parent) = current {
        if tsrs_ast::is_statement(parent) {
            break;
        }
        if tsrs_ast::is_function_like(Some(parent)) {
            return false;
        }
        let Some(before) = evaluated_before(parent, child) else { return false };
        ahead.extend(before);
        child = parent;
        current = parent.parent();
    }
    let Some(statement) = current else { return false };
    if is_repeated_in(statement, child) {
        return false;
    }
    if !ahead.iter().all(|&e| is_pure(e)) {
        return false;
    }
    ahead.iter().all(|&e| is_constant(e)) || hoisted.iter().all(|&e| is_pure(e))
}

/// `evaluatedBefore(parent, child)`: what `parent` evaluates before `child` when `child` is evaluated once and
/// unconditionally as part of it; `None` when it is not, or the position is not one this understands.
fn evaluated_before(parent: P<Node>, child: P<Node>) -> Option<Vec<P<Node>>> {
    match parent.kind() {
        Kind::ParenthesizedExpression
        | Kind::AsExpression
        | Kind::TypeAssertionExpression
        | Kind::NonNullExpression
        | Kind::SatisfiesExpression
        | Kind::AwaitExpression
        | Kind::YieldExpression
        | Kind::VoidExpression
        | Kind::TypeOfExpression
        | Kind::DeleteExpression
        | Kind::PrefixUnaryExpression
        | Kind::SpreadElement
        | Kind::SpreadAssignment
        | Kind::TemplateSpan => Some(Vec::new()),
        Kind::PropertyAccessExpression => (parent.expression() == Some(child)).then(Vec::new),
        Kind::ElementAccessExpression => {
            let object = parent.expression().expect("an element access has an object");
            Some(if object == child { Vec::new() } else { vec![object] })
        }
        Kind::CallExpression | Kind::NewExpression => {
            let callee = parent.expression().expect("a call has a callee");
            if callee == child {
                return Some(Vec::new());
            }
            let args = parent.arguments();
            let index = args.iter().position(|&a| a == child)?;
            let mut before = vec![callee];
            before.extend_from_slice(&args[..index]);
            Some(before)
        }
        Kind::BinaryExpression => {
            let binary = parent.as_binary_expression();
            let left = binary.left;
            if child == left {
                return Some(Vec::new());
            }
            let operator = binary.operator_token().kind();
            if matches!(operator, Kind::AmpersandAmpersandToken | Kind::BarBarToken | Kind::QuestionQuestionToken) {
                return None;
            }
            if !tsrs_ast::is_assignment_operator(operator) {
                return Some(vec![left]);
            }
            // An assignment target is not a value: what runs ahead of the right-hand side is the object (and
            // index) it is stored into.
            match left.kind() {
                Kind::Identifier => Some(Vec::new()),
                Kind::PropertyAccessExpression => Some(vec![left.expression().expect("an object")]),
                Kind::ElementAccessExpression => Some(vec![
                    left.expression().expect("an object"),
                    left.as_element_access_expression().argument_expression,
                ]),
                _ => None,
            }
        }
        Kind::ConditionalExpression => (parent.as_conditional_expression().condition == child).then(Vec::new),
        Kind::ArrayLiteralExpression => {
            let elements = parent.elements();
            let index = elements.iter().position(|&e| e == child)?;
            Some(elements[..index].to_vec())
        }
        Kind::PropertyAssignment => {
            if parent.initializer() != Some(child) {
                return None;
            }
            let name = parent.name().expect("a property has a name");
            Some(if name.kind() == Kind::ComputedPropertyName {
                vec![name.as_computed_property_name().expression]
            } else {
                Vec::new()
            })
        }
        Kind::ObjectLiteralExpression => {
            let properties = parent.properties();
            let index = properties.iter().position(|&p| p == child)?;
            let mut before = Vec::new();
            for &property in &properties[..index] {
                match property.kind() {
                    Kind::PropertyAssignment => {
                        let name = property.name().expect("a property has a name");
                        if name.kind() == Kind::ComputedPropertyName {
                            before.push(name.as_computed_property_name().expression);
                        }
                        before.extend(property.initializer());
                    }
                    Kind::ShorthandPropertyAssignment => before.extend(property.name()),
                    Kind::SpreadAssignment => before.extend(property.expression()),
                    _ => {}
                }
            }
            Some(before)
        }
        Kind::TemplateExpression => {
            let spans = parent.as_template_expression().template_spans.nodes();
            let index = spans.iter().position(|&s| s == child)?;
            Some(spans[..index].iter().filter_map(|s| s.expression()).collect())
        }
        Kind::VariableDeclaration => (parent.initializer() == Some(child)).then(Vec::new),
        Kind::VariableDeclarationList => {
            let declarations = parent.as_variable_declaration_list().declarations().nodes();
            let index = declarations.iter().position(|&d| d == child)?;
            Some(declarations[..index].iter().filter_map(|d| d.initializer()).collect())
        }
        _ => None,
    }
}

/// `isRepeatedIn(statement, child)`: `statement` evaluates `child` on every pass of a loop.
fn is_repeated_in(statement: P<Node>, child: P<Node>) -> bool {
    match statement.kind() {
        Kind::WhileStatement | Kind::DoStatement => statement.expression() == Some(child),
        Kind::ForStatement => {
            let for_statement = statement.as_for_statement();
            for_statement.condition == Some(child) || for_statement.incrementor == Some(child)
        }
        _ => false,
    }
}

/// `isPure`: an expression that evaluates without touching state: a literal, a closure, or a plain read.
pub(crate) fn is_pure(node: P<Node>) -> bool {
    if is_constant(node) || matches!(node.kind(), Kind::Identifier | Kind::ThisKeyword) {
        return true;
    }
    match node.kind() {
        Kind::PropertyAccessExpression => node.expression().is_some_and(is_pure),
        Kind::ElementAccessExpression => {
            node.expression().is_some_and(is_pure) && is_pure(node.as_element_access_expression().argument_expression)
        }
        Kind::ParenthesizedExpression
        | Kind::AsExpression
        | Kind::TypeAssertionExpression
        | Kind::NonNullExpression
        | Kind::SatisfiesExpression
        | Kind::TypeOfExpression => node.expression().is_some_and(is_pure),
        Kind::PrefixUnaryExpression => {
            let unary = node.as_prefix_unary_expression();
            !matches!(unary.operator, Kind::PlusPlusToken | Kind::MinusMinusToken) && is_pure(unary.operand)
        }
        Kind::BinaryExpression => {
            let binary = node.as_binary_expression();
            !tsrs_ast::is_assignment_operator(binary.operator_token().kind())
                && is_pure(binary.left)
                && is_pure(binary.right())
        }
        Kind::TemplateExpression => node
            .as_template_expression()
            .template_spans
            .nodes()
            .iter()
            .all(|s| s.expression().is_some_and(is_pure)),
        Kind::ArrayLiteralExpression => node.elements().iter().all(|&e| is_pure(e)),
        Kind::SpreadElement => node.expression().is_some_and(is_pure),
        Kind::ObjectLiteralExpression => node.properties().iter().all(|&property| match property.kind() {
            Kind::PropertyAssignment => {
                let name = property.name().expect("a property has a name");
                (name.kind() != Kind::ComputedPropertyName || is_pure(name.as_computed_property_name().expression))
                    && property.initializer().is_some_and(is_pure)
            }
            Kind::SpreadAssignment => property.expression().is_some_and(is_pure),
            _ => true,
        }),
        _ => false,
    }
}

/// `isConstant`: an expression that reads nothing: a literal, a keyword, or a closure only being created.
pub(crate) fn is_constant(node: P<Node>) -> bool {
    tsrs_ast::is_literal_expression(node)
        || matches!(
            node.kind(),
            Kind::TrueKeyword | Kind::FalseKeyword | Kind::NullKeyword | Kind::ArrowFunction | Kind::FunctionExpression
        )
}
