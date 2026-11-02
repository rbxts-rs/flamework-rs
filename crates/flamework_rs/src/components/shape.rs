//! Instance shapes (the TS transformer's `util/functions/buildInstanceShape.ts`): the instance tree a type describes,
//! as the data `@flamework-experimental/components` checks and watches,
//! `{ isA = { "Model" }, children = { Root = { isA = { "BasePart" } } } }` for `Model & { Root: BasePart }`.
//!
//! A type that says more than classes and children has no shape (a union whose members declare children of their
//! own, a member that is not an instance, a type that is its own descendant); the caller falls back to a `t` guard.

use rbxts_luau::create;
use rbxts_plugin::ts::{Node, P, Type};
use rbxts_plugin::{Cx, LuauExpr, PResult, synthesized_string};
use tsrs_checker::TypeFlags;

use crate::guards::types::{extract_types, instance_type_of, is_instance_type};

/// An instance tree: the classes the instance may be, its children (in declaration order) and whether it may be
/// missing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstanceShape {
    pub is_a: Vec<String>,
    pub children: Option<Vec<(String, InstanceShape)>>,
    pub optional: bool,
}

/// `buildInstanceShape`: the shape of `ty` as Luau, or `None` when the type has none. `component_children` names
/// the direct children typed as a component: their own trees belong to that component, so the shape stops at their
/// class.
pub fn build_instance_shape(
    cx: &mut Cx<'_>,
    ty: P<Type>,
    component_children: &[String],
) -> PResult<Option<LuauExpr>> {
    let mut visiting = Vec::new();
    let shape = read_shape(cx, ty, false, true, Some(component_children), &mut visiting)?;
    Ok(shape.as_ref().map(emit_shape))
}

/// `readShape`.
fn read_shape(
    cx: &mut Cx<'_>,
    ty: P<Type>,
    allow_optional: bool,
    descend: bool,
    component_children: Option<&[String]>,
    visiting: &mut Vec<P<Type>>,
) -> PResult<Option<InstanceShape>> {
    let mut members = vec![ty];
    let mut optional = false;

    if ty.flags().intersects(TypeFlags::Union) {
        let (is_optional, types) = extract_types(cx, ty.types());
        if is_optional && !allow_optional {
            return Ok(None);
        }
        optional = is_optional;
        members = types;
    }
    if members.is_empty() {
        return Ok(None);
    }

    let mut is_a: Vec<String> = Vec::new();
    let mut children: Option<Vec<(String, InstanceShape)>> = None;
    let member_count = members.len();
    for member in members {
        if !is_instance_type(cx, member) {
            return Ok(None);
        }
        let instance_type = match instance_type_of(cx, member) {
            Ok(instance_type) => instance_type,
            Err(at) => return Err(nominal_intersection(cx, at)),
        };
        let Some(class_name) = instance_type.symbol().map(|s| s.name().to_owned()) else { return Ok(None) };
        if !is_a.contains(&class_name) {
            is_a.push(class_name);
        }

        if !descend {
            continue;
        }
        if visiting.contains(&member) {
            return Ok(None);
        }

        let mut declared: Vec<(String, InstanceShape)> = Vec::new();
        for property in cx.ts5_properties_of_type(member) {
            let name = property.name();
            if cx.checker().get_property_of_type_exported(instance_type, name).is_some() {
                continue;
            }
            // A symbol key or a private name is not reflected (`crate::guards::types::is_internal_property`).
            let Some(property_type) = crate::guards::types::type_of_named_property(cx, member, property) else {
                return Ok(None);
            };
            visiting.push(member);
            let descend_child = !component_children.is_some_and(|names| names.iter().any(|n| n == name));
            let child = read_shape(cx, property_type, true, descend_child, None, visiting);
            visiting.pop();
            let Some(child) = child? else { return Ok(None) };
            // `Map.set`: a name met again keeps its first place.
            match declared.iter_mut().find(|(n, _)| n == name) {
                Some(entry) => entry.1 = child,
                None => declared.push((name.to_owned(), child)),
            }
        }

        if !declared.is_empty() {
            // Which children go with which class is more than a shape says.
            if member_count > 1 {
                return Ok(None);
            }
            children = Some(declared);
        }
    }

    Ok(Some(InstanceShape { is_a, children, optional }))
}

/// `getInstanceTypeFromType`'s error: at the type's declaration, else the file.
pub(crate) fn nominal_intersection(cx: &mut Cx<'_>, at: Option<P<Node>>) -> rbxts_plugin::Abort {
    let at = at.unwrap_or_else(|| cx.file().source.get().as_node());
    cx.error(at, "Intersection between nominal types is forbidden.")
}

/// `emitShape`.
fn emit_shape(shape: &InstanceShape) -> LuauExpr {
    let mut fields = vec![(
        synthesized_string("isA"),
        create::array(shape.is_a.iter().map(|name| synthesized_string(name)).collect()),
    )];
    if let Some(children) = &shape.children {
        let children: Vec<_> =
            children.iter().map(|(name, child)| (synthesized_string(name), emit_shape(child))).collect();
        fields.push((synthesized_string("children"), create::map(children)));
    }
    if shape.optional {
        fields.push((synthesized_string("optional"), create::bool(true)));
    }
    create::map(fields)
}
