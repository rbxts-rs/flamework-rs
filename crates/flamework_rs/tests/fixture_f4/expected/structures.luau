-- Compiled with roblox-ts v3.0.0
local TS = _G[script]
local Flamework = TS.import(script, TS.getModule(script, "@flamework-experimental", "core").out).Flamework
local Level
do
	local _inverse = {}
	Level = setmetatable({}, {
		__index = _inverse,
	})
	Level.Low = 0
	_inverse[0] = "Low"
	Level.Mid = 5
	_inverse[5] = "Mid"
	Level.High = 10
	_inverse[10] = "High"
end
local Named = {
	Alpha = "alpha",
	Beta = "beta",
}
local point = Flamework.createSerializer({
	serialize = function(v)
		local buf = buffer.create(16)
		buffer.writef64(buf, 0, v.x)
		buffer.writef64(buf, 8, v.y)
		return buf
	end,
	deserialize = function(buf_1)
		local value_1 = {
			x = buffer.readf64(buf_1, 0),
			y = buffer.readf64(buf_1, 8),
		}
		if buffer.len(buf_1) ~= 16 then
			error("malformed payload")
		end
		return value_1
	end,
})
local codec = {}
local vsize = function(n)
	return if n < 128 then 1 elseif n < 16384 then 2 elseif n < 2097152 then 3 elseif n < 268435456 then 4 else 5
end
local vwrite = function(buf_2, o, n_1)
	while n_1 >= 128 do
		buffer.writeu8(buf_2, o, n_1 % 128 + 128)
		o += 1
		n_1 = math.floor(n_1 / 128)
	end
	buffer.writeu8(buf_2, o, n_1)
	return o + 1
end
local vread = function(buf_3, o_1)
	local n_2 = 0
	local scale = 1
	while true do
		local b_1 = buffer.readu8(buf_3, o_1)
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
local literals = { "a", "b", "c" }
local literalIndex = {
	a = 0,
	b = 1,
	c = 2,
}
codec.s_Entity = function(v_1)
	local array = v_1.tags
	local n_3 = #array
	local size = vsize(n_3)
	for i = 1, n_3 do
		local item = array[i]
		if item == nil then
			error(`[Flamework] the array has no value at Entity.tags[{i - 1}]`)
		end
		local length = #item
		size += vsize(length) + length
	end
	local v_2 = v_1.maybe
	return #(v_1.name) + 1 + size + (if v_2 ~= nil then 9 else 1) + 23
