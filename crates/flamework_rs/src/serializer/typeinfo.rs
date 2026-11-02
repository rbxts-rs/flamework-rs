//! What the serializer reads off types and symbols, from the TS transformer's guard file and its small helpers:
//! `getLiteral`, `simplifyUnion`, `extractTypes`, `enumMemberOf` / `enumMemberOrigins`, `isInstanceType`,
//! `isConditionalType` (`buildGuardFromType.ts`), `getPropertyKey` / `keyAccess` / `keyName` / `keySegment`
//! (`propertyKey.ts`) and `localName` (`identifierName.ts`).
//!
//! The helpers F3 (guards) asks the same question with are F3's (`crate::guards::types`, `crate::guards::names`):
//! `resolveName`, `aliasSymbol`, `extractTypes`, `isInstanceType`, `getPropertyKey` and `localName`. `getLiteral` and
//! `simplifyUnion` stay here: the serializer needs their values as its own expressions ([`ir::E`]), which it ranks,
//! prints in layout texts and compares, where F3's are Luau.
//!
//! Every property listing the serializer's layout depends on comes from [`Cx::properties_of_type`]: TypeScript 7.1's
//! order (a class's or interface's own members, then the inherited ones, each in declaration order).

use rbxts_plugin::Cx;
use rbxts_plugin::ts::{Kind as SyntaxKind, P, Symbol, Type};
use tsrs_ast::{CheckFlags, SymbolFlags};
use tsrs_checker::{LiteralValue, ObjectFlags, TypeFlags};

use super::ir::{self, E};

/// A property's key in a Luau table: a string, or a number.
#[derive(Clone, Debug, PartialEq)]
pub enum TableKey {
    Str(String),
    Num(f64),
}

pub(crate) use crate::guards::names::local_name;
pub(crate) use crate::guards::types::{
    alias_symbol, escaped_property_name, extract_types, global as resolve_global, is_instance_type,
    type_of_named_property,
};

/// `type.aliasTypeArguments` (`None` when there are none).
pub fn alias_type_arguments(ty: P<Type>) -> Option<&'static [P<Type>]> {
    ty.alias().map(|a| a.type_arguments()).filter(|args| !args.is_empty())
}

/// `type.getProperty(name)`.
pub fn get_property(cx: &mut Cx<'_>, ty: P<Type>, name: &str) -> Option<P<Symbol>> {
    cx.checker().get_property_of_type_exported(ty, name)
}

/// A literal type's value.
pub fn literal_value(ty: P<Type>) -> Option<LiteralValue> {
    if !ty.flags().intersects(TypeFlags::StringLiteral | TypeFlags::NumberLiteral | TypeFlags::BooleanLiteral | TypeFlags::BigIntLiteral) {
        return None;
    }
    if !matches!(ty.data_tag(), tsrs_checker::TypeDataTag::Literal) {
        return None;
    }
    ty.as_literal_type().value()
}

/// `type.isStringLiteral() ? type.value : undefined`.
pub fn string_literal(ty: P<Type>) -> Option<&'static str> {
    if !ty.flags().intersects(TypeFlags::StringLiteral) {
        return None;
    }
    match literal_value(ty) {
        Some(LiteralValue::String(s)) => Some(s),
        _ => None,
    }
}

/// `type.isNumberLiteral() ? type.value : undefined`.
pub fn number_literal(ty: P<Type>) -> Option<f64> {
    if !ty.flags().intersects(TypeFlags::NumberLiteral) {
        return None;
    }
    match literal_value(ty) {
        Some(LiteralValue::Number(n)) => Some(n.0),
        _ => None,
    }
}

/// `IntrinsicType.intrinsicName`.
pub fn intrinsic_name(ty: P<Type>) -> Option<&'static str> {
    if !ty.flags().intersects(TypeFlags::Intrinsic) || !matches!(ty.data_tag(), tsrs_checker::TypeDataTag::Intrinsic) {
        return None;
    }
    Some(ty.as_intrinsic_type().intrinsic_name())
}

