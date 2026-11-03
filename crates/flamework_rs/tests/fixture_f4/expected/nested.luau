-- Compiled with roblox-ts v3.0.0
local TS = _G[script]
local t = TS.import(script, TS.getModule(script, "@rbxts", "t").lib.ts).t
--[[
	A serializer macro whose argument can be another one: the inner value is built (and printed) first.

	@metadata macro
]]
local function wrap(inner, meta)
	return { inner, meta }
end
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
local guard = t.set(t.string)
local guard_1 = t.array(t.string)
codec.s_A = function(v)
	local v_1 = v.list
	local size = 1
	if guard_1(v_1) then
		local n_3 = #v_1
		local size_1 = vsize(n_3)
		for i = 1, n_3 do
			local item = v_1[i]
			if item == nil then
				error(`[Flamework] the array has no value at A.list[{i - 1}]`)
			end
			local length = #item
			size_1 += vsize(length) + length
		end
		size += size_1
	elseif guard(v_1) then
		local size_2 = 0
		local n_4 = 0
		for item_1 in v_1 do
			n_4 += 1
			local length_1 = #item_1
			size_2 += vsize(length_1) + length_1
		end
		size_2 += vsize(n_4)
		size += size_2
	elseif type(v_1) == "table" then
		local size_3 = 0
		local n_5 = 0
		for key, entry in v_1 do
			n_5 += 1
			local length_2 = #key
			size_3 += vsize(length_2) + length_2 + codec.s_A(entry)
		end
		size_3 += vsize(n_5)
		size += size_3
	end
	return size + 1
