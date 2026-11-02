-- Compiled with roblox-ts v3.0.0
local TS = require(game:GetService("ReplicatedStorage"):WaitForChild("rbxts_include"):WaitForChild("RuntimeLib"))
local t = TS.import(script, game:GetService("ReplicatedStorage"), "rbxts_include", "node_modules", "@rbxts", "t", "lib", "ts").t
local Flamework = TS.import(script, game:GetService("ReplicatedStorage"), "rbxts_include", "node_modules", "@flamework-experimental", "core", "out").Flamework
local _macros = TS.import(script, game:GetService("ReplicatedStorage"), "TS", "macros")
local Macros = _macros.Macros
local macros = _macros.macros
local sideEffect = _macros.sideEffect
-- Flamework user macro metadata (line 16)
local constant_16 = {
	shared = true,
}
local fieldInfo_1 = { {
	name = "fxcoins",
	kind = "number",
	optional = false,
	readonly = false,
}, {
	name = "fxuserId",
	kind = "number",
	optional = false,
	readonly = true,
}, {
	name = "fxnickname",
	kind = "union(string|undefined)",
	optional = true,
	readonly = false,
} }
local codec = {}
local vsize = function(n)
	return if n < 128 then 1 elseif n < 16384 then 2 elseif n < 2097152 then 3 elseif n < 268435456 then 4 else 5
end
local vwrite = function(buf, o, n_1)
	while n_1 >= 128 do
		buffer.writeu8(buf, o, n_1 % 128 + 128)
		o += 1
		n_1 = math.floor(n_1 / 128)
	end
	buffer.writeu8(buf, o, n_1)
	return o + 1
end
local vread = function(buf_1, o_1)
	local n_2 = 0
	local scale = 1
	while true do
		local b_1 = buffer.readu8(buf_1, o_1)
		o_1 += 1
		n_2 += b_1 % 128 * scale
		if b_1 < 128 then
			return n_2, o_1
		end
		scale *= 128
		if scale > 268435456 then
			error("malformed payload")
		end
	end
end
codec.s_Box = function(v)
	local array = v.tags
	local n_3 = #array
	local size = vsize(n_3)
	for i = 1, n_3 do
		local item = array[i]
		if item == nil then
			error(`[Flamework] the array has no value at Box.tags[{i - 1}]`)
		end
		local length = #item
		size += vsize(length) + length
	end
	return size + 8
