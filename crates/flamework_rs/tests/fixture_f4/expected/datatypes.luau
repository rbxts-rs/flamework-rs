-- Compiled with roblox-ts v3.0.0
local TS = _G[script]
local Flamework = TS.import(script, TS.getModule(script, "@flamework-experimental", "core").out).Flamework
local vector3 = Flamework.createSerializer({
	serialize = function(v)
		local buf = buffer.create(12)
		buffer.writef32(buf, 0, v.X)
		buffer.writef32(buf, 4, v.Y)
		buffer.writef32(buf, 8, v.Z)
		return buf
	end,
	deserialize = function(buf_1)
		local value = Vector3.new(buffer.readf32(buf_1, 0), buffer.readf32(buf_1, 4), buffer.readf32(buf_1, 8))
		if buffer.len(buf_1) ~= 12 then
			error("malformed payload")
		end
		return value
	end,
})
local vector2 = Flamework.createSerializer({
	serialize = function(v_1)
		local buf_2 = buffer.create(8)
		buffer.writef32(buf_2, 0, v_1.X)
		buffer.writef32(buf_2, 4, v_1.Y)
		return buf_2
	end,
	deserialize = function(buf_3)
		local value_1 = Vector2.new(buffer.readf32(buf_3, 0), buffer.readf32(buf_3, 4))
		if buffer.len(buf_3) ~= 8 then
			error("malformed payload")
		end
		return value_1
	end,
})
local vector3int16 = Flamework.createSerializer({
	serialize = function(v_2)
		local buf_4 = buffer.create(6)
		buffer.writei16(buf_4, 0, v_2.X)
		buffer.writei16(buf_4, 2, v_2.Y)
		buffer.writei16(buf_4, 4, v_2.Z)
		return buf_4
	end,
	deserialize = function(buf_5)
		local value_2 = Vector3int16.new(buffer.readi16(buf_5, 0), buffer.readi16(buf_5, 2), buffer.readi16(buf_5, 4))
		if buffer.len(buf_5) ~= 6 then
			error("malformed payload")
		end
		return value_2
	end,
})
local vector2int16 = Flamework.createSerializer({
	serialize = function(v_3)
		local buf_6 = buffer.create(4)
		buffer.writei16(buf_6, 0, v_3.X)
		buffer.writei16(buf_6, 2, v_3.Y)
		return buf_6
	end,
	deserialize = function(buf_7)
		local value_3 = Vector2int16.new(buffer.readi16(buf_7, 0), buffer.readi16(buf_7, 2))
		if buffer.len(buf_7) ~= 4 then
			error("malformed payload")
		end
		return value_3
	end,
})
local color3 = Flamework.createSerializer({
	serialize = function(v_4)
		local buf_8 = buffer.create(12)
		buffer.writef32(buf_8, 0, v_4.R)
		buffer.writef32(buf_8, 4, v_4.G)
		buffer.writef32(buf_8, 8, v_4.B)
		return buf_8
	end,
	deserialize = function(buf_9)
		local value_4 = Color3.new(buffer.readf32(buf_9, 0), buffer.readf32(buf_9, 4), buffer.readf32(buf_9, 8))
		if buffer.len(buf_9) ~= 12 then
			error("malformed payload")
		end
		return value_4
	end,
})
local udim = Flamework.createSerializer({
	serialize = function(v_5)
		local buf_10 = buffer.create(8)
		buffer.writef32(buf_10, 0, v_5.Scale)
		buffer.writei32(buf_10, 4, v_5.Offset)
		return buf_10
	end,
	deserialize = function(buf_11)
		local value_5 = UDim.new(buffer.readf32(buf_11, 0), buffer.readi32(buf_11, 4))
		if buffer.len(buf_11) ~= 8 then
			error("malformed payload")
		end
		return value_5
	end,
})
local udim2 = Flamework.createSerializer({
	serialize = function(v_6)
		local buf_12 = buffer.create(16)
		buffer.writef32(buf_12, 0, v_6.X.Scale)
		buffer.writei32(buf_12, 4, v_6.X.Offset)
		buffer.writef32(buf_12, 8, v_6.Y.Scale)
		buffer.writei32(buf_12, 12, v_6.Y.Offset)
		return buf_12
	end,
	deserialize = function(buf_13)
		local value_6 = UDim2.new(buffer.readf32(buf_13, 0), buffer.readi32(buf_13, 4), buffer.readf32(buf_13, 8), buffer.readi32(buf_13, 12))
		if buffer.len(buf_13) ~= 16 then
			error("malformed payload")
		end
		return value_6
	end,
})
local numberRange = Flamework.createSerializer({
	serialize = function(v_7)
		local buf_14 = buffer.create(8)
		buffer.writef32(buf_14, 0, v_7.Min)
		buffer.writef32(buf_14, 4, v_7.Max)
		return buf_14
	end,
	deserialize = function(buf_15)
		local value_7 = NumberRange.new(buffer.readf32(buf_15, 0), buffer.readf32(buf_15, 4))
		if buffer.len(buf_15) ~= 8 then
			error("malformed payload")
		end
		return value_7
	end,
})
local rect = Flamework.createSerializer({
	serialize = function(v_8)
		local buf_16 = buffer.create(16)
		buffer.writef32(buf_16, 0, v_8.Min.X)
		buffer.writef32(buf_16, 4, v_8.Min.Y)
		buffer.writef32(buf_16, 8, v_8.Max.X)
		buffer.writef32(buf_16, 12, v_8.Max.Y)
		return buf_16
	end,
	deserialize = function(buf_17)
		local value_8 = Rect.new(buffer.readf32(buf_17, 0), buffer.readf32(buf_17, 4), buffer.readf32(buf_17, 8), buffer.readf32(buf_17, 12))
		if buffer.len(buf_17) ~= 16 then
			error("malformed payload")
		end
		return value_8
	end,
})
local brickColor = Flamework.createSerializer({
	serialize = function(v_9)
		local buf_18 = buffer.create(2)
		buffer.writeu16(buf_18, 0, v_9.Number)
		return buf_18
	end,
	deserialize = function(buf_19)
		local value_9 = BrickColor.new(buffer.readu16(buf_19, 0))
		if buffer.len(buf_19) ~= 2 then
			error("malformed payload")
		end
		return value_9
	end,
})
local cframe = Flamework.createSerializer({
	serialize = function(v_10)
		local buf_20 = buffer.create(48)
		local c0, c1, c2, c3, c4, c5, c6, c7, c8, c9, c10, c11 = v_10:GetComponents()
		buffer.writef32(buf_20, 0, c0)
		buffer.writef32(buf_20, 4, c1)
		buffer.writef32(buf_20, 8, c2)
		buffer.writef32(buf_20, 12, c3)
		buffer.writef32(buf_20, 16, c4)
		buffer.writef32(buf_20, 20, c5)
		buffer.writef32(buf_20, 24, c6)
		buffer.writef32(buf_20, 28, c7)
		buffer.writef32(buf_20, 32, c8)
		buffer.writef32(buf_20, 36, c9)
		buffer.writef32(buf_20, 40, c10)
		buffer.writef32(buf_20, 44, c11)
		return buf_20
	end,
	deserialize = function(buf_21)
		local value_10 = CFrame.new(buffer.readf32(buf_21, 0), buffer.readf32(buf_21, 4), buffer.readf32(buf_21, 8), buffer.readf32(buf_21, 12), buffer.readf32(buf_21, 16), buffer.readf32(buf_21, 20), buffer.readf32(buf_21, 24), buffer.readf32(buf_21, 28), buffer.readf32(buf_21, 32), buffer.readf32(buf_21, 36), buffer.readf32(buf_21, 40), buffer.readf32(buf_21, 44))
		if buffer.len(buf_21) ~= 48 then
			error("malformed payload")
		end
		return value_10
	end,
})
local enum_Material = {}
for _, item in Enum.Material:GetEnumItems() do
	enum_Material[item.Value] = item
