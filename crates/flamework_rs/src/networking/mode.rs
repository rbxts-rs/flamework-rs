//! How a networking member's values travel (`util/functions/networkMode.ts`), and `explainRemovedNetworkMarker`.

use rbxts_plugin::ts::{Diagnostic, Kind, Node, P, Type};
use rbxts_plugin::{Cx, PResult, PluginDiagnostic};
use tsrs_ast::SymbolFlags;

use crate::packages::NETWORKING_PACKAGE;
use crate::plugin::FwInstance;

/// `NetworkMode`: `raw` (as they are, `Networking.Raw*`), `plain` (packed when `networking.serialization` is on),
/// `serialized` (packed either way, `Networking.Serialized*`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum NetworkMode {
    Raw,
    Plain,
    Serialized,
}

/// `hasNetworkMarker`: the declared member type carries one of networking's markers.
pub(crate) fn has_marker(cx: &mut Cx<'_>, member: Option<P<Type>>, marker: &str) -> bool {
    member.is_some_and(|member| cx.checker().get_property_of_type_exported(member, marker).is_some())
}

/// `getNetworkMode`: the mode a member's markers ask for. A member declared both raw and serialized is refused here,
/// wherever the transformer first meets it. `member` is the declared type (`None`, `unknown` or `any` read as plain),
/// `name` what to call it in the message.
pub(crate) fn network_mode(
    cx: &mut Cx<'_>,
    member: Option<P<Type>>,
    node: P<Node>,
    name: Option<&str>,
) -> PResult<NetworkMode> {
    let raw = has_marker(cx, member, "_flamework_raw");
    let serialized = has_marker(cx, member, "_flamework_serialized");
    if raw && serialized {
        let subject = match name {
            Some(name) => format!("The networking member '{name}'"),
            None => "This networking member".to_owned(),
        };
        let diagnostic = PluginDiagnostic::error(
            node,
            format!(
                "{subject} is declared both Raw and Serialized. Raw sends its values as they are; Serialized packs them into a buffer."
            ),
        )
        .with_hint("Keep one of the two markers.");
        return Err(cx.report(diagnostic).expect("an error"));
    }
    Ok(if raw {
        NetworkMode::Raw
    } else if serialized {
        NetworkMode::Serialized
    } else {
        NetworkMode::Plain
    })
}

/// `isPackedMode`: whether a member in this mode is packed into a buffer, given the project's
/// `networking.serialization`.
pub(crate) fn is_packed(fw: &FwInstance, mode: NetworkMode) -> bool {
    match mode {
        NetworkMode::Raw => false,
        NetworkMode::Plain => serialization_on(fw),
        NetworkMode::Serialized => true,
    }
}

/// `networking.serialization === true`.
pub(crate) fn serialization_on(fw: &FwInstance) -> bool {
    fw.shared
        .loaded
        .project
        .get("networking")
        .and_then(|networking| networking.get("serialization"))
        .is_some_and(|value| value.as_bool() == Some(true))
}

/// `getLiteralText`: a string or number literal type's text, for a member's name in messages.
pub(crate) fn literal_text(key: Option<P<Type>>) -> Option<String> {
    let key = key?;
    match crate::macro_type::literal_of(key) {
        Some(crate::macro_type::Literal::Str(text)) => Some(text),
        Some(crate::macro_type::Literal::Num(n)) => Some(crate::js::number_to_string(n)),
        _ => None,
    }
}

/// The markers dropped before beta, each with the marker that replaces it (the same type).
const REMOVED_MARKERS: [(&str, &str); 2] = [("SerializedReliable", "Serialized"), ("RawReliable", "Raw")];

/// `explainRemovedNetworkMarker`: TypeScript's error for `Networking.SerializedReliable` or `Networking.RawReliable`
/// (2694 "has no exported member", 2724 "… Did you mean …?"), as one that names the marker replacing it; `None` for
/// any other diagnostic, and where the namespace is not networking's own.
pub(crate) fn explain_removed_marker(cx: &mut Cx<'_>, diagnostic: P<Diagnostic>) -> Option<PluginDiagnostic> {
    if diagnostic.code() != 2694 && diagnostic.code() != 2724 {
        return None;
    }
    let file = diagnostic.file()?;
    let start = diagnostic.pos()?;
    let name = identifier_at(file.as_node(), start)?;
    let parent = name.parent()?;
    if parent.kind() != Kind::QualifiedName || parent.as_qualified_name().right != name {
        return None;
    }
    let (_, replacement) = REMOVED_MARKERS.iter().find(|(removed, _)| *removed == name.text())?;

    // Only networking's namespace, however it was imported or renamed.
    let left = parent.as_qualified_name().left;
    let mut namespace = cx.checker().get_symbol_at_location_exported(left)?;
    if namespace.flags().intersects(SymbolFlags::Alias) {
        namespace = cx.checker().get_aliased_symbol(namespace);
    }
    let by_networking = namespace.declarations().iter().any(|&declaration| {
        crate::class_hook::declaring_package(cx, declaration).as_deref() == Some(NETWORKING_PACKAGE)
    });
    if !by_networking {
        return None;
    }

    let written = cx.source_text(left);
    let removed = name.text();
    Some(
        PluginDiagnostic::error(
            parent,
            format!(
                "'{written}.{removed}' was removed before beta. It was the same type as '{written}.{replacement}', which replaces it."
            ),
        )
        .with_hint(format!(
            "Write '{written}.{replacement}<...>'. '{replacement}Unreliable' would make the event unreliable."
        )),
    )
}

/// The identifier whose token starts at `start` (`getTokenAtPosition`), if one does.
pub(crate) fn identifier_at(root: P<Node>, start: u32) -> Option<P<Node>> {
    let mut current = root;
    loop {
        let mut next = None;
        current.for_each_child(&mut |child| {
            let (Some(pos), Some(end)) = (child.pos().source_offset(), child.end().source_offset()) else { return false };
            if pos <= start && start < end {
                next = Some(child);
                return true;
            }
            false
        });
        match next {
            Some(child) => current = child,
            None => break,
        }
    }
    let end = current.end().source_offset()? as usize;
    (current.kind() == Kind::Identifier && end.checked_sub(current.text().len()) == Some(start as usize))
        .then_some(current)
}

/// `getNetworkMode` then `isPackedMode` for a member named by the literal type `key` (a network decoder's or
/// `network-unreliable`'s inputs); a member declared both raw and serialized is refused.
pub(crate) fn check_packed(
    fw: &FwInstance,
    cx: &mut Cx<'_>,
    member: Option<P<Type>>,
    node: P<Node>,
    key: Option<P<Type>>,
) -> PResult<bool> {
    let name = literal_text(key);
    let mode = network_mode(cx, member, node, name.as_deref())?;
    Ok(is_packed(fw, mode))
}
