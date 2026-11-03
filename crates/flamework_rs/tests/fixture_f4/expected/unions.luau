-- Compiled with roblox-ts v3.0.0
local TS = _G[script]
local t = TS.import(script, TS.getModule(script, "@rbxts", "t").lib.ts).t
local Flamework = TS.import(script, TS.getModule(script, "@flamework-experimental", "core").out).Flamework
local codec = {}
local vsize = function(n_1)
	return if n_1 < 128 then 1 elseif n_1 < 16384 then 2 elseif n_1 < 2097152 then 3 elseif n_1 < 268435456 then 4 else 5
end
local vwrite = function(buf, o, n_2)
	while n_2 >= 128 do
		buffer.writeu8(buf, o, n_2 % 128 + 128)
		o += 1
		n_2 = math.floor(n_2 / 128)
	end
	buffer.writeu8(buf, o, n_2)
	return o + 1
end
local vread = function(buf_1, o_1)
	local n_3 = 0
	local scale = 1
	while true do
		local b_1 = buffer.readu8(buf_1, o_1)
		o_1 += 1
		n_3 += b_1 % 128 * scale
		if b_1 < 128 then
			return n_3, o_1
		end
		scale *= 128
		if scale > 268435456 then
			error("malformed payload")
		end
	end
end
codec.s_Wallet = function(v_1)
	local size = 1
	if v_1.Coins ~= nil then
		size += 8
	elseif v_1.Items ~= nil then
		local array = v_1.Items
		local n_4 = #array
		local size_1 = vsize(n_4)
		for i = 1, n_4 do
			local item = array[i]
			if item == nil then
				error(`[Flamework] the array has no value at Wallet.Items[{i - 1}]`)
			end
			local length = #item
			size_1 += vsize(length) + length
		end
		size += size_1
	end
	return size