end
local material = Flamework.createSerializer({
	serialize = function(v_11)
		local buf_22 = buffer.create(2)
		buffer.writeu16(buf_22, 0, v_11.Value)
		return buf_22
	end,
	deserialize = function(buf_23)
		local value_11 = enum_Material[buffer.readu16(buf_23, 0)]
		if buffer.len(buf_23) ~= 2 then
			error("malformed payload")
		end
		return value_11
	end,
})
local enum_KeyCode = {}
for _, item_1 in Enum.KeyCode:GetEnumItems() do
	enum_KeyCode[item_1.Value] = item_1
end
local keyCode = Flamework.createSerializer({
	serialize = function(v_12)
		local buf_24 = buffer.create(if v_12 ~= nil then 3 else 1)
		local o = 0
		buffer.writeu8(buf_24, o, if v_12 ~= nil then 1 else 0)
		o += 1
		if v_12 ~= nil then
			buffer.writeu16(buf_24, o, v_12.Value)
			o += 2
		end
		return buf_24
	end,
	deserialize = function(buf_25)
		local o_1 = 0
		local present = buffer.readu8(buf_25, o_1) ~= 0
		local value_12
		o_1 += 1
		if present then
			value_12 = enum_KeyCode[buffer.readu16(buf_25, o_1)]
			o_1 += 2
		end
		if o_1 ~= buffer.len(buf_25) then
			error("malformed payload")
		end
		return value_12
	end,
})
local literals = { Enum.Material.Plastic, Enum.Material.Wood }
local literalIndex = {
	[Enum.Material.Plastic] = 0,
	[Enum.Material.Wood] = 1,
}
local someMaterials = Flamework.createSerializer({
	serialize = function(v_13)
		local buf_26 = buffer.create(1)
		local index = literalIndex[v_13]
		if index == nil then
			error("value is not one of the literals its type allows")
		end
		buffer.writeu8(buf_26, 0, index)
		return buf_26
	end,
	deserialize = function(buf_27)
		local literal = literals[buffer.readu8(buf_27, 0) + 1]
		if literal == nil then
			error("malformed payload")
		end
		if buffer.len(buf_27) ~= 1 then
			error("malformed payload")
		end
		return literal
	end,
})
local oneMaterial = Flamework.createSerializer({
	serialize = function(v_14)
		local buf_28 = buffer.create(0)
		return buf_28
	end,
	deserialize = function(buf_29)
		local value_13 = Enum.Material.Plastic
		if buffer.len(buf_29) ~= 0 then
			error("malformed payload")
		end
		return value_13
	end,
})
local vsize = function(n)
	return if n < 128 then 1 elseif n < 16384 then 2 elseif n < 2097152 then 3 elseif n < 268435456 then 4 else 5
