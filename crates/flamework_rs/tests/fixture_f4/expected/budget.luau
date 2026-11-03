-- Compiled with roblox-ts v3.0.0
local TS = _G[script]
local Flamework = TS.import(script, TS.getModule(script, "@flamework-experimental", "core").out).Flamework
local literals = { "a1", "b1" }
local literalIndex = {
	a1 = 0,
	b1 = 1,
}
-- More literal groups and enums than the TS transformer's 16 file locals: all of them locals here.
local l1 = Flamework.createSerializer({
	serialize = function(v)
		local buf = buffer.create(1)
		local index = literalIndex[v]
		if index == nil then
			error("value is not one of the literals its type allows")
		end
		buffer.writeu8(buf, 0, index)
		return buf
	end,
	deserialize = function(buf_1)
		local literal = literals[buffer.readu8(buf_1, 0) + 1]
		if literal == nil then
			error("malformed payload")
		end
		if buffer.len(buf_1) ~= 1 then
			error("malformed payload")
		end
		return literal
	end,
})
local literals_1 = { "a2", "b2" }
local literalIndex_1 = {
	a2 = 0,
	b2 = 1,
}
local l2 = Flamework.createSerializer({
	serialize = function(v_1)
		local buf_2 = buffer.create(1)
		local index_1 = literalIndex_1[v_1]
		if index_1 == nil then
			error("value is not one of the literals its type allows")
		end
		buffer.writeu8(buf_2, 0, index_1)
		return buf_2
	end,
	deserialize = function(buf_3)
		local literal_1 = literals_1[buffer.readu8(buf_3, 0) + 1]
		if literal_1 == nil then
			error("malformed payload")
		end
		if buffer.len(buf_3) ~= 1 then
			error("malformed payload")
		end
		return literal_1
	end,
})
local literals_2 = { "a3", "b3" }
local literalIndex_2 = {
	a3 = 0,
	b3 = 1,
}
local l3 = Flamework.createSerializer({
	serialize = function(v_2)
		local buf_4 = buffer.create(1)
		local index_2 = literalIndex_2[v_2]
		if index_2 == nil then
			error("value is not one of the literals its type allows")
		end
		buffer.writeu8(buf_4, 0, index_2)
		return buf_4
	end,
	deserialize = function(buf_5)
		local literal_2 = literals_2[buffer.readu8(buf_5, 0) + 1]
		if literal_2 == nil then
			error("malformed payload")
		end
		if buffer.len(buf_5) ~= 1 then
			error("malformed payload")
		end
		return literal_2
	end,
})
local literals_3 = { "a4", "b4" }
local literalIndex_3 = {
	a4 = 0,
	b4 = 1,
}
local l4 = Flamework.createSerializer({
	serialize = function(v_3)
		local buf_6 = buffer.create(1)
		local index_3 = literalIndex_3[v_3]
		if index_3 == nil then
			error("value is not one of the literals its type allows")
		end
		buffer.writeu8(buf_6, 0, index_3)
		return buf_6
	end,
	deserialize = function(buf_7)
		local literal_3 = literals_3[buffer.readu8(buf_7, 0) + 1]
		if literal_3 == nil then
			error("malformed payload")
		end
		if buffer.len(buf_7) ~= 1 then
			error("malformed payload")
		end
		return literal_3
	end,
})
local literals_4 = { "a5", "b5" }
local literalIndex_4 = {
	a5 = 0,
	b5 = 1,
}
local l5 = Flamework.createSerializer({
	serialize = function(v_4)
		local buf_8 = buffer.create(1)
		local index_4 = literalIndex_4[v_4]
		if index_4 == nil then
			error("value is not one of the literals its type allows")
		end
		buffer.writeu8(buf_8, 0, index_4)
		return buf_8
	end,
	deserialize = function(buf_9)
		local literal_4 = literals_4[buffer.readu8(buf_9, 0) + 1]
		if literal_4 == nil then
			error("malformed payload")
		end
		if buffer.len(buf_9) ~= 1 then
			error("malformed payload")
		end
		return literal_4
	end,
})
local literals_5 = { "a6", "b6" }
local literalIndex_5 = {
	a6 = 0,
	b6 = 1,
}
local l6 = Flamework.createSerializer({
	serialize = function(v_5)
		local buf_10 = buffer.create(1)
		local index_5 = literalIndex_5[v_5]
		if index_5 == nil then
			error("value is not one of the literals its type allows")
		end
		buffer.writeu8(buf_10, 0, index_5)
		return buf_10
	end,
	deserialize = function(buf_11)
		local literal_5 = literals_5[buffer.readu8(buf_11, 0) + 1]
		if literal_5 == nil then
			error("malformed payload")
		end
		if buffer.len(buf_11) ~= 1 then
			error("malformed payload")
		end
		return literal_5
	end,
})
local literals_6 = { "a7", "b7" }
local literalIndex_6 = {
	a7 = 0,
	b7 = 1,
}
local l7 = Flamework.createSerializer({
	serialize = function(v_6)
		local buf_12 = buffer.create(1)
		local index_6 = literalIndex_6[v_6]
		if index_6 == nil then
			error("value is not one of the literals its type allows")
		end
		buffer.writeu8(buf_12, 0, index_6)
		return buf_12
	end,
	deserialize = function(buf_13)
		local literal_6 = literals_6[buffer.readu8(buf_13, 0) + 1]
		if literal_6 == nil then
			error("malformed payload")
		end
		if buffer.len(buf_13) ~= 1 then
			error("malformed payload")
		end
		return literal_6
	end,
})
local literals_7 = { "a8", "b8" }
local literalIndex_7 = {
	a8 = 0,
	b8 = 1,
}
local l8 = Flamework.createSerializer({
	serialize = function(v_7)
		local buf_14 = buffer.create(1)
		local index_7 = literalIndex_7[v_7]
		if index_7 == nil then
			error("value is not one of the literals its type allows")
		end
		buffer.writeu8(buf_14, 0, index_7)
		return buf_14
	end,
	deserialize = function(buf_15)
		local literal_7 = literals_7[buffer.readu8(buf_15, 0) + 1]
		if literal_7 == nil then
			error("malformed payload")
		end
		if buffer.len(buf_15) ~= 1 then
			error("malformed payload")
		end
		return literal_7
	end,
})
local literals_8 = { "a9", "b9" }
local literalIndex_8 = {
	a9 = 0,
	b9 = 1,
}
local l9 = Flamework.createSerializer({
	serialize = function(v_8)
		local buf_16 = buffer.create(1)
		local index_8 = literalIndex_8[v_8]
		if index_8 == nil then
			error("value is not one of the literals its type allows")
		end
		buffer.writeu8(buf_16, 0, index_8)
		return buf_16
	end,
	deserialize = function(buf_17)
		local literal_8 = literals_8[buffer.readu8(buf_17, 0) + 1]
		if literal_8 == nil then
			error("malformed payload")
		end
		if buffer.len(buf_17) ~= 1 then
			error("malformed payload")
		end
		return literal_8
	end,
})
local enum_Material = {}
for _, item in Enum.Material:GetEnumItems() do
	enum_Material[item.Value] = item
