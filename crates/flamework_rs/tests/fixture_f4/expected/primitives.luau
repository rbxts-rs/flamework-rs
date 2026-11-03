-- Compiled with roblox-ts v3.0.0
local TS = _G[script]
local Flamework = TS.import(script, TS.getModule(script, "@flamework-experimental", "core").out).Flamework
-- Numbers at every width, strict and implicit, and the varint.
local f64 = Flamework.createSerializer({
	serialize = function(v)
		local buf = buffer.create(8)
		buffer.writef64(buf, 0, v)
		return buf
	end,
	deserialize = function(buf_1)
		local value_1 = buffer.readf64(buf_1, 0)
		if buffer.len(buf_1) ~= 8 then
			error("malformed payload")
		end
		return value_1
	end,
})
local u8 = Flamework.createSerializer({
	serialize = function(v_1)
		local buf_2 = buffer.create(1)
		buffer.writeu8(buf_2, 0, v_1)
		return buf_2
	end,
	deserialize = function(buf_3)
		local value_2 = buffer.readu8(buf_3, 0)
		if buffer.len(buf_3) ~= 1 then
			error("malformed payload")
		end
		return value_2
	end,
})
local i8 = Flamework.createSerializer({
	serialize = function(v_2)
		local buf_4 = buffer.create(1)
		buffer.writei8(buf_4, 0, v_2)
		return buf_4
	end,
	deserialize = function(buf_5)
		local value_3 = buffer.readi8(buf_5, 0)
		if buffer.len(buf_5) ~= 1 then
			error("malformed payload")
		end
		return value_3
	end,
})
local u16 = Flamework.createSerializer({
	serialize = function(v_3)
		local buf_6 = buffer.create(2)
		buffer.writeu16(buf_6, 0, v_3)
		return buf_6
	end,
	deserialize = function(buf_7)
		local value_4 = buffer.readu16(buf_7, 0)
		if buffer.len(buf_7) ~= 2 then
			error("malformed payload")
		end
		return value_4
	end,
})
local i16 = Flamework.createSerializer({
	serialize = function(v_4)
		local buf_8 = buffer.create(2)
		buffer.writei16(buf_8, 0, v_4)
		return buf_8
	end,
	deserialize = function(buf_9)
		local value_5 = buffer.readi16(buf_9, 0)
		if buffer.len(buf_9) ~= 2 then
			error("malformed payload")
		end
		return value_5
	end,
})
local u32 = Flamework.createSerializer({
	serialize = function(v_5)
		local buf_10 = buffer.create(4)
		buffer.writeu32(buf_10, 0, v_5)
		return buf_10
	end,
	deserialize = function(buf_11)
		local value_6 = buffer.readu32(buf_11, 0)
		if buffer.len(buf_11) ~= 4 then
			error("malformed payload")
		end
		return value_6
	end,
})
local i32 = Flamework.createSerializer({
	serialize = function(v_6)
		local buf_12 = buffer.create(4)
		buffer.writei32(buf_12, 0, v_6)
		return buf_12
	end,
	deserialize = function(buf_13)
		local value_7 = buffer.readi32(buf_13, 0)
		if buffer.len(buf_13) ~= 4 then
			error("malformed payload")
		end
		return value_7
	end,
})
local u64 = Flamework.createSerializer({
	serialize = function(v_7)
		local buf_14 = buffer.create(8)
		buffer.writeu32(buf_14, 0, v_7 % 4294967296)
		buffer.writeu32(buf_14, 4, math.floor(v_7 / 4294967296))
		return buf_14
	end,
	deserialize = function(buf_15)
		local value_8 = buffer.readu32(buf_15, 4) * 4294967296 + buffer.readu32(buf_15, 0)
		if buffer.len(buf_15) ~= 8 then
			error("malformed payload")
		end
		return value_8
	end,
})
local i64 = Flamework.createSerializer({
	serialize = function(v_8)
		local buf_16 = buffer.create(8)
		buffer.writeu32(buf_16, 0, v_8 % 4294967296)
		buffer.writei32(buf_16, 4, math.floor(v_8 / 4294967296))
		return buf_16
	end,
	deserialize = function(buf_17)
		local value_9 = buffer.readi32(buf_17, 4) * 4294967296 + buffer.readu32(buf_17, 0)
		if buffer.len(buf_17) ~= 8 then
			error("malformed payload")
		end
		return value_9
	end,
})
local f32 = Flamework.createSerializer({
	serialize = function(v_9)
		local buf_18 = buffer.create(4)
		buffer.writef32(buf_18, 0, v_9)
		return buf_18
	end,
	deserialize = function(buf_19)
		local value_10 = buffer.readf32(buf_19, 0)
		if buffer.len(buf_19) ~= 4 then
			error("malformed payload")
		end
		return value_10
	end,
})
local vsize = function(n)
	return if n < 128 then 1 elseif n < 16384 then 2 elseif n < 2097152 then 3 elseif n < 268435456 then 4 else 5
