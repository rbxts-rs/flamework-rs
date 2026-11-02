-- Compiled with roblox-ts v3.0.0
local TS = require(game:GetService("ReplicatedStorage"):WaitForChild("rbxts_include"):WaitForChild("RuntimeLib"))
local t = TS.import(script, game:GetService("ReplicatedStorage"), "rbxts_include", "node_modules", "@rbxts", "t", "lib", "ts").t
local Flamework = TS.import(script, game:GetService("ReplicatedStorage"), "rbxts_include", "node_modules", "@flamework-experimental", "core", "out").Flamework
local _argdefs = TS.import(script, game:GetService("ReplicatedStorage"), "TS", "argdefs")
local two = _argdefs.two
local wrap = _argdefs.wrap
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
		local value_4, o_4 = codec.r_Node(buf_2, o_2)
		array_2[i_2] = value_4
		o_2 = o_4
	end
	local value_5 = {
		v = v_1,
		kids = array_2,
	}
	return value_5, o_2
end
-- A guard in a written argument ahead of a generated serializer: TS5 names the guard's locals first (print order).
local w = wrap(Flamework.createGuard((function()
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
end)()), {
	serialize = function(v_2)
		local v_3 = v_2
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
		local v_4 = v_2
		if guard(v_4) then
			buffer.writeu8(buf_3, o_5, 0)
			o_5 = codec.w_Node(buf_3, o_5 + 1, v_4)
		elseif type(v_4) == "table" then
			buffer.writeu8(buf_3, o_5, 1)
			-- ▼ ReadonlyMap.size ▼
			local _size = 0
			for _ in v_4 do
				_size += 1
			end
			-- ▲ ReadonlyMap.size ▲
			local n_5 = _size
			o_5 = vwrite(buf_3, o_5 + 1, n_5)
			for key_1, entry_1 in v_4 do
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
})
codec.s_Map_string_u8 = function(v_6)
	local size_3 = 0
	local n_6 = 0
	for key_2, entry_3 in v_6 do
		n_6 += 1
		local length_3 = #key_2
		size_3 += vsize(length_3) + length_3 + 1
	end
	size_3 += vsize(n_6)
	return size_3
end
codec.w_Map_string_u8 = function(buf_7, o_11, v_6)
	-- ▼ ReadonlyMap.size ▼
	local _size = 0
	for _ in v_6 do
		_size += 1
	end
	-- ▲ ReadonlyMap.size ▲
	local n_7 = _size
	o_11 = vwrite(buf_7, o_11, n_7)
	for key_3, entry_4 in v_6 do
		local length_4 = #key_3
		o_11 = vwrite(buf_7, o_11, length_4)
		buffer.writestring(buf_7, o_11, key_3)
		o_11 += length_4
		buffer.writeu8(buf_7, o_11, entry_4)
		o_11 += 1
	end
	return o_11
end
codec.r_Map_string_u8 = function(buf_7, o_11)
	local count_2, o_12 = vread(buf_7, o_11)
	o_11 = o_12
	if count_2 * 2 > buffer.len(buf_7) - o_11 then
		error("malformed payload")
	end
	local map_1 = {}
	for _1 = 1, count_2 do
		local length_5, o_13 = vread(buf_7, o_11)
		local text_1 = buffer.readstring(buf_7, o_13, length_5)
		o_11 = o_13 + length_5
		local entry_5 = buffer.readu8(buf_7, o_11)
		map_1[text_1] = entry_5
		o_11 += 1
	end
	return map_1, o_11
end
-- Generated, written (holding a guard), generated: named in index order.
local interleaved = two({
	serialize = function(v_5)
		local buf_5 = buffer.create(codec.s_Node(v_5))
		codec.w_Node(buf_5, 0, v_5)
		return buf_5
	end,
	deserialize = function(buf_6)
		local value_8, o_10 = codec.r_Node(buf_6, 0)
		if o_10 ~= buffer.len(buf_6) then
			error("malformed payload")
		end
		return value_8
	end,
}, Flamework.createGuard((function()
	local depth_2 = 0
	local memo_2
	local Node_3
	Node_3 = t.interface({
		v = t.number,
		kids = t.array(function(value_9)
			if depth_2 >= 1000 then
				return false
			end
			local results_2
			if type(value_9) == "table" then
				results_2 = memo_2[Node_3]
				if results_2 == nil then
					results_2 = {}
					memo_2[Node_3] = results_2
				else
					local known_2 = results_2[value_9]
					if known_2 ~= nil then
						return known_2
					end
				end
			end
			depth_2 += 1
			local result_4 = Node_3(value_9)
			depth_2 -= 1
			if results_2 ~= nil then
				results_2[value_9] = result_4
			end
			return result_4
		end),
	})
	return function(value_10)
		local outerDepth_2 = depth_2
		local outerMemo_2 = memo_2
		memo_2 = {}
		local ok_2, result_5 = pcall(Node_3, value_10)
		depth_2 = outerDepth_2
		memo_2 = outerMemo_2
		if not ok_2 then
			error(result_5, 0)
		end
		return result_5
	end
end)()), {
	serialize = function(v_7)
		local buf_8 = buffer.create(codec.s_Map_string_u8(v_7))
		codec.w_Map_string_u8(buf_8, 0, v_7)
		return buf_8
	end,
	deserialize = function(buf_9)
		local value_11, o_14 = codec.r_Map_string_u8(buf_9, 0)
		if o_14 ~= buffer.len(buf_9) then
			error("malformed payload")
		end
		return value_11
	end,
})
local Wrapper
do
	Wrapper = setmetatable({}, {
		__tostring = function()
			return "Wrapper"
		end,
	})
	Wrapper.__index = Wrapper
	function Wrapper.new(...)
		local self = setmetatable({}, Wrapper)
		return self:constructor(...) or self
	end
	function Wrapper:constructor()
	end
	function Wrapper:wrap(inner, meta)
		return { inner, meta }
	end
