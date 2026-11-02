-- Compiled with roblox-ts v3.0.0
local TS = _G[script]
local Events = TS.import(script, script.Parent, "client").Events
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
codec.checkWidth = function(width, value_1, where, unit)
	if unit == nil then
		unit = ""
	end
	local message = `[Flamework] {width} cannot hold {value_1}{unit}, at {where}`
	error(message, 2)
end
-- Past the 64 locals a function may hold, sends are packed in a do block of their own; past the 16 of a file the
-- same. The returned send fits in what is left.
local function many(value)
	local arg = `n{value + 0}`
	local arg_1 = compute()
	local length = #arg
	local buf_2 = buffer.create(vsize(length) + length + 1)
	local o_2 = 0
	local length_1 = #arg
	o_2 = vwrite(buf_2, o_2, length_1)
	buffer.writestring(buf_2, o_2, arg)
	o_2 += length_1
	if not (arg_1 >= 0 and arg_1 <= 255 and arg_1 % 1 == 0) then
		codec.checkWidth("u8", arg_1, "'bravo' [1]")
	end
	buffer.writeu8(buf_2, o_2, arg_1)
	Events.bravo:_fire(buf_2)
	local arg_2 = `n{value + 1}`
	local arg_3 = compute()
	local length_2 = #arg_2
	local buf_3 = buffer.create(vsize(length_2) + length_2 + 1)
	local o_3 = 0
	local length_3 = #arg_2
	o_3 = vwrite(buf_3, o_3, length_3)
	buffer.writestring(buf_3, o_3, arg_2)
	o_3 += length_3
	if not (arg_3 >= 0 and arg_3 <= 255 and arg_3 % 1 == 0) then
		codec.checkWidth("u8", arg_3, "'bravo' [1]")
	end
	buffer.writeu8(buf_3, o_3, arg_3)
	Events.bravo:_fire(buf_3)
	local arg_4 = `n{value + 2}`
	local arg_5 = compute()
	local length_4 = #arg_4
	local buf_4 = buffer.create(vsize(length_4) + length_4 + 1)
	local o_4 = 0
	local length_5 = #arg_4
	o_4 = vwrite(buf_4, o_4, length_5)
	buffer.writestring(buf_4, o_4, arg_4)
	o_4 += length_5
	if not (arg_5 >= 0 and arg_5 <= 255 and arg_5 % 1 == 0) then
		codec.checkWidth("u8", arg_5, "'bravo' [1]")
	end
	buffer.writeu8(buf_4, o_4, arg_5)
	Events.bravo:_fire(buf_4)
	local arg_6 = `n{value + 3}`
	local arg_7 = compute()
	local length_6 = #arg_6
	local buf_5 = buffer.create(vsize(length_6) + length_6 + 1)
	local o_5 = 0
	local length_7 = #arg_6
	o_5 = vwrite(buf_5, o_5, length_7)
	buffer.writestring(buf_5, o_5, arg_6)
	o_5 += length_7
	if not (arg_7 >= 0 and arg_7 <= 255 and arg_7 % 1 == 0) then
		codec.checkWidth("u8", arg_7, "'bravo' [1]")
	end
	buffer.writeu8(buf_5, o_5, arg_7)
	Events.bravo:_fire(buf_5)
	local arg_8 = `n{value + 4}`
	local arg_9 = compute()
	local length_8 = #arg_8
	local buf_6 = buffer.create(vsize(length_8) + length_8 + 1)
	local o_6 = 0
	local length_9 = #arg_8
	o_6 = vwrite(buf_6, o_6, length_9)
	buffer.writestring(buf_6, o_6, arg_8)
	o_6 += length_9
	if not (arg_9 >= 0 and arg_9 <= 255 and arg_9 % 1 == 0) then
		codec.checkWidth("u8", arg_9, "'bravo' [1]")
	end
	buffer.writeu8(buf_6, o_6, arg_9)
	Events.bravo:_fire(buf_6)
	local arg_10 = `n{value + 5}`
	local arg_11 = compute()
	local length_10 = #arg_10
	local buf_7 = buffer.create(vsize(length_10) + length_10 + 1)
	local o_7 = 0
	local length_11 = #arg_10
	o_7 = vwrite(buf_7, o_7, length_11)
	buffer.writestring(buf_7, o_7, arg_10)
	o_7 += length_11
	if not (arg_11 >= 0 and arg_11 <= 255 and arg_11 % 1 == 0) then
		codec.checkWidth("u8", arg_11, "'bravo' [1]")
	end
	buffer.writeu8(buf_7, o_7, arg_11)
	Events.bravo:_fire(buf_7)
	local arg_12 = `n{value + 6}`
	local arg_13 = compute()
	local length_12 = #arg_12
	local buf_8 = buffer.create(vsize(length_12) + length_12 + 1)
	local o_8 = 0
	local length_13 = #arg_12
	o_8 = vwrite(buf_8, o_8, length_13)
	buffer.writestring(buf_8, o_8, arg_12)
	o_8 += length_13
	if not (arg_13 >= 0 and arg_13 <= 255 and arg_13 % 1 == 0) then
		codec.checkWidth("u8", arg_13, "'bravo' [1]")
	end
	buffer.writeu8(buf_8, o_8, arg_13)
	Events.bravo:_fire(buf_8)
	local arg_14 = `n{value + 7}`
	local arg_15 = compute()
	local length_14 = #arg_14
	local buf_9 = buffer.create(vsize(length_14) + length_14 + 1)
	local o_9 = 0
	local length_15 = #arg_14
	o_9 = vwrite(buf_9, o_9, length_15)
	buffer.writestring(buf_9, o_9, arg_14)
	o_9 += length_15
	if not (arg_15 >= 0 and arg_15 <= 255 and arg_15 % 1 == 0) then
		codec.checkWidth("u8", arg_15, "'bravo' [1]")
	end
	buffer.writeu8(buf_9, o_9, arg_15)
	Events.bravo:_fire(buf_9)
	local arg_16 = `n{value + 8}`
	local arg_17 = compute()
	local length_16 = #arg_16
	local buf_10 = buffer.create(vsize(length_16) + length_16 + 1)
	local o_10 = 0
	local length_17 = #arg_16
	o_10 = vwrite(buf_10, o_10, length_17)
	buffer.writestring(buf_10, o_10, arg_16)
	o_10 += length_17
	if not (arg_17 >= 0 and arg_17 <= 255 and arg_17 % 1 == 0) then
		codec.checkWidth("u8", arg_17, "'bravo' [1]")
	end
	buffer.writeu8(buf_10, o_10, arg_17)
	Events.bravo:_fire(buf_10)
	local arg_18 = `n{value + 9}`
	local arg_19 = compute()
	local length_18 = #arg_18
	local buf_11 = buffer.create(vsize(length_18) + length_18 + 1)
	local o_11 = 0
	local length_19 = #arg_18
	o_11 = vwrite(buf_11, o_11, length_19)
	buffer.writestring(buf_11, o_11, arg_18)
	o_11 += length_19
	if not (arg_19 >= 0 and arg_19 <= 255 and arg_19 % 1 == 0) then
		codec.checkWidth("u8", arg_19, "'bravo' [1]")
	end
	buffer.writeu8(buf_11, o_11, arg_19)
	Events.bravo:_fire(buf_11)
	do
		local arg_20 = `n{value + 10}`
		local arg_21 = compute()
		local length_20 = #arg_20
		local buf_12 = buffer.create(vsize(length_20) + length_20 + 1)
		local o_12 = 0
		local length_21 = #arg_20
		o_12 = vwrite(buf_12, o_12, length_21)
		buffer.writestring(buf_12, o_12, arg_20)
		o_12 += length_21
		if not (arg_21 >= 0 and arg_21 <= 255 and arg_21 % 1 == 0) then
			codec.checkWidth("u8", arg_21, "'bravo' [1]")
		end
		buffer.writeu8(buf_12, o_12, arg_21)
		Events.bravo:_fire(buf_12)
	end
	do
		local arg_22 = `n{value + 11}`
		local arg_23 = compute()
		local length_22 = #arg_22
		local buf_13 = buffer.create(vsize(length_22) + length_22 + 1)
		local o_13 = 0
		local length_23 = #arg_22
		o_13 = vwrite(buf_13, o_13, length_23)
		buffer.writestring(buf_13, o_13, arg_22)
		o_13 += length_23
		if not (arg_23 >= 0 and arg_23 <= 255 and arg_23 % 1 == 0) then
			codec.checkWidth("u8", arg_23, "'bravo' [1]")
		end
		buffer.writeu8(buf_13, o_13, arg_23)
		Events.bravo:_fire(buf_13)
	end
	do
		local arg_24 = `n{value + 12}`
		local arg_25 = compute()
		local length_24 = #arg_24
		local buf_14 = buffer.create(vsize(length_24) + length_24 + 1)
		local o_14 = 0
		local length_25 = #arg_24
		o_14 = vwrite(buf_14, o_14, length_25)
		buffer.writestring(buf_14, o_14, arg_24)
		o_14 += length_25
		if not (arg_25 >= 0 and arg_25 <= 255 and arg_25 % 1 == 0) then
			codec.checkWidth("u8", arg_25, "'bravo' [1]")
		end
		buffer.writeu8(buf_14, o_14, arg_25)
		Events.bravo:_fire(buf_14)
	end
	do
		local arg_26 = `n{value + 13}`
		local arg_27 = compute()
		local length_26 = #arg_26
		local buf_15 = buffer.create(vsize(length_26) + length_26 + 1)
		local o_15 = 0
		local length_27 = #arg_26
		o_15 = vwrite(buf_15, o_15, length_27)
		buffer.writestring(buf_15, o_15, arg_26)
		o_15 += length_27
		if not (arg_27 >= 0 and arg_27 <= 255 and arg_27 % 1 == 0) then
			codec.checkWidth("u8", arg_27, "'bravo' [1]")
		end
		buffer.writeu8(buf_15, o_15, arg_27)
		Events.bravo:_fire(buf_15)
	end
	do
		local arg_28 = `n{value + 14}`
		local arg_29 = compute()
		local length_28 = #arg_28
		local buf_16 = buffer.create(vsize(length_28) + length_28 + 1)
		local o_16 = 0
		local length_29 = #arg_28
		o_16 = vwrite(buf_16, o_16, length_29)
		buffer.writestring(buf_16, o_16, arg_28)
		o_16 += length_29
		if not (arg_29 >= 0 and arg_29 <= 255 and arg_29 % 1 == 0) then
			codec.checkWidth("u8", arg_29, "'bravo' [1]")
		end
		buffer.writeu8(buf_16, o_16, arg_29)
		Events.bravo:_fire(buf_16)
	end
	do
		local arg_30 = `n{value + 15}`
		local arg_31 = compute()
		local length_30 = #arg_30
		local buf_17 = buffer.create(vsize(length_30) + length_30 + 1)
		local o_17 = 0
		local length_31 = #arg_30
		o_17 = vwrite(buf_17, o_17, length_31)
		buffer.writestring(buf_17, o_17, arg_30)
		o_17 += length_31
		if not (arg_31 >= 0 and arg_31 <= 255 and arg_31 % 1 == 0) then
			codec.checkWidth("u8", arg_31, "'bravo' [1]")
		end
		buffer.writeu8(buf_17, o_17, arg_31)
		Events.bravo:_fire(buf_17)
	end
	do
		local arg_32 = `n{value + 16}`
		local arg_33 = compute()
		local length_32 = #arg_32
		local buf_18 = buffer.create(vsize(length_32) + length_32 + 1)
		local o_18 = 0
		local length_33 = #arg_32
		o_18 = vwrite(buf_18, o_18, length_33)
		buffer.writestring(buf_18, o_18, arg_32)
		o_18 += length_33
		if not (arg_33 >= 0 and arg_33 <= 255 and arg_33 % 1 == 0) then
			codec.checkWidth("u8", arg_33, "'bravo' [1]")
		end
		buffer.writeu8(buf_18, o_18, arg_33)
		Events.bravo:_fire(buf_18)
	end
	do
		local arg_34 = `n{value + 17}`
		local arg_35 = compute()
		local length_34 = #arg_34
		local buf_19 = buffer.create(vsize(length_34) + length_34 + 1)
		local o_19 = 0
		local length_35 = #arg_34
		o_19 = vwrite(buf_19, o_19, length_35)
		buffer.writestring(buf_19, o_19, arg_34)
		o_19 += length_35
		if not (arg_35 >= 0 and arg_35 <= 255 and arg_35 % 1 == 0) then
			codec.checkWidth("u8", arg_35, "'bravo' [1]")
		end
		buffer.writeu8(buf_19, o_19, arg_35)
		Events.bravo:_fire(buf_19)
	end
	do
		local arg_36 = `n{value + 18}`
		local arg_37 = compute()
		local length_36 = #arg_36
		local buf_20 = buffer.create(vsize(length_36) + length_36 + 1)
		local o_20 = 0
		local length_37 = #arg_36
		o_20 = vwrite(buf_20, o_20, length_37)
		buffer.writestring(buf_20, o_20, arg_36)
		o_20 += length_37
		if not (arg_37 >= 0 and arg_37 <= 255 and arg_37 % 1 == 0) then
			codec.checkWidth("u8", arg_37, "'bravo' [1]")
		end
		buffer.writeu8(buf_20, o_20, arg_37)
		Events.bravo:_fire(buf_20)
	end
	do
		local arg_38 = `n{value + 19}`
		local arg_39 = compute()
		local length_38 = #arg_38
		local buf_21 = buffer.create(vsize(length_38) + length_38 + 1)
		local o_21 = 0
		local length_39 = #arg_38
		o_21 = vwrite(buf_21, o_21, length_39)
		buffer.writestring(buf_21, o_21, arg_38)
		o_21 += length_39
		if not (arg_39 >= 0 and arg_39 <= 255 and arg_39 % 1 == 0) then
			codec.checkWidth("u8", arg_39, "'bravo' [1]")
		end
		buffer.writeu8(buf_21, o_21, arg_39)
		Events.bravo:_fire(buf_21)
	end
	do
		local arg_40 = `n{value + 20}`
		local arg_41 = compute()
		local length_40 = #arg_40
		local buf_22 = buffer.create(vsize(length_40) + length_40 + 1)
		local o_22 = 0
		local length_41 = #arg_40
		o_22 = vwrite(buf_22, o_22, length_41)
		buffer.writestring(buf_22, o_22, arg_40)
		o_22 += length_41
		if not (arg_41 >= 0 and arg_41 <= 255 and arg_41 % 1 == 0) then
			codec.checkWidth("u8", arg_41, "'bravo' [1]")
		end
		buffer.writeu8(buf_22, o_22, arg_41)
		Events.bravo:_fire(buf_22)
	end
	do
		local arg_42 = `n{value + 21}`
		local arg_43 = compute()
		local length_42 = #arg_42
		local buf_23 = buffer.create(vsize(length_42) + length_42 + 1)
		local o_23 = 0
		local length_43 = #arg_42
		o_23 = vwrite(buf_23, o_23, length_43)
		buffer.writestring(buf_23, o_23, arg_42)
		o_23 += length_43
		if not (arg_43 >= 0 and arg_43 <= 255 and arg_43 % 1 == 0) then
			codec.checkWidth("u8", arg_43, "'bravo' [1]")
		end
		buffer.writeu8(buf_23, o_23, arg_43)
		Events.bravo:_fire(buf_23)
	end
	do
		local arg_44 = `n{value + 22}`
		local arg_45 = compute()
		local length_44 = #arg_44
		local buf_24 = buffer.create(vsize(length_44) + length_44 + 1)
		local o_24 = 0
		local length_45 = #arg_44
		o_24 = vwrite(buf_24, o_24, length_45)
		buffer.writestring(buf_24, o_24, arg_44)
		o_24 += length_45
		if not (arg_45 >= 0 and arg_45 <= 255 and arg_45 % 1 == 0) then
			codec.checkWidth("u8", arg_45, "'bravo' [1]")
		end
		buffer.writeu8(buf_24, o_24, arg_45)
		Events.bravo:_fire(buf_24)
	end
	do
		local arg_46 = `n{value + 23}`
		local arg_47 = compute()
		local length_46 = #arg_46
		local buf_25 = buffer.create(vsize(length_46) + length_46 + 1)
		local o_25 = 0
		local length_47 = #arg_46
		o_25 = vwrite(buf_25, o_25, length_47)
		buffer.writestring(buf_25, o_25, arg_46)
		o_25 += length_47
		if not (arg_47 >= 0 and arg_47 <= 255 and arg_47 % 1 == 0) then
			codec.checkWidth("u8", arg_47, "'bravo' [1]")
		end
		buffer.writeu8(buf_25, o_25, arg_47)
		Events.bravo:_fire(buf_25)
	end
	do
		local arg_48 = `n{value + 24}`
		local arg_49 = compute()
		local length_48 = #arg_48
		local buf_26 = buffer.create(vsize(length_48) + length_48 + 1)
		local o_26 = 0
		local length_49 = #arg_48
		o_26 = vwrite(buf_26, o_26, length_49)
		buffer.writestring(buf_26, o_26, arg_48)
		o_26 += length_49
		if not (arg_49 >= 0 and arg_49 <= 255 and arg_49 % 1 == 0) then
			codec.checkWidth("u8", arg_49, "'bravo' [1]")
		end
		buffer.writeu8(buf_26, o_26, arg_49)
		Events.bravo:_fire(buf_26)
	end
	do
		local arg_50 = `n{value + 25}`
		local arg_51 = compute()
		local length_50 = #arg_50
		local buf_27 = buffer.create(vsize(length_50) + length_50 + 1)
		local o_27 = 0
		local length_51 = #arg_50
		o_27 = vwrite(buf_27, o_27, length_51)
		buffer.writestring(buf_27, o_27, arg_50)
		o_27 += length_51
		if not (arg_51 >= 0 and arg_51 <= 255 and arg_51 % 1 == 0) then
			codec.checkWidth("u8", arg_51, "'bravo' [1]")
		end
		buffer.writeu8(buf_27, o_27, arg_51)
		Events.bravo:_fire(buf_27)
	end
	do
		local arg_52 = `n{value + 26}`
		local arg_53 = compute()
		local length_52 = #arg_52
		local buf_28 = buffer.create(vsize(length_52) + length_52 + 1)
		local o_28 = 0
		local length_53 = #arg_52
		o_28 = vwrite(buf_28, o_28, length_53)
		buffer.writestring(buf_28, o_28, arg_52)
		o_28 += length_53
		if not (arg_53 >= 0 and arg_53 <= 255 and arg_53 % 1 == 0) then
			codec.checkWidth("u8", arg_53, "'bravo' [1]")
		end
		buffer.writeu8(buf_28, o_28, arg_53)
		Events.bravo:_fire(buf_28)
	end
	do
		local arg_54 = `n{value + 27}`
		local arg_55 = compute()
		local length_54 = #arg_54
		local buf_29 = buffer.create(vsize(length_54) + length_54 + 1)
		local o_29 = 0
		local length_55 = #arg_54
		o_29 = vwrite(buf_29, o_29, length_55)
		buffer.writestring(buf_29, o_29, arg_54)
		o_29 += length_55
		if not (arg_55 >= 0 and arg_55 <= 255 and arg_55 % 1 == 0) then
			codec.checkWidth("u8", arg_55, "'bravo' [1]")
		end
		buffer.writeu8(buf_29, o_29, arg_55)
		Events.bravo:_fire(buf_29)
	end
	do
		local arg_56 = `n{value + 28}`
		local arg_57 = compute()
		local length_56 = #arg_56
		local buf_30 = buffer.create(vsize(length_56) + length_56 + 1)
		local o_30 = 0
		local length_57 = #arg_56
		o_30 = vwrite(buf_30, o_30, length_57)
		buffer.writestring(buf_30, o_30, arg_56)
		o_30 += length_57
		if not (arg_57 >= 0 and arg_57 <= 255 and arg_57 % 1 == 0) then
			codec.checkWidth("u8", arg_57, "'bravo' [1]")
		end
		buffer.writeu8(buf_30, o_30, arg_57)
		Events.bravo:_fire(buf_30)
	end
	do
		local arg_58 = `n{value + 29}`
		local arg_59 = compute()
		local length_58 = #arg_58
		local buf_31 = buffer.create(vsize(length_58) + length_58 + 1)
		local o_31 = 0
		local length_59 = #arg_58
		o_31 = vwrite(buf_31, o_31, length_59)
		buffer.writestring(buf_31, o_31, arg_58)
		o_31 += length_59
		if not (arg_59 >= 0 and arg_59 <= 255 and arg_59 % 1 == 0) then
			codec.checkWidth("u8", arg_59, "'bravo' [1]")
		end
		buffer.writeu8(buf_31, o_31, arg_59)
		Events.bravo:_fire(buf_31)
	end
	do
		local arg_60 = `n{value + 30}`
		local arg_61 = compute()
		local length_60 = #arg_60
		local buf_32 = buffer.create(vsize(length_60) + length_60 + 1)
		local o_32 = 0
		local length_61 = #arg_60
		o_32 = vwrite(buf_32, o_32, length_61)
		buffer.writestring(buf_32, o_32, arg_60)
		o_32 += length_61
		if not (arg_61 >= 0 and arg_61 <= 255 and arg_61 % 1 == 0) then
			codec.checkWidth("u8", arg_61, "'bravo' [1]")
		end
		buffer.writeu8(buf_32, o_32, arg_61)
		Events.bravo:_fire(buf_32)
	end
	do
		local arg_62 = `n{value + 31}`
		local arg_63 = compute()
		local length_62 = #arg_62
		local buf_33 = buffer.create(vsize(length_62) + length_62 + 1)
		local o_33 = 0
		local length_63 = #arg_62
		o_33 = vwrite(buf_33, o_33, length_63)
		buffer.writestring(buf_33, o_33, arg_62)
		o_33 += length_63
		if not (arg_63 >= 0 and arg_63 <= 255 and arg_63 % 1 == 0) then
			codec.checkWidth("u8", arg_63, "'bravo' [1]")
		end
		buffer.writeu8(buf_33, o_33, arg_63)
		Events.bravo:_fire(buf_33)
	end
	do
		local arg_64 = `n{value + 32}`
		local arg_65 = compute()
		local length_64 = #arg_64
		local buf_34 = buffer.create(vsize(length_64) + length_64 + 1)
		local o_34 = 0
		local length_65 = #arg_64
		o_34 = vwrite(buf_34, o_34, length_65)
		buffer.writestring(buf_34, o_34, arg_64)
		o_34 += length_65
		if not (arg_65 >= 0 and arg_65 <= 255 and arg_65 % 1 == 0) then
			codec.checkWidth("u8", arg_65, "'bravo' [1]")
		end
		buffer.writeu8(buf_34, o_34, arg_65)
		Events.bravo:_fire(buf_34)
	end
	do
		local arg_66 = `n{value + 33}`
		local arg_67 = compute()
		local length_66 = #arg_66
		local buf_35 = buffer.create(vsize(length_66) + length_66 + 1)
		local o_35 = 0
		local length_67 = #arg_66
		o_35 = vwrite(buf_35, o_35, length_67)
		buffer.writestring(buf_35, o_35, arg_66)
		o_35 += length_67
		if not (arg_67 >= 0 and arg_67 <= 255 and arg_67 % 1 == 0) then
			codec.checkWidth("u8", arg_67, "'bravo' [1]")
		end
		buffer.writeu8(buf_35, o_35, arg_67)
		Events.bravo:_fire(buf_35)
	end
	do
		local arg_68 = `n{value + 34}`
		local arg_69 = compute()
		local length_68 = #arg_68
		local buf_36 = buffer.create(vsize(length_68) + length_68 + 1)
		local o_36 = 0
		local length_69 = #arg_68
		o_36 = vwrite(buf_36, o_36, length_69)
		buffer.writestring(buf_36, o_36, arg_68)
		o_36 += length_69
		if not (arg_69 >= 0 and arg_69 <= 255 and arg_69 % 1 == 0) then
			codec.checkWidth("u8", arg_69, "'bravo' [1]")
		end
		buffer.writeu8(buf_36, o_36, arg_69)
		Events.bravo:_fire(buf_36)
	end
	do
		local arg_70 = `n{value + 35}`
		local arg_71 = compute()
		local length_70 = #arg_70
		local buf_37 = buffer.create(vsize(length_70) + length_70 + 1)
		local o_37 = 0
		local length_71 = #arg_70
		o_37 = vwrite(buf_37, o_37, length_71)
		buffer.writestring(buf_37, o_37, arg_70)
		o_37 += length_71
		if not (arg_71 >= 0 and arg_71 <= 255 and arg_71 % 1 == 0) then
			codec.checkWidth("u8", arg_71, "'bravo' [1]")
		end
		buffer.writeu8(buf_37, o_37, arg_71)
		Events.bravo:_fire(buf_37)
	end
	do
		local arg_72 = `n{value + 36}`
		local arg_73 = compute()
		local length_72 = #arg_72
		local buf_38 = buffer.create(vsize(length_72) + length_72 + 1)
		local o_38 = 0
		local length_73 = #arg_72
		o_38 = vwrite(buf_38, o_38, length_73)
		buffer.writestring(buf_38, o_38, arg_72)
		o_38 += length_73
		if not (arg_73 >= 0 and arg_73 <= 255 and arg_73 % 1 == 0) then
			codec.checkWidth("u8", arg_73, "'bravo' [1]")
		end
		buffer.writeu8(buf_38, o_38, arg_73)
		Events.bravo:_fire(buf_38)
	end
	do
		local arg_74 = `n{value + 37}`
		local arg_75 = compute()
		local length_74 = #arg_74
		local buf_39 = buffer.create(vsize(length_74) + length_74 + 1)
		local o_39 = 0
		local length_75 = #arg_74
		o_39 = vwrite(buf_39, o_39, length_75)
		buffer.writestring(buf_39, o_39, arg_74)
		o_39 += length_75
		if not (arg_75 >= 0 and arg_75 <= 255 and arg_75 % 1 == 0) then
			codec.checkWidth("u8", arg_75, "'bravo' [1]")
		end
		buffer.writeu8(buf_39, o_39, arg_75)
		Events.bravo:_fire(buf_39)
	end
	do
		local arg_76 = `n{value + 38}`
		local arg_77 = compute()
		local length_76 = #arg_76
		local buf_40 = buffer.create(vsize(length_76) + length_76 + 1)
		local o_40 = 0
		local length_77 = #arg_76
		o_40 = vwrite(buf_40, o_40, length_77)
		buffer.writestring(buf_40, o_40, arg_76)
		o_40 += length_77
		if not (arg_77 >= 0 and arg_77 <= 255 and arg_77 % 1 == 0) then
			codec.checkWidth("u8", arg_77, "'bravo' [1]")
		end
		buffer.writeu8(buf_40, o_40, arg_77)
		Events.bravo:_fire(buf_40)
	end
	do
		local arg_78 = `n{value + 39}`
		local arg_79 = compute()
		local length_78 = #arg_78
		local buf_41 = buffer.create(vsize(length_78) + length_78 + 1)
		local o_41 = 0
		local length_79 = #arg_78
		o_41 = vwrite(buf_41, o_41, length_79)
		buffer.writestring(buf_41, o_41, arg_78)
		o_41 += length_79
		if not (arg_79 >= 0 and arg_79 <= 255 and arg_79 % 1 == 0) then
			codec.checkWidth("u8", arg_79, "'bravo' [1]")
		end
		buffer.writeu8(buf_41, o_41, arg_79)
		Events.bravo:_fire(buf_41)
	end
	do
		local arg_80 = `n{value + 40}`
		local arg_81 = compute()
		local length_80 = #arg_80
		local buf_42 = buffer.create(vsize(length_80) + length_80 + 1)
		local o_42 = 0
		local length_81 = #arg_80
		o_42 = vwrite(buf_42, o_42, length_81)
		buffer.writestring(buf_42, o_42, arg_80)
		o_42 += length_81
		if not (arg_81 >= 0 and arg_81 <= 255 and arg_81 % 1 == 0) then
			codec.checkWidth("u8", arg_81, "'bravo' [1]")
		end
		buffer.writeu8(buf_42, o_42, arg_81)
		Events.bravo:_fire(buf_42)
	end
	do
		local arg_82 = `n{value + 41}`
		local arg_83 = compute()
		local length_82 = #arg_82
		local buf_43 = buffer.create(vsize(length_82) + length_82 + 1)
		local o_43 = 0
		local length_83 = #arg_82
		o_43 = vwrite(buf_43, o_43, length_83)
		buffer.writestring(buf_43, o_43, arg_82)
		o_43 += length_83
		if not (arg_83 >= 0 and arg_83 <= 255 and arg_83 % 1 == 0) then
			codec.checkWidth("u8", arg_83, "'bravo' [1]")
		end
		buffer.writeu8(buf_43, o_43, arg_83)
		Events.bravo:_fire(buf_43)
	end
	do
		local arg_84 = `n{value + 42}`
		local arg_85 = compute()
		local length_84 = #arg_84
		local buf_44 = buffer.create(vsize(length_84) + length_84 + 1)
		local o_44 = 0
		local length_85 = #arg_84
		o_44 = vwrite(buf_44, o_44, length_85)
		buffer.writestring(buf_44, o_44, arg_84)
		o_44 += length_85
		if not (arg_85 >= 0 and arg_85 <= 255 and arg_85 % 1 == 0) then
			codec.checkWidth("u8", arg_85, "'bravo' [1]")
		end
		buffer.writeu8(buf_44, o_44, arg_85)
		Events.bravo:_fire(buf_44)
	end
	do
		local arg_86 = `n{value + 43}`
		local arg_87 = compute()
		local length_86 = #arg_86
		local buf_45 = buffer.create(vsize(length_86) + length_86 + 1)
		local o_45 = 0
		local length_87 = #arg_86
		o_45 = vwrite(buf_45, o_45, length_87)
		buffer.writestring(buf_45, o_45, arg_86)
		o_45 += length_87
		if not (arg_87 >= 0 and arg_87 <= 255 and arg_87 % 1 == 0) then
			codec.checkWidth("u8", arg_87, "'bravo' [1]")
		end
		buffer.writeu8(buf_45, o_45, arg_87)
		Events.bravo:_fire(buf_45)
	end
	do
		local arg_88 = `n{value + 44}`
		local arg_89 = compute()
		local length_88 = #arg_88
		local buf_46 = buffer.create(vsize(length_88) + length_88 + 1)
		local o_46 = 0
		local length_89 = #arg_88
		o_46 = vwrite(buf_46, o_46, length_89)
		buffer.writestring(buf_46, o_46, arg_88)
		o_46 += length_89
		if not (arg_89 >= 0 and arg_89 <= 255 and arg_89 % 1 == 0) then
			codec.checkWidth("u8", arg_89, "'bravo' [1]")
		end
		buffer.writeu8(buf_46, o_46, arg_89)
		Events.bravo:_fire(buf_46)
	end
	do
		local arg_90 = `n{value + 45}`
		local arg_91 = compute()
		local length_90 = #arg_90
		local buf_47 = buffer.create(vsize(length_90) + length_90 + 1)
		local o_47 = 0
		local length_91 = #arg_90
		o_47 = vwrite(buf_47, o_47, length_91)
		buffer.writestring(buf_47, o_47, arg_90)
		o_47 += length_91
		if not (arg_91 >= 0 and arg_91 <= 255 and arg_91 % 1 == 0) then
			codec.checkWidth("u8", arg_91, "'bravo' [1]")
		end
		buffer.writeu8(buf_47, o_47, arg_91)
		Events.bravo:_fire(buf_47)
	end
	do
		local arg_92 = `n{value + 46}`
		local arg_93 = compute()
		local length_92 = #arg_92
		local buf_48 = buffer.create(vsize(length_92) + length_92 + 1)
		local o_48 = 0
		local length_93 = #arg_92
		o_48 = vwrite(buf_48, o_48, length_93)
		buffer.writestring(buf_48, o_48, arg_92)
		o_48 += length_93
		if not (arg_93 >= 0 and arg_93 <= 255 and arg_93 % 1 == 0) then
			codec.checkWidth("u8", arg_93, "'bravo' [1]")
		end
		buffer.writeu8(buf_48, o_48, arg_93)
		Events.bravo:_fire(buf_48)
	end
	do
		local arg_94 = `n{value + 47}`
		local arg_95 = compute()
		local length_94 = #arg_94
		local buf_49 = buffer.create(vsize(length_94) + length_94 + 1)
		local o_49 = 0
		local length_95 = #arg_94
		o_49 = vwrite(buf_49, o_49, length_95)
		buffer.writestring(buf_49, o_49, arg_94)
		o_49 += length_95
		if not (arg_95 >= 0 and arg_95 <= 255 and arg_95 % 1 == 0) then
			codec.checkWidth("u8", arg_95, "'bravo' [1]")
		end
		buffer.writeu8(buf_49, o_49, arg_95)
		Events.bravo:_fire(buf_49)
	end
	do
		local arg_96 = `n{value + 48}`
		local arg_97 = compute()
		local length_96 = #arg_96
		local buf_50 = buffer.create(vsize(length_96) + length_96 + 1)
		local o_50 = 0
		local length_97 = #arg_96
		o_50 = vwrite(buf_50, o_50, length_97)
		buffer.writestring(buf_50, o_50, arg_96)
		o_50 += length_97
		if not (arg_97 >= 0 and arg_97 <= 255 and arg_97 % 1 == 0) then
			codec.checkWidth("u8", arg_97, "'bravo' [1]")
		end
		buffer.writeu8(buf_50, o_50, arg_97)
		Events.bravo:_fire(buf_50)
	end
	do
		local arg_98 = `n{value + 49}`
		local arg_99 = compute()
		local length_98 = #arg_98
		local buf_51 = buffer.create(vsize(length_98) + length_98 + 1)
		local o_51 = 0
		local length_99 = #arg_98
		o_51 = vwrite(buf_51, o_51, length_99)
		buffer.writestring(buf_51, o_51, arg_98)
		o_51 += length_99
		if not (arg_99 >= 0 and arg_99 <= 255 and arg_99 % 1 == 0) then
			codec.checkWidth("u8", arg_99, "'bravo' [1]")
		end
		buffer.writeu8(buf_51, o_51, arg_99)
		Events.bravo:_fire(buf_51)
	end
	do
		local arg_100 = `n{value + 50}`
		local arg_101 = compute()
		local length_100 = #arg_100
		local buf_52 = buffer.create(vsize(length_100) + length_100 + 1)
		local o_52 = 0
		local length_101 = #arg_100
		o_52 = vwrite(buf_52, o_52, length_101)
		buffer.writestring(buf_52, o_52, arg_100)
		o_52 += length_101
		if not (arg_101 >= 0 and arg_101 <= 255 and arg_101 % 1 == 0) then
			codec.checkWidth("u8", arg_101, "'bravo' [1]")
		end
		buffer.writeu8(buf_52, o_52, arg_101)
		Events.bravo:_fire(buf_52)
	end
	do
		local arg_102 = `n{value + 51}`
		local arg_103 = compute()
		local length_102 = #arg_102
		local buf_53 = buffer.create(vsize(length_102) + length_102 + 1)
		local o_53 = 0
		local length_103 = #arg_102
		o_53 = vwrite(buf_53, o_53, length_103)
		buffer.writestring(buf_53, o_53, arg_102)
		o_53 += length_103
		if not (arg_103 >= 0 and arg_103 <= 255 and arg_103 % 1 == 0) then
			codec.checkWidth("u8", arg_103, "'bravo' [1]")
		end
		buffer.writeu8(buf_53, o_53, arg_103)
		Events.bravo:_fire(buf_53)
	end
	do
		local arg_104 = `n{value + 52}`
		local arg_105 = compute()
		local length_104 = #arg_104
		local buf_54 = buffer.create(vsize(length_104) + length_104 + 1)
		local o_54 = 0
		local length_105 = #arg_104
		o_54 = vwrite(buf_54, o_54, length_105)
		buffer.writestring(buf_54, o_54, arg_104)
		o_54 += length_105
		if not (arg_105 >= 0 and arg_105 <= 255 and arg_105 % 1 == 0) then
			codec.checkWidth("u8", arg_105, "'bravo' [1]")
		end
		buffer.writeu8(buf_54, o_54, arg_105)
		Events.bravo:_fire(buf_54)
	end
	do
		local arg_106 = `n{value + 53}`
		local arg_107 = compute()
		local length_106 = #arg_106
		local buf_55 = buffer.create(vsize(length_106) + length_106 + 1)
		local o_55 = 0
		local length_107 = #arg_106
		o_55 = vwrite(buf_55, o_55, length_107)
		buffer.writestring(buf_55, o_55, arg_106)
		o_55 += length_107
		if not (arg_107 >= 0 and arg_107 <= 255 and arg_107 % 1 == 0) then
			codec.checkWidth("u8", arg_107, "'bravo' [1]")
		end
		buffer.writeu8(buf_55, o_55, arg_107)
		Events.bravo:_fire(buf_55)
	end
	do
		local arg_108 = `n{value + 54}`
		local arg_109 = compute()
		local length_108 = #arg_108
		local buf_56 = buffer.create(vsize(length_108) + length_108 + 1)
		local o_56 = 0
		local length_109 = #arg_108
		o_56 = vwrite(buf_56, o_56, length_109)
		buffer.writestring(buf_56, o_56, arg_108)
		o_56 += length_109
		if not (arg_109 >= 0 and arg_109 <= 255 and arg_109 % 1 == 0) then
			codec.checkWidth("u8", arg_109, "'bravo' [1]")
		end
		buffer.writeu8(buf_56, o_56, arg_109)
		Events.bravo:_fire(buf_56)
	end
	do
		local arg_110 = `n{value + 55}`
		local arg_111 = compute()
		local length_110 = #arg_110
		local buf_57 = buffer.create(vsize(length_110) + length_110 + 1)
		local o_57 = 0
		local length_111 = #arg_110
		o_57 = vwrite(buf_57, o_57, length_111)
		buffer.writestring(buf_57, o_57, arg_110)
		o_57 += length_111
		if not (arg_111 >= 0 and arg_111 <= 255 and arg_111 % 1 == 0) then
			codec.checkWidth("u8", arg_111, "'bravo' [1]")
		end
		buffer.writeu8(buf_57, o_57, arg_111)
		Events.bravo:_fire(buf_57)
	end
	do
		local arg_112 = `n{value + 56}`
		local arg_113 = compute()
		local length_112 = #arg_112
		local buf_58 = buffer.create(vsize(length_112) + length_112 + 1)
		local o_58 = 0
		local length_113 = #arg_112
		o_58 = vwrite(buf_58, o_58, length_113)
		buffer.writestring(buf_58, o_58, arg_112)
		o_58 += length_113
		if not (arg_113 >= 0 and arg_113 <= 255 and arg_113 % 1 == 0) then
			codec.checkWidth("u8", arg_113, "'bravo' [1]")
		end
		buffer.writeu8(buf_58, o_58, arg_113)
		Events.bravo:_fire(buf_58)
	end
	do
		local arg_114 = `n{value + 57}`
		local arg_115 = compute()
		local length_114 = #arg_114
		local buf_59 = buffer.create(vsize(length_114) + length_114 + 1)
		local o_59 = 0
		local length_115 = #arg_114
		o_59 = vwrite(buf_59, o_59, length_115)
		buffer.writestring(buf_59, o_59, arg_114)
		o_59 += length_115
		if not (arg_115 >= 0 and arg_115 <= 255 and arg_115 % 1 == 0) then
			codec.checkWidth("u8", arg_115, "'bravo' [1]")
		end
		buffer.writeu8(buf_59, o_59, arg_115)
		Events.bravo:_fire(buf_59)
	end
	do
		local arg_116 = `n{value + 58}`
		local arg_117 = compute()
		local length_116 = #arg_116
		local buf_60 = buffer.create(vsize(length_116) + length_116 + 1)
		local o_60 = 0
		local length_117 = #arg_116
		o_60 = vwrite(buf_60, o_60, length_117)
		buffer.writestring(buf_60, o_60, arg_116)
		o_60 += length_117
		if not (arg_117 >= 0 and arg_117 <= 255 and arg_117 % 1 == 0) then
			codec.checkWidth("u8", arg_117, "'bravo' [1]")
		end
		buffer.writeu8(buf_60, o_60, arg_117)
		Events.bravo:_fire(buf_60)
	end
	do
		local arg_118 = `n{value + 59}`
		local arg_119 = compute()
		local length_118 = #arg_118
		local buf_61 = buffer.create(vsize(length_118) + length_118 + 1)
		local o_61 = 0
		local length_119 = #arg_118
		o_61 = vwrite(buf_61, o_61, length_119)
		buffer.writestring(buf_61, o_61, arg_118)
		o_61 += length_119
		if not (arg_119 >= 0 and arg_119 <= 255 and arg_119 % 1 == 0) then
			codec.checkWidth("u8", arg_119, "'bravo' [1]")
		end
		buffer.writeu8(buf_61, o_61, arg_119)
		Events.bravo:_fire(buf_61)
	end
	do
		local arg_120 = `n{value + 60}`
		local arg_121 = compute()
		local length_120 = #arg_120
		local buf_62 = buffer.create(vsize(length_120) + length_120 + 1)
		local o_62 = 0
		local length_121 = #arg_120
		o_62 = vwrite(buf_62, o_62, length_121)
		buffer.writestring(buf_62, o_62, arg_120)
		o_62 += length_121
		if not (arg_121 >= 0 and arg_121 <= 255 and arg_121 % 1 == 0) then
			codec.checkWidth("u8", arg_121, "'bravo' [1]")
		end
		buffer.writeu8(buf_62, o_62, arg_121)
		Events.bravo:_fire(buf_62)
	end
	do
		local arg_122 = `n{value + 61}`
		local arg_123 = compute()
		local length_122 = #arg_122
		local buf_63 = buffer.create(vsize(length_122) + length_122 + 1)
		local o_63 = 0
		local length_123 = #arg_122
		o_63 = vwrite(buf_63, o_63, length_123)
		buffer.writestring(buf_63, o_63, arg_122)
		o_63 += length_123
		if not (arg_123 >= 0 and arg_123 <= 255 and arg_123 % 1 == 0) then
			codec.checkWidth("u8", arg_123, "'bravo' [1]")
		end
		buffer.writeu8(buf_63, o_63, arg_123)
		Events.bravo:_fire(buf_63)
	end
	do
		local arg_124 = `n{value + 62}`
		local arg_125 = compute()
		local length_124 = #arg_124
		local buf_64 = buffer.create(vsize(length_124) + length_124 + 1)
		local o_64 = 0
		local length_125 = #arg_124
		o_64 = vwrite(buf_64, o_64, length_125)
		buffer.writestring(buf_64, o_64, arg_124)
		o_64 += length_125
		if not (arg_125 >= 0 and arg_125 <= 255 and arg_125 % 1 == 0) then
			codec.checkWidth("u8", arg_125, "'bravo' [1]")
		end
		buffer.writeu8(buf_64, o_64, arg_125)
		Events.bravo:_fire(buf_64)
	end
	do
		local arg_126 = `n{value + 63}`
		local arg_127 = compute()
		local length_126 = #arg_126
		local buf_65 = buffer.create(vsize(length_126) + length_126 + 1)
		local o_65 = 0
		local length_127 = #arg_126
		o_65 = vwrite(buf_65, o_65, length_127)
		buffer.writestring(buf_65, o_65, arg_126)
		o_65 += length_127
		if not (arg_127 >= 0 and arg_127 <= 255 and arg_127 % 1 == 0) then
			codec.checkWidth("u8", arg_127, "'bravo' [1]")
		end
		buffer.writeu8(buf_65, o_65, arg_127)
		Events.bravo:_fire(buf_65)
	end
	do
		local arg_128 = `n{value + 64}`
		local arg_129 = compute()
		local length_128 = #arg_128
		local buf_66 = buffer.create(vsize(length_128) + length_128 + 1)
		local o_66 = 0
		local length_129 = #arg_128
		o_66 = vwrite(buf_66, o_66, length_129)
		buffer.writestring(buf_66, o_66, arg_128)
		o_66 += length_129
		if not (arg_129 >= 0 and arg_129 <= 255 and arg_129 % 1 == 0) then
			codec.checkWidth("u8", arg_129, "'bravo' [1]")
		end
		buffer.writeu8(buf_66, o_66, arg_129)
		Events.bravo:_fire(buf_66)
	end
	do
		local arg_130 = `n{value + 65}`
		local arg_131 = compute()
		local length_130 = #arg_130
		local buf_67 = buffer.create(vsize(length_130) + length_130 + 1)
		local o_67 = 0
		local length_131 = #arg_130
		o_67 = vwrite(buf_67, o_67, length_131)
		buffer.writestring(buf_67, o_67, arg_130)
		o_67 += length_131
		if not (arg_131 >= 0 and arg_131 <= 255 and arg_131 % 1 == 0) then
			codec.checkWidth("u8", arg_131, "'bravo' [1]")
		end
		buffer.writeu8(buf_67, o_67, arg_131)
		Events.bravo:_fire(buf_67)
	end
	do
		local arg_132 = `n{value + 66}`
		local arg_133 = compute()
		local length_132 = #arg_132
		local buf_68 = buffer.create(vsize(length_132) + length_132 + 1)
		local o_68 = 0
		local length_133 = #arg_132
		o_68 = vwrite(buf_68, o_68, length_133)
		buffer.writestring(buf_68, o_68, arg_132)
		o_68 += length_133
		if not (arg_133 >= 0 and arg_133 <= 255 and arg_133 % 1 == 0) then
			codec.checkWidth("u8", arg_133, "'bravo' [1]")
		end
		buffer.writeu8(buf_68, o_68, arg_133)
		Events.bravo:_fire(buf_68)
	end
	do
		local arg_134 = `n{value + 67}`
		local arg_135 = compute()
		local length_134 = #arg_134
		local buf_69 = buffer.create(vsize(length_134) + length_134 + 1)
		local o_69 = 0
		local length_135 = #arg_134
		o_69 = vwrite(buf_69, o_69, length_135)
		buffer.writestring(buf_69, o_69, arg_134)
		o_69 += length_135
		if not (arg_135 >= 0 and arg_135 <= 255 and arg_135 % 1 == 0) then
			codec.checkWidth("u8", arg_135, "'bravo' [1]")
		end
		buffer.writeu8(buf_69, o_69, arg_135)
		Events.bravo:_fire(buf_69)
	end
	do
		local arg_136 = `n{value + 68}`
		local arg_137 = compute()
		local length_136 = #arg_136
		local buf_70 = buffer.create(vsize(length_136) + length_136 + 1)
		local o_70 = 0
		local length_137 = #arg_136
		o_70 = vwrite(buf_70, o_70, length_137)
		buffer.writestring(buf_70, o_70, arg_136)
		o_70 += length_137
		if not (arg_137 >= 0 and arg_137 <= 255 and arg_137 % 1 == 0) then
			codec.checkWidth("u8", arg_137, "'bravo' [1]")
		end
		buffer.writeu8(buf_70, o_70, arg_137)
		Events.bravo:_fire(buf_70)
	end
	do
		local arg_138 = `n{value + 69}`
		local arg_139 = compute()
		local length_138 = #arg_138
		local buf_71 = buffer.create(vsize(length_138) + length_138 + 1)
		local o_71 = 0
		local length_139 = #arg_138
		o_71 = vwrite(buf_71, o_71, length_139)
		buffer.writestring(buf_71, o_71, arg_138)
		o_71 += length_139
		if not (arg_139 >= 0 and arg_139 <= 255 and arg_139 % 1 == 0) then
			codec.checkWidth("u8", arg_139, "'bravo' [1]")
		end
		buffer.writeu8(buf_71, o_71, arg_139)
		Events.bravo:_fire(buf_71)
	end
	local arg_140 = {
		x = value,
		y = compute(),
	}
	local buf_72 = buffer.create(17)
	buffer.writef64(buf_72, 0, arg_140.x)
	buffer.writef64(buf_72, 8, arg_140.y)
	buffer.writeu8(buf_72, 16, 0)
	return Events.delta:_fire(buf_72)