end
local vwrite = function(buf_30, o_2, n_1)
	while n_1 >= 128 do
		buffer.writeu8(buf_30, o_2, n_1 % 128 + 128)
		o_2 += 1
		n_1 = math.floor(n_1 / 128)
	end
	buffer.writeu8(buf_30, o_2, n_1)
	return o_2 + 1
end
local vread = function(buf_31, o_3)
	local n_2 = 0
	local scale = 1
	while true do
		local b = buffer.readu8(buf_31, o_3)
		o_3 += 1
		n_2 += b % 128 * scale
		if b < 128 then
			return n_2, o_3
		end
		scale *= 128
		if scale > 268435456 then
			error("malformed payload")
		end
	end
end
local colors = Flamework.createSerializer({
	serialize = function(v_15)
		local n_3 = #v_15
		local size = vsize(n_3)
		for i = 1, n_3 do
			local item_2 = v_15[i]
			if item_2 == nil then
				error(`[Flamework] the array has no value at value[{i - 1}]`)
			end
			local size_1 = 1
			if typeof(item_2) == "Color3" then
				size_1 += 12
			elseif typeof(item_2) == "BrickColor" then
				size_1 += 2
			end
			size += size_1
		end
		local buf_32 = buffer.create(size)
		local o_4 = 0
		o_4 = vwrite(buf_32, o_4, #v_15)
		for i_1 = 1, #v_15 do
			local item_3 = v_15[i_1]
			if typeof(item_3) == "Color3" then
				buffer.writeu8(buf_32, o_4, 0)
				buffer.writef32(buf_32, o_4 + 1, item_3.R)
				buffer.writef32(buf_32, o_4 + 5, item_3.G)
				buffer.writef32(buf_32, o_4 + 9, item_3.B)
				o_4 += 13
			elseif typeof(item_3) == "BrickColor" then
				buffer.writeu8(buf_32, o_4, 1)
				buffer.writeu16(buf_32, o_4 + 1, item_3.Number)
				o_4 += 3
			else
				error("value matches none of the union's members")
			end
		end
		return buf_32
	end,
	deserialize = function(buf_33)
		local o_5 = 0
		local count, o_6 = vread(buf_33, o_5)
		o_5 = o_6
		if count * 3 > buffer.len(buf_33) - o_5 then
			error("malformed payload")
		end
		local array = table.create(count)
		for i_2 = 1, count do
			local tag = buffer.readu8(buf_33, o_5)
			local value_14
			o_5 += 1
			if tag == 0 then
				value_14 = Color3.new(buffer.readf32(buf_33, o_5), buffer.readf32(buf_33, o_5 + 4), buffer.readf32(buf_33, o_5 + 8))
				o_5 += 12
			elseif tag == 1 then
				value_14 = BrickColor.new(buffer.readu16(buf_33, o_5))
				o_5 += 2
			else
				error("malformed payload")
			end
			array[i_2] = value_14
		end
		if o_5 ~= buffer.len(buf_33) then
			error("malformed payload")
		end
		return array
	end,
})
local placements = Flamework.createSerializer({
	serialize = function(v_16)
		local size_2 = 0
		local n_4 = 0
		for key, entry in v_16 do
			n_4 += 1
			size_2 += 52
		end
		size_2 += vsize(n_4)
		local buf_34 = buffer.create(size_2)
		local blobs = {}
		local o_7 = 0
		local n_5 = 0
		for _ in v_16 do
			n_5 += 1
		end
		o_7 = vwrite(buf_34, o_7, n_5)
		for key_1, entry_1 in v_16 do
			if key_1 ~= nil then
				table.insert(blobs, key_1)
				buffer.writeu32(buf_34, o_7, #blobs)
			else
				buffer.writeu32(buf_34, o_7, 0)
			end
			local c0_1, c1_1, c2_1, c3_1, c4_1, c5_1, c6_1, c7_1, c8_1, c9_1, c10_1, c11_1 = entry_1:GetComponents()
			buffer.writef32(buf_34, o_7 + 4, c0_1)
			buffer.writef32(buf_34, o_7 + 8, c1_1)
			buffer.writef32(buf_34, o_7 + 12, c2_1)
			buffer.writef32(buf_34, o_7 + 16, c3_1)
			buffer.writef32(buf_34, o_7 + 20, c4_1)
			buffer.writef32(buf_34, o_7 + 24, c5_1)
			buffer.writef32(buf_34, o_7 + 28, c6_1)
			buffer.writef32(buf_34, o_7 + 32, c7_1)
			buffer.writef32(buf_34, o_7 + 36, c8_1)
			buffer.writef32(buf_34, o_7 + 40, c9_1)
			buffer.writef32(buf_34, o_7 + 44, c10_1)
			buffer.writef32(buf_34, o_7 + 48, c11_1)
			o_7 += 52
		end
		return buf_34, blobs
	end,
	deserialize = function(buf_35, blobs_1)
		local o_8 = 0
		local count_1, o_9 = vread(buf_35, o_8)
		o_8 = o_9
		if count_1 * 52 > buffer.len(buf_35) - o_8 then
			error("malformed payload")
		end
		local map = {}
		for _ = 1, count_1 do
			local key_2 = blobs_1[buffer.readu32(buf_35, o_8)]
			local entry_2 = CFrame.new(buffer.readf32(buf_35, o_8 + 4), buffer.readf32(buf_35, o_8 + 8), buffer.readf32(buf_35, o_8 + 12), buffer.readf32(buf_35, o_8 + 16), buffer.readf32(buf_35, o_8 + 20), buffer.readf32(buf_35, o_8 + 24), buffer.readf32(buf_35, o_8 + 28), buffer.readf32(buf_35, o_8 + 32), buffer.readf32(buf_35, o_8 + 36), buffer.readf32(buf_35, o_8 + 40), buffer.readf32(buf_35, o_8 + 44), buffer.readf32(buf_35, o_8 + 48))
			map[key_2] = entry_2
			o_8 += 52
		end
		if o_8 ~= buffer.len(buf_35) then
			error("malformed payload")
		end
		return map
	end,
})
return {
	vector3 = vector3,
	vector2 = vector2,
	vector3int16 = vector3int16,
	vector2int16 = vector2int16,
	color3 = color3,
	udim = udim,
	udim2 = udim2,
	numberRange = numberRange,
	rect = rect,
	brickColor = brickColor,
	cframe = cframe,
	material = material,
	keyCode = keyCode,
	someMaterials = someMaterials,
	oneMaterial = oneMaterial,
	colors = colors,
	placements = placements,
}