end
local wrapper = Wrapper.new()
local maybeWrapper = wrapper
codec.s_NodeArray = function(v_8)
	local n_8 = #v_8
	local size_4 = vsize(n_8)
	for i_3 = 1, n_8 do
		local item_2 = v_8[i_3]
		if item_2 == nil then
			error(`[Flamework] the array has no value at (Node[])[{i_3 - 1}]`)
		end
		size_4 += codec.s_Node(item_2)
	end
	return size_4
end
codec.w_NodeArray = function(buf_10, o_15, v_8)
	o_15 = vwrite(buf_10, o_15, #v_8)
	for i_4 = 1, #v_8 do
		local item_3 = v_8[i_4]
		o_15 = codec.w_Node(buf_10, o_15, item_3)
	end
	return o_15
end
codec.r_NodeArray = function(buf_10, o_15)
	local count_3, o_16 = vread(buf_10, o_15)
	o_15 = o_16
	if count_3 * 2 > buffer.len(buf_10) - o_15 then
		error("malformed payload")
	end
	local array_3 = table.create(count_3)
	for i_5 = 1, count_3 do
		local value_14, o_17 = codec.r_Node(buf_10, o_15)
		array_3[i_5] = value_14
		o_15 = o_17
	end
	return array_3, o_15
end
-- The same through a method, `o?.m()` and `super.m()`.
local viaMethod = wrapper:wrap(Flamework.createGuard((function()
	local depth_3 = 0
	local memo_3
	local Node_4
	Node_4 = t.interface({
		v = t.number,
		kids = t.array(function(value_12)
			if depth_3 >= 1000 then
				return false
			end
			local results_3
			if type(value_12) == "table" then
				results_3 = memo_3[Node_4]
				if results_3 == nil then
					results_3 = {}
					memo_3[Node_4] = results_3
				else
					local known_3 = results_3[value_12]
					if known_3 ~= nil then
						return known_3
					end
				end
			end
			depth_3 += 1
			local result_6 = Node_4(value_12)
			depth_3 -= 1
			if results_3 ~= nil then
				results_3[value_12] = result_6
			end
			return result_6
		end),
	})
	return function(value_13)
		local outerDepth_3 = depth_3
		local outerMemo_3 = memo_3
		memo_3 = {}
		local ok_3, result_7 = pcall(Node_4, value_13)
		depth_3 = outerDepth_3
		memo_3 = outerMemo_3
		if not ok_3 then
			error(result_7, 0)
		end
		return result_7
	end
end)()), {
	serialize = function(v_9)
		local buf_11 = buffer.create(codec.s_NodeArray(v_9))
		codec.w_NodeArray(buf_11, 0, v_9)
		return buf_11
	end,
	deserialize = function(buf_12)
		local value_15, o_18 = codec.r_NodeArray(buf_12, 0)
		if o_18 ~= buffer.len(buf_12) then
			error("malformed payload")
		end
		return value_15
	end,
})
local _viaOptional = maybeWrapper
if _viaOptional ~= nil then
	_viaOptional = _viaOptional:wrap(Flamework.createGuard((function()
		local depth_4 = 0
		local memo_4
		local Node_5
		Node_5 = t.interface({
			v = t.number,
			kids = t.array(function(value_16)
				if depth_4 >= 1000 then
					return false
				end
				local results_4
				if type(value_16) == "table" then
					results_4 = memo_4[Node_5]
					if results_4 == nil then
						results_4 = {}
						memo_4[Node_5] = results_4
					else
						local known_4 = results_4[value_16]
						if known_4 ~= nil then
							return known_4
						end
					end
				end
				depth_4 += 1
				local result_8 = Node_5(value_16)
				depth_4 -= 1
				if results_4 ~= nil then
					results_4[value_16] = result_8
				end
				return result_8
			end),
		})
		return function(value_17)
			local outerDepth_4 = depth_4
			local outerMemo_4 = memo_4
			memo_4 = {}
			local ok_4, result_9 = pcall(Node_5, value_17)
			depth_4 = outerDepth_4
			memo_4 = outerMemo_4
			if not ok_4 then
				error(result_9, 0)
			end
			return result_9
		end
	end)()), {
		serialize = function(v_10)
			local size_5 = 0
			local n_9 = 0
			for item_4 in v_10 do
				n_9 += 1
				size_5 += codec.s_Node(item_4)
			end
			size_5 += vsize(n_9)
			local buf_13 = buffer.create(size_5)
			local o_19 = 0
			-- ▼ ReadonlySet.size ▼
			local _size = 0
			for _ in v_10 do
				_size += 1
			end
			-- ▲ ReadonlySet.size ▲
			local n_10 = _size
			o_19 = vwrite(buf_13, o_19, n_10)
			for item_5 in v_10 do
				o_19 = codec.w_Node(buf_13, o_19, item_5)
			end
			return buf_13
		end,
		deserialize = function(buf_14)
			local o_20 = 0
			local count_4, o_21 = vread(buf_14, o_20)
			o_20 = o_21
			if count_4 * 2 > buffer.len(buf_14) - o_20 then
				error("malformed payload")
			end
			local set = {}
			for _2 = 1, count_4 do
				local value_18, o_22 = codec.r_Node(buf_14, o_20)
				set[value_18] = true
				o_20 = o_22
			end
			if o_20 ~= buffer.len(buf_14) then
				error("malformed payload")
			end
			return set
		end,
	})
end
local viaOptional = _viaOptional
local Derived
do
	local super = Wrapper
	Derived = setmetatable({}, {
		__tostring = function()
			return "Derived"
		end,
		__index = super,
	})
	Derived.__index = Derived
	function Derived.new(...)
		local self = setmetatable({}, Derived)
		return self:constructor(...) or self
	end
	function Derived:constructor(...)
		super.constructor(self, ...)
	end
	function Derived:run()
		return super.wrap(self, Flamework.createGuard((function()
			local depth_5 = 0
			local memo_5
			local Node_6
			Node_6 = t.interface({
				v = t.number,
				kids = t.array(function(value_19)
					if depth_5 >= 1000 then
						return false
					end
					local results_5
					if type(value_19) == "table" then
						results_5 = memo_5[Node_6]
						if results_5 == nil then
							results_5 = {}
							memo_5[Node_6] = results_5
						else
							local known_5 = results_5[value_19]
							if known_5 ~= nil then
								return known_5
							end
						end
					end
					depth_5 += 1
					local result_10 = Node_6(value_19)
					depth_5 -= 1
					if results_5 ~= nil then
						results_5[value_19] = result_10
					end
					return result_10
				end),
			})
			return function(value_20)
				local outerDepth_5 = depth_5
				local outerMemo_5 = memo_5
				memo_5 = {}
				local ok_5, result_11 = pcall(Node_6, value_20)
				depth_5 = outerDepth_5
				memo_5 = outerMemo_5
				if not ok_5 then
					error(result_11, 0)
				end
				return result_11
			end
		end)()), {
			serialize = function(v_11)
				local size_6 = 0
				local n_11 = 0
				for key_4, entry_6 in v_11 do
					n_11 += 1
					size_6 += 1 + codec.s_Node(entry_6)
				end
				size_6 += vsize(n_11)
				local buf_15 = buffer.create(size_6)
				local o_23 = 0
				-- ▼ ReadonlyMap.size ▼
				local _size = 0
				for _ in v_11 do
					_size += 1
				end
				-- ▲ ReadonlyMap.size ▲
				local n_12 = _size
				o_23 = vwrite(buf_15, o_23, n_12)
				for key_5, entry_7 in v_11 do
					buffer.writeu8(buf_15, o_23, key_5)
					o_23 = codec.w_Node(buf_15, o_23 + 1, entry_7)
				end
				return buf_15
			end,
			deserialize = function(buf_16)
				local o_24 = 0
				local count_5, o_25 = vread(buf_16, o_24)
				o_24 = o_25
				if count_5 * 3 > buffer.len(buf_16) - o_24 then
					error("malformed payload")
				end
				local map_2 = {}
				for _3 = 1, count_5 do
					local key_6 = buffer.readu8(buf_16, o_24)
					local value_21, o_26 = codec.r_Node(buf_16, o_24 + 1)
					map_2[key_6] = value_21
					o_24 = o_26
				end
				if o_24 ~= buffer.len(buf_16) then
					error("malformed payload")
				end
				return map_2
			end,
		})
	end
end
return {
	w = w,
	interleaved = interleaved,
	viaMethod = viaMethod,
	viaOptional = viaOptional,
	Derived = Derived,
}
