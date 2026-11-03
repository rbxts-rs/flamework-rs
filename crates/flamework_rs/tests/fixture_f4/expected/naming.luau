-- Compiled with roblox-ts v3.0.0
local TS = _G[script]
local t = TS.import(script, TS.getModule(script, "@rbxts", "t").lib.ts).t
local Flamework = TS.import(script, TS.getModule(script, "@flamework-experimental", "core").out).Flamework
-- A guard (F3) ahead of the file's first serializer: its locals are named after the serializers' file-level ones.
local before = Flamework.createGuard((function()
	local depth = 0
	local memo
	local Node_1
	Node_1 = t.interface({
		v = t.number,
		kids = t.array(function(value)
			if depth >= 1000 then
				return false
			end
			local results
			if type(value) == "table" then
				results = memo[Node_1]
				if results == nil then
					results = {}
					memo[Node_1] = results
				else
					local known = results[value]
					if known ~= nil then
						return known
					end
				end
			end
			depth += 1
			local result = Node_1(value)
			depth -= 1
			if results ~= nil then
				results[value] = result
			end
			return result
		end),
	})
	local guard = t.union(t.string, Node_1)
	return function(value_1)
		local outerDepth = depth
		local outerMemo = memo
		memo = {}
		local ok, result_1 = pcall(guard, value_1)
		depth = outerDepth
		memo = outerMemo
		if not ok then
			error(result_1, 0)
		end
		return result_1
	end
end)())
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
local guard_1 = t.map(t.string, t.number)
local guard_2 = t.array(t.number)
local s = Flamework.createSerializer({
	serialize = function(v_1)
		local size = 1
		if guard_2(v_1) then
			local n_3 = #v_1
			size += vsize(n_3) + n_3 * 1
		elseif guard_1(v_1) then
			local size_1 = 0
			local n_4 = 0
			for key, entry in v_1 do
				n_4 += 1
				local length = #key
				size_1 += vsize(length) + length + 1
			end
			size_1 += vsize(n_4)
			size += size_1
		elseif type(v_1) == "table" then
			local size_2 = 0
			local n_5 = 0
			for item in v_1 do
				n_5 += 1
				local length_1 = #item
				size_2 += vsize(length_1) + length_1
			end
			size_2 += vsize(n_5)
			size += size_2
		end
		local buf_2 = buffer.create(size)
		local o_2 = 0
		if guard_2(v_1) then
			buffer.writeu8(buf_2, o_2, 0)
			o_2 = vwrite(buf_2, o_2 + 1, #v_1)
			for i = 1, #v_1 do
				local item_1 = v_1[i]
				if item_1 == nil then
					error(`[Flamework] the array has no value at value[{i - 1}]`)
				end
				buffer.writeu8(buf_2, o_2, item_1)
				o_2 += 1
			end
		elseif guard_1(v_1) then
			buffer.writeu8(buf_2, o_2, 1)
			local n_6 = 0
			for _ in v_1 do
				n_6 += 1
			end
			o_2 = vwrite(buf_2, o_2 + 1, n_6)
			for key_1, entry_1 in v_1 do
				local length_2 = #key_1
				o_2 = vwrite(buf_2, o_2, length_2)
				buffer.writestring(buf_2, o_2, key_1)
				o_2 += length_2
				buffer.writeu8(buf_2, o_2, entry_1)
				o_2 += 1
			end
		elseif type(v_1) == "table" then
			buffer.writeu8(buf_2, o_2, 2)
			local n_7 = 0
			for _ in v_1 do
				n_7 += 1
			end
			o_2 = vwrite(buf_2, o_2 + 1, n_7)
			for item_2 in v_1 do
				local length_3 = #item_2
				o_2 = vwrite(buf_2, o_2, length_3)
				buffer.writestring(buf_2, o_2, item_2)
				o_2 += length_3
			end
		else
			error("value matches none of the union's members")
		end
		return buf_2
	end,
	deserialize = function(buf_3)
		local o_3 = 0
		local tag = buffer.readu8(buf_3, o_3)
		local value_2
		o_3 += 1
		if tag == 0 then
			local count, o_4 = vread(buf_3, o_3)
			o_3 = o_4
			if count > buffer.len(buf_3) - o_3 then
				error("malformed payload")
			end
			local array = table.create(count)
			for i_1 = 1, count do
				array[i_1] = buffer.readu8(buf_3, o_3)
				o_3 += 1
			end
			value_2 = array
		elseif tag == 1 then
			local count_1, o_5 = vread(buf_3, o_3)
			o_3 = o_5
			if count_1 * 2 > buffer.len(buf_3) - o_3 then
				error("malformed payload")
			end
			local map = {}
			for _ = 1, count_1 do
				local length_4, o_6 = vread(buf_3, o_3)
				local text = buffer.readstring(buf_3, o_6, length_4)
				o_3 = o_6 + length_4
				local entry_2 = buffer.readu8(buf_3, o_3)
				map[text] = entry_2
				o_3 += 1
			end
			value_2 = map
		elseif tag == 2 then
			local count_2, o_7 = vread(buf_3, o_3)
			o_3 = o_7
			if count_2 > buffer.len(buf_3) - o_3 then
				error("malformed payload")
			end
			local set = {}
			for _1 = 1, count_2 do
				local length_5, o_8 = vread(buf_3, o_3)
				local text_1 = buffer.readstring(buf_3, o_8, length_5)
				o_3 = o_8 + length_5
				set[text_1] = true
			end
			value_2 = set
		else
			error("malformed payload")
		end
		if o_3 ~= buffer.len(buf_3) then
			error("malformed payload")
		end
		return value_2
	end,
})
return {
	before = before,
	s = s,
}