end
local vwrite = function(buf_20, o, n_1)
	while n_1 >= 128 do
		buffer.writeu8(buf_20, o, n_1 % 128 + 128)
		o += 1
		n_1 = math.floor(n_1 / 128)
	end
	buffer.writeu8(buf_20, o, n_1)
	return o + 1
end
local vread = function(buf_21, o_1)
	local n_2 = 0
	local scale = 1
	while true do
		local b = buffer.readu8(buf_21, o_1)
		o_1 += 1
		n_2 += b % 128 * scale
		if b < 128 then
			return n_2, o_1
		end
		scale *= 128
		if scale > 268435456 then
			error("malformed payload")
		end
	end
end
local varint = Flamework.createSerializer({
	serialize = function(v_10)
		local buf_22 = buffer.create(vsize(v_10))
		local o_2 = 0
		o_2 = vwrite(buf_22, o_2, v_10)
		return buf_22
	end,
	deserialize = function(buf_23)
		local o_3 = 0
		local n_3, o_4 = vread(buf_23, o_3)
		o_3 = o_4
		if o_3 ~= buffer.len(buf_23) then
			error("malformed payload")
		end
		return n_3
	end,
})
local codec = {}
codec.checkWidth = function(width_1, value_11, where, unit)
	if unit == nil then
		unit = ""
	end
	local message = `[Flamework] {width_1} cannot hold {value_11}{unit}, at {where}`
	error(message, 2)