end
local arg_141 = compute()
local length_140 = #"top0"
local buf_73 = buffer.create(vsize(length_140) + length_140 + 17)
local o_72 = 0
local text = "top0"
local length_141 = #text
o_72 = vwrite(buf_73, o_72, length_141)
buffer.writestring(buf_73, o_72, text)
o_72 += length_141
buffer.writeu8(buf_73, o_72, 2)
buffer.writef64(buf_73, o_72 + 1, arg_141)
buffer.writef64(buf_73, o_72 + 9, 0)
Events.foxtrot:_fire(buf_73)
local arg_142 = compute()
local length_142 = #"top1"
local buf_74 = buffer.create(vsize(length_142) + length_142 + 17)
local o_73 = 0
local text_1 = "top1"
local length_143 = #text_1
o_73 = vwrite(buf_74, o_73, length_143)
buffer.writestring(buf_74, o_73, text_1)
o_73 += length_143
buffer.writeu8(buf_74, o_73, 2)
buffer.writef64(buf_74, o_73 + 1, arg_142)
buffer.writef64(buf_74, o_73 + 9, 1)
Events.foxtrot:_fire(buf_74)
do
	local arg_143 = compute()
	local length_144 = #"top2"
	local buf_75 = buffer.create(vsize(length_144) + length_144 + 17)
	local o_74 = 0
	local text_2 = "top2"
	local length_145 = #text_2
	o_74 = vwrite(buf_75, o_74, length_145)
	buffer.writestring(buf_75, o_74, text_2)
	o_74 += length_145
	buffer.writeu8(buf_75, o_74, 2)
	buffer.writef64(buf_75, o_74 + 1, arg_143)
	buffer.writef64(buf_75, o_74 + 9, 2)
	Events.foxtrot:_fire(buf_75)
