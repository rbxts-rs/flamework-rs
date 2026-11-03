-- Compiled with roblox-ts v3.0.0
local TS = _G[script]
local t = TS.import(script, TS.getModule(script, "@rbxts", "t").lib.ts).t
local _network = TS.import(script, script.Parent, "network")
local GlobalEvents = _network.GlobalEvents
local GlobalFunctions = _network.GlobalFunctions
local vsize = function(n)
	return if n < 128 then 1 elseif n < 16384 then 2 elseif n < 2097152 then 3 elseif n < 268435456 then 4 else 5
end
local vwrite = function(buf_1, o, n_1)
	while n_1 >= 128 do
		buffer.writeu8(buf_1, o, n_1 % 128 + 128)
		o += 1
		n_1 = math.floor(n_1 / 128)
	end
	buffer.writeu8(buf_1, o, n_1)
	return o + 1
end
local vread = function(buf_2, o_1)
	local n_2 = 0
	local scale = 1
	while true do
		local b = buffer.readu8(buf_2, o_1)
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
local Events = GlobalEvents:createClient({}, {
	incomingIds = { "alpha", "lima", "mike" },
	incoming = {
		alpha = { { t.number }, nil },
		lima = { { t.string }, nil },
		mike = { { t.number, t.array(t.boolean) }, nil },
	},
	incomingUnreliable = {},
	outgoingIds = { "alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel", "india", "letters" },
	outgoingUnreliable = {
		golf = true,
	},
	incomingSerializers = {
		alpha = function(buf)
			local value_1 = { buffer.readf64(buf, 0) }
			if buffer.len(buf) ~= 8 then
				error("malformed payload")
			end
			return value_1
		end,
		lima = function(buf_3)
			local o_2 = 0
			local length, o_3 = vread(buf_3, o_2)
			local text_1 = buffer.readstring(buf_3, o_3, length)
			o_2 = o_3 + length
			local value_2 = { text_1 }
			if o_2 ~= buffer.len(buf_3) then
				error("malformed payload")
			end
			return value_2
		end,
		mike = function(buf_4)
			local o_4 = 0
			local arg = buffer.readf64(buf_4, o_4)
			local count, o_5 = vread(buf_4, o_4 + 8)
			o_4 = o_5
			if count > buffer.len(buf_4) - o_4 then
				error("malformed payload")
			end
			local array = table.create(count)
			for i_1 = 1, count do
				array[i_1] = buffer.readu8(buf_4, o_4) ~= 0
				o_4 += 1
			end
			local value_3 = { arg, array }
			if o_4 ~= buffer.len(buf_4) then
				error("malformed payload")
			end
			return value_3
		end,
	},
	namespaceIds = { "juliet" },
	namespaces = {
		juliet = {
			incomingIds = {},
			incoming = {},
			incomingUnreliable = {},
			outgoingIds = { "kilo" },
			outgoingUnreliable = {},
			incomingSerializers = {},
			namespaceIds = {},
			namespaces = {},
		},
	},
})
local Functions = GlobalFunctions:createClient({}, {
	incomingIds = { "romeo" },
	incoming = {
		romeo = { { t.number }, nil },
	},
	outgoingIds = { "november", "oscar", "papa", "quebec" },
	outgoing = {
		november = t.number,
		oscar = t.string,
		papa = t.none,
		quebec = t.optional(t.interface({
			x = t.number,
			y = t.number,
		})),
	},
	incomingSerializers = {
		romeo = function(buf_5)
			local value_4 = { buffer.readf64(buf_5, 0) }
			if buffer.len(buf_5) ~= 8 then
				error("malformed payload")
			end
			return value_4
		end,
	},
	outgoingResults = {
		november = function(buf_6)
			local value_5 = { buffer.readf64(buf_6, 0) }
			if buffer.len(buf_6) ~= 8 then
				error("malformed payload")
			end
			return value_5
		end,
		oscar = function(buf_7)
			local o_6 = 0
			local length_1, o_7 = vread(buf_7, o_6)
			local text_2 = buffer.readstring(buf_7, o_7, length_1)
			o_6 = o_7 + length_1
			local value_6 = { text_2 }
			if o_6 ~= buffer.len(buf_7) then
				error("malformed payload")
			end
			return value_6
		end,
		quebec = function(buf_8)
			local o_8 = 0
			local present = buffer.readu8(buf_8, o_8) ~= 0
			local value_7
			o_8 += 1
			if present then
				local x_1 = buffer.readf64(buf_8, o_8)
				local y_1 = buffer.readf64(buf_8, o_8 + 8)
				value_7 = {
					x = x_1,
					y = y_1,
				}
				o_8 += 16
			end
			local value_8 = { value_7 }
			if o_8 ~= buffer.len(buf_8) then
				error("malformed payload")
			end
			return value_8
		end,
	},
	namespaceIds = {},
	namespaces = {},
})
local codec = {}
codec.checkWidth = function(width, value_9, where, unit)
	if unit == nil then
		unit = ""
	end
	local message = `[Flamework] {width} cannot hold {value_9}{unit}, at {where}`
	error(message, 2)
