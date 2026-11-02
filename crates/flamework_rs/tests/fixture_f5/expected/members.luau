-- Compiled with roblox-ts v3.0.0
local TS = _G[script]
local Events = TS.import(script, script.Parent, "client").Events
local codec = {}
local vsize = function(n)
	return if n < 128 then 1 elseif n < 16384 then 2 elseif n < 2097152 then 3 elseif n < 268435456 then 4 else 5
end
local vwrite = function(buf_2, o, n_1)
	while n_1 >= 128 do
		buffer.writeu8(buf_2, o, n_1 % 128 + 128)
		o += 1
		n_1 = math.floor(n_1 / 128)
	end
	buffer.writeu8(buf_2, o, n_1)
	return o + 1
end
local vread = function(buf_3, o_1)
	local n_2 = 0
	local scale = 1
	while true do
		local b = buffer.readu8(buf_3, o_1)
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
-- Sends in class members. TypeScript's printer named their locals in source order, where roblox-ts compiles the
-- instance properties' initializers first (in the constructor), then the constructor's body, the methods and the
-- static properties.
local Members
do
	Members = setmetatable({}, {
		__tostring = function()
			return "Members"
		end,
	})
	Members.__index = Members
	function Members.new(...)
		local self = setmetatable({}, Members)
		return self:constructor(...) or self
	end
	function Members:constructor(value)
		self.events = Events
		self.arrow = function(value)
			return (function()
				local arg = value * compute()
				local buf = buffer.create(8)
				buffer.writef64(buf, 0, arg)
				return self.events.alpha:_fire(buf)
			end)()
		end
		local arg_1 = value * compute()
		local buf_1 = buffer.create(8)
		buffer.writef64(buf_1, 0, arg_1)
		self.events.alpha:_fire(buf_1)
	end
	function Members:method(value)
		local arg_2 = tostring(value)
		local arg_3 = compute()
		local length = #arg_2
		local buf_4 = buffer.create(vsize(length) + length + 1)
		local o_2 = 0
		local length_1 = #arg_2
		o_2 = vwrite(buf_4, o_2, length_1)
		buffer.writestring(buf_4, o_2, arg_2)
		o_2 += length_1
		if not (arg_3 >= 0 and arg_3 <= 255 and arg_3 % 1 == 0) then
			codec.checkWidth("u8", arg_3, "'bravo' [1]")
		end
		buffer.writeu8(buf_4, o_2, arg_3)
		self.events.bravo:_fire(buf_4)
		local arg_4 = compute()
		local buf_5 = buffer.create(8)
		buffer.writef64(buf_5, 0, arg_4)
		return self.events.alpha:_fire(buf_5)
	end
	Members.first = (function()
		local arg_5 = compute()
		local buf_6 = buffer.create(8)
		buffer.writef64(buf_6, 0, arg_5)
		return Events.alpha:_fire(buf_6)
	end)()
	Members.last = (function()
		local arg_6 = compute()
		local buf_7 = buffer.create(8)
		buffer.writef64(buf_7, 0, arg_6)
		return Events.alpha:_fire(buf_7)
	end)()
end
return {
	Members = Members,
}
