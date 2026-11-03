-- Compiled with roblox-ts v3.0.0
local TS = require(game:GetService("ReplicatedStorage"):WaitForChild("rbxts_include"):WaitForChild("RuntimeLib"))
local t = TS.import(script, game:GetService("ReplicatedStorage"), "rbxts_include", "node_modules", "@rbxts", "t", "lib", "ts").t
local Flamework = TS.import(script, game:GetService("ReplicatedStorage"), "rbxts_include", "node_modules", "@flamework-experimental", "core", "out").Flamework
local wrap = TS.import(script, game:GetService("ReplicatedStorage"), "TS", "argdefs").wrap
-- Calls TS5 leaves alone after a serializer was made: theirs must take no names (a callee holding a call, a property
-- holding the macro).
local function getWrap()
	return wrap
end
local holderObj = {
	inner = {
		wrap = wrap,
	},
}
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
codec.s_Node = function(v)
	local array = v.kids
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
codec.w_Node = function(buf_2, o_2, v)
	buffer.writeu8(buf_2, o_2, v.v)
	local array_1 = v.kids
	o_2 = vwrite(buf_2, o_2 + 1, #array_1)
	for i_1 = 1, #array_1 do
		local item_1 = array_1[i_1]
		o_2 = codec.w_Node(buf_2, o_2, item_1)
	end
	return o_2
end
codec.r_Node = function(buf_2, o_2)
	local v_1 = buffer.readu8(buf_2, o_2)
	local count, o_3 = vread(buf_2, o_2 + 1)
	o_2 = o_3
	if count * 2 > buffer.len(buf_2) - o_2 then
		error("malformed payload")
	end
	local array_2 = table.create(count)
	for i_2 = 1, count do
		local value, o_4 = codec.r_Node(buf_2, o_2)
		array_2[i_2] = value
		o_2 = o_4
	end
	local value_1 = {
		v = v_1,
		kids = array_2,
	}
	return value_1, o_2
end
local first = wrap(1, {
	serialize = function(v_2)
		local buf_3 = buffer.create(codec.s_Node(v_2))
		codec.w_Node(buf_3, 0, v_2)
		return buf_3
	end,
	deserialize = function(buf_4)
		local value_2, o_5 = codec.r_Node(buf_4, 0)
		if o_5 ~= buffer.len(buf_4) then
			error("malformed payload")
		end
		return value_2
	end,
})
local c1 = getWrap()(Flamework.createGuard((function()
	local depth = 0
	local memo
	local Node_1
	Node_1 = t.interface({
		v = t.number,
		kids = t.array(function(value_3)
			if depth >= 1000 then
				return false
			end
			local results
			if type(value_3) == "table" then
				results = memo[Node_1]
				if results == nil then
					results = {}
					memo[Node_1] = results
				else
					local known = results[value_3]
					if known ~= nil then
						return known
					end
				end
			end
			depth += 1
			local result = Node_1(value_3)
			depth -= 1
			if results ~= nil then
				results[value_3] = result
			end
			return result
		end),
	})
	return function(value_4)
		local outerDepth = depth
		local outerMemo = memo
		memo = {}
		local ok, result_1 = pcall(Node_1, value_4)
		depth = outerDepth
		memo = outerMemo
		if not ok then
			error(result_1, 0)
		end
		return result_1
	end
end)()))
local c2 = holderObj.inner.wrap(Flamework.createGuard((function()
	local depth_1 = 0
	local memo_1
	local Node_2
	Node_2 = t.interface({
		v = t.number,
		kids = t.array(function(value_5)
			if depth_1 >= 1000 then
				return false
			end
			local results_1
			if type(value_5) == "table" then
				results_1 = memo_1[Node_2]
				if results_1 == nil then
					results_1 = {}
					memo_1[Node_2] = results_1
				else
					local known_1 = results_1[value_5]
					if known_1 ~= nil then
						return known_1
					end
				end
			end
			depth_1 += 1
			local result_2 = Node_2(value_5)
			depth_1 -= 1
			if results_1 ~= nil then
				results_1[value_5] = result_2
			end
			return result_2
		end),
	})
	return function(value_6)
		local outerDepth_1 = depth_1
		local outerMemo_1 = memo_1
		memo_1 = {}
		local ok_1, result_3 = pcall(Node_2, value_6)
		depth_1 = outerDepth_1
		memo_1 = outerMemo_1
		if not ok_1 then
			error(result_3, 0)
		end
		return result_3
	end
end)()))
local after = wrap(Flamework.createGuard((function()
	local depth_2 = 0
	local memo_2
	local Node_3
	Node_3 = t.interface({
		v = t.number,
		kids = t.array(function(value_7)
			if depth_2 >= 1000 then
				return false
			end
			local results_2
			if type(value_7) == "table" then
				results_2 = memo_2[Node_3]
				if results_2 == nil then
					results_2 = {}
					memo_2[Node_3] = results_2
				else
					local known_2 = results_2[value_7]
					if known_2 ~= nil then
						return known_2
					end
				end
			end
			depth_2 += 1
			local result_4 = Node_3(value_7)
			depth_2 -= 1
			if results_2 ~= nil then
				results_2[value_7] = result_4
			end
			return result_4
		end),
	})
	return function(value_8)
		local outerDepth_2 = depth_2
		local outerMemo_2 = memo_2
		memo_2 = {}
		local ok_2, result_5 = pcall(Node_3, value_8)
		depth_2 = outerDepth_2
		memo_2 = outerMemo_2
		if not ok_2 then
			error(result_5, 0)
		end
		return result_5
	end
end)()), {
	serialize = function(v_3)
		local size_1 = 0
		local n_4 = 0
		for key, entry in v_3 do
			n_4 += 1
			local length = #key
			size_1 += vsize(length) + length + 1
		end
		size_1 += vsize(n_4)
		local buf_5 = buffer.create(size_1)
		local o_6 = 0
		local n_5 = 0
		for _ in v_3 do
			n_5 += 1
		end
		o_6 = vwrite(buf_5, o_6, n_5)
		for key_1, entry_1 in v_3 do
			local length_1 = #key_1
			o_6 = vwrite(buf_5, o_6, length_1)
			buffer.writestring(buf_5, o_6, key_1)
			o_6 += length_1
			buffer.writeu8(buf_5, o_6, entry_1)
			o_6 += 1
		end
		return buf_5
	end,
	deserialize = function(buf_6)
		local o_7 = 0
		local count_1, o_8 = vread(buf_6, o_7)
		o_7 = o_8
		if count_1 * 2 > buffer.len(buf_6) - o_7 then
			error("malformed payload")
		end
		local map = {}
		for _ = 1, count_1 do
			local length_2, o_9 = vread(buf_6, o_7)
			local text = buffer.readstring(buf_6, o_9, length_2)
			o_7 = o_9 + length_2
			local entry_2 = buffer.readu8(buf_6, o_7)
			map[text] = entry_2
			o_7 += 1
		end
		if o_7 ~= buffer.len(buf_6) then
			error("malformed payload")
		end
		return map
	end,
})
return {
	first = first,
	c1 = c1,
	c2 = c2,
	after = after,
}