end
codec.w_A = function(buf_2, o_2, v)
	buffer.writeu8(buf_2, o_2, v.a)
	local v_2 = v.list
	o_2 += 1
	if guard_1(v_2) then
		buffer.writeu8(buf_2, o_2, 0)
		o_2 = vwrite(buf_2, o_2 + 1, #v_2)
		for i_1 = 1, #v_2 do
			local item_2 = v_2[i_1]
			local length_3 = #item_2
			o_2 = vwrite(buf_2, o_2, length_3)
			buffer.writestring(buf_2, o_2, item_2)
			o_2 += length_3
		end
	elseif guard(v_2) then
		buffer.writeu8(buf_2, o_2, 1)
		local n_6 = 0
		for _ in v_2 do
			n_6 += 1
		end
		o_2 = vwrite(buf_2, o_2 + 1, n_6)
		for item_3 in v_2 do
			local length_4 = #item_3
			o_2 = vwrite(buf_2, o_2, length_4)
			buffer.writestring(buf_2, o_2, item_3)
			o_2 += length_4
		end
	elseif type(v_2) == "table" then
		buffer.writeu8(buf_2, o_2, 2)
		local n_7 = 0
		for _ in v_2 do
			n_7 += 1
		end
		o_2 = vwrite(buf_2, o_2 + 1, n_7)
		for key_1, entry_1 in v_2 do
			local length_5 = #key_1
			o_2 = vwrite(buf_2, o_2, length_5)
			buffer.writestring(buf_2, o_2, key_1)
			o_2 += length_5
			o_2 = codec.w_A(buf_2, o_2, entry_1)
		end
	else
		error("value matches none of the union's members")
	end
	return o_2
end
codec.r_A = function(buf_2, o_2)
	local a_1 = buffer.readu8(buf_2, o_2)
	local tag = buffer.readu8(buf_2, o_2 + 1)
	local value
	o_2 += 2
	if tag == 0 then
		local count, o_3 = vread(buf_2, o_2)
		o_2 = o_3
		if count > buffer.len(buf_2) - o_2 then
			error("malformed payload")
		end
		local array = table.create(count)
		for i_2 = 1, count do
			local length_6, o_4 = vread(buf_2, o_2)
			local text = buffer.readstring(buf_2, o_4, length_6)
			o_2 = o_4 + length_6
			array[i_2] = text
		end
		value = array
	elseif tag == 1 then
		local count_1, o_5 = vread(buf_2, o_2)
		o_2 = o_5
		if count_1 > buffer.len(buf_2) - o_2 then
			error("malformed payload")
		end
		local set = {}
		for _ = 1, count_1 do
			local length_7, o_6 = vread(buf_2, o_2)
			local text_1 = buffer.readstring(buf_2, o_6, length_7)
			o_2 = o_6 + length_7
			set[text_1] = true
		end
		value = set
	elseif tag == 2 then
		local count_2, o_7 = vread(buf_2, o_2)
		o_2 = o_7
		if count_2 * 4 > buffer.len(buf_2) - o_2 then
			error("malformed payload")
		end
		local map = {}
		for _1 = 1, count_2 do
			local length_8, o_8 = vread(buf_2, o_2)
			local text_2 = buffer.readstring(buf_2, o_8, length_8)
			o_2 = o_8 + length_8
			local value_1, o_9 = codec.r_A(buf_2, o_2)
			map[text_2] = value_1
			o_2 = o_9
		end
		value = map
	else
		error("malformed payload")
	end
	local value_2 = {
		a = a_1,
		list = value,
	}
	return value_2, o_2
end
codec.s_B = function(v_3)
	local length_9 = #v_3.b
	local v_4 = v_3.list
	local size_4 = 1
	if type(v_4) == "string" then
		local length_10 = #v_4
		size_4 += vsize(length_10) + length_10
	elseif type(v_4) == "table" then
		local n_8 = #v_4
		local size_5 = vsize(n_8)
		for i_3 = 1, n_8 do
			local item_4 = v_4[i_3]
			if item_4 == nil then
				error(`[Flamework] the array has no value at B.list[{i_3 - 1}]`)
			end
			size_5 += codec.s_A(item_4)
		end
		size_4 += size_5
	end
	return vsize(length_9) + length_9 + size_4
end
codec.w_B = function(buf_3, o_10, v_3)
	local text_3 = v_3.b
	local length_11 = #text_3
	o_10 = vwrite(buf_3, o_10, length_11)
	buffer.writestring(buf_3, o_10, text_3)
	o_10 += length_11
	local v_5 = v_3.list
	if type(v_5) == "string" then
		buffer.writeu8(buf_3, o_10, 1)
		local length_12 = #v_5
		o_10 = vwrite(buf_3, o_10 + 1, length_12)
		buffer.writestring(buf_3, o_10, v_5)
		o_10 += length_12
	elseif type(v_5) == "table" then
		buffer.writeu8(buf_3, o_10, 0)
		o_10 = vwrite(buf_3, o_10 + 1, #v_5)
		for i_4 = 1, #v_5 do
			local item_5 = v_5[i_4]
			o_10 = codec.w_A(buf_3, o_10, item_5)
		end
	else
		error("value matches none of the union's members")
	end
	return o_10
end
codec.r_B = function(buf_3, o_10)
	local length_13, o_11 = vread(buf_3, o_10)
	local text_4 = buffer.readstring(buf_3, o_11, length_13)
	o_10 = o_11 + length_13
	local tag_1 = buffer.readu8(buf_3, o_10)
	local value_3
	o_10 += 1
	if tag_1 == 0 then
		local count_3, o_12 = vread(buf_3, o_10)
		o_10 = o_12
		if count_3 * 3 > buffer.len(buf_3) - o_10 then
			error("malformed payload")
		end
		local array_1 = table.create(count_3)
		for i_5 = 1, count_3 do
			local value_4, o_13 = codec.r_A(buf_3, o_10)
			array_1[i_5] = value_4
			o_10 = o_13
		end
		value_3 = array_1
	elseif tag_1 == 1 then
		local length_14, o_14 = vread(buf_3, o_10)
		local text_5 = buffer.readstring(buf_3, o_14, length_14)
		o_10 = o_14 + length_14
		value_3 = text_5
	else
		error("malformed payload")
	end
	local value_5 = {
		b = text_4,
		list = value_3,
	}
	return value_5, o_10
end
local nested = wrap(wrap(0, {
	serialize = function(v_6)
		local buf_4 = buffer.create(codec.s_B(v_6))
		codec.w_B(buf_4, 0, v_6)
		return buf_4
	end,
	deserialize = function(buf_5)
		local value_6, o_15 = codec.r_B(buf_5, 0)
		if o_15 ~= buffer.len(buf_5) then
			error("malformed payload")
		end
		return value_6
	end,
}), {
	serialize = function(v_7)
		local buf_6 = buffer.create(codec.s_A(v_7))
		codec.w_A(buf_6, 0, v_7)
		return buf_6
	end,
	deserialize = function(buf_7)
		local value_7, o_16 = codec.r_A(buf_7, 0)
		if o_16 ~= buffer.len(buf_7) then
			error("malformed payload")
		end
		return value_7
	end,
})
codec.s_Map_string_A = function(v_8)
	local size_6 = 0
	local n_9 = 0
	for key_2, entry_2 in v_8 do
		n_9 += 1
		local length_15 = #key_2
		size_6 += vsize(length_15) + length_15 + codec.s_A(entry_2)
	end
	size_6 += vsize(n_9)
	return size_6
end
codec.w_Map_string_A = function(buf_8, o_17, v_8)
	local n_10 = 0
	for _ in v_8 do
		n_10 += 1
	end
	o_17 = vwrite(buf_8, o_17, n_10)
	for key_3, entry_3 in v_8 do
		local length_16 = #key_3
		o_17 = vwrite(buf_8, o_17, length_16)
		buffer.writestring(buf_8, o_17, key_3)
		o_17 += length_16
		o_17 = codec.w_A(buf_8, o_17, entry_3)
	end
	return o_17
end
codec.r_Map_string_A = function(buf_8, o_17)
	local count_4, o_18 = vread(buf_8, o_17)
	o_17 = o_18
	if count_4 * 4 > buffer.len(buf_8) - o_17 then
		error("malformed payload")
	end
	local map_1 = {}
	for _2 = 1, count_4 do
		local length_17, o_19 = vread(buf_8, o_17)
		local text_6 = buffer.readstring(buf_8, o_19, length_17)
		o_17 = o_19 + length_17
		local value_8, o_20 = codec.r_A(buf_8, o_17)
		map_1[text_6] = value_8
		o_17 = o_20
	end
	return map_1, o_17
end
local deeper = wrap({ wrap(wrap(1, {
	serialize = function(v_9)
		local buf_9 = buffer.create(codec.s_Map_string_A(v_9))
		codec.w_Map_string_A(buf_9, 0, v_9)
		return buf_9
	end,
	deserialize = function(buf_10)
		local value_9, o_21 = codec.r_Map_string_A(buf_10, 0)
		if o_21 ~= buffer.len(buf_10) then
			error("malformed payload")
		end
		return value_9
	end,
}), {
	serialize = function(v_10)
		local n_11 = #v_10
		local size_7 = vsize(n_11)
		for i_6 = 1, n_11 do
			local item_6 = v_10[i_6]
			if item_6 == nil then
				error(`[Flamework] the array has no value at value[{i_6 - 1}]`)
			end
			size_7 += codec.s_B(item_6)
		end
		local buf_11 = buffer.create(size_7)
		local o_22 = 0
		o_22 = vwrite(buf_11, o_22, #v_10)
		for i_7 = 1, #v_10 do
			local item_7 = v_10[i_7]
			o_22 = codec.w_B(buf_11, o_22, item_7)
		end
		return buf_11
	end,
	deserialize = function(buf_12)
		local o_23 = 0
		local count_5, o_24 = vread(buf_12, o_23)
		o_23 = o_24
		if count_5 * 3 > buffer.len(buf_12) - o_23 then
			error("malformed payload")
		end
		local array_2 = table.create(count_5)
		for i_8 = 1, count_5 do
			local value_10, o_25 = codec.r_B(buf_12, o_23)
			array_2[i_8] = value_10
			o_23 = o_25
		end
		if o_23 ~= buffer.len(buf_12) then
			error("malformed payload")
		end
		return array_2
	end,
}) }, {
	serialize = function(v_11)
		local size_8 = 1
		if v_11.a ~= nil then
			size_8 += codec.s_A(v_11)
		elseif v_11.b ~= nil then
			size_8 += codec.s_B(v_11)
		end
		local buf_13 = buffer.create(size_8)
		local o_26 = 0
		if v_11.a ~= nil then
			buffer.writeu8(buf_13, o_26, 0)
			o_26 = codec.w_A(buf_13, o_26 + 1, v_11)
		elseif v_11.b ~= nil then
			buffer.writeu8(buf_13, o_26, 1)
			o_26 = codec.w_B(buf_13, o_26 + 1, v_11)
		else
			error("value matches none of the union's members")
		end
		return buf_13
	end,
	deserialize = function(buf_14)
		local o_27 = 0
		local tag_2 = buffer.readu8(buf_14, o_27)
		local value_11
		o_27 += 1
		if tag_2 == 0 then
			local value_12, o_28 = codec.r_A(buf_14, o_27)
			value_11 = value_12
			o_27 = o_28
		elseif tag_2 == 1 then
			local value_13, o_29 = codec.r_B(buf_14, o_27)
			value_11 = value_13
			o_27 = o_29
		else
			error("malformed payload")
		end
		if o_27 ~= buffer.len(buf_14) then
			error("malformed payload")
		end
		return value_11
	end,
})
return {
	nested = nested,
	deeper = deeper,
}