end
local e1 = Flamework.createSerializer({
	serialize = function(v_9)
		local buf_18 = buffer.create(2)
		buffer.writeu16(buf_18, 0, v_9.Value)
		return buf_18
	end,
	deserialize = function(buf_19)
		local value = enum_Material[buffer.readu16(buf_19, 0)]
		if buffer.len(buf_19) ~= 2 then
			error("malformed payload")
		end
		return value
	end,
})
local enum_KeyCode = {}
for _, item_1 in Enum.KeyCode:GetEnumItems() do
	enum_KeyCode[item_1.Value] = item_1
end
local e2 = Flamework.createSerializer({
	serialize = function(v_10)
		local buf_20 = buffer.create(2)
		buffer.writeu16(buf_20, 0, v_10.Value)
		return buf_20
	end,
	deserialize = function(buf_21)
		local value_1 = enum_KeyCode[buffer.readu16(buf_21, 0)]
		if buffer.len(buf_21) ~= 2 then
			error("malformed payload")
		end
		return value_1
	end,
})
local enum_UserInputType = {}
for _, item_2 in Enum.UserInputType:GetEnumItems() do
	enum_UserInputType[item_2.Value] = item_2
end
local e3 = Flamework.createSerializer({
	serialize = function(v_11)
		local buf_22 = buffer.create(2)
		buffer.writeu16(buf_22, 0, v_11.Value)
		return buf_22
	end,
	deserialize = function(buf_23)
		local value_2 = enum_UserInputType[buffer.readu16(buf_23, 0)]
		if buffer.len(buf_23) ~= 2 then
			error("malformed payload")
		end
		return value_2
	end,
})
local literals_9 = { "a10", "b10" }
local literalIndex_9 = {
	a10 = 0,
	b10 = 1,
}
local l10 = Flamework.createSerializer({
	serialize = function(v_12)
		local buf_24 = buffer.create(1)
		local index_9 = literalIndex_9[v_12]
		if index_9 == nil then
			error("value is not one of the literals its type allows")
		end
		buffer.writeu8(buf_24, 0, index_9)
		return buf_24
	end,
	deserialize = function(buf_25)
		local literal_9 = literals_9[buffer.readu8(buf_25, 0) + 1]
		if literal_9 == nil then
			error("malformed payload")
		end
		if buffer.len(buf_25) ~= 1 then
			error("malformed payload")
		end
		return literal_9
	end,
})
return {
	l1 = l1,
	l2 = l2,
	l3 = l3,
	l4 = l4,
	l5 = l5,
	l6 = l6,
	l7 = l7,
	l8 = l8,
	l9 = l9,
	e1 = e1,
	e2 = e2,
	e3 = e3,
	l10 = l10,
}
