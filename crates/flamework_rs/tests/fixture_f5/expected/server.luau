-- Compiled with roblox-ts v3.0.0
local TS = _G[script]
local t = TS.import(script, TS.getModule(script, "@rbxts", "t").lib.ts).t
local _network = TS.import(script, script.Parent, "network")
local GlobalEvents = _network.GlobalEvents
local GlobalFunctions = _network.GlobalFunctions
local function record(label)
	return function(processNext)
		return function(player, ...)
			local args = { ... }
			print(label)
			return processNext(player, unpack(args))
		end
	end
end
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
-- The middleware's names stay as written without obfuscation.
local Events = GlobalEvents:createServer({
	middleware = {
		alpha = { record("alpha") },
		juliet = {
			kilo = { record("kilo") },
		},
	},
}, {
	incomingIds = { "alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel", "india", "letters" },
	incoming = {
		alpha = { { t.number }, nil },
		bravo = { { t.string, t.number }, nil },
		charlie = { {}, nil },
		delta = { { t.interface({
			x = t.number,
			y = t.number,
		}), t.optional(t.string) }, nil },
		echo = { {}, t.number },
		foxtrot = { { t.string }, t.number },
		golf = { { t.number }, nil },
		hotel = { { t.number }, nil },
		india = { { t.instanceIsA("Instance"), t.string }, nil },
		letters = { {}, t.string },
	},
	incomingUnreliable = {
		golf = true,
	},
	outgoingIds = { "alpha", "lima", "mike" },
	outgoingUnreliable = {},
	incomingSerializers = {
		alpha = (function(buf)
			local value = { buffer.readf64(buf, 0) }
			if buffer.len(buf) ~= 8 then
				error("malformed payload")
			end
			return value
		end),
		bravo = (function(buf_3)
			local o_2 = 0
			local length, o_3 = vread(buf_3, o_2)
			local text_1 = buffer.readstring(buf_3, o_3, length)
			o_2 = o_3 + length
			local arg = buffer.readu8(buf_3, o_2)
			local value_1 = { text_1, arg }
			o_2 += 1
			if o_2 ~= buffer.len(buf_3) then
				error("malformed payload")
			end
			return value_1
		end),
		delta = (function(buf_4)
			local o_4 = 0
			local x_1 = buffer.readf64(buf_4, o_4)
			local y = buffer.readf64(buf_4, o_4 + 8)
			local arg_1 = {
				x = x_1,
				y = y,
			}
			local present = buffer.readu8(buf_4, o_4 + 16) ~= 0
			local value_2
			o_4 += 17
			if present then
				local length_1, o_5 = vread(buf_4, o_4)
				local text_2 = buffer.readstring(buf_4, o_5, length_1)
				o_4 = o_5 + length_1
				value_2 = text_2
			end
			local value_3 = { arg_1, value_2 }
			if o_4 ~= buffer.len(buf_4) then
				error("malformed payload")
			end
			return value_3
		end),
		echo = (function(buf_5)
			local o_6 = 0
			local count, o_7 = vread(buf_5, o_6)
			o_6 = o_7
			if count * 8 > buffer.len(buf_5) - o_6 then
				error("malformed payload")
			end
			local list = {}
			for i = 1, count do
				list[i] = buffer.readf64(buf_5, o_6)
				o_6 += 8
			end
			if o_6 ~= buffer.len(buf_5) then
				error("malformed payload")
			end
			return list
		end),
		foxtrot = (function(buf_6)
			local o_8 = 0
			local length_2, o_9 = vread(buf_6, o_8)
			local text_3 = buffer.readstring(buf_6, o_9, length_2)
			o_8 = o_9 + length_2
			local count_1, o_10 = vread(buf_6, o_8)
			o_8 = o_10
			if count_1 * 8 > buffer.len(buf_6) - o_8 then
				error("malformed payload")
			end
			local list_1 = { text_3 }
			for i_1 = 1, count_1 do
				list_1[i_1 + 1] = buffer.readf64(buf_6, o_8)
				o_8 += 8
			end
			if o_8 ~= buffer.len(buf_6) then
				error("malformed payload")
			end
			return list_1
		end),
		golf = (function(buf_7)
			local value_4 = { buffer.readf64(buf_7, 0) }
			if buffer.len(buf_7) ~= 8 then
				error("malformed payload")
			end
			return value_4
		end),
		india = (function(buf_8, blobs)
			local o_11 = 0
			local arg_2 = blobs[buffer.readu32(buf_8, o_11)]
			local length_3, o_12 = vread(buf_8, o_11 + 4)
			local text_4 = buffer.readstring(buf_8, o_12, length_3)
			o_11 = o_12 + length_3
			local value_5 = { arg_2, text_4 }
			if o_11 ~= buffer.len(buf_8) then
				error("malformed payload")
			end
			return value_5
		end),
		letters = (function(buf_9)
			local o_13 = 0
			local count_2, o_14 = vread(buf_9, o_13)
			o_13 = o_14
			if count_2 > buffer.len(buf_9) - o_13 then
				error("malformed payload")
			end
			local list_2 = {}
			for i_2 = 1, count_2 do
				local length_4, o_15 = vread(buf_9, o_13)
				local text_5 = buffer.readstring(buf_9, o_15, length_4)
				o_13 = o_15 + length_4
				list_2[i_2] = text_5
			end
			if o_13 ~= buffer.len(buf_9) then
				error("malformed payload")
			end
			return list_2
		end),
	},
	namespaceIds = { "juliet" },
	namespaces = {
		juliet = {
			incomingIds = { "kilo" },
			incoming = {
				kilo = { { t.boolean }, nil },
			},
			incomingUnreliable = {},
			outgoingIds = {},
			outgoingUnreliable = {},
			incomingSerializers = {
				kilo = (function(buf_10)
					local value_6 = { buffer.readu8(buf_10, 0) ~= 0 }
					if buffer.len(buf_10) ~= 1 then
						error("malformed payload")
					end
					return value_6
				end),
			},
			namespaceIds = {},
			namespaces = {},
		},
	},
})
local Functions = GlobalFunctions:createServer({}, {
	incomingIds = { "november", "oscar", "papa", "quebec" },
	incoming = {
		november = { { t.number }, nil },
		oscar = { { t.string }, nil },
		papa = { {}, nil },
		quebec = { { t.interface({
			x = t.number,
			y = t.number,
		}) }, nil },
	},
	outgoingIds = { "romeo" },
	outgoing = {
		romeo = t.string,
	},
	incomingSerializers = {
		november = (function(buf_11)
			local value_7 = { buffer.readf64(buf_11, 0) }
			if buffer.len(buf_11) ~= 8 then
				error("malformed payload")
			end
			return value_7
		end),
		oscar = (function(buf_12)
			local o_16 = 0
			local length_5, o_17 = vread(buf_12, o_16)
			local text_6 = buffer.readstring(buf_12, o_17, length_5)
			o_16 = o_17 + length_5
			local value_8 = { text_6 }
			if o_16 ~= buffer.len(buf_12) then
				error("malformed payload")
			end
			return value_8
		end),
		quebec = (function(buf_13)
			local value_9 = { {
				x = buffer.readf64(buf_13, 0),
				y = buffer.readf64(buf_13, 8),
			} }
			if buffer.len(buf_13) ~= 16 then
				error("malformed payload")
			end
			return value_9
		end),
	},
	outgoingResults = {
		romeo = (function(buf_14)
			local o_18 = 0
			local length_6, o_19 = vread(buf_14, o_18)
			local text_7 = buffer.readstring(buf_14, o_19, length_6)
			o_18 = o_19 + length_6
			local value_10 = { text_7 }
			if o_18 ~= buffer.len(buf_14) then
				error("malformed payload")
			end
			return value_10
		end),
	},
	namespaceIds = {},
	namespaces = {},
})
-- Sends from the server: to one player, to all but one, to everyone, and the member called itself.
local function sendAll(player, others, text)
	local length_7 = #text
	local buf_15 = buffer.create(vsize(length_7) + length_7)
	local o_20 = 0
	local length_8 = #text
	o_20 = vwrite(buf_15, o_20, length_8)
	buffer.writestring(buf_15, o_20, text)
	o_20 += length_8
	Events.lima:_fire(player, buf_15)
	local length_9 = #"skipped"
	local buf_16 = buffer.create(vsize(length_9) + length_9)
	local o_21 = 0
	local text_8 = "skipped"
	local length_10 = #text_8
	o_21 = vwrite(buf_16, o_21, length_10)
	buffer.writestring(buf_16, o_21, text_8)
	o_21 += length_10
	Events.lima:_except(player, buf_16)
	local arg_3 = `{text}!`
	local length_11 = #arg_3
	local buf_17 = buffer.create(vsize(length_11) + length_11)
	local o_22 = 0
	local length_12 = #arg_3
	o_22 = vwrite(buf_17, o_22, length_12)
	buffer.writestring(buf_17, o_22, arg_3)
	o_22 += length_12
	Events.lima:_broadcast(buf_17)
	local arg_4 = string.upper(text)
	local length_13 = #arg_4
	local buf_18 = buffer.create(vsize(length_13) + length_13)
	local o_23 = 0
	local length_14 = #arg_4
	o_23 = vwrite(buf_18, o_23, length_14)
	buffer.writestring(buf_18, o_23, arg_4)
	o_23 += length_14
	Events.lima:_fire(player, buf_18)
	local arg_5 = #others
	local arg_6 = { true, false }
	local n_3 = #arg_6
	local buf_19 = buffer.create(vsize(n_3) + n_3 * 1 + 8)
	local o_24 = 0
	buffer.writef64(buf_19, o_24, arg_5)
	o_24 = vwrite(buf_19, o_24 + 8, #arg_6)
	for i_3 = 1, #arg_6 do
		local item = arg_6[i_3]
		if item == nil then
			error(`[Flamework] the array has no value at 'mike' [1][{i_3 - 1}]`)
		end
		buffer.writeu8(buf_19, o_24, if item then 1 else 0)
		o_24 += 1
	end
	Events.mike:_fire(others, buf_19)
	local buf_20 = buffer.create(8)
	buffer.writef64(buf_20, 0, 1)
	Events.alpha:_broadcast(buf_20)
end
-- Callbacks: an arrow, an identifier, a function whose result carries nothing, and a Promise.
local function callbacks()
	local target = Functions.november
	local callback = function(player, a)
		return a + 1
	end
	target:_setCallback(callback, function(value_11)
		local buf_21 = buffer.create(8)
		buffer.writef64(buf_21, 0, value_11)
		return { buf_21 }
	end)
	local answer = function(player, name)
		return TS.Promise.resolve(`hi {name}`)
	end
	local target_1 = Functions.oscar
	local callback_1 = answer
	target_1:_setCallback(callback_1, function(value_12)
		local length_15 = #value_12
		local buf_22 = buffer.create(vsize(length_15) + length_15)
		local o_25 = 0
		local length_16 = #value_12
		o_25 = vwrite(buf_22, o_25, length_16)
		buffer.writestring(buf_22, o_25, value_12)
		o_25 += length_16
		return { buf_22 }
	end)
	local target_2 = Functions.papa
	local callback_2 = function() end
	target_2:_setCallback(callback_2, function(value_13)
		return nil
	end)
	local target_3 = Functions.quebec
	local callback_3 = function(player, coord)
		return if coord.x > 0 then coord else nil
	end
	target_3:_setCallback(callback_3, function(value_14)
		local v = value_14
		local buf_23 = buffer.create(if v ~= nil then 17 else 1)
		local o_26 = 0
		local v_1 = value_14
		buffer.writeu8(buf_23, o_26, if v_1 ~= nil then 1 else 0)
		o_26 += 1
		if v_1 ~= nil then
			buffer.writef64(buf_23, o_26, v_1.x)
			buffer.writef64(buf_23, o_26 + 8, v_1.y)
			o_26 += 16
		end
		return { buf_23 }
	end)
end
-- Invoking the client, with and without a timeout.
local invokeClient = TS.async(function(player)
	local buf_24 = buffer.create(8)
	buffer.writef64(buf_24, 0, 5)
	local text = TS.await(Functions.romeo:_invoke(player, buf_24))
	local arg_7 = #text
	local buf_25 = buffer.create(8)
	buffer.writef64(buf_25, 0, arg_7)
	local again = TS.await(Functions.romeo:_invokeWithTimeout(player, 2, buf_25))
	local arg_8 = #again
	local buf_26 = buffer.create(8)
	buffer.writef64(buf_26, 0, arg_8)
	return Functions.romeo:_invoke(player, buf_26)
end)
return {
	sendAll = sendAll,
	callbacks = callbacks,
	invokeClient = invokeClient,
	Events = Events,
	Functions = Functions,
}
