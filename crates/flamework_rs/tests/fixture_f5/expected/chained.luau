-- Compiled with roblox-ts v3.0.0
-- Sends through `?.`, packed inside the compiler's nil check: each operand ahead of a `?.` is read once, into a
-- local, before the arguments, and the send goes through it (JavaScript's order). The TS transformer checks the
-- operands, packs, and then reads the chain again for the send.
local function chained(maybe, value)
	local _result = maybe
	if _result ~= nil then
		local arg = value * 3
		local buf = buffer.create(8)
		buffer.writef64(buf, 0, arg)
		_result = _result.alpha:_fire(buf)
	end
	local _result_1 = maybe
	if _result_1 ~= nil then
		_result_1 = _result_1.charlie:_fire()
	end
end
-- An argument that clears the operand ahead of the `?.` does not change the send (it was read before it).
local function cleared(holder)
	local _result = holder.ev
	if _result ~= nil then
		local arg_1 = (function()
			holder.ev = nil
			return 1
		end)()
		local buf_1 = buffer.create(8)
		buffer.writef64(buf_1, 0, arg_1)
		_result = _result.alpha:_fire(buf_1)
	end
end
local codec = {}
local vsize = function(n)
	return if n < 128 then 1 elseif n < 16384 then 2 elseif n < 2097152 then 3 elseif n < 268435456 then 4 else 5
end
local vwrite = function(buf_3, o, n_1)
	while n_1 >= 128 do
		buffer.writeu8(buf_3, o, n_1 % 128 + 128)
		o += 1
		n_1 = math.floor(n_1 / 128)
	end
	buffer.writeu8(buf_3, o, n_1)
	return o + 1
end
local vread = function(buf_4, o_1)
	local n_2 = 0
	local scale = 1
	while true do
		local b = buffer.readu8(buf_4, o_1)
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
codec.checkWidth = function(width, value_1, where, unit)
	if unit == nil then
		unit = ""
	end
	local message = `[Flamework] {width} cannot hold {value_1}{unit}, at {where}`
	error(message, 2)
end
-- Other shapes: deeper, two `?.`, `?.` before the member called, the member called itself, a key with effects, a
-- call operand, setCallback, a value used, invoke.
local function shapes(holder, store)
	local _result = holder.ev
	if _result ~= nil then
		local arg_2 = compute() > 0
		local buf_2 = buffer.create(1)
		buffer.writeu8(buf_2, 0, if arg_2 then 1 else 0)
		_result = _result.juliet.kilo:_fire(buf_2)
	end
	local _result_1 = holder.inner
	if _result_1 ~= nil then
		_result_1 = _result_1.ev
		if _result_1 ~= nil then
			local arg_3 = compute()
			local length = #"both"
			local buf_5 = buffer.create(vsize(length) + length + 1)
			local o_2 = 0
			local text = "both"
			local length_1 = #text
			o_2 = vwrite(buf_5, o_2, length_1)
			buffer.writestring(buf_5, o_2, text)
			o_2 += length_1
			if not (arg_3 >= 0 and arg_3 <= 255 and arg_3 % 1 == 0) then
				codec.checkWidth("u8", arg_3, "'bravo' [1]")
			end
			buffer.writeu8(buf_5, o_2, arg_3)
			_result_1 = _result_1.bravo:_fire(buf_5)
		end
	end
	local _result_2 = holder.alpha
	if _result_2 ~= nil then
		local arg_4 = compute()
		local buf_6 = buffer.create(8)
		buffer.writef64(buf_6, 0, arg_4)
		_result_2:_fire(buf_6)
	end
	local _result_3 = holder.ev
	if _result_3 ~= nil then
		local arg_5 = compute()
		local buf_7 = buffer.create(8)
		buffer.writef64(buf_7, 0, arg_5)
		_result_3.alpha:_fire(buf_7)
	end
	local _result_4 = store[key()]
	if _result_4 ~= nil then
		local target = _result_4.alpha
		local arg_6 = compute()
		local buf_8 = buffer.create(8)
		buffer.writef64(buf_8, 0, arg_6)
		_result_4 = target:_fire(buf_8)
	end
	local _result_5 = pick()
	if _result_5 ~= nil then
		local target_1 = _result_5.alpha
		local arg_7 = compute()
		local buf_9 = buffer.create(8)
		buffer.writef64(buf_9, 0, arg_7)
		_result_5 = target_1:_fire(buf_9)
	end
	local _result_6 = holder.fns
	if _result_6 ~= nil then
		local target_2 = _result_6.romeo
		local callback = function(value)
			return `{value}`
		end
		_result_6 = target_2:_setCallback(callback, function(value_2)
			local length_2 = #value_2
			local buf_10 = buffer.create(vsize(length_2) + length_2)
			local o_3 = 0
			local length_3 = #value_2
			o_3 = vwrite(buf_10, o_3, length_3)
			buffer.writestring(buf_10, o_3, value_2)
			o_3 += length_3
			return { buf_10 }
		end)
	end
	local _sent = holder.ev
	if _sent ~= nil then
		local arg_8 = compute()
		local buf_11 = buffer.create(8)
		buffer.writef64(buf_11, 0, arg_8)
		_sent = _sent.alpha:_fire(buf_11)
	end
	local sent = _sent
	local _result_7 = holder.fns
	if _result_7 ~= nil then
		local arg_9 = compute()
		local buf_12 = buffer.create(8)
		buffer.writef64(buf_12, 0, arg_9)
		_result_7 = _result_7.november:_invoke(buf_12)
	end
	return { sent, _result_7 }
end
-- At the top of the file.
local _result = top.ev
if _result ~= nil then
	local arg_10 = compute()
	local buf_13 = buffer.create(8)
	buffer.writef64(buf_13, 0, arg_10)
	_result = _result.alpha:_fire(buf_13)
end
return {
	chained = chained,
	cleared = cleared,
	shapes = shapes,
}
