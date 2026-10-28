//! Components (F6): what the TS transformer does for `@flamework-experimental/components`, and the key obfuscation
//! networking's remotes use.
//!
//! - [`config`]: the `intrinsic-component-config` parameter intrinsic (`@Component({...})` gets its `links`, attribute
//!   guards and `instanceShape` / `instanceGuard`), called from the user-macro hook ([`crate::macros`]);
//! - [`shape`]: instance shapes (`buildInstanceShape`);
//! - [`setter`]: writes to a component's `attributes` (the `assignment` hook, [`TAG_ATTRIBUTES`]);
//! - [`obfuscation`]: obfuscated keys (the `property_key_answer` hook, [`TAG_KEY_OBFUSCATION`]).
//!
//! Component ids in `links` are requested as any other id ([`crate::uid`]), so `flamework.build` records them in the
//! TS's order: after the ids of the written config argument.

pub(crate) mod config;
pub(crate) mod obfuscation;
pub(crate) mod setter;
pub mod shape;

use rbxts_plugin::ts::{P, Symbol};
use rbxts_plugin::{Cx, Tag};

pub use obfuscation::KEY_OBFUSCATION_MARKER;

use crate::plugin::FwInstance;

/// The tag of an assignment whose receiver is a component's `attributes` (`intrinsic-component-attributes`).
pub const TAG_ATTRIBUTES: Tag = Tag(2);

/// The tag of an access whose receiver's type carries [`KEY_OBFUSCATION_MARKER`].
pub const TAG_KEY_OBFUSCATION: Tag = Tag(3);

impl FwInstance {
    /// `isAttributesAccess`'s symbol test: the receiver's symbol is declared with
    /// `@metadata intrinsic-component-attributes` (`NodeMetadata.fromSymbol`, its value declaration).
    pub(crate) fn is_attributes_symbol(&self, cx: &mut Cx<'_>, symbol: P<Symbol>) -> bool {
        match symbol.value_declaration() {
            Some(declaration) => self.metadata(cx, declaration).is_requested("intrinsic-component-attributes"),
            None => false,
        }
    }
}