end
do
	local arg_144 = compute()
	local length_146 = #"top3"
	local buf_76 = buffer.create(vsize(length_146) + length_146 + 17)
	local o_75 = 0
	local text_3 = "top3"
	local length_147 = #text_3
	o_75 = vwrite(buf_76, o_75, length_147)
	buffer.writestring(buf_76, o_75, text_3)
	o_75 += length_147
	buffer.writeu8(buf_76, o_75, 2)
	buffer.writef64(buf_76, o_75 + 1, arg_144)
	buffer.writef64(buf_76, o_75 + 9, 3)
	Events.foxtrot:_fire(buf_76)
end
do
	local arg_145 = compute()
	local length_148 = #"top4"
	local buf_77 = buffer.create(vsize(length_148) + length_148 + 17)
	local o_76 = 0
	local text_4 = "top4"
	local length_149 = #text_4
	o_76 = vwrite(buf_77, o_76, length_149)
	buffer.writestring(buf_77, o_76, text_4)
	o_76 += length_149
	buffer.writeu8(buf_77, o_76, 2)
	buffer.writef64(buf_77, o_76 + 1, arg_145)
	buffer.writef64(buf_77, o_76 + 9, 4)
	Events.foxtrot:_fire(buf_77)
end
do
	local arg_146 = compute()
	local length_150 = #"top5"
	local buf_78 = buffer.create(vsize(length_150) + length_150 + 17)
	local o_77 = 0
	local text_5 = "top5"
	local length_151 = #text_5
	o_77 = vwrite(buf_78, o_77, length_151)
	buffer.writestring(buf_78, o_77, text_5)
	o_77 += length_151
	buffer.writeu8(buf_78, o_77, 2)
	buffer.writef64(buf_78, o_77 + 1, arg_146)
	buffer.writef64(buf_78, o_77 + 9, 5)
	Events.foxtrot:_fire(buf_78)
