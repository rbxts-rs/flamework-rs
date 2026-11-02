//! The `intrinsic-component-config` parameter intrinsic (the TS transformer's `macros/intrinsics/components.ts` and
//! `macros/updateComponentConfig.ts`): `@Component({...})` on a class gets the config its types imply, merged into
//! the written object, in the TS's order:
//! - `links`: the components and instances the attributes and the instance tree link to;
//! - `attributes`: a guard per attribute (F3's [`crate::guards::guard_fields_in_context`]), after the written ones,
//!   but for the attributes written by hand and those the base class already checks with the same type;
//! - `instanceShape` (the instance tree as data, [`super::shape`]) or `instanceGuard` (a `t` guard) when the
//!   instance type is narrower than the base class's and no `instanceGuard` is written.
//!
//! The TS rebuilt the object with `links` and then `attributes` moved to the end, and roblox-ts evaluated the fields
//! in that order. Here, without a written `attributes`, the written literal is compiled with [`ObjectEdits`] and the
//! generated fields appended (roblox-ts's own lowering). With one, the written member is dropped and the literal
//! compiled in the TS's emitted order ([`FieldOrder::Emitted`]): the other written fields, then `links`, then
//! `attributes` (a deferred value, built at its place, so its statements run after the fields before it), then the
//! instance's field. A method in the literal (or in a written `attributes` object) is refused as roblox-ts refused it
//! in the TS's rebuilt literal, whether or not a field moves; the compiler refuses an accessor that stays (a spread is
//! refused by the TS's checks before).

use rbxts_luau::{Expr, create};
use rbxts_plugin::ts::{Kind, Node, P, Symbol, Type};
use rbxts_plugin::{Cx, FieldOrder, LuauExpr, ObjectEdits, PResult, PluginDiagnostic, synthesized_string};
use tsrs_ast::SymbolFlags;
use tsrs_checker::{LiteralValue, TypeFlags};

use super::shape::build_instance_shape;
use crate::guards::types::is_instance_type;
use crate::metadata::get_symbol;
use crate::plugin::FwInstance;

/// The property every component carries, which tells a component type apart from the Instance types around it.
const COMPONENT_BRAND: &str = "_flamework_link_instance";

/// The ancestor class declaration a call is the decorator of (`ts.findAncestor(node, ts.isDecorator)` whose parent is
/// a class declaration), as the TS finds it.
pub(crate) fn decorated_class(call: P<Node>) -> Option<P<Node>> {
    let mut current = call.parent();
    while let Some(node) = current {
        if node.kind() == Kind::Decorator {
            return node.parent().filter(|parent| parent.kind() == Kind::ClassDeclaration);
        }
        current = node.parent();
    }
    None
}

/// A written field of the config literal, by the TS's test `x.name && "text" in x.name && x.name.text === name`: the
/// first field whose name is an identifier, a string or numeric literal (not a computed name) with that text.
fn written_field(literal: Option<P<Node>>, name: &str) -> Option<P<Node>> {
    literal?.properties().iter().copied().find(|&property| {
        property.name().is_some_and(|n| {
            matches!(
                n.kind(),
                Kind::Identifier
                    | Kind::PrivateIdentifier
                    | Kind::StringLiteral
                    | Kind::NumericLiteral
                    | Kind::NoSubstitutionTemplateLiteral
            ) && n.text() == name
        })
    })
}

/// A field the TS rebuilt as a property assignment with an object literal value (`f.is.propertyAssignmentDeclaration`
/// and `f.is.object`): its object literal.
fn object_initializer(field: Option<P<Node>>) -> Option<P<Node>> {
    let field = field.filter(|f| f.kind() == Kind::PropertyAssignment)?;
    field.initializer().filter(|i| i.kind() == Kind::ObjectLiteralExpression)
}

