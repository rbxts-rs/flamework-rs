-- Compiled with roblox-ts v3.0.0
local TS = _G[script]
local t = TS.import(script, TS.getModule(script, "@rbxts", "t").lib.ts).t
local Flamework = TS.import(script, TS.getModule(script, "@flamework-experimental", "core").out).Flamework
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
local guard = (function()
	local depth_1 = 0
	local memo_1
	local Node_2
	Node_2 = t.interface({
		v = t.number,
		kids = t.array(function(value_2)
			if depth_1 >= 1000 then
				return false
			end
			local results_1
			if type(value_2) == "table" then
				results_1 = memo_1[Node_2]
				if results_1 == nil then
					results_1 = {}
					memo_1[Node_2] = results_1
				else
					local known_1 = results_1[value_2]
					if known_1 ~= nil then
						return known_1
					end
				end
			end
			depth_1 += 1
			local result_2 = Node_2(value_2)
			depth_1 -= 1
			if results_1 ~= nil then
				results_1[value_2] = result_2
			end
			return result_2
		end),
	})
	return function(value_3)
		local outerDepth_1 = depth_1
		local outerMemo_1 = memo_1
		memo_1 = {}
		local ok_1, result_3 = pcall(Node_2, value_3)
		depth_1 = outerDepth_1
		memo_1 = outerMemo_1
		if not ok_1 then
			error(result_3, 0)
		end
		return result_3
	end
end)()
codec.s_Node = function(v_1)
	local array = v_1.kids
	local n_3 = #array
	local size = vsize(n_3)
	for i = 1, n_3 do
		local item = array[i]
		if item == nil then
			error(`[Flamework] the array has no value at Node.kids[{i - 1}]`)
		end
		size += codec.s_Node(item)
	end
	return size + 1
end
codec.w_Node = function(buf_2, o_2, v_1)
	buffer.writeu8(buf_2, o_2, v_1.v)
	local array_1 = v_1.kids
	o_2 = vwrite(buf_2, o_2 + 1, #array_1)
	for i_1 = 1, #array_1 do
		local item_1 = array_1[i_1]
		o_2 = codec.w_Node(buf_2, o_2, item_1)
	end
	return o_2
end
codec.r_Node = function(buf_2, o_2)
	local v_2 = buffer.readu8(buf_2, o_2)
	local count, o_3 = vread(buf_2, o_2 + 1)
	o_2 = o_3
	if count * 2 > buffer.len(buf_2) - o_2 then
		error("malformed payload")
	end
	local array_2 = table.create(count)
	for i_2 = 1, count do
		local value_4, o_4 = codec.r_Node(buf_2, o_2)
		array_2[i_2] = value_4
		o_2 = o_4
	end
	local value_5 = {
		v = v_2,
		kids = array_2,
	}
	return value_5, o_2
end
-- A guard (F3) earlier in the root statement than a serializer: what the serializer hoisted printed ahead of the
-- statement, so it is named first.
local both = { Flamework.createGuard((function()
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
	return function(value_1)
		local outerDepth = depth
		local outerMemo = memo
		memo = {}
		local ok, result_1 = pcall(Node_1, value_1)
		depth = outerDepth
		memo = outerMemo
		if not ok then
			error(result_1, 0)
		end
		return result_1
	end
end)()), Flamework.createSerializer({
	serialize = function(v_3)
		local size_1 = 1
		if guard(v_3) then
			size_1 += codec.s_Node(v_3)
		elseif type(v_3) == "table" then
			local size_2 = 0
			local n_4 = 0
			for key, entry in v_3 do
				n_4 += 1
				local length = #key
				size_2 += vsize(length) + length + 1
			end
			size_2 += vsize(n_4)
			size_1 += size_2
		end
		local buf_3 = buffer.create(size_1)
		local o_5 = 0
		if guard(v_3) then
			buffer.writeu8(buf_3, o_5, 0)
			o_5 = codec.w_Node(buf_3, o_5 + 1, v_3)
		elseif type(v_3) == "table" then
			buffer.writeu8(buf_3, o_5, 1)
			local n_5 = 0
			for _ in v_3 do
				n_5 += 1
			end
			o_5 = vwrite(buf_3, o_5 + 1, n_5)
			for key_1, entry_1 in v_3 do
				local length_1 = #key_1
				o_5 = vwrite(buf_3, o_5, length_1)
				buffer.writestring(buf_3, o_5, key_1)
				o_5 += length_1
				buffer.writeu8(buf_3, o_5, entry_1)
				o_5 += 1
			end
		else
			error("value matches none of the union's members")
		end
		return buf_3
	end,
	deserialize = function(buf_4)
		local o_6 = 0
		local tag = buffer.readu8(buf_4, o_6)
		local value_6
		o_6 += 1
		if tag == 0 then
			local value_7, o_7 = codec.r_Node(buf_4, o_6)
			value_6 = value_7
			o_6 = o_7
		elseif tag == 1 then
			local count_1, o_8 = vread(buf_4, o_6)
			o_6 = o_8
			if count_1 * 2 > buffer.len(buf_4) - o_6 then
				error("malformed payload")
			end
			local map = {}
			for _ = 1, count_1 do
				local length_2, o_9 = vread(buf_4, o_6)
				local text = buffer.readstring(buf_4, o_9, length_2)
				o_6 = o_9 + length_2
				local entry_2 = buffer.readu8(buf_4, o_6)
				map[text] = entry_2
				o_6 += 1
			end
			value_6 = map
		else
			error("malformed payload")
		end
		if o_6 ~= buffer.len(buf_4) then
			error("malformed payload")
		end
		return value_6
	end,
}) }
return {
	both = both,
}
