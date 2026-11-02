//! Classification (`describe`, `classify`, unions, intersections, objects) and spelling (`spell`, `listOf`).

use std::rc::Rc;

use rbxts_plugin::ts::{Kind as SyntaxKind, Node, P, Symbol, Type};
use rbxts_plugin::{Abort, PluginDiagnostic};
use tsrs_ast::SymbolFlags;
use tsrs_checker::{ElementFlags, SignatureKind, TypeFlags};

use super::typeinfo::{self, alias_symbol, alias_type_arguments};
use super::{Alternative, Field, GResult, Gen, Kind, LengthWidth, Shape, Width};

/// Roblox datatypes with a buffer representation: the fields written, in constructor order.
/// A datatype's fields: the width and the property path of each.
pub(crate) type DatatypeFields = &'static [(Width, &'static [&'static str])];

pub(crate) const DATATYPES: &[(&str, DatatypeFields)] = &[
    ("Vector3", &[(Width::F32, &["X"]), (Width::F32, &["Y"]), (Width::F32, &["Z"])]),
    ("Vector2", &[(Width::F32, &["X"]), (Width::F32, &["Y"])]),
    ("Vector3int16", &[(Width::I16, &["X"]), (Width::I16, &["Y"]), (Width::I16, &["Z"])]),
    ("Vector2int16", &[(Width::I16, &["X"]), (Width::I16, &["Y"])]),
    ("Color3", &[(Width::F32, &["R"]), (Width::F32, &["G"]), (Width::F32, &["B"])]),
    ("UDim", &[(Width::F32, &["Scale"]), (Width::I32, &["Offset"])]),
    (
        "UDim2",
        &[(Width::F32, &["X", "Scale"]), (Width::I32, &["X", "Offset"]), (Width::F32, &["Y", "Scale"]), (Width::I32, &["Y", "Offset"])],
    ),
    ("NumberRange", &[(Width::F32, &["Min"]), (Width::F32, &["Max"])]),
    ("Rect", &[(Width::F32, &["Min", "X"]), (Width::F32, &["Min", "Y"]), (Width::F32, &["Max", "X"]), (Width::F32, &["Max", "Y"])]),
    ("BrickColor", &[(Width::U16, &["Number"])]),
];

pub(crate) fn datatype(name: &str) -> Option<(&'static str, DatatypeFields)> {
    DATATYPES.iter().find(|(n, _)| *n == name).map(|(n, fields)| (*n, *fields))
}

const STRING_BRANDS: [(&str, LengthWidth); 3] =
    [("u8_string", LengthWidth::U8), ("u16_string", LengthWidth::U16), ("u32_string", LengthWidth::U32)];
const BUFFER_BRANDS: [(&str, LengthWidth); 2] = [("u16_buffer", LengthWidth::U16), ("u32_buffer", LengthWidth::U32)];

fn is_number_brand(literal: &str) -> bool {
    Width::from_brand(literal).is_some() || literal == "varint"
}
fn is_string_brand(literal: &str) -> bool {
    STRING_BRANDS.iter().any(|(b, _)| *b == literal)
}
fn is_buffer_brand(literal: &str) -> bool {
    BUFFER_BRANDS.iter().any(|(b, _)| *b == literal)
}

/// A brand's literal, and whether every property naming it is optional.
struct Brand {
    brand: &'static str,
    implicit: bool,
}

/// JavaScript's `/\s+/g` → `" "`.
pub(crate) fn collapse_whitespace(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_space = false;
    for c in text.chars() {
        if c.is_whitespace() || c == '\u{feff}' {
            if !in_space {
                out.push(' ');
            }
            in_space = true;
        } else {
            out.push(c);
            in_space = false;
        }
    }
    out
}

impl Gen<'_, '_, '_> {
    /// A new synthetic kind, kept alive for the file.
    pub(crate) fn kind(&mut self, kind: Kind) -> Rc<Kind> {
        let rc = Rc::new(kind);
        self.g.keep.push(rc.clone());
        rc
    }

    pub(crate) fn shape(&mut self, kind: Kind) -> Shape {
        Shape::Kind(self.kind(kind))
    }

    /// `fail(message)`: the build error, with the types the value was reached through.
    pub(crate) fn fail(&mut self, message: &str) -> Abort {
        let trail = self.g.trail.clone();
        let chain: Vec<String> = trail.iter().map(|&t| self.type_text(t)).collect();
        let chain = chain.join(" > ");
        let what = if self.g.ordering_macro {
            "Flamework cannot order the values this macro lists"
        } else {
            "Flamework cannot serialize this type"
        };
        let mut diagnostic = PluginDiagnostic::error(self.g.diagnostic_node, format!("{what}: {message}."));
        if !chain.is_empty() {
            diagnostic = diagnostic.with_hint(format!("Reached through: {chain}"));
        }
        self.cx.report(diagnostic).expect("an error")
    }

    /// `resolve(name)`: a global type's symbol.
    pub(crate) fn resolve(&mut self, name: &str) -> Option<P<Symbol>> {
        typeinfo::resolve_global(self.cx, name, SymbolFlags::Type)
    }

    pub(crate) fn type_to_string(&mut self, ty: P<Type>) -> String {
        self.cx.checker().type_to_string_exported(ty)
    }

    /// `typeText(type)`.
    pub(crate) fn type_text(&mut self, ty: P<Type>) -> String {
        if let Some(&node) = self.g.written_as.get(&ty) {
            return collapse_whitespace(self.cx.source_text(node));
        }
        let text = self.type_to_string(ty);
        if text != "never" || !ty.flags().intersects(TypeFlags::Intersection) {
            return text;
        }
        if let Some(alias) = alias_symbol(ty) {
            let args = alias_type_arguments(ty).map(|args| {
                let texts: Vec<String> = args.iter().map(|&a| self.type_text(a)).collect();
                format!("<{}>", texts.join(", "))
            });
            return format!("{}{}", alias.name(), args.unwrap_or_default());
        }
        let members: Vec<String> = ty
            .types()
            .iter()
            .map(|&m| if m.flags().intersects(TypeFlags::Union) { format!("({})", self.type_text(m)) } else { self.type_text(m) })
            .collect();
        members.join(" & ")
    }

    /// `serializerName`: the type argument as written (spaces collapsed), or as printed; cut past 60 characters.
    pub(crate) fn serializer_name(&mut self, written: Option<P<Node>>, ty: P<Type>) -> String {
        let text = match written {
            Some(node) => self.cx.source_text(node).to_owned(),
            None => self.type_to_string(ty),
        };
        let text = collapse_whitespace(&text);
        let units: Vec<u16> = text.encode_utf16().collect();
        if units.len() > 60 { format!("{}...", String::from_utf16_lossy(&units[..57])) } else { text }
    }

    /// `unwrapPromise`.
    pub(crate) fn unwrap_promise(&mut self, ty: P<Type>) -> P<Type> {
        let promise = self.resolve_value_or_type("Promise");
        if promise.is_some() && ty.symbol() == promise {
            return super::typeinfo::type_arguments(self.cx, ty).first().copied().unwrap_or(ty);
        }
        ty
    }

    fn resolve_value_or_type(&mut self, name: &str) -> Option<P<Symbol>> {
        self.resolve(name)
    }

    // --- describe / classify ---------------------------------------------------------------------------------

    /// `describe(shape)`.
    pub(crate) fn describe(&mut self, shape: &Shape) -> GResult<Rc<Kind>> {
        match shape {
            Shape::Kind(kind) => Ok(kind.clone()),
            Shape::Type(ty) => {
                if let Some(kind) = self.g.kinds.get(ty) {
                    return Ok(kind.clone());
                }
                let kind = self.classify(*ty)?;
                self.g.kinds.insert(*ty, kind.clone());
                Ok(kind)
            }
        }
    }

    fn classify(&mut self, ty: P<Type>) -> GResult<Rc<Kind>> {
        let flags = ty.flags();
        if flags.intersects(TypeFlags::Union) {
            return self.classify_union(ty, None);
        }
        if typeinfo::is_instance_type(self.cx, ty) {
            return Ok(self.kind(Kind::Blob { typeof_name: Some("Instance".into()) }));
        }
        if flags.intersects(TypeFlags::Intersection) {
            return self.classify_intersection(ty);
        }
        if typeinfo::is_conditional_type(ty) {
            // `resolvedTrueType` / `resolvedFalseType`: tsrs 0.20 keeps the fields private; the exported getters
            // resolve them the same way (`getTrueTypeFromConditionalType`).
            let branches = [
                self.cx.checker().get_true_type_of_conditional_type(ty),
                self.cx.checker().get_false_type_of_conditional_type(ty),
            ];
            let alternatives =
                branches.into_iter().map(|b| Alternative { shape: Shape::Type(b), ty: Some(b) }).collect();
            return Ok(self.union_kind(alternatives, ty));
        }
        if flags.intersects(TypeFlags::TypeVariable) {
            let constraint = self.cx.checker().get_base_constraint_of_type_exported(ty);
            return match constraint {
                Some(c) => self.describe(&Shape::Type(c)),
                None => Ok(self.kind(Kind::Blob { typeof_name: None })),
            };
        }
        if let Some(literals) = typeinfo::get_literal(self.cx, ty, false) {
            if literals.len() == 1 {
                return Ok(self.kind(Kind::Constant(literals.into_iter().next().unwrap())));
            }
            let origins = typeinfo::enum_member_origins(self.cx, ty, literals.len());
            let values = self.sort_literals(literals, &origins);
            return Ok(self.kind(Kind::Literals(values)));
        }
        if flags.intersects(TypeFlags::Void | TypeFlags::Undefined | TypeFlags::Null) {
            return Ok(self.kind(Kind::Nothing));
        }
        if flags.intersects(TypeFlags::Any | TypeFlags::Unknown) {
            return Ok(self.kind(Kind::Blob { typeof_name: None }));
        }
        if flags.intersects(TypeFlags::Never) {
            return Err(self.fail("`never` has no values"));
        }
        if flags.intersects(TypeFlags::String | TypeFlags::TemplateLiteral | TypeFlags::StringMapping) {
            return Ok(self.kind(Kind::String { length: LengthWidth::V, implicit: false }));
        }
        if flags.intersects(TypeFlags::Number) {
            return Ok(self.kind(Kind::Number { width: Width::F64, implicit: false }));
        }
        if flags.intersects(TypeFlags::BigInt) {
            return Err(self.fail("bigint does not exist in Luau"));
        }
        if flags.intersects(TypeFlags::ESSymbolLike) {
            return Err(self.fail("symbols cannot be sent"));
        }

        if tsrs_checker::is_tuple_type_exported(ty) {
            let list = self.list_of(ty, None)?;
            return self.describe(&list);
        }

        if self.cx.checker().is_array_type_exported(ty) {
            let element = super::typeinfo::type_arguments(self.cx, ty).first().copied();
            let Some(element) = element else { return Err(self.fail("an array without an element type")) };
            self.inherit_hint(ty, Some(&Shape::Type(element)));
            return Ok(self.kind(Kind::Array(Shape::Type(element))));
        }

        if !self.cx.checker().get_signatures_of_type(ty, SignatureKind::Call).is_empty() {
            return Err(self.fail("functions cannot be sent"));
        }

        let Some(symbol) = ty.symbol() else { return Ok(self.kind(Kind::Blob { typeof_name: None })) };

        let is = |gn: &mut Self, name: &str| gn.resolve(name) == Some(symbol);
        if is(self, "Map") || is(self, "ReadonlyMap") {
            let args = super::typeinfo::type_arguments(self.cx, ty);
            let (Some(&key), Some(&value)) = (args.first(), args.get(1)) else {
                return Err(self.fail("a Map without key and value types"));
            };
            self.inherit_hint(ty, Some(&Shape::Type(value)));
            return Ok(self.kind(Kind::Map(Shape::Type(key), Shape::Type(value))));
        }
        if is(self, "Set") || is(self, "ReadonlySet") {
            let args = super::typeinfo::type_arguments(self.cx, ty);
            let Some(&element) = args.first() else { return Err(self.fail("a Set without an element type")) };
            self.inherit_hint(ty, Some(&Shape::Type(element)));
            return Ok(self.kind(Kind::Set(Shape::Type(element))));
        }
        if is(self, "WeakMap") || is(self, "WeakSet") {
            return Err(self.fail("weak collections cannot be sent"));
        }
        if is(self, "Promise") {
            return Err(self.fail("a Promise cannot be sent; send its resolved value"));
        }
        if is(self, "buffer") {
            return Ok(self.kind(Kind::Buffer { length: LengthWidth::V, implicit: false }));
        }

        let name = symbol.name();
        let global = self.resolve(name) == Some(symbol);
        if global {
            if name == "CFrame" {
                return Ok(self.kind(Kind::CFrame));
            }
            if let Some((datatype, _)) = datatype(name) {
                return Ok(self.kind(Kind::Datatype(datatype)));
            }
        }

        let enum_namespace = self.resolve("Enum");
        if let Some(parent) = symbol.parent()
            && enum_namespace.is_some()
            && Some(self.cx.checker().get_merged_symbol(parent)) == enum_namespace
        {
            return Ok(self.kind(Kind::Enum(name.to_owned())));
        }

        if is_roblox_type(Some(symbol)) {
            let typeof_name = if global && self.is_typeof_name(name) { Some(name.to_owned()) } else { None };
            return Ok(self.kind(Kind::Blob { typeof_name }));
        }
        if self.has_nominal_marker(ty) {
            return Ok(self.kind(Kind::Blob { typeof_name: None }));
        }
        if ty.is_class() {
            return Ok(self.kind(Kind::Blob { typeof_name: None }));
        }
        self.classify_object(ty)
    }

    /// `hasNominalMarker`: a `_nominal_` property.
    pub(crate) fn has_nominal_marker(&mut self, ty: P<Type>) -> bool {
        typeinfo::properties_in_ts_order(self.cx, ty).iter().any(|p| p.name().starts_with("_nominal_"))
    }

    /// `isTypeofName`.
    pub(crate) fn is_typeof_name(&mut self, name: &str) -> bool {
        if name == "Instance" {
            return true;
        }
        if self.g.checkable_types.is_none() {
            let symbol = self.resolve("CheckableTypes");
            let ty = symbol.map(|s| self.cx.checker().get_declared_type_of_symbol_exported(s));
            self.g.checkable_types = Some(ty);
        }
        match self.g.checkable_types.unwrap() {
            None => true,
            Some(ty) => typeinfo::get_property(self.cx, ty, name).is_some(),
        }
    }

    // --- unions ------------------------------------------------------------------------------------------------

    /// `classifyUnion(type, node)`.
    pub(crate) fn classify_union(&mut self, ty: P<Type>, node: Option<P<Node>>) -> GResult<Rc<Kind>> {
        if ty == self.cx.checker().get_boolean_type() {
            return Ok(self.kind(Kind::Boolean));
        }
        let entry = self.alternatives_of(ty);
        let (is_optional, alternatives) = (&entry.0, &entry.1);
        for alternative in alternatives {
            if let Some(t) = alternative.ty {
                self.inherit_hint(ty, Some(&Shape::Type(t)));
            }
        }
        let inner: Shape = if alternatives.is_empty() {
            self.shape(Kind::Nothing)
        } else if alternatives.len() == 1 {
            alternatives[0].shape.clone()
        } else if alternatives.len() > 0xff {
            self.shape(Kind::Blob { typeof_name: None })
        } else {
            let node = node.or_else(|| alias_node(ty));
            let ordered = self.order_alternatives(alternatives, node, ty)?;
            Shape::Kind(self.union_kind(ordered, ty))
        };
        if *is_optional {
            return Ok(self.kind(Kind::Optional(inner)));
        }
        self.describe(&inner)
    }

    /// `unionKind(alternatives, type)`.
    pub(crate) fn union_kind(&mut self, alternatives: Vec<Alternative>, ty: P<Type>) -> Rc<Kind> {
        let whole = alternatives.iter().position(|a| a.ty.is_some_and(|t| t.flags().intersects(TypeFlags::Number)));
        let whole = if alternatives.len() <= 0xff { whole } else { None };
        self.kind(Kind::Union { alternatives, ty: Some(ty), whole })
    }

    /// `alternativesOf(type)`.
    pub(crate) fn alternatives_of(&mut self, ty: P<Type>) -> Rc<(bool, Vec<Alternative>)> {
        if let Some(entry) = self.g.union_alternatives.get(&ty) {
            return entry.clone();
        }
        let simplified = typeinfo::simplify_union(self.cx, ty);
        let (is_optional, members) = typeinfo::extract_types(self.cx, &simplified.types);
        let mut alternatives = Vec::new();
        for member in members {
            alternatives.push(Alternative { shape: Shape::Type(member), ty: Some(member) });
        }
        for name in simplified.enums {
            let shape = self.shape(Kind::Enum(name));
            alternatives.push(Alternative { shape, ty: None });
        }
        if simplified.literals.len() == 1 {
            let shape = self.shape(Kind::Constant(simplified.literals.into_iter().next().unwrap()));
            alternatives.push(Alternative { shape, ty: None });
        } else if simplified.literals.len() > 1 {
            let values = self.sort_literals(simplified.literals, &simplified.literal_origins);
            let shape = self.shape(Kind::Literals(values));
            alternatives.push(Alternative { shape, ty: None });
        }
        let entry = Rc::new((is_optional, alternatives));
        self.g.union_alternatives.insert(ty, entry.clone());
        entry
    }

    // --- intersections -----------------------------------------------------------------------------------------

    fn classify_intersection(&mut self, ty: P<Type>) -> GResult<Rc<Kind>> {
        let types: Vec<P<Type>> = ty.types().to_vec();
        for &member in &types {
            if typeinfo::get_property(self.cx, member, "LUA_TUPLE").is_some() {
                return Err(self.fail(
                    "a LuaTuple is several values at runtime, not a table; declare a tuple type such as `[A, B]` instead",
                ));
            }
        }

        let disjoint = types.iter().copied().find(|m| m.flags().intersects(TypeFlags::DisjointDomains));
        if let Some(disjoint) = disjoint {
            if disjoint.flags().intersects(TypeFlags::Number) {
                let Some(found) = self.find_brand(ty, is_number_brand)? else {
                    return Ok(self.kind(Kind::Number { width: Width::F64, implicit: false }));
                };
                if found.brand == "varint" {
                    return Ok(self.kind(Kind::Varint { implicit: found.implicit }));
                }
                let width = Width::from_brand(found.brand).unwrap();
                return Ok(self.kind(Kind::Number { width, implicit: found.implicit }));
            }
            if disjoint.flags().intersects(TypeFlags::String) {
                let Some(found) = self.find_brand(ty, is_string_brand)? else {
                    return Ok(self.kind(Kind::String { length: LengthWidth::V, implicit: false }));
                };
                let length = STRING_BRANDS.iter().find(|(b, _)| *b == found.brand).unwrap().1;
                return Ok(self.kind(Kind::String { length, implicit: found.implicit }));
            }
            return self.describe(&Shape::Type(disjoint));
        }

        let buffer = self.resolve("buffer");
        if types.iter().any(|m| m.symbol().is_some() && m.symbol() == buffer) {
            let Some(found) = self.find_brand(ty, is_buffer_brand)? else {
                return Ok(self.kind(Kind::Buffer { length: LengthWidth::V, implicit: false }));
            };
            let length = BUFFER_BRANDS.iter().find(|(b, _)| *b == found.brand).unwrap().1;
            return Ok(self.kind(Kind::Buffer { length, implicit: found.implicit }));
        }

        for &member in &types {
            let Some(symbol) = member.symbol() else { continue };
            let name = symbol.name();
            if (datatype(name).is_some() || name == "CFrame") && self.resolve(name) == Some(symbol) {
                return self.describe(&Shape::Type(member));
            }
        }
        for &member in &types {
            if typeinfo::is_instance_type(self.cx, member) {
                return Ok(self.kind(Kind::Blob { typeof_name: Some("Instance".into()) }));
            }
        }
        if types.iter().any(|m| is_roblox_type(m.symbol())) || self.has_nominal_marker(ty) {
            return Ok(self.kind(Kind::Blob { typeof_name: None }));
        }
        self.classify_object(ty)
    }

    /// `findBrand(type, isWidth)`.
    fn find_brand(&mut self, ty: P<Type>, is_width: fn(&str) -> bool) -> GResult<Option<Brand>> {
        let mut found: Option<Brand> = None;
        for &member in ty.types() {
            if !member.flags().intersects(TypeFlags::Object) {
                continue;
            }
            for property in typeinfo::properties_in_ts_order(self.cx, member) {
                let optional = property.flags().intersects(SymbolFlags::Optional);
                let mut property_type = typeinfo::type_of_named_property(self.cx, member, property);
                if let Some(t) = property_type
                    && optional
                {
                    property_type = Some(self.cx.checker().get_non_nullable_type(t));
                }
                let Some(brand) = property_type.and_then(typeinfo::string_literal) else { continue };
                if !is_width(brand) {
                    continue;
                }
                if let Some(previous) = &found
                    && previous.brand != brand
                {
                    let rename = |literal: &str| {
                        for (from, to) in [
                            ("u8_string", "string8"),
                            ("u16_string", "string16"),
                            ("u32_string", "string32"),
                            ("u16_buffer", "buffer16"),
                            ("u32_buffer", "buffer32"),
                        ] {
                            if literal == from {
                                return to.to_owned();
                            }
                        }
                        literal.to_owned()
                    };
                    let (a, b) = (rename(previous.brand), rename(brand));
                    let text = self.type_text(ty);
                    return Err(self.fail(&format!(
                        "'{text}' names two widths, {a} and {b}; a value is written at one width, so keep one of them"
                    )));
                }
                let implicit = found.as_ref().is_none_or(|f| f.implicit) && optional;
                found = Some(Brand { brand, implicit });
            }
        }
        Ok(found)
    }

    // --- objects -----------------------------------------------------------------------------------------------

    fn classify_object(&mut self, ty: P<Type>) -> GResult<Rc<Kind>> {
        let index_infos = self.cx.checker().get_index_infos_of_type_exported(ty).to_vec();
        let mut properties = Vec::new();
        for property in typeinfo::properties_in_ts_order(self.cx, ty) {
            let property_type = typeinfo::type_of_named_property(self.cx, ty, property);
            if let Some(t) = property_type
                && !t.flags().intersects(TypeFlags::Never | TypeFlags::UniqueESSymbol)
            {
                properties.push(property);
            }
        }

        if properties.is_empty() && index_infos.is_empty() {
            return Ok(self.kind(Kind::Blob { typeof_name: None }));
        }
        if !index_infos.is_empty() {
            if !properties.is_empty() || index_infos.len() > 1 {
                return Ok(self.kind(Kind::Blob { typeof_name: None }));
            }
            let info = index_infos[0];
            let key = info.key_type();
            let value = info.value_type();
            return Ok(self.kind(Kind::Map(Shape::Type(key), Shape::Type(value))));
        }

        let ordered = self.field_order(ty, properties);
        let mut fields = Vec::new();
        for property in ordered {
            let property_type = typeinfo::type_of_named_property(self.cx, ty, property).unwrap();
            if !self.cx.checker().get_signatures_of_type(property_type, SignatureKind::Call).is_empty() {
                return Err(self.fail(&format!("property '{}' is a function", property.name())));
            }
            let declared = declared_type_node(property.value_declaration());
            let written = self.spell(declared, property_type)?;
            if let Shape::Type(t) = &written
                && !self.g.hints.contains_key(t)
            {
                self.g.hints.insert(*t, property.name().to_owned());
            }
            let optional = property.flags().intersects(SymbolFlags::Optional) && !has_undefined(property_type);
            let shape = if optional { self.shape(Kind::Optional(written)) } else { written };
            let key = typeinfo::get_property_key(self.cx, property);
            fields.push(Field { name: property.name().to_owned(), key, shape });
        }
        Ok(self.kind(Kind::Object { fields }))
    }

    // --- spelling ----------------------------------------------------------------------------------------------

    /// `spell(node, type)`: the shape of `type` as written at `node`.
    pub(crate) fn spell(&mut self, node: Option<P<Node>>, ty: P<Type>) -> GResult<Shape> {
        let Some(node) = node else { return Ok(Shape::Type(ty)) };
        self.name_after(node, ty);
        match node.kind() {
            SyntaxKind::ParenthesizedType => return self.spell(node.type_node(), ty),
            SyntaxKind::TypeOperator if node.as_type_operator_node().operator == SyntaxKind::ReadonlyKeyword => {
                return self.spell(node.type_node(), ty);
            }
            _ => {}
        }

        if ty.flags().intersects(TypeFlags::Intersection)
            && !self.g.written_as.contains_key(&ty)
            && self.cx.checker().get_type_from_type_node_exported(node) == ty
        {
            self.g.written_as.insert(ty, node);
        }

        match node.kind() {
            SyntaxKind::UnionType => {
                if ty.flags().intersects(TypeFlags::Union) {
                    let kind = self.classify_union(ty, Some(node))?;
                    return Ok(Shape::Kind(kind));
                }
                Ok(Shape::Type(ty))
            }
            SyntaxKind::ArrayType => {
                let kind = self.describe(&Shape::Type(ty))?;
                let Kind::Array(element) = &*kind else { return Ok(Shape::Type(ty)) };
                let Shape::Type(element_type) = element else { return Ok(Shape::Type(ty)) };
                let spelled = self.spell(Some(node.as_array_type_node().element_type), *element_type)?;
                if spelled.same(element) { Ok(Shape::Type(ty)) } else { Ok(self.shape(Kind::Array(spelled))) }
            }
            SyntaxKind::TypeReference if node.type_argument_list().is_some() => {
                let args = node.type_arguments();
                let written = self.cx.checker().get_type_from_type_node_exported(node);
                if written != ty {
                    if args.len() == 1 && self.unwrap_promise(written) == ty {
                        return self.spell(Some(args[0]), ty);
                    }
                    return Ok(Shape::Type(ty));
                }
                let kind = self.describe(&Shape::Type(ty))?;
                match &*kind {
                    Kind::Array(element) | Kind::Set(element) if args.len() == 1 => {
                        let Shape::Type(element_type) = element else { return Ok(Shape::Type(ty)) };
                        let spelled = self.spell(Some(args[0]), *element_type)?;
                        if spelled.same(element) {
                            return Ok(Shape::Type(ty));
                        }
                        let k = if matches!(&*kind, Kind::Array(_)) { Kind::Array(spelled) } else { Kind::Set(spelled) };
                        Ok(self.shape(k))
                    }
                    Kind::Map(key, value) if args.len() == 2 => {
                        let (Shape::Type(key_type), Shape::Type(value_type)) = (key, value) else {
                            return Ok(Shape::Type(ty));
                        };
                        let spelled_key = self.spell(Some(args[0]), *key_type)?;
                        let spelled_value = self.spell(Some(args[1]), *value_type)?;
                        if spelled_key.same(key) && spelled_value.same(value) {
                            return Ok(Shape::Type(ty));
                        }
                        Ok(self.shape(Kind::Map(spelled_key, spelled_value)))
                    }
                    _ => Ok(Shape::Type(ty)),
                }
            }
            SyntaxKind::TupleType if tsrs_checker::is_tuple_type_exported(ty) => self.list_of(ty, Some(node)),
            _ => Ok(Shape::Type(ty)),
        }
    }

    /// `spellElement`: a rest element's node is the array, its type the element.
    fn spell_element(&mut self, node: Option<P<Node>>, ty: P<Type>, rest: bool) -> GResult<Shape> {
        let Some(n) = node else { return self.spell(None, ty) };
        if !rest {
            return self.spell(node, ty);
        }
        if n.kind() == SyntaxKind::ArrayType {
            return self.spell(Some(n.as_array_type_node().element_type), ty);
        }
        if n.kind() == SyntaxKind::TypeReference && n.type_arguments_len() == Some(1) {
            return self.spell(Some(n.type_arguments()[0]), ty);
        }
        Ok(Shape::Type(ty))
    }

    /// `listOf(type, node)`: the argument list a tuple type describes.
    pub(crate) fn list_of(&mut self, ty: P<Type>, node: Option<P<Node>>) -> GResult<Shape> {
        if !tsrs_checker::is_tuple_type_exported(ty) {
            if self.cx.checker().is_array_type_exported(ty) {
                let element = super::typeinfo::type_arguments(self.cx, ty).first().copied();
                if let Some(element) = element {
                    let element = self.unwrap_promise(element);
                    return Ok(self.shape(Kind::List { elements: vec![], rest: Some(Shape::Type(element)), after: None }));
                }
            }
            let element = self.unwrap_promise(ty);
            return Ok(self.shape(Kind::List { elements: vec![Shape::Type(element)], rest: None, after: None }));
        }

        let mut elements = Vec::new();
        let mut rest: Option<Shape> = None;
        let mut after = Vec::new();
        let types = super::typeinfo::type_arguments(self.cx, ty);
        let target = ty.target().expect("a tuple reference's target");
        let infos = target.as_tuple_type().element_infos();
        for (i, &raw) in types.iter().enumerate() {
            let Some(info) = infos.get(i) else { break };
            let element = self.unwrap_promise(raw);
            let flags = info.flags;
            let is_rest = flags.intersects(ElementFlags::Rest);
            let written = match node {
                Some(n) => tuple_element_node(n.as_tuple_type_node().elements().nodes().get(i).copied()),
                None => declared_type_node(info.labeled_declaration()),
            };
            let shape = self.spell_element(written, element, is_rest)?;
            if is_rest {
                rest = Some(shape);
            } else if rest.is_some() {
                after.push(shape);
            } else if flags.intersects(ElementFlags::Optional) && !has_undefined(element) {
                elements.push(self.shape(Kind::Optional(shape)));
            } else {
                elements.push(shape);
            }
        }
        let after = if after.is_empty() { None } else { Some(after) };
        Ok(self.shape(Kind::List { elements, rest, after }))
    }

    /// `resultOf(fn, node)`: a function type's (resolved) result as a one-element list.
    pub(crate) fn result_of(&mut self, function: P<Type>, node: P<Node>) -> GResult<Shape> {
        let signature = self.cx.checker().get_signatures_of_type(function, SignatureKind::Call).first().copied();
        let Some(signature) = signature else {
            let text = self.type_to_string(function);
            let diagnostic = PluginDiagnostic::error(node, format!("Flamework expected a function type here, got '{text}'."));
            return Err(self.cx.report(diagnostic).expect("an error"));
        };
        let return_type = self.cx.checker().get_return_type_of_signature(signature);
        let return_type = self.unwrap_promise(return_type);
        let declared = signature.declaration().and_then(|d| d.type_node());
        let element = self.spell(declared, return_type)?;
        Ok(self.shape(Kind::List { elements: vec![element], rest: None, after: None }))
    }

    // --- hints -------------------------------------------------------------------------------------------------

    /// `nameAfter(node, type)`.
    fn name_after(&mut self, node: P<Node>, ty: P<Type>) {
        if self.g.hints.contains_key(&ty) {
            return;
        }
        let text = self.cx.source_text(node);
        if !text.contains('{') {
            self.g.hints.insert(ty, text.to_owned());
        }
    }

    /// `inheritHint(parent, child)`.
    pub(crate) fn inherit_hint(&mut self, parent: P<Type>, child: Option<&Shape>) {
        let Some(hint) = self.g.hints.get(&parent).cloned() else { return };
        if let Some(Shape::Type(child)) = child
            && !self.g.hints.contains_key(child)
        {
            self.g.hints.insert(*child, hint);
        }
    }
}