/// `transformComponentConfig` for one config parameter of `class`'s decorator: the Luau value of the argument
/// (`argument`, or an empty object when it is missing or not an object literal, which the TS replaces too).
pub(crate) fn component_config(
    fw: &FwInstance,
    cx: &mut Cx<'_>,
    class: P<Node>,
    argument: Option<P<Node>>,
) -> PResult<LuauExpr> {
    let literal = argument.filter(|a| a.kind() == Kind::ObjectLiteralExpression);
    let config = Config { fw, class, at: class.name().unwrap_or(class) };

    // Links first: they report a component the guards cannot be generated for, which reads far better than the
    // "cannot generate a guard for a class" the guards would raise.
    let links = config.links(cx)?;
    let attributes = config.attribute_guards(cx, literal)?;
    let instance = config.instance_guard(cx, literal)?;

    // The written `attributes` field the TS moves after `links`, and the value it gets.
    let mut moved: Option<P<Node>> = None;
    let attributes = attributes.map(|attributes| {
        let field = written_field(literal, "attributes");
        moved = field;
        match object_initializer(field) {
            // `f.update.object(attributes.initializer, [...written, ...guards])`, under its written name.
            Some(object) => AttributesValue::Merged { object, guards: attributes.guards },
            // Anything else written gives way to the generated guards (`f.object(filteredGuards)`).
            None => AttributesValue::Generated(attributes.guards),
        }
    });

    let Some(literal) = literal else {
        // `f.object([])` with the generated fields.
        let mut fields = Vec::new();
        fields.extend(links.map(|links| (synthesized_string("links"), links)));
        if let Some(attributes) = attributes {
            fields.push((synthesized_string("attributes"), attributes.compile(cx)?));
        }
        fields.extend(instance.map(|(name, value)| (synthesized_string(name), value)));
        return Ok(create::map(fields));
    };
    // The TS always rebuilt the literal (and a written `attributes` object), and roblox-ts refused a method in the
    // rebuilt one wherever it was (but for the written `attributes` the TS dropped).
    let nested = moved.and_then(|moved| object_initializer(Some(moved)));
    let mut abort = refuse_methods(cx, literal, moved);
    if let Some(nested) = nested {
        abort = abort.or(refuse_methods(cx, nested, None));
    }
    // TypeScript refused the rebuilt literal when a written field has the name of a generated one (TS1117).
    let mut generated: Vec<String> = Vec::new();
    generated.extend(links.as_ref().map(|_| "links".to_owned()));
    generated.extend(attributes.as_ref().map(|_| "attributes".to_owned()));
    generated.extend(instance.as_ref().map(|(name, _)| (*name).to_owned()));
    abort = abort.or(refuse_duplicates(cx, literal, moved, &generated));
    if let (Some(nested), Some(AttributesValue::Merged { guards, .. })) = (nested, &attributes) {
        let names: Vec<String> = guards
            .iter()
            .filter_map(|(key, _)| if let Expr::String(s) = key { Some(s.value.clone()) } else { None })
            .collect();
        abort = abort.or(refuse_duplicates(cx, nested, None, &names));
    }
    if let Some(abort) = abort {
        return Err(abort);
    }
    let mut edits = ObjectEdits::default();
    let Some(moved) = moved else {
        // The written fields as written, then `links`, `attributes` and the instance's field (data, evaluated after
        // the written values).
        edits.append.extend(links.map(|links| (synthesized_string("links"), links)));
        if let Some(attributes) = attributes {
            edits.append.push((synthesized_string("attributes"), attributes.compile(cx)?));
        }
        edits.append.extend(instance.map(|(name, value)| (synthesized_string(name), value)));
        return cx.compile_object_literal(literal, edits);
    };
    let attributes = attributes.expect("a moved field has a value");
    let properties = literal.properties();

    // The moved field keeps its written name (`f.update.propertyAssignmentDeclaration(..., attributes.name)`); one that
    // was not an object literal is rebuilt as `attributes`.
    let key = match moved.name() {
        Some(name) if matches!(attributes, AttributesValue::Merged { .. }) && name.kind() == Kind::StringLiteral => {
            cx.compile_expr(name)?
        }
        Some(name) if matches!(attributes, AttributesValue::Merged { .. }) && name.kind() == Kind::Identifier => {
            create::string(name.text())
        }
        _ => synthesized_string("attributes"),
    };
    // The TS's emitted order: the written members but the moved one, `links`, `attributes`, the instance's field.
    let written = u32::try_from(properties.len()).expect("a literal's member count fits in u32");
    let mut order: Vec<u32> = (0..written).filter(|&i| properties[i as usize] != moved).collect();
    edits.drop.push(moved);
    // The list is the written members, then `append` (`links`, the instance's field), then `append_deferred`.
    let mut next = written;
    if let Some(links) = links {
        edits.append.push((synthesized_string("links"), links));
        order.push(next);
        next += 1;
    }
    let instance_index = instance.map(|(name, value)| {
        edits.append.push((synthesized_string(name), value));
        next += 1;
        next - 1
    });
    order.push(next);
    order.extend(instance_index);
    edits.append_deferred.push((key, 0));
    edits.order = FieldOrder::Emitted(order);
    let mut attributes = Some(attributes);
    cx.compile_object_literal_with(literal, edits, |cx, _| {
        attributes.take().expect("one deferred value, built once").compile(cx)
    })
}