end
local implicitU8 = Flamework.createSerializer({
	serialize = function(v_11)
		local buf_24 = buffer.create(1)
		if not (v_11 >= 0 and v_11 <= 255 and v_11 % 1 == 0) then
			codec.checkWidth("u8", v_11, "value")
		end
		buffer.writeu8(buf_24, 0, v_11)
		return buf_24
	end,
	deserialize = function(buf_25)
		local value_12 = buffer.readu8(buf_25, 0)
		if buffer.len(buf_25) ~= 1 then
			error("malformed payload")
		end
		return value_12
	end,
})
local implicitI64 = Flamework.createSerializer({
	serialize = function(v_12)
		local buf_26 = buffer.create(8)
		if not (v_12 >= -9007199254740992 and v_12 <= 9007199254740992 and v_12 % 1 == 0) then
			codec.checkWidth("i64", v_12, "value")
		end
		buffer.writeu32(buf_26, 0, v_12 % 4294967296)
		buffer.writei32(buf_26, 4, math.floor(v_12 / 4294967296))
		return buf_26
	end,
	deserialize = function(buf_27)
		local value_13 = buffer.readi32(buf_27, 4) * 4294967296 + buffer.readu32(buf_27, 0)
		if buffer.len(buf_27) ~= 8 then
			error("malformed payload")
		end
		return value_13
	end,
})
local implicitF32 = Flamework.createSerializer({
	serialize = function(v_13)
		local buf_28 = buffer.create(4)
		if math.abs(v_13) > 3.4028234663852886e+38 and math.abs(v_13) < math.huge then
			codec.checkWidth("f32", v_13, "value")
		end
		buffer.writef32(buf_28, 0, v_13)
		return buf_28
	end,
	deserialize = function(buf_29)
		local value_14 = buffer.readf32(buf_29, 0)
		if buffer.len(buf_29) ~= 4 then
			error("malformed payload")
		end
		return value_14
	end,
})
local implicitVarint = Flamework.createSerializer({
	serialize = function(v_14)
		local buf_30 = buffer.create(vsize(v_14))
		local o_5 = 0
		if not (v_14 >= 0 and v_14 <= 34359738367 and v_14 % 1 == 0) then
			codec.checkWidth("varint", v_14, "value")
		end
		o_5 = vwrite(buf_30, o_5, v_14)
		return buf_30
	end,
	deserialize = function(buf_31)
		local o_6 = 0
		local n_4, o_7 = vread(buf_31, o_6)
		o_6 = o_7
		if o_6 ~= buffer.len(buf_31) then
			error("malformed payload")
		end
		return n_4
	end,
})
local ownBrand = Flamework.createSerializer({
	serialize = function(v_15)
		local buf_32 = buffer.create(2)
		buffer.writeu16(buf_32, 0, v_15)
		return buf_32
	end,
	deserialize = function(buf_33)
		local value_15 = buffer.readu16(buf_33, 0)
		if buffer.len(buf_33) ~= 2 then
			error("malformed payload")
		end
		return value_15
	end,
})
-- Strings and buffers with every length prefix.
local text = Flamework.createSerializer({
	serialize = function(v_16)
		local length = #v_16
		local buf_34 = buffer.create(vsize(length) + length)
		local o_8 = 0
		local length_1 = #v_16
		o_8 = vwrite(buf_34, o_8, length_1)
		buffer.writestring(buf_34, o_8, v_16)
		o_8 += length_1
		return buf_34
	end,
	deserialize = function(buf_35)
		local o_9 = 0
		local length_2, o_10 = vread(buf_35, o_9)
		local text_1 = buffer.readstring(buf_35, o_10, length_2)
		o_9 = o_10 + length_2
		if o_9 ~= buffer.len(buf_35) then
			error("malformed payload")
		end
		return text_1
	end,
})
local text8 = Flamework.createSerializer({
	serialize = function(v_17)
		local buf_36 = buffer.create(#v_17 + 1)
		local o_11 = 0
		local length_3 = #v_17
		if length_3 > 255 then
			error("string is longer than its u8 length prefix allows")
		end
		buffer.writeu8(buf_36, o_11, length_3)
		buffer.writestring(buf_36, o_11 + 1, v_17)
		o_11 += length_3 + 1
		return buf_36
	end,
	deserialize = function(buf_37)
		local o_12 = 0
		local length_4 = buffer.readu8(buf_37, o_12)
		local text_2 = buffer.readstring(buf_37, o_12 + 1, length_4)
		o_12 += length_4 + 1
		if o_12 ~= buffer.len(buf_37) then
			error("malformed payload")
		end
		return text_2
	end,
})
local text16 = Flamework.createSerializer({
	serialize = function(v_18)
		local buf_38 = buffer.create(#v_18 + 2)
		local o_13 = 0
		local length_5 = #v_18
		if length_5 > 65535 then
			error("string is longer than its u16 length prefix allows")
		end
		buffer.writeu16(buf_38, o_13, length_5)
		buffer.writestring(buf_38, o_13 + 2, v_18)
		o_13 += length_5 + 2
		return buf_38
	end,
	deserialize = function(buf_39)
		local o_14 = 0
		local length_6 = buffer.readu16(buf_39, o_14)
		local text_3 = buffer.readstring(buf_39, o_14 + 2, length_6)
		o_14 += length_6 + 2
		if o_14 ~= buffer.len(buf_39) then
			error("malformed payload")
		end
		return text_3
	end,
})
local text32 = Flamework.createSerializer({
	serialize = function(v_19)
		local buf_40 = buffer.create(#v_19 + 4)
		local o_15 = 0
		local length_7 = #v_19
		buffer.writeu32(buf_40, o_15, length_7)
		buffer.writestring(buf_40, o_15 + 4, v_19)
		o_15 += length_7 + 4
		return buf_40
	end,
	deserialize = function(buf_41)
		local o_16 = 0
		local length_8 = buffer.readu32(buf_41, o_16)
		local text_4 = buffer.readstring(buf_41, o_16 + 4, length_8)
		o_16 += length_8 + 4
		if o_16 ~= buffer.len(buf_41) then
			error("malformed payload")
		end
		return text_4
	end,
})
local implicitText8 = Flamework.createSerializer({
	serialize = function(v_20)
		local buf_42 = buffer.create(#v_20 + 1)
		local o_17 = 0
		local length_9 = #v_20
		if length_9 > 255 then
			codec.checkWidth("string8", length_9, "value", " bytes")
			error("string is longer than its u8 length prefix allows")
		end
		buffer.writeu8(buf_42, o_17, length_9)
		buffer.writestring(buf_42, o_17 + 1, v_20)
		o_17 += length_9 + 1
		return buf_42
	end,
	deserialize = function(buf_43)
		local o_18 = 0
		local length_10 = buffer.readu8(buf_43, o_18)
		local text_5 = buffer.readstring(buf_43, o_18 + 1, length_10)
		o_18 += length_10 + 1
		if o_18 ~= buffer.len(buf_43) then
			error("malformed payload")
		end
		return text_5
	end,
})
local bytes = Flamework.createSerializer({
	serialize = function(v_21)
		local length_11 = buffer.len(v_21)
		local buf_44 = buffer.create(vsize(length_11) + length_11)
		local o_19 = 0
		local length_12 = buffer.len(v_21)
		o_19 = vwrite(buf_44, o_19, length_12)
		buffer.copy(buf_44, o_19, v_21)
		o_19 += length_12
		return buf_44
	end,
	deserialize = function(buf_45)
		local o_20 = 0
		local length_13, o_21 = vread(buf_45, o_20)
		if length_13 > buffer.len(buf_45) - o_21 then
			error("malformed payload")
		end
		local bytes_1 = buffer.create(length_13)
		buffer.copy(bytes_1, 0, buf_45, o_21, length_13)
		o_20 = o_21 + length_13
		if o_20 ~= buffer.len(buf_45) then
			error("malformed payload")
		end
		return bytes_1
	end,
})
local bytes16 = Flamework.createSerializer({
	serialize = function(v_22)
		local buf_46 = buffer.create(buffer.len(v_22) + 2)
		local o_22 = 0
		local length_14 = buffer.len(v_22)
		if length_14 > 65535 then
			error("buffer is longer than its u16 length prefix allows")
		end
		buffer.writeu16(buf_46, o_22, length_14)
		buffer.copy(buf_46, o_22 + 2, v_22)
		o_22 += length_14 + 2
		return buf_46
	end,
	deserialize = function(buf_47)
		local o_23 = 0
		local length_15 = buffer.readu16(buf_47, o_23)
		if length_15 > buffer.len(buf_47) - (o_23 + 2) then
			error("malformed payload")
		end
		local bytes_2 = buffer.create(length_15)
		buffer.copy(bytes_2, 0, buf_47, o_23 + 2, length_15)
		o_23 += length_15 + 2
		if o_23 ~= buffer.len(buf_47) then
			error("malformed payload")
		end
		return bytes_2
	end,
})
local bytes32 = Flamework.createSerializer({
	serialize = function(v_23)
		local buf_48 = buffer.create(buffer.len(v_23) + 4)
		local o_24 = 0
		local length_16 = buffer.len(v_23)
		buffer.writeu32(buf_48, o_24, length_16)
		buffer.copy(buf_48, o_24 + 4, v_23)
		o_24 += length_16 + 4
		return buf_48
	end,
	deserialize = function(buf_49)
		local o_25 = 0
		local length_17 = buffer.readu32(buf_49, o_25)
		if length_17 > buffer.len(buf_49) - (o_25 + 4) then
			error("malformed payload")
		end
		local bytes_3 = buffer.create(length_17)
		buffer.copy(bytes_3, 0, buf_49, o_25 + 4, length_17)
		o_25 += length_17 + 4
		if o_25 ~= buffer.len(buf_49) then
			error("malformed payload")
		end
		return bytes_3
	end,
})
local template = Flamework.createSerializer({
	serialize = function(v_24)
		local length_18 = #v_24
		local buf_50 = buffer.create(vsize(length_18) + length_18)
		local o_26 = 0
		local length_19 = #v_24
		o_26 = vwrite(buf_50, o_26, length_19)
		buffer.writestring(buf_50, o_26, v_24)
		o_26 += length_19
		return buf_50
	end,
	deserialize = function(buf_51)
		local o_27 = 0
		local length_20, o_28 = vread(buf_51, o_27)
		local text_6 = buffer.readstring(buf_51, o_28, length_20)
		o_27 = o_28 + length_20
		if o_27 ~= buffer.len(buf_51) then
			error("malformed payload")
		end
		return text_6
	end,
})
local upper = Flamework.createSerializer({
	serialize = function(v_25)
		local length_21 = #v_25
		local buf_52 = buffer.create(vsize(length_21) + length_21)
		local o_29 = 0
		local length_22 = #v_25
		o_29 = vwrite(buf_52, o_29, length_22)
		buffer.writestring(buf_52, o_29, v_25)
		o_29 += length_22
		return buf_52
	end,
	deserialize = function(buf_53)
		local o_30 = 0
		local length_23, o_31 = vread(buf_53, o_30)
		local text_7 = buffer.readstring(buf_53, o_31, length_23)
		o_30 = o_31 + length_23
		if o_30 ~= buffer.len(buf_53) then
			error("malformed payload")
		end
		return text_7
	end,
})
-- Booleans, literals and nothing.
local flag = Flamework.createSerializer({
	serialize = function(v_26)
		local buf_54 = buffer.create(1)
		buffer.writeu8(buf_54, 0, if v_26 then 1 else 0)
		return buf_54
	end,
	deserialize = function(buf_55)
		local value_16 = buffer.readu8(buf_55, 0) ~= 0
		if buffer.len(buf_55) ~= 1 then
			error("malformed payload")
		end
		return value_16
	end,
})
local one = Flamework.createSerializer({
	serialize = function(v_27)
		local buf_56 = buffer.create(0)
		return buf_56
	end,
	deserialize = function(buf_57)
		if buffer.len(buf_57) ~= 0 then
			error("malformed payload")
		end
		return "one"
	end,
})
local seven = Flamework.createSerializer({
	serialize = function(v_28)
		local buf_58 = buffer.create(0)
		return buf_58
	end,
	deserialize = function(buf_59)
		if buffer.len(buf_59) ~= 0 then
			error("malformed payload")
		end
		return 7
	end,
})
local literals = { "fly", "run", "walk" }
local literalIndex = {
	fly = 0,
	run = 1,
	walk = 2,
}
local mode = Flamework.createSerializer({
	serialize = function(v_29)
		local buf_60 = buffer.create(1)
		local index = literalIndex[v_29]
		if index == nil then
			error("value is not one of the literals its type allows")
		end
		buffer.writeu8(buf_60, 0, index)
		return buf_60
	end,
	deserialize = function(buf_61)
		local literal = literals[buffer.readu8(buf_61, 0) + 1]
		if literal == nil then
			error("malformed payload")
		end
		if buffer.len(buf_61) ~= 1 then
			error("malformed payload")
		end
		return literal
	end,
})
local literals_1 = { 0, 1, -1, 2, -3, 10 }
local literalIndex_1 = {
	[0] = 0,
	[1] = 1,
	[-1] = 2,
	[2] = 3,
	[-3] = 4,
	[10] = 5,
}
local numbers = Flamework.createSerializer({
	serialize = function(v_30)
		local buf_62 = buffer.create(1)
		local index_1 = literalIndex_1[v_30]
		if index_1 == nil then
			error("value is not one of the literals its type allows")
		end
		buffer.writeu8(buf_62, 0, index_1)
		return buf_62
	end,
	deserialize = function(buf_63)
		local literal_1 = literals_1[buffer.readu8(buf_63, 0) + 1]
		if literal_1 == nil then
			error("malformed payload")
		end
		if buffer.len(buf_63) ~= 1 then
			error("malformed payload")
		end
		return literal_1
	end,
})
local literals_2 = { true, "", "number", 3, "a" }
local literalIndex_2 = {
	[true] = 0,
	[""] = 1,
	number = 2,
	[3] = 3,
	a = 4,
}
local mixedLiterals = Flamework.createSerializer({
	serialize = function(v_31)
		local buf_64 = buffer.create(1)
		local index_2 = literalIndex_2[v_31]
		if index_2 == nil then
			error("value is not one of the literals its type allows")
		end
		buffer.writeu8(buf_64, 0, index_2)
		return buf_64
	end,
	deserialize = function(buf_65)
		local literal_2 = literals_2[buffer.readu8(buf_65, 0) + 1]
		if literal_2 == nil then
			error("malformed payload")
		end
		if buffer.len(buf_65) ~= 1 then
			error("malformed payload")
		end
		return literal_2
	end,
})
local nothing = Flamework.createSerializer({
	serialize = function(v_32)
		local buf_66 = buffer.create(0)
		return buf_66
	end,
	deserialize = function(buf_67)
		if buffer.len(buf_67) ~= 0 then
			error("malformed payload")
		end
		return nil
	end,
})
local maybe = Flamework.createSerializer({
	serialize = function(v_33)
		local buf_68 = buffer.create(if v_33 ~= nil then 9 else 1)
		local o_32 = 0
		buffer.writeu8(buf_68, o_32, if v_33 ~= nil then 1 else 0)
		o_32 += 1
		if v_33 ~= nil then
			buffer.writef64(buf_68, o_32, v_33)
			o_32 += 8
		end
		return buf_68
	end,
	deserialize = function(buf_69)
		local o_33 = 0
		local present = buffer.readu8(buf_69, o_33) ~= 0
		local value_17
		o_33 += 1
		if present then
			value_17 = buffer.readf64(buf_69, o_33)
			o_33 += 8
		end
		if o_33 ~= buffer.len(buf_69) then
			error("malformed payload")
		end
		return value_17
	end,
})
local maybeText = Flamework.createSerializer({
	serialize = function(v_34)
		local size = 1
		if v_34 ~= nil then
			local length_24 = #v_34
			size += vsize(length_24) + length_24
		end
		local buf_70 = buffer.create(size)
		local o_34 = 0
		buffer.writeu8(buf_70, o_34, if v_34 ~= nil then 1 else 0)
		o_34 += 1
		if v_34 ~= nil then
			local length_25 = #v_34
			o_34 = vwrite(buf_70, o_34, length_25)
			buffer.writestring(buf_70, o_34, v_34)
			o_34 += length_25
		end
		return buf_70
	end,
	deserialize = function(buf_71)
		local o_35 = 0
		local present_1 = buffer.readu8(buf_71, o_35) ~= 0
		local value_18
		o_35 += 1
		if present_1 then
			local length_26, o_36 = vread(buf_71, o_35)
			local text_8 = buffer.readstring(buf_71, o_36, length_26)
			o_35 = o_36 + length_26
			value_18 = text_8
		end
		if o_35 ~= buffer.len(buf_71) then
			error("malformed payload")
		end
		return value_18
	end,
})
-- Values that travel whole.
local anything = Flamework.createSerializer({
	serialize = function(v_35)
		local buf_72 = buffer.create(4)
		local blobs = {}
		if v_35 ~= nil then
			table.insert(blobs, v_35)
			buffer.writeu32(buf_72, 0, #blobs)
		else
			buffer.writeu32(buf_72, 0, 0)
		end
		return buf_72, blobs
	end,
	deserialize = function(buf_73, blobs_1)
		local value_19 = blobs_1[buffer.readu32(buf_73, 0)]
		if buffer.len(buf_73) ~= 4 then
			error("malformed payload")
		end
		return value_19
	end,
})
local object = Flamework.createSerializer({
	serialize = function(v_36)
		local buf_74 = buffer.create(4)
		local blobs_2 = {}
		if v_36 ~= nil then
			table.insert(blobs_2, v_36)
			buffer.writeu32(buf_74, 0, #blobs_2)
		else
			buffer.writeu32(buf_74, 0, 0)
		end
		return buf_74, blobs_2
	end,
	deserialize = function(buf_75, blobs_3)
		local value_20 = blobs_3[buffer.readu32(buf_75, 0)]
		if buffer.len(buf_75) ~= 4 then
			error("malformed payload")
		end
		return value_20
	end,
})
local part = Flamework.createSerializer({
	serialize = function(v_37)
		local buf_76 = buffer.create(4)
		local blobs_4 = {}
		if v_37 ~= nil then
			table.insert(blobs_4, v_37)
			buffer.writeu32(buf_76, 0, #blobs_4)
		else
			buffer.writeu32(buf_76, 0, 0)
		end
		return buf_76, blobs_4
	end,
	deserialize = function(buf_77, blobs_5)
		local value_21 = blobs_5[buffer.readu32(buf_77, 0)]
		if buffer.len(buf_77) ~= 4 then
			error("malformed payload")
		end
		return value_21
	end,
})
local font = Flamework.createSerializer({
	serialize = function(v_38)
		local buf_78 = buffer.create(4)
		local blobs_6 = {}
		if v_38 ~= nil then
			table.insert(blobs_6, v_38)
			buffer.writeu32(buf_78, 0, #blobs_6)
		else
			buffer.writeu32(buf_78, 0, 0)
		end
		return buf_78, blobs_6
	end,
	deserialize = function(buf_79, blobs_7)
		local value_22 = blobs_7[buffer.readu32(buf_79, 0)]
		if buffer.len(buf_79) ~= 4 then
			error("malformed payload")
		end
		return value_22
	end,
})
local signal = Flamework.createSerializer({
	serialize = function(v_39)
		local buf_80 = buffer.create(4)
		local blobs_8 = {}
		if v_39 ~= nil then
			table.insert(blobs_8, v_39)
			buffer.writeu32(buf_80, 0, #blobs_8)
		else
			buffer.writeu32(buf_80, 0, 0)
		end
		return buf_80, blobs_8
	end,
	deserialize = function(buf_81, blobs_9)
		local value_23 = blobs_9[buffer.readu32(buf_81, 0)]
		if buffer.len(buf_81) ~= 4 then
			error("malformed payload")
		end
		return value_23
	end,
})
local Thing
do
	Thing = setmetatable({}, {
		__tostring = function()
			return "Thing"
		end,
	})
	Thing.__index = Thing
	function Thing.new(...)
		local self = setmetatable({}, Thing)
		return self:constructor(...) or self
	end
	function Thing:constructor()
		self.value = 1
	end
end
local thing = Flamework.createSerializer({
	serialize = function(v_40)
		local buf_82 = buffer.create(4)
		local blobs_10 = {}
		if v_40 ~= nil then
			table.insert(blobs_10, v_40)
			buffer.writeu32(buf_82, 0, #blobs_10)
		else
			buffer.writeu32(buf_82, 0, 0)
		end
		return buf_82, blobs_10
	end,
	deserialize = function(buf_83, blobs_11)
		local value_24 = blobs_11[buffer.readu32(buf_83, 0)]
		if buffer.len(buf_83) ~= 4 then
			error("malformed payload")
		end
		return value_24
	end,
})
return {
	f64 = f64,
	u8 = u8,
	i8 = i8,
	u16 = u16,
	i16 = i16,
	u32 = u32,
	i32 = i32,
	u64 = u64,
	i64 = i64,
	f32 = f32,
	varint = varint,
	implicitU8 = implicitU8,
	implicitI64 = implicitI64,
	implicitF32 = implicitF32,
	implicitVarint = implicitVarint,
	ownBrand = ownBrand,
	text = text,
	text8 = text8,
	text16 = text16,
	text32 = text32,
	implicitText8 = implicitText8,
	bytes = bytes,
	bytes16 = bytes16,
	bytes32 = bytes32,
	template = template,
	upper = upper,
	flag = flag,
	one = one,
	seven = seven,
	mode = mode,
	numbers = numbers,
	mixedLiterals = mixedLiterals,
	nothing = nothing,
	maybe = maybe,
	maybeText = maybeText,
	anything = anything,
	object = object,
	part = part,
	font = font,
	signal = signal,
	thing = thing,
}
