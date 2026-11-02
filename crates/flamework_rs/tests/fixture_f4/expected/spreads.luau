-- Compiled with roblox-ts v3.0.0
local TS = _G[script]
local Flamework = TS.import(script, TS.getModule(script, "@flamework-experimental", "core").out).Flamework
--[[
	Properties a spread or an object rest copied out of a mapped type: each keeps its link to the mapped type that made
	it (`links.mappedType`), and its run of fields goes in that type's order.
]]
local spreadPickValue = function()
	local _object = {
		first = 1,
	}
	for _k, _v in picked do
		_object[_k] = _v
	end
	return _object
end
local spreadRecValue = function()
	local _object = table.clone(recorded)
	setmetatable(_object, nil)
	_object.k = true
	return _object
end
local spreadOmitValue = function()
	local _object = table.clone(omitted)
	setmetatable(_object, nil)
	_object.extra = 1
	return _object
end
local spreadChainValue = function()
	local _object = table.clone(chained)
	setmetatable(_object, nil)
	_object.n = 2
	return _object
end
-- Two mapped runs with a written property between them.
local twoRunsValue = function()
	local _object = {
		a1 = 1,
	}
	for _k, _v in keyed do
		_object[_k] = _v
	end
	_object.z9 = true
	for _k, _v in picked do
		_object[_k] = _v
	end
	return _object
end
-- Object rests of one run (properties one mapped type made, a single property): their order does not depend on the
-- order the rest lists them in.
local restValue = function()
	local _b = four.b
	local _rest = {}
	for _key, _value in four do
		if _key ~= "b" then
			_rest[_key] = _value
		end
	end
	local rest = _rest
	return rest
end
local oneLeftValue = function()
	local _a = two.a
	local _rest = {}
	for _key, _value in two do
		if _key ~= "a" then
			_rest[_key] = _value
		end
	end
	local rest = _rest
	return rest
end
-- A spread of a rest of one run: the rest's properties keep their block, in the mapped type's order.
local fromRestValue = function()
	local _object = {
		k = true,
	}
	for _k, _v in restOfFour do
		_object[_k] = _v
	end
	_object.z = 1
	return _object
end
-- A generic interface's instance: its mapped bases are instantiated for the argument.
local spreadExtendedValue = function()
	local _object = table.clone(extended)
	setmetatable(_object, nil)
	_object.z = 1
	return _object