/// `getLiteral(type, withoutEnums)`: the value(s) of a literal type as expressions.
pub fn get_literal(cx: &mut Cx<'_>, ty: P<Type>, without_enums: bool) -> Option<Vec<E>> {
    if let Some(s) = string_literal(ty) {
        return Some(vec![ir::string(s)]);
    }
    if let Some(n) = number_literal(ty) {
        return Some(vec![ir::num(n)]);
    }
    // `trueType` / `regularTrueType` and the false ones: TS7's four boolean literal types.
    if ty.flags().intersects(TypeFlags::BooleanLiteral)
        && let Some(crate::macro_type::Literal::Bool(value)) = crate::macro_type::literal_of(ty)
    {
        return Some(vec![ir::bool(value)]);
    }

    if ty.flags().intersects(TypeFlags::Enum) {
        let symbol = ty.symbol()?;
        let declarations = symbol.declarations();
        if declarations.len() != 1 || declarations[0].kind() != SyntaxKind::EnumDeclaration {
            return None;
        }
        let members: Vec<_> = declarations[0].as_enum_declaration().members.nodes().to_vec();
        let mut values = Vec::new();
        for member in members {
            match cx.checker().get_constant_value(member)? {
                LiteralValue::String(s) => values.push(ir::string(s)),
                LiteralValue::Number(n) => values.push(ir::num(n.0)),
                _ => return None,
            }
        }
        return Some(values);
    }

    if !without_enums {
        let symbol = ty.symbol()?;
        let enum_type = resolve_global(cx, "Enum", SymbolFlags::Type);
        if let Some(parent) = symbol.parent()
            && let Some(grand) = parent.parent()
            && enum_type.is_some()
            && Some(cx.checker().get_merged_symbol(grand)) == enum_type
        {
            return Some(vec![enum_item(parent.name(), symbol.name())]);
        }
    }
    None
}

/// `f.field(f.field("Enum", group), item)`: `Enum["Material"]["Plastic"]`.
pub fn enum_item(group: &str, item: &str) -> E {
    ir::elem(ir::elem(ir::global("Enum"), ir::string(group)), ir::string(item))
}

/// A TypeScript enum member a literal is: the merged enum and the member's index.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnumMemberOrigin {
    pub enum_symbol: P<Symbol>,
    pub index: usize,
}

/// `enumMemberOrigins(type, count)`.
pub fn enum_member_origins(cx: &mut Cx<'_>, ty: P<Type>, count: usize) -> Vec<Option<EnumMemberOrigin>> {
    if ty.flags().intersects(TypeFlags::StringLiteral | TypeFlags::NumberLiteral)
        && let Some(member) = enum_member_of(cx, ty)
    {
        return vec![Some(member)];
    }
    if ty.flags().intersects(TypeFlags::Enum)
        && let Some(symbol) = ty.symbol()
    {
        let enum_symbol = cx.checker().get_merged_symbol(symbol);
        return (0..count).map(|index| Some(EnumMemberOrigin { enum_symbol, index })).collect();
    }
    vec![None; count]
}

/// `enumMemberOf(type)`.
pub fn enum_member_of(cx: &mut Cx<'_>, ty: P<Type>) -> Option<EnumMemberOrigin> {
    if !ty.flags().intersects(TypeFlags::EnumLiteral | TypeFlags::Enum) {
        return None;
    }
    let symbol = ty.symbol()?;
    if !symbol.flags().intersects(SymbolFlags::EnumMember) {
        return None;
    }
    let parent = symbol.parent()?;
    let enum_symbol = cx.checker().get_merged_symbol(parent);
    Some(EnumMemberOrigin { enum_symbol, index: enum_member_index(enum_symbol, symbol) })
}

fn enum_member_index(enum_symbol: P<Symbol>, member: P<Symbol>) -> usize {
    let mut index = 0;
    for &declaration in enum_symbol.declarations() {
        if declaration.kind() != SyntaxKind::EnumDeclaration {
            continue;
        }
        for &declared in declaration.as_enum_declaration().members.nodes() {
            if Some(declared) == member.value_declaration() {
                return index;
            }
            index += 1;
        }
    }
    index
}

/// `simplifyUnion(type)`.
pub struct Simplified {
    pub enums: Vec<String>,
    pub types: Vec<P<Type>>,
    pub literals: Vec<E>,
    pub literal_origins: Vec<Option<EnumMemberOrigin>>,
}

