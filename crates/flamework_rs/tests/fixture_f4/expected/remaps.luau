-- Compiled with roblox-ts v3.0.0
local TS = _G[script]
local Flamework = TS.import(script, TS.getModule(script, "@flamework-experimental", "core").out).Flamework
local Dir = {
	Up = "up",
	Down = "down",
	Left = "left",
}
--[[
	Mapped types whose `as` clause renames their keys: their fields go by the key each was made for
	(`links.keyType`), before the renaming; several keys that name one field go where the first of them would.
]]
local spreadRemapValue = function()
	local _object = table.clone(remapped)
	setmetatable(_object, nil)
	_object.q = 1
	return _object
end
local remapRev = Flamework.createSerializer({
	version = 1,
}, {
	version = 1,
	serialize = function(v)
		local buf = buffer.create(7)
		buffer.writeu8(buf, 0, 1)
		buffer.writeu32(buf, 1, 3205040167)
		buffer.writeu8(buf, 5, v.z)
		buffer.writeu8(buf, 6, v.y)
		return buf
	end,
	deserialize = function(buf_1)
		if buffer.len(buf_1) < 5 then
			error("[Flamework] this buffer is too short for the header of RemapRev, version 1: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_1, 0) ~= 1 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_1, 0)}, and this serializer of RemapRev reads version 1: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_1, 1) ~= 3205040167 then
			error("[Flamework] RemapRev's layout changed since this buffer was written as version 1; bump the version and keep a reader for the old one")
		end
		local value = {
			z = buffer.readu8(buf_1, 5),
			y = buffer.readu8(buf_1, 6),
		}
		if buffer.len(buf_1) ~= 7 then
			error("malformed payload")
		end
		return value
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
codec.s_RemapNum = function(v_1)
	local length = #(v_1.b)
	local length_1 = #(v_1.a)
	return vsize(length) + length + (vsize(length_1) + length_1)
end
codec.w_RemapNum = function(buf_4, o_2, v_1)
	local text = v_1.b
	local length_2 = #text
	o_2 = vwrite(buf_4, o_2, length_2)
	buffer.writestring(buf_4, o_2, text)
	o_2 += length_2
	local text_1 = v_1.a
	local length_3 = #text_1
	o_2 = vwrite(buf_4, o_2, length_3)
	buffer.writestring(buf_4, o_2, text_1)
	o_2 += length_3
	return o_2
end
codec.r_RemapNum = function(buf_4, o_2)
	local length_4, o_3 = vread(buf_4, o_2)
	local text_2 = buffer.readstring(buf_4, o_3, length_4)
	o_2 = o_3 + length_4
	local length_5, o_4 = vread(buf_4, o_2)
	local text_3 = buffer.readstring(buf_4, o_4, length_5)
	o_2 = o_4 + length_5
	local value_1 = {
		b = text_2,
		a = text_3,
	}
	return value_1, o_2
end
local remapNum = Flamework.createSerializer({
	version = 2,
}, {
	version = 2,
	serialize = function(v_2)
		local buf_5 = buffer.create(codec.s_RemapNum(v_2) + 5)
		buffer.writeu8(buf_5, 0, 2)
		buffer.writeu32(buf_5, 1, 1642729269)
		codec.w_RemapNum(buf_5, 5, v_2)
		return buf_5
	end,
	deserialize = function(buf_6)
		if buffer.len(buf_6) < 5 then
			error("[Flamework] this buffer is too short for the header of RemapNum, version 2: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_6, 0) ~= 2 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_6, 0)}, and this serializer of RemapNum reads version 2: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_6, 1) ~= 1642729269 then
			error("[Flamework] RemapNum's layout changed since this buffer was written as version 2; bump the version and keep a reader for the old one")
		end
		local value_2, o_5 = codec.r_RemapNum(buf_6, 5)
		if o_5 ~= buffer.len(buf_6) then
			error("malformed payload")
		end
		return value_2
	end,
})
local remapMulti = Flamework.createSerializer({
	version = 3,
}, {
	version = 3,
	serialize = function(v_3)
		local buf_7 = buffer.create(8)
		buffer.writeu8(buf_7, 0, 3)
		buffer.writeu32(buf_7, 1, 1647224758)
		buffer.writeu8(buf_7, 5, v_3.z1)
		buffer.writeu8(buf_7, 6, v_3.z2)
		buffer.writeu8(buf_7, 7, v_3.y)
		return buf_7
	end,
	deserialize = function(buf_8)
		if buffer.len(buf_8) < 5 then
			error("[Flamework] this buffer is too short for the header of RemapMulti, version 3: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_8, 0) ~= 3 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_8, 0)}, and this serializer of RemapMulti reads version 3: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_8, 1) ~= 1647224758 then
			error("[Flamework] RemapMulti's layout changed since this buffer was written as version 3; bump the version and keep a reader for the old one")
		end
		local value_3 = {
			z1 = buffer.readu8(buf_8, 5),
			z2 = buffer.readu8(buf_8, 6),
			y = buffer.readu8(buf_8, 7),
		}
		if buffer.len(buf_8) ~= 8 then
			error("malformed payload")
		end
		return value_3
	end,
})
local remapTmpl = Flamework.createSerializer({
	version = 4,
}, {
	version = 4,
	serialize = function(v_4)
		local buf_9 = buffer.create(7)
		buffer.writeu8(buf_9, 0, 4)
		buffer.writeu32(buf_9, 1, 2093738968)
		buffer.writeu8(buf_9, 5, v_4.x_a)
		buffer.writeu8(buf_9, 6, v_4.x_b)
		return buf_9
	end,
	deserialize = function(buf_10)
		if buffer.len(buf_10) < 5 then
			error("[Flamework] this buffer is too short for the header of RemapTmpl, version 4: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_10, 0) ~= 4 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_10, 0)}, and this serializer of RemapTmpl reads version 4: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_10, 1) ~= 2093738968 then
			error("[Flamework] RemapTmpl's layout changed since this buffer was written as version 4; bump the version and keep a reader for the old one")
		end
		local value_4 = {
			x_a = buffer.readu8(buf_10, 5),
			x_b = buffer.readu8(buf_10, 6),
		}
		if buffer.len(buf_10) ~= 7 then
			error("malformed payload")
		end
		return value_4
	end,
})
local trueBranch = Flamework.createSerializer({
	version = 5,
}, {
	version = 5,
	serialize = function(v_5)
		local buf_11 = buffer.create(7)
		buffer.writeu8(buf_11, 0, 5)
		buffer.writeu32(buf_11, 1, 360934417)
		buffer.writeu8(buf_11, 5, v_5.a)
		buffer.writeu8(buf_11, 6, v_5.c)
		return buf_11
	end,
	deserialize = function(buf_12)
		if buffer.len(buf_12) < 5 then
			error("[Flamework] this buffer is too short for the header of TrueBranch, version 5: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_12, 0) ~= 5 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_12, 0)}, and this serializer of TrueBranch reads version 5: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_12, 1) ~= 360934417 then
			error("[Flamework] TrueBranch's layout changed since this buffer was written as version 5; bump the version and keep a reader for the old one")
		end
		local value_5 = {
			a = buffer.readu8(buf_12, 5),
			c = buffer.readu8(buf_12, 6),
		}
		if buffer.len(buf_12) ~= 7 then
			error("malformed payload")
		end
		return value_5
	end,
})
local shadowed = Flamework.createSerializer({
	version = 6,
}, {
	version = 6,
	serialize = function(v_6)
		local buf_13 = buffer.create(7)
		buffer.writeu8(buf_13, 0, 6)
		buffer.writeu32(buf_13, 1, 970178940)
		buffer.writeu8(buf_13, 5, v_6.a)
		buffer.writeu8(buf_13, 6, v_6.b)
		return buf_13
	end,
	deserialize = function(buf_14)
		if buffer.len(buf_14) < 5 then
			error("[Flamework] this buffer is too short for the header of Shadowed, version 6: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_14, 0) ~= 6 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_14, 0)}, and this serializer of Shadowed reads version 6: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_14, 1) ~= 970178940 then
			error("[Flamework] Shadowed's layout changed since this buffer was written as version 6; bump the version and keep a reader for the old one")
		end
		local value_6 = {
			a = buffer.readu8(buf_14, 5),
			b = buffer.readu8(buf_14, 6),
		}
		if buffer.len(buf_14) ~= 7 then
			error("malformed payload")
		end
		return value_6
	end,
})
local excluded = Flamework.createSerializer({
	version = 7,
}, {
	version = 7,
	serialize = function(v_7)
		local buf_15 = buffer.create(7)
		buffer.writeu8(buf_15, 0, 7)
		buffer.writeu32(buf_15, 1, 360934417)
		buffer.writeu8(buf_15, 5, v_7.a)
		buffer.writeu8(buf_15, 6, v_7.c)
		return buf_15
	end,
	deserialize = function(buf_16)
		if buffer.len(buf_16) < 5 then
			error("[Flamework] this buffer is too short for the header of Excluded, version 7: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_16, 0) ~= 7 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_16, 0)}, and this serializer of Excluded reads version 7: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_16, 1) ~= 360934417 then
			error("[Flamework] Excluded's layout changed since this buffer was written as version 7; bump the version and keep a reader for the old one")
		end
		local value_7 = {
			a = buffer.readu8(buf_16, 5),
			c = buffer.readu8(buf_16, 6),
		}
		if buffer.len(buf_16) ~= 7 then
			error("malformed payload")
		end
		return value_7
	end,
})
local extracted = Flamework.createSerializer({
	version = 8,
}, {
	version = 8,
	serialize = function(v_8)
		local buf_17 = buffer.create(7)
		buffer.writeu8(buf_17, 0, 8)
		buffer.writeu32(buf_17, 1, 970178940)
		buffer.writeu8(buf_17, 5, v_8.a)
		buffer.writeu8(buf_17, 6, v_8.b)
		return buf_17
	end,
	deserialize = function(buf_18)
		if buffer.len(buf_18) < 5 then
			error("[Flamework] this buffer is too short for the header of Extracted, version 8: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_18, 0) ~= 8 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_18, 0)}, and this serializer of Extracted reads version 8: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_18, 1) ~= 970178940 then
			error("[Flamework] Extracted's layout changed since this buffer was written as version 8; bump the version and keep a reader for the old one")
		end
		local value_8 = {
			a = buffer.readu8(buf_18, 5),
			b = buffer.readu8(buf_18, 6),
		}
		if buffer.len(buf_18) ~= 7 then
			error("malformed payload")
		end
		return value_8
	end,
})
local merged = Flamework.createSerializer({
	version = 9,
}, {
	version = 9,
	serialize = function(v_9)
		local buf_19 = buffer.create(8)
		buffer.writeu8(buf_19, 0, 9)
		buffer.writeu32(buf_19, 1, 1597296013)
		buffer.writeu8(buf_19, 5, v_9.a)
		buffer.writeu8(buf_19, 6, v_9.b)
		buffer.writeu8(buf_19, 7, v_9.d)
		return buf_19
	end,
	deserialize = function(buf_20)
		if buffer.len(buf_20) < 5 then
			error("[Flamework] this buffer is too short for the header of Merged, version 9: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_20, 0) ~= 9 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_20, 0)}, and this serializer of Merged reads version 9: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_20, 1) ~= 1597296013 then
			error("[Flamework] Merged's layout changed since this buffer was written as version 9; bump the version and keep a reader for the old one")
		end
		local value_9 = {
			a = buffer.readu8(buf_20, 5),
			b = buffer.readu8(buf_20, 6),
			d = buffer.readu8(buf_20, 7),
		}
		if buffer.len(buf_20) ~= 8 then
			error("malformed payload")
		end
		return value_9
	end,
})
local upper = Flamework.createSerializer({
	version = 10,
}, {
	version = 10,
	serialize = function(v_10)
		local buf_21 = buffer.create(8)
		buffer.writeu8(buf_21, 0, 10)
		buffer.writeu32(buf_21, 1, 2403167853)
		buffer.writeu8(buf_21, 5, v_10.A)
		buffer.writeu8(buf_21, 6, v_10.B)
		buffer.writeu8(buf_21, 7, v_10.C)
		return buf_21
	end,
	deserialize = function(buf_22)
		if buffer.len(buf_22) < 5 then
			error("[Flamework] this buffer is too short for the header of Upper, version 10: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_22, 0) ~= 10 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_22, 0)}, and this serializer of Upper reads version 10: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_22, 1) ~= 2403167853 then
			error("[Flamework] Upper's layout changed since this buffer was written as version 10; bump the version and keep a reader for the old one")
		end
		local value_10 = {
			A = buffer.readu8(buf_22, 5),
			B = buffer.readu8(buf_22, 6),
			C = buffer.readu8(buf_22, 7),
		}
		if buffer.len(buf_22) ~= 8 then
			error("malformed payload")
		end
		return value_10
	end,
})
local numbered = Flamework.createSerializer({
	version = 11,
}, {
	version = 11,
	serialize = function(v_11)
		local buf_23 = buffer.create(8)
		buffer.writeu8(buf_23, 0, 11)
		buffer.writeu32(buf_23, 1, 1206031541)
		buffer.writeu8(buf_23, 5, v_11.n1)
		buffer.writeu8(buf_23, 6, v_11.n2)
		buffer.writeu8(buf_23, 7, v_11.n10)
		return buf_23
	end,
	deserialize = function(buf_24)
		if buffer.len(buf_24) < 5 then
			error("[Flamework] this buffer is too short for the header of Numbered, version 11: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_24, 0) ~= 11 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_24, 0)}, and this serializer of Numbered reads version 11: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_24, 1) ~= 1206031541 then
			error("[Flamework] Numbered's layout changed since this buffer was written as version 11; bump the version and keep a reader for the old one")
		end
		local value_11 = {
			n1 = buffer.readu8(buf_24, 5),
			n2 = buffer.readu8(buf_24, 6),
			n10 = buffer.readu8(buf_24, 7),
		}
		if buffer.len(buf_24) ~= 8 then
			error("malformed payload")
		end
		return value_11
	end,
})
local directions = Flamework.createSerializer({
	version = 12,
}, {
	version = 12,
	serialize = function(v_12)
		local buf_25 = buffer.create(8)
		buffer.writeu8(buf_25, 0, 12)
		buffer.writeu32(buf_25, 1, 3690003689)
		buffer.writeu8(buf_25, 5, v_12.d_up)
		buffer.writeu8(buf_25, 6, v_12.d_down)
		buffer.writeu8(buf_25, 7, v_12.d_left)
		return buf_25
	end,
	deserialize = function(buf_26)
		if buffer.len(buf_26) < 5 then
			error("[Flamework] this buffer is too short for the header of Directions, version 12: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_26, 0) ~= 12 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_26, 0)}, and this serializer of Directions reads version 12: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_26, 1) ~= 3690003689 then
			error("[Flamework] Directions's layout changed since this buffer was written as version 12; bump the version and keep a reader for the old one")
		end
		local value_12 = {
			d_up = buffer.readu8(buf_26, 5),
			d_down = buffer.readu8(buf_26, 6),
			d_left = buffer.readu8(buf_26, 7),
		}
		if buffer.len(buf_26) ~= 8 then
			error("malformed payload")
		end
		return value_12
	end,
})
codec.s_Homomorphic = function(v_13)
	local length_6 = #(v_13.get_alpha)
	return vsize(length_6) + length_6 + 2
end
codec.w_Homomorphic = function(buf_27, o_6, v_13)
	buffer.writeu8(buf_27, o_6, v_13.get_zeta)
	local text_4 = v_13.get_alpha
	local length_7 = #text_4
	o_6 = vwrite(buf_27, o_6 + 1, length_7)
	buffer.writestring(buf_27, o_6, text_4)
	o_6 += length_7
	buffer.writeu8(buf_27, o_6, if v_13.get_mid then 1 else 0)
	o_6 += 1
	return o_6
end
codec.r_Homomorphic = function(buf_27, o_6)
	local get_zeta = buffer.readu8(buf_27, o_6)
	local length_8, o_7 = vread(buf_27, o_6 + 1)
	local text_5 = buffer.readstring(buf_27, o_7, length_8)
	o_6 = o_7 + length_8
	local get_mid = buffer.readu8(buf_27, o_6) ~= 0
	local value_13 = {
		get_zeta = get_zeta,
		get_alpha = text_5,
		get_mid = get_mid,
	}
	o_6 += 1
	return value_13, o_6
end
local homomorphic = Flamework.createSerializer({
	version = 13,
}, {
	version = 13,
	serialize = function(v_14)
		local buf_28 = buffer.create(codec.s_Homomorphic(v_14) + 5)
		buffer.writeu8(buf_28, 0, 13)
		buffer.writeu32(buf_28, 1, 1295277828)
		codec.w_Homomorphic(buf_28, 5, v_14)
		return buf_28
	end,
	deserialize = function(buf_29)
		if buffer.len(buf_29) < 5 then
			error("[Flamework] this buffer is too short for the header of Homomorphic, version 13: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_29, 0) ~= 13 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_29, 0)}, and this serializer of Homomorphic reads version 13: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_29, 1) ~= 1295277828 then
			error("[Flamework] Homomorphic's layout changed since this buffer was written as version 13; bump the version and keep a reader for the old one")
		end
		local value_14, o_8 = codec.r_Homomorphic(buf_29, 5)
		if o_8 ~= buffer.len(buf_29) then
			error("malformed payload")
		end
		return value_14
	end,
})
local spreadRemap = Flamework.createSerializer({
	version = 14,
}, {
	version = 14,
	serialize = function(v_15)
		local buf_30 = buffer.create(8)
		buffer.writeu8(buf_30, 0, 14)
		buffer.writeu32(buf_30, 1, 765851391)
		buffer.writeu8(buf_30, 5, v_15.q)
		buffer.writeu8(buf_30, 6, v_15.z)
		buffer.writeu8(buf_30, 7, v_15.y)
		return buf_30
	end,
	deserialize = function(buf_31)
		if buffer.len(buf_31) < 5 then
			error("[Flamework] this buffer is too short for the header of SpreadRemap, version 14: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_31, 0) ~= 14 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_31, 0)}, and this serializer of SpreadRemap reads version 14: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_31, 1) ~= 765851391 then
			error("[Flamework] SpreadRemap's layout changed since this buffer was written as version 14; bump the version and keep a reader for the old one")
		end
		local value_15 = {
			q = buffer.readu8(buf_31, 5),
			z = buffer.readu8(buf_31, 6),
			y = buffer.readu8(buf_31, 7),
		}
		if buffer.len(buf_31) ~= 8 then
			error("malformed payload")
		end
		return value_15
	end,
})
codec.s_Partial = function(v_16)
	local v_17 = v_16.z
	local v_18 = v_16.y
	return (if v_17 ~= nil then 2 else 1) + (if v_18 ~= nil then 2 else 1)
end
codec.w_Partial = function(buf_32, o_9, v_16)
	local v_19 = v_16.z
	buffer.writeu8(buf_32, o_9, if v_19 ~= nil then 1 else 0)
	o_9 += 1
	if v_19 ~= nil then
		buffer.writeu8(buf_32, o_9, v_19)
		o_9 += 1
	end
	local v_20 = v_16.y
	buffer.writeu8(buf_32, o_9, if v_20 ~= nil then 1 else 0)
	o_9 += 1
	if v_20 ~= nil then
		buffer.writeu8(buf_32, o_9, v_20)
		o_9 += 1
	end
	return o_9
end
codec.r_Partial = function(buf_32, o_9)
	local present = buffer.readu8(buf_32, o_9) ~= 0
	local value_16
	o_9 += 1
	if present then
		value_16 = buffer.readu8(buf_32, o_9)
		o_9 += 1
	end
	local present_1 = buffer.readu8(buf_32, o_9) ~= 0
	local value_17
	o_9 += 1
	if present_1 then
		value_17 = buffer.readu8(buf_32, o_9)
		o_9 += 1
	end
	local value_18 = {
		z = value_16,
		y = value_17,
	}
	return value_18, o_9
end
local partialRemap = Flamework.createSerializer({
	version = 15,
}, {
	version = 15,
	serialize = function(v_21)
		local buf_33 = buffer.create(codec.s_Partial(v_21) + 5)
		buffer.writeu8(buf_33, 0, 15)
		buffer.writeu32(buf_33, 1, 1365748141)
		codec.w_Partial(buf_33, 5, v_21)
		return buf_33
	end,
	deserialize = function(buf_34)
		if buffer.len(buf_34) < 5 then
			error("[Flamework] this buffer is too short for the header of PartialRemap, version 15: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_34, 0) ~= 15 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_34, 0)}, and this serializer of PartialRemap reads version 15: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_34, 1) ~= 1365748141 then
			error("[Flamework] PartialRemap's layout changed since this buffer was written as version 15; bump the version and keep a reader for the old one")
		end
		local value_19, o_10 = codec.r_Partial(buf_34, 5)
		if o_10 ~= buffer.len(buf_34) then
			error("malformed payload")
		end
		return value_19
	end,
})
return {
	remapRev = remapRev,
	remapNum = remapNum,
	remapMulti = remapMulti,
	remapTmpl = remapTmpl,
	trueBranch = trueBranch,
	shadowed = shadowed,
	excluded = excluded,
	extracted = extracted,
	merged = merged,
	upper = upper,
	numbered = numbered,
	directions = directions,
	homomorphic = homomorphic,
	spreadRemap = spreadRemap,
	partialRemap = partialRemap,
}