end
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
local spreadPick = Flamework.createSerializer({
	version = 1,
}, {
	version = 1,
	serialize = function(v)
		local length = #(v.alpha)
		local buf_2 = buffer.create(vsize(length) + length + 2 + 5)
		buffer.writeu8(buf_2, 0, 1)
		buffer.writeu32(buf_2, 1, 754612628)
		local o_2 = 5
		local text = v.alpha
		local length_1 = #text
		o_2 = vwrite(buf_2, o_2, length_1)
		buffer.writestring(buf_2, o_2, text)
		o_2 += length_1
		buffer.writeu8(buf_2, o_2, v.zeta)
		buffer.writeu8(buf_2, o_2 + 1, v.first)
		return buf_2
	end,
	deserialize = function(buf_3)
		if buffer.len(buf_3) < 5 then
			error("[Flamework] this buffer is too short for the header of SpreadPick, version 1: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_3, 0) ~= 1 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_3, 0)}, and this serializer of SpreadPick reads version 1: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_3, 1) ~= 754612628 then
			error("[Flamework] SpreadPick's layout changed since this buffer was written as version 1; bump the version and keep a reader for the old one")
		end
		local o_3 = 5
		local length_2, o_4 = vread(buf_3, o_3)
		local text_1 = buffer.readstring(buf_3, o_4, length_2)
		o_3 = o_4 + length_2
		local zeta_1 = buffer.readu8(buf_3, o_3)
		local first_1 = buffer.readu8(buf_3, o_3 + 1)
		local value = {
			alpha = text_1,
			zeta = zeta_1,
			first = first_1,
		}
		o_3 += 2
		if o_3 ~= buffer.len(buf_3) then
			error("malformed payload")
		end
		return value
	end,
})
local spreadRec = Flamework.createSerializer({
	version = 2,
}, {
	version = 2,
	serialize = function(v_1)
		local buf_4 = buffer.create(8)
		buffer.writeu8(buf_4, 0, 2)
		buffer.writeu32(buf_4, 1, 3008227866)
		buffer.writeu8(buf_4, 5, if v_1.k then 1 else 0)
		buffer.writeu8(buf_4, 6, v_1.aa)
		buffer.writeu8(buf_4, 7, v_1.zz)
		return buf_4
	end,
	deserialize = function(buf_5)
		if buffer.len(buf_5) < 5 then
			error("[Flamework] this buffer is too short for the header of SpreadRec, version 2: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_5, 0) ~= 2 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_5, 0)}, and this serializer of SpreadRec reads version 2: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_5, 1) ~= 3008227866 then
			error("[Flamework] SpreadRec's layout changed since this buffer was written as version 2; bump the version and keep a reader for the old one")
		end
		local value_1 = {
			k = buffer.readu8(buf_5, 5) ~= 0,
			aa = buffer.readu8(buf_5, 6),
			zz = buffer.readu8(buf_5, 7),
		}
		if buffer.len(buf_5) ~= 8 then
			error("malformed payload")
		end
		return value_1
	end,
})
local spreadOmit = Flamework.createSerializer({
	version = 3,
}, {
	version = 3,
	serialize = function(v_2)
		local length_3 = #(v_2.alpha)
		local buf_6 = buffer.create(vsize(length_3) + length_3 + 2 + 5)
		buffer.writeu8(buf_6, 0, 3)
		buffer.writeu32(buf_6, 1, 3546260729)
		local o_5 = 5
		local text_2 = v_2.alpha
		local length_4 = #text_2
		o_5 = vwrite(buf_6, o_5, length_4)
		buffer.writestring(buf_6, o_5, text_2)
		o_5 += length_4
		buffer.writeu8(buf_6, o_5, v_2.zeta)
		buffer.writeu8(buf_6, o_5 + 1, v_2.extra)
		return buf_6
	end,
	deserialize = function(buf_7)
		if buffer.len(buf_7) < 5 then
			error("[Flamework] this buffer is too short for the header of SpreadOmit, version 3: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_7, 0) ~= 3 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_7, 0)}, and this serializer of SpreadOmit reads version 3: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_7, 1) ~= 3546260729 then
			error("[Flamework] SpreadOmit's layout changed since this buffer was written as version 3; bump the version and keep a reader for the old one")
		end
		local o_6 = 5
		local length_5, o_7 = vread(buf_7, o_6)
		local text_3 = buffer.readstring(buf_7, o_7, length_5)
		o_6 = o_7 + length_5
		local zeta_2 = buffer.readu8(buf_7, o_6)
		local extra_1 = buffer.readu8(buf_7, o_6 + 1)
		local value_2 = {
			alpha = text_3,
			zeta = zeta_2,
			extra = extra_1,
		}
		o_6 += 2
		if o_6 ~= buffer.len(buf_7) then
			error("malformed payload")
		end
		return value_2
	end,
})
local spreadChain = Flamework.createSerializer({
	version = 4,
}, {
	version = 4,
	serialize = function(v_3)
		local v_4 = v_3.alpha
		local size = 1
		if v_4 ~= nil then
			local length_6 = #v_4
			size += vsize(length_6) + length_6
		end
		local v_5 = v_3.zeta
		local buf_8 = buffer.create(size + (if v_5 ~= nil then 2 else 1) + 1 + 5)
		buffer.writeu8(buf_8, 0, 4)
		buffer.writeu32(buf_8, 1, 994114310)
		local o_8 = 5
		local v_6 = v_3.alpha
		buffer.writeu8(buf_8, o_8, if v_6 ~= nil then 1 else 0)
		o_8 += 1
		if v_6 ~= nil then
			local length_7 = #v_6
			o_8 = vwrite(buf_8, o_8, length_7)
			buffer.writestring(buf_8, o_8, v_6)
			o_8 += length_7
		end
		local v_7 = v_3.zeta
		buffer.writeu8(buf_8, o_8, if v_7 ~= nil then 1 else 0)
		o_8 += 1
		if v_7 ~= nil then
			buffer.writeu8(buf_8, o_8, v_7)
			o_8 += 1
		end
		buffer.writeu8(buf_8, o_8, v_3.n)
		return buf_8
	end,
	deserialize = function(buf_9)
		if buffer.len(buf_9) < 5 then
			error("[Flamework] this buffer is too short for the header of SpreadChain, version 4: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_9, 0) ~= 4 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_9, 0)}, and this serializer of SpreadChain reads version 4: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_9, 1) ~= 994114310 then
			error("[Flamework] SpreadChain's layout changed since this buffer was written as version 4; bump the version and keep a reader for the old one")
		end
		local o_9 = 5
		local present = buffer.readu8(buf_9, o_9) ~= 0
		local value_3
		o_9 += 1
		if present then
			local length_8, o_10 = vread(buf_9, o_9)
			local text_4 = buffer.readstring(buf_9, o_10, length_8)
			o_9 = o_10 + length_8
			value_3 = text_4
		end
		local present_1 = buffer.readu8(buf_9, o_9) ~= 0
		local value_4
		o_9 += 1
		if present_1 then
			value_4 = buffer.readu8(buf_9, o_9)
			o_9 += 1
		end
		local n_4 = buffer.readu8(buf_9, o_9)
		local value_5 = {
			alpha = value_3,
			zeta = value_4,
			n = n_4,
		}
		o_9 += 1
		if o_9 ~= buffer.len(buf_9) then
			error("malformed payload")
		end
		return value_5
	end,
})
local twoRuns = Flamework.createSerializer({
	version = 5,
}, {
	version = 5,
	serialize = function(v_8)
		local length_9 = #(v_8.alpha)
		local buf_10 = buffer.create(vsize(length_9) + length_9 + 5 + 5)
		buffer.writeu8(buf_10, 0, 5)
		buffer.writeu32(buf_10, 1, 3975425654)
		local o_11 = 5
		local text_5 = v_8.alpha
		local length_10 = #text_5
		o_11 = vwrite(buf_10, o_11, length_10)
		buffer.writestring(buf_10, o_11, text_5)
		o_11 += length_10
		buffer.writeu8(buf_10, o_11, v_8.zeta)
		buffer.writeu8(buf_10, o_11 + 1, v_8.a1)
		buffer.writeu8(buf_10, o_11 + 2, if v_8.z9 then 1 else 0)
		buffer.writeu8(buf_10, o_11 + 3, v_8.c)
		buffer.writeu8(buf_10, o_11 + 4, v_8.q)
		return buf_10
	end,
	deserialize = function(buf_11)
		if buffer.len(buf_11) < 5 then
			error("[Flamework] this buffer is too short for the header of TwoRuns, version 5: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_11, 0) ~= 5 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_11, 0)}, and this serializer of TwoRuns reads version 5: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_11, 1) ~= 3975425654 then
			error("[Flamework] TwoRuns's layout changed since this buffer was written as version 5; bump the version and keep a reader for the old one")
		end
		local o_12 = 5
		local length_11, o_13 = vread(buf_11, o_12)
		local text_6 = buffer.readstring(buf_11, o_13, length_11)
		o_12 = o_13 + length_11
		local zeta_3 = buffer.readu8(buf_11, o_12)
		local a1_1 = buffer.readu8(buf_11, o_12 + 1)
		local z9_1 = buffer.readu8(buf_11, o_12 + 2) ~= 0
		local c_1 = buffer.readu8(buf_11, o_12 + 3)
		local q_1 = buffer.readu8(buf_11, o_12 + 4)
		local value_6 = {
			alpha = text_6,
			zeta = zeta_3,
			a1 = a1_1,
			z9 = z9_1,
			c = c_1,
			q = q_1,
		}
		o_12 += 5
		if o_12 ~= buffer.len(buf_11) then
			error("malformed payload")
		end
		return value_6
	end,
})
local rest = Flamework.createSerializer({
	version = 6,
}, {
	version = 6,
	serialize = function(v_9)
		local buf_12 = buffer.create(8)
		buffer.writeu8(buf_12, 0, 6)
		buffer.writeu32(buf_12, 1, 2701451297)
		buffer.writeu8(buf_12, 5, v_9.a)
		buffer.writeu8(buf_12, 6, v_9.c)
		buffer.writeu8(buf_12, 7, v_9.d)
		return buf_12
	end,
	deserialize = function(buf_13)
		if buffer.len(buf_13) < 5 then
			error("[Flamework] this buffer is too short for the header of Rest, version 6: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_13, 0) ~= 6 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_13, 0)}, and this serializer of Rest reads version 6: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_13, 1) ~= 2701451297 then
			error("[Flamework] Rest's layout changed since this buffer was written as version 6; bump the version and keep a reader for the old one")
		end
		local value_7 = {
			a = buffer.readu8(buf_13, 5),
			c = buffer.readu8(buf_13, 6),
			d = buffer.readu8(buf_13, 7),
		}
		if buffer.len(buf_13) ~= 8 then
			error("malformed payload")
		end
		return value_7
	end,
})
local spreadExtended = Flamework.createSerializer({
	version = 7,
}, {
	version = 7,
	serialize = function(v_10)
		local buf_14 = buffer.create(11)
		buffer.writeu8(buf_14, 0, 7)
		buffer.writeu32(buf_14, 1, 3074865978)
		buffer.writeu8(buf_14, 5, if v_10.mid then 1 else 0)
		buffer.writeu8(buf_14, 6, v_10.zeta)
		buffer.writeu8(buf_14, 7, v_10.own)
		buffer.writeu8(buf_14, 8, v_10.z)
		buffer.writeu8(buf_14, 9, v_10.b)
		buffer.writeu8(buf_14, 10, v_10.y)
		return buf_14
	end,
	deserialize = function(buf_15)
		if buffer.len(buf_15) < 5 then
			error("[Flamework] this buffer is too short for the header of SpreadExtended, version 7: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_15, 0) ~= 7 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_15, 0)}, and this serializer of SpreadExtended reads version 7: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_15, 1) ~= 3074865978 then
			error("[Flamework] SpreadExtended's layout changed since this buffer was written as version 7; bump the version and keep a reader for the old one")
		end
		local value_8 = {
			mid = buffer.readu8(buf_15, 5) ~= 0,
			zeta = buffer.readu8(buf_15, 6),
			own = buffer.readu8(buf_15, 7),
			z = buffer.readu8(buf_15, 8),
			b = buffer.readu8(buf_15, 9),
			y = buffer.readu8(buf_15, 10),
		}
		if buffer.len(buf_15) ~= 11 then
			error("malformed payload")
		end
		return value_8
	end,
})
local generic = Flamework.createSerializer({
	version = 8,
}, {
	version = 8,
	serialize = function(v_11)
		local buf_16 = buffer.create(10)
		buffer.writeu8(buf_16, 0, 8)
		buffer.writeu32(buf_16, 1, 1482917304)
		buffer.writeu8(buf_16, 5, v_11.own)
		buffer.writeu8(buf_16, 6, if v_11.mid then 1 else 0)
		buffer.writeu8(buf_16, 7, v_11.zeta)
		buffer.writeu8(buf_16, 8, v_11.b)
		buffer.writeu8(buf_16, 9, v_11.y)
		return buf_16
	end,
	deserialize = function(buf_17)
		if buffer.len(buf_17) < 5 then
			error("[Flamework] this buffer is too short for the header of Extended<Base>, version 8: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_17, 0) ~= 8 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_17, 0)}, and this serializer of Extended<Base> reads version 8: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_17, 1) ~= 1482917304 then
			error("[Flamework] Extended<Base>'s layout changed since this buffer was written as version 8; bump the version and keep a reader for the old one")
		end
		local value_9 = {
			own = buffer.readu8(buf_17, 5),
			mid = buffer.readu8(buf_17, 6) ~= 0,
			zeta = buffer.readu8(buf_17, 7),
			b = buffer.readu8(buf_17, 8),
			y = buffer.readu8(buf_17, 9),
		}
		if buffer.len(buf_17) ~= 10 then
			error("malformed payload")
		end
		return value_9
	end,
})
local codec = {}
codec.s_Partial = function(v_12)
	local v_13 = v_12.alpha
	local size_1 = 1
	if v_13 ~= nil then
		local length_12 = #v_13
		size_1 += vsize(length_12) + length_12
	end
	local v_14 = v_12.zeta
	local v_15 = v_12.first
	return size_1 + (if v_14 ~= nil then 2 else 1) + (if v_15 ~= nil then 2 else 1)
end
codec.w_Partial = function(buf_18, o_14, v_12)
	local v_16 = v_12.alpha
	buffer.writeu8(buf_18, o_14, if v_16 ~= nil then 1 else 0)
	o_14 += 1
	if v_16 ~= nil then
		local length_13 = #v_16
		o_14 = vwrite(buf_18, o_14, length_13)
		buffer.writestring(buf_18, o_14, v_16)
		o_14 += length_13
	end
	local v_17 = v_12.zeta
	buffer.writeu8(buf_18, o_14, if v_17 ~= nil then 1 else 0)
	o_14 += 1
	if v_17 ~= nil then
		buffer.writeu8(buf_18, o_14, v_17)
		o_14 += 1
	end
	local v_18 = v_12.first
	buffer.writeu8(buf_18, o_14, if v_18 ~= nil then 1 else 0)
	o_14 += 1
	if v_18 ~= nil then
		buffer.writeu8(buf_18, o_14, v_18)
		o_14 += 1
	end
	return o_14
end
codec.r_Partial = function(buf_18, o_14)
	local present_2 = buffer.readu8(buf_18, o_14) ~= 0
	local value_10
	o_14 += 1
	if present_2 then
		local length_14, o_15 = vread(buf_18, o_14)
		local text_7 = buffer.readstring(buf_18, o_15, length_14)
		o_14 = o_15 + length_14
		value_10 = text_7
	end
	local present_3 = buffer.readu8(buf_18, o_14) ~= 0
	local value_11
	o_14 += 1
	if present_3 then
		value_11 = buffer.readu8(buf_18, o_14)
		o_14 += 1
	end
	local present_4 = buffer.readu8(buf_18, o_14) ~= 0
	local value_12
	o_14 += 1
	if present_4 then
		value_12 = buffer.readu8(buf_18, o_14)
		o_14 += 1
	end
	local value_13 = {
		alpha = value_10,
		zeta = value_11,
		first = value_12,
	}
	return value_13, o_14
end
local partialSpread = Flamework.createSerializer({
	version = 9,
}, {
	version = 9,
	serialize = function(v_19)
		local buf_19 = buffer.create(codec.s_Partial(v_19) + 5)
		buffer.writeu8(buf_19, 0, 9)
		buffer.writeu32(buf_19, 1, 1237792099)
		codec.w_Partial(buf_19, 5, v_19)
		return buf_19
	end,
	deserialize = function(buf_20)
		if buffer.len(buf_20) < 5 then
			error("[Flamework] this buffer is too short for the header of PartialSpread, version 9: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_20, 0) ~= 9 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_20, 0)}, and this serializer of PartialSpread reads version 9: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_20, 1) ~= 1237792099 then
			error("[Flamework] PartialSpread's layout changed since this buffer was written as version 9; bump the version and keep a reader for the old one")
		end
		local value_14, o_16 = codec.r_Partial(buf_20, 5)
		if o_16 ~= buffer.len(buf_20) then
			error("malformed payload")
		end
		return value_14
	end,
})
codec.s_SpreadOmit = function(v_20)
	local length_15 = #(v_20.alpha)
	return vsize(length_15) + length_15 + 2
end
codec.w_SpreadOmit = function(buf_21, o_17, v_20)
	local text_8 = v_20.alpha
	local length_16 = #text_8
	o_17 = vwrite(buf_21, o_17, length_16)
	buffer.writestring(buf_21, o_17, text_8)
	o_17 += length_16
	buffer.writeu8(buf_21, o_17, v_20.zeta)
	buffer.writeu8(buf_21, o_17 + 1, v_20.extra)
	o_17 += 2
	return o_17
end
codec.r_SpreadOmit = function(buf_21, o_17)
	local length_17, o_18 = vread(buf_21, o_17)
	local text_9 = buffer.readstring(buf_21, o_18, length_17)
	o_17 = o_18 + length_17
	local zeta_4 = buffer.readu8(buf_21, o_17)
	local extra_2 = buffer.readu8(buf_21, o_17 + 1)
	local value_15 = {
		alpha = text_9,
		zeta = zeta_4,
		extra = extra_2,
	}
	o_17 += 2
	return value_15, o_17
end
local inArray = Flamework.createSerializer({
	version = 10,
}, {
	version = 10,
	serialize = function(v_21)
		local array = v_21.list
		local n_5 = #array
		local size_2 = vsize(n_5)
		for i = 1, n_5 do
			local item = array[i]
			if item == nil then
				error(`[Flamework] the array has no value at value.list[{i - 1}]`)
			end
			size_2 += codec.s_SpreadOmit(item)
		end
		local map_1 = v_21.map
		local size_3 = 0
		local n_6 = 0
		for key, entry in map_1 do
			n_6 += 1
			local length_18 = #key
			size_3 += vsize(length_18) + length_18 + 3
		end
		size_3 += vsize(n_6)
		local buf_22 = buffer.create(size_2 + size_3 + 5)
		buffer.writeu8(buf_22, 0, 10)
		buffer.writeu32(buf_22, 1, 601137068)
		local o_19 = 5
		local array_1 = v_21.list
		o_19 = vwrite(buf_22, o_19, #array_1)
		for i_1 = 1, #array_1 do
			local item_1 = array_1[i_1]
			o_19 = codec.w_SpreadOmit(buf_22, o_19, item_1)
		end
		local map_2 = v_21.map
		-- ▼ ReadonlyMap.size ▼
		local _size = 0
		for _ in map_2 do
			_size += 1
		end
		-- ▲ ReadonlyMap.size ▲
		local n_7 = _size
		o_19 = vwrite(buf_22, o_19, n_7)
		for key_1, entry_1 in map_2 do
			local length_19 = #key_1
			o_19 = vwrite(buf_22, o_19, length_19)
			buffer.writestring(buf_22, o_19, key_1)
			o_19 += length_19
			buffer.writeu8(buf_22, o_19, if entry_1.k then 1 else 0)
			buffer.writeu8(buf_22, o_19 + 1, entry_1.aa)
			buffer.writeu8(buf_22, o_19 + 2, entry_1.zz)
			o_19 += 3
		end
		return buf_22
	end,
	deserialize = function(buf_23)
		if buffer.len(buf_23) < 5 then
			error("[Flamework] this buffer is too short for the header of { list: SpreadOmit[]; map: Map<string, SpreadRec> }, version 10: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_23, 0) ~= 10 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_23, 0)}, and this serializer of \{ list: SpreadOmit[]; map: Map<string, SpreadRec> \} reads version 10: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_23, 1) ~= 601137068 then
			error("[Flamework] { list: SpreadOmit[]; map: Map<string, SpreadRec> }'s layout changed since this buffer was written as version 10; bump the version and keep a reader for the old one")
		end
		local o_20 = 5
		local count, o_21 = vread(buf_23, o_20)
		o_20 = o_21
		if count * 3 > buffer.len(buf_23) - o_20 then
			error("malformed payload")
		end
		local array_2 = table.create(count)
		for i_2 = 1, count do
			local value_16, o_22 = codec.r_SpreadOmit(buf_23, o_20)
			array_2[i_2] = value_16
			o_20 = o_22
		end
		local count_1, o_23 = vread(buf_23, o_20)
		o_20 = o_23
		if count_1 * 4 > buffer.len(buf_23) - o_20 then
			error("malformed payload")
		end
		local map_3 = {}
		for _ = 1, count_1 do
			local length_20, o_24 = vread(buf_23, o_20)
			local text_10 = buffer.readstring(buf_23, o_24, length_20)
			o_20 = o_24 + length_20
			local k_1 = buffer.readu8(buf_23, o_20) ~= 0
			local aa_1 = buffer.readu8(buf_23, o_20 + 1)
			local zz_1 = buffer.readu8(buf_23, o_20 + 2)
			local entry_2 = {
				k = k_1,
				aa = aa_1,
				zz = zz_1,
			}
			map_3[text_10] = entry_2
			o_20 += 3
		end
		local value_17 = {
			list = array_2,
			map = map_3,
		}
		if o_20 ~= buffer.len(buf_23) then
			error("malformed payload")
		end
		return value_17
	end,
})
local oneLeft = Flamework.createSerializer({
	version = 11,
}, {
	version = 11,
	serialize = function(v_22)
		local buf_24 = buffer.create(9)
		buffer.writeu8(buf_24, 0, 11)
		buffer.writeu32(buf_24, 1, 1334739964)
		local object = v_22.left
		buffer.writeu8(buf_24, 5, object.b)
		local object_1 = v_22.rest
		buffer.writeu8(buf_24, 6, object_1.a)
		buffer.writeu8(buf_24, 7, object_1.c)
		buffer.writeu8(buf_24, 8, object_1.d)
		return buf_24
	end,
	deserialize = function(buf_25)
		if buffer.len(buf_25) < 5 then
			error("[Flamework] this buffer is too short for the header of { left: OneLeft; rest: Rest }, version 11: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_25, 0) ~= 11 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_25, 0)}, and this serializer of \{ left: OneLeft; rest: Rest \} reads version 11: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_25, 1) ~= 1334739964 then
			error("[Flamework] { left: OneLeft; rest: Rest }'s layout changed since this buffer was written as version 11; bump the version and keep a reader for the old one")
		end
		local value_18 = {
			left = {
				b = buffer.readu8(buf_25, 5),
			},
			rest = {
				a = buffer.readu8(buf_25, 6),
				c = buffer.readu8(buf_25, 7),
				d = buffer.readu8(buf_25, 8),
			},
		}
		if buffer.len(buf_25) ~= 9 then
			error("malformed payload")
		end
		return value_18
	end,
})
local fromRest = Flamework.createSerializer({
	version = 12,
}, {
	version = 12,
	serialize = function(v_23)
		local buf_26 = buffer.create(10)
		buffer.writeu8(buf_26, 0, 12)
		buffer.writeu32(buf_26, 1, 1954398951)
		buffer.writeu8(buf_26, 5, if v_23.k then 1 else 0)
		buffer.writeu8(buf_26, 6, v_23.z)
		buffer.writeu8(buf_26, 7, v_23.a)
		buffer.writeu8(buf_26, 8, v_23.c)
		buffer.writeu8(buf_26, 9, v_23.d)
		return buf_26
	end,
	deserialize = function(buf_27)
		if buffer.len(buf_27) < 5 then
			error("[Flamework] this buffer is too short for the header of FromRest, version 12: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_27, 0) ~= 12 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_27, 0)}, and this serializer of FromRest reads version 12: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_27, 1) ~= 1954398951 then
			error("[Flamework] FromRest's layout changed since this buffer was written as version 12; bump the version and keep a reader for the old one")
		end
		local value_19 = {
			k = buffer.readu8(buf_27, 5) ~= 0,
			z = buffer.readu8(buf_27, 6),
			a = buffer.readu8(buf_27, 7),
			c = buffer.readu8(buf_27, 8),
			d = buffer.readu8(buf_27, 9),
		}
		if buffer.len(buf_27) ~= 10 then
			error("malformed payload")
		end
		return value_19
	end,
})
return {
	spreadPick = spreadPick,
	spreadRec = spreadRec,
	spreadOmit = spreadOmit,
	spreadChain = spreadChain,
	twoRuns = twoRuns,
	rest = rest,
	spreadExtended = spreadExtended,
	generic = generic,
	partialSpread = partialSpread,
	inArray = inArray,
	oneLeft = oneLeft,
	fromRest = fromRest,
}
