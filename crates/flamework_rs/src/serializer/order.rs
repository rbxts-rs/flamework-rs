//! Every order on the wire (`orderAlternatives`, the ranks, `typeKey`, `sortLiterals`, `fieldOrder`): a function
//! of the types alone, never of type ids. `INTRINSIC_ORDER` and `TYPEOF_NAMES` keep TS5's start-up order.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use rbxts_plugin::ts::{Kind as SyntaxKind, Node, P, Symbol, Type};
use tsrs_ast::{CheckFlags, SymbolFlags};
use tsrs_checker::{ElementFlags, ObjectFlags, SignatureKind, TypeFlags, TypeFormatFlags};

use super::lit::Lit;
use super::typeinfo::{self, EnumMemberOrigin, alias_symbol, alias_type_arguments, check_flags, json_string};
use super::{Alternative, GResult, Gen, Kind, Shape};
use crate::js::{number_to_string, utf16_cmp};

/// TypeScript's built-in types by `intrinsicName`, in the order its checker creates them.
pub(crate) const INTRINSIC_ORDER: [&str; 16] = [
    "any", "error", "unresolved", "intrinsic", "unknown", "undefined", "null", "string", "number", "bigint", "false",
    "true", "symbol", "void", "never", "object",
];

/// The names `typeof` returns, in the order the checker creates their string literal types.
pub(crate) const TYPEOF_NAMES: [&str; 8] = ["string", "number", "bigint", "boolean", "symbol", "undefined", "object", "function"];

const MAX_SAFE: f64 = 9007199254740991.0;

/// A part of a rank: a number or a text (JavaScript's `number | string`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum RankPart {
    Num(f64),
    Str(String),
}

impl RankPart {
    fn text(&self) -> String {
        match self {
            RankPart::Num(n) => number_to_string(*n),
            RankPart::Str(s) => s.clone(),
        }
    }
}

/// `compareTuples`.
pub(crate) fn compare_tuples(a: &[RankPart], b: &[RankPart]) -> Ordering {
    for (x, y) in a.iter().zip(b) {
        if x == y {
            continue;
        }
        if let (RankPart::Num(x), RankPart::Num(y)) = (x, y) {
            return x.partial_cmp(y).unwrap_or(Ordering::Equal);
        }
        return utf16_cmp(&x.text(), &y.text());
    }
    a.len().cmp(&b.len())
}

/// `compare(x, y)` of `sortLiterals`: `x < y ? -1 : x > y ? 1 : 0`.
fn compare_parts(x: &RankPart, y: &RankPart) -> Ordering {
    match (x, y) {
        (RankPart::Num(x), RankPart::Num(y)) => x.partial_cmp(y).unwrap_or(Ordering::Equal),
        (RankPart::Str(x), RankPart::Str(y)) => utf16_cmp(x, y),
        // Mixed: JavaScript compares a number with a numeric conversion of the string.
        (RankPart::Num(x), RankPart::Str(y)) => x.partial_cmp(&crate::js::string_to_number(y)).unwrap_or(Ordering::Equal),
        (RankPart::Str(x), RankPart::Num(y)) => crate::js::string_to_number(x).partial_cmp(y).unwrap_or(Ordering::Equal),
    }
}

