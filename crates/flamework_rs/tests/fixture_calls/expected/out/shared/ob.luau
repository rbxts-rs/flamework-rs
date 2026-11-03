-- Compiled with roblox-ts v3.0.0
local TS = require(game:GetService("ReplicatedStorage"):WaitForChild("rbxts_include"):WaitForChild("RuntimeLib"))
local wrap = TS.import(script, game:GetService("ReplicatedStorage"), "TS", "argdefs").wrap
-- The macro reached through an object's property (`inner = { wrap }`): a plain call in TS5, no serializer.
local holderObj = {
	inner = {
		wrap = wrap,
	},
}
local v = holderObj.inner.wrap(1)
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
local after = wrap(1, {
	serialize = function(v_1)
		local size = 0
		local n_3 = 0
		for key, entry in v_1 do
			n_3 += 1
			local length = #key
			size += vsize(length) + length + 1
		end
		size += vsize(n_3)
		local buf_2 = buffer.create(size)
		local o_2 = 0
		local n_4 = 0
		for _ in v_1 do
			n_4 += 1
		end
		o_2 = vwrite(buf_2, o_2, n_4)
		for key_1, entry_1 in v_1 do
			local length_1 = #key_1
			o_2 = vwrite(buf_2, o_2, length_1)
			buffer.writestring(buf_2, o_2, key_1)
			o_2 += length_1
			buffer.writeu8(buf_2, o_2, entry_1)
			o_2 += 1
		end
		return buf_2
	end,
	deserialize = function(buf_3)
		local o_3 = 0
		local count, o_4 = vread(buf_3, o_3)
		o_3 = o_4
		if count * 2 > buffer.len(buf_3) - o_3 then
			error("malformed payload")
		end
		local map = {}
		for _ = 1, count do
			local length_2, o_5 = vread(buf_3, o_3)
			local text = buffer.readstring(buf_3, o_5, length_2)
			o_3 = o_5 + length_2
			local entry_2 = buffer.readu8(buf_3, o_3)
			map[text] = entry_2
			o_3 += 1
		end
		if o_3 ~= buffer.len(buf_3) then
			error("malformed payload")
		end
		return map
	end,
})
return {
	v = v,
	after = after,
}
