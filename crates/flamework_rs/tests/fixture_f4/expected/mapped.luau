-- Compiled with roblox-ts v3.0.0
local TS = _G[script]
local Flamework = TS.import(script, TS.getModule(script, "@flamework-experimental", "core").out).Flamework
-- Mapped types and the types that inherit their properties (spreads and `as` remaps: spreads.ts, remaps.ts).
-- A generic interface's instance: its mapped bases are instantiated for the argument.
-- `as` clauses that give each key itself or drop it.
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
codec.s_ExtPick = function(v)
	local length = #(v.alpha)
	return vsize(length) + length + 2
end
codec.w_ExtPick = function(buf_2, o_2, v)
	buffer.writeu8(buf_2, o_2, v.own)
	local text = v.alpha
	local length_1 = #text
	o_2 = vwrite(buf_2, o_2 + 1, length_1)
	buffer.writestring(buf_2, o_2, text)
	o_2 += length_1
	buffer.writeu8(buf_2, o_2, v.zeta)
	o_2 += 1
	return o_2
end
codec.r_ExtPick = function(buf_2, o_2)
	local own_1 = buffer.readu8(buf_2, o_2)
	local length_2, o_3 = vread(buf_2, o_2 + 1)
	local text_1 = buffer.readstring(buf_2, o_3, length_2)
	o_2 = o_3 + length_2
	local zeta_1 = buffer.readu8(buf_2, o_2)
	local value = {
		own = own_1,
		alpha = text_1,
		zeta = zeta_1,
	}
	o_2 += 1
	return value, o_2