end
codec.w_Entity = function(buf_4, o_2, v_1, blobs)
	buffer.writeu16(buf_4, o_2, v_1.id)
	local text = v_1.name
	local length_1 = #text
	if length_1 > 255 then
		error("string is longer than its u8 length prefix allows")
	end
	buffer.writeu8(buf_4, o_2 + 2, length_1)
	buffer.writestring(buf_4, o_2 + 3, text)
	o_2 += length_1 + 3
	local array_1 = v_1.tags
	o_2 = vwrite(buf_4, o_2, #array_1)
	for i_1 = 1, #array_1 do
		local item_1 = array_1[i_1]
		local length_2 = #item_1
		o_2 = vwrite(buf_4, o_2, length_2)
		buffer.writestring(buf_4, o_2, item_1)
		o_2 += length_2
	end
	local object = v_1.where
	buffer.writef64(buf_4, o_2, object.x)
	buffer.writef64(buf_4, o_2 + 8, object.y)
	local v_3 = v_1.mode
	local index = literalIndex[v_3]
	if index == nil then
		error("value is not one of the literals its type allows")
	end
	buffer.writeu8(buf_4, o_2 + 16, index)
	local v_4 = v_1.maybe
	buffer.writeu8(buf_4, o_2 + 17, if v_4 ~= nil then 1 else 0)
	o_2 += 18
	if v_4 ~= nil then
		buffer.writef64(buf_4, o_2, v_4)
		o_2 += 8
	end
	local blob = v_1.owner
	if blob ~= nil then
		table.insert(blobs, blob)
		buffer.writeu32(buf_4, o_2, #blobs)
	else
		buffer.writeu32(buf_4, o_2, 0)
	end
	o_2 += 4
	return o_2
end
codec.r_Entity = function(buf_4, o_2, blobs)
	local id_1 = buffer.readu16(buf_4, o_2)
	local length_3 = buffer.readu8(buf_4, o_2 + 2)
	local text_1 = buffer.readstring(buf_4, o_2 + 3, length_3)
	o_2 += length_3 + 3
	local count, o_3 = vread(buf_4, o_2)
	o_2 = o_3
	if count > buffer.len(buf_4) - o_2 then
		error("malformed payload")
	end
	local array_2 = table.create(count)
	for i_2 = 1, count do
		local length_4, o_4 = vread(buf_4, o_2)
		local text_2 = buffer.readstring(buf_4, o_4, length_4)
		o_2 = o_4 + length_4
		array_2[i_2] = text_2
	end
	local x_1 = buffer.readf64(buf_4, o_2)
	local y_1 = buffer.readf64(buf_4, o_2 + 8)
	local where_1 = {
		x = x_1,
		y = y_1,
	}
	local literal = literals[buffer.readu8(buf_4, o_2 + 16) + 1]
	if literal == nil then
		error("malformed payload")
	end
	local present = buffer.readu8(buf_4, o_2 + 17) ~= 0
	local value_2
	o_2 += 18
	if present then
		value_2 = buffer.readf64(buf_4, o_2)
		o_2 += 8
	end
	local owner_1 = blobs[buffer.readu32(buf_4, o_2)]
	local value_3 = {
		id = id_1,
		name = text_1,
		tags = array_2,
		where = where_1,
		mode = literal,
		maybe = value_2,
		kind = "entity",
		owner = owner_1,
	}
	o_2 += 4
	return value_3, o_2
end
local entity = Flamework.createSerializer({
	serialize = function(v_5)
		local buf_5 = buffer.create(codec.s_Entity(v_5))
		local blobs_1 = {}
		codec.w_Entity(buf_5, 0, v_5, blobs_1)
		return buf_5, blobs_1
	end,
	deserialize = function(buf_6, blobs_2)
		local value_4, o_5 = codec.r_Entity(buf_6, 0, blobs_2)
		if o_5 ~= buffer.len(buf_6) then
			error("malformed payload")
		end
		return value_4
	end,
})
codec.s_Derived = function(v_6)
	local length_5 = #(v_6.shared)
	return vsize(length_5) + length_5 + 9
end
codec.w_Derived = function(buf_7, o_6, v_6)
	buffer.writeu8(buf_7, o_6, if v_6.own then 1 else 0)
	local text_3 = v_6.shared
	local length_6 = #text_3
	o_6 = vwrite(buf_7, o_6 + 1, length_6)
	buffer.writestring(buf_7, o_6, text_3)
	o_6 += length_6
	buffer.writef64(buf_7, o_6, v_6.base)
	o_6 += 8
	return o_6
end
codec.r_Derived = function(buf_7, o_6)
	local own_1 = buffer.readu8(buf_7, o_6) ~= 0
	local length_7, o_7 = vread(buf_7, o_6 + 1)
	local text_4 = buffer.readstring(buf_7, o_7, length_7)
	o_6 = o_7 + length_7
	local base_1 = buffer.readf64(buf_7, o_6)
	local value_5 = {
		own = own_1,
		shared = text_4,
		base = base_1,
	}
	o_6 += 8
	return value_5, o_6
end
local derived = Flamework.createSerializer({
	serialize = function(v_7)
		local buf_8 = buffer.create(codec.s_Derived(v_7))
		codec.w_Derived(buf_8, 0, v_7)
		return buf_8
	end,
	deserialize = function(buf_9)
		local value_6, o_8 = codec.r_Derived(buf_9, 0)
		if o_8 ~= buffer.len(buf_9) then
			error("malformed payload")
		end
		return value_6
	end,
})
codec.s_Keys = function(v_8)
	local length_8 = #(v_8["two words"])
	return vsize(length_8) + length_8 + 17
end
codec.w_Keys = function(buf_10, o_9, v_8)
	buffer.writef64(buf_10, o_9, v_8[10])
	local text_5 = v_8["two words"]
	local length_9 = #text_5
	o_9 = vwrite(buf_10, o_9 + 8, length_9)
	buffer.writestring(buf_10, o_9, text_5)
	o_9 += length_9
	buffer.writeu8(buf_10, o_9, if v_8[-1] then 1 else 0)
	buffer.writef64(buf_10, o_9 + 1, v_8.plain)
	o_9 += 9
	return o_9
end
codec.r_Keys = function(buf_10, o_9)
	local v_10 = buffer.readf64(buf_10, o_9)
	local length_10, o_10 = vread(buf_10, o_9 + 8)
	local text_6 = buffer.readstring(buf_10, o_10, length_10)
	o_9 = o_10 + length_10
	local _1 = buffer.readu8(buf_10, o_9) ~= 0
	local plain_1 = buffer.readf64(buf_10, o_9 + 1)
	local value_7 = {
		[10] = v_10,
		["two words"] = text_6,
		[-1] = _1,
		plain = plain_1,
	}
	o_9 += 9
	return value_7, o_9
end
local keys = Flamework.createSerializer({
	serialize = function(v_9)
		local buf_11 = buffer.create(codec.s_Keys(v_9))
		codec.w_Keys(buf_11, 0, v_9)
		return buf_11
	end,
	deserialize = function(buf_12)
		local value_8, o_11 = codec.r_Keys(buf_12, 0)
		if o_11 ~= buffer.len(buf_12) then
			error("malformed payload")
		end
		return value_8
	end,
})
codec.s_Tree = function(v_11)
	local array_3 = v_11.children
	local n_4 = #array_3
	local size_1 = vsize(n_4)
	for i_3 = 1, n_4 do
		local item_2 = array_3[i_3]
		if item_2 == nil then
			error(`[Flamework] the array has no value at Tree.children[{i_3 - 1}]`)
		end
		size_1 += codec.s_Tree(item_2)
	end
	return size_1 + 8
end
codec.w_Tree = function(buf_13, o_12, v_11)
	buffer.writef64(buf_13, o_12, v_11.value)
	local array_4 = v_11.children
	o_12 = vwrite(buf_13, o_12 + 8, #array_4)
	for i_4 = 1, #array_4 do
		local item_3 = array_4[i_4]
		o_12 = codec.w_Tree(buf_13, o_12, item_3)
	end
	return o_12
end
codec.r_Tree = function(buf_13, o_12)
	local value_9 = buffer.readf64(buf_13, o_12)
	local count_1, o_13 = vread(buf_13, o_12 + 8)
	o_12 = o_13
	if count_1 * 9 > buffer.len(buf_13) - o_12 then
		error("malformed payload")
	end
	local array_5 = table.create(count_1)
	for i_5 = 1, count_1 do
		local value_10, o_14 = codec.r_Tree(buf_13, o_12)
		array_5[i_5] = value_10
		o_12 = o_14
	end
	local value_11 = {
		value = value_9,
		children = array_5,
	}
	return value_11, o_12
end
local tree = Flamework.createSerializer({
	serialize = function(v_12)
		local buf_14 = buffer.create(codec.s_Tree(v_12))
		codec.w_Tree(buf_14, 0, v_12)
		return buf_14
	end,
	deserialize = function(buf_15)
		local value_12, o_15 = codec.r_Tree(buf_15, 0)
		if o_15 ~= buffer.len(buf_15) then
			error("malformed payload")
		end
		return value_12
	end,
})
codec.s_Chain = function(v_13)
	local v_14 = v_13.next
	local size_2 = 1
	if v_14 ~= nil then
		size_2 += codec.s_Chain(v_14)
	end
	return size_2 + 1
end
codec.w_Chain = function(buf_16, o_16, v_13)
	buffer.writeu8(buf_16, o_16, v_13.value)
	local v_15 = v_13.next
	buffer.writeu8(buf_16, o_16 + 1, if v_15 ~= nil then 1 else 0)
	o_16 += 2
	if v_15 ~= nil then
		o_16 = codec.w_Chain(buf_16, o_16, v_15)
	end
	return o_16
end
codec.r_Chain = function(buf_16, o_16)
	local value_13 = buffer.readu8(buf_16, o_16)
	local present_1 = buffer.readu8(buf_16, o_16 + 1) ~= 0
	local value_14
	o_16 += 2
	if present_1 then
		local value_15, o_17 = codec.r_Chain(buf_16, o_16)
		value_14 = value_15
		o_16 = o_17
	end
	local value_16 = {
		value = value_13,
		next = value_14,
	}
	return value_16, o_16
end
local chain = Flamework.createSerializer({
	serialize = function(v_16)
		local buf_17 = buffer.create(codec.s_Chain(v_16))
		codec.w_Chain(buf_17, 0, v_16)
		return buf_17
	end,
	deserialize = function(buf_18)
		local value_17, o_18 = codec.r_Chain(buf_18, 0)
		if o_18 ~= buffer.len(buf_18) then
			error("malformed payload")
		end
		return value_17
	end,
})
codec.s_Patch_Tree = function(v_17)
	local v_18 = v_17.value
	local v_19 = v_17.children
	local size_3 = 1
	if v_19 ~= nil then
		local n_5 = #v_19
		local size_4 = vsize(n_5)
		for i_6 = 1, n_5 do
			local item_4 = v_19[i_6]
			local size_5 = 1
			if item_4 ~= nil then
				size_5 += codec.s_Patch_Tree(item_4)
			end
			size_4 += size_5
		end
		size_3 += size_4
	end
	return (if v_18 ~= nil then 9 else 1) + size_3
end
codec.w_Patch_Tree = function(buf_19, o_19, v_17)
	local v_20 = v_17.value
	buffer.writeu8(buf_19, o_19, if v_20 ~= nil then 1 else 0)
	o_19 += 1
	if v_20 ~= nil then
		buffer.writef64(buf_19, o_19, v_20)
		o_19 += 8
	end
	local v_21 = v_17.children
	buffer.writeu8(buf_19, o_19, if v_21 ~= nil then 1 else 0)
	o_19 += 1
	if v_21 ~= nil then
		o_19 = vwrite(buf_19, o_19, #v_21)
		for i_7 = 1, #v_21 do
			local item_5 = v_21[i_7]
			buffer.writeu8(buf_19, o_19, if item_5 ~= nil then 1 else 0)
			o_19 += 1
			if item_5 ~= nil then
				o_19 = codec.w_Patch_Tree(buf_19, o_19, item_5)
			end
		end
	end
	return o_19
end
codec.r_Patch_Tree = function(buf_19, o_19)
	local present_2 = buffer.readu8(buf_19, o_19) ~= 0
	local value_18
	o_19 += 1
	if present_2 then
		value_18 = buffer.readf64(buf_19, o_19)
		o_19 += 8
	end
	local present_3 = buffer.readu8(buf_19, o_19) ~= 0
	local value_19
	o_19 += 1
	if present_3 then
		local count_2, o_20 = vread(buf_19, o_19)
		o_19 = o_20
		if count_2 > buffer.len(buf_19) - o_19 then
			error("malformed payload")
		end
		local array_6 = table.create(count_2)
		for i_8 = 1, count_2 do
			local present_4 = buffer.readu8(buf_19, o_19) ~= 0
			local value_20
			o_19 += 1
			if present_4 then
				local value_21, o_21 = codec.r_Patch_Tree(buf_19, o_19)
				value_20 = value_21
				o_19 = o_21
			end
			array_6[i_8] = value_20
		end
		value_19 = array_6
	end
	local value_22 = {
		value = value_18,
		children = value_19,
	}
	return value_22, o_19
end
local patch = Flamework.createSerializer({
	serialize = function(v_22)
		local buf_20 = buffer.create(codec.s_Patch_Tree(v_22))
		codec.w_Patch_Tree(buf_20, 0, v_22)
		return buf_20
	end,
	deserialize = function(buf_21)
		local value_23, o_22 = codec.r_Patch_Tree(buf_21, 0)
		if o_22 ~= buffer.len(buf_21) then
			error("malformed payload")
		end
		return value_23
	end,
})
codec.s_Partial = function(v_23)
	local v_24 = v_23.x
	local v_25 = v_23.y
	return (if v_24 ~= nil then 9 else 1) + (if v_25 ~= nil then 9 else 1)
end
codec.w_Partial = function(buf_22, o_23, v_23)
	local v_26 = v_23.x
	buffer.writeu8(buf_22, o_23, if v_26 ~= nil then 1 else 0)
	o_23 += 1
	if v_26 ~= nil then
		buffer.writef64(buf_22, o_23, v_26)
		o_23 += 8
	end
	local v_27 = v_23.y
	buffer.writeu8(buf_22, o_23, if v_27 ~= nil then 1 else 0)
	o_23 += 1
	if v_27 ~= nil then
		buffer.writef64(buf_22, o_23, v_27)
		o_23 += 8
	end
	return o_23
end
codec.r_Partial = function(buf_22, o_23)
	local present_5 = buffer.readu8(buf_22, o_23) ~= 0
	local value_24
	o_23 += 1
	if present_5 then
		value_24 = buffer.readf64(buf_22, o_23)
		o_23 += 8
	end
	local present_6 = buffer.readu8(buf_22, o_23) ~= 0
	local value_25
	o_23 += 1
	if present_6 then
		value_25 = buffer.readf64(buf_22, o_23)
		o_23 += 8
	end
	local value_26 = {
		x = value_24,
		y = value_25,
	}
	return value_26, o_23
end
local partial = Flamework.createSerializer({
	serialize = function(v_28)
		local buf_23 = buffer.create(codec.s_Partial(v_28))
		codec.w_Partial(buf_23, 0, v_28)
		return buf_23
	end,
	deserialize = function(buf_24)
		local value_27, o_24 = codec.r_Partial(buf_24, 0)
		if o_24 ~= buffer.len(buf_24) then
			error("malformed payload")
		end
		return value_27
	end,
})
local record = Flamework.createSerializer({
	serialize = function(v_29)
		local buf_25 = buffer.create(24)
		buffer.writef64(buf_25, 0, v_29.a)
		buffer.writef64(buf_25, 8, v_29.b)
		buffer.writef64(buf_25, 16, v_29.c)
		return buf_25
	end,
	deserialize = function(buf_26)
		local value_28 = {
			a = buffer.readf64(buf_26, 0),
			b = buffer.readf64(buf_26, 8),
			c = buffer.readf64(buf_26, 16),
		}
		if buffer.len(buf_26) ~= 24 then
			error("malformed payload")
		end
		return value_28
	end,
})
codec.s_Record = function(v_30)
	local length_11 = #(v_30[0])
	local length_12 = #(v_30[5])
	local length_13 = #(v_30[10])
	return vsize(length_11) + length_11 + (vsize(length_12) + length_12) + (vsize(length_13) + length_13)
end
codec.w_Record = function(buf_27, o_25, v_30)
	local text_7 = v_30[0]
	local length_14 = #text_7
	o_25 = vwrite(buf_27, o_25, length_14)
	buffer.writestring(buf_27, o_25, text_7)
	o_25 += length_14
	local text_8 = v_30[5]
	local length_15 = #text_8
	o_25 = vwrite(buf_27, o_25, length_15)
	buffer.writestring(buf_27, o_25, text_8)
	o_25 += length_15
	local text_9 = v_30[10]
	local length_16 = #text_9
	o_25 = vwrite(buf_27, o_25, length_16)
	buffer.writestring(buf_27, o_25, text_9)
	o_25 += length_16
	return o_25
end
codec.r_Record = function(buf_27, o_25)
	local length_17, o_26 = vread(buf_27, o_25)
	local text_10 = buffer.readstring(buf_27, o_26, length_17)
	o_25 = o_26 + length_17
	local length_18, o_27 = vread(buf_27, o_25)
	local text_11 = buffer.readstring(buf_27, o_27, length_18)
	o_25 = o_27 + length_18
	local length_19, o_28 = vread(buf_27, o_25)
	local text_12 = buffer.readstring(buf_27, o_28, length_19)
	o_25 = o_28 + length_19
	local value_29 = {
		[0] = text_10,
		[5] = text_11,
		[10] = text_12,
	}
	return value_29, o_25
end
local levelRecord = Flamework.createSerializer({
	serialize = function(v_31)
		local buf_28 = buffer.create(codec.s_Record(v_31))
		codec.w_Record(buf_28, 0, v_31)
		return buf_28
	end,
	deserialize = function(buf_29)
		local value_30, o_29 = codec.r_Record(buf_29, 0)
		if o_29 ~= buffer.len(buf_29) then
			error("malformed payload")
		end
		return value_30
	end,
})
codec.s_Pick = function(v_32)
	return #(v_32.name) + 1 + 2
end
codec.w_Pick = function(buf_30, o_30, v_32)
	buffer.writeu16(buf_30, o_30, v_32.id)
	local text_13 = v_32.name
	local length_20 = #text_13
	if length_20 > 255 then
		error("string is longer than its u8 length prefix allows")
	end
	buffer.writeu8(buf_30, o_30 + 2, length_20)
	buffer.writestring(buf_30, o_30 + 3, text_13)
	o_30 += length_20 + 3
	return o_30
end
codec.r_Pick = function(buf_30, o_30)
	local id_2 = buffer.readu16(buf_30, o_30)
	local length_21 = buffer.readu8(buf_30, o_30 + 2)
	local text_14 = buffer.readstring(buf_30, o_30 + 3, length_21)
	o_30 += length_21 + 3
	local value_31 = {
		id = id_2,
		name = text_14,
	}
	return value_31, o_30
end
local picked = Flamework.createSerializer({
	serialize = function(v_33)
		local buf_31 = buffer.create(codec.s_Pick(v_33))
		codec.w_Pick(buf_31, 0, v_33)
		return buf_31
	end,
	deserialize = function(buf_32)
		local value_32, o_31 = codec.r_Pick(buf_32, 0)
		if o_31 ~= buffer.len(buf_32) then
			error("malformed payload")
		end
		return value_32
	end,
})
local intersection = Flamework.createSerializer({
	serialize = function(v_34)
		local buf_33 = buffer.create(24)
		buffer.writef64(buf_33, 0, v_34.x)
		buffer.writef64(buf_33, 8, v_34.y)
		buffer.writef64(buf_33, 16, v_34.z)
		return buf_33
	end,
	deserialize = function(buf_34)
		local value_33 = {
			x = buffer.readf64(buf_34, 0),
			y = buffer.readf64(buf_34, 8),
			z = buffer.readf64(buf_34, 16),
		}
		if buffer.len(buf_34) ~= 24 then
			error("malformed payload")
		end
		return value_33
	end,
})
local literals_1 = { 0, 5, 10 }
local literalIndex_1 = {
	[0] = 0,
	[5] = 1,
	[10] = 2,
}
local level = Flamework.createSerializer({
	serialize = function(v_35)
		local buf_35 = buffer.create(1)
		local v_36 = v_35
		local index_1 = literalIndex_1[v_36]
		if index_1 == nil then
			error("value is not one of the literals its type allows")
		end
		buffer.writeu8(buf_35, 0, index_1)
		return buf_35
	end,
	deserialize = function(buf_36)
		local literal_1 = literals_1[buffer.readu8(buf_36, 0) + 1]
		if literal_1 == nil then
			error("malformed payload")
		end
		if buffer.len(buf_36) ~= 1 then
			error("malformed payload")
		end
		return literal_1
	end,
})
local literals_2 = { "alpha", "beta" }
local literalIndex_2 = {
	alpha = 0,
	beta = 1,
}
local named = Flamework.createSerializer({
	serialize = function(v_37)
		local buf_37 = buffer.create(1)
		local v_38 = v_37
		local index_2 = literalIndex_2[v_38]
		if index_2 == nil then
			error("value is not one of the literals its type allows")
		end
		buffer.writeu8(buf_37, 0, index_2)
		return buf_37
	end,
	deserialize = function(buf_38)
		local literal_2 = literals_2[buffer.readu8(buf_38, 0) + 1]
		if literal_2 == nil then
			error("malformed payload")
		end
		if buffer.len(buf_38) ~= 1 then
			error("malformed payload")
		end
		return literal_2
	end,
})
local literals_3 = { 0, 10 }
local literalIndex_3 = {
	[0] = 0,
	[10] = 1,
}
local levels = Flamework.createSerializer({
	serialize = function(v_39)
		local buf_39 = buffer.create(1)
		local v_40 = v_39
		local index_3 = literalIndex_3[v_40]
		if index_3 == nil then
			error("value is not one of the literals its type allows")
		end
		buffer.writeu8(buf_39, 0, index_3)
		return buf_39
	end,
	deserialize = function(buf_40)
		local literal_3 = literals_3[buffer.readu8(buf_40, 0) + 1]
		if literal_3 == nil then
			error("malformed payload")
		end
		if buffer.len(buf_40) ~= 1 then
			error("malformed payload")
		end
		return literal_3
	end,
})
-- Collections.
local numbers = Flamework.createSerializer({
	serialize = function(v_41)
		local n_6 = #v_41
		local buf_41 = buffer.create(vsize(n_6) + n_6 * 8)
		local o_32 = 0
		o_32 = vwrite(buf_41, o_32, #v_41)
		for i_9 = 1, #v_41 do
			local item_6 = v_41[i_9]
			if item_6 == nil then
				error(`[Flamework] the array has no value at value[{i_9 - 1}]`)
			end
			buffer.writef64(buf_41, o_32, item_6)
			o_32 += 8
		end
		return buf_41
	end,
	deserialize = function(buf_42)
		local o_33 = 0
		local count_3, o_34 = vread(buf_42, o_33)
		o_33 = o_34
		if count_3 * 8 > buffer.len(buf_42) - o_33 then
			error("malformed payload")
		end
		local array_7 = table.create(count_3)
		for i_10 = 1, count_3 do
			array_7[i_10] = buffer.readf64(buf_42, o_33)
			o_33 += 8
		end
		if o_33 ~= buffer.len(buf_42) then
			error("malformed payload")
		end
		return array_7
	end,
})
codec.s_stringArray = function(v_42)
	local n_7 = #v_42
	local size_6 = vsize(n_7)
	for i_11 = 1, n_7 do
		local item_7 = v_42[i_11]
		if item_7 == nil then
			error(`[Flamework] the array has no value at (string[])[{i_11 - 1}]`)
		end
		local length_22 = #item_7
		size_6 += vsize(length_22) + length_22
	end
	return size_6
end
codec.w_stringArray = function(buf_43, o_35, v_42)
	o_35 = vwrite(buf_43, o_35, #v_42)
	for i_12 = 1, #v_42 do
		local item_8 = v_42[i_12]
		local length_23 = #item_8
		o_35 = vwrite(buf_43, o_35, length_23)
		buffer.writestring(buf_43, o_35, item_8)
		o_35 += length_23
	end
	return o_35
end
codec.r_stringArray = function(buf_43, o_35)
	local count_4, o_36 = vread(buf_43, o_35)
	o_35 = o_36
	if count_4 > buffer.len(buf_43) - o_35 then
		error("malformed payload")
	end
	local array_8 = table.create(count_4)
	for i_13 = 1, count_4 do
		local length_24, o_37 = vread(buf_43, o_35)
		local text_15 = buffer.readstring(buf_43, o_37, length_24)
		o_35 = o_37 + length_24
		array_8[i_13] = text_15
	end
	return array_8, o_35
end
local texts = Flamework.createSerializer({
	serialize = function(v_43)
		local buf_44 = buffer.create(codec.s_stringArray(v_43))
		codec.w_stringArray(buf_44, 0, v_43)
		return buf_44
	end,
	deserialize = function(buf_45)
		local value_34, o_38 = codec.r_stringArray(buf_45, 0)
		if o_38 ~= buffer.len(buf_45) then
			error("malformed payload")
		end
		return value_34
	end,
})
local readonlyTexts = Flamework.createSerializer({
	serialize = function(v_44)
		local n_8 = #v_44
		local size_7 = vsize(n_8)
		for i_14 = 1, n_8 do
			local item_9 = v_44[i_14]
			if item_9 == nil then
				error(`[Flamework] the array has no value at value[{i_14 - 1}]`)
			end
			local length_25 = #item_9
			size_7 += vsize(length_25) + length_25
		end
		local buf_46 = buffer.create(size_7)
		local o_39 = 0
		o_39 = vwrite(buf_46, o_39, #v_44)
		for i_15 = 1, #v_44 do
			local item_10 = v_44[i_15]
			local length_26 = #item_10
			o_39 = vwrite(buf_46, o_39, length_26)
			buffer.writestring(buf_46, o_39, item_10)
			o_39 += length_26
		end
		return buf_46
	end,
	deserialize = function(buf_47)
		local o_40 = 0
		local count_5, o_41 = vread(buf_47, o_40)
		o_40 = o_41
		if count_5 > buffer.len(buf_47) - o_40 then
			error("malformed payload")
		end
		local array_9 = table.create(count_5)
		for i_16 = 1, count_5 do
			local length_27, o_42 = vread(buf_47, o_40)
			local text_16 = buffer.readstring(buf_47, o_42, length_27)
			o_40 = o_42 + length_27
			array_9[i_16] = text_16
		end
		if o_40 ~= buffer.len(buf_47) then
			error("malformed payload")
		end
		return array_9
	end,
})
codec.s_numberArray = function(v_45)
	local n_9 = #v_45
	return vsize(n_9) + n_9 * 8
end
codec.w_numberArray = function(buf_48, o_43, v_45)
	o_43 = vwrite(buf_48, o_43, #v_45)
	for i_17 = 1, #v_45 do
		local item_11 = v_45[i_17]
		if item_11 == nil then
			error(`[Flamework] the array has no value at (number[])[{i_17 - 1}]`)
		end
		buffer.writef64(buf_48, o_43, item_11)
		o_43 += 8
	end
	return o_43
end
codec.r_numberArray = function(buf_48, o_43)
	local count_6, o_44 = vread(buf_48, o_43)
	o_43 = o_44
	if count_6 * 8 > buffer.len(buf_48) - o_43 then
		error("malformed payload")
	end
	local array_10 = table.create(count_6)
	for i_18 = 1, count_6 do
		array_10[i_18] = buffer.readf64(buf_48, o_43)
		o_43 += 8
	end
	return array_10, o_43
end
local grid = Flamework.createSerializer({
	serialize = function(v_46)
		local n_10 = #v_46
		local size_8 = vsize(n_10)
		for i_19 = 1, n_10 do
			local item_12 = v_46[i_19]
			if item_12 == nil then
				error(`[Flamework] the array has no value at value[{i_19 - 1}]`)
			end
			size_8 += codec.s_numberArray(item_12)
		end
		local buf_49 = buffer.create(size_8)
		local o_45 = 0
		o_45 = vwrite(buf_49, o_45, #v_46)
		for i_20 = 1, #v_46 do
			local item_13 = v_46[i_20]
			o_45 = codec.w_numberArray(buf_49, o_45, item_13)
		end
		return buf_49
	end,
	deserialize = function(buf_50)
		local o_46 = 0
		local count_7, o_47 = vread(buf_50, o_46)
		o_46 = o_47
		if count_7 > buffer.len(buf_50) - o_46 then
			error("malformed payload")
		end
		local array_11 = table.create(count_7)
		for i_21 = 1, count_7 do
			local value_35, o_48 = codec.r_numberArray(buf_50, o_46)
			array_11[i_21] = value_35
			o_46 = o_48
		end
		if o_46 ~= buffer.len(buf_50) then
			error("malformed payload")
		end
		return array_11
	end,
})
local set = Flamework.createSerializer({
	serialize = function(v_47)
		local size_9 = 0
		local n_11 = 0
		for item_14 in v_47 do
			n_11 += 1
			local length_28 = #item_14
			size_9 += vsize(length_28) + length_28
		end
		size_9 += vsize(n_11)
		local buf_51 = buffer.create(size_9)
		local o_49 = 0
		-- ▼ ReadonlySet.size ▼
		local _size = 0
		for _ in v_47 do
			_size += 1
		end
		-- ▲ ReadonlySet.size ▲
		local n_12 = _size
		o_49 = vwrite(buf_51, o_49, n_12)
		for item_15 in v_47 do
			local length_29 = #item_15
			o_49 = vwrite(buf_51, o_49, length_29)
			buffer.writestring(buf_51, o_49, item_15)
			o_49 += length_29
		end
		return buf_51
	end,
	deserialize = function(buf_52)
		local o_50 = 0
		local count_8, o_51 = vread(buf_52, o_50)
		o_50 = o_51
		if count_8 > buffer.len(buf_52) - o_50 then
			error("malformed payload")
		end
		local set_1 = {}
		for _ = 1, count_8 do
			local length_30, o_52 = vread(buf_52, o_50)
			local text_17 = buffer.readstring(buf_52, o_52, length_30)
			o_50 = o_52 + length_30
			set_1[text_17] = true
		end
		if o_50 ~= buffer.len(buf_52) then
			error("malformed payload")
		end
		return set_1
	end,
})
local fixedSet = Flamework.createSerializer({
	serialize = function(v_48)
		local size_10 = 0
		local n_13 = 0
		for item_16 in v_48 do
			n_13 += 1
			size_10 += 1
		end
		size_10 += vsize(n_13)
		local buf_53 = buffer.create(size_10)
		local o_53 = 0
		-- ▼ ReadonlySet.size ▼
		local _size = 0
		for _ in v_48 do
			_size += 1
		end
		-- ▲ ReadonlySet.size ▲
		local n_14 = _size
		o_53 = vwrite(buf_53, o_53, n_14)
		for item_17 in v_48 do
			buffer.writeu8(buf_53, o_53, item_17)
			o_53 += 1
		end
		return buf_53
	end,
	deserialize = function(buf_54)
		local o_54 = 0
		local count_9, o_55 = vread(buf_54, o_54)
		o_54 = o_55
		if count_9 > buffer.len(buf_54) - o_54 then
			error("malformed payload")
		end
		local set_2 = {}
		for _2 = 1, count_9 do
			set_2[buffer.readu8(buf_54, o_54)] = true
			o_54 += 1
		end
		if o_54 ~= buffer.len(buf_54) then
			error("malformed payload")
		end
		return set_2
	end,
})
local map = Flamework.createSerializer({
	serialize = function(v_49)
		local size_11 = 0
		local n_15 = 0
		for key_1, entry in v_49 do
			n_15 += 1
			local length_31 = #key_1
			size_11 += vsize(length_31) + length_31 + 8
		end
		size_11 += vsize(n_15)
		local buf_55 = buffer.create(size_11)
		local o_56 = 0
		-- ▼ ReadonlyMap.size ▼
		local _size = 0
		for _ in v_49 do
			_size += 1
		end
		-- ▲ ReadonlyMap.size ▲
		local n_16 = _size
		o_56 = vwrite(buf_55, o_56, n_16)
		for key_2, entry_1 in v_49 do
			local length_32 = #key_2
			o_56 = vwrite(buf_55, o_56, length_32)
			buffer.writestring(buf_55, o_56, key_2)
			o_56 += length_32
			buffer.writef64(buf_55, o_56, entry_1)
			o_56 += 8
		end
		return buf_55
	end,
	deserialize = function(buf_56)
		local o_57 = 0
		local count_10, o_58 = vread(buf_56, o_57)
		o_57 = o_58
		if count_10 * 9 > buffer.len(buf_56) - o_57 then
			error("malformed payload")
		end
		local map_1 = {}
		for _3 = 1, count_10 do
			local length_33, o_59 = vread(buf_56, o_57)
			local text_18 = buffer.readstring(buf_56, o_59, length_33)
			o_57 = o_59 + length_33
			local entry_2 = buffer.readf64(buf_56, o_57)
			map_1[text_18] = entry_2
			o_57 += 8
		end
		if o_57 ~= buffer.len(buf_56) then
			error("malformed payload")
		end
		return map_1
	end,
})
local fixedMap = Flamework.createSerializer({
	serialize = function(v_50)
		local size_12 = 0
		local n_17 = 0
		for key_3, entry_3 in v_50 do
			n_17 += 1
			size_12 += 13
		end
		size_12 += vsize(n_17)
		local buf_57 = buffer.create(size_12)
		local o_60 = 0
		-- ▼ ReadonlyMap.size ▼
		local _size = 0
		for _ in v_50 do
			_size += 1
		end
		-- ▲ ReadonlyMap.size ▲
		local n_18 = _size
		o_60 = vwrite(buf_57, o_60, n_18)
		for key_4, entry_4 in v_50 do
			buffer.writeu8(buf_57, o_60, key_4)
			buffer.writef32(buf_57, o_60 + 1, entry_4.X)
			buffer.writef32(buf_57, o_60 + 5, entry_4.Y)
			buffer.writef32(buf_57, o_60 + 9, entry_4.Z)
			o_60 += 13
		end
		return buf_57
	end,
	deserialize = function(buf_58)
		local o_61 = 0
		local count_11, o_62 = vread(buf_58, o_61)
		o_61 = o_62
		if count_11 * 13 > buffer.len(buf_58) - o_61 then
			error("malformed payload")
		end
		local map_2 = {}
		for _4 = 1, count_11 do
			local key_5 = buffer.readu8(buf_58, o_61)
			local entry_5 = Vector3.new(buffer.readf32(buf_58, o_61 + 1), buffer.readf32(buf_58, o_61 + 5), buffer.readf32(buf_58, o_61 + 9))
			map_2[key_5] = entry_5
			o_61 += 13
		end
		if o_61 ~= buffer.len(buf_58) then
			error("malformed payload")
		end
		return map_2
	end,
})
local readonlyMap = Flamework.createSerializer({
	serialize = function(v_51)
		local size_13 = 0
		local n_19 = 0
		for key_6, entry_6 in v_51 do
			n_19 += 1
			local length_34 = #key_6
			local size_14 = 0
			local n_20 = 0
			for item_18 in entry_6 do
				n_20 += 1
				size_14 += 8
			end
			size_14 += vsize(n_20)
			size_13 += vsize(length_34) + length_34 + size_14
		end
		size_13 += vsize(n_19)
		local buf_59 = buffer.create(size_13)
		local o_63 = 0
		-- ▼ ReadonlyMap.size ▼
		local _size = 0
		for _ in v_51 do
			_size += 1
		end
		-- ▲ ReadonlyMap.size ▲
		local n_21 = _size
		o_63 = vwrite(buf_59, o_63, n_21)
		for key_7, entry_7 in v_51 do
			local length_35 = #key_7
			o_63 = vwrite(buf_59, o_63, length_35)
			buffer.writestring(buf_59, o_63, key_7)
			o_63 += length_35
			-- ▼ ReadonlySet.size ▼
			local _size_1 = 0
			for _ in entry_7 do
				_size_1 += 1
			end
			-- ▲ ReadonlySet.size ▲
			local n_22 = _size_1
			o_63 = vwrite(buf_59, o_63, n_22)
			for item_19 in entry_7 do
				buffer.writef64(buf_59, o_63, item_19)
				o_63 += 8
			end
		end
		return buf_59
	end,
	deserialize = function(buf_60)
		local o_64 = 0
		local count_12, o_65 = vread(buf_60, o_64)
		o_64 = o_65
		if count_12 * 2 > buffer.len(buf_60) - o_64 then
			error("malformed payload")
		end
		local map_3 = {}
		for _6 = 1, count_12 do
			local length_36, o_66 = vread(buf_60, o_64)
			local text_19 = buffer.readstring(buf_60, o_66, length_36)
			o_64 = o_66 + length_36
			local count_13, o_67 = vread(buf_60, o_64)
			o_64 = o_67
			if count_13 * 8 > buffer.len(buf_60) - o_64 then
				error("malformed payload")
			end
			local set_3 = {}
			for _5 = 1, count_13 do
				set_3[buffer.readf64(buf_60, o_64)] = true
				o_64 += 8
			end
			map_3[text_19] = set_3
		end
		if o_64 ~= buffer.len(buf_60) then
			error("malformed payload")
		end
		return map_3
	end,
})
codec.s_Set_string = function(v_52)
	local size_15 = 0
	local n_23 = 0
	for item_20 in v_52 do
		n_23 += 1
		local length_37 = #item_20
		size_15 += vsize(length_37) + length_37
	end
	size_15 += vsize(n_23)
	return size_15
end
codec.w_Set_string = function(buf_61, o_68, v_52)
	-- ▼ ReadonlySet.size ▼
	local _size = 0
	for _ in v_52 do
		_size += 1
	end
	-- ▲ ReadonlySet.size ▲
	local n_24 = _size
	o_68 = vwrite(buf_61, o_68, n_24)
	for item_21 in v_52 do
		local length_38 = #item_21
		o_68 = vwrite(buf_61, o_68, length_38)
		buffer.writestring(buf_61, o_68, item_21)
		o_68 += length_38
	end
	return o_68
end
codec.r_Set_string = function(buf_61, o_68)
	local count_14, o_69 = vread(buf_61, o_68)
	o_68 = o_69
	if count_14 > buffer.len(buf_61) - o_68 then
		error("malformed payload")
	end
	local set_4 = {}
	for _7 = 1, count_14 do
		local length_39, o_70 = vread(buf_61, o_68)
		local text_20 = buffer.readstring(buf_61, o_70, length_39)
		o_68 = o_70 + length_39
		set_4[text_20] = true
	end
	return set_4, o_68
end
local nested = Flamework.createSerializer({
	serialize = function(v_53)
		local size_16 = 0
		local n_25 = 0
		for key_8, entry_8 in v_53 do
			n_25 += 1
			local n_26 = #entry_8
			local size_17 = vsize(n_26)
			for i_22 = 1, n_26 do
				local item_22 = entry_8[i_22]
				if item_22 == nil then
					error(`[Flamework] the array has no value at value<value>[{i_22 - 1}]`)
				end
				size_17 += codec.s_Set_string(item_22)
			end
			size_16 += 4 + size_17
		end
		size_16 += vsize(n_25)
		local buf_62 = buffer.create(size_16)
		local blobs_3 = {}
		local o_71 = 0
		-- ▼ ReadonlyMap.size ▼
		local _size = 0
		for _ in v_53 do
			_size += 1
		end
		-- ▲ ReadonlyMap.size ▲
		local n_27 = _size
		o_71 = vwrite(buf_62, o_71, n_27)
		for key_9, entry_9 in v_53 do
			if key_9 ~= nil then
				table.insert(blobs_3, key_9)
				buffer.writeu32(buf_62, o_71, #blobs_3)
			else
				buffer.writeu32(buf_62, o_71, 0)
			end
			o_71 = vwrite(buf_62, o_71 + 4, #entry_9)
			for i_23 = 1, #entry_9 do
				local item_23 = entry_9[i_23]
				o_71 = codec.w_Set_string(buf_62, o_71, item_23)
			end
		end
		return buf_62, blobs_3
	end,
	deserialize = function(buf_63, blobs_4)
		local o_72 = 0
		local count_15, o_73 = vread(buf_63, o_72)
		o_72 = o_73
		if count_15 * 5 > buffer.len(buf_63) - o_72 then
			error("malformed payload")
		end
		local map_4 = {}
		for _8 = 1, count_15 do
			local key_10 = blobs_4[buffer.readu32(buf_63, o_72)]
			local count_16, o_74 = vread(buf_63, o_72 + 4)
			o_72 = o_74
			if count_16 > buffer.len(buf_63) - o_72 then
				error("malformed payload")
			end
			local array_12 = table.create(count_16)
			for i_24 = 1, count_16 do
				local value_36, o_75 = codec.r_Set_string(buf_63, o_72)
				array_12[i_24] = value_36
				o_72 = o_75
			end
			map_4[key_10] = array_12
		end
		if o_72 ~= buffer.len(buf_63) then
			error("malformed payload")
		end
		return map_4
	end,
})
local indexed = Flamework.createSerializer({
	serialize = function(v_54)
		local size_18 = 0
		local n_28 = 0
		for key_11, entry_10 in v_54 do
			n_28 += 1
			local length_40 = #key_11
			size_18 += vsize(length_40) + length_40 + 16
		end
		size_18 += vsize(n_28)
		local buf_64 = buffer.create(size_18)
		local o_76 = 0
		-- ▼ ReadonlyMap.size ▼
		local _size = 0
		for _ in v_54 do
			_size += 1
		end
		-- ▲ ReadonlyMap.size ▲
		local n_29 = _size
		o_76 = vwrite(buf_64, o_76, n_29)
		for key_12, entry_11 in v_54 do
			local length_41 = #key_12
			o_76 = vwrite(buf_64, o_76, length_41)
			buffer.writestring(buf_64, o_76, key_12)
			o_76 += length_41
			buffer.writef64(buf_64, o_76, entry_11.x)
			buffer.writef64(buf_64, o_76 + 8, entry_11.y)
			o_76 += 16
		end
		return buf_64
	end,
	deserialize = function(buf_65)
		local o_77 = 0
		local count_17, o_78 = vread(buf_65, o_77)
		o_77 = o_78
		if count_17 * 17 > buffer.len(buf_65) - o_77 then
			error("malformed payload")
		end
		local map_5 = {}
		for _9 = 1, count_17 do
			local length_42, o_79 = vread(buf_65, o_77)
			local text_21 = buffer.readstring(buf_65, o_79, length_42)
			o_77 = o_79 + length_42
			local x_2 = buffer.readf64(buf_65, o_77)
			local y_2 = buffer.readf64(buf_65, o_77 + 8)
			local entry_12 = {
				x = x_2,
				y = y_2,
			}
			map_5[text_21] = entry_12
			o_77 += 16
		end
		if o_77 ~= buffer.len(buf_65) then
			error("malformed payload")
		end
		return map_5
	end,
})
local zeros = 0
local markers = Flamework.createSerializer({
	serialize = function(v_55)
		local n_30 = #v_55
		local size_19 = vsize(n_30)
		for i_25 = 1, n_30 do
			local item_24 = v_55[i_25]
			if item_24 == nil then
				error(`[Flamework] the array has no value at value[{i_25 - 1}]`)
			end
			local n_31 = #item_24
			size_19 += vsize(n_31)
		end
		local buf_66 = buffer.create(size_19)
		local o_80 = 0
		o_80 = vwrite(buf_66, o_80, #v_55)
		for i_26 = 1, #v_55 do
			local item_25 = v_55[i_26]
			o_80 = vwrite(buf_66, o_80, #item_25)
			for i_27 = 1, #item_25 do
				local item_26 = item_25[i_27]
				if item_26 == nil then
					error(`[Flamework] the array has no value at value[][{i_27 - 1}]`)
				end
			end
		end
		return buf_66
	end,
	deserialize = function(buf_67)
		zeros = 0
		local o_81 = 0
		local count_18, o_82 = vread(buf_67, o_81)
		o_81 = o_82
		if count_18 > buffer.len(buf_67) - o_81 then
			error("malformed payload")
		end
		local array_13 = table.create(count_18)
		for i_29 = 1, count_18 do
			local count_19, o_83 = vread(buf_67, o_81)
			o_81 = o_83
			zeros += count_19
			if zeros > 65535 then
				error("malformed payload")
			end
			local array_14 = table.create(count_19)
			for i_28 = 1, count_19 do
				array_14[i_28] = {
					type = "marker",
				}
			end
			array_13[i_29] = array_14
		end
		if o_81 ~= buffer.len(buf_67) then
			error("malformed payload")
		end
		return array_13
	end,
})
-- Tuples.
local pair = Flamework.createSerializer({
	serialize = function(v_56)
		local length_43 = #(v_56[2])
		local buf_68 = buffer.create(vsize(length_43) + length_43 + 8)
		local o_84 = 0
		buffer.writef64(buf_68, o_84, v_56[1])
		local text_22 = v_56[2]
		local length_44 = #text_22
		o_84 = vwrite(buf_68, o_84 + 8, length_44)
		buffer.writestring(buf_68, o_84, text_22)
		o_84 += length_44
		return buf_68
	end,
	deserialize = function(buf_69)
		local o_85 = 0
		local arg = buffer.readf64(buf_69, o_85)
		local length_45, o_86 = vread(buf_69, o_85 + 8)
		local text_23 = buffer.readstring(buf_69, o_86, length_45)
		o_85 = o_86 + length_45
		local value_37 = { arg, text_23 }
		if o_85 ~= buffer.len(buf_69) then
			error("malformed payload")
		end
		return value_37
	end,
})
local optionalTuple = Flamework.createSerializer({
	serialize = function(v_57)
		local v_58 = v_57[2]
		local size_20 = 1
		if v_58 ~= nil then
			local length_46 = #v_58
			size_20 += vsize(length_46) + length_46
		end
		local count_20 = math.max(#v_57 - 2, 0)
		local buf_70 = buffer.create(size_20 + (vsize(count_20) + count_20 * 1) + 8)
		local o_87 = 0
		buffer.writef64(buf_70, o_87, v_57[1])
		local v_59 = v_57[2]
		buffer.writeu8(buf_70, o_87 + 8, if v_59 ~= nil then 1 else 0)
		o_87 += 9
		if v_59 ~= nil then
			local length_47 = #v_59
			o_87 = vwrite(buf_70, o_87, length_47)
			buffer.writestring(buf_70, o_87, v_59)
			o_87 += length_47
		end
		local count_21 = math.max(#v_57 - 2, 0)
		o_87 = vwrite(buf_70, o_87, count_21)
		for i_30 = 3, count_21 + 2 do
			local item_27 = v_57[i_30]
			if item_27 == nil then
				error(`[Flamework] the tuple has no value at value[{i_30 - 1}]`)
			end
			buffer.writeu8(buf_70, o_87, if item_27 then 1 else 0)
			o_87 += 1
		end
		return buf_70
	end,
	deserialize = function(buf_71)
		local o_88 = 0
		local arg_1 = buffer.readf64(buf_71, o_88)
		local present_7 = buffer.readu8(buf_71, o_88 + 8) ~= 0
		local value_38
		o_88 += 9
		if present_7 then
			local length_48, o_89 = vread(buf_71, o_88)
			local text_24 = buffer.readstring(buf_71, o_89, length_48)
			o_88 = o_89 + length_48
			value_38 = text_24
		end
		local count_22, o_90 = vread(buf_71, o_88)
		o_88 = o_90
		if count_22 > buffer.len(buf_71) - o_88 then
			error("malformed payload")
		end
		local list = { arg_1, value_38 }
		for i_31 = 1, count_22 do
			list[i_31 + 2] = buffer.readu8(buf_71, o_88) ~= 0
			o_88 += 1
		end
		if o_88 ~= buffer.len(buf_71) then
			error("malformed payload")
		end
		return list
	end,
})
local restFirst = Flamework.createSerializer({
	serialize = function(v_60)
		local count_23 = math.max(#v_60 - 1, 0)
		local size_21 = vsize(count_23) + 1
		for i_32 = 1, count_23 do
			local item_28 = v_60[i_32]
			if item_28 == nil then
				error(`[Flamework] the tuple has no value at value[{i_32 - 1}]`)
			end
			local length_49 = #item_28
			size_21 += vsize(length_49) + length_49
		end
		local buf_72 = buffer.create(size_21)
		local o_91 = 0
		local count_24 = math.max(#v_60 - 1, 0)
		o_91 = vwrite(buf_72, o_91, count_24)
		for i_33 = 1, count_24 do
			local text_25 = v_60[i_33]
			local length_50 = #text_25
			o_91 = vwrite(buf_72, o_91, length_50)
			buffer.writestring(buf_72, o_91, text_25)
			o_91 += length_50
		end
		buffer.writeu8(buf_72, o_91, if v_60[count_24 + 1] then 1 else 0)
		return buf_72
	end,
	deserialize = function(buf_73)
		local o_92 = 0
		local count_25, o_93 = vread(buf_73, o_92)
		o_92 = o_93
		if count_25 > buffer.len(buf_73) - o_92 then
			error("malformed payload")
		end
		local list_1 = {}
		for i_34 = 1, count_25 do
			local length_51, o_94 = vread(buf_73, o_92)
			local text_26 = buffer.readstring(buf_73, o_94, length_51)
			o_92 = o_94 + length_51
			list_1[i_34] = text_26
		end
		local arg_2 = buffer.readu8(buf_73, o_92) ~= 0
		list_1[count_25 + 1] = arg_2
		o_92 += 1
		if o_92 ~= buffer.len(buf_73) then
			error("malformed payload")
		end
		return list_1
	end,
})
local restMiddle = Flamework.createSerializer({
	serialize = function(v_61)
		local count_26 = math.max(#v_61 - 2, 0)
		local size_22 = vsize(count_26) + 9
		for i_35 = 2, count_26 + 1 do
			local item_29 = v_61[i_35]
			if item_29 == nil then
				error(`[Flamework] the tuple has no value at value[{i_35 - 1}]`)
			end
			local length_52 = #item_29
			size_22 += vsize(length_52) + length_52
		end
		local buf_74 = buffer.create(size_22)
		local o_95 = 0
		buffer.writef64(buf_74, o_95, v_61[1])
		local count_27 = math.max(#v_61 - 2, 0)
		o_95 = vwrite(buf_74, o_95 + 8, count_27)
		for i_36 = 2, count_27 + 1 do
			local text_27 = v_61[i_36]
			local length_53 = #text_27
			o_95 = vwrite(buf_74, o_95, length_53)
			buffer.writestring(buf_74, o_95, text_27)
			o_95 += length_53
		end
		buffer.writeu8(buf_74, o_95, if v_61[count_27 + 2] then 1 else 0)
		return buf_74
	end,
	deserialize = function(buf_75)
		local o_96 = 0
		local arg_3 = buffer.readf64(buf_75, o_96)
		local count_28, o_97 = vread(buf_75, o_96 + 8)
		o_96 = o_97
		if count_28 > buffer.len(buf_75) - o_96 then
			error("malformed payload")
		end
		local list_2 = { arg_3 }
		for i_37 = 1, count_28 do
			local length_54, o_98 = vread(buf_75, o_96)
			local text_28 = buffer.readstring(buf_75, o_98, length_54)
			o_96 = o_98 + length_54
			list_2[i_37 + 1] = text_28
		end
		local arg_4 = buffer.readu8(buf_75, o_96) ~= 0
		list_2[count_28 + 2] = arg_4
		o_96 += 1
		if o_96 ~= buffer.len(buf_75) then
			error("malformed payload")
		end
		return list_2
	end,
})
local labelled = Flamework.createSerializer({
	serialize = function(v_62)
		local v_63 = v_62[2]
		local buf_76 = buffer.create((if v_63 ~= nil then 2 else 1) + 12)
		local o_99 = 0
		local vector3 = v_62[1]
		buffer.writef32(buf_76, o_99, vector3.X)
		buffer.writef32(buf_76, o_99 + 4, vector3.Y)
		buffer.writef32(buf_76, o_99 + 8, vector3.Z)
		local v_64 = v_62[2]
		buffer.writeu8(buf_76, o_99 + 12, if v_64 ~= nil then 1 else 0)
		o_99 += 13
		if v_64 ~= nil then
			buffer.writeu8(buf_76, o_99, v_64)
			o_99 += 1
		end
		return buf_76
	end,
	deserialize = function(buf_77)
		local o_100 = 0
		local arg_5 = Vector3.new(buffer.readf32(buf_77, o_100), buffer.readf32(buf_77, o_100 + 4), buffer.readf32(buf_77, o_100 + 8))
		local present_8 = buffer.readu8(buf_77, o_100 + 12) ~= 0
		local value_39
		o_100 += 13
		if present_8 then
			value_39 = buffer.readu8(buf_77, o_100)
			o_100 += 1
		end
		local value_40 = { arg_5, value_39 }
		if o_100 ~= buffer.len(buf_77) then
			error("malformed payload")
		end
		return value_40
	end,
})
-- Promises unwrap.
local promised = Flamework.createSerializer({
	serialize = function(v_65)
		local buf_78 = buffer.create(16)
		buffer.writef64(buf_78, 0, v_65.x)
		buffer.writef64(buf_78, 8, v_65.y)
		return buf_78
	end,
	deserialize = function(buf_79)
		local value_41 = {
			x = buffer.readf64(buf_79, 0),
			y = buffer.readf64(buf_79, 8),
		}
		if buffer.len(buf_79) ~= 16 then
			error("malformed payload")
		end
		return value_41
	end,
})
-- Versioned.
local versioned = Flamework.createSerializer({
	version = 3,
}, {
	version = 3,
	serialize = function(v_66)
		local buf_80 = buffer.create(codec.s_Entity(v_66) + 5)
		buffer.writeu8(buf_80, 0, 3)
		buffer.writeu32(buf_80, 1, 2388057634)
		local blobs_5 = {}
		codec.w_Entity(buf_80, 5, v_66, blobs_5)
		return buf_80, blobs_5
	end,
	deserialize = function(buf_81, blobs_6)
		if buffer.len(buf_81) < 5 then
			error("[Flamework] this buffer is too short for the header of Entity, version 3: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_81, 0) ~= 3 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_81, 0)}, and this serializer of Entity reads version 3: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_81, 1) ~= 2388057634 then
			error("[Flamework] Entity's layout changed since this buffer was written as version 3; bump the version and keep a reader for the old one")
		end
		local value_42, o_101 = codec.r_Entity(buf_81, 5, blobs_6)
		if o_101 ~= buffer.len(buf_81) then
			error("malformed payload")
		end
		return value_42
	end,
})
local versionedNumber = Flamework.createSerializer({
	version = 0,
}, {
	version = 0,
	serialize = function(v_67)
		local buf_82 = buffer.create(13)
		buffer.writeu8(buf_82, 0, 0)
		buffer.writeu32(buf_82, 1, 2209142783)
		buffer.writef64(buf_82, 5, v_67)
		return buf_82
	end,
	deserialize = function(buf_83)
		if buffer.len(buf_83) < 5 then
			error("[Flamework] this buffer is too short for the header of number, version 0: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_83, 0) ~= 0 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_83, 0)}, and this serializer of number reads version 0: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_83, 1) ~= 2209142783 then
			error("[Flamework] number's layout changed since this buffer was written as version 0; bump the version and keep a reader for the old one")
		end
		local value_43 = buffer.readf64(buf_83, 5)
		if buffer.len(buf_83) ~= 13 then
			error("malformed payload")
		end
		return value_43
	end,
})
local VERSION = 7
local versionedConst = Flamework.createSerializer({
	version = VERSION,
}, {
	version = 7,
	serialize = function(v_68)
		local length_55 = #(v_68[2])
		local buf_84 = buffer.create(vsize(length_55) + length_55 + 8 + 5)
		buffer.writeu8(buf_84, 0, 7)
		buffer.writeu32(buf_84, 1, 3163455859)
		local o_102 = 5
		buffer.writef64(buf_84, o_102, v_68[1])
		local text_29 = v_68[2]
		local length_56 = #text_29
		o_102 = vwrite(buf_84, o_102 + 8, length_56)
		buffer.writestring(buf_84, o_102, text_29)
		o_102 += length_56
		return buf_84
	end,
	deserialize = function(buf_85)
		if buffer.len(buf_85) < 5 then
			error("[Flamework] this buffer is too short for the header of [number, string], version 7: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_85, 0) ~= 7 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_85, 0)}, and this serializer of [number, string] reads version 7: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_85, 1) ~= 3163455859 then
			error("[Flamework] [number, string]'s layout changed since this buffer was written as version 7; bump the version and keep a reader for the old one")
		end
		local o_103 = 5
		local arg_6 = buffer.readf64(buf_85, o_103)
		local length_57, o_104 = vread(buf_85, o_103 + 8)
		local text_30 = buffer.readstring(buf_85, o_104, length_57)
		o_103 = o_104 + length_57
		local value_44 = { arg_6, text_30 }
		if o_103 ~= buffer.len(buf_85) then
			error("malformed payload")
		end
		return value_44
	end,
})
return {
	point = point,
	entity = entity,
	derived = derived,
	keys = keys,
	tree = tree,
	chain = chain,
	patch = patch,
	partial = partial,
	record = record,
	levelRecord = levelRecord,
	picked = picked,
	intersection = intersection,
	level = level,
	named = named,
	levels = levels,
	numbers = numbers,
	texts = texts,
	readonlyTexts = readonlyTexts,
	grid = grid,
	set = set,
	fixedSet = fixedSet,
	map = map,
	fixedMap = fixedMap,
	readonlyMap = readonlyMap,
	nested = nested,
	indexed = indexed,
	markers = markers,
	pair = pair,
	optionalTuple = optionalTuple,
	restFirst = restFirst,
	restMiddle = restMiddle,
	labelled = labelled,
	promised = promised,
	versioned = versioned,
	versionedNumber = versionedNumber,
	versionedConst = versionedConst,
}