end
do
	local arg_147 = compute()
	local length_152 = #"top6"
	local buf_79 = buffer.create(vsize(length_152) + length_152 + 17)
	local o_78 = 0
	local text_6 = "top6"
	local length_153 = #text_6
	o_78 = vwrite(buf_79, o_78, length_153)
	buffer.writestring(buf_79, o_78, text_6)
	o_78 += length_153
	buffer.writeu8(buf_79, o_78, 2)
	buffer.writef64(buf_79, o_78 + 1, arg_147)
	buffer.writef64(buf_79, o_78 + 9, 6)
	Events.foxtrot:_fire(buf_79)
end
do
	local arg_148 = compute()
	local length_154 = #"top7"
	local buf_80 = buffer.create(vsize(length_154) + length_154 + 17)
	local o_79 = 0
	local text_7 = "top7"
	local length_155 = #text_7
	o_79 = vwrite(buf_80, o_79, length_155)
	buffer.writestring(buf_80, o_79, text_7)
	o_79 += length_155
	buffer.writeu8(buf_80, o_79, 2)
	buffer.writef64(buf_80, o_79 + 1, arg_148)
	buffer.writef64(buf_80, o_79 + 9, 7)
	Events.foxtrot:_fire(buf_80)
end
do
	local arg_149 = compute()
	local length_156 = #"top8"
	local buf_81 = buffer.create(vsize(length_156) + length_156 + 17)
	local o_80 = 0
	local text_8 = "top8"
	local length_157 = #text_8
	o_80 = vwrite(buf_81, o_80, length_157)
	buffer.writestring(buf_81, o_80, text_8)
	o_80 += length_157
	buffer.writeu8(buf_81, o_80, 2)
	buffer.writef64(buf_81, o_80 + 1, arg_149)
	buffer.writef64(buf_81, o_80 + 9, 8)
	Events.foxtrot:_fire(buf_81)
