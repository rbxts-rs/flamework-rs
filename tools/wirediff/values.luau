--!nolint
--[[
	The value side of the wire-diff harness: the layout parser, the seeded value generator and the invalid-value
	mutations.

	A harvested module carries its type as the serializer's wire key (`wireKey(shape, stack, false)`: the layout text
	plus ` checked` on implicit widths and the `typeof` name of a blob), which is enough to build values of the type:

		u8 .. f64 [checked] | varint [checked] | boolean | string(v|u8|u16|u32) [checked] | buffer(...) [checked]
		=<literal> | literals(<literal>, ...) | nothing | blob[(<typeof>)] | datatype(<Name>) | cframe | enum(<Name>)
		optional(k) | array(k) | set(k) | map(k, k) | list(k, ...[; ...k][; k, ...]) | object(<key>: k, ...)
		union(k, ...[; whole N]) | ^N

	`^N` names the N-th enclosing shape (`^1` the one whose text holds it). A literal is a JSON string, a JavaScript
	number, `true`, `false`, `undefined` or `Enum.<Enum>.<Item>`; anything else (`#80`, a TS enum member) is kept as
	unknown, and a type holding one is generated without it where it can be.
]]

local Values = {}

--- The number of values of each generated list (a list may end in nils), by the list.
Values.counts = setmetatable({}, { __mode = "k" }) :: { [any]: number }

-- --- seeded randomness ------------------------------------------------------------------------------------------

