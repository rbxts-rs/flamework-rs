//! The type questions the guard builder asks (`buildGuardFromType.ts`'s helpers, `getInstanceTypeFromType.ts`,
//! `propertyKey.ts`), on this instance's checker. The serializer (F4) asks most of them too.

use rbxts_luau::{Expr, create};
use rbxts_plugin::ts::{Kind, Node, P, Symbol, Type};
use rbxts_plugin::{Cx, synthesized_string};
use tsrs_checker::{LiteralValue, ObjectFlags, TypeFlags};
use tsrs_ast::SymbolFlags;

/// `checker.resolveName(name, undefined, meaning, false)`: a global by name.
pub(crate) fn global(cx: &mut Cx<'_>, name: &str, meaning: SymbolFlags) -> Option<P<Symbol>> {
    cx.checker().resolve_name_exported(name, None::<P<Node>>, meaning, false)
}

/// A global type symbol (`resolveName(name, undefined, SymbolFlags.Type, false)`).
pub(crate) fn global_type(cx: &mut Cx<'_>, name: &str) -> Option<P<Symbol>> {
    global(cx, name, SymbolFlags::Type)
}

/// `getDeclarationOfType`: the first declaration of the type's symbol.
pub(crate) fn declaration_of_type(ty: P<Type>) -> Option<P<Node>> {
    ty.symbol()?.declarations().first().copied()
}

/// The type's alias symbol (`type.aliasSymbol`).
pub(crate) fn alias_symbol(ty: P<Type>) -> Option<P<Symbol>> {
    ty.alias().and_then(|alias| alias.symbol())
}

/// `isInstanceType`: the type has the `_nominal_Instance` brand Roblox's instance types carry.
pub fn is_instance_type(cx: &mut Cx<'_>, ty: P<Type>) -> bool {
    cx.checker().get_property_of_type_exported(ty, "_nominal_Instance").is_some()
}

/// The type of one of `ty`'s own properties, or nothing for a property Flamework does not reflect: one keyed by a
/// unique symbol or a private name ([`is_internal_property`]). Guards, attribute guards, serializers, `Many` objects
/// and `getFields` leave such a property out.
pub(crate) fn type_of_named_property(cx: &mut Cx<'_>, ty: P<Type>, property: P<Symbol>) -> Option<P<Type>> {
    if is_internal_property(cx, property) {
        return None;
    }
    cx.checker().get_type_of_property_of_type_exported(ty, property.name())
}

/// Whether a property is keyed by a unique symbol or a private name, which Flamework does not reflect
/// ([`type_of_named_property`]). tsrs spells both with its internal-name prefix (U+007F: `\u{7f}@sym@12`,
/// `\u{7f}#3@#x`), but a user's string key may start with U+007F too (`"\u007fab"`), so the prefix only rules the
/// rest out: the property is internal when a declaration names it with a private identifier, or its name type (a
/// computed key's, a mapped or copied property's) is a unique symbol. A user name with leading underscores (`__x`)
/// is a plain name and is reflected.
pub(crate) fn is_internal_property(cx: &mut Cx<'_>, property: P<Symbol>) -> bool {
    if !property.name().starts_with(tsrs_ast::INTERNAL_SYMBOL_NAME_PREFIX) {
        return false;
    }
    let private = property
        .declarations()
        .iter()
        .any(|declaration| declaration.name().is_some_and(|name| name.kind() == Kind::PrivateIdentifier));
    private
        || cx
            .checker()
            .get_name_type_of_symbol(property)
            .is_some_and(|name_type| name_type.flags().intersects(TypeFlags::UniqueESSymbol))
}