pub fn simplify_union(cx: &mut Cx<'_>, ty: P<Type>) -> Simplified {
    let enum_type = resolve_global(cx, "Enum", SymbolFlags::Type);
    if let Some(alias) = alias_symbol(ty)
        && let Some(parent) = alias.parent()
        && enum_type.is_some()
        && Some(cx.checker().get_merged_symbol(parent)) == enum_type
    {
        return Simplified { enums: vec![alias.name().to_owned()], types: vec![], literals: vec![], literal_origins: vec![] };
    }

    let current: Vec<P<Type>> = ty.types().to_vec();
    let mut possible_enums: Vec<(P<Symbol>, Vec<P<Type>>)> = Vec::new();
    let mut out = Simplified { enums: vec![], types: vec![], literals: vec![], literal_origins: vec![] };
    let is_boolean = current.iter().filter(|t| t.flags().intersects(TypeFlags::BooleanLiteral)).count() == 2;
    if is_boolean {
        out.types.push(cx.checker().get_boolean_type());
    }

    for &member in &current {
        if member.flags().intersects(TypeFlags::ESSymbolLike) {
            continue;
        }
        if is_boolean && member.flags().intersects(TypeFlags::BooleanLiteral) {
            continue;
        }
        if let Some(literal) = get_literal(cx, member, true) {
            let origins = enum_member_origins(cx, member, literal.len());
            out.literals.extend(literal);
            out.literal_origins.extend(origins);
            continue;
        }
        let Some(symbol) = member.symbol() else {
            out.types.push(member);
            continue;
        };
        let Some(enum_kind) = symbol.parent() else {
            out.types.push(member);
            continue;
        };
        let is_roblox_enum = match enum_kind.parent() {
            Some(grand) => enum_type.is_some() && Some(cx.checker().get_merged_symbol(grand)) == enum_type,
            None => false,
        };
        if !is_roblox_enum {
            out.types.push(member);
            continue;
        }
        if enum_kind.exports().and_then(|e| e.lookup(symbol.name())) == Some(symbol) {
            match possible_enums.iter_mut().find(|(k, _)| *k == enum_kind) {
                Some((_, set)) => {
                    if !set.contains(&member) {
                        set.push(member);
                    }
                }
                None => possible_enums.push((enum_kind, vec![member])),
            }
        }
    }

    for (symbol, set) in possible_enums {
        let mut items = 0;
        if let Some(exports) = symbol.exports() {
            exports.for_each(|_, member| {
                if member.flags().intersects(SymbolFlags::Interface) {
                    items += 1;
                }
            });
        }
        if set.len() == items {
            out.enums.push(symbol.name().to_owned());
        } else {
            for member in set {
                let name = member.symbol().map(|s| s.name()).unwrap_or_default();
                out.literals.push(enum_item(symbol.name(), name));
                out.literal_origins.push(None);
            }
        }
    }
    out
}

/// `isConditionalType`.
pub fn is_conditional_type(ty: P<Type>) -> bool {
    ty.flags().intersects(TypeFlags::Conditional)
}

/// `getObjectFlags(type) & Reference`.
pub fn is_reference(ty: P<Type>) -> bool {
    ty.flags().intersects(TypeFlags::Object) && ty.object_flags().intersects(ObjectFlags::Reference)
}

/// `ts.getCheckFlags(symbol)`.
pub fn check_flags(symbol: P<Symbol>) -> CheckFlags {
    if symbol.flags().intersects(SymbolFlags::Transient) { symbol.check_flags() } else { CheckFlags::None }
}

// --- property keys ------------------------------------------------------------------------------------------------

/// `getPropertyKey` (F3's [`crate::guards::types::property_key`]).
pub fn get_property_key(cx: &mut Cx<'_>, property: P<Symbol>) -> TableKey {
    match crate::guards::types::property_key(cx, property) {
        crate::guards::types::TableKey::Number(n) => TableKey::Num(n),
        crate::guards::types::TableKey::String(s) => TableKey::Str(s),
    }
}

/// `/^[A-Za-z_$][\w$]*$/`.
pub fn is_identifier_like(key: &str) -> bool {
    let mut chars = key.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_' || c == '$')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

/// `numberKey`: `10`, `-1`, `1e999` for an infinity.
fn number_key(value: f64) -> E {
    ir::num(value)
}

/// `keyAccess(object, key)`.
pub fn key_access(object: E, key: &TableKey) -> E {
    match key {
        TableKey::Num(n) => ir::elem(object, number_key(*n)),
        TableKey::Str(s) if is_identifier_like(s) => ir::prop(object, s),
        TableKey::Str(s) => ir::elem(object, ir::string(s)),
    }
}

/// `keyName(key)`.
pub fn key_name(key: &TableKey) -> ir::Key {
    match key {
        TableKey::Str(s) => ir::Key::Str(s.clone()),
        TableKey::Num(n) if *n >= 0.0 || n.is_nan() => ir::Key::Num(*n),
        TableKey::Num(n) => ir::Key::Computed(number_key(*n)),
    }
}

/// `keySegment(key)`.
pub fn key_segment(key: &TableKey) -> String {
    match key {
        TableKey::Num(n) => format!("[{}]", crate::js::number_to_string(*n)),
        TableKey::Str(s) if is_identifier_like(s) => format!(".{s}"),
        TableKey::Str(s) => format!("[{}]", json_string(s)),
    }
}

/// `JSON.stringify(text)`.
pub fn json_string(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// `checker.getTypeArguments(type)` of a type reference; nothing for any other type (tsrs panics on one).
pub fn type_arguments(cx: &mut Cx<'_>, ty: P<Type>) -> Vec<P<Type>> {
    if ty.try_as_type_reference().is_none() || ty.target().is_none() {
        return Vec::new();
    }
    cx.checker().get_type_arguments_exported(ty).to_vec()
}

/// TS5's `__type` / `__object`: the names of anonymous type and object literal symbols (`\u{7f}type` and
/// `\u{7f}object` in TS7).
pub fn is_anonymous_name(name: &str) -> bool {
    matches!(name, "__type" | "__object" | "\u{7f}type" | "\u{7f}object")
}
