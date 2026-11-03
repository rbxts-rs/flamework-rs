//! Where a call site's packing goes (`transformNetworkingCall.ts`: `emitWithStatements`, `hoistsCleanly`,
//! `evaluatedBefore`; `util/luauLimits.ts`: `enclosingScope`, `enclosingStatement`), and what the compiler does not
//! count of the locals live there. AST facts only, read off the nodes as written.
//!
//! The locals live where a site's statements land come from the compiler (`Cx::live_before_statement`), exact for
//! the Luau emitted so far. What it cannot see when the hook runs (its documented gaps) is added here
//! ([`unseen_locals`], [`MAIN_CHUNK_LATE`]) or avoided ([`in_class_expression_static_block`]).

use rbxts_plugin::ts::{Kind, Node, P};

/// Locals one send may leave live ahead of its statement, past which it is packed in a function of its own.
pub(crate) const SEND_LEAK: u32 = 8;
/// Locals the sends of one function may leave live ahead of their statements (`FUNCTION_LOCAL_BUDGET`).
pub(crate) const FUNCTION_LEAK: u32 = 64;
/// The same for the file's main chunk (`LOCAL_BUDGET`).
pub(crate) const FILE_LEAK: u32 = 16;
/// Live locals, the leaked ones included, past which a send is not packed ahead of its statement: Luau's 200, less
/// the margin of 20 kept everywhere ([`crate::serializer::limits::MARGIN`]).
pub(crate) const AHEAD_LOCALS: u32 = 180;
/// Main-chunk locals the compiler's count cannot include when a site in the main chunk is built: what the site's own
/// build hoists ahead of the root statement once it is done (at most the file's [`crate::serializer::LOCAL_BUDGET`]
/// slots, `codec`, three global aliases, the varint helpers and `zeros`: 24), and what the end of the file decides
/// (a later plugin import, `local TS`).
pub(crate) const MAIN_CHUNK_LATE: u32 = 32;

/// The scope a call site's code lands in (`enclosingScope`): the function around it (a class static block counts),
/// or the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Scope {
    Function(P<Node>),
    File,
}

impl Scope {
    /// How many locals the sends of this scope may leave live ahead of their statements.
    pub(crate) fn leak_cap(self) -> u32 {
        match self {
            Scope::Function(_) => FUNCTION_LEAK,
            Scope::File => FILE_LEAK,
        }
    }
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

/// Locals live at `node` that `Cx::live_before_statement` does not count: one for each `for … of` around it, in the
/// same function, whose head assigns to existing targets (`for ([a = 1] of list)`, `for (obj[key()] of list)`):
/// roblox-ts lowers such a head after the body, so its temporary is not seen yet (rbxts-rs documents it as one local
/// and one register too few). A head that declares its variables (`for (const x of list)`) is counted in full.
pub(crate) fn unseen_locals(node: P<Node>) -> u32 {
    let mut count = 0;
    let mut child = node;
    let mut current = node.parent();
    while let Some(parent) = current {
        if tsrs_ast::is_function_like(Some(parent)) {
            break;
        }
        if parent.kind() == Kind::ForOfStatement
            && parent.statement() == child
            && parent.initializer().is_some_and(|head| head.kind() != Kind::VariableDeclarationList)
        {
            count += 1;
        }
        child = parent;
        current = parent.parent();
    }
    count
}

/// `node` is in a class expression's static block (not in a function inside it): there the compiler's count misses
/// the prerequisites the statement around the class emitted before it, so a site's code goes in a function of its own.
pub(crate) fn in_class_expression_static_block(node: P<Node>) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        if tsrs_ast::is_function_like(Some(parent)) {
            return false;
        }
        if parent.kind() == Kind::ClassStaticBlockDeclaration {
            return parent.parent().is_some_and(|class| class.kind() == Kind::ClassExpression);
        }
        current = parent.parent();
    }
    false
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