end
codec.w_Wallet = function(buf_2, o_2, v_1)
	if v_1.Coins ~= nil then
		buffer.writeu8(buf_2, o_2, 0)
		buffer.writef64(buf_2, o_2 + 1, v_1.Coins)
		o_2 += 9
	elseif v_1.Items ~= nil then
		buffer.writeu8(buf_2, o_2, 1)
		local array_1 = v_1.Items
		o_2 = vwrite(buf_2, o_2 + 1, #array_1)
		for i_1 = 1, #array_1 do
			local item_1 = array_1[i_1]
			local length_1 = #item_1
			o_2 = vwrite(buf_2, o_2, length_1)
			buffer.writestring(buf_2, o_2, item_1)
			o_2 += length_1
		end
	else
		error("value matches none of the union's members")
	end
	return o_2
end
codec.r_Wallet = function(buf_2, o_2)
	local tag = buffer.readu8(buf_2, o_2)
	local value_1
	o_2 += 1
	if tag == 0 then
		local Coins_1 = buffer.readf64(buf_2, o_2)
		value_1 = {
			Coins = Coins_1,
		}
		o_2 += 8
	elseif tag == 1 then
		local count, o_3 = vread(buf_2, o_2)
		o_2 = o_3
		if count > buffer.len(buf_2) - o_2 then
			error("malformed payload")
		end
		local array_2 = table.create(count)
		for i_2 = 1, count do
			local length_2, o_4 = vread(buf_2, o_2)
			local text = buffer.readstring(buf_2, o_4, length_2)
			o_2 = o_4 + length_2
			array_2[i_2] = text
		end
		value_1 = {
			Items = array_2,
		}
	else
		error("malformed payload")
	end
	return value_1, o_2
end
local wallet = Flamework.createSerializer({
	serialize = function(v_2)
		local buf_3 = buffer.create(codec.s_Wallet(v_2))
		codec.w_Wallet(buf_3, 0, v_2)
		return buf_3
	end,
	deserialize = function(buf_4)
		local value_2, o_5 = codec.r_Wallet(buf_4, 0)
		if o_5 ~= buffer.len(buf_4) then
			error("malformed payload")
		end
		return value_2
	end,
})
codec.s_Shape = function(v_3)
	local size_2 = 1
	if v_3.kind == "circle" then
		size_2 += 8
	elseif v_3.kind == "rect" then
		size_2 += 16
	end
	return size_2
end
codec.w_Shape = function(buf_5, o_6, v_3)
	if v_3.kind == "circle" then
		buffer.writeu8(buf_5, o_6, 0)
		buffer.writef64(buf_5, o_6 + 1, v_3.radius)
		o_6 += 9
	elseif v_3.kind == "rect" then
		buffer.writeu8(buf_5, o_6, 1)
		buffer.writef64(buf_5, o_6 + 1, v_3.w)
		buffer.writef64(buf_5, o_6 + 9, v_3.h)
		o_6 += 17
	else
		error("value matches none of the union's members")
	end
	return o_6
end
codec.r_Shape = function(buf_5, o_6)
	local tag_1 = buffer.readu8(buf_5, o_6)
	local value_3
	o_6 += 1
	if tag_1 == 0 then
		local radius_1 = buffer.readf64(buf_5, o_6)
		value_3 = {
			kind = "circle",
			radius = radius_1,
		}
		o_6 += 8
	elseif tag_1 == 1 then
		local w_1 = buffer.readf64(buf_5, o_6)
		local h_1 = buffer.readf64(buf_5, o_6 + 8)
		value_3 = {
			kind = "rect",
			w = w_1,
			h = h_1,
		}
		o_6 += 16
	else
		error("malformed payload")
	end
	return value_3, o_6
end
local shape = Flamework.createSerializer({
	serialize = function(v_4)
		local buf_6 = buffer.create(codec.s_Shape(v_4))
		codec.w_Shape(buf_6, 0, v_4)
		return buf_6
	end,
	deserialize = function(buf_7)
		local value_4, o_7 = codec.r_Shape(buf_7, 0)
		if o_7 ~= buffer.len(buf_7) then
			error("malformed payload")
		end
		return value_4
	end,
})
local literals = { 5, "lit" }
local literalIndex = {
	[5] = 0,
	lit = 1,
}
codec.s_Mixed = function(v_5)
	local size_3 = 1
	if typeof(v_5) == "Instance" then
		size_3 += 4
	elseif typeof(v_5) == "Vector3" then
		size_3 += 12
	elseif type(v_5) == "table" and v_5.kind == "a" then
		size_3 += 8
	elseif type(v_5) == "table" and v_5.kind == "b" then
		local length_3 = #v_5.s
		size_3 += vsize(length_3) + length_3
	elseif literalIndex[v_5] ~= nil then
		size_3 += 1
	elseif type(v_5) == "table" then
		local n_5 = #v_5
		size_3 += vsize(n_5) + n_5 * 8
	end
	return size_3
end
codec.w_Mixed = function(buf_8, o_8, v_5, blobs)
	if typeof(v_5) == "Instance" then
		buffer.writeu8(buf_8, o_8, 0)
		if v_5 ~= nil then
			table.insert(blobs, v_5)
			buffer.writeu32(buf_8, o_8 + 1, #blobs)
		else
			buffer.writeu32(buf_8, o_8 + 1, 0)
		end
		o_8 += 5
	elseif typeof(v_5) == "Vector3" then
		buffer.writeu8(buf_8, o_8, 1)
		buffer.writef32(buf_8, o_8 + 1, v_5.X)
		buffer.writef32(buf_8, o_8 + 5, v_5.Y)
		buffer.writef32(buf_8, o_8 + 9, v_5.Z)
		o_8 += 13
	elseif type(v_5) == "table" and v_5.kind == "a" then
		buffer.writeu8(buf_8, o_8, 2)
		buffer.writef64(buf_8, o_8 + 1, v_5.v)
		o_8 += 9
	elseif type(v_5) == "table" and v_5.kind == "b" then
		buffer.writeu8(buf_8, o_8, 3)
		local text_1 = v_5.s
		local length_4 = #text_1
		o_8 = vwrite(buf_8, o_8 + 1, length_4)
		buffer.writestring(buf_8, o_8, text_1)
		o_8 += length_4
	elseif literalIndex[v_5] ~= nil then
		buffer.writeu8(buf_8, o_8, 5)
		local index = literalIndex[v_5]
		if index == nil then
			error("value is not one of the literals its type allows")
		end
		buffer.writeu8(buf_8, o_8 + 1, index)
		o_8 += 2
	elseif type(v_5) == "table" then
		buffer.writeu8(buf_8, o_8, 4)
		o_8 = vwrite(buf_8, o_8 + 1, #v_5)
		for i_3 = 1, #v_5 do
			local item_2 = v_5[i_3]
			if item_2 == nil then
				error(`[Flamework] the array has no value at Mixed[{i_3 - 1}]`)
			end
			buffer.writef64(buf_8, o_8, item_2)
			o_8 += 8
		end
	else
		error("value matches none of the union's members")
	end
	return o_8
end
codec.r_Mixed = function(buf_8, o_8, blobs)
	local tag_2 = buffer.readu8(buf_8, o_8)
	local value_5
	o_8 += 1
	if tag_2 == 0 then
		value_5 = blobs[buffer.readu32(buf_8, o_8)]
		o_8 += 4
	elseif tag_2 == 1 then
		value_5 = Vector3.new(buffer.readf32(buf_8, o_8), buffer.readf32(buf_8, o_8 + 4), buffer.readf32(buf_8, o_8 + 8))
		o_8 += 12
	elseif tag_2 == 2 then
		local v_6 = buffer.readf64(buf_8, o_8)
		value_5 = {
			kind = "a",
			v = v_6,
		}
		o_8 += 8
	elseif tag_2 == 3 then
		local length_5, o_9 = vread(buf_8, o_8)
		local text_2 = buffer.readstring(buf_8, o_9, length_5)
		o_8 = o_9 + length_5
		value_5 = {
			kind = "b",
			s = text_2,
		}
	elseif tag_2 == 4 then
		local count_1, o_10 = vread(buf_8, o_8)
		o_8 = o_10
		if count_1 * 8 > buffer.len(buf_8) - o_8 then
			error("malformed payload")
		end
		local array_3 = table.create(count_1)
		for i_4 = 1, count_1 do
			array_3[i_4] = buffer.readf64(buf_8, o_8)
			o_8 += 8
		end
		value_5 = array_3
	elseif tag_2 == 5 then
		local literal = literals[buffer.readu8(buf_8, o_8) + 1]
		if literal == nil then
			error("malformed payload")
		end
		value_5 = literal
		o_8 += 1
	else
		error("malformed payload")
	end
	return value_5, o_8
end
local mixed = Flamework.createSerializer({
	serialize = function(v_7)
		local buf_9 = buffer.create(codec.s_Mixed(v_7))
		local blobs_1 = {}
		codec.w_Mixed(buf_9, 0, v_7, blobs_1)
		return buf_9, blobs_1
	end,
	deserialize = function(buf_10, blobs_2)
		local value_6, o_11 = codec.r_Mixed(buf_10, 0, blobs_2)
		if o_11 ~= buffer.len(buf_10) then
			error("malformed payload")
		end
		return value_6
	end,
})
local sortOf = Flamework.createSerializer({
	serialize = function(v_8)
		local size_4 = 1
		if type(v_8) == "number" then
			size_4 += if v_8 < 34359738368 and 1 / v_8 > 0 and v_8 % 1 == 0 then vsize(v_8) else 8
		elseif type(v_8) == "string" then
			local length_6 = #v_8
			size_4 += vsize(length_6) + length_6
		end
		local buf_11 = buffer.create(size_4)
		local o_12 = 0
		if type(v_8) == "number" then
			if v_8 < 34359738368 and 1 / v_8 > 0 and v_8 % 1 == 0 then
				buffer.writeu8(buf_11, o_12, 2)
				o_12 = vwrite(buf_11, o_12 + 1, v_8)
			else
				buffer.writeu8(buf_11, o_12, 0)
				buffer.writef64(buf_11, o_12 + 1, v_8)
				o_12 += 9
			end
		elseif type(v_8) == "string" then
			buffer.writeu8(buf_11, o_12, 1)
			local length_7 = #v_8
			o_12 = vwrite(buf_11, o_12 + 1, length_7)
			buffer.writestring(buf_11, o_12, v_8)
			o_12 += length_7
		else
			error("value matches none of the union's members")
		end
		return buf_11
	end,
	deserialize = function(buf_12)
		local o_13 = 0
		local tag_3 = buffer.readu8(buf_12, o_13)
		local value_7
		o_13 += 1
		if tag_3 == 0 then
			value_7 = buffer.readf64(buf_12, o_13)
			o_13 += 8
		elseif tag_3 == 1 then
			local length_8, o_14 = vread(buf_12, o_13)
			local text_3 = buffer.readstring(buf_12, o_14, length_8)
			o_13 = o_14 + length_8
			value_7 = text_3
		elseif tag_3 == 2 then
			local n_6, o_15 = vread(buf_12, o_13)
			value_7 = n_6
			o_13 = o_15
		else
			error("malformed payload")
		end
		if o_13 ~= buffer.len(buf_12) then
			error("malformed payload")
		end
		return value_7
	end,
})
local sortOfReversed = Flamework.createSerializer({
	serialize = function(v_9)
		local size_5 = 1
		if type(v_9) == "string" then
			local length_9 = #v_9
			size_5 += vsize(length_9) + length_9
		elseif type(v_9) == "number" then
			size_5 += if v_9 < 34359738368 and 1 / v_9 > 0 and v_9 % 1 == 0 then vsize(v_9) else 8
		end
		local buf_13 = buffer.create(size_5)
		local o_16 = 0
		if type(v_9) == "string" then
			buffer.writeu8(buf_13, o_16, 0)
			local length_10 = #v_9
			o_16 = vwrite(buf_13, o_16 + 1, length_10)
			buffer.writestring(buf_13, o_16, v_9)
			o_16 += length_10
		elseif type(v_9) == "number" then
			if v_9 < 34359738368 and 1 / v_9 > 0 and v_9 % 1 == 0 then
				buffer.writeu8(buf_13, o_16, 2)
				o_16 = vwrite(buf_13, o_16 + 1, v_9)
			else
				buffer.writeu8(buf_13, o_16, 1)
				buffer.writef64(buf_13, o_16 + 1, v_9)
				o_16 += 9
			end
		else
			error("value matches none of the union's members")
		end
		return buf_13
	end,
	deserialize = function(buf_14)
		local o_17 = 0
		local tag_4 = buffer.readu8(buf_14, o_17)
		local value_8
		o_17 += 1
		if tag_4 == 0 then
			local length_11, o_18 = vread(buf_14, o_17)
			local text_4 = buffer.readstring(buf_14, o_18, length_11)
			o_17 = o_18 + length_11
			value_8 = text_4
		elseif tag_4 == 1 then
			value_8 = buffer.readf64(buf_14, o_17)
			o_17 += 8
		elseif tag_4 == 2 then
			local n_7, o_19 = vread(buf_14, o_17)
			value_8 = n_7
			o_17 = o_19
		else
			error("malformed payload")
		end
		if o_17 ~= buffer.len(buf_14) then
			error("malformed payload")
		end
		return value_8
	end,
})
local narrow = Flamework.createSerializer({
	serialize = function(v_10)
		local size_6 = 1
		if type(v_10) == "number" and (v_10 >= 0 and v_10 <= 65535 and v_10 % 1 == 0) then
			size_6 += 2
		elseif type(v_10) == "number" then
			size_6 += if v_10 < 34359738368 and 1 / v_10 > 0 and v_10 % 1 == 0 then vsize(v_10) else 8
		end
		local buf_15 = buffer.create(size_6)
		local o_20 = 0
		if type(v_10) == "number" and (v_10 >= 0 and v_10 <= 65535 and v_10 % 1 == 0) then
			buffer.writeu8(buf_15, o_20, 0)
			buffer.writeu16(buf_15, o_20 + 1, v_10)
			o_20 += 3
		elseif type(v_10) == "number" then
			if v_10 < 34359738368 and 1 / v_10 > 0 and v_10 % 1 == 0 then
				buffer.writeu8(buf_15, o_20, 2)
				o_20 = vwrite(buf_15, o_20 + 1, v_10)
			else
				buffer.writeu8(buf_15, o_20, 1)
				buffer.writef64(buf_15, o_20 + 1, v_10)
				o_20 += 9
			end
		else
			error("value matches none of the union's members")
		end
		return buf_15
	end,
	deserialize = function(buf_16)
		local o_21 = 0
		local tag_5 = buffer.readu8(buf_16, o_21)
		local value_9
		o_21 += 1
		if tag_5 == 0 then
			value_9 = buffer.readu16(buf_16, o_21)
			o_21 += 2
		elseif tag_5 == 1 then
			value_9 = buffer.readf64(buf_16, o_21)
			o_21 += 8
		elseif tag_5 == 2 then
			local n_8, o_22 = vread(buf_16, o_21)
			value_9 = n_8
			o_21 = o_22
		else
			error("malformed payload")
		end
		if o_21 ~= buffer.len(buf_16) then
			error("malformed payload")
		end
		return value_9
	end,
})
local narrowOnly = Flamework.createSerializer({
	serialize = function(v_11)
		local size_7 = 1
		if type(v_11) == "number" and (v_11 >= 0 and v_11 <= 255 and v_11 % 1 == 0) then
			size_7 += 1
		elseif type(v_11) == "string" then
			local length_12 = #v_11
			size_7 += vsize(length_12) + length_12
		end
		local buf_17 = buffer.create(size_7)
		local o_23 = 0
		if type(v_11) == "number" and (v_11 >= 0 and v_11 <= 255 and v_11 % 1 == 0) then
			buffer.writeu8(buf_17, o_23, 0)
			buffer.writeu8(buf_17, o_23 + 1, v_11)
			o_23 += 2
		elseif type(v_11) == "string" then
			buffer.writeu8(buf_17, o_23, 1)
			local length_13 = #v_11
			o_23 = vwrite(buf_17, o_23 + 1, length_13)
			buffer.writestring(buf_17, o_23, v_11)
			o_23 += length_13
		else
			error("value matches none of the union's members")
		end
		return buf_17
	end,
	deserialize = function(buf_18)
		local o_24 = 0
		local tag_6 = buffer.readu8(buf_18, o_24)
		local value_10
		o_24 += 1
		if tag_6 == 0 then
			value_10 = buffer.readu8(buf_18, o_24)
			o_24 += 1
		elseif tag_6 == 1 then
			local length_14, o_25 = vread(buf_18, o_24)
			local text_5 = buffer.readstring(buf_18, o_25, length_14)
			o_24 = o_25 + length_14
			value_10 = text_5
		else
			error("malformed payload")
		end
		if o_24 ~= buffer.len(buf_18) then
			error("malformed payload")
		end
		return value_10
	end,
})
codec.s_Choice = function(v_12)
	local size_8 = 1
	if type(v_12) == "string" then
		local length_15 = #v_12
		size_8 += vsize(length_15) + length_15
	elseif type(v_12) == "number" then
		size_8 += if v_12 < 34359738368 and 1 / v_12 > 0 and v_12 % 1 == 0 then vsize(v_12) else 8
	elseif type(v_12) == "table" and v_12.alpha ~= nil then
		size_8 += 8
	end
	return size_8
end
codec.w_Choice = function(buf_19, o_26, v_12)
	if type(v_12) == "string" then
		buffer.writeu8(buf_19, o_26, 0)
		local length_16 = #v_12
		o_26 = vwrite(buf_19, o_26 + 1, length_16)
		buffer.writestring(buf_19, o_26, v_12)
		o_26 += length_16
	elseif type(v_12) == "number" then
		if v_12 < 34359738368 and 1 / v_12 > 0 and v_12 % 1 == 0 then
			buffer.writeu8(buf_19, o_26, 3)
			o_26 = vwrite(buf_19, o_26 + 1, v_12)
		else
			buffer.writeu8(buf_19, o_26, 1)
			buffer.writef64(buf_19, o_26 + 1, v_12)
			o_26 += 9
		end
	elseif type(v_12) == "table" and v_12.alpha ~= nil then
		buffer.writeu8(buf_19, o_26, 2)
		buffer.writef64(buf_19, o_26 + 1, v_12.alpha)
		o_26 += 9
	else
		error("value matches none of the union's members")
	end
	return o_26
end
codec.r_Choice = function(buf_19, o_26)
	local tag_7 = buffer.readu8(buf_19, o_26)
	local value_11
	o_26 += 1
	if tag_7 == 0 then
		local length_17, o_27 = vread(buf_19, o_26)
		local text_6 = buffer.readstring(buf_19, o_27, length_17)
		o_26 = o_27 + length_17
		value_11 = text_6
	elseif tag_7 == 1 then
		value_11 = buffer.readf64(buf_19, o_26)
		o_26 += 8
	elseif tag_7 == 2 then
		local alpha_1 = buffer.readf64(buf_19, o_26)
		value_11 = {
			alpha = alpha_1,
		}
		o_26 += 8
	elseif tag_7 == 3 then
		local n_9, o_28 = vread(buf_19, o_26)
		value_11 = n_9
		o_26 = o_28
	else
		error("malformed payload")
	end
	return value_11, o_26
end
local choice = Flamework.createSerializer({
	serialize = function(v_13)
		local buf_20 = buffer.create(codec.s_Choice(v_13))
		codec.w_Choice(buf_20, 0, v_13)
		return buf_20
	end,
	deserialize = function(buf_21)
		local value_12, o_29 = codec.r_Choice(buf_21, 0)
		if o_29 ~= buffer.len(buf_21) then
			error("malformed payload")
		end
		return value_12
	end,
})
codec.s_Box = function(v_14)
	local v_15 = v_14.value
	local size_9 = 1
	if v_15.alpha ~= nil then
		size_9 += 8
	elseif v_15.zed ~= nil then
		size_9 += 1
	end
	return size_9
end
codec.w_Box = function(buf_22, o_30, v_14)
	local v_16 = v_14.value
	if v_16.alpha ~= nil then
		buffer.writeu8(buf_22, o_30, 0)
		buffer.writef64(buf_22, o_30 + 1, v_16.alpha)
		o_30 += 9
	elseif v_16.zed ~= nil then
		buffer.writeu8(buf_22, o_30, 1)
		buffer.writeu8(buf_22, o_30 + 1, if v_16.zed then 1 else 0)
		o_30 += 2
	else
		error("value matches none of the union's members")
	end
	return o_30
end
codec.r_Box = function(buf_22, o_30)
	local tag_8 = buffer.readu8(buf_22, o_30)
	local value_13
	o_30 += 1
	if tag_8 == 0 then
		local alpha_2 = buffer.readf64(buf_22, o_30)
		value_13 = {
			alpha = alpha_2,
		}
		o_30 += 8
	elseif tag_8 == 1 then
		local zed_1 = buffer.readu8(buf_22, o_30) ~= 0
		value_13 = {
			zed = zed_1,
		}
		o_30 += 1
	else
		error("malformed payload")
	end
	local value_14 = {
		value = value_13,
	}
	return value_14, o_30
end
local boxes = Flamework.createSerializer({
	serialize = function(v_17)
		local buf_23 = buffer.create(codec.s_Box(v_17))
		codec.w_Box(buf_23, 0, v_17)
		return buf_23
	end,
	deserialize = function(buf_24)
		local value_15, o_31 = codec.r_Box(buf_24, 0)
		if o_31 ~= buffer.len(buf_24) then
			error("malformed payload")
		end
		return value_15
	end,
})
local unwritten = Flamework.createSerializer({
	serialize = function(v_18)
		local n_10 = #v_18
		local size_10 = vsize(n_10)
		for i_5 = 1, n_10 do
			local item_3 = v_18[i_5]
			if item_3 == nil then
				error(`[Flamework] the array has no value at value[{i_5 - 1}]`)
			end
			local size_11 = 1
			if item_3.alpha ~= nil then
				size_11 += 8
			elseif item_3.zed ~= nil then
				size_11 += 1
			end
			size_10 += size_11
		end
		local buf_25 = buffer.create(size_10)
		local o_32 = 0
		o_32 = vwrite(buf_25, o_32, #v_18)
		for i_6 = 1, #v_18 do
			local item_4 = v_18[i_6]
			if item_4.alpha ~= nil then
				buffer.writeu8(buf_25, o_32, 0)
				buffer.writef64(buf_25, o_32 + 1, item_4.alpha)
				o_32 += 9
			elseif item_4.zed ~= nil then
				buffer.writeu8(buf_25, o_32, 1)
				buffer.writeu8(buf_25, o_32 + 1, if item_4.zed then 1 else 0)
				o_32 += 2
			else
				error("value matches none of the union's members")
			end
		end
		return buf_25
	end,
	deserialize = function(buf_26)
		local o_33 = 0
		local count_2, o_34 = vread(buf_26, o_33)
		o_33 = o_34
		if count_2 * 2 > buffer.len(buf_26) - o_33 then
			error("malformed payload")
		end
		local array_4 = table.create(count_2)
		for i_7 = 1, count_2 do
			local tag_9 = buffer.readu8(buf_26, o_33)
			local value_16
			o_33 += 1
			if tag_9 == 0 then
				local alpha_3 = buffer.readf64(buf_26, o_33)
				value_16 = {
					alpha = alpha_3,
				}
				o_33 += 8
			elseif tag_9 == 1 then
				local zed_2 = buffer.readu8(buf_26, o_33) ~= 0
				value_16 = {
					zed = zed_2,
				}
				o_33 += 1
			else
				error("malformed payload")
			end
			array_4[i_7] = value_16
		end
		if o_33 ~= buffer.len(buf_26) then
			error("malformed payload")
		end
		return array_4
	end,
})
local withBoolean = Flamework.createSerializer({
	serialize = function(v_19)
		local size_12 = 1
		if type(v_19) == "boolean" then
			size_12 += 1
		elseif type(v_19) == "string" then
			local length_18 = #v_19
			size_12 += vsize(length_18) + length_18
		elseif typeof(v_19) == "Vector3" then
			size_12 += 12
		end
		local buf_27 = buffer.create(size_12)
		local o_35 = 0
		if type(v_19) == "boolean" then
			buffer.writeu8(buf_27, o_35, 0)
			buffer.writeu8(buf_27, o_35 + 1, if v_19 then 1 else 0)
			o_35 += 2
		elseif type(v_19) == "string" then
			buffer.writeu8(buf_27, o_35, 1)
			local length_19 = #v_19
			o_35 = vwrite(buf_27, o_35 + 1, length_19)
			buffer.writestring(buf_27, o_35, v_19)
			o_35 += length_19
		elseif typeof(v_19) == "Vector3" then
			buffer.writeu8(buf_27, o_35, 2)
			buffer.writef32(buf_27, o_35 + 1, v_19.X)
			buffer.writef32(buf_27, o_35 + 5, v_19.Y)
			buffer.writef32(buf_27, o_35 + 9, v_19.Z)
			o_35 += 13
		else
			error("value matches none of the union's members")
		end
		return buf_27
	end,
	deserialize = function(buf_28)
		local o_36 = 0
		local tag_10 = buffer.readu8(buf_28, o_36)
		local value_17
		o_36 += 1
		if tag_10 == 0 then
			value_17 = buffer.readu8(buf_28, o_36) ~= 0
			o_36 += 1
		elseif tag_10 == 1 then
			local length_20, o_37 = vread(buf_28, o_36)
			local text_7 = buffer.readstring(buf_28, o_37, length_20)
			o_36 = o_37 + length_20
			value_17 = text_7
		elseif tag_10 == 2 then
			value_17 = Vector3.new(buffer.readf32(buf_28, o_36), buffer.readf32(buf_28, o_36 + 4), buffer.readf32(buf_28, o_36 + 8))
			o_36 += 12
		else
			error("malformed payload")
		end
		if o_36 ~= buffer.len(buf_28) then
			error("malformed payload")
		end
		return value_17
	end,
})
local enum_KeyCode = {}
for _, item_5 in Enum.KeyCode:GetEnumItems() do
	enum_KeyCode[item_5.Value] = item_5
end
local withEnum = Flamework.createSerializer({
	serialize = function(v_20)
		local size_13 = 1
		if typeof(v_20) == "EnumItem" and v_20.EnumType == Enum.KeyCode then
			size_13 += 2
		elseif type(v_20) == "number" then
			size_13 += if v_20 < 34359738368 and 1 / v_20 > 0 and v_20 % 1 == 0 then vsize(v_20) else 8
		end
		local buf_29 = buffer.create(size_13)
		local o_38 = 0
		if typeof(v_20) == "EnumItem" and v_20.EnumType == Enum.KeyCode then
			buffer.writeu8(buf_29, o_38, 0)
			buffer.writeu16(buf_29, o_38 + 1, v_20.Value)
			o_38 += 3
		elseif type(v_20) == "number" then
			if v_20 < 34359738368 and 1 / v_20 > 0 and v_20 % 1 == 0 then
				buffer.writeu8(buf_29, o_38, 2)
				o_38 = vwrite(buf_29, o_38 + 1, v_20)
			else
				buffer.writeu8(buf_29, o_38, 1)
				buffer.writef64(buf_29, o_38 + 1, v_20)
				o_38 += 9
			end
		else
			error("value matches none of the union's members")
		end
		return buf_29
	end,
	deserialize = function(buf_30)
		local o_39 = 0
		local tag_11 = buffer.readu8(buf_30, o_39)
		local value_18
		o_39 += 1
		if tag_11 == 0 then
			value_18 = enum_KeyCode[buffer.readu16(buf_30, o_39)]
			o_39 += 2
		elseif tag_11 == 1 then
			value_18 = buffer.readf64(buf_30, o_39)
			o_39 += 8
		elseif tag_11 == 2 then
			local n_11, o_40 = vread(buf_30, o_39)
			value_18 = n_11
			o_39 = o_40
		else
			error("malformed payload")
		end
		if o_39 ~= buffer.len(buf_30) then
			error("malformed payload")
		end
		return value_18
	end,
})
codec.s_Partial = function(v_21)
	local v_22 = v_21.n
	local size_14 = 1
	if v_22 ~= nil then
		local length_21 = #v_22
		size_14 += vsize(length_21) + length_21
	end
	local v_23 = v_21.at
	local v_24 = v_21.s
	return size_14 + (if v_23 ~= nil then 9 else 1) + (if v_24 ~= nil then 2 else 1)
end
codec.w_Partial = function(buf_31, o_41, v_21)
	local v_25 = v_21.n
	buffer.writeu8(buf_31, o_41, if v_25 ~= nil then 1 else 0)
	o_41 += 1
	if v_25 ~= nil then
		local length_22 = #v_25
		o_41 = vwrite(buf_31, o_41, length_22)
		buffer.writestring(buf_31, o_41, v_25)
		o_41 += length_22
	end
	local v_26 = v_21.at
	buffer.writeu8(buf_31, o_41, if v_26 ~= nil then 1 else 0)
	o_41 += 1
	if v_26 ~= nil then
		buffer.writef64(buf_31, o_41, v_26)
		o_41 += 8
	end
	local v_27 = v_21.s
	buffer.writeu8(buf_31, o_41, if v_27 ~= nil then 1 else 0)
	o_41 += 1
	if v_27 ~= nil then
		buffer.writeu8(buf_31, o_41, if v_27 then 1 else 0)
		o_41 += 1
	end
	return o_41
end
codec.r_Partial = function(buf_31, o_41)
	local present = buffer.readu8(buf_31, o_41) ~= 0
	local value_19
	o_41 += 1
	if present then
		local length_23, o_42 = vread(buf_31, o_41)
		local text_8 = buffer.readstring(buf_31, o_42, length_23)
		o_41 = o_42 + length_23
		value_19 = text_8
	end
	local present_1 = buffer.readu8(buf_31, o_41) ~= 0
	local value_20
	o_41 += 1
	if present_1 then
		value_20 = buffer.readf64(buf_31, o_41)
		o_41 += 8
	end
	local present_2 = buffer.readu8(buf_31, o_41) ~= 0
	local value_21
	o_41 += 1
	if present_2 then
		value_21 = buffer.readu8(buf_31, o_41) ~= 0
		o_41 += 1
	end
	local value_22 = {
		n = value_19,
		at = value_20,
		s = value_21,
	}
	return value_22, o_41
end
codec.s_PatchOrNone = function(v_28)
	local size_15 = 1
	if v_28.__none ~= nil then
	elseif type(v_28) == "table" then
		size_15 += codec.s_Partial(v_28)
	end
	return size_15
end
codec.w_PatchOrNone = function(buf_32, o_43, v_28)
	if v_28.__none ~= nil then
		buffer.writeu8(buf_32, o_43, 1)
		o_43 += 1
	elseif type(v_28) == "table" then
		buffer.writeu8(buf_32, o_43, 0)
		o_43 = codec.w_Partial(buf_32, o_43 + 1, v_28)
	else
		error("value matches none of the union's members")
	end
	return o_43
end
codec.r_PatchOrNone = function(buf_32, o_43)
	local tag_12 = buffer.readu8(buf_32, o_43)
	local value_23
	o_43 += 1
	if tag_12 == 0 then
		local value_24, o_44 = codec.r_Partial(buf_32, o_43)
		value_23 = value_24
		o_43 = o_44
	elseif tag_12 == 1 then
		value_23 = {
			__none = "__none",
		}
	else
		error("malformed payload")
	end
	return value_23, o_43
end
local patchOrNone = Flamework.createSerializer({
	serialize = function(v_29)
		local buf_33 = buffer.create(codec.s_PatchOrNone(v_29))
		codec.w_PatchOrNone(buf_33, 0, v_29)
		return buf_33
	end,
	deserialize = function(buf_34)
		local value_25, o_45 = codec.r_PatchOrNone(buf_34, 0)
		if o_45 ~= buffer.len(buf_34) then
			error("malformed payload")
		end
		return value_25
	end,
})
local guard = t.interface({
	a = t.optional(t.number),
})
codec.s_Partial_1 = function(v_30)
	local v_31 = v_30.b
	local size_16 = 1
	if v_31 ~= nil then
		local length_24 = #v_31
		size_16 += vsize(length_24) + length_24
	end
	return size_16
end
codec.w_Partial_1 = function(buf_35, o_46, v_30)
	local v_32 = v_30.b
	buffer.writeu8(buf_35, o_46, if v_32 ~= nil then 1 else 0)
	o_46 += 1
	if v_32 ~= nil then
		local length_25 = #v_32
		o_46 = vwrite(buf_35, o_46, length_25)
		buffer.writestring(buf_35, o_46, v_32)
		o_46 += length_25
	end
	return o_46
end
codec.r_Partial_1 = function(buf_35, o_46)
	local present_3 = buffer.readu8(buf_35, o_46) ~= 0
	local value_26
	o_46 += 1
	if present_3 then
		local length_26, o_47 = vread(buf_35, o_46)
		local text_9 = buffer.readstring(buf_35, o_47, length_26)
		o_46 = o_47 + length_26
		value_26 = text_9
	end
	local value_27 = {
		b = value_26,
	}
	return value_27, o_46
end
codec.s_Partial_2 = function(v_33)
	local v_34 = v_33.a
	return if v_34 ~= nil then 9 else 1
end
codec.w_Partial_2 = function(buf_36, o_48, v_33)
	local v_35 = v_33.a
	buffer.writeu8(buf_36, o_48, if v_35 ~= nil then 1 else 0)
	o_48 += 1
	if v_35 ~= nil then
		buffer.writef64(buf_36, o_48, v_35)
		o_48 += 8
	end
	return o_48
end
codec.r_Partial_2 = function(buf_36, o_48)
	local present_4 = buffer.readu8(buf_36, o_48) ~= 0
	local value_28
	o_48 += 1
	if present_4 then
		value_28 = buffer.readf64(buf_36, o_48)
		o_48 += 8
	end
	local value_29 = {
		a = value_28,
	}
	return value_29, o_48
end
codec.s_Ambiguous = function(v_36)
	local size_17 = 1
	if guard(v_36) then
		size_17 += codec.s_Partial_2(v_36)
	elseif type(v_36) == "table" then
		size_17 += codec.s_Partial_1(v_36)
	end
	return size_17
end
codec.w_Ambiguous = function(buf_37, o_49, v_36)
	if guard(v_36) then
		buffer.writeu8(buf_37, o_49, 0)
		o_49 = codec.w_Partial_2(buf_37, o_49 + 1, v_36)
	elseif type(v_36) == "table" then
		buffer.writeu8(buf_37, o_49, 1)
		o_49 = codec.w_Partial_1(buf_37, o_49 + 1, v_36)
	else
		error("value matches none of the union's members")
	end
	return o_49
end
codec.r_Ambiguous = function(buf_37, o_49)
	local tag_13 = buffer.readu8(buf_37, o_49)
	local value_30
	o_49 += 1
	if tag_13 == 0 then
		local value_31, o_50 = codec.r_Partial_2(buf_37, o_49)
		value_30 = value_31
		o_49 = o_50
	elseif tag_13 == 1 then
		local value_32, o_51 = codec.r_Partial_1(buf_37, o_49)
		value_30 = value_32
		o_49 = o_51
	else
		error("malformed payload")
	end
	return value_30, o_49
end
local ambiguous = Flamework.createSerializer({
	serialize = function(v_37)
		local buf_38 = buffer.create(codec.s_Ambiguous(v_37))
		codec.w_Ambiguous(buf_38, 0, v_37)
		return buf_38
	end,
	deserialize = function(buf_39)
		local value_33, o_52 = codec.r_Ambiguous(buf_39, 0)
		if o_52 ~= buffer.len(buf_39) then
			error("malformed payload")
		end
		return value_33
	end,
})
local guard_1 = t.interface({
	pos = t.interface({
		x = t.number,
		y = t.number,
		z = t.number,
	}),
})
codec.s_MapOrHolder = function(v_38)
	local size_18 = 1
	if guard_1(v_38) then
		size_18 += 24
	elseif type(v_38) == "table" then
		local size_19 = 0
		local n_12 = 0
		for key, entry in v_38 do
			n_12 += 1
			local length_27 = #key
			size_19 += vsize(length_27) + length_27 + 8
		end
		size_19 += vsize(n_12)
		size_18 += size_19
	end
	return size_18
end
codec.w_MapOrHolder = function(buf_40, o_53, v_38)
	if guard_1(v_38) then
		buffer.writeu8(buf_40, o_53, 1)
		local object = v_38.pos
		buffer.writef64(buf_40, o_53 + 1, object.x)
		buffer.writef64(buf_40, o_53 + 9, object.y)
		buffer.writef64(buf_40, o_53 + 17, object.z)
		o_53 += 25
	elseif type(v_38) == "table" then
		buffer.writeu8(buf_40, o_53, 0)
		local n_13 = 0
		for _ in v_38 do
			n_13 += 1
		end
		o_53 = vwrite(buf_40, o_53 + 1, n_13)
		for key_1, entry_1 in v_38 do
			local length_28 = #key_1
			o_53 = vwrite(buf_40, o_53, length_28)
			buffer.writestring(buf_40, o_53, key_1)
			o_53 += length_28
			buffer.writef64(buf_40, o_53, entry_1.x)
			o_53 += 8
		end
	else
		error("value matches none of the union's members")
	end
	return o_53
end
codec.r_MapOrHolder = function(buf_40, o_53)
	local tag_14 = buffer.readu8(buf_40, o_53)
	local value_34
	o_53 += 1
	if tag_14 == 0 then
		local count_3, o_54 = vread(buf_40, o_53)
		o_53 = o_54
		if count_3 * 9 > buffer.len(buf_40) - o_53 then
			error("malformed payload")
		end
		local map = {}
		for _ = 1, count_3 do
			local length_29, o_55 = vread(buf_40, o_53)
			local text_10 = buffer.readstring(buf_40, o_55, length_29)
			o_53 = o_55 + length_29
			local x_1 = buffer.readf64(buf_40, o_53)
			local entry_2 = {
				x = x_1,
			}
			map[text_10] = entry_2
			o_53 += 8
		end
		value_34 = map
	elseif tag_14 == 1 then
		local x_2 = buffer.readf64(buf_40, o_53)
		local y_1 = buffer.readf64(buf_40, o_53 + 8)
		local z_1 = buffer.readf64(buf_40, o_53 + 16)
		local pos_1 = {
			x = x_2,
			y = y_1,
			z = z_1,
		}
		value_34 = {
			pos = pos_1,
		}
		o_53 += 24
	else
		error("malformed payload")
	end
	return value_34, o_53
end
local mapOrHolder = Flamework.createSerializer({
	serialize = function(v_39)
		local buf_41 = buffer.create(codec.s_MapOrHolder(v_39))
		codec.w_MapOrHolder(buf_41, 0, v_39)
		return buf_41
	end,
	deserialize = function(buf_42)
		local value_35, o_56 = codec.r_MapOrHolder(buf_42, 0)
		if o_56 ~= buffer.len(buf_42) then
			error("malformed payload")
		end
		return value_35
	end,
})
local optionalUnion = Flamework.createSerializer({
	serialize = function(v_40)
		local size_20 = 1
		if v_40 ~= nil then
			local size_21 = 1
			if v_40.kind == "circle" then
				size_21 += 8
			elseif v_40.kind == "rect" then
				size_21 += 16
			end
			size_20 += size_21
		end
		local buf_43 = buffer.create(size_20)
		local o_57 = 0
		buffer.writeu8(buf_43, o_57, if v_40 ~= nil then 1 else 0)
		o_57 += 1
		if v_40 ~= nil then
			if v_40.kind == "circle" then
				buffer.writeu8(buf_43, o_57, 0)
				buffer.writef64(buf_43, o_57 + 1, v_40.radius)
				o_57 += 9
			elseif v_40.kind == "rect" then
				buffer.writeu8(buf_43, o_57, 1)
				buffer.writef64(buf_43, o_57 + 1, v_40.w)
				buffer.writef64(buf_43, o_57 + 9, v_40.h)
				o_57 += 17
			else
				error("value matches none of the union's members")
			end
		end
		return buf_43
	end,
	deserialize = function(buf_44)
		local o_58 = 0
		local present_5 = buffer.readu8(buf_44, o_58) ~= 0
		local value_36
		o_58 += 1
		if present_5 then
			local tag_15 = buffer.readu8(buf_44, o_58)
			local value_37
			o_58 += 1
			if tag_15 == 0 then
				local radius_2 = buffer.readf64(buf_44, o_58)
				value_37 = {
					kind = "circle",
					radius = radius_2,
				}
				o_58 += 8
			elseif tag_15 == 1 then
				local w_2 = buffer.readf64(buf_44, o_58)
				local h_2 = buffer.readf64(buf_44, o_58 + 8)
				value_37 = {
					kind = "rect",
					w = w_2,
					h = h_2,
				}
				o_58 += 16
			else
				error("malformed payload")
			end
			value_36 = value_37
		end
		if o_58 ~= buffer.len(buf_44) then
			error("malformed payload")
		end
		return value_36
	end,
})
codec.s_Wallet_1 = function(v_41)
	local array_5 = v_41.Items
	local n_14 = #array_5
	local size_22 = vsize(n_14)
	for i_8 = 1, n_14 do
		local item_6 = array_5[i_8]
		if item_6 == nil then
			error(`[Flamework] the array has no value at (\{ Items: string[]; \}).Items[{i_8 - 1}]`)
		end
		local length_30 = #item_6
		size_22 += vsize(length_30) + length_30
	end
	return size_22
end
codec.w_Wallet_1 = function(buf_45, o_59, v_41)
	local array_6 = v_41.Items
	o_59 = vwrite(buf_45, o_59, #array_6)
	for i_9 = 1, #array_6 do
		local item_7 = array_6[i_9]
		local length_31 = #item_7
		o_59 = vwrite(buf_45, o_59, length_31)
		buffer.writestring(buf_45, o_59, item_7)
		o_59 += length_31
	end
	return o_59
end
codec.r_Wallet_1 = function(buf_45, o_59)
	local count_4, o_60 = vread(buf_45, o_59)
	o_59 = o_60
	if count_4 > buffer.len(buf_45) - o_59 then
		error("malformed payload")
	end
	local array_7 = table.create(count_4)
	for i_10 = 1, count_4 do
		local length_32, o_61 = vread(buf_45, o_59)
		local text_11 = buffer.readstring(buf_45, o_61, length_32)
		o_59 = o_61 + length_32
		array_7[i_10] = text_11
	end
	local value_38 = {
		Items = array_7,
	}
	return value_38, o_59
end
local nestedUnion = Flamework.createSerializer({
	serialize = function(v_42)
		local n_15 = #v_42
		local size_23 = vsize(n_15)
		for i_11 = 1, n_15 do
			local item_8 = v_42[i_11]
			if item_8 == nil then
				error(`[Flamework] the array has no value at value[{i_11 - 1}]`)
			end
			local size_24 = 1
			if item_8.Coins ~= nil then
				size_24 += 8
			elseif item_8.Items ~= nil then
				size_24 += codec.s_Wallet_1(item_8)
			elseif item_8.radius ~= nil then
				size_24 += 8
			elseif item_8.w ~= nil then
				size_24 += 16
			end
			size_23 += size_24
		end
		local buf_46 = buffer.create(size_23)
		local o_62 = 0
		o_62 = vwrite(buf_46, o_62, #v_42)
		for i_12 = 1, #v_42 do
			local item_9 = v_42[i_12]
			if item_9.Coins ~= nil then
				buffer.writeu8(buf_46, o_62, 0)
				buffer.writef64(buf_46, o_62 + 1, item_9.Coins)
				o_62 += 9
			elseif item_9.Items ~= nil then
				buffer.writeu8(buf_46, o_62, 1)
				o_62 = codec.w_Wallet_1(buf_46, o_62 + 1, item_9)
			elseif item_9.radius ~= nil then
				buffer.writeu8(buf_46, o_62, 2)
				buffer.writef64(buf_46, o_62 + 1, item_9.radius)
				o_62 += 9
			elseif item_9.w ~= nil then
				buffer.writeu8(buf_46, o_62, 3)
				buffer.writef64(buf_46, o_62 + 1, item_9.w)
				buffer.writef64(buf_46, o_62 + 9, item_9.h)
				o_62 += 17
			else
				error("value matches none of the union's members")
			end
		end
		return buf_46
	end,
	deserialize = function(buf_47)
		local o_63 = 0
		local count_5, o_64 = vread(buf_47, o_63)
		o_63 = o_64
		if count_5 * 2 > buffer.len(buf_47) - o_63 then
			error("malformed payload")
		end
		local array_8 = table.create(count_5)
		for i_13 = 1, count_5 do
			local tag_16 = buffer.readu8(buf_47, o_63)
			local value_39
			o_63 += 1
			if tag_16 == 0 then
				local Coins_2 = buffer.readf64(buf_47, o_63)
				value_39 = {
					Coins = Coins_2,
				}
				o_63 += 8
			elseif tag_16 == 1 then
				local value_40, o_65 = codec.r_Wallet_1(buf_47, o_63)
				value_39 = value_40
				o_63 = o_65
			elseif tag_16 == 2 then
				local radius_3 = buffer.readf64(buf_47, o_63)
				value_39 = {
					kind = "circle",
					radius = radius_3,
				}
				o_63 += 8
			elseif tag_16 == 3 then
				local w_3 = buffer.readf64(buf_47, o_63)
				local h_3 = buffer.readf64(buf_47, o_63 + 8)
				value_39 = {
					kind = "rect",
					w = w_3,
					h = h_3,
				}
				o_63 += 16
			else
				error("malformed payload")
			end
			array_8[i_13] = value_39
		end
		if o_63 ~= buffer.len(buf_47) then
			error("malformed payload")
		end
		return array_8
	end,
})
local blobUnion = Flamework.createSerializer({
	serialize = function(v_43)
		local size_25 = 1
		if type(v_43) == "string" then
			local length_33 = #v_43
			size_25 += vsize(length_33) + length_33
		elseif type(v_43) == "table" then
			local n_16 = #v_43
			size_25 += vsize(n_16) + n_16 * 4
		end
		local buf_48 = buffer.create(size_25)
		local blobs_3 = {}
		local o_66 = 0
		if type(v_43) == "string" then
			buffer.writeu8(buf_48, o_66, 1)
			local length_34 = #v_43
			o_66 = vwrite(buf_48, o_66 + 1, length_34)
			buffer.writestring(buf_48, o_66, v_43)
			o_66 += length_34
		elseif type(v_43) == "table" then
			buffer.writeu8(buf_48, o_66, 0)
			o_66 = vwrite(buf_48, o_66 + 1, #v_43)
			for i_14 = 1, #v_43 do
				local item_10 = v_43[i_14]
				if item_10 ~= nil then
					table.insert(blobs_3, item_10)
					buffer.writeu32(buf_48, o_66, #blobs_3)
				else
					buffer.writeu32(buf_48, o_66, 0)
				end
				o_66 += 4
			end
		else
			error("value matches none of the union's members")
		end
		return buf_48, blobs_3
	end,
	deserialize = function(buf_49, blobs_4)
		local o_67 = 0
		local tag_17 = buffer.readu8(buf_49, o_67)
		local value_41
		o_67 += 1
		if tag_17 == 0 then
			local count_6, o_68 = vread(buf_49, o_67)
			o_67 = o_68
			if count_6 * 4 > buffer.len(buf_49) - o_67 then
				error("malformed payload")
			end
			local array_9 = table.create(count_6)
			for i_15 = 1, count_6 do
				array_9[i_15] = blobs_4[buffer.readu32(buf_49, o_67)]
				o_67 += 4
			end
			value_41 = array_9
		elseif tag_17 == 1 then
			local length_35, o_69 = vread(buf_49, o_67)
			local text_12 = buffer.readstring(buf_49, o_69, length_35)
			o_67 = o_69 + length_35
			value_41 = text_12
		else
			error("malformed payload")
		end
		if o_67 ~= buffer.len(buf_49) then
			error("malformed payload")
		end
		return value_41
	end,
})
local cframes = Flamework.createSerializer({
	serialize = function(v_44)
		local size_26 = 0
		local n_17 = 0
		for item_11 in v_44 do
			n_17 += 1
			local size_27 = 1
			if typeof(item_11) == "CFrame" then
				size_27 += 48
			elseif type(item_11) == "string" then
				local length_36 = #item_11
				size_27 += vsize(length_36) + length_36
			end
			size_26 += size_27
		end
		size_26 += vsize(n_17)
		local buf_50 = buffer.create(size_26)
		local o_70 = 0
		local n_18 = 0
		for _ in v_44 do
			n_18 += 1
		end
		o_70 = vwrite(buf_50, o_70, n_18)
		for item_12 in v_44 do
			if typeof(item_12) == "CFrame" then
				buffer.writeu8(buf_50, o_70, 0)
				local c0, c1, c2, c3, c4, c5, c6, c7, c8, c9, c10, c11 = item_12:GetComponents()
				buffer.writef32(buf_50, o_70 + 1, c0)
				buffer.writef32(buf_50, o_70 + 5, c1)
				buffer.writef32(buf_50, o_70 + 9, c2)
				buffer.writef32(buf_50, o_70 + 13, c3)
				buffer.writef32(buf_50, o_70 + 17, c4)
				buffer.writef32(buf_50, o_70 + 21, c5)
				buffer.writef32(buf_50, o_70 + 25, c6)
				buffer.writef32(buf_50, o_70 + 29, c7)
				buffer.writef32(buf_50, o_70 + 33, c8)
				buffer.writef32(buf_50, o_70 + 37, c9)
				buffer.writef32(buf_50, o_70 + 41, c10)
				buffer.writef32(buf_50, o_70 + 45, c11)
				o_70 += 49
			elseif type(item_12) == "string" then
				buffer.writeu8(buf_50, o_70, 1)
				local length_37 = #item_12
				o_70 = vwrite(buf_50, o_70 + 1, length_37)
				buffer.writestring(buf_50, o_70, item_12)
				o_70 += length_37
			else
				error("value matches none of the union's members")
			end
		end
		return buf_50
	end,
	deserialize = function(buf_51)
		local o_71 = 0
		local count_7, o_72 = vread(buf_51, o_71)
		o_71 = o_72
		if count_7 * 2 > buffer.len(buf_51) - o_71 then
			error("malformed payload")
		end
		local set = {}
		for _1 = 1, count_7 do
			local tag_18 = buffer.readu8(buf_51, o_71)
			local value_42
			o_71 += 1
			if tag_18 == 0 then
				value_42 = CFrame.new(buffer.readf32(buf_51, o_71), buffer.readf32(buf_51, o_71 + 4), buffer.readf32(buf_51, o_71 + 8), buffer.readf32(buf_51, o_71 + 12), buffer.readf32(buf_51, o_71 + 16), buffer.readf32(buf_51, o_71 + 20), buffer.readf32(buf_51, o_71 + 24), buffer.readf32(buf_51, o_71 + 28), buffer.readf32(buf_51, o_71 + 32), buffer.readf32(buf_51, o_71 + 36), buffer.readf32(buf_51, o_71 + 40), buffer.readf32(buf_51, o_71 + 44))
				o_71 += 48
			elseif tag_18 == 1 then
				local length_38, o_73 = vread(buf_51, o_71)
				local text_13 = buffer.readstring(buf_51, o_73, length_38)
				o_71 = o_73 + length_38
				value_42 = text_13
			else
				error("malformed payload")
			end
			set[value_42] = true
		end
		if o_71 ~= buffer.len(buf_51) then
			error("malformed payload")
		end
		return set
	end,
})
return {
	wallet = wallet,
	shape = shape,
	mixed = mixed,
	sortOf = sortOf,
	sortOfReversed = sortOfReversed,
	narrow = narrow,
	narrowOnly = narrowOnly,
	choice = choice,
	boxes = boxes,
	unwritten = unwritten,
	withBoolean = withBoolean,
	withEnum = withEnum,
	patchOrNone = patchOrNone,
	ambiguous = ambiguous,
	mapOrHolder = mapOrHolder,
	optionalUnion = optionalUnion,
	nestedUnion = nestedUnion,
	blobUnion = blobUnion,
	cframes = cframes,
}