/// The methods of a config literal: roblox-ts refused each in the literal the TS rebuilt (`validateMethodAssignment`),
/// which it does not do for the written literal. Reported at the method, where TS 5 gives a position in its reprinted
/// literal.
fn refuse_methods(cx: &mut Cx<'_>, literal: P<Node>, dropped: Option<P<Node>>) -> Option<rbxts_plugin::Abort> {
    let mut abort = None;
    for &property in literal.properties() {
        if property.kind() == Kind::MethodDeclaration && Some(property) != dropped {
            let message = "Attempted to assign method where non-method was expected.";
            let diagnostic = PluginDiagnostic::error(property, message).with_source("roblox-ts");
            abort = abort.or(cx.report(diagnostic));
        }
    }
    abort
}

/// The written fields (but `dropped`) of a literal the TS rebuilt that have the name of a field it added: TypeScript's
/// TS1117 for the rebuilt literal, reported at the written name (TS 5 gives a position in its reprinted literal).
fn refuse_duplicates(
    cx: &mut Cx<'_>,
    literal: P<Node>,
    dropped: Option<P<Node>>,
    added: &[String],
) -> Option<rbxts_plugin::Abort> {
    let mut abort = None;
    for &property in literal.properties() {
        let Some(name) = property.name().filter(|_| Some(property) != dropped) else { continue };
        if effective_name(cx, name).is_some_and(|text| added.contains(&text)) {
            let message = "An object literal cannot have multiple properties with the same name.";
            abort = abort.or(cx.report(PluginDiagnostic::error(name, message).with_code(1117)));
        }
    }
    abort
}

/// The name a property name gives the field: its text, or a computed name's literal (type).
fn effective_name(cx: &mut Cx<'_>, name: P<Node>) -> Option<String> {
    if name.kind() != Kind::ComputedPropertyName {
        return Some(name.text().to_owned());
    }
    let expression = name.expression()?;
    if matches!(expression.kind(), Kind::StringLiteral | Kind::NoSubstitutionTemplateLiteral | Kind::NumericLiteral) {
        return Some(expression.text().to_owned());
    }
    let ty = cx.checker().get_type_at_location(expression);
    if !ty.flags().intersects(TypeFlags::StringLiteral | TypeFlags::NumberLiteral) {
        return None;
    }
    match ty.as_literal_type().value() {
        Some(LiteralValue::String(text)) => Some(text.to_owned()),
        Some(LiteralValue::Number(n)) => Some(rbxts_luau::js_number_to_string(n.0)),
        _ => None,
    }
}

/// The value of `attributes`, compiled where the TS emits it.
enum AttributesValue {
    Merged { object: P<Node>, guards: Vec<(LuauExpr, LuauExpr)> },
    Generated(Vec<(LuauExpr, LuauExpr)>),
}