end
-- Arguments read where they are (identifiers, literals, parameters) and bound (anything else); a target that is
-- more than a plain read is bound first.
local function shapes(value, coord, part)
	local buf_9 = buffer.create(8)
	buffer.writef64(buf_9, 0, value)
	Events.alpha:_fire(buf_9)
	local buf_10 = buffer.create(8)
	buffer.writef64(buf_10, 0, 3)
	Events.alpha:_fire(buf_10)
	local arg_1 = compute()
	local buf_11 = buffer.create(8)
	buffer.writef64(buf_11, 0, arg_1)
	Events.alpha:_fire(buf_11)
	local length_2 = #"name"
	local buf_12 = buffer.create(vsize(length_2) + length_2 + 1)
	local o_9 = 0
	local text_3 = "name"
	local length_3 = #text_3
	o_9 = vwrite(buf_12, o_9, length_3)
	buffer.writestring(buf_12, o_9, text_3)
	o_9 += length_3
	buffer.writeu8(buf_12, o_9, 7)
	Events.bravo:_fire(buf_12)
	local arg_2 = `n{value}`
	local length_4 = #arg_2
	local buf_13 = buffer.create(vsize(length_4) + length_4 + 1)
	local o_10 = 0
	local length_5 = #arg_2
	o_10 = vwrite(buf_13, o_10, length_5)
	buffer.writestring(buf_13, o_10, arg_2)
	o_10 += length_5
	if not (value >= 0 and value <= 255 and value % 1 == 0) then
		codec.checkWidth("u8", value, "'bravo' [1]")
	end
	buffer.writeu8(buf_13, o_10, value)
	Events.bravo:_fire(buf_13)
	Events.charlie:_fire()
	local buf_14 = buffer.create(17)
	buffer.writef64(buf_14, 0, coord.x)
	buffer.writef64(buf_14, 8, coord.y)
	buffer.writeu8(buf_14, 16, 0)
	Events.delta:_fire(buf_14)
	local arg_3 = {
		x = value,
		y = 2,
	}
	local v = "tag"
	local size_1 = 1
	if v ~= nil then
		local length_6 = #v
		size_1 += vsize(length_6) + length_6
	end
	local buf_15 = buffer.create(size_1 + 16)
	local o_11 = 0
	buffer.writef64(buf_15, o_11, arg_3.x)
	buffer.writef64(buf_15, o_11 + 8, arg_3.y)
	local v_1 = "tag"
	buffer.writeu8(buf_15, o_11 + 16, if v_1 ~= nil then 1 else 0)
	o_11 += 17
	if v_1 ~= nil then
		local length_7 = #v_1
		o_11 = vwrite(buf_15, o_11, length_7)
		buffer.writestring(buf_15, o_11, v_1)
		o_11 += length_7
	end
	Events.delta:_fire(buf_15)
	local buf_16 = buffer.create(1)
	buffer.writeu8(buf_16, 0, 0)
	Events.echo:_fire(buf_16)
	local buf_17 = buffer.create(25)
	buffer.writeu8(buf_17, 0, 3)
	buffer.writef64(buf_17, 1, 1)
	buffer.writef64(buf_17, 9, 2)
	buffer.writef64(buf_17, 17, value)
	Events.echo:_fire(buf_17)
	local arg_4 = compute()
	local length_8 = #"first"
	local buf_18 = buffer.create(vsize(length_8) + length_8 + 17)
	local o_12 = 0
	local text_4 = "first"
	local length_9 = #text_4
	o_12 = vwrite(buf_18, o_12, length_9)
	buffer.writestring(buf_18, o_12, text_4)
	o_12 += length_9
	buffer.writeu8(buf_18, o_12, 2)
	buffer.writef64(buf_18, o_12 + 1, 1)
	buffer.writef64(buf_18, o_12 + 9, arg_4)
	Events.foxtrot:_fire(buf_18)
	local buf_19 = buffer.create(8)
	buffer.writef64(buf_19, 0, value)
	Events.golf:_fire(buf_19)
	Events.hotel:fire(value)
	local length_10 = #"label"
	local buf_20 = buffer.create(vsize(length_10) + length_10 + 4)
	local blobs = {}
	local o_13 = 0
	if part ~= nil then
		table.insert(blobs, part)
		buffer.writeu32(buf_20, o_13, #blobs)
	else
		buffer.writeu32(buf_20, o_13, 0)
	end
	local text_5 = "label"
	local length_11 = #text_5
	o_13 = vwrite(buf_20, o_13 + 4, length_11)
	buffer.writestring(buf_20, o_13, text_5)
	o_13 += length_11
	Events.india:_fire(buf_20, blobs)
	local buf_21 = buffer.create(1)
	buffer.writeu8(buf_21, 0, if true then 1 else 0)
	Events.juliet.kilo:_fire(buf_21)
	local arg_5 = value + 1
	local buf_22 = buffer.create(8)
	buffer.writef64(buf_22, 0, arg_5)
	Events.alpha:_fire(buf_22)
	local target = pick().alpha
	local buf_23 = buffer.create(8)
	buffer.writef64(buf_23, 0, value)
	target:_fire(buf_23)
	local length_12 = #"x"
	local buf_24 = buffer.create(vsize(length_12) + length_12 + 1)
	local o_14 = 0
	local text_6 = "x"
	local length_13 = #text_6
	o_14 = vwrite(buf_24, o_14, length_13)
	buffer.writestring(buf_24, o_14, text_6)
	o_14 += length_13
	buffer.writeu8(buf_24, o_14, 1)
	handlers[1].bravo:_fire(buf_24)
	local alias = Events
	local buf_25 = buffer.create(8)
	buffer.writef64(buf_25, 0, value)
	alias.alpha:_fire(buf_25)
end
-- Spread arguments: the list is gathered first, as roblox-ts gathers an array literal with spreads.
local function spreads(values, more)
	local _array = {}
	local _length = #_array
	table.move(values, 1, #values, _length + 1, _array)
	local args = _array
	local count_1 = #args
	local buf_26 = buffer.create(vsize(count_1) + count_1 * 8)
	local o_15 = 0
	local count_2 = #args
	o_15 = vwrite(buf_26, o_15, count_2)
	for i_2 = 1, count_2 do
		local item_1 = args[i_2]
		if item_1 == nil then
			error(`[Flamework] the argument list has no value at 'echo' [{i_2 - 1}]`)
		end
		buffer.writef64(buf_26, o_15, item_1)
		o_15 += 8
	end
	Events.echo:_fire(buf_26)
	local _array_1 = { 1 }
	local _length_1 = #_array_1
	table.move(values, 1, #values, _length_1 + 1, _array_1)
	local args_1 = _array_1
	local count_3 = #args_1
	local buf_27 = buffer.create(vsize(count_3) + count_3 * 8)
	local o_16 = 0
	local count_4 = #args_1
	o_16 = vwrite(buf_27, o_16, count_4)
	for i_3 = 1, count_4 do
		local item_2 = args_1[i_3]
		if item_2 == nil then
			error(`[Flamework] the argument list has no value at 'echo' [{i_3 - 1}]`)
		end
		buffer.writef64(buf_27, o_16, item_2)
		o_16 += 8
	end
	Events.echo:_fire(buf_27)
	local _array_2 = {}
	local _length_2 = #_array_2
	local _valuesLength = #values
	table.move(values, 1, _valuesLength, _length_2 + 1, _array_2)
	_length_2 += _valuesLength
	_array_2[_length_2 + 1] = 2
	local args_2 = _array_2
	local count_5 = #args_2
	local buf_28 = buffer.create(vsize(count_5) + count_5 * 8)
	local o_17 = 0
	local count_6 = #args_2
	o_17 = vwrite(buf_28, o_17, count_6)
	for i_4 = 1, count_6 do
		local item_3 = args_2[i_4]
		if item_3 == nil then
			error(`[Flamework] the argument list has no value at 'echo' [{i_4 - 1}]`)
		end
		buffer.writef64(buf_28, o_17, item_3)
		o_17 += 8
	end
	Events.echo:_fire(buf_28)
	local _array_3 = {}
	local _length_3 = #_array_3
	local _valuesLength_1 = #values
	table.move(values, 1, _valuesLength_1, _length_3 + 1, _array_3)
	_length_3 += _valuesLength_1
	table.move(more, 1, #more, _length_3 + 1, _array_3)
	local args_3 = _array_3
	local count_7 = #args_3
	local buf_29 = buffer.create(vsize(count_7) + count_7 * 8)
	local o_18 = 0
	local count_8 = #args_3
	o_18 = vwrite(buf_29, o_18, count_8)
	for i_5 = 1, count_8 do
		local item_4 = args_3[i_5]
		if item_4 == nil then
			error(`[Flamework] the argument list has no value at 'echo' [{i_5 - 1}]`)
		end
		buffer.writef64(buf_29, o_18, item_4)
		o_18 += 8
	end
	Events.echo:_fire(buf_29)
	local _array_4 = { "first" }
	local _length_4 = #_array_4
	table.move(values, 1, #values, _length_4 + 1, _array_4)
	local args_4 = _array_4
	local length_14 = #args_4[1]
	local count_9 = math.max(#args_4 - 1, 0)
	local buf_30 = buffer.create(vsize(length_14) + length_14 + (vsize(count_9) + count_9 * 8))
	local o_19 = 0
	local text_7 = args_4[1]
	local length_15 = #text_7
	o_19 = vwrite(buf_30, o_19, length_15)
	buffer.writestring(buf_30, o_19, text_7)
	o_19 += length_15
	local count_10 = math.max(#args_4 - 1, 0)
	o_19 = vwrite(buf_30, o_19, count_10)
	for i_6 = 2, count_10 + 1 do
		local item_5 = args_4[i_6]
		if item_5 == nil then
			error(`[Flamework] the argument list has no value at 'foxtrot' [{i_6 - 1}]`)
		end
		buffer.writef64(buf_30, o_19, item_5)
		o_19 += 8
	end
	Events.foxtrot:_fire(buf_30)
end
-- Where the call runs: ahead of its statement when that runs it exactly when the call runs, in a function otherwise.
local function positions(flag, items)
	local sent = flag and (function()
		local arg_6 = compute()
		local buf_31 = buffer.create(8)
		buffer.writef64(buf_31, 0, arg_6)
		return Events.alpha:_fire(buf_31)
	end)()
	local picked = if #items > 0 then (function()
		local arg_7 = items[1]
		local buf_32 = buffer.create(8)
		buffer.writef64(buf_32, 0, arg_7)
		return Events.alpha:_fire(buf_32)
	end)() else nil
	local list = { #items, (function()
		local arg_8 = #items
		local buf_33 = buffer.create(8)
		buffer.writef64(buf_33, 0, arg_8)
		return Events.alpha:_fire(buf_33)
	end)() }
	local i = 0
	while true do
		local _condition = i < 3
		if _condition then
			(function()
				local buf_34 = buffer.create(8)
				buffer.writef64(buf_34, 0, i)
				return Events.alpha:_fire(buf_34)
			end)()
			_condition = true
		end
		if not _condition then
			break
		end
		i += 1
	end
	for j = 0, 1 do
		local arg_9 = j * 2
		local buf_35 = buffer.create(8)
		buffer.writef64(buf_35, 0, arg_9)
		Events.alpha:_fire(buf_35)
	end
	if flag then
		local arg_10 = compute()
		local buf_36 = buffer.create(8)
		buffer.writef64(buf_36, 0, arg_10)
		Events.alpha:_fire(buf_36)
	end
	local _arg0 = function(item)
		return (function()
			local arg_11 = item * 2
			local buf_37 = buffer.create(8)
			buffer.writef64(buf_37, 0, arg_11)
			return Events.alpha:_fire(buf_37)
		end)()
	end
	for _k, _v in items do
		_arg0(_v, _k - 1, items)
	end
	local text = `{Events.charlie:_fire()}`
	return { sent, picked, list, text }
end
local function returned(value)
	local arg_12 = value * 2
	local buf_38 = buffer.create(8)
	buffer.writef64(buf_38, 0, arg_12)
	return Events.alpha:_fire(buf_38)
end
-- A send inside another send's argument, and inside a callback.
local function nested(value)
	local arg_14 = (function()
		local arg_13 = value + 1
		local buf_39 = buffer.create(8)
		buffer.writef64(buf_39, 0, arg_13)
		Events.alpha:_fire(buf_39)
		return value
	end)()
	local buf_40 = buffer.create(8)
	buffer.writef64(buf_40, 0, arg_14)
	Events.alpha:_fire(buf_40)
	Events.lima:connect(function(text)
		return (function()
			local length_16 = #text
			local buf_41 = buffer.create(vsize(length_16) + length_16 + 1)
			local o_20 = 0
			local length_17 = #text
			o_20 = vwrite(buf_41, o_20, length_17)
			buffer.writestring(buf_41, o_20, text)
			o_20 += length_17
			buffer.writeu8(buf_41, o_20, 1)
			return Events.bravo:_fire(buf_41)
		end)()
	end)
end
-- Invoking the server and answering it.
local functions = TS.async(function(coord)
	local buf_42 = buffer.create(8)
	buffer.writef64(buf_42, 0, 1)
	local sum = TS.await(Functions.november:_invoke(buf_42))
	local arg_15 = `n{sum}`
	local length_18 = #arg_15
	local buf_43 = buffer.create(vsize(length_18) + length_18)
	local o_21 = 0
	local length_19 = #arg_15
	o_21 = vwrite(buf_43, o_21, length_19)
	buffer.writestring(buf_43, o_21, arg_15)
	o_21 += length_19
	local name = TS.await(Functions.oscar:_invokeWithTimeout(5, buf_43))
	TS.await(Functions.papa:_invoke())
	local arg_16 = {
		x = sum,
		y = #name,
	}
	local buf_44 = buffer.create(16)
	buffer.writef64(buf_44, 0, arg_16.x)
	buffer.writef64(buf_44, 8, arg_16.y)
	local back = TS.await(Functions.quebec:_invoke(buf_44))
	local target_1 = Functions.romeo
	local callback = function(value)
		return `{value}`
	end
	target_1:_setCallback(callback, function(value_10)
		local length_20 = #value_10
		local buf_45 = buffer.create(vsize(length_20) + length_20)
		local o_22 = 0
		local length_21 = #value_10
		o_22 = vwrite(buf_45, o_22, length_21)
		buffer.writestring(buf_45, o_22, value_10)
		o_22 += length_21
		return { buf_45 }
	end)
	return back or coord
end)
-- A class method: `this` is a plain read.
local Sender
do
	Sender = setmetatable({}, {
		__tostring = function()
			return "Sender"
		end,
	})
	Sender.__index = Sender
	function Sender.new(...)
		local self = setmetatable({}, Sender)
		return self:constructor(...) or self
	end
	function Sender:constructor()
		self.events = Events
	end
	function Sender:send(value)
		local buf_46 = buffer.create(8)
		buffer.writef64(buf_46, 0, value)
		self.events.alpha:_fire(buf_46)
		local arg_17 = tostring(value)
		local length_22 = #arg_17
		local buf_47 = buffer.create(vsize(length_22) + length_22 + 1)
		local o_23 = 0
		local length_23 = #arg_17
		o_23 = vwrite(buf_47, o_23, length_23)
		buffer.writestring(buf_47, o_23, arg_17)
		o_23 += length_23
		if not (value >= 0 and value <= 255 and value % 1 == 0) then
			codec.checkWidth("u8", value, "'bravo' [1]")
		end
		buffer.writeu8(buf_47, o_23, value)
		self.events.bravo:_fire(buf_47)
	end
end
local buf_48 = buffer.create(8)
buffer.writef64(buf_48, 0, 1)
-- At the top of the file.
Events.alpha:_fire(buf_48)
Events.charlie:_fire()
local arg_18 = compute()
local length_24 = #"top"
local buf_49 = buffer.create(vsize(length_24) + length_24 + 1)
local o_24 = 0
local text_8 = "top"
local length_25 = #text_8
o_24 = vwrite(buf_49, o_24, length_25)
buffer.writestring(buf_49, o_24, text_8)
o_24 += length_25
if not (arg_18 >= 0 and arg_18 <= 255 and arg_18 % 1 == 0) then
	codec.checkWidth("u8", arg_18, "'bravo' [1]")
end
buffer.writeu8(buf_49, o_24, arg_18)
Events.bravo:_fire(buf_49)
return {
	shapes = shapes,
	spreads = spreads,
	positions = positions,
	returned = returned,
	nested = nested,
	functions = functions,
	Events = Events,
	Functions = Functions,
	Sender = Sender,
}
