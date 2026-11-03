-- Compiled with roblox-ts v3.0.0
local TS = _G[script]
local Flamework = TS.import(script, TS.getModule(script, "@flamework-experimental", "core").out).Flamework
-- Properties TS5's name lookup cannot find again are left out: unique-symbol keys and private names.
-- Leading underscores are a name like any other.
--[[
	A union nothing spells (`Box<A | B>` reaches `value: T`) orders its members by a key of each type, whose object
	text spells a symbol key as TS5 does (`__@sym@12`, before letters).
]]
local symbolKeyed = Flamework.createSerializer({
	version = 1,
}, {
	version = 1,
	serialize = function(v)
		local buf = buffer.create(6)
		buffer.writeu8(buf, 0, 1)
		buffer.writeu32(buf, 1, 3521894566)
		buffer.writeu8(buf, 5, v.b)
		return buf
	end,
	deserialize = function(buf_1)
		if buffer.len(buf_1) < 5 then
			error("[Flamework] this buffer is too short for the header of SymbolKeyed, version 1: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_1, 0) ~= 1 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_1, 0)}, and this serializer of SymbolKeyed reads version 1: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_1, 1) ~= 3521894566 then
			error("[Flamework] SymbolKeyed's layout changed since this buffer was written as version 1; bump the version and keep a reader for the old one")
		end
		local value_1 = {
			b = buffer.readu8(buf_1, 5),
		}
		if buffer.len(buf_1) ~= 6 then
			error("malformed payload")
		end
		return value_1
	end,
})
local mappedSymbol = Flamework.createSerializer({
	version = 2,
}, {
	version = 2,
	serialize = function(v_1)
		local buf_2 = buffer.create(6)
		buffer.writeu8(buf_2, 0, 2)
		buffer.writeu32(buf_2, 1, 3521894566)
		buffer.writeu8(buf_2, 5, v_1.b)
		return buf_2
	end,
	deserialize = function(buf_3)
		if buffer.len(buf_3) < 5 then
			error("[Flamework] this buffer is too short for the header of MappedSymbol, version 2: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_3, 0) ~= 2 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_3, 0)}, and this serializer of MappedSymbol reads version 2: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_3, 1) ~= 3521894566 then
			error("[Flamework] MappedSymbol's layout changed since this buffer was written as version 2; bump the version and keep a reader for the old one")
		end
		local value_2 = {
			b = buffer.readu8(buf_3, 5),
		}
		if buffer.len(buf_3) ~= 6 then
			error("malformed payload")
		end
		return value_2
	end,
})
local recordSymbol = Flamework.createSerializer({
	serialize = function(v_2)
		local buf_4 = buffer.create(2)
		buffer.writeu8(buf_4, 0, v_2.a)
		buffer.writeu8(buf_4, 1, v_2.z)
		return buf_4
	end,
	deserialize = function(buf_5)
		local value_3 = {
			a = buffer.readu8(buf_5, 0),
			z = buffer.readu8(buf_5, 1),
		}
		if buffer.len(buf_5) ~= 2 then
			error("malformed payload")
		end
		return value_3
	end,
})
local privateNames = Flamework.createSerializer({
	serialize = function(v_3)
		local buf_6 = buffer.create(4)
		local blobs = {}
		if v_3 ~= nil then
			table.insert(blobs, v_3)
			buffer.writeu32(buf_6, 0, #blobs)
		else
			buffer.writeu32(buf_6, 0, 0)
		end
		return buf_6, blobs
	end,
	deserialize = function(buf_7, blobs_1)
		local value_4 = blobs_1[buffer.readu32(buf_7, 0)]
		if buffer.len(buf_7) ~= 4 then
			error("malformed payload")
		end
		return value_4
	end,
})
local codec = {}
local vsize = function(n_1)
	return if n_1 < 128 then 1 elseif n_1 < 16384 then 2 elseif n_1 < 2097152 then 3 elseif n_1 < 268435456 then 4 else 5
end
local vwrite = function(buf_8, o, n_2)
	while n_2 >= 128 do
		buffer.writeu8(buf_8, o, n_2 % 128 + 128)
		o += 1
		n_2 = math.floor(n_2 / 128)
	end
	buffer.writeu8(buf_8, o, n_2)
	return o + 1
end
local vread = function(buf_9, o_1)
	local n_3 = 0
	local scale = 1
	while true do
		local b_1 = buffer.readu8(buf_9, o_1)
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
codec.s_Nested = function(v_4)
	local array = v_4.list
	local n_4 = #array
	return vsize(n_4) + n_4 * 1 + 1
end
codec.w_Nested = function(buf_10, o_2, v_4)
	local object = v_4.inner
	buffer.writeu8(buf_10, o_2, object.keep)
	local array_1 = v_4.list
	o_2 = vwrite(buf_10, o_2 + 1, #array_1)
	for i = 1, #array_1 do
		local item = array_1[i]
		if item == nil then
			error(`[Flamework] the array has no value at Nested.list[{i - 1}]`)
		end
		buffer.writeu8(buf_10, o_2, item.x)
		o_2 += 1
	end
	return o_2
end
codec.r_Nested = function(buf_10, o_2)
	local keep_1 = buffer.readu8(buf_10, o_2)
	local inner_1 = {
		keep = keep_1,
	}
	local count, o_3 = vread(buf_10, o_2 + 1)
	o_2 = o_3
	if count > buffer.len(buf_10) - o_2 then
		error("malformed payload")
	end
	local array_2 = table.create(count)
	for i_1 = 1, count do
		local x_1 = buffer.readu8(buf_10, o_2)
		array_2[i_1] = {
			x = x_1,
		}
		o_2 += 1
	end
	local value_5 = {
		inner = inner_1,
		list = array_2,
	}
	return value_5, o_2
end
local nested = Flamework.createSerializer({
	serialize = function(v_5)
		local buf_11 = buffer.create(codec.s_Nested(v_5))
		codec.w_Nested(buf_11, 0, v_5)
		return buf_11
	end,
	deserialize = function(buf_12)
		local value_6, o_4 = codec.r_Nested(buf_12, 0)
		if o_4 ~= buffer.len(buf_12) then
			error("malformed payload")
		end
		return value_6
	end,
})
local underscored = Flamework.createSerializer({
	serialize = function(v_6)
		local buf_13 = buffer.create(3)
		buffer.writeu8(buf_13, 0, v_6.__x)
		buffer.writeu8(buf_13, 1, v_6.___y)
		buffer.writeu8(buf_13, 2, v_6.a)
		return buf_13
	end,
	deserialize = function(buf_14)
		local value_7 = {
			__x = buffer.readu8(buf_14, 0),
			___y = buffer.readu8(buf_14, 1),
			a = buffer.readu8(buf_14, 2),
		}
		if buffer.len(buf_14) ~= 3 then
			error("malformed payload")
		end
		return value_7
	end,
})
codec.s_Unspelled = function(v_7)
	local v_8 = v_7.value
	local size = 1
	if v_8.b ~= nil then
		size += 1
	elseif v_8.a ~= nil then
		size += 2
	end
	return size
end
codec.w_Unspelled = function(buf_15, o_5, v_7)
	local v_9 = v_7.value
	if v_9.b ~= nil then
		buffer.writeu8(buf_15, o_5, 0)
		buffer.writeu8(buf_15, o_5 + 1, v_9.b)
		o_5 += 2
	elseif v_9.a ~= nil then
		buffer.writeu8(buf_15, o_5, 1)
		buffer.writeu8(buf_15, o_5 + 1, v_9.a)
		buffer.writeu8(buf_15, o_5 + 2, v_9.c)
		o_5 += 3
	else
		error("value matches none of the union's members")
	end
	return o_5
end
codec.r_Unspelled = function(buf_15, o_5)
	local tag = buffer.readu8(buf_15, o_5)
	local value_8
	o_5 += 1
	if tag == 0 then
		local b_2 = buffer.readu8(buf_15, o_5)
		value_8 = {
			b = b_2,
		}
		o_5 += 1
	elseif tag == 1 then
		local a_1 = buffer.readu8(buf_15, o_5)
		local c_1 = buffer.readu8(buf_15, o_5 + 1)
		value_8 = {
			a = a_1,
			c = c_1,
		}
		o_5 += 2
	else
		error("malformed payload")
	end
	local value_9 = {
		value = value_8,
	}
	return value_9, o_5
end
local unspelled = Flamework.createSerializer({
	version = 3,
}, {
	version = 3,
	serialize = function(v_10)
		local buf_16 = buffer.create(codec.s_Unspelled(v_10) + 5)
		buffer.writeu8(buf_16, 0, 3)
		buffer.writeu32(buf_16, 1, 1836250549)
		codec.w_Unspelled(buf_16, 5, v_10)
		return buf_16
	end,
	deserialize = function(buf_17)
		if buffer.len(buf_17) < 5 then
			error("[Flamework] this buffer is too short for the header of Unspelled, version 3: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_17, 0) ~= 3 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_17, 0)}, and this serializer of Unspelled reads version 3: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_17, 1) ~= 1836250549 then
			error("[Flamework] Unspelled's layout changed since this buffer was written as version 3; bump the version and keep a reader for the old one")
		end
		local value_10, o_6 = codec.r_Unspelled(buf_17, 5)
		if o_6 ~= buffer.len(buf_17) then
			error("malformed payload")
		end
		return value_10
	end,
})
local unspelledOptional = Flamework.createSerializer({
	serialize = function(v_11)
		local buf_18 = buffer.create(2)
		local v_12 = v_11.value
		if v_12.m ~= nil then
			buffer.writeu8(buf_18, 0, 0)
			buffer.writeu8(buf_18, 1, v_12.m)
		elseif v_12.n ~= nil then
			buffer.writeu8(buf_18, 0, 1)
			buffer.writeu8(buf_18, 1, if v_12.n then 1 else 0)
		else
			error("value matches none of the union's members")
		end
		return buf_18
	end,
	deserialize = function(buf_19)
		local tag_1 = buffer.readu8(buf_19, 0)
		local value_11
		if tag_1 == 0 then
			value_11 = {
				m = buffer.readu8(buf_19, 1),
			}
		elseif tag_1 == 1 then
			value_11 = {
				n = buffer.readu8(buf_19, 1) ~= 0,
			}
		else
			error("malformed payload")
		end
		local value_12 = {
			value = value_11,
		}
		if buffer.len(buf_19) ~= 2 then
			error("malformed payload")
		end
		return value_12
	end,
})
return {
	symbolKeyed = symbolKeyed,
	mappedSymbol = mappedSymbol,
	recordSymbol = recordSymbol,
	privateNames = privateNames,
	nested = nested,
	underscored = underscored,
	unspelled = unspelled,
	unspelledOptional = unspelledOptional,
}