--- A xorshift32 generator. Every pair gets its own, seeded from the run's seed and the pair's path, so results do
--- not depend on how pairs are split between workers.
function Values.rng(seed: number)
	local s = seed % 4294967296
	if s == 0 then
		s = 2463534242
	end
	local r = {}
	function r.u32(): number
		s = bit32.bxor(s, bit32.lshift(s, 13))
		s = bit32.bxor(s, bit32.rshift(s, 17))
		s = bit32.bxor(s, bit32.lshift(s, 5))
		return s
	end
	function r.float(): number
		return r.u32() / 4294967296
	end
	--- An integer in [a, b], for spans up to 2^53.
	function r.int(a: number, b: number): number
		local span = b - a + 1
		if span <= 4294967296 then
			return a + r.u32() % span
		end
		local x = (r.u32() % 2097152) * 4294967296 + r.u32()
		return a + math.floor(x / 9007199254740992 * span)
	end
	function r.chance(p: number): boolean
		return r.float() < p
	end
	function r.pick(list: { any }): any
		return list[r.u32() % #list + 1]
	end
	return r
end

--- FNV-1a of a string, for per-pair seeds.
function Values.hash(text: string, seed: number): number
	local h = bit32.bxor(2166136261, seed % 4294967296)
	for i = 1, #text do
		h = bit32.bxor(h, string.byte(text, i))
		-- h * 16777619 mod 2^32, without losing bits to doubles.
		h = (bit32.lshift(h, 24) + h * 403) % 4294967296
	end
	return h
end

-- --- the parser ---------------------------------------------------------------------------------------------------

local NUMBER_WIDTHS = {
	u8 = { 0, 255, int = true },
	i8 = { -128, 127, int = true },
	u16 = { 0, 65535, int = true },
	i16 = { -32768, 32767, int = true },
	u32 = { 0, 4294967295, int = true },
	i32 = { -2147483648, 2147483647, int = true },
	u64 = { 0, 2 ^ 53, int = true },
	i64 = { -(2 ^ 53), 2 ^ 53, int = true },
	f32 = { -3.4028234663852886e38, 3.4028234663852886e38 },
	f64 = { -math.huge, math.huge },
}
Values.NUMBER_WIDTHS = NUMBER_WIDTHS

local LENGTH_MAX = { v = 4294967295, u8 = 255, u16 = 65535, u32 = 4294967295 }

local function decodeJsonString(text: string, at: number): (string, number)
	-- at: the opening quote
	local parts = {}
	local i = at + 1
	while true do
		local c = string.sub(text, i, i)
		if c == "" then
			error("unterminated string")
		elseif c == '"' then
			return table.concat(parts), i + 1
		elseif c == "\\" then
			local e = string.sub(text, i + 1, i + 1)
			if e == "u" then
				local code = tonumber(string.sub(text, i + 2, i + 5), 16)
				i += 6
				if code >= 0xD800 and code <= 0xDBFF and string.sub(text, i, i + 1) == "\\u" then
					local low = tonumber(string.sub(text, i + 2, i + 5), 16)
					code = 0x10000 + (code - 0xD800) * 0x400 + (low - 0xDC00)
					i += 6
				end
				table.insert(parts, utf8.char(code))
			else
				local map = { b = "\b", f = "\f", n = "\n", r = "\r", t = "\t" }
				table.insert(parts, map[e] or e)
				i += 2
			end
		else
			table.insert(parts, c)
			i += 1
		end
	end
end

local function literalOf(text: string): any
	if text == "true" then
		return { ok = true, value = true }
	elseif text == "false" then
		return { ok = true, value = false }
	elseif text == "undefined" then
		return { ok = true, value = nil }
	elseif text == "Infinity" then
		return { ok = true, value = math.huge }
	elseif text == "-Infinity" then
		return { ok = true, value = -math.huge }
	elseif text == "NaN" then
		return { ok = true, value = 0 / 0 }
	end
	local enumName, itemName = string.match(text, "^Enum%.([%w_]+)%.([%w_]+)$")
	if enumName then
		return { ok = true, enum = enumName, item = itemName }
	end
	local n = tonumber(text)
	if n ~= nil and string.match(text, "^%-?[%d%.]") then
		return { ok = true, value = n }
	end
	return { ok = false, text = text }
end

--- Parses a wire key into a tree of nodes (`{ t = "<kind>", ... }`). Raises on text it does not know.
function Values.parse(text: string): any
	local pos = 1
	local function spaces()
		while string.sub(text, pos, pos) == " " do
			pos += 1
		end
	end
	local function accept(s: string): boolean
		spaces()
		if string.sub(text, pos, pos + #s - 1) == s then
			pos += #s
			return true
		end
		return false
	end
	local function expect(s: string)
		if not accept(s) then
			error(`expected '{s}' at {pos}: {string.sub(text, pos, pos + 30)}`)
		end
	end
	local function name(): string
		spaces()
		local s = string.match(text, "^[%w_]+", pos)
		if not s then
			error(`expected a name at {pos}`)
		end
		pos += #s
		return s
	end
	local function literal(): any
		spaces()
		if string.sub(text, pos, pos) == '"' then
			local s, after = decodeJsonString(text, pos)
			pos = after
			return { ok = true, value = s }
		end
		local s = string.match(text, "^[^,%);]+", pos)
		if not s then
			error(`expected a literal at {pos}`)
		end
		pos += #s
		return literalOf((string.gsub(s, "%s+$", "")))
	end
	local function checked(node: any): any
		if accept(" checked") or accept("checked") then
			node.checked = true
		end
		return node
	end

	local node
	local function list(): { any }
		local items = {}
		spaces()
		if string.sub(text, pos, pos) == ")" or string.sub(text, pos, pos) == ";" then
			return items
		end
		repeat
			table.insert(items, node())
		until not accept(",")
		return items
	end

	function node(): any
		spaces()
		if accept("^") then
			local n = string.match(text, "^%d+", pos)
			pos += #n
			return { t = "ref", up = tonumber(n) }
		end
		if accept("=") then
			return { t = "const", value = literal() }
		end
		local word = name()
		if NUMBER_WIDTHS[word] then
			return checked({ t = "num", width = word })
		elseif word == "varint" then
			return checked({ t = "varint" })
		elseif word == "boolean" then
			return { t = "bool" }
		elseif word == "string" or word == "buffer" then
			expect("(")
			local length = name()
			expect(")")
			return checked({ t = word, length = length })
		elseif word == "literals" then
			expect("(")
			local values = {}
			repeat
				table.insert(values, literal())
			until not accept(",")
			expect(")")
			return { t = "literals", values = values }
		elseif word == "nothing" then
			return { t = "nothing" }
		elseif word == "blob" then
			if accept("(") then
				local n = name()
				expect(")")
				return { t = "blob", name = n }
			end
			return { t = "blob" }
		elseif word == "datatype" or word == "enum" then
			expect("(")
			local n = name()
			expect(")")
			return { t = word, name = n }
		elseif word == "cframe" then
			return { t = "cframe" }
		elseif word == "optional" or word == "array" or word == "set" then
			expect("(")
			local inner = node()
			expect(")")
			return { t = word, inner = inner }
		elseif word == "map" then
			expect("(")
			local k = node()
			expect(",")
			local v = node()
			expect(")")
			return { t = "map", key = k, value = v }
		elseif word == "list" then
			expect("(")
			local result = { t = "list", elements = list(), after = {} }
			if accept(";") then
				if accept("...") then
					result.rest = node()
					if accept(";") then
						result.after = list()
					end
				else
					result.after = list()
				end
			end
			expect(")")
			return result
		elseif word == "object" then
			expect("(")
			local fields = {}
			spaces()
			if string.sub(text, pos, pos) ~= ")" then
				repeat
					spaces()
					local key
					if string.sub(text, pos, pos) == '"' then
						key, pos = decodeJsonString(text, pos)
					else
						local s = string.match(text, "^[^:]+", pos)
						pos += #s
						key = tonumber(s)
					end
					expect(":")
					table.insert(fields, { key = key, node = node() })
				until not accept(",")
			end
			expect(")")
			return { t = "object", fields = fields }
		elseif word == "union" then
			expect("(")
			local result = { t = "union", members = list() }
			if accept(";") then
				expect("whole")
				local n = string.match(text, "^%s*(%d+)", pos)
				pos = select(2, string.find(text, "^%s*%d+", pos)) + 1
				result.whole = tonumber(n) + 1
			end
			expect(")")
			return result
		end
		error(`unknown layout word '{word}'`)
	end

	local root = node()
	spaces()
	if pos <= #text then
		error(`text after the layout at {pos}`)
	end
	return root
end

-- --- generation ---------------------------------------------------------------------------------------------------

--[[
	A generator context: the environment's constructors (`ctx.env`) and the rng. Nodes are generated with the stack
	of their enclosing nodes (for `^N`) and the number of back-references followed so far, bounded at
	`MAX_DEPTH`: past it, a reference takes its type's smallest value.
]]
local MAX_DEPTH = 4
Values.MAX_DEPTH = MAX_DEPTH

local function resolve(stack: { any }, up: number): (any, { any })
	local index = #stack - up + 1
	local target = stack[index]
	if target == nil then
		error(`^${up} reaches above the layout`)
	end
	return target, table.move(stack, 1, index - 1, 1, {})
end

local function push(stack: { any }, node: any): { any }
	local copy = table.move(stack, 1, #stack, 1, {})
	table.insert(copy, node)
	return copy
end

local function literalValue(ctx: any, lit: any): (boolean, any)
	if not lit.ok then
		return false, nil
	end
	if lit.enum then
		return true, ctx.env.Enum[lit.enum][lit.item]
	end
	return true, lit.value
end

local function usableKey(k: any): boolean
	return k ~= nil and k == k
end

local function bigString(n: number, byte: number): string
	return string.rep(string.char(byte), n)
end

local function numberEdges(width: string): { number }
	local w = NUMBER_WIDTHS[width]
	if width == "f32" then
		return { 0, -0, 1.5, -2.25, 3.4028234663852886e38, -3.4028234663852886e38, 1.401298464324817e-45, math.huge, -math.huge, 0 / 0 }
	elseif width == "f64" then
		return {
			0, -0, 0.1, 1.7976931348623157e308, 5e-324, math.huge, -math.huge, 0 / 0, 2 ^ 53, -(2 ^ 31),
			4294967295, 4294967296, 34359738367, 34359738368, -34359738368,
		}
	elseif width == "u64" then
		return { 0, 1, 2 ^ 32 - 1, 2 ^ 32, 2 ^ 40 + 5, 2 ^ 53 }
	elseif width == "i64" then
		return { 0, 1, -1, -2, -(2 ^ 31), -(2 ^ 32), 2 ^ 32, -(2 ^ 53), 2 ^ 53 }
	end
	local result = { w[1], w[2], 0, 1 }
	if w[1] < 0 then
		table.insert(result, -1)
	end
	if w[2] > 255 then
		table.insert(result, 255)
		table.insert(result, 256)
	else
		table.insert(result, 127)
		table.insert(result, 128)
	end
	return result
end

local VARINT_EDGES = { 0, 1, 127, 128, 16383, 16384, 2097151, 2097152, 268435455, 268435456, 4294967295, 34359738367 }

--- Empty, short, past each varint length step (127/128, 16383/16384) and each length prefix (255/256, 65536).
local function stringEdges(): { string }
	return {
		"", "a", "\0", "\xff\0é", bigString(127, 119), bigString(128, 120), bigString(255, 120), bigString(256, 121),
		bigString(16383, 118), bigString(16384, 117), bigString(65536, 122),
	}
end

--- A large count (a varint's second byte) for collections and rest lists.
local LARGE = 128

--- About how much a value weighs (elements, and bytes / 64): a large collection is made of light values only, so that
--- nested ones stay small.
local function weight(v: any): number
	local t = type(v)
	if t == "string" then
		return 1 + #v // 64
	elseif t == "buffer" then
		return 1 + buffer.len(v) // 64
	elseif t == "table" then
		local total = 1
		for k, x in v do
			total += weight(k) + weight(x)
			if total > 64 then
				return total
			end
		end
		return total
	end
	return 1
end

--- The values of `list` light enough to repeat `LARGE` times (at least one: the lightest).
local function light(list: { any }): { any }
	local result = {}
	local lightest, least = nil, math.huge
	for _, v in list do
		local w = weight(v)
		if w <= 64 then
			table.insert(result, v)
		end
		if w < least then
			lightest, least = v, w
		end
	end
	if #result == 0 and lightest ~= nil then
		table.insert(result, lightest)
	end
	return result
end

local DATATYPE_EDGES = {
	Vector3 = function(e)
		return { e.Vector3.new(0, 0, 0), e.Vector3.new(1, -2, 3.5), e.Vector3.new(1e30, -1e-30, 0) }
	end,
	Vector2 = function(e)
		return { e.Vector2.new(0, 0), e.Vector2.new(-1.5, 2) }
	end,
	Vector3int16 = function(e)
		return { e.Vector3int16.new(0, 0, 0), e.Vector3int16.new(-32768, 32767, 1) }
	end,
	Vector2int16 = function(e)
		return { e.Vector2int16.new(0, 0), e.Vector2int16.new(-32768, 32767) }
	end,
	Color3 = function(e)
		return { e.Color3.new(0, 0, 0), e.Color3.new(1, 0.5, 0.25) }
	end,
	UDim = function(e)
		return { e.UDim.new(0, 0), e.UDim.new(0.5, -10), e.UDim.new(1, 2147483647) }
	end,
	UDim2 = function(e)
		return { e.UDim2.new(0, 0, 0, 0), e.UDim2.new(0.5, 10, 1, -10) }
	end,
	NumberRange = function(e)
		return { e.NumberRange.new(0, 1), e.NumberRange.new(-5, -5) }
	end,
	Rect = function(e)
		return { e.Rect.new(0, 0, 0, 0), e.Rect.new(-1, 2.5, 3, 4) }
	end,
	BrickColor = function(e)
		return { e.BrickColor.new(1), e.BrickColor.new(194) }
	end,
}

local DATATYPE_RANDOM = {
	Vector3 = function(e, f, i)
		return e.Vector3.new(f(), f(), f())
	end,
	Vector2 = function(e, f, i)
		return e.Vector2.new(f(), f())
	end,
	Vector3int16 = function(e, f, i)
		return e.Vector3int16.new(i(), i(), i())
	end,
	Vector2int16 = function(e, f, i)
		return e.Vector2int16.new(i(), i())
	end,
	Color3 = function(e, f, i)
		return e.Color3.new(f(), f(), f())
	end,
	UDim = function(e, f, i)
		return e.UDim.new(f(), i())
	end,
	UDim2 = function(e, f, i)
		return e.UDim2.new(f(), i(), f(), i())
	end,
	NumberRange = function(e, f, i)
		local a, b = f(), f()
		return e.NumberRange.new(math.min(a, b), math.max(a, b))
	end,
	Rect = function(e, f, i)
		return e.Rect.new(f(), f(), f(), f())
	end,
	BrickColor = function(e, f, i)
		local numbers = { 1, 5, 21, 23, 26, 28, 194, 199, 1001, 1003, 1032 }
		return e.BrickColor.new(numbers[math.abs(i()) % #numbers + 1])
	end,
}

local function cframes(e: any): { any }
	return {
		e.CFrame.new(),
		e.CFrame.new(1, 2, 3),
		e.CFrame.new(-1e6, 0.5, 1e6) * e.CFrame.Angles(0.5, 1, -0.25),
	}
end

local function blobValues(ctx: any, name: string?): { any }
	local env = ctx.env
	if name == nil then
		return { env.newInstance("Folder"), 1, "blob", true }
	elseif name == "Instance" then
		return { env.newInstance("Folder"), env.newInstance("Part") }
	end
	return { env.opaqueValue(name) }
end

local function enumItems(ctx: any, name: string): { any }
	local ok, items = pcall(function()
		return ctx.env.Enum[name]:GetEnumItems()
	end)
	if not ok or #items == 0 then
		error(`no items for Enum.{name}`)
	end
	return items
end

--- The smallest value of a node: what a reference past the depth bound takes.
local function minimal(ctx: any, node: any, stack: { any }, guard: number): any
	if guard > 16 then
		error("the layout has no finite value")
	end
	local t = node.t
	if t == "num" or t == "varint" then
		return 0
	elseif t == "bool" then
		return false
	elseif t == "string" then
		return ""
	elseif t == "buffer" then
		return buffer.create(0)
	elseif t == "const" then
		local ok, v = literalValue(ctx, node.value)
		if not ok then
			error("an unknown literal")
		end
		return v
	elseif t == "literals" then
		for _, lit in node.values do
			local ok, v = literalValue(ctx, lit)
			if ok then
				return v
			end
		end
		error("only unknown literals")
	elseif t == "nothing" or t == "optional" then
		return nil
	elseif t == "blob" then
		return blobValues(ctx, node.name)[1]
	elseif t == "datatype" then
		return DATATYPE_EDGES[node.name](ctx.env)[1]
	elseif t == "cframe" then
		return ctx.env.CFrame.new()
	elseif t == "enum" then
		return enumItems(ctx, node.name)[1]
	elseif t == "array" or t == "set" or t == "map" then
		return {}
	elseif t == "list" then
		local inner = push(stack, node)
		local result = {}
		local n = 0
		for _, e in node.elements do
			n += 1
			result[n] = minimal(ctx, e, inner, guard + 1)
		end
		for _, e in node.after do
			n += 1
			result[n] = minimal(ctx, e, inner, guard + 1)
		end
		Values.counts[result] = n
		return result
	elseif t == "object" then
		local inner = push(stack, node)
		local result = {}
		for _, f in node.fields do
			result[f.key] = minimal(ctx, f.node, inner, guard + 1)
		end
		return result
	elseif t == "union" then
		local inner = push(stack, node)
		local leaves = { num = true, varint = true, bool = true, string = true, buffer = true, nothing = true, optional = true, array = true, set = true, map = true, const = true, literals = true, enum = true, blob = true, datatype = true, cframe = true }
		for _, m in node.members do
			if leaves[m.t] then
				local ok, v = pcall(minimal, ctx, m, inner, guard + 1)
				if ok then
					return v
				end
			end
		end
		return minimal(ctx, node.members[1], inner, guard + 1)
	elseif t == "ref" then
		local target, below = resolve(stack, node.up)
		return minimal(ctx, target, below, guard + 1)
	end
	error(`no minimal value for {t}`)
end

local EDGE_CAP = 512

--- `nil` in a list of edge values (a list cannot hold `nil`).
local NIL = setmetatable({}, { __tostring = function() return "nil" end })
Values.NIL = NIL
local function unbox(v: any): any
	if v == NIL then
		return nil
	end
	return v
end

local function capped(list: { any }): { any }
	if #list > EDGE_CAP then
		return table.move(list, 1, EDGE_CAP, 1, {})
	end
	return list
end

local edges
local random

--- Up to `n` distinct keys of a node, in a fixed order: `0 .. n-1` for numbers, `"k1" ..` for strings, else the
--- node's edge values and then random ones, without repeats.
local function distinctKeys(ctx: any, node: any, stack: { any }, n: number): { any }
	local t = node.t
	local result = {}
	if t == "num" or t == "varint" then
		for i = 0, n - 1 do
			table.insert(result, i)
		end
		return result
	elseif t == "string" then
		for i = 1, n do
			table.insert(result, "k" .. i)
		end
		return result
	end
	local seen = {}
	local function add(k: any)
		if #result < n and usableKey(k) and k ~= NIL and not seen[k] then
			seen[k] = true
			table.insert(result, k)
		end
	end
	local ok, list = pcall(edges, ctx, node, stack)
	if ok then
		for _, k in list do
			add(k)
		end
	end
	for _ = 1, 4 * n do
		if #result >= n then
			break
		end
		local made, k = pcall(random, ctx, node, stack, 0)
		if made then
			add(k)
		end
	end
	return result
end

--- Each child varied through its edges while the others keep their first (`build(index, value)` makes a value with
--- child `index` set to `value`).
local function varied(children: { { any } }, build: (number?, any) -> any): { any }
	local result = { build(nil, nil) }
	for i, list in children do
		for j = 2, #list do
			table.insert(result, build(i, list[j]))
			if #result >= EDGE_CAP then
				return result
			end
		end
	end
	return result
end

--- The edge values of a node: every edge per kind, each part of a structure varied in turn.
function edges(ctx: any, node: any, stack: { any }): { any }
	local t = node.t
	if t == "num" then
		return numberEdges(node.width)
	elseif t == "varint" then
		return table.clone(VARINT_EDGES)
	elseif t == "bool" then
		return { false, true }
	elseif t == "string" then
		return stringEdges()
	elseif t == "buffer" then
		local result = {}
		for _, s in stringEdges() do
			table.insert(result, buffer.fromstring(s))
		end
		return result
	elseif t == "const" or t == "literals" then
		local result = {}
		for _, lit in (if t == "const" then { node.value } else node.values) do
			local ok, v = literalValue(ctx, lit)
			if ok then
				table.insert(result, if v == nil then NIL else v)
			end
		end
		if #result == 0 then
			error("only unknown literals")
		end
		return result
	elseif t == "nothing" then
		return { NIL }
	elseif t == "blob" then
		return blobValues(ctx, node.name)
	elseif t == "datatype" then
		return DATATYPE_EDGES[node.name](ctx.env)
	elseif t == "cframe" then
		return cframes(ctx.env)
	elseif t == "enum" then
		return table.clone(enumItems(ctx, node.name))
	elseif t == "ref" then
		return { minimal(ctx, node, stack, 0) }
	end

	local inner = push(stack, node)
	if t == "optional" then
		local result = { NIL }
		for _, v in edges(ctx, node.inner, inner) do
			table.insert(result, v)
		end
		return capped(result)
	elseif t == "array" then
		local values = edges(ctx, node.inner, inner)
		local result = { {} }
		for _, v in values do
			table.insert(result, { unbox(v) })
		end
		if #values >= 2 then
			table.insert(result, { unbox(values[1]), unbox(values[2]), unbox(values[#values]) })
		end
		local cycle = light(values)
		local large = {}
		for i = 1, LARGE do
			large[i] = unbox(cycle[(i - 1) % #cycle + 1])
		end
		table.insert(result, large)
		return capped(result)
	elseif t == "set" then
		local result = { {} }
		local all = {}
		for _, v in edges(ctx, node.inner, inner) do
			if usableKey(v) and v ~= NIL then
				table.insert(result, { [v] = true })
				all[v] = true
			end
		end
		table.insert(result, all)
		local large = {}
		for _, k in distinctKeys(ctx, node.inner, inner, LARGE) do
			large[k] = true
		end
		table.insert(result, large)
		return capped(result)
	elseif t == "map" then
		local keys = edges(ctx, node.key, inner)
		local values = edges(ctx, node.value, inner)
		local result = { {} }
		for i = 1, math.max(#keys, #values) do
			local k = keys[(i - 1) % #keys + 1]
			if usableKey(k) and k ~= NIL then
				table.insert(result, { [k] = unbox(values[(i - 1) % math.max(#values, 1) + 1]) })
			end
		end
		local cycle = light(values)
		local large = {}
		for i, k in distinctKeys(ctx, node.key, inner, LARGE) do
			large[k] = unbox(cycle[(i - 1) % #cycle + 1])
		end
		table.insert(result, large)
		return capped(result)
	elseif t == "list" then
		local parts = {}
		for _, e in node.elements do
			table.insert(parts, edges(ctx, e, inner))
		end
		local restEdges = if node.rest then edges(ctx, node.rest, inner) else nil
		local afterParts = {}
		for _, e in node.after do
			table.insert(afterParts, edges(ctx, e, inner))
		end
		local function make(restValues: { any }, index: number?, value: any): any
			local result = {}
			local n = 0
			for i, list in parts do
				n += 1
				result[n] = unbox(if index == i then value else list[1])
			end
			for _, v in restValues do
				n += 1
				result[n] = unbox(v)
			end
			for i, list in afterParts do
				n += 1
				result[n] = unbox(if index == #parts + i then value else list[1])
			end
			Values.counts[result] = n
			return result
		end
		local all = table.move(afterParts, 1, #afterParts, #parts + 1, table.move(parts, 1, #parts, 1, {}))
		local result = varied(all, function(index, value)
			return make({}, index, value)
		end)
		if restEdges then
			for _, v in restEdges do
				table.insert(result, make({ v }))
			end
			if #restEdges >= 2 then
				table.insert(result, make({ restEdges[1], restEdges[2] }))
			end
			local cycle = light(restEdges)
			local large = {}
			for i = 1, LARGE do
				large[i] = cycle[(i - 1) % #cycle + 1]
			end
			table.insert(result, make(large))
		end
		return capped(result)
	elseif t == "object" then
		local parts = {}
		for _, f in node.fields do
			table.insert(parts, edges(ctx, f.node, inner))
		end
		return capped(varied(parts, function(index, value)
			local result = {}
			for i, f in node.fields do
				local v = if index == i then value else parts[i][1]
				result[f.key] = unbox(v)
			end
			return result
		end))
	elseif t == "union" then
		local result = {}
		for i, m in node.members do
			local ok, values = pcall(edges, ctx, m, inner)
			if ok then
				for _, v in values do
					table.insert(result, v)
				end
			end
			if i == node.whole then
				for _, v in { 1, -1, 127, 128, 2 ^ 31, -(2 ^ 32), 2 ^ 53, 1e20, 0.5, 4294967295, 4294967296, 34359738367, 34359738368, -34359738368 } do
					table.insert(result, v)
				end
			end
		end
		if #result == 0 then
			error("no member of the union can be generated")
		end
		return capped(result)
	end
	error(`no edges for {t}`)
end

local function randomString(r: any): string
	local n = if r.chance(0.05) then r.int(33, 300) else r.int(0, 32)
	local bytes = table.create(n)
	local ascii = r.chance(0.5)
	for i = 1, n do
		bytes[i] = if ascii then r.int(32, 126) else r.int(0, 255)
	end
	return if n == 0 then "" else string.char(table.unpack(bytes))
end

local function randomNumber(r: any, width: string): number
	local w = NUMBER_WIDTHS[width]
	if w.int then
		if r.chance(0.3) then
			-- small magnitudes, where most real values are
			return math.clamp(r.int(-300, 300), w[1], w[2])
		end
		return r.int(w[1], w[2])
	end
	if r.chance(0.05) then
		return r.pick(numberEdges(width))
	end
	local mantissa = r.float() * 2 - 1
	local exponent = if width == "f32" then r.int(-40, 40) else r.int(-300, 300)
	if r.chance(0.5) then
		exponent = r.int(-8, 20)
	end
	return mantissa * 2 ^ exponent
end

local function count(r: any): number
	local n = 0
	while n < 6 and r.chance(0.55) do
		n += 1
	end
	return n
end

--- A random value of a node. `rest` forces a list's number of rest values (a call site's arity).
function random(ctx: any, node: any, stack: { any }, depth: number, rest: number?): any
	local r = ctx.r
	local t = node.t
	if t == "num" then
		return randomNumber(r, node.width)
	elseif t == "varint" then
		return if r.chance(0.5) then r.int(0, 300) else r.int(0, 2 ^ r.int(1, 35) - 1)
	elseif t == "bool" then
		return r.chance(0.5)
	elseif t == "string" then
		return randomString(r)
	elseif t == "buffer" then
		return buffer.fromstring(randomString(r))
	elseif t == "const" or t == "literals" then
		return unbox(r.pick(edges(ctx, node, stack)))
	elseif t == "nothing" then
		return nil
	elseif t == "blob" then
		return r.pick(blobValues(ctx, node.name))
	elseif t == "datatype" then
		local f = function()
			return randomNumber(r, "f32") % 1e6
		end
		local i = function()
			return r.int(-32768, 32767)
		end
		return DATATYPE_RANDOM[node.name](ctx.env, f, i)
	elseif t == "cframe" then
		local e = ctx.env
		local f = function()
			return (r.float() * 2 - 1) * 1000
		end
		return e.CFrame.new(f(), f(), f()) * e.CFrame.Angles(r.float() * 6, r.float() * 6, r.float() * 6)
	elseif t == "enum" then
		return r.pick(enumItems(ctx, node.name))
	elseif t == "ref" then
		local target, below = resolve(stack, node.up)
		if depth >= MAX_DEPTH then
			return minimal(ctx, target, below, 0)
		end
		return random(ctx, target, below, depth + 1)
	end

	local inner = push(stack, node)
	if t == "optional" then
		return if r.chance(0.3) then nil else random(ctx, node.inner, inner, depth)
	elseif t == "array" then
		local result = {}
		for i = 1, count(r) do
			result[i] = random(ctx, node.inner, inner, depth)
		end
		return result
	elseif t == "set" then
		local result = {}
		for _ = 1, count(r) do
			local k = random(ctx, node.inner, inner, depth)
			if usableKey(k) then
				result[k] = true
			end
		end
		return result
	elseif t == "map" then
		local result = {}
		for _ = 1, count(r) do
			local k = random(ctx, node.key, inner, depth)
			if usableKey(k) then
				result[k] = random(ctx, node.value, inner, depth)
			end
		end
		return result
	elseif t == "list" then
		local result = {}
		local n = 0
		for _, e in node.elements do
			n += 1
			result[n] = random(ctx, e, inner, depth)
		end
		if node.rest then
			for _ = 1, rest or count(r) do
				n += 1
				result[n] = random(ctx, node.rest, inner, depth)
			end
		end
		for _, e in node.after do
			n += 1
			result[n] = random(ctx, e, inner, depth)
		end
		Values.counts[result] = n
		return result
	elseif t == "object" then
		local result = {}
		for _, f in node.fields do
			result[f.key] = random(ctx, f.node, inner, depth)
		end
		return result
	elseif t == "union" then
		for _ = 1, 8 do
			local index = r.int(1, #node.members)
			if index == node.whole and r.chance(0.5) then
				return if r.chance(0.5) then r.int(-1000, 1000) else r.int(-(2 ^ 53), 2 ^ 53)
			end
			local ok, v = pcall(random, ctx, node.members[index], inner, depth)
			if ok then
				return v
			end
		end
		return minimal(ctx, node, stack, 0)
	end
	error(`no random value for {t}`)
end

function Values.edges(ctx: any, root: any): { any }
	return capped(edges(ctx, root, {}))
end

function Values.random(ctx: any, root: any, rest: number?): any
	return random(ctx, root, {}, 0, rest)
end

-- --- invalid values -----------------------------------------------------------------------------------------------

--[[
	Mutation points of a value: every place in it with the node that describes it, and a setter. A mutation picks a
	kind (wrong type, out of width, non-integer, hole, unknown literal, missing field) and a point it applies to.
]]
local function points(node: any, value: any, stack: { any }, set: (any) -> (), out: { any }, depth: number)
	if depth > 6 then
		return
	end
	if node.t == "ref" then
		local target, below = resolve(stack, node.up)
		return points(target, value, below, set, out, depth + 1)
	end
	table.insert(out, { node = node, value = value, set = set })
	local inner = push(stack, node)
	if type(value) ~= "table" then
		if node.t == "optional" and value ~= nil then
			points(node.inner, value, inner, set, out, depth)
		elseif node.t == "union" then
			-- the member is not known; mutate the value as a leaf only
		end
		return
	end
	if node.t == "optional" then
		points(node.inner, value, inner, set, out, depth)
	elseif node.t == "array" then
		for i, v in ipairs(value) do
			points(node.inner, v, inner, function(x)
				value[i] = x
			end, out, depth)
		end
	elseif node.t == "list" then
		for i, e in node.elements do
			points(e, value[i], inner, function(x)
				value[i] = x
			end, out, depth)
		end
	elseif node.t == "object" then
		for _, f in node.fields do
			points(f.node, value[f.key], inner, function(x)
				value[f.key] = x
			end, out, depth)
		end
	elseif node.t == "map" then
		-- In a fixed order (keys by reference iterate as their addresses fall): primitive keys only, sorted.
		local keys = {}
		for k in value do
			local kind = type(k)
			if kind == "number" or kind == "string" or kind == "boolean" then
				table.insert(keys, k)
			end
		end
		table.sort(keys, function(a, b)
			if type(a) ~= type(b) then
				return type(a) < type(b)
			end
			return tostring(a) < tostring(b)
		end)
		for _, k in keys do
			points(node.value, value[k], inner, function(x)
				value[k] = x
			end, out, depth)
		end
	end
end

local MUTATIONS = { "wrongType", "outOfWidth", "nonInteger", "hole", "unknownLiteral", "missingField" }

local function mutate(r: any, kind: string, point: any): boolean
	local node, value, set = point.node, point.value, point.set
	local t = node.t
	if kind == "wrongType" then
		local kinds = { number = "x", string = 1, boolean = 0, table = "t", buffer = "b", ["nil"] = true }
		local replacement = kinds[type(value)]
		if replacement == nil then
			replacement = {}
		end
		if t == "blob" and node.name == nil then
			return false
		end
		set(replacement)
		return true
	elseif kind == "outOfWidth" then
		if t == "num" and NUMBER_WIDTHS[node.width].int then
			local w = NUMBER_WIDTHS[node.width]
			set(if r.chance(0.5) then w[2] + 1 else w[1] - 1)
			return true
		elseif t == "varint" then
			set(if r.chance(0.5) then -1 else 2 ^ 36)
			return true
		elseif (t == "string" or t == "buffer") and (node.length == "u8" or node.length == "u16") then
			local s = bigString(LENGTH_MAX[node.length] + 1, 97)
			set(if t == "string" then s else buffer.fromstring(s))
			return true
		end
		return false
	elseif kind == "nonInteger" then
		if (t == "num" and NUMBER_WIDTHS[node.width].int) or t == "varint" then
			set(r.pick({ 0.5, 1.25, -2.5, 0 / 0 }))
			return true
		end
		return false
	elseif kind == "hole" then
		if (t == "array" or t == "list") and type(value) == "table" and #value >= 2 then
			value[1] = nil
			return true
		end
		return false
	elseif kind == "unknownLiteral" then
		if t == "const" or t == "literals" then
			set(r.pick({ "__fw_unknown__", 123456.5, true }))
			return true
		end
		return false
	elseif kind == "missingField" then
		if t == "object" and type(value) == "table" then
			local required = {}
			for _, f in node.fields do
				if f.node.t ~= "optional" and f.node.t ~= "nothing" and value[f.key] ~= nil then
					table.insert(required, f.key)
				end
			end
			if #required == 0 then
				return false
			end
			value[r.pick(required)] = nil
			return true
		end
		return false
	end
	return false
end

--- An invalid value made from `value` (which it changes) by mutation `kind`, or `nil, false` when no point of the
--- value takes it. A top-level replacement comes back through `holder`.
function Values.invalid(r: any, root: any, value: any, kind: string): (any, boolean)
	local holder = { value }
	local out = {}
	points(root, value, {}, function(x)
		holder[1] = x
	end, out, 0)
	local candidates = {}
	for _, p in out do
		table.insert(candidates, p)
	end
	for _ = 1, #candidates do
		local i = r.int(1, #candidates)
		local p = candidates[i]
		if mutate(r, kind, p) then
			return holder[1], true
		end
		table.remove(candidates, i)
		if #candidates == 0 then
			break
		end
	end
	return nil, false
end

Values.MUTATIONS = MUTATIONS

-- --- the companion's guards --------------------------------------------------------------------------------------

--[[
	`__fwguard(<key>)`: whether a value is one of the key's values, by its shape. A companion encoder (a network
	decoder's, built only to make buffers for it) tests union members with this in place of a `t` guard.
]]
local function matches(ctx: any, node: any, v: any, stack: { any }, depth: number): boolean
	local t = node.t
	local typeOf = ctx.env.typeOf
	if t == "num" then
		local w = NUMBER_WIDTHS[node.width]
		return type(v) == "number" and (not w.int or (v % 1 == 0 and v >= w[1] and v <= w[2]))
	elseif t == "varint" then
		return type(v) == "number"
	elseif t == "bool" then
		return type(v) == "boolean"
	elseif t == "string" or t == "buffer" then
		return type(v) == t
	elseif t == "const" or t == "literals" then
		for _, lit in (if t == "const" then { node.value } else node.values) do
			local ok, value = literalValue(ctx, lit)
			if ok and value == v then
				return true
			end
		end
		return false
	elseif t == "nothing" then
		return v == nil
	elseif t == "blob" then
		return node.name == nil or typeOf(v) == node.name
	elseif t == "datatype" then
		return typeOf(v) == node.name
	elseif t == "cframe" then
		return typeOf(v) == "CFrame"
	elseif t == "enum" then
		return typeOf(v) == "EnumItem" and v.EnumType == ctx.env.Enum[node.name]
	elseif t == "ref" then
		if depth > 16 then
			return true
		end
		local target, below = resolve(stack, node.up)
		return matches(ctx, target, v, below, depth + 1)
	end
	local inner = push(stack, node)
	if t == "optional" then
		return v == nil or matches(ctx, node.inner, v, inner, depth)
	elseif t == "union" then
		for _, m in node.members do
			if matches(ctx, m, v, inner, depth) then
				return true
			end
		end
		return false
	end
	if typeOf(v) ~= "table" then
		return false
	end
	if t == "array" then
		for _, x in v do
			if not matches(ctx, node.inner, x, inner, depth) then
				return false
			end
		end
		return true
	elseif t == "set" then
		for k, x in v do
			if x ~= true or not matches(ctx, node.inner, k, inner, depth) then
				return false
			end
		end
		return true
	elseif t == "map" then
		for k, x in v do
			if not matches(ctx, node.key, k, inner, depth) or not matches(ctx, node.value, x, inner, depth) then
				return false
			end
		end
		return true
	elseif t == "list" then
		for i, e in node.elements do
			if not matches(ctx, e, v[i], inner, depth) then
				return false
			end
		end
		return true
	elseif t == "object" then
		for _, f in node.fields do
			if not matches(ctx, f.node, v[f.key], inner, depth) then
				return false
			end
		end
		return true
	end
	return false
end

--- The test `__fwguard(key)` gives, parsed once per key.
function Values.guard(ctx: any): (string) -> (any) -> boolean
	local cache = {}
	return function(key: string)
		local test = cache[key]
		if test == nil then
			local root = Values.parse(key)
			test = function(v: any): boolean
				return matches(ctx, root, v, {}, 0)
			end
			cache[key] = test
		end
		return test
	end
end

return Values
