-- Compiled with roblox-ts v3.0.0
local TS = _G[script]
local Events = TS.import(script, script.Parent, "client").Events
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
-- What a spread argument can be, spread as roblox-ts spreads it: an array whose type is generic, a union of array
-- types or read-only (`table.move`), a set's values and a string's characters (a loop each).
local function generic(list)
	do
		local _array = {}
		local _length = #_array
		table.move(list, 1, #list, _length + 1, _array)
		local args = _array
		local count = #args
		local buf_2 = buffer.create(vsize(count) + count * 8)
		local o_2 = 0
		local count_1 = #args
		o_2 = vwrite(buf_2, o_2, count_1)
		for i = 1, count_1 do
			local item = args[i]
			if item == nil then
				error(`[Flamework] the argument list has no value at 'echo' [{i - 1}]`)
			end
			buffer.writef64(buf_2, o_2, item)
			o_2 += 8
		end
		Events.echo:_fire(buf_2)
	end
end
local function union(list)
	do
		local _array = { 1 }
		local _length = #_array
		table.move(list, 1, #list, _length + 1, _array)
		local args_1 = _array
		local count_2 = #args_1
		local buf_3 = buffer.create(vsize(count_2) + count_2 * 8)
		local o_3 = 0
		local count_3 = #args_1
		o_3 = vwrite(buf_3, o_3, count_3)
		for i_1 = 1, count_3 do
			local item_1 = args_1[i_1]
			if item_1 == nil then
				error(`[Flamework] the argument list has no value at 'echo' [{i_1 - 1}]`)
			end
			buffer.writef64(buf_3, o_3, item_1)
			o_3 += 8
		end
		Events.echo:_fire(buf_3)
	end
end
local function readonly(list)
	do
		local _array = {}
		local _length = #_array
		local _listLength = #list
		table.move(list, 1, _listLength, _length + 1, _array)
		_length += _listLength
		_array[_length + 1] = 2
		local args_2 = _array
		local count_4 = #args_2
		local buf_4 = buffer.create(vsize(count_4) + count_4 * 8)
		local o_4 = 0
		local count_5 = #args_2
		o_4 = vwrite(buf_4, o_4, count_5)
		for i_2 = 1, count_5 do
			local item_2 = args_2[i_2]
			if item_2 == nil then
				error(`[Flamework] the argument list has no value at 'echo' [{i_2 - 1}]`)
			end
			buffer.writef64(buf_4, o_4, item_2)
			o_4 += 8
		end
		Events.echo:_fire(buf_4)
	end
end
local function set(values, more)
	do
		local _array = {}
		local _length = #_array
		for _v in values do
			_length += 1
			_array[_length] = _v
		end
		local args_3 = _array
		local count_6 = #args_3
		local buf_5 = buffer.create(vsize(count_6) + count_6 * 8)
		local o_5 = 0
		local count_7 = #args_3
		o_5 = vwrite(buf_5, o_5, count_7)
		for i_3 = 1, count_7 do
			local item_3 = args_3[i_3]
			if item_3 == nil then
				error(`[Flamework] the argument list has no value at 'echo' [{i_3 - 1}]`)
			end
			buffer.writef64(buf_5, o_5, item_3)
			o_5 += 8
		end
		Events.echo:_fire(buf_5)
	end
	do
		local _array = {}
		local _length = #_array
		local _moreLength = #more
		table.move(more, 1, _moreLength, _length + 1, _array)
		_length += _moreLength
		_array[_length + 1] = 3
		_length += 1
		for _v in values do
			_length += 1
			_array[_length] = _v
		end
		_array[_length + 2] = 4
		local args_4 = _array
		local count_8 = #args_4
		local buf_6 = buffer.create(vsize(count_8) + count_8 * 8)
		local o_6 = 0
		local count_9 = #args_4
		o_6 = vwrite(buf_6, o_6, count_9)
		for i_4 = 1, count_9 do
			local item_4 = args_4[i_4]
			if item_4 == nil then
				error(`[Flamework] the argument list has no value at 'echo' [{i_4 - 1}]`)
			end
			buffer.writef64(buf_6, o_6, item_4)
			o_6 += 8
		end
		Events.echo:_fire(buf_6)
	end
end
local function text(word)
	do
		local _array = {}
		local _length = #_array
		for _char in string.gmatch(word, utf8.charpattern) do
			_length += 1
			_array[_length] = _char
		end
		local args_5 = _array
		local count_10 = #args_5
		local size = vsize(count_10)
		for i_5 = 1, count_10 do
			local item_5 = args_5[i_5]
			if item_5 == nil then
				error(`[Flamework] the argument list has no value at 'letters' [{i_5 - 1}]`)
			end
			local length = #item_5
			size += vsize(length) + length
		end
		local buf_7 = buffer.create(size)
		local o_7 = 0
		local count_11 = #args_5
		o_7 = vwrite(buf_7, o_7, count_11)
		for i_6 = 1, count_11 do
			local text_1 = args_5[i_6]
			local length_1 = #text_1
			o_7 = vwrite(buf_7, o_7, length_1)
			buffer.writestring(buf_7, o_7, text_1)
			o_7 += length_1
		end
		Events.letters:_fire(buf_7)
	end
	do
		local _array = { "a" }
		local _length = #_array
		for _char in string.gmatch(word, utf8.charpattern) do
			_length += 1
			_array[_length] = _char
		end
		_array[_length + 1] = "b"
		local args_6 = _array
		local count_12 = #args_6
		local size_1 = vsize(count_12)
		for i_7 = 1, count_12 do
			local item_6 = args_6[i_7]
			if item_6 == nil then
				error(`[Flamework] the argument list has no value at 'letters' [{i_7 - 1}]`)
			end
			local length_2 = #item_6
			size_1 += vsize(length_2) + length_2
		end
		local buf_8 = buffer.create(size_1)
		local o_8 = 0
		local count_13 = #args_6
		o_8 = vwrite(buf_8, o_8, count_13)
		for i_8 = 1, count_13 do
			local text_2 = args_6[i_8]
			local length_3 = #text_2
			o_8 = vwrite(buf_8, o_8, length_3)
			buffer.writestring(buf_8, o_8, text_2)
			o_8 += length_3
		end
		Events.letters:_fire(buf_8)
	end
end
return {
	generic = generic,
	union = union,
	readonly = readonly,
	set = set,
	text = text,
}
