-- Compiled with roblox-ts v3.0.0
local TS = require(game:GetService("ReplicatedStorage"):WaitForChild("rbxts_include"):WaitForChild("RuntimeLib"))
local t = TS.import(script, game:GetService("ReplicatedStorage"), "rbxts_include", "node_modules", "@rbxts", "t", "lib", "ts").t
local _macros = TS.import(script, game:GetService("ReplicatedStorage"), "TS", "macros")
local macros = _macros.macros
local sideEffect = _macros.sideEffect
-- o.m?.()
local _self = macros
local _id0 = _self.id
if _id0 ~= nil then
	_id0 = _id0(_self, "shared/macros@Economy")
end
local id0 = _id0
local _self_1 = macros
local _id1 = _self_1.id
if _id1 ~= nil then
	_id1 = _id1(_self_1, "shared/macros@Economy")
end
local id1 = _id1
local _self_2 = macros
local _guard2 = _self_2.guard
if _guard2 ~= nil then
	_guard2 = _guard2(_self_2, t.interface({
		value = t.number,
		tags = t.array(t.string),
	}))
end
local guard2 = _guard2
local _self_3 = macros
local _text3 = _self_3.text
if _text3 ~= nil then
	_text3 = _text3(_self_3, "Shop | undefined")
end
local text3 = _text3
local _self_4 = macros
local _dependency4 = _self_4.dependency
if _dependency4 ~= nil then
	_dependency4 = _dependency4(_self_4, {
		id = "shared/macros@Economy",
	})
end
local dependency4 = _dependency4
local _self_5 = macros
local _concise5 = _self_5.concise
if _concise5 ~= nil then
	_concise5 = _concise5(_self_5, "shared/macros@Economy")
end
local concise5 = _concise5
local _self_6 = macros
local _where6 = _self_6.where
if _where6 ~= nil then
	_where6 = _where6(_self_6, 10, 23, 16, "942ef642-1f1c-5871-aa4d-d9ce899df552", "macros.where?.()")
end
local where6 = _where6
local _self_7 = macros
local _obfuscated7 = _self_7.obfuscated
if _obfuscated7 ~= nil then
	_obfuscated7 = _obfuscated7(_self_7, "playerJoined")
end
local obfuscated7 = _obfuscated7
local _self_8 = macros
local _labels8 = _self_8.labels
if _labels8 ~= nil then
	_labels8 = _labels8(_self_8, { "name", "age" })
end
local labels8 = _labels8
local _self_9 = macros
local _emit9 = _self_9.emit
if _emit9 ~= nil then
	_emit9 = _emit9(_self_9, {
		kind = "x",
		list = { 1, 2 },
	})
end
local emit9 = _emit9
-- Flamework user macro metadata (line 14)
local constant_14 = {
	shared = true,
}
-- Flamework hoisted this macro's metadata (constant_14) to the file root.
local _self_10 = macros
local _constant10 = _self_10.constant
if _constant10 ~= nil then
	_constant10 = _constant10(_self_10, constant_14)
end
local constant10 = _constant10
local _self_11 = macros
local _path11 = _self_11.path
if _path11 ~= nil then
	_path11 = _path11(_self_11, "src/server/commands", { "ServerScriptService", "TS", "commands" })
end
local path11 = _path11
local _self_12 = macros
local _glob12 = _self_12.glob
if _glob12 ~= nil then
	_glob12 = _glob12(_self_12, "src/server/commands/*.ts", "src/server/commands/*.ts")
end
local glob12 = _glob12
local _self_13 = macros
local _env13 = _self_13.env
if _env13 ~= nil then
	_env13 = _env13(_self_13, "F_CALLS_CHANNEL", "beta")
end
local env13 = _env13
local _self_14 = macros
local _keys14 = _self_14.keys
if _keys14 ~= nil then
	_keys14 = _keys14(_self_14, {
		playerJoined = 1,
		playerLeft = 2,
	})
end
local keys14 = _keys14
local _self_15 = macros
local _shuffled15 = _self_15.shuffled
if _shuffled15 ~= nil then
	_shuffled15 = _shuffled15(_self_15, { "a", "b", "c" })
end
local shuffled15 = _shuffled15
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
local _self_16 = macros
local _fieldInfo16 = _self_16.fieldInfo
if _fieldInfo16 ~= nil then
	_fieldInfo16 = _fieldInfo16(_self_16, fieldInfo_1)
end
local fieldInfo16 = _fieldInfo16
local _self_17 = macros
local _tupleGuards17 = _self_17.tupleGuards
if _tupleGuards17 ~= nil then
	_tupleGuards17 = _tupleGuards17(_self_17, { { t.string, t.number }, nil })
end
local tupleGuards17 = _tupleGuards17
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
local _self_18 = macros
local _serializer18 = _self_18.serializer
if _serializer18 ~= nil then
	_serializer18 = _serializer18(_self_18, {
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
end
local serializer18 = _serializer18
local _self_19 = macros
local _second19 = _self_19.second
if _second19 ~= nil then
	_second19 = _second19(_self_19, nil, "shared/macros@Economy")
end
local second19 = _second19
local _self_20 = macros
local _second20 = _self_20.second
if _second20 ~= nil then
	_second20 = _second20(_self_20, sideEffect(), "shared/macros@Economy")
end
local second20 = _second20
local _self_21 = macros
local _withOptions21 = _self_21.withOptions
if _withOptions21 ~= nil then
	_withOptions21 = _withOptions21(_self_21, {
		name = "x",
	}, "shared/macros@Economy")
end
local withOptions21 = _withOptions21
-- Flamework user macro metadata (line 31)
local constant_31 = {
	hoisted = 1,
}
local function statements()
	local _self_22 = macros
	local _result = _self_22.where
	if _result ~= nil then
		_result(_self_22, 28, 2, 16, "edd57d26-7ef3-5475-aee7-97bbb72a67a0", "macros.where?.()")
	end
	local _self_23 = macros
	local _result_1 = _self_23.second
	if _result_1 ~= nil then
		_result_1(_self_23, nil, "shared/macros@Economy")
	end
	local _self_24 = macros
	local _result_2 = _self_24.second
	if _result_2 ~= nil then
		_result_2(_self_24, sideEffect(), "shared/macros@Economy")
	end
	-- Flamework hoisted this macro's metadata (constant_31) to the file root.
	local _self_25 = macros
	local _result_3 = _self_25.constant
	if _result_3 ~= nil then
		_result_3 = _result_3(_self_25, constant_31)
	end
	return _result_3
end
return {
	statements = statements,
	id0 = id0,
	id1 = id1,
	guard2 = guard2,
	text3 = text3,
	dependency4 = dependency4,
	concise5 = concise5,
	where6 = where6,
	obfuscated7 = obfuscated7,
	labels8 = labels8,
	emit9 = emit9,
	constant10 = constant10,
	path11 = path11,
	glob12 = glob12,
	env13 = env13,
	keys14 = keys14,
	shuffled15 = shuffled15,
	fieldInfo16 = fieldInfo16,
	tupleGuards17 = tupleGuards17,
	serializer18 = serializer18,
	second19 = second19,
	second20 = second20,
	withOptions21 = withOptions21,
}
