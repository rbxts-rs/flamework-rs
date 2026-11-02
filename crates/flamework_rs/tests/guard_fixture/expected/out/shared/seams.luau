-- Compiled with roblox-ts v3.0.0
local TS = require(game:GetService("ReplicatedStorage"):WaitForChild("rbxts_include"):WaitForChild("RuntimeLib"))
local t = TS.import(script, game:GetService("ReplicatedStorage"), "rbxts_include", "node_modules", "@rbxts", "t", "lib", "ts").t
local Flamework = TS.import(script, game:GetService("ReplicatedStorage"), "rbxts_include", "node_modules", "@flamework-experimental", "core", "out").Flamework
-- Strings the TS builder makes with `f.string` (patterns, literals, keys) print as TypeScript's printer writes a
-- synthesized literal: non-ASCII as `\u{…}`, quotes and control characters escaped by name.
local template = Flamework.createGuard(t.match("^\u{00E9}%-.*%-\u{2713}$"))
local astral = Flamework.createGuard(t.match("^\u{1f600}.*$"))
local literal = Flamework.createGuard(t.literal("na\u{00EF}ve"))
local controls = Flamework.createGuard(t.literal("\v\f\b\0"))
local keys = Flamework.createGuard(t.interface({
	["na\u{00EF}ve"] = t.number,
	["\u{65E5}\u{672C}"] = t.string,
	["a\"b"] = t.boolean,
}))
local childNames = Flamework.createGuard(t.intersection(t.instanceIsA("Model"), t.children({
	["\u{00DC}n\u{00EF}"] = t.instanceIsA("Part"),
})))
-- Properties in TypeScript 7.1's order: a derived interface's own members first, then the inherited ones; a mapped
-- type over it and a spread follow the declarations.
local inherited = Flamework.createGuard(t.interface({
	own = t.boolean,
	shared = t.number,
	base = t.string,
}))
local inheritedChildren = Flamework.createGuard(t.intersection(t.instanceIsA("Model"), t.children({
	Alpha = t.instanceIsA("Part"),
	Zeta = t.instanceIsA("Folder"),
})))
local mapped = Flamework.createGuard(t.interface({
	base = t.optional(t.string),
	own = t.optional(t.boolean),
	shared = t.optional(t.number),
}))
local _object = table.clone({
	q = 1,
	p = "x",
})
setmetatable(_object, nil)
_object.a = true
_object.q = 2
local spreadSource = _object
local spread = Flamework.createGuard(t.interface({
	p = t.string,
	a = t.boolean,
	q = t.number,
}))
local Readonly_1 = t.interface({
	base = t.string,
	own = t.boolean,
	shared = t.number,
})
local dedupKeys = Flamework.createGuard(t.interface({
	z = Readonly_1,
	y = Readonly_1,
	x = Readonly_1,
}))
return {
	template = template,
	astral = astral,
	literal = literal,
	controls = controls,
	keys = keys,
	childNames = childNames,
	inherited = inherited,
	inheritedChildren = inheritedChildren,
	mapped = mapped,
	spread = spread,
	dedupKeys = dedupKeys,
}
