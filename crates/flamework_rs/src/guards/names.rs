//! Names of the locals a guard declares, and of the project's names a generated local is made from.
//!
//! The TS builder makes a guard's locals with `factory.createUniqueName(base, Optimistic)`: `base` when the file uses no
//! such name yet, else `base_1`, `base_2`, … (`makeUniqueName`). Here they are placeholders, named once the guard is
//! complete ([`crate::luau_names`]).

use rbxts_luau::Expr;
use rbxts_plugin::Cx;
use rbxts_plugin::ts::P;
use tsrs_ast::{Node, SymbolFlags};

use crate::luau_names::walk_expr;

/// Names no generated local can take (`identifierName.ts` `UNUSABLE`): JavaScript's reserved words and those of strict
/// mode and modules, `arguments` and `eval`, Luau's keywords, and the Luau globals roblox-ts reserves.
const UNUSABLE: [&str; 82] = [
    "break", "case", "catch", "class", "const", "continue", "debugger", "default", "delete", "do", "else", "enum",
    "export", "extends", "false", "finally", "for", "function", "if", "import", "in", "instanceof", "new", "null",
    "return", "super", "switch", "this", "throw", "true", "try", "typeof", "var", "void", "while", "with", "await",
    "implements", "interface", "let", "package", "private", "protected", "public", "static", "yield", "arguments",
    "eval", "and", "elseif", "end", "local", "nil", "not", "or", "repeat", "then", "until", "_G", "TS", "assert",
    "bit32", "coroutine", "error", "exports", "game", "getmetatable", "ipairs", "math", "next", "pairs", "pcall",
    "require", "script", "select", "self", "setmetatable", "string", "table", "tostring", "type", "unpack",
];

/// `localName` (`identifierName.ts`): a name from the project (a field, a type) made into one a generated local can
/// have. Characters JavaScript's `\W` matches become `_`; `v_` goes in front of a name that is still no Luau
/// identifier, is [`UNUSABLE`], or names a global value or namespace (a local would hide it from the code after it).
pub(crate) fn local_name(cx: &mut Cx<'_>, hint: &str) -> String {
    let mut name = String::with_capacity(hint.len());
    for c in hint.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            name.push(c);
        } else {
            // `W` matches per UTF-16 unit: a character outside the BMP becomes two underscores.
            name.extend(std::iter::repeat_n('_', c.len_utf16()));
        }
    }
    let is_luau_identifier = name
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if !is_luau_identifier || UNUSABLE.contains(&name.as_str()) {
        return format!("v_{name}");
    }
    let global =
        cx.checker().resolve_name_exported(&name, None::<P<Node>>, SymbolFlags::Value | SymbolFlags::Namespace, false);
    if global.is_some() { format!("v_{name}") } else { name }
}

/// A symbol's name as TypeScript 5 spells it: tsrs marks internal names (`\u{7f}type`) where TS uses a `__` prefix
/// (`__type`, `__object`, `__index`).
pub(crate) fn ts_symbol_name(name: &str) -> std::borrow::Cow<'_, str> {
    match name.strip_prefix('\u{7f}') {
        Some(rest) => std::borrow::Cow::Owned(format!("__{rest}")),
        None => std::borrow::Cow::Borrowed(name),
    }
}

/// The globals of the TS builder's guard that a guard's Luau names, in render order (the serializer's
/// `checkGlobalsIn`, which visits the TS guard's identifiers): `typeIs` (which roblox-ts compiled to `type(…)`),
/// `error`, `Enum`, and `Promise` (the runtime library's `TS.Promise`).
pub(crate) fn ts_globals_in(expr: &Expr) -> Vec<&'static str> {
    let mut found = Vec::new();
    let mut copy = expr.clone();
    walk_expr(
        &mut copy,
        &mut |expr: &mut Expr| match expr {
            Expr::Call(call) if matches!(&call.expression, Expr::Identifier(id) if id.name == "type") => {
                found.push("typeIs");
            }
            Expr::Identifier(id) if id.name == "error" => found.push("error"),
            Expr::Identifier(id) if id.name == "Enum" => found.push("Enum"),
            Expr::PropertyAccess(access)
                if access.name == "Promise" && matches!(&access.expression, Expr::Identifier(id) if id.name == "TS") =>
            {
                found.push("Promise");
            }
            _ => {}
        },
        &mut |_| {},
    );
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbol_names_follow_typescript_5() {
        assert_eq!(ts_symbol_name("\u{7f}type"), "__type");
        assert_eq!(ts_symbol_name("Tree"), "Tree");
    }
}