end
codec.w_Box = function(buf_2, o_2, v)
	buffer.writef64(buf_2, o_2, v.value)
	local array_1 = v.tags
	o_2 = vwrite(buf_2, o_2 + 8, #array_1)
	for i_1 = 1, #array_1 do
		local item_1 = array_1[i_1]
		local length_1 = #item_1
		o_2 = vwrite(buf_2, o_2, length_1)
		buffer.writestring(buf_2, o_2, item_1)
		o_2 += length_1
	end
	return o_2
end
codec.r_Box = function(buf_2, o_2)
	local value = buffer.readf64(buf_2, o_2)
	local count, o_3 = vread(buf_2, o_2 + 8)
	o_2 = o_3
	if count > buffer.len(buf_2) - o_2 then
		error("malformed payload")
	end
	local array_2 = table.create(count)
	for i_2 = 1, count do
		local length_2, o_4 = vread(buf_2, o_2)
		local text_1 = buffer.readstring(buf_2, o_4, length_2)
		o_2 = o_4 + length_2
		array_2[i_2] = text_1
	end
	local value_1 = {
		value = value,
		tags = array_2,
	}
	return value_1, o_2
end
-- Flamework user macro metadata (line 56)
local constant_56 = {
	shared = true,
}
-- super.m() and super.m?.()
local Derived
do
	local super = Macros
	Derived = setmetatable({}, {
		__tostring = function()
			return "Derived"
		end,
		__index = super,
	})
	Derived.__index = Derived
	function Derived.new(...)
		local self = setmetatable({}, Derived)
		return self:constructor(...) or self
	end
	function Derived:constructor(...)
		super.constructor(self, ...)
	end
	function Derived:plain()
		local id0 = super.id(self, "shared/macros@Economy")
		local id1 = super.id(self, "shared/macros@Economy")
		local guard2 = super.guard(self, t.interface({
			value = t.number,
			tags = t.array(t.string),
		}))
		local text3 = super.text(self, "Shop | undefined")
		local dependency4 = super.dependency(self, {
			id = "shared/macros@Economy",
		})
		local concise5 = super.concise(self, "shared/macros@Economy")
		local where6 = super.where(self, 12, 18, 13, "8157ead3-6f82-5f32-ae2c-c6390c6ecd6f", "super.where()")
		local obfuscated7 = super.obfuscated(self, "playerJoined")
		local labels8 = super.labels(self, { "name", "age" })
		local emit9 = super.emit(self, {
			kind = "x",
			list = { 1, 2 },
		})
		-- Flamework hoisted this macro's metadata (constant_16) to the file root.
		local constant10 = super.constant(self, constant_16)
		local path11 = super.path(self, "src/server/commands", { "ServerScriptService", "TS", "commands" })
		local glob12 = super.glob(self, "src/server/commands/*.ts", "src/server/commands/*.ts")
		local env13 = super.env(self, "F_CALLS_CHANNEL", "beta")
		local keys14 = super.keys(self, {
			playerJoined = 1,
			playerLeft = 2,
		})
		local shuffled15 = super.shuffled(self, { "a", "b", "c" })
		local fieldInfo16 = super.fieldInfo(self, fieldInfo_1)
		local tupleGuards17 = super.tupleGuards(self, { { t.string, t.number }, nil })
		local serializer18 = super.serializer(self, {
			serialize = function(v_1)
				local buf_3 = buffer.create(codec.s_Box(v_1))
				codec.w_Box(buf_3, 0, v_1)
				return buf_3
			end,
			deserialize = function(buf_4)
				local value_2, o_5 = codec.r_Box(buf_4, 0)
				if o_5 ~= buffer.len(buf_4) then
					error("malformed payload")
				end
				return value_2
			end,
		})
		local second19 = super.second(self, nil, "shared/macros@Economy")
		local second20 = super.second(self, sideEffect(), "shared/macros@Economy")
		local withOptions21 = super.withOptions(self, {
			name = "x",
		}, "shared/macros@Economy")
		local inlined22 = "Economy"
		local inlinedValue23 = "shared/macros@Economy"
		local inlinedValue24 = sideEffect()
		local implementsOf25 = Flamework._implements(macros, "shared/macros@Shop")
		local implementsOf26 = Flamework._implements(sideEffect(), "shared/macros@Shop")
		return { id0, id1, guard2, text3, dependency4, concise5, where6, obfuscated7, labels8, emit9, constant10, path11, glob12, env13, keys14, shuffled15, fieldInfo16, tupleGuards17, serializer18, second19, second20, withOptions21, inlined22, inlinedValue23, inlinedValue24, implementsOf25, implementsOf26 }
	end
	function Derived:plainStatements()
		super.where(self, 37, 3, 13, "5a39569b-f518-59c1-a8a0-4d56898e23c5", "super.where()")
		super.second(self, nil, "shared/macros@Economy")
		super.second(self, sideEffect(), "shared/macros@Economy")
		local _ = "Economy"
		Flamework._implements(macros, "shared/macros@Shop")
		Flamework._implements(sideEffect(), "shared/macros@Shop")
	end
	function Derived:optional()
		local _id0 = super.id
		if _id0 ~= nil then
			_id0 = _id0(self, "shared/macros@Economy")
		end
		local id0 = _id0
		local _id1 = super.id
		if _id1 ~= nil then
			_id1 = _id1(self, "shared/macros@Economy")
		end
		local id1 = _id1
		local _guard2 = super.guard
		if _guard2 ~= nil then
			_guard2 = _guard2(self, t.interface({
				value = t.number,
				tags = t.array(t.string),
			}))
		end
		local guard2 = _guard2
		local _text3 = super.text
		if _text3 ~= nil then
			_text3 = _text3(self, "Shop | undefined")
		end
		local text3 = _text3
		local _dependency4 = super.dependency
		if _dependency4 ~= nil then
			_dependency4 = _dependency4(self, {
				id = "shared/macros@Economy",
			})
		end
		local dependency4 = _dependency4
		local _concise5 = super.concise
		if _concise5 ~= nil then
			_concise5 = _concise5(self, "shared/macros@Economy")
		end
		local concise5 = _concise5
		local _where6 = super.where
		if _where6 ~= nil then
			_where6 = _where6(self, 52, 18, 15, "8157ead3-6f82-5f32-ae2c-c6390c6ecd6f", "super.where?.()")
		end
		local where6 = _where6
		local _obfuscated7 = super.obfuscated
		if _obfuscated7 ~= nil then
			_obfuscated7 = _obfuscated7(self, "playerJoined")
		end
		local obfuscated7 = _obfuscated7
		local _labels8 = super.labels
		if _labels8 ~= nil then
			_labels8 = _labels8(self, { "name", "age" })
		end
		local labels8 = _labels8
		local _emit9 = super.emit
		if _emit9 ~= nil then
			_emit9 = _emit9(self, {
				kind = "x",
				list = { 1, 2 },
			})
		end
		local emit9 = _emit9
		-- Flamework hoisted this macro's metadata (constant_56) to the file root.
		local _constant10 = super.constant
		if _constant10 ~= nil then
			_constant10 = _constant10(self, constant_56)
		end
		local constant10 = _constant10
		local _path11 = super.path
		if _path11 ~= nil then
			_path11 = _path11(self, "src/server/commands", { "ServerScriptService", "TS", "commands" })
		end
		local path11 = _path11
		local _glob12 = super.glob
		if _glob12 ~= nil then
			_glob12 = _glob12(self, "src/server/commands/*.ts", "src/server/commands/*.ts")
		end
		local glob12 = _glob12
		local _env13 = super.env
		if _env13 ~= nil then
			_env13 = _env13(self, "F_CALLS_CHANNEL", "beta")
		end
		local env13 = _env13
		local _keys14 = super.keys
		if _keys14 ~= nil then
			_keys14 = _keys14(self, {
				playerJoined = 1,
				playerLeft = 2,
			})
		end
		local keys14 = _keys14
		local _shuffled15 = super.shuffled
		if _shuffled15 ~= nil then
			_shuffled15 = _shuffled15(self, { "a", "b", "c" })
		end
		local shuffled15 = _shuffled15
		local _fieldInfo16 = super.fieldInfo
		if _fieldInfo16 ~= nil then
			_fieldInfo16 = _fieldInfo16(self, fieldInfo_1)
		end
		local fieldInfo16 = _fieldInfo16
		local _tupleGuards17 = super.tupleGuards
		if _tupleGuards17 ~= nil then
			_tupleGuards17 = _tupleGuards17(self, { { t.string, t.number }, nil })
		end
		local tupleGuards17 = _tupleGuards17
		local _serializer18 = super.serializer
		if _serializer18 ~= nil then
			_serializer18 = _serializer18(self, {
				serialize = function(v_2)
					local buf_5 = buffer.create(codec.s_Box(v_2))
					codec.w_Box(buf_5, 0, v_2)
					return buf_5
				end,
				deserialize = function(buf_6)
					local value_3, o_6 = codec.r_Box(buf_6, 0)
					if o_6 ~= buffer.len(buf_6) then
						error("malformed payload")
					end
					return value_3
				end,
			})
		end
		local serializer18 = _serializer18
		local _second19 = super.second
		if _second19 ~= nil then
			_second19 = _second19(self, nil, "shared/macros@Economy")
		end
		local second19 = _second19
		local _second20 = super.second
		if _second20 ~= nil then
			_second20 = _second20(self, sideEffect(), "shared/macros@Economy")
		end
		local second20 = _second20
		local _withOptions21 = super.withOptions
		if _withOptions21 ~= nil then
			_withOptions21 = _withOptions21(self, {
				name = "x",
			}, "shared/macros@Economy")
		end
		local withOptions21 = _withOptions21
		return { id0, id1, guard2, text3, dependency4, concise5, where6, obfuscated7, labels8, emit9, constant10, path11, glob12, env13, keys14, shuffled15, fieldInfo16, tupleGuards17, serializer18, second19, second20, withOptions21 }
	end
	function Derived:optionalStatements()
		local _result = super.where
		if _result ~= nil then
			_result(self, 72, 3, 15, "1883e97f-d10c-5371-b053-be999f74b812", "super.where?.()")
		end
		local _result_1 = super.second
		if _result_1 ~= nil then
			_result_1(self, nil, "shared/macros@Economy")
		end
		local _result_2 = super.second
		if _result_2 ~= nil then
			_result_2(self, sideEffect(), "shared/macros@Economy")
		end
	end
	function Derived:generic()
		return { super.id(self, "$tp:T"), super.text(self, "T"), "T" }
	end
end
return {
	Derived = Derived,
}
