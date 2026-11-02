-- Compiled with roblox-ts v3.0.0
local TS = require(game:GetService("ReplicatedStorage"):WaitForChild("rbxts_include"):WaitForChild("RuntimeLib"))
local _markers = TS.import(script, game:GetService("ReplicatedStorage"), "TS", "markers")
local constant = _markers.constant
local emit = _markers.emit
local fieldInfo = _markers.fieldInfo
local textOf = _markers.textOf
local where = _markers.where
-- Strings the TS transformer makes with `f.string` print as TypeScript's printer writes a synthesized literal:
-- non-ASCII as `\u{…}`, quotes and control characters escaped by name.
local typeText = textOf("\"say \\\"hi\\\" \u{2013} \u{00FC}n\u{00EF}\"")
local callerText = where(7, 27, 16, "13ea90f5-0731-5a5e-bd19-8cc00f7dd13c", "where(/* \"\u{00E9}\" */)")
local emitted = emit({
	["na\u{00EF}ve"] = "\u{00FC}n\u{00EF}",
	plain = "a\"b",
	controls = "\t\v",
})
-- Properties in TypeScript 7.1's order: a derived interface's own members first, then the inherited ones; a mapped
-- type over it follows the declarations.
local ordered = emit({
	own = 3,
	shared = 2,
	base = 1,
})
local fieldInfo_1 = { {
	name = "fxown",
	kind = "number:3",
	optional = false,
	readonly = false,
}, {
	name = "fxshared",
	kind = "number:2",
	optional = false,
	readonly = false,
}, {
	name = "fxbase",
	kind = "number:1",
	optional = false,
	readonly = false,
} }
local orderedFields = fieldInfo(fieldInfo_1)
local orderedMapped = emit({
	base = 1,
	own = 3,
	shared = 2,
})
local fieldInfo_2 = { {
	name = "fxbase",
	kind = "number:1",
	optional = false,
	readonly = false,
}, {
	name = "fxown",
	kind = "number:3",
	optional = false,
	readonly = false,
}, {
	name = "fxshared",
	kind = "number:2",
	optional = false,
	readonly = false,
} }
local orderedMappedFields = fieldInfo(fieldInfo_2)
-- Flamework user macro metadata (line 27)
local constant_27 = {
	deep = 1,
}
-- The "hoisted" comment goes ahead of the TS statement around the call (the TS's `state.prereq`).
local function conditional(flag)
	-- Flamework hoisted this macro's metadata (constant_27) to the file root.
	local value = if flag then constant(constant_27) else nil
	return value
end
-- Flamework user macro metadata (line 32)
local constant_32 = {
	deeper = 2,
}
local function shortCircuit(flag)
	-- Flamework hoisted this macro's metadata (constant_32) to the file root.
	return flag and { constant(constant_32) }
end
-- `{@link}` keys are the link's text as TypeScript 5 parses it.
-- @metadata macro {@link text  intrinsic-inline}
local function spaced(text)
	return text
end
-- @metadata macro {@link text | intrinsic-inline}
local function piped(text)
	return text
end
-- @metadata macro {@link text intrinsic-inline }
local function trailing(text)
	return text
end
local spacedText = "\"s\""
local pipedText = piped("\"p\"")
local trailingText = trailing("\"t\"")
return {
	conditional = conditional,
	shortCircuit = shortCircuit,
	spaced = spaced,
	piped = piped,
	trailing = trailing,
	typeText = typeText,
	callerText = callerText,
	emitted = emitted,
	ordered = ordered,
	orderedFields = orderedFields,
	orderedMapped = orderedMapped,
	orderedMappedFields = orderedMappedFields,
	spacedText = spacedText,
	pipedText = pipedText,
	trailingText = trailingText,
}