/// `property.name` as TS5 gives it: an internal name escaped as TS5 spells it (`__@sym@12`; a private name is `#x`,
/// as `symbolName` gives it), any other name as it is. A symbol's id differs between the compilers, so two keys whose
/// texts agree up to the id (two unique symbols of one name, a user key `"__@zeta@1"` against a real `[zeta]`) may
/// order otherwise than in TS5, whose own order there follows its global symbol counter (README, "Differences").
pub(crate) fn ts5_property_name(cx: &mut Cx<'_>, property: P<Symbol>) -> std::borrow::Cow<'static, str> {
    if is_internal_property(cx, property) {
        std::borrow::Cow::Owned(tsrs_ast::escape_internal_symbol_name(tsrs_ast::symbol_name(property)))
    } else {
        std::borrow::Cow::Borrowed(property.name())
    }
}

/// `isObjectType`
pub(crate) fn is_object(ty: P<Type>) -> bool {
    ty.flags().intersects(TypeFlags::Object)
}

/// `type.isClassOrInterface()`
pub(crate) fn is_class_or_interface(ty: P<Type>) -> bool {
    object_flags(ty).intersects(ObjectFlags::ClassOrInterface)
}

/// `getObjectFlags`: a type's object flags, or none for a type that has none.
pub(crate) fn object_flags(ty: P<Type>) -> ObjectFlags {
    if ty.flags().intersects(TypeFlags::ObjectFlagsType) { ty.object_flags() } else { ObjectFlags::empty() }
}

/// `type.getCallSignatures().length > 0`
pub(crate) fn has_call_signatures(cx: &mut Cx<'_>, ty: P<Type>) -> bool {
    !cx.checker().get_call_signatures(ty).is_empty()
}

/// `extractTypes`: whether `undefined` or `void` is among the types, and the others in order.
pub fn extract_types(cx: &mut Cx<'_>, types: &[P<Type>]) -> (bool, Vec<P<Type>>) {
    let undefined = cx.checker().get_undefined_type();
    let void = cx.checker().get_void_type();
    let optional = types.iter().any(|&t| t == undefined || t == void);
    (optional, types.iter().copied().filter(|&t| t != undefined && t != void).collect())
}

/// A literal value as the TS builder writes it: `f.string(value)` or `f.number(value)` (a negative number is a
/// negation, as `f.number` makes it).
fn literal_expr(value: LiteralValue) -> Option<Expr> {
    match value {
        LiteralValue::String(text) => Some(synthesized_string(text)),
        LiteralValue::Number(number) => Some(create::number(number.0)),
        LiteralValue::Boolean(value) => Some(create::bool(value)),
        LiteralValue::BigInt(_) => None,
    }
}

/// `Enum.<kind>.<item>`: a Roblox enum item.
pub(crate) fn roblox_enum_item(kind: &str, item: &str) -> Expr {
    create::property(create::property(create::id("Enum"), kind), item)
}

/// Whether `symbol` is the global `Enum` namespace (merged).
fn is_roblox_enum_root(cx: &mut Cx<'_>, symbol: P<Symbol>, enum_type: Option<P<Symbol>>) -> bool {
    let merged = cx.checker().get_merged_symbol(symbol);
    Some(merged) == enum_type
}

/// `getLiteral`: the values of a literal type, a whole TypeScript enum of constant members, or (unless
/// `without_enums`) a Roblox enum item.
pub fn get_literal(cx: &mut Cx<'_>, ty: P<Type>, without_enums: bool) -> Option<Vec<Expr>> {
    if ty.flags().intersects(TypeFlags::StringLiteral | TypeFlags::NumberLiteral) {
        return ty.as_literal_type().value().and_then(literal_expr).map(|e| vec![e]);
    }

    // TS's `getTrueType()` / `getFalseType()` are the regular literal types.
    if ty == cx.checker().regular_true_type {
        return Some(vec![create::bool(true)]);
    }
    if ty == cx.checker().regular_false_type {
        return Some(vec![create::bool(false)]);
    }

    if ty.flags().intersects(TypeFlags::Enum) {
        let declarations = ty.symbol()?.declarations();
        let [declaration] = declarations else { return None };
        if declaration.kind() != Kind::EnumDeclaration {
            return None;
        }
        let mut values = Vec::new();
        for &member in declaration.members() {
            let constant = cx.checker().get_constant_value(member)?;
            values.push(literal_expr(constant)?);
        }
        return Some(values);
    }

    if !without_enums {
        let symbol = ty.symbol()?;
        let enum_type = global_type(cx, "Enum");
        let parent = symbol.parent()?;
        let grandparent = parent.parent()?;
        if is_roblox_enum_root(cx, grandparent, enum_type) {
            return Some(vec![roblox_enum_item(parent.name(), symbol.name())]);
        }
    }
    None
}

/// `simplifyUnion` (without the serializer's literal origins): a union as the guard sees it.
pub struct SimplifiedUnion {
    /// Whole Roblox enums (`Enum.<name>`), for `t.enum`.
    pub enums: Vec<String>,
    /// The members that are neither literals nor Roblox enum items, `boolean` first when both booleans are there.
    pub types: Vec<P<Type>>,
    /// Literal values and the items of partly present Roblox enums.
    pub literals: Vec<Expr>,
}

/// `simplifyUnion`
pub fn simplify_union(cx: &mut Cx<'_>, ty: P<Type>) -> SimplifiedUnion {
    let enum_type = global_type(cx, "Enum");
    if let Some(alias) = alias_symbol(ty)
        && let Some(parent) = alias.parent()
        && is_roblox_enum_root(cx, parent, enum_type)
    {
        return SimplifiedUnion { enums: vec![alias.name().to_owned()], types: Vec::new(), literals: Vec::new() };
    }

    let members = ty.types();
    // Roblox enum kinds in first-seen order, each with its items in first-seen order.
    let mut possible_enums: Vec<(P<Symbol>, Vec<P<Type>>)> = Vec::new();
    let mut enums = Vec::new();
    let mut types = Vec::new();
    let mut literals = Vec::new();
    let is_boolean = members.iter().filter(|t| t.flags().intersects(TypeFlags::BooleanLiteral)).count() == 2;

    if is_boolean {
        types.push(cx.checker().get_boolean_type());
    }

    for &member in members {
        // Symbols do not exist in Luau.
        if member.flags().intersects(TypeFlags::ESSymbolLike) {
            continue;
        }
        // A whole `boolean`: the literals are left out.
        if is_boolean && member.flags().intersects(TypeFlags::BooleanLiteral) {
            continue;
        }
        if let Some(literal) = get_literal(cx, member, true) {
            literals.extend(literal);
            continue;
        }
        let Some(symbol) = member.symbol() else {
            types.push(member);
            continue;
        };
        let Some(enum_kind) = symbol.parent() else {
            types.push(member);
            continue;
        };
        let Some(root) = enum_kind.parent() else {
            types.push(member);
            continue;
        };
        if !is_roblox_enum_root(cx, root, enum_type) {
            types.push(member);
            continue;
        }
        let exported = enum_kind.exports().and_then(|exports| exports.lookup(symbol.name()));
        if exported == Some(symbol) {
            match possible_enums.iter_mut().find(|(kind, _)| *kind == enum_kind) {
                Some((_, items)) => {
                    if !items.contains(&member) {
                        items.push(member);
                    }
                }
                None => possible_enums.push((enum_kind, vec![member])),
            }
        }
    }

    for (kind, items) in possible_enums {
        // Every item of the enum is there. The namespace also exports `GetEnumItems` and, for some enums, alias
        // constants, so only the item interfaces count.
        let count = kind
            .exports()
            .map_or(0, |exports| exports.values().iter().filter(|m| m.flags().intersects(SymbolFlags::Interface)).count());
        if items.len() == count {
            enums.push(kind.name().to_owned());
        } else {
            for item in items {
                let name = item.symbol().map_or("", |s| s.name());
                literals.push(roblox_enum_item(kind.name(), name));
            }
        }
    }

    SimplifiedUnion { enums, types, literals }
}

/// `getInstanceTypeFromType`: the most specific instance class among the type's `_nominal_*` brands. `Err` with
/// the node to report at when the brands belong to unrelated classes ("Intersection between nominal types is
/// forbidden.").
pub(crate) fn instance_type_of(cx: &mut Cx<'_>, ty: P<Type>) -> Result<P<Type>, Option<P<Node>>> {
    let nominal = nominal_properties(cx, ty);
    let mut specific = ty;
    let mut specific_count = 0;
    for property in &nominal {
        let name = &property.name()["_nominal_".len()..];
        let Some(instance_symbol) = global_type(cx, name) else { continue };
        let Some(&declaration) = instance_symbol.declarations().first() else { continue };
        let instance_type = cx.checker().get_type_at_location(declaration);
        let count = nominal_properties(cx, instance_type).len();
        if count > specific_count {
            specific = instance_type;
            specific_count = count;
        }
    }
    for property in &nominal {
        if cx.checker().get_property_of_type_exported(specific, property.name()).is_none() {
            return Err(declaration_of_type(ty));
        }
    }
    Ok(specific)
}

fn nominal_properties(cx: &mut Cx<'_>, ty: P<Type>) -> Vec<P<Symbol>> {
    // Counted and looked up only, so the order does not matter: TS7's list, without the cost of reordering an instance
    // type's hundreds of properties.
    cx.checker().get_properties_of_type(ty).iter().copied().filter(|p| p.name().starts_with("_nominal_")).collect()
}

/// A key of a Luau table (`propertyKey.ts` `TableKey`): a string or a number.
#[derive(Clone, Debug, PartialEq)]
pub enum TableKey {
    String(String),
    Number(f64),
}

/// `getPropertyKey`: the key generated code uses for a property, as TypeScript's `keyof` works it out. A mapped or
/// late-bound property carries its key's literal type (`nameType`); any other has its declaration's name: a numeric
/// literal or a computed number is a number, an identifier or a string a string.
pub fn property_key(cx: &mut Cx<'_>, property: P<Symbol>) -> TableKey {
    if property.flags().intersects(SymbolFlags::Transient)
        && let Some(name_type) = cx.checker().get_name_type_of_symbol(property)
    {
        if name_type.flags().intersects(TypeFlags::NumberLiteral)
            && let Some(LiteralValue::Number(number)) = name_type.as_literal_type().value()
        {
            return TableKey::Number(number.0);
        }
        return TableKey::String(property.name().to_owned());
    }

    let declaration = property.value_declaration().or_else(|| property.declarations().first().copied());
    if let Some(name) = declaration.and_then(tsrs_ast::get_name_of_declaration) {
        if name.kind() == Kind::NumericLiteral {
            return TableKey::Number(rbxts_luau::js_string_to_number(name.text()));
        }
        if name.kind() == Kind::ComputedPropertyName
            && let Some(expression) = name.expression()
        {
            let ty = cx.checker().get_type_at_location(expression);
            if ty.flags().intersects(TypeFlags::NumberLiteral)
                && let Some(LiteralValue::Number(number)) = ty.as_literal_type().value()
            {
                return TableKey::Number(number.0);
            }
        }
    }
    TableKey::String(property.name().to_owned())
}

/// `keyName`: the key as a Luau table key. A string renders as `name =` when it is an identifier, else
/// `["name"] =`; a number as `[10] =`, `[-1] =` (an infinity is `1e999`, as the TS writes it).
pub fn key_expr(key: &TableKey) -> Expr {
    match key {
        TableKey::String(text) => synthesized_string(text),
        TableKey::Number(value) if value.is_infinite() => {
            let literal = create::number_text("1e999");
            if *value < 0.0 { create::unary(rbxts_luau::UnaryOperator::Neg, literal) } else { literal }
        }
        // -0 is 0, which a Luau table takes for the same key.
        TableKey::Number(value) => create::number(if *value == 0.0 { 0.0 } else { *value }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_render_as_roblox_ts_writes_them() {
        let render = |key: TableKey| {
            rbxts_luau::render_expr(&create::map(vec![(key_expr(&key), create::number(1.0))]), &Default::default())
        };
        assert_eq!(render(TableKey::String("name".into())), "{\n\tname = 1,\n}");
        assert_eq!(render(TableKey::String("two words".into())), "{\n\t[\"two words\"] = 1,\n}");
        assert_eq!(render(TableKey::Number(10.0)), "{\n\t[10] = 1,\n}");
        assert_eq!(render(TableKey::Number(-1.0)), "{\n\t[-1] = 1,\n}");
        assert_eq!(render(TableKey::Number(1.5)), "{\n\t[1.5] = 1,\n}");
    }
}