impl AttributesValue {
    fn compile(self, cx: &mut Cx<'_>) -> PResult<LuauExpr> {
        match self {
            AttributesValue::Merged { object, guards } => {
                cx.compile_object_literal(object, ObjectEdits { append: guards, ..ObjectEdits::default() })
            }
            AttributesValue::Generated(guards) => Ok(create::map(guards)),
        }
    }
}

/// The generated attribute guards.
struct AttributeGuards {
    guards: Vec<(LuauExpr, LuauExpr)>,
}

struct Config<'f> {
    fw: &'f FwInstance,
    class: P<Node>,
    /// `node.name ?? node`.
    at: P<Node>,
}

impl Config<'_> {
    fn class_type(&self, cx: &mut Cx<'_>) -> P<Type> {
        cx.checker().get_type_at_location(self.class)
    }

    /// `getMarkedType`: one of `BaseComponent`'s type parameters, read through the property that carries it, so a
    /// subclass gets the instantiated type and an unrelated class nothing.
    fn marked_type(&self, cx: &mut Cx<'_>, property: &str, marker: &str) -> Option<P<Type>> {
        let ty = self.class_type(cx);
        let symbol = cx.checker().get_property_of_type_exported(ty, property)?;
        let declaration = symbol.value_declaration()?;
        if !self.fw.metadata(cx, declaration).is_requested(marker) {
            return None;
        }
        cx.checker().get_type_of_symbol_at_location(symbol, Some(self.class))
    }

    /// `getSuperClasses(...)[0]`: the class declaration the class extends.
    fn super_class(&self, cx: &mut Cx<'_>) -> Option<P<Node>> {
        super_class_of(cx, self.class)
    }

    // --- links ---

    /// `updateLinks`: the components and instances the attributes and the instance tree link to.
    fn links(&self, cx: &mut Cx<'_>) -> PResult<Option<LuauExpr>> {
        let attributes_type =
            self.marked_type(cx, "_flamework_link_attributes", "intrinsic-component-attribute-links");
        let instance_type = self.marked_type(cx, COMPONENT_BRAND, "intrinsic-component-instance-links");
        if attributes_type.is_none() && instance_type.is_none() {
            return Ok(None);
        }

        let mut links = Vec::new();
        if let Some(attributes_type) = attributes_type {
            for property in cx.ts5_properties_of_type(attributes_type) {
                let Some(declared) = cx.checker().get_type_of_symbol_at_location(property, Some(self.class)) else {
                    continue;
                };
                let target = cx.checker().get_non_nullable_type(declared);
                let optional = is_optional_member(cx, property, declared);
                if is_component_type(cx, target) {
                    // No guard or shape of its own: the instance a component link names has to carry that
                    // component, and that component's tracker is what checks its tree.
                    links.push(self.link(cx, "attribute", property.name(), optional, Some(target), None)?);
                } else if is_instance_type(cx, target) {
                    links.push(self.link(cx, "attribute", property.name(), optional, None, Some(target))?);
                }
            }
        }

        if let Some(instance_type) = instance_type {
            for (property, declared) in self.declared_children(cx, instance_type)? {
                let target = cx.checker().get_non_nullable_type(declared);
                // A child that may be missing is a child `this.instance` cannot be indexed for.
                if is_optional_member(cx, property, declared) {
                    return Err(self.child_is_required(cx, property, property.name()));
                }
                if is_component_type(cx, target) {
                    links.push(self.link(cx, "child", property.name(), false, Some(target), None)?);
                } else if is_instance_type(cx, target) {
                    let mut ancestors = vec![instance_type];
                    self.assert_instance_tree(cx, target, property.name(), &mut ancestors)?;
                }
            }
        }

        Ok((!links.is_empty()).then(|| create::array(links)))
    }

    /// `createLink`.
    fn link(
        &self,
        cx: &mut Cx<'_>,
        kind: &str,
        name: &str,
        optional: bool,
        component: Option<P<Type>>,
        guard_type: Option<P<Type>>,
    ) -> PResult<LuauExpr> {
        let mut fields = vec![
            (synthesized_string("kind"), synthesized_string(kind)),
            (synthesized_string("name"), synthesized_string(name)),
            (synthesized_string("optional"), create::bool(optional)),
        ];
        if let Some(guard_type) = guard_type {
            match build_instance_shape(cx, guard_type, &[])? {
                Some(shape) => fields.push((synthesized_string("shape"), shape)),
                None => {
                    let context = format!("Failed to generate a guard for the '{name}' link");
                    let guard = crate::guards::guard_for_in_context(
                        cx,
                        self.fw.shared.guard_config(),
                        self.at,
                        guard_type,
                        Some((self.at, context)),
                    )?;
                    fields.push((synthesized_string("guard"), guard));
                }
            }
        }
        if let Some(component) = component {
            let id = self.fw.type_uid(cx, component, self.at)?;
            fields.push((synthesized_string("component"), self.fw.id_expr(cx, id)));
        }
        Ok(create::map(fields))
    }

    /// `getDeclaredChildren`: the members of an instance type that are not part of the Roblox class itself (the
    /// children the instance must have), with their types.
    fn declared_children(&self, cx: &mut Cx<'_>, ty: P<Type>) -> PResult<Vec<(P<Symbol>, P<Type>)>> {
        let instance_type = match crate::guards::types::instance_type_of(cx, ty) {
            Ok(instance_type) => instance_type,
            Err(at) => return Err(super::shape::nominal_intersection(cx, at)),
        };
        let mut children = Vec::new();
        for property in cx.ts5_properties_of_type(ty) {
            if cx.checker().get_property_of_type_exported(instance_type, property.name()).is_some() {
                continue;
            }
            if let Some(property_type) = cx.checker().get_type_of_symbol_at_location(property, Some(self.class)) {
                children.push((property, property_type));
            }
        }
        Ok(children)
    }

    /// `getComponentChildNames`: the direct children of the declared tree that name a component.
    fn component_child_names(&self, cx: &mut Cx<'_>) -> PResult<Vec<String>> {
        let mut names = Vec::new();
        let Some(instance_type) = self.marked_type(cx, COMPONENT_BRAND, "intrinsic-component-instance-links") else {
            return Ok(names);
        };
        for (property, declared) in self.declared_children(cx, instance_type)? {
            let target = cx.checker().get_non_nullable_type(declared);
            if is_component_type(cx, target) && !names.iter().any(|n| n == property.name()) {
                names.push(property.name().to_owned());
            }
        }
        Ok(names)
    }

    /// `assertChildIsRequired`.
    fn child_is_required(&self, cx: &mut Cx<'_>, property: P<Symbol>, path: &str) -> rbxts_plugin::Abort {
        let component = self.class.name().map_or(String::new(), |n| format!(" of '{}'", n.text()));
        let diagnostic = PluginDiagnostic::error(
            self.property_node(property),
            format!(
                "Child '{path}' of the instance tree{component} is optional, which Flamework does not allow: \
                Roblox raises when a child that does not exist is indexed, so 'this.instance.{path}' would error \
                rather than be undefined."
            ),
        )
        .with_hint(
            "Require the child, or leave it out of the tree and reach for it with FindFirstChild; a component that \
            may be missing can be named by an optional link attribute, or looked up with getComponent.",
        );
        cx.report(diagnostic).expect("an error")
    }

    /// `getPropertyNode`: the child itself wherever it was written down, the component otherwise.
    fn property_node(&self, property: P<Symbol>) -> P<Node> {
        let Some(declaration) = property.value_declaration().or_else(|| property.declarations().first().copied())
        else {
            return self.at;
        };
        declaration.name().unwrap_or(declaration)
    }

    /// `assertInstanceTree`: the children declared below a direct child of the instance tree.
    fn assert_instance_tree(
        &self,
        cx: &mut Cx<'_>,
        ty: P<Type>,
        path: &str,
        ancestors: &mut Vec<P<Type>>,
    ) -> PResult<()> {
        let component = self.class.name().map_or(String::new(), |n| format!(" of '{}'", n.text()));
        if ancestors.contains(&ty) {
            let shown = cx.checker().type_to_string_exported(ty);
            let diagnostic = PluginDiagnostic::error(
                self.at,
                format!(
                    "Child '{path}' of the instance tree{component} has the type of the tree around it, '{shown}', \
                    so it would hold that tree again below it, without end. No instance has that tree."
                ),
            )
            .with_hint(
                "Declare the child with a type that stops, or leave it out of the tree and reach for it with \
                FindFirstChild.",
            );
            return Err(cx.report(diagnostic).expect("an error"));
        }

        ancestors.push(ty);
        for (property, declared) in self.declared_children(cx, ty)? {
            let target = cx.checker().get_non_nullable_type(declared);
            let child_path = format!("{path}.{}", property.name());
            if cx.checker().get_property_of_type_exported(target, COMPONENT_BRAND).is_some() {
                let shown = cx.checker().type_to_string_exported(target);
                let diagnostic = PluginDiagnostic::error(
                    self.at,
                    format!(
                        "Component '{shown}' is linked at '{child_path}', which is not a direct child of this \
                        component."
                    ),
                )
                .with_hint(
                    "Only a direct child of the instance tree can name a component. Declare it on the component \
                    attached to that child, or look it up with getComponent.",
                );
                return Err(cx.report(diagnostic).expect("an error"));
            }
            if is_instance_type(cx, target) {
                if is_optional_member(cx, property, declared) {
                    return Err(self.child_is_required(cx, property, &child_path));
                }
                self.assert_instance_tree(cx, target, &child_path, ancestors)?;
            }
        }
        ancestors.pop();
        Ok(())
    }

    // --- attributes ---

    /// `updateAttributeGuards`: a guard per attribute, from the written shape of the attributes (an instance-valued
    /// attribute is stored as an `InstanceHandle`, so that is what is checked), but for the omitted ones.
    fn attribute_guards(&self, cx: &mut Cx<'_>, literal: Option<P<Node>>) -> PResult<Option<AttributeGuards>> {
        let Some(attributes_type) =
            self.marked_type(cx, "_flamework_attribute_guards", "intrinsic-component-attribute-guards")
        else {
            return Ok(None);
        };
        let shown = cx.checker().type_to_string_exported(attributes_type);
        let context = format!("Failed to generate component attributes: {shown}");
        let guards = crate::guards::guard_fields_in_context(
            cx,
            self.fw.shared.guard_config(),
            self.at,
            attributes_type,
            false,
            Some((self.at, context)),
        )?;
        let written = written_field(literal, "attributes");
        let omitted = self.omitted_guards(cx, written);
        let guards = guards
            .into_iter()
            .filter(|(key, _)| !omitted.iter().any(|name| *key == synthesized_string(name)))
            .collect();
        Ok(Some(AttributeGuards { guards }))
    }

    /// `calculateOmittedGuards`: the attributes written by hand, and those the base class checks with the same type.
    fn omitted_guards(&self, cx: &mut Cx<'_>, written: Option<P<Node>>) -> Vec<String> {
        let mut omitted = Vec::new();
        if let Some(object) = object_initializer(written) {
            for &property in object.properties() {
                if let Some(name) =
                    property.name().filter(|n| matches!(n.kind(), Kind::Identifier | Kind::StringLiteral))
                {
                    omitted.push(name.text().to_owned());
                }
            }
        }

        let ty = self.class_type(cx);
        let Some(property) = cx.checker().get_property_of_type_exported(ty, "_flamework_attribute_guards") else {
            return omitted;
        };
        let Some(super_class) = self.super_class(cx) else { return omitted };
        let super_type = cx.checker().get_type_at_location(super_class);
        let Some(super_property) = cx.checker().get_property_of_type_exported(super_type, "_flamework_attribute_guards")
        else {
            return omitted;
        };
        let Some(attributes) = cx.checker().get_type_of_symbol_at_location(property, Some(self.class)) else {
            return omitted;
        };
        let Some(super_attributes) = cx.checker().get_type_of_symbol_at_location(super_property, Some(super_class))
        else {
            return omitted;
        };
        for super_member in cx.checker().get_properties_of_type(super_attributes).to_vec() {
            let name = super_member.name();
            // A symbol key or a private name is not reflected (`crate::guards::types::is_internal_property`).
            let prop = crate::guards::types::type_of_named_property(cx, attributes, super_member);
            let super_prop = crate::guards::types::type_of_named_property(cx, super_attributes, super_member);
            if prop.is_some() && prop == super_prop {
                omitted.push(name.to_owned());
            }
        }
        omitted
    }

    // --- the instance ---

    /// `updateInstanceGuard`: `instanceShape` or `instanceGuard` when the instance type is narrower than the base
    /// class's and none is written.
    fn instance_guard(&self, cx: &mut Cx<'_>, literal: Option<P<Node>>) -> PResult<Option<(&'static str, LuauExpr)>> {
        let ty = self.class_type(cx);
        let Some(property) = cx.checker().get_property_of_type_exported(ty, "instance") else { return Ok(None) };
        let Some(declaration) = property.value_declaration() else { return Ok(None) };
        if !self.fw.metadata(cx, declaration).is_requested("intrinsic-component-instance") {
            return Ok(None);
        }
        let Some(super_class) = self.super_class(cx) else { return Ok(None) };
        if written_field(literal, "instanceGuard").is_some() {
            return Ok(None);
        }
        let Some(instance_type) = cx.checker().get_type_of_symbol_at_location(property, Some(self.class)) else {
            return Ok(None);
        };
        let super_type = cx.checker().get_type_at_location(super_class);
        let Some(super_property) = cx.checker().get_property_of_type_exported(super_type, "instance") else {
            return Ok(None);
        };
        let Some(super_instance_type) = cx.checker().get_type_of_symbol_at_location(super_property, Some(super_class))
        else {
            return Ok(None);
        };
        if cx.checker().is_type_assignable_to_exported(super_instance_type, instance_type) {
            return Ok(None);
        }

        // As data wherever the type is only classes and children; as a guard otherwise. A child naming a component
        // has a tree of its own that is that component's business, so the shape stops at its class.
        let component_children = self.component_child_names(cx)?;
        if let Some(shape) = build_instance_shape(cx, instance_type, &component_children)? {
            return Ok(Some(("instanceShape", shape)));
        }
        let guard = crate::guards::guard_for(cx, self.fw.shared.guard_config(), self.class, instance_type)?;
        Ok(Some(("instanceGuard", guard)))
    }
}