end
do
	local arg_150 = compute()
	local length_158 = #"top9"
	local buf_82 = buffer.create(vsize(length_158) + length_158 + 17)
	local o_81 = 0
	local text_9 = "top9"
	local length_159 = #text_9
	o_81 = vwrite(buf_82, o_81, length_159)
	buffer.writestring(buf_82, o_81, text_9)
	o_81 += length_159
	buffer.writeu8(buf_82, o_81, 2)
	buffer.writef64(buf_82, o_81 + 1, arg_150)
	buffer.writef64(buf_82, o_81 + 9, 9)
	Events.foxtrot:_fire(buf_82)
end
do
	local arg_151 = compute()
	local length_160 = #"top10"
	local buf_83 = buffer.create(vsize(length_160) + length_160 + 17)
	local o_82 = 0
	local text_10 = "top10"
	local length_161 = #text_10
	o_82 = vwrite(buf_83, o_82, length_161)
	buffer.writestring(buf_83, o_82, text_10)
	o_82 += length_161
	buffer.writeu8(buf_83, o_82, 2)
	buffer.writef64(buf_83, o_82 + 1, arg_151)
	buffer.writef64(buf_83, o_82 + 9, 10)
	Events.foxtrot:_fire(buf_83)
end
do
	local arg_152 = compute()
	local length_162 = #"top11"
	local buf_84 = buffer.create(vsize(length_162) + length_162 + 17)
	local o_83 = 0
	local text_11 = "top11"
	local length_163 = #text_11
	o_83 = vwrite(buf_84, o_83, length_163)
	buffer.writestring(buf_84, o_83, text_11)
	o_83 += length_163
	buffer.writeu8(buf_84, o_83, 2)
	buffer.writef64(buf_84, o_83 + 1, arg_152)
	buffer.writef64(buf_84, o_83 + 9, 11)
	Events.foxtrot:_fire(buf_84)
end
return {
	many = many,
}
