-- Compiled with roblox-ts v3.0.0
local TS = _G[script]
local Flamework = TS.import(script, TS.getModule(script, "@flamework-experimental", "core").out).Flamework
--[[
	A serializer macro of the project's own: the `serializer` intrinsic outside core.

	@metadata macro
]]
local function serializerOf(meta)
	return meta
end
--[[
	A versioned one, its options read from the linked parameter.

	@metadata macro {@link options intrinsic-serializer-options}
]]
local function versionedOf(options, meta)
	return meta
end
--[[
	Two serializers from one call, in an object the macro emits.

	@metadata macro
]]
local function pairOf(meta)
	return meta
end
local point = serializerOf({
	serialize = function(v)
		local buf = buffer.create(10)
		buffer.writef64(buf, 0, v.x)
		buffer.writeu16(buf, 8, v.y)
		return buf
	end,
	deserialize = function(buf_1)
		local value = {
			x = buffer.readf64(buf_1, 0),
			y = buffer.readu16(buf_1, 8),
		}
		if buffer.len(buf_1) ~= 10 then
			error("malformed payload")
		end
		return value
	end,
})
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
		local b = buffer.readu8(buf_3, o_1)
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
local stored = versionedOf({
	version = 2,
}, {
	version = 2,
	serialize = function(v_1)
		local length = #v_1.name
		local buf_4 = buffer.create(vsize(length) + length + 6)
		buffer.writeu8(buf_4, 0, 2)
		buffer.writeu32(buf_4, 1, 83185237)
		local o_2 = 5
		local text = v_1.name
		local length_1 = #text
		o_2 = vwrite(buf_4, o_2, length_1)
		buffer.writestring(buf_4, o_2, text)
		o_2 += length_1
		buffer.writeu8(buf_4, o_2, v_1.level)
		return buf_4
	end,
	deserialize = function(buf_5)
		if buffer.len(buf_5) < 5 then
			error("[Flamework] this buffer is too short for the header of { name: string; level: Serialization.u8 }, version 2: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_5, 0) ~= 2 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_5, 0)}, and this serializer of \{ name: string; level: Serialization.u8 \} reads version 2: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_5, 1) ~= 83185237 then
			error("[Flamework] { name: string; level: Serialization.u8 }'s layout changed since this buffer was written as version 2; bump the version and keep a reader for the old one")
		end
		local o_3 = 5
		local length_2, o_4 = vread(buf_5, o_3)
		local text_1 = buffer.readstring(buf_5, o_4, length_2)
		o_3 = o_4 + length_2
		local level_1 = buffer.readu8(buf_5, o_3)
		local value_1 = {
			name = text_1,
			level = level_1,
		}
		o_3 += 1
		if o_3 ~= buffer.len(buf_5) then
			error("malformed payload")
		end
		return value_1
	end,
})
local literals = { "off", "on" }
local literalIndex = {
	off = 0,
	on = 1,
}
local both = pairOf({
	first = {
		serialize = function(v_2)
			local buf_6 = buffer.create(10)
			buffer.writef64(buf_6, 0, v_2.x)
			buffer.writeu16(buf_6, 8, v_2.y)
			return buf_6
		end,
		deserialize = function(buf_7)
			local value_2 = {
				x = buffer.readf64(buf_7, 0),
				y = buffer.readu16(buf_7, 8),
			}
			if buffer.len(buf_7) ~= 10 then
				error("malformed payload")
			end
			return value_2
		end,
	},
	second = {
		serialize = function(v_3)
			local buf_8 = buffer.create(1)
			local index = literalIndex[v_3]
			if index == nil then
				error("value is not one of the literals its type allows")
			end
			buffer.writeu8(buf_8, 0, index)
			return buf_8
		end,
		deserialize = function(buf_9)
			local literal = literals[buffer.readu8(buf_9, 0) + 1]
			if literal == nil then
				error("malformed payload")
			end
			if buffer.len(buf_9) ~= 1 then
				error("malformed payload")
			end
			return literal
		end,
	},
})
local core = Flamework.createSerializer({
	serialize = function(v_4)
		local buf_10 = buffer.create(10)
		buffer.writef64(buf_10, 0, v_4.x)
		buffer.writeu16(buf_10, 8, v_4.y)
		return buf_10
	end,
	deserialize = function(buf_11)
		local value_3 = {
			x = buffer.readf64(buf_11, 0),
			y = buffer.readu16(buf_11, 8),
		}
		if buffer.len(buf_11) ~= 10 then
			error("malformed payload")
		end
		return value_3
	end,
})
local function nested()
	-- A serializer inside a function: its helpers go ahead of the function's statement.
	return serializerOf({
		serialize = function(v_5)
			local size = 0
			local n_3 = 0
			for key, entry in v_5 do
				n_3 += 1
				local length_3 = #key
				size += vsize(length_3) + length_3 + 10
			end
			size += vsize(n_3)
			local buf_12 = buffer.create(size)
			local o_5 = 0
			local n_4 = 0
			for _ in v_5 do
				n_4 += 1
			end
			o_5 = vwrite(buf_12, o_5, n_4)
			for key_1, entry_1 in v_5 do
				local length_4 = #key_1
				o_5 = vwrite(buf_12, o_5, length_4)
				buffer.writestring(buf_12, o_5, key_1)
				o_5 += length_4
				buffer.writef64(buf_12, o_5, entry_1.x)
				buffer.writeu16(buf_12, o_5 + 8, entry_1.y)
				o_5 += 10
			end
			return buf_12
		end,
		deserialize = function(buf_13)
			local o_6 = 0
			local count, o_7 = vread(buf_13, o_6)
			o_6 = o_7
			if count * 11 > buffer.len(buf_13) - o_6 then
				error("malformed payload")
			end
			local map = {}
			for _ = 1, count do
				local length_5, o_8 = vread(buf_13, o_6)
				local text_2 = buffer.readstring(buf_13, o_8, length_5)
				o_6 = o_8 + length_5
				local x_1 = buffer.readf64(buf_13, o_6)
				local y_1 = buffer.readu16(buf_13, o_6 + 8)
				local entry_2 = {
					x = x_1,
					y = y_1,
				}
				map[text_2] = entry_2
				o_6 += 10
			end
			if o_6 ~= buffer.len(buf_13) then
				error("malformed payload")
			end
			return map
		end,
	})
end
return {
	nested = nested,
	point = point,
	stored = stored,
	both = both,
	core = core,
}
