-- Compiled with roblox-ts v3.0.0
local TS = _G[script]
local Flamework = TS.import(script, TS.getModule(script, "@flamework-experimental", "core").out).Flamework
-- An unnamed class expression: TS5 names its symbol `__class`, which sorts before `{` in a union member's key.
local _class
do
	_class = setmetatable({}, {
		__tostring = function()
			return "Anonymous"
		end,
	})
	_class.__index = _class
	function _class.new(...)
		local self = setmetatable({}, _class)
		return self:constructor(...) or self
	end
	function _class:constructor()
		self.k = 0
		self.m = ""
	end
end
local AnonC = _class
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
		local b = buffer.readu8(buf_1, o_1)
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
codec.s_WithObject = function(v)
	local v_1 = v.value
	local size = 1
	if type(v_1) == "table" and v_1.a ~= nil then
		local length = #v_1.a
		size += vsize(length) + length
	elseif true then
		size += 4
	end
	return size
end
codec.w_WithObject = function(buf_2, o_2, v, blobs)
	local v_2 = v.value
	if type(v_2) == "table" and v_2.a ~= nil then
		buffer.writeu8(buf_2, o_2, 1)
		local text = v_2.a
		local length_1 = #text
		o_2 = vwrite(buf_2, o_2 + 1, length_1)
		buffer.writestring(buf_2, o_2, text)
		o_2 += length_1
	elseif true then
		buffer.writeu8(buf_2, o_2, 0)
		if v_2 ~= nil then
			table.insert(blobs, v_2)
			buffer.writeu32(buf_2, o_2 + 1, #blobs)
		else
			buffer.writeu32(buf_2, o_2 + 1, 0)
		end
		o_2 += 5
	else
		error("value matches none of the union's members")
	end
	return o_2
end
codec.r_WithObject = function(buf_2, o_2, blobs)
	local tag = buffer.readu8(buf_2, o_2)
	local value_1
	o_2 += 1
	if tag == 0 then
		value_1 = blobs[buffer.readu32(buf_2, o_2)]
		o_2 += 4
	elseif tag == 1 then
		local length_2, o_3 = vread(buf_2, o_2)
		local text_1 = buffer.readstring(buf_2, o_3, length_2)
		o_2 = o_3 + length_2
		value_1 = {
			a = text_1,
		}
	else
		error("malformed payload")
	end
	local value_2 = {
		value = value_1,
	}
	return value_2, o_2
end
local withObject = Flamework.createSerializer({
	version = 1,
}, {
	version = 1,
	serialize = function(v_3)
		local buf_3 = buffer.create(codec.s_WithObject(v_3) + 5)
		buffer.writeu8(buf_3, 0, 1)
		buffer.writeu32(buf_3, 1, 2736294284)
		local blobs_1 = {}
		codec.w_WithObject(buf_3, 5, v_3, blobs_1)
		return buf_3, blobs_1
	end,
	deserialize = function(buf_4, blobs_2)
		if buffer.len(buf_4) < 5 then
			error("[Flamework] this buffer is too short for the header of WithObject, version 1: no versioned serializer wrote it")
		end
		if buffer.readu8(buf_4, 0) ~= 1 then
			error(`[Flamework] this buffer is version {buffer.readu8(buf_4, 0)}, and this serializer of WithObject reads version 1: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without`)
		end
		if buffer.readu32(buf_4, 1) ~= 2736294284 then
			error("[Flamework] WithObject's layout changed since this buffer was written as version 1; bump the version and keep a reader for the old one")
		end
		local value_3, o_4 = codec.r_WithObject(buf_4, 5, blobs_2)
		if o_4 ~= buffer.len(buf_4) then
			error("malformed payload")
		end
		return value_3
	end,
})
local withClass = Flamework.createSerializer({
	serialize = function(v_4)
		local buf_5 = buffer.create(5)
		local blobs_3 = {}
		local v_5 = v_4.value
		if true then
			buffer.writeu8(buf_5, 0, 0)
			if v_5 ~= nil then
				table.insert(blobs_3, v_5)
				buffer.writeu32(buf_5, 1, #blobs_3)
			else
				buffer.writeu32(buf_5, 1, 0)
			end
		elseif true then
			buffer.writeu8(buf_5, 0, 1)
			if v_5 ~= nil then
				table.insert(blobs_3, v_5)
				buffer.writeu32(buf_5, 1, #blobs_3)
			else
				buffer.writeu32(buf_5, 1, 0)
			end
		else
			error("value matches none of the union's members")
		end
		return buf_5, blobs_3
	end,
	deserialize = function(buf_6, blobs_4)
		local tag_1 = buffer.readu8(buf_6, 0)
		local value_4
		if tag_1 == 0 then
			value_4 = blobs_4[buffer.readu32(buf_6, 1)]
		elseif tag_1 == 1 then
			value_4 = blobs_4[buffer.readu32(buf_6, 1)]
		else
			error("malformed payload")
		end
		local value_5 = {
			value = value_4,
		}
		if buffer.len(buf_6) ~= 5 then
			error("malformed payload")
		end
		return value_5
	end,
})
return {
	AnonC = AnonC,
	withObject = withObject,
	withClass = withClass,
}