/// `isRobloxType(symbol)`: declared in `@rbxts/types`.
pub(crate) fn is_roblox_type(symbol: Option<P<Symbol>>) -> bool {
    let Some(symbol) = symbol else { return false };
    symbol.declarations().iter().any(|&declaration| {
        tsrs_ast::get_source_file_of_node(declaration).is_some_and(|file| {
            let name = file.file_name();
            name.contains("/@rbxts/types/") || name.contains("\\@rbxts\\types\\") || name.contains("/@rbxts\\types") || name.contains("\\@rbxts/types")
        })
    })
}

/// `hasUndefined(type)`.
pub(crate) fn has_undefined(ty: P<Type>) -> bool {
    ty.flags().intersects(TypeFlags::Union)
        && ty.types().iter().any(|m| m.flags().intersects(TypeFlags::Undefined | TypeFlags::Void))
}

/// `aliasNode(type)`: the union node of its alias's declaration.
pub(crate) fn alias_node(ty: P<Type>) -> Option<P<Node>> {
    let declaration = alias_symbol(ty)?.declarations().first().copied()?;
    if declaration.kind() != SyntaxKind::TypeAliasDeclaration {
        return None;
    }
    let ty_node = declaration.type_node()?;
    (ty_node.kind() == SyntaxKind::UnionType).then_some(ty_node)
}

/// `tupleElementNode`.
fn tuple_element_node(element: Option<P<Node>>) -> Option<P<Node>> {
    let element = element?;
    match element.kind() {
        SyntaxKind::NamedTupleMember | SyntaxKind::RestType | SyntaxKind::OptionalType => element.type_node(),
        _ => Some(element),
    }
}

/// `declaredTypeNode`.
pub(crate) fn declared_type_node(declaration: Option<P<Node>>) -> Option<P<Node>> {
    let declaration = declaration?;
    match declaration.kind() {
        SyntaxKind::PropertySignature | SyntaxKind::PropertyDeclaration | SyntaxKind::Parameter | SyntaxKind::NamedTupleMember => {
            declaration.type_node()
        }
        _ => None,
    }
}

trait TypeArgumentsLen {
    fn type_arguments_len(&self) -> Option<usize>;
}

impl TypeArgumentsLen for P<Node> {
    fn type_arguments_len(&self) -> Option<usize> {
        self.type_argument_list().map(|l| l.nodes().len())
    }
}