/// `getComponentInstanceType(...) !== undefined`: the type is a component (it has the brand and an `instance`).
fn is_component_type(cx: &mut Cx<'_>, ty: P<Type>) -> bool {
    cx.checker().get_property_of_type_exported(ty, COMPONENT_BRAND).is_some()
        && cx.checker().get_property_of_type_exported(ty, "instance").is_some()
}

/// `isOptionalMember`.
fn is_optional_member(cx: &mut Cx<'_>, symbol: P<Symbol>, ty: P<Type>) -> bool {
    symbol.flags().intersects(SymbolFlags::Optional) || cx.checker().get_non_nullable_type(ty) != ty
}

/// `getSuperClasses(checker, node)[0]`: the class declaration of the `extends` clause's expression, aliases skipped.
pub(crate) fn super_class_of(cx: &mut Cx<'_>, class: P<Node>) -> Option<P<Node>> {
    let clauses = class.class_like_data().and_then(|d| d.heritage_clauses())?;
    let mut expression = None;
    for &clause in clauses.nodes() {
        let heritage = clause.as_heritage_clause();
        if heritage.token == Kind::ExtendsKeyword {
            expression = heritage.types.get().nodes().first().and_then(|t| t.expression());
            break;
        }
    }
    let symbol = get_symbol(cx, expression?)?;
    symbol.declarations().iter().copied().find(|d| d.kind() == Kind::ClassDeclaration)
}