end
local extPick = Flamework.createSerializer({
	version = 1,
}, {
	version = 1,
	serialize = function(v_1)
		local buf_3 = buffer.create(codec.s_ExtPick(v_1) + 5)
		buffer.writeu8(buf_3, 0, 1)
		buffer.writeu32(buf_3, 1, 2260264203)
		codec.w_ExtPick(buf_3, 5, v_1)
		return buf_3
	end,
	deserialize = function(buf_4)
		if buffer.len(buf_4) < 5 then
			error("[Flamework] this buffer is too short for the header of ExtPick, version 1: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_4, 0) ~= 1 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_4, 0)}, and this serializer of ExtPick reads version 1: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_4, 1) ~= 2260264203 then
			error("[Flamework] ExtPick's layout changed since this buffer was written as version 1; bump the version and keep a reader for the old one")
		end
		local value_1, o_4 = codec.r_ExtPick(buf_4, 5)
		if o_4 ~= buffer.len(buf_4) then
			error("malformed payload")
		end
		return value_1
	end,
})
codec.s_InterChain = function(v_2)
	local v_3 = v_2.alpha
	local size = 1
	if v_3 ~= nil then
		local length_3 = #v_3
		size += vsize(length_3) + length_3
	end
	local v_4 = v_2.mid
	return size + (if v_4 ~= nil then 2 else 1) + 1
end
codec.w_InterChain = function(buf_5, o_5, v_2)
	buffer.writeu8(buf_5, o_5, v_2.zeta)
	local v_5 = v_2.alpha
	buffer.writeu8(buf_5, o_5 + 1, if v_5 ~= nil then 1 else 0)
	o_5 += 2
	if v_5 ~= nil then
		local length_4 = #v_5
		o_5 = vwrite(buf_5, o_5, length_4)
		buffer.writestring(buf_5, o_5, v_5)
		o_5 += length_4
	end
	local v_6 = v_2.mid
	buffer.writeu8(buf_5, o_5, if v_6 ~= nil then 1 else 0)
	o_5 += 1
	if v_6 ~= nil then
		buffer.writeu8(buf_5, o_5, if v_6 then 1 else 0)
		o_5 += 1
	end
	return o_5
end
codec.r_InterChain = function(buf_5, o_5)
	local zeta_2 = buffer.readu8(buf_5, o_5)
	local present = buffer.readu8(buf_5, o_5 + 1) ~= 0
	local value_2
	o_5 += 2
	if present then
		local length_5, o_6 = vread(buf_5, o_5)
		local text_2 = buffer.readstring(buf_5, o_6, length_5)
		o_5 = o_6 + length_5
		value_2 = text_2
	end
	local present_1 = buffer.readu8(buf_5, o_5) ~= 0
	local value_3
	o_5 += 1
	if present_1 then
		value_3 = buffer.readu8(buf_5, o_5) ~= 0
		o_5 += 1
	end
	local value_4 = {
		zeta = zeta_2,
		alpha = value_2,
		mid = value_3,
	}
	return value_4, o_5
end
local interChain = Flamework.createSerializer({
	version = 2,
}, {
	version = 2,
	serialize = function(v_7)
		local buf_6 = buffer.create(codec.s_InterChain(v_7) + 5)
		buffer.writeu8(buf_6, 0, 2)
		buffer.writeu32(buf_6, 1, 2306523165)
		codec.w_InterChain(buf_6, 5, v_7)
		return buf_6
	end,
	deserialize = function(buf_7)
		if buffer.len(buf_7) < 5 then
			error("[Flamework] this buffer is too short for the header of InterChain, version 2: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_7, 0) ~= 2 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_7, 0)}, and this serializer of InterChain reads version 2: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_7, 1) ~= 2306523165 then
			error("[Flamework] InterChain's layout changed since this buffer was written as version 2; bump the version and keep a reader for the old one")
		end
		local value_5, o_7 = codec.r_InterChain(buf_7, 5)
		if o_7 ~= buffer.len(buf_7) then
			error("malformed payload")
		end
		return value_5
	end,
})
codec.s_Readonly = function(v_8)
	local v_9 = v_8.mid
	local v_10 = v_8.zeta
	return (if v_9 ~= nil then 2 else 1) + (if v_10 ~= nil then 2 else 1)
end
codec.w_Readonly = function(buf_8, o_8, v_8)
	local v_11 = v_8.mid
	buffer.writeu8(buf_8, o_8, if v_11 ~= nil then 1 else 0)
	o_8 += 1
	if v_11 ~= nil then
		buffer.writeu8(buf_8, o_8, if v_11 then 1 else 0)
		o_8 += 1
	end
	local v_12 = v_8.zeta
	buffer.writeu8(buf_8, o_8, if v_12 ~= nil then 1 else 0)
	o_8 += 1
	if v_12 ~= nil then
		buffer.writeu8(buf_8, o_8, v_12)
		o_8 += 1
	end
	return o_8
end
codec.r_Readonly = function(buf_8, o_8)
	local present_2 = buffer.readu8(buf_8, o_8) ~= 0
	local value_6
	o_8 += 1
	if present_2 then
		value_6 = buffer.readu8(buf_8, o_8) ~= 0
		o_8 += 1
	end
	local present_3 = buffer.readu8(buf_8, o_8) ~= 0
	local value_7
	o_8 += 1
	if present_3 then
		value_7 = buffer.readu8(buf_8, o_8)
		o_8 += 1
	end
	local value_8 = {
		mid = value_6,
		zeta = value_7,
	}
	return value_8, o_8
end
local omitPartial = Flamework.createSerializer({
	version = 3,
}, {
	version = 3,
	serialize = function(v_13)
		local buf_9 = buffer.create(codec.s_Readonly(v_13) + 5)
		buffer.writeu8(buf_9, 0, 3)
		buffer.writeu32(buf_9, 1, 462209100)
		codec.w_Readonly(buf_9, 5, v_13)
		return buf_9
	end,
	deserialize = function(buf_10)
		if buffer.len(buf_10) < 5 then
			error("[Flamework] this buffer is too short for the header of OmitPartial, version 3: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_10, 0) ~= 3 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_10, 0)}, and this serializer of OmitPartial reads version 3: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_10, 1) ~= 462209100 then
			error("[Flamework] OmitPartial's layout changed since this buffer was written as version 3; bump the version and keep a reader for the old one")
		end
		local value_9, o_9 = codec.r_Readonly(buf_10, 5)
		if o_9 ~= buffer.len(buf_10) then
			error("malformed payload")
		end
		return value_9
	end,
})
local keyed = Flamework.createSerializer({
	version = 4,
}, {
	version = 4,
	serialize = function(v_14)
		local buf_11 = buffer.create(8)
		buffer.writeu8(buf_11, 0, 4)
		buffer.writeu32(buf_11, 1, 762812039)
		buffer.writeu8(buf_11, 5, v_14.aa)
		buffer.writeu8(buf_11, 6, v_14.mm)
		buffer.writeu8(buf_11, 7, v_14.zz)
		return buf_11
	end,
	deserialize = function(buf_12)
		if buffer.len(buf_12) < 5 then
			error("[Flamework] this buffer is too short for the header of Keyed, version 4: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_12, 0) ~= 4 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_12, 0)}, and this serializer of Keyed reads version 4: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_12, 1) ~= 762812039 then
			error("[Flamework] Keyed's layout changed since this buffer was written as version 4; bump the version and keep a reader for the old one")
		end
		local value_10 = {
			aa = buffer.readu8(buf_12, 5),
			mm = buffer.readu8(buf_12, 6),
			zz = buffer.readu8(buf_12, 7),
		}
		if buffer.len(buf_12) ~= 8 then
			error("malformed payload")
		end
		return value_10
	end,
})
codec.s_Homomorphic = function(v_15)
	local length_6 = #(v_15.alpha)
	return vsize(length_6) + length_6 + 1
end
codec.w_Homomorphic = function(buf_13, o_10, v_15)
	buffer.writeu8(buf_13, o_10, v_15.zeta)
	local text_3 = v_15.alpha
	local length_7 = #text_3
	o_10 = vwrite(buf_13, o_10 + 1, length_7)
	buffer.writestring(buf_13, o_10, text_3)
	o_10 += length_7
	return o_10
end
codec.r_Homomorphic = function(buf_13, o_10)
	local zeta_3 = buffer.readu8(buf_13, o_10)
	local length_8, o_11 = vread(buf_13, o_10 + 1)
	local text_4 = buffer.readstring(buf_13, o_11, length_8)
	o_10 = o_11 + length_8
	local value_11 = {
		zeta = zeta_3,
		alpha = text_4,
	}
	return value_11, o_10
end
local homomorphic = Flamework.createSerializer({
	version = 5,
}, {
	version = 5,
	serialize = function(v_16)
		local buf_14 = buffer.create(codec.s_Homomorphic(v_16) + 5)
		buffer.writeu8(buf_14, 0, 5)
		buffer.writeu32(buf_14, 1, 2058273299)
		codec.w_Homomorphic(buf_14, 5, v_16)
		return buf_14
	end,
	deserialize = function(buf_15)
		if buffer.len(buf_15) < 5 then
			error("[Flamework] this buffer is too short for the header of Homomorphic, version 5: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_15, 0) ~= 5 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_15, 0)}, and this serializer of Homomorphic reads version 5: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_15, 1) ~= 2058273299 then
			error("[Flamework] Homomorphic's layout changed since this buffer was written as version 5; bump the version and keep a reader for the old one")
		end
		local value_12, o_12 = codec.r_Homomorphic(buf_15, 5)
		if o_12 ~= buffer.len(buf_15) then
			error("malformed payload")
		end
		return value_12
	end,
})
codec.s_ExtGenericDeep = function(v_17)
	local length_9 = #(v_17.alpha)
	return vsize(length_9) + length_9 + 5
end
codec.w_ExtGenericDeep = function(buf_16, o_13, v_17)
	buffer.writeu8(buf_16, o_13, v_17.x)
	local text_5 = v_17.alpha
	local length_10 = #text_5
	o_13 = vwrite(buf_16, o_13 + 1, length_10)
	buffer.writestring(buf_16, o_13, text_5)
	o_13 += length_10
	buffer.writeu8(buf_16, o_13, v_17.zeta)
	buffer.writeu8(buf_16, o_13 + 1, v_17.own)
	buffer.writeu8(buf_16, o_13 + 2, v_17.b)
	buffer.writeu8(buf_16, o_13 + 3, v_17.q)
	o_13 += 4
	return o_13
end
codec.r_ExtGenericDeep = function(buf_16, o_13)
	local x_1 = buffer.readu8(buf_16, o_13)
	local length_11, o_14 = vread(buf_16, o_13 + 1)
	local text_6 = buffer.readstring(buf_16, o_14, length_11)
	o_13 = o_14 + length_11
	local zeta_4 = buffer.readu8(buf_16, o_13)
	local own_2 = buffer.readu8(buf_16, o_13 + 1)
	local b_2 = buffer.readu8(buf_16, o_13 + 2)
	local q_1 = buffer.readu8(buf_16, o_13 + 3)
	local value_13 = {
		x = x_1,
		alpha = text_6,
		zeta = zeta_4,
		own = own_2,
		b = b_2,
		q = q_1,
	}
	o_13 += 4
	return value_13, o_13
end
local extGeneric = Flamework.createSerializer({
	version = 6,
}, {
	version = 6,
	serialize = function(v_18)
		local buf_17 = buffer.create(codec.s_ExtGenericDeep(v_18) + 5)
		buffer.writeu8(buf_17, 0, 6)
		buffer.writeu32(buf_17, 1, 61248167)
		codec.w_ExtGenericDeep(buf_17, 5, v_18)
		return buf_17
	end,
	deserialize = function(buf_18)
		if buffer.len(buf_18) < 5 then
			error("[Flamework] this buffer is too short for the header of ExtGenericDeep<Base>, version 6: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_18, 0) ~= 6 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_18, 0)}, and this serializer of ExtGenericDeep<Base> reads version 6: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_18, 1) ~= 61248167 then
			error("[Flamework] ExtGenericDeep<Base>'s layout changed since this buffer was written as version 6; bump the version and keep a reader for the old one")
		end
		local value_14, o_15 = codec.r_ExtGenericDeep(buf_18, 5)
		if o_15 ~= buffer.len(buf_18) then
			error("malformed payload")
		end
		return value_14
	end,
})
local identity = Flamework.createSerializer({
	version = 7,
}, {
	version = 7,
	serialize = function(v_19)
		local buf_19 = buffer.create(7)
		buffer.writeu8(buf_19, 0, 7)
		buffer.writeu32(buf_19, 1, 970178940)
		buffer.writeu8(buf_19, 5, v_19.a)
		buffer.writeu8(buf_19, 6, v_19.b)
		return buf_19
	end,
	deserialize = function(buf_20)
		if buffer.len(buf_20) < 5 then
			error("[Flamework] this buffer is too short for the header of Identity, version 7: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_20, 0) ~= 7 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_20, 0)}, and this serializer of Identity reads version 7: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_20, 1) ~= 970178940 then
			error("[Flamework] Identity's layout changed since this buffer was written as version 7; bump the version and keep a reader for the old one")
		end
		local value_15 = {
			a = buffer.readu8(buf_20, 5),
			b = buffer.readu8(buf_20, 6),
		}
		if buffer.len(buf_20) ~= 7 then
			error("malformed payload")
		end
		return value_15
	end,
})
local filtered = Flamework.createSerializer({
	version = 8,
}, {
	version = 8,
	serialize = function(v_20)
		local buf_21 = buffer.create(7)
		buffer.writeu8(buf_21, 0, 8)
		buffer.writeu32(buf_21, 1, 192802371)
		buffer.writeu8(buf_21, 5, v_20.a)
		buffer.writeu8(buf_21, 6, v_20.d)
		return buf_21
	end,
	deserialize = function(buf_22)
		if buffer.len(buf_22) < 5 then
			error("[Flamework] this buffer is too short for the header of Filtered, version 8: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_22, 0) ~= 8 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_22, 0)}, and this serializer of Filtered reads version 8: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_22, 1) ~= 192802371 then
			error("[Flamework] Filtered's layout changed since this buffer was written as version 8; bump the version and keep a reader for the old one")
		end
		local value_16 = {
			a = buffer.readu8(buf_22, 5),
			d = buffer.readu8(buf_22, 6),
		}
		if buffer.len(buf_22) ~= 7 then
			error("malformed payload")
		end
		return value_16
	end,
})
return {
	extPick = extPick,
	interChain = interChain,
	omitPartial = omitPartial,
	keyed = keyed,
	homomorphic = homomorphic,
	extGeneric = extGeneric,
	identity = identity,
	filtered = filtered,
}
