//! Networking (`@flamework-experimental/networking`): the TS transformer's `transformNetworkingCall.ts`,
//! `util/functions/networkMode.ts` and `macros/intrinsics/networking.ts`.
//!
//! - **Call sites** ([`calls`]): a call that sends over a packed networking member (`fire`, `except`, `broadcast`,
//!   `invoke`, `invokeWithTimeout`, or the member called itself) packs its values where it is made and goes to the
//!   member's hidden `_fire` / `_except` / … entry point; `setCallback(cb)` registers the callback with a packer for
//!   its result (`_setCallback(cb, pack)`). The calls are claimed by the receiver's or callee's type (the
//!   `_flamework_packing` marker every handler member carries), then checked as the TS checks them.
//! - **Network declarations**: the `network-decoder` / `network-result-decoder` intrinsics (a decoder per packed
//!   member, [`crate::serializer::build_decoder`]), `network-unreliable`, and `intrinsic-middleware` ([`middleware`]),
//!   all reached through user macros ([`crate::macros`]).
//! - **Modes** ([`mode`]): how a member's markers and `networking.serialization` decide whether it is packed.
//! - **`explainRemovedNetworkMarker`** ([`mode::explain_removed_marker`]): TypeScript's error for a marker dropped before
//!   beta, as one naming the marker that replaces it.

pub(crate) mod analysis;
pub(crate) mod calls;
pub(crate) mod middleware;
pub(crate) mod mode;
pub(crate) mod scope;

use rbxts_plugin::{Interest, Tag};

/// The tag of a call whose receiver carries a networking member's marker (`x.fire(…)`, `x.setCallback(…)`).
pub const TAG_RECEIVER: Tag = Tag(10);
/// The tag of a call whose callee carries it (`handler.event(…)`).
pub const TAG_CALLEE: Tag = Tag(11);

/// The marker every handler member carries, raw ones too (`_flamework_packing`): claiming by it lets a call whose
/// target may be a raw or a packed member reach the TS's check of mixed targets.
pub const PACKING_MARKER: &str = "_flamework_packing";

/// The methods a call site packs for, and `setCallback`.
pub const METHODS: [&str; 6] = ["fire", "except", "broadcast", "invoke", "invokeWithTimeout", "setCallback"];

/// Registers the networking call interests.
pub(crate) fn interest(interest: &mut Interest) {
    interest.calls_by_receiver(&METHODS, PACKING_MARKER, TAG_RECEIVER);
    interest.calls_by_callee_marker(PACKING_MARKER, TAG_CALLEE);
}