/// `valueRank(value)`.
pub(crate) fn value_rank(value: LiteralValueRef<'_>) -> [RankPart; 3] {
    use RankPart::{Num, Str};
    match value {
        LiteralValueRef::Bool(false) => [Num(0.0), Num(0.0), Num(0.0)],
        LiteralValueRef::Bool(true) => [Num(0.0), Num(1.0), Num(0.0)],
        LiteralValueRef::Str("") => [Num(0.0), Num(2.0), Num(0.0)],
        LiteralValueRef::Num(0.0) => [Num(0.0), Num(3.0), Num(0.0)],
        LiteralValueRef::Str(s) => match TYPEOF_NAMES.iter().position(|t| *t == s) {
            Some(i) => [Num(0.0), Num(4.0 + i as f64), Num(0.0)],
            None => [Num(2.0), Str(s.to_owned()), Num(0.0)],
        },
        LiteralValueRef::Num(n) => [Num(1.0), Num(n.abs()), Num(if n < 0.0 { 1.0 } else { 0.0 })],
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum LiteralValueRef<'a> {
    Bool(bool),
    Num(f64),
    Str(&'a str),
}

/// `printLiteral(expression)` of a literal's expression (an enum item is a TS element access, `#213`; `undefined` an
/// identifier, `#80`).
pub(crate) fn print_literal(literal: &Lit) -> String {
    match literal {
        Lit::Str(s) => json_string(s),
        Lit::Num(n) => number_to_string(*n),
        Lit::Bool(true) => "true".into(),
        Lit::Bool(false) => "false".into(),
        Lit::Nil => "#80".into(),
        Lit::EnumItem { .. } => "#213".into(),
    }
}

/// `literalKey(expression)`: `Enum.Material.Plastic` for an enum item, `undefined` for nil.
pub(crate) fn literal_key(literal: &Lit) -> String {
    match literal {
        Lit::EnumItem { group, item } => format!("Enum.{group}.{item}"),
        Lit::Nil => "undefined".into(),
        _ => print_literal(literal),
    }
}

/// The alternative ranks (`AlternativeRank`).
#[derive(Clone, Debug)]
pub(crate) struct AlternativeRank {
    group: f64,
    depth: f64,
    key: String,
    index: f64,
}

fn compare_ranks(a: &AlternativeRank, b: &AlternativeRank) -> Ordering {
    a.group
        .partial_cmp(&b.group)
        .unwrap()
        .then(a.depth.partial_cmp(&b.depth).unwrap())
        .then_with(|| utf16_cmp(&a.key, &b.key))
        .then(a.index.partial_cmp(&b.index).unwrap())
}

/// `qualifiedName(symbol)`.
pub(crate) fn qualified_name(symbol: P<Symbol>) -> String {
    // TS5's spelling of an internal name (`__class` of a class expression, `__type`), which sorts before letters and
    // `{`, where tsrs's `class` sorts after them.
    let mut name = tsrs_ast::escape_internal_symbol_name(symbol.name());
    let mut parent = symbol.parent();
    while let Some(p) = parent {
        if !p.flags().intersects(SymbolFlags::Namespace) || p.name().starts_with('"') {
            break;
        }
        name = format!("{}.{}", p.name(), name);
        parent = p.parent();
    }
    name
}

impl Gen<'_, '_, '_> {
    /// `orderAlternatives(alternatives, node, union)`.
    pub(crate) fn order_alternatives(
        &mut self,
        alternatives: &[Alternative],
        node: Option<P<Node>>,
        union: P<Type>,
    ) -> GResult<Vec<Alternative>> {
        let mut ordered: Vec<usize> = Vec::new();
        let mut aliases: HashSet<P<Node>> = HashSet::new();
        if let Some(node) = node {
            let members: Vec<P<Node>> = node.as_union_type_node().types().nodes().to_vec();
            for member in members {
                self.order_visit(alternatives, member, union, &mut ordered, &mut aliases)?;
            }
        }
        let all: Vec<usize> = (0..alternatives.len()).collect();
        self.place_by_key(alternatives, &all, union, &mut ordered, None)?;
        Ok(ordered.into_iter().map(|i| alternatives[i].clone()).collect())
    }

    fn place_by_key(
        &mut self,
        alternatives: &[Alternative],
        group: &[usize],
        union: P<Type>,
        ordered: &mut Vec<usize>,
        startup: Option<&HashMap<usize, f64>>,
    ) -> GResult<()> {
        let members: Vec<usize> = group.iter().copied().filter(|i| !ordered.contains(i)).collect();
        for i in self.by_key(alternatives, &members, union, startup)? {
            if !ordered.contains(&i) {
                ordered.push(i);
            }
        }
        Ok(())
    }

    fn order_visit(
        &mut self,
        alternatives: &[Alternative],
        member: P<Node>,
        union: P<Type>,
        ordered: &mut Vec<usize>,
        aliases: &mut HashSet<P<Node>>,
    ) -> GResult<()> {
        if member.kind() == SyntaxKind::ParenthesizedType {
            return self.order_visit(alternatives, member.type_node().unwrap(), union, ordered, aliases);
        }
        let member_type = self.cx.checker().get_type_from_type_node_exported(member);
        if !member_type.flags().intersects(TypeFlags::Union) {
            if let Some(i) = self.alternative_for(alternatives, member_type)
                && !ordered.contains(&i)
            {
                ordered.push(i);
            }
            return Ok(());
        }

        let mut found: Vec<usize> = Vec::new();
        let mut startup: HashMap<usize, f64> = HashMap::new();
        for &constituent in member_type.types() {
            let Some(alternative) = self.alternative_for(alternatives, constituent) else { continue };
            if !found.contains(&alternative) {
                found.push(alternative);
            }
            if let Some(at) = self.startup_rank(constituent) {
                let current = startup.get(&alternative).copied().unwrap_or(f64::INFINITY);
                startup.insert(alternative, current.min(at));
            }
        }
        if found.len() <= 1 {
            for i in found {
                if !ordered.contains(&i) {
                    ordered.push(i);
                }
            }
            return Ok(());
        }
        if is_typescript_enum(member_type) {
            for i in self.by_declaration(alternatives, member_type, &found) {
                if !ordered.contains(&i) {
                    ordered.push(i);
                }
            }
            return Ok(());
        }

        let with_startup: Vec<usize> = found.iter().copied().filter(|i| startup.contains_key(i)).collect();
        self.place_by_key(alternatives, &with_startup, union, ordered, Some(&startup))?;
        if member.kind() == SyntaxKind::UnionType {
            let members: Vec<P<Node>> = member.as_union_type_node().types().nodes().to_vec();
            for m in members {
                self.order_visit(alternatives, m, union, ordered, aliases)?;
            }
            return Ok(());
        }
        let alias = if alias_type_arguments(member_type).is_none() { super::classify::alias_node(member_type) } else { None };
        if let Some(alias) = alias
            && aliases.insert(alias)
        {
            self.order_visit(alternatives, alias, union, ordered, aliases)?;
        }
        self.place_by_key(alternatives, &found, union, ordered, Some(&startup))
    }

    /// `byKey(members, union, rank)`: a tie fails the build.
    fn by_key(
        &mut self,
        alternatives: &[Alternative],
        members: &[usize],
        union: P<Type>,
        startup: Option<&HashMap<usize, f64>>,
    ) -> GResult<Vec<usize>> {
        let mut ranked = Vec::new();
        for &i in members {
            let rank = match startup {
                Some(startup) => self.member_rank(&alternatives[i], startup.get(&i).copied())?,
                None => self.alternative_rank(&alternatives[i])?,
            };
            ranked.push((i, rank));
        }
        ranked.sort_by(|a, b| compare_ranks(&a.1, &b.1));
        for w in 1..ranked.len() {
            if compare_ranks(&ranked[w].1, &ranked[w - 1].1) != Ordering::Equal {
                continue;
            }
            let a = self.alternative_name(&alternatives[ranked[w - 1].0])?;
            let b = self.alternative_name(&alternatives[ranked[w].0])?;
            let text = self.type_text(union);
            return Err(self.fail(&format!(
                "the union '{text}' has two members, '{a}' and '{b}', that nothing but TypeScript's internal type ids would put in an order, and a union's members are numbered by their order. Declare an alias for the union (`type Choice = A | B`) and use it where the value is declared, so that its written order numbers them, or rename one of the two"
            )));
        }
        Ok(ranked.into_iter().map(|(i, _)| i).collect())
    }

    /// `alternativeRank(alternative)`.
    fn alternative_rank(&mut self, alternative: &Alternative) -> GResult<AlternativeRank> {
        if let Some(ty) = alternative.ty {
            if ty == self.cx.checker().get_boolean_type() {
                return Ok(AlternativeRank { group: 0.0, depth: 0.0, key: "boolean".into(), index: 0.0 });
            }
            if let Some(intrinsic) = intrinsic_rank(ty) {
                return Ok(AlternativeRank { group: 1.0, depth: 0.0, key: String::new(), index: intrinsic as f64 });
            }
            if let Some(member) = typeinfo::enum_member_of(self.cx, ty) {
                return Ok(AlternativeRank {
                    group: 2.0,
                    depth: 0.0,
                    key: qualified_name(member.enum_symbol),
                    index: member.index as f64,
                });
            }
            let depth = self.nesting_depth(ty, &mut HashSet::new());
            let key = self.type_key(ty, &mut HashSet::new());
            return Ok(AlternativeRank { group: 2.0, depth: depth as f64, key, index: -1.0 });
        }
        let kind = self.describe(&alternative.shape)?;
        Ok(match &*kind {
            Kind::Enum(name) => AlternativeRank { group: 3.0, depth: 0.0, key: format!("Enum.{name}"), index: 0.0 },
            Kind::Constant(value) => AlternativeRank { group: 4.0, depth: 0.0, key: literal_key(value), index: 0.0 },
            Kind::Literals(values) => AlternativeRank {
                group: 4.0,
                depth: 0.0,
                key: values.iter().map(literal_key).collect::<Vec<_>>().join(" | "),
                index: 0.0,
            },
            other => AlternativeRank { group: 2.0, depth: 0.0, key: other.name().into(), index: -1.0 },
        })
    }

    /// `memberRank(alternative, startup)`.
    fn member_rank(&mut self, alternative: &Alternative, startup: Option<f64>) -> GResult<AlternativeRank> {
        match startup {
            Some(at) => Ok(AlternativeRank { group: 1.0, depth: 0.0, key: String::new(), index: at }),
            None => self.alternative_rank(alternative),
        }
    }

    /// `startupRank(type)`.
    fn startup_rank(&mut self, ty: P<Type>) -> Option<f64> {
        if let Some(intrinsic) = intrinsic_rank(ty) {
            return Some(intrinsic as f64);
        }
        if ty.flags().intersects(TypeFlags::EnumLiteral) {
            return None;
        }
        let n = INTRINSIC_ORDER.len() as f64;
        if let Some(s) = typeinfo::string_literal(ty) {
            if s.is_empty() {
                return Some(n);
            }
            if let Some(i) = TYPEOF_NAMES.iter().position(|t| *t == s) {
                return Some(n + 2.0 + i as f64);
            }
        }
        if typeinfo::number_literal(ty) == Some(0.0) {
            return Some(n + 1.0);
        }
        None
    }

    /// `nestingDepth(type, seen)`.
    pub(crate) fn nesting_depth(&mut self, ty: P<Type>, seen: &mut HashSet<P<Type>>) -> u32 {
        if ty.flags().intersects(TypeFlags::Union) {
            let members: Vec<P<Type>> = ty.types().to_vec();
            return members.into_iter().map(|m| self.nesting_depth(m, seen)).max().unwrap_or(0);
        }
        let mut parts: Vec<P<Type>> = alias_type_arguments(ty).map(<[_]>::to_vec).unwrap_or_default();
        if ty.flags().intersects(TypeFlags::Intersection) {
            parts.extend_from_slice(ty.types());
        } else if typeinfo::is_reference(ty) {
            let arguments = super::typeinfo::type_arguments(self.cx, ty);
            let count = reference_type_parameter_count(ty);
            parts.extend(arguments.into_iter().take(count));
        }
        if parts.is_empty() || seen.contains(&ty) {
            return 0;
        }
        seen.insert(ty);
        let depth = parts.into_iter().map(|p| self.nesting_depth(p, seen)).max().unwrap_or(0);
        seen.remove(&ty);
        1 + depth
    }

    /// `byDeclaration(enumType, found)`.
    fn by_declaration(&mut self, alternatives: &[Alternative], enum_type: P<Type>, found: &[usize]) -> Vec<usize> {
        let mut first: HashMap<usize, usize> = HashMap::new();
        for &constituent in enum_type.types() {
            let alternative = self.alternative_for(alternatives, constituent).filter(|a| found.contains(a));
            let index = typeinfo::enum_member_of(self.cx, constituent).map(|m| m.index);
            let (Some(alternative), Some(index)) = (alternative, index) else { continue };
            let current = first.get(&alternative).copied().unwrap_or(usize::MAX);
            first.insert(alternative, current.min(index));
        }
        let mut sorted = found.to_vec();
        sorted.sort_by_key(|a| first.get(a).copied().unwrap_or(usize::MAX));
        sorted
    }

    /// `typeKey(type, seen)`.
    pub(crate) fn type_key(&mut self, ty: P<Type>, seen: &mut HashSet<P<Type>>) -> String {
        if ty == self.cx.checker().get_boolean_type() {
            return "boolean".into();
        }
        if ty.flags().intersects(TypeFlags::EnumLiteral)
            && let Some(symbol) = ty.symbol()
        {
            return qualified_name(symbol);
        }
        if let Some(literals) = typeinfo::get_literal(self.cx, ty, true) {
            let origins = typeinfo::enum_member_origins(self.cx, ty, literals.len());
            let sorted = self.sort_literals(literals, &origins);
            return sorted.iter().map(literal_key).collect::<Vec<_>>().join(" | ");
        }
        if let Some(name) = typeinfo::intrinsic_name(ty) {
            return name.to_owned();
        }
        if seen.contains(&ty) {
            return "...".into();
        }
        seen.insert(ty);
        let key = self.type_key_inner(ty, seen);
        seen.remove(&ty);
        key
    }

    fn type_key_inner(&mut self, ty: P<Type>, seen: &mut HashSet<P<Type>>) -> String {
        let sorted = |mut parts: Vec<String>| {
            parts.sort_by(|a, b| utf16_cmp(a, b));
            parts
        };
        if let Some(alias) = alias_symbol(ty) {
            let args = alias_type_arguments(ty).map(<[_]>::to_vec).unwrap_or_default();
            return format!("{}{}", qualified_name(alias), self.args_key(&args, seen));
        }
        if ty.flags().intersects(TypeFlags::Union) {
            let members: Vec<P<Type>> = ty.types().to_vec();
            let keys = members.into_iter().map(|m| self.type_key(m, seen)).collect();
            return sorted(keys).join(" | ");
        }
        if ty.flags().intersects(TypeFlags::Intersection) {
            let members: Vec<P<Type>> = ty.types().to_vec();
            let keys = members.into_iter().map(|m| self.type_key(m, seen)).collect();
            return sorted(keys).join(" & ");
        }
        if tsrs_checker::is_tuple_type_exported(ty) {
            let flags = ty.target().map(|t| t.as_tuple_type().element_flags()).unwrap_or_default();
            let arguments = super::typeinfo::type_arguments(self.cx, ty);
            let elements: Vec<String> = arguments
                .into_iter()
                .enumerate()
                .map(|(i, element)| {
                    let f = flags.get(i).copied().unwrap_or(ElementFlags::Required);
                    let key = self.type_key(element, seen);
                    if f.intersects(ElementFlags::Variable) {
                        format!("...{key}[]")
                    } else if f.intersects(ElementFlags::Optional) {
                        format!("{key}?")
                    } else {
                        key
                    }
                })
                .collect();
            return format!("[{}]", elements.join(", "));
        }
        if let Some(symbol) = ty.symbol()
            && !typeinfo::is_anonymous_name(symbol.name())
        {
            let arguments = if typeinfo::is_reference(ty) {
                let all = super::typeinfo::type_arguments(self.cx, ty);
                let count = reference_type_parameter_count(ty);
                all.into_iter().take(count).collect()
            } else {
                Vec::new()
            };
            return format!("{}{}", qualified_name(symbol), self.args_key(&arguments, seen));
        }
        if ty.flags().intersects(TypeFlags::Object) {
            let mut parts = Vec::new();
            for property in self.cx.properties_of_type(ty) {
                let optional = if property.flags().intersects(SymbolFlags::Optional) { "?" } else { "" };
                let property_type = typeinfo::type_of_named_property(self.cx, ty, property);
                let key = match property_type {
                    Some(t) => self.type_key(t, seen),
                    None => "unknown".into(),
                };
                // Escaped: a symbol key's `__@sym@12` sorts before letters, where the internal
                // `\u{7f}@sym@12` would sort after them (see `escaped_property_name` for two symbols of one text).
                let name = typeinfo::escaped_property_name(self.cx, property);
                parts.push(format!("{name}{optional}: {key}"));
            }
            let infos = self.cx.checker().get_index_infos_of_type_exported(ty).to_vec();
            for info in infos {
                let k = self.type_key(info.key_type(), seen);
                let v = self.type_key(info.value_type(), seen);
                parts.push(format!("[{k}]: {v}"));
            }
            if !self.cx.checker().get_signatures_of_type(ty, SignatureKind::Call).is_empty() {
                parts.push("()".into());
            }
            return format!("{{ {} }}", sorted(parts).join("; "));
        }
        // `typeToString(type, undefined, NoTruncation)`: the flags replace TS's defaults.
        self.cx.checker().type_to_string_ex_exported(ty, None, TypeFormatFlags::NoTruncation, None)
    }

    fn args_key(&mut self, list: &[P<Type>], seen: &mut HashSet<P<Type>>) -> String {
        if list.is_empty() {
            return String::new();
        }
        let keys: Vec<String> = list.iter().map(|&t| self.type_key(t, seen)).collect();
        format!("<{}>", keys.join(", "))
    }

    /// `alternativeFor(alternatives, constituent)`.
    pub(crate) fn alternative_for(&mut self, alternatives: &[Alternative], constituent: P<Type>) -> Option<usize> {
        if let Some(i) = alternatives.iter().position(|a| a.ty == Some(constituent)) {
            return Some(i);
        }
        if constituent.flags().intersects(TypeFlags::BooleanLiteral) {
            let boolean = self.cx.checker().get_boolean_type();
            if let Some(i) = alternatives.iter().position(|a| a.ty == Some(boolean)) {
                return Some(i);
            }
        }
        let enum_name = self.roblox_enum_of(constituent);
        if let Some(name) = &enum_name
            && let Some(i) = alternatives.iter().position(|a| match &a.shape {
                Shape::Kind(k) => matches!(&**k, Kind::Enum(n) if n == name),
                _ => false,
            })
        {
            return Some(i);
        }
        if enum_name.is_some() || typeinfo::get_literal(self.cx, constituent, true).is_some() {
            return alternatives.iter().position(|a| match &a.shape {
                Shape::Kind(k) => matches!(&**k, Kind::Constant(_) | Kind::Literals(_)),
                _ => false,
            });
        }
        None
    }

    /// `robloxEnumOf(type)`: `Material` for `Enum.Material.Plastic`.
    fn roblox_enum_of(&mut self, ty: P<Type>) -> Option<String> {
        let symbol = ty.symbol()?;
        let enum_namespace = self.resolve("Enum")?;
        let group = symbol.parent()?;
        let grand = group.parent()?;
        (self.cx.checker().get_merged_symbol(grand) == enum_namespace).then(|| group.name().to_owned())
    }

    /// `alternativeName(alternative)`.
    pub(crate) fn alternative_name(&mut self, alternative: &Alternative) -> GResult<String> {
        if let Some(ty) = alternative.ty {
            return Ok(self.type_to_string(ty));
        }
        let kind = self.describe(&alternative.shape)?;
        Ok(match &*kind {
            Kind::Constant(value) => literal_key(value),
            Kind::Literals(values) => values.iter().map(literal_key).collect::<Vec<_>>().join(" | "),
            Kind::Enum(name) => format!("Enum.{name}"),
            other => other.name().to_owned(),
        })
    }

    /// `sortLiterals(values, origins)`.
    pub(crate) fn sort_literals(&mut self, values: Vec<Lit>, origins: &[Option<EnumMemberOrigin>]) -> Vec<Lit> {
        let mut ranked: Vec<(Lit, [RankPart; 3], String)> = values
            .into_iter()
            .enumerate()
            .map(|(i, value)| {
                let rank = match origins.get(i).copied().flatten() {
                    Some(origin) => [
                        RankPart::Num(3.0),
                        RankPart::Str(qualified_name(origin.enum_symbol)),
                        RankPart::Num(origin.index as f64),
                    ],
                    None => match &value {
                        Lit::Bool(b) => value_rank(LiteralValueRef::Bool(*b)),
                        Lit::Str(s) => value_rank(LiteralValueRef::Str(s)),
                        Lit::Num(n) => value_rank(LiteralValueRef::Num(*n)),
                        other => [RankPart::Num(4.0), RankPart::Str(literal_key(other)), RankPart::Num(0.0)],
                    },
                };
                let text = literal_key(&value);
                (value, rank, text)
            })
            .collect();
        ranked.sort_by(|a, b| {
            compare_parts(&a.1[0], &b.1[0])
                .then_with(|| compare_parts(&a.1[1], &b.1[1]))
                .then_with(|| compare_parts(&a.1[2], &b.1[2]))
                .then_with(|| utf16_cmp(&a.2, &b.2))
        });
        ranked.into_iter().map(|(v, ..)| v).collect()
    }

    // --- fields ------------------------------------------------------------------------------------------------

    /// `fieldOrder(type, properties)`.
    pub(crate) fn field_order(&mut self, ty: P<Type>, properties: Vec<P<Symbol>>) -> Vec<P<Symbol>> {
        let order = self.property_order(ty, &mut HashSet::new());
        let mut position: HashMap<&str, usize> = HashMap::new();
        for (i, name) in order.iter().enumerate() {
            position.insert(name.as_str(), i);
        }
        let mut sorted = properties;
        sorted.sort_by(|a, b| {
            let at = |p: &P<Symbol>| position.get(p.name()).copied().unwrap_or(usize::MAX);
            at(a).cmp(&at(b)).then_with(|| utf16_cmp(a.name(), b.name()))
        });
        sorted
    }

    /// `propertyOrder(type, seen)`.
    fn property_order(&mut self, ty: P<Type>, seen: &mut HashSet<P<Type>>) -> Vec<String> {
        let properties = self.cx.properties_of_type(ty);
        let by_name = || {
            let mut names: Vec<String> = properties.iter().map(|p| p.name().to_owned()).collect();
            names.sort_by(|a, b| utf16_cmp(a, b));
            names
        };
        if seen.contains(&ty) {
            return by_name();
        }
        seen.insert(ty);
        let result = self.property_order_inner(ty, seen, &properties, by_name);
        seen.remove(&ty);
        result
    }

    fn property_order_inner(
        &mut self,
        ty: P<Type>,
        seen: &mut HashSet<P<Type>>,
        properties: &[P<Symbol>],
        by_name: impl Fn() -> Vec<String>,
    ) -> Vec<String> {
        if ty.flags().intersects(TypeFlags::Intersection) {
            let mut order: Vec<String> = Vec::new();
            for &part in ty.types() {
                for name in self.property_order(part, seen) {
                    if !order.contains(&name) {
                        order.push(name);
                    }
                }
            }
            return order;
        }
        if ty.flags().intersects(TypeFlags::Object) && ty.object_flags().intersects(ObjectFlags::Mapped) {
            return self.mapped_order(ty, seen);
        }
        let mapped_by: Vec<Option<P<Type>>> = properties.iter().map(|&p| self.mapped_type_of(p)).collect();
        let unknown_origin = properties.iter().zip(&mapped_by).any(|(p, m)| m.is_none() && p.declarations().is_empty());
        let synthetic = properties.iter().any(|&p| check_flags(p).intersects(CheckFlags::SyntheticProperty));
        if unknown_origin || (synthetic && mapped_by.iter().any(Option::is_some)) {
            return by_name();
        }
        let mut order = Vec::new();
        let mut start = 0;
        while start < properties.len() {
            let mapped = mapped_by[start];
            let mut end = start + 1;
            while mapped.is_some() && end < properties.len() && mapped_by[end] == mapped {
                end += 1;
            }
            let mut run: Vec<String> = properties[start..end].iter().map(|p| p.name().to_owned()).collect();
            if let Some(mapped) = mapped {
                let inner = self.property_order(mapped, seen);
                let at = |name: &String| inner.iter().position(|n| n == name).map_or(-1, |i| i as i64);
                run.sort_by_key(at);
            }
            order.extend(run);
            start = end;
        }
        order
    }

    /// `(property as MappedSymbol).links.mappedType`: the mapped type that made `property`, wherever the property is
    /// now (a spread or an object rest copies the symbol, an interface inherits it from an instantiated base).
    fn mapped_type_of(&mut self, property: P<Symbol>) -> Option<P<Type>> {
        if !check_flags(property).intersects(CheckFlags::Mapped) {
            return None;
        }
        self.cx.checker().value_symbol_links.try_get(property).and_then(|links| links.containing_type())
    }

    /// `mappedOrder(type, seen)`.
    fn mapped_order(&mut self, ty: P<Type>, seen: &mut HashSet<P<Type>>) -> Vec<String> {
        let properties = self.cx.properties_of_type(ty);
        let mapped = ty.as_mapped_type();
        let homomorphic = mapped
            .declaration
            .get()
            .and_then(|d| {
                let parameter = d.as_mapped_type_node().type_parameter;
                parameter.as_type_parameter_declaration().constraint
            })
            .is_some_and(|c| c.kind() == SyntaxKind::TypeOperator && c.as_type_operator_node().operator == SyntaxKind::KeyOfKeyword);
        let modifiers = mapped.modifiers_type.get();

        let mut ranks: Vec<(P<Symbol>, Vec<RankPart>)> = Vec::new();
        if homomorphic && let Some(modifiers) = modifiers {
            let apparent = self.cx.checker().get_apparent_type_exported(modifiers);
            let inner = self.property_order(apparent, seen);
            for &p in &properties {
                let origin = self.mapped_links_origin(p).unwrap_or(p);
                let at = inner.iter().position(|n| n == origin.name()).map_or(MAX_SAFE, |i| i as f64);
                ranks.push((p, vec![RankPart::Num(at)]));
            }
        } else if !homomorphic {
            for &p in &properties {
                let rank = match self.mapped_links_key(p) {
                    Some(key) => self.key_rank(key),
                    None => vec![RankPart::Num(MAX_SAFE)],
                };
                ranks.push((p, rank));
            }
        } else {
            let mut names: Vec<String> = properties.iter().map(|p| p.name().to_owned()).collect();
            names.sort_by(|a, b| utf16_cmp(a, b));
            return names;
        }
        ranks.sort_by(|a, b| compare_tuples(&a.1, &b.1).then_with(|| utf16_cmp(a.0.name(), b.0.name())));
        ranks.into_iter().map(|(p, _)| p.name().to_owned()).collect()
    }

    fn mapped_links_origin(&mut self, property: P<Symbol>) -> Option<P<Symbol>> {
        if !check_flags(property).intersects(CheckFlags::Mapped) {
            return None;
        }
        self.cx.checker().mapped_symbol_links.try_get(property).and_then(|l| l.synthetic_origin.get())
    }

    /// `(property as MappedSymbol).links.keyType`: the key a mapped type's property was made for, before an `as`
    /// clause renamed it (several keys that name one property: their union).
    fn mapped_links_key(&mut self, property: P<Symbol>) -> Option<P<Type>> {
        if !check_flags(property).intersects(CheckFlags::Mapped) {
            return None;
        }
        self.cx.checker().mapped_symbol_links.try_get(property).and_then(|links| links.key_type.get())
    }

    /// `keyRank(key)`.
    fn key_rank(&mut self, key: P<Type>) -> Vec<RankPart> {
        if key.flags().intersects(TypeFlags::Union) {
            let members: Vec<P<Type>> = key.types().to_vec();
            let mut least: Option<Vec<RankPart>> = None;
            for m in members {
                let rank = self.key_rank(m);
                least = Some(match least {
                    Some(l) if compare_tuples(&rank, &l) != Ordering::Less => l,
                    _ => rank,
                });
            }
            return least.unwrap_or_default();
        }
        if let Some(member) = typeinfo::enum_member_of(self.cx, key) {
            return vec![RankPart::Num(3.0), RankPart::Str(qualified_name(member.enum_symbol)), RankPart::Num(member.index as f64)];
        }
        if let Some(n) = typeinfo::number_literal(key) {
            return value_rank(LiteralValueRef::Num(n)).to_vec();
        }
        if let Some(s) = typeinfo::string_literal(key) {
            return value_rank(LiteralValueRef::Str(s)).to_vec();
        }
        vec![RankPart::Num(5.0), RankPart::Str(String::new()), RankPart::Num(0.0)]
    }
}

/// `intrinsicRank(type)`.
fn intrinsic_rank(ty: P<Type>) -> Option<usize> {
    let name = typeinfo::intrinsic_name(ty)?;
    INTRINSIC_ORDER.iter().position(|n| *n == name)
}

/// `isTypeScriptEnum(type)`.
fn is_typescript_enum(ty: P<Type>) -> bool {
    ty.flags().intersects(TypeFlags::EnumLiteral) && ty.symbol().is_some_and(|s| s.flags().intersects(SymbolFlags::Enum))
}

/// `(type as TypeReference).target.typeParameters?.length`.
fn reference_type_parameter_count(ty: P<Type>) -> usize {
    ty.target()
        .and_then(|t| t.try_as_interface_type())
        .map(|i| i.type_parameters().len())
        .unwrap_or(0)
}
