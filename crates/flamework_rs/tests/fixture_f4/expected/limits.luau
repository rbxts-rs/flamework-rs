-- Compiled with roblox-ts v3.0.0
local TS = _G[script]
local Flamework = TS.import(script, TS.getModule(script, "@flamework-experimental", "core").out).Flamework
-- A type with enough variable-size fields to take a function past Luau's limits: built one field at a time.
-- Under the thresholds: built plainly.
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
codec.s_Wide = function(v)
	local length = #(v.f0)
	local length_1 = #(v.f1)
	local length_2 = #(v.f2)
	local length_3 = #(v.f3)
	local length_4 = #(v.f4)
	local length_5 = #(v.f5)
	local length_6 = #(v.f6)
	local length_7 = #(v.f7)
	local length_8 = #(v.f8)
	local length_9 = #(v.f9)
	local length_10 = #(v.f10)
	local length_11 = #(v.f11)
	local length_12 = #(v.f12)
	local length_13 = #(v.f13)
	local length_14 = #(v.f14)
	local length_15 = #(v.f15)
	local length_16 = #(v.f16)
	local length_17 = #(v.f17)
	local length_18 = #(v.f18)
	local length_19 = #(v.f19)
	local length_20 = #(v.f20)
	local length_21 = #(v.f21)
	local length_22 = #(v.f22)
	local length_23 = #(v.f23)
	local length_24 = #(v.f24)
	local length_25 = #(v.f25)
	local length_26 = #(v.f26)
	local length_27 = #(v.f27)
	local length_28 = #(v.f28)
	local length_29 = #(v.f29)
	local length_30 = #(v.f30)
	local length_31 = #(v.f31)
	local length_32 = #(v.f32)
	local length_33 = #(v.f33)
	local length_34 = #(v.f34)
	local length_35 = #(v.f35)
	local length_36 = #(v.f36)
	local length_37 = #(v.f37)
	local length_38 = #(v.f38)
	local length_39 = #(v.f39)
	local length_40 = #(v.f40)
	local length_41 = #(v.f41)
	local length_42 = #(v.f42)
	local length_43 = #(v.f43)
	local length_44 = #(v.f44)
	local length_45 = #(v.f45)
	local length_46 = #(v.f46)
	local length_47 = #(v.f47)
	local length_48 = #(v.f48)
	local length_49 = #(v.f49)
	local length_50 = #(v.f50)
	local length_51 = #(v.f51)
	local length_52 = #(v.f52)
	local length_53 = #(v.f53)
	local length_54 = #(v.f54)
	local length_55 = #(v.f55)
	local length_56 = #(v.f56)
	local length_57 = #(v.f57)
	local length_58 = #(v.f58)
	local length_59 = #(v.f59)
	local length_60 = #(v.f60)
	local length_61 = #(v.f61)
	local length_62 = #(v.f62)
	local length_63 = #(v.f63)
	local length_64 = #(v.f64)
	local length_65 = #(v.f65)
	local length_66 = #(v.f66)
	local length_67 = #(v.f67)
	local length_68 = #(v.f68)
	local length_69 = #(v.f69)
	local length_70 = #(v.f70)
	local length_71 = #(v.f71)
	local length_72 = #(v.f72)
	local length_73 = #(v.f73)
	local length_74 = #(v.f74)
	local length_75 = #(v.f75)
	local length_76 = #(v.f76)
	local length_77 = #(v.f77)
	local length_78 = #(v.f78)
	local length_79 = #(v.f79)
	local length_80 = #(v.f80)
	local length_81 = #(v.f81)
	local length_82 = #(v.f82)
	local length_83 = #(v.f83)
	local length_84 = #(v.f84)
	local length_85 = #(v.f85)
	local length_86 = #(v.f86)
	local length_87 = #(v.f87)
	local length_88 = #(v.f88)
	local length_89 = #(v.f89)
	local length_90 = #(v.f90)
	local length_91 = #(v.f91)
	local length_92 = #(v.f92)
	local length_93 = #(v.f93)
	local length_94 = #(v.f94)
	local length_95 = #(v.f95)
	local length_96 = #(v.f96)
	local length_97 = #(v.f97)
	local length_98 = #(v.f98)
	local length_99 = #(v.f99)
	local length_100 = #(v.f100)
	local length_101 = #(v.f101)
	local length_102 = #(v.f102)
	local length_103 = #(v.f103)
	local length_104 = #(v.f104)
	local length_105 = #(v.f105)
	local length_106 = #(v.f106)
	local length_107 = #(v.f107)
	local length_108 = #(v.f108)
	local length_109 = #(v.f109)
	return vsize(length) + length + (vsize(length_1) + length_1) + (vsize(length_2) + length_2) + (vsize(length_3) + length_3) + (vsize(length_4) + length_4) + (vsize(length_5) + length_5) + (vsize(length_6) + length_6) + (vsize(length_7) + length_7) + (vsize(length_8) + length_8) + (vsize(length_9) + length_9) + (vsize(length_10) + length_10) + (vsize(length_11) + length_11) + (vsize(length_12) + length_12) + (vsize(length_13) + length_13) + (vsize(length_14) + length_14) + (vsize(length_15) + length_15) + (vsize(length_16) + length_16) + (vsize(length_17) + length_17) + (vsize(length_18) + length_18) + (vsize(length_19) + length_19) + (vsize(length_20) + length_20) + (vsize(length_21) + length_21) + (vsize(length_22) + length_22) + (vsize(length_23) + length_23) + (vsize(length_24) + length_24) + (vsize(length_25) + length_25) + (vsize(length_26) + length_26) + (vsize(length_27) + length_27) + (vsize(length_28) + length_28) + (vsize(length_29) + length_29) + (vsize(length_30) + length_30) + (vsize(length_31) + length_31) + (vsize(length_32) + length_32) + (vsize(length_33) + length_33) + (vsize(length_34) + length_34) + (vsize(length_35) + length_35) + (vsize(length_36) + length_36) + (vsize(length_37) + length_37) + (vsize(length_38) + length_38) + (vsize(length_39) + length_39) + (vsize(length_40) + length_40) + (vsize(length_41) + length_41) + (vsize(length_42) + length_42) + (vsize(length_43) + length_43) + (vsize(length_44) + length_44) + (vsize(length_45) + length_45) + (vsize(length_46) + length_46) + (vsize(length_47) + length_47) + (vsize(length_48) + length_48) + (vsize(length_49) + length_49) + (vsize(length_50) + length_50) + (vsize(length_51) + length_51) + (vsize(length_52) + length_52) + (vsize(length_53) + length_53) + (vsize(length_54) + length_54) + (vsize(length_55) + length_55) + (vsize(length_56) + length_56) + (vsize(length_57) + length_57) + (vsize(length_58) + length_58) + (vsize(length_59) + length_59) + (vsize(length_60) + length_60) + (vsize(length_61) + length_61) + (vsize(length_62) + length_62) + (vsize(length_63) + length_63) + (vsize(length_64) + length_64) + (vsize(length_65) + length_65) + (vsize(length_66) + length_66) + (vsize(length_67) + length_67) + (vsize(length_68) + length_68) + (vsize(length_69) + length_69) + (vsize(length_70) + length_70) + (vsize(length_71) + length_71) + (vsize(length_72) + length_72) + (vsize(length_73) + length_73) + (vsize(length_74) + length_74) + (vsize(length_75) + length_75) + (vsize(length_76) + length_76) + (vsize(length_77) + length_77) + (vsize(length_78) + length_78) + (vsize(length_79) + length_79) + (vsize(length_80) + length_80) + (vsize(length_81) + length_81) + (vsize(length_82) + length_82) + (vsize(length_83) + length_83) + (vsize(length_84) + length_84) + (vsize(length_85) + length_85) + (vsize(length_86) + length_86) + (vsize(length_87) + length_87) + (vsize(length_88) + length_88) + (vsize(length_89) + length_89) + (vsize(length_90) + length_90) + (vsize(length_91) + length_91) + (vsize(length_92) + length_92) + (vsize(length_93) + length_93) + (vsize(length_94) + length_94) + (vsize(length_95) + length_95) + (vsize(length_96) + length_96) + (vsize(length_97) + length_97) + (vsize(length_98) + length_98) + (vsize(length_99) + length_99) + (vsize(length_100) + length_100) + (vsize(length_101) + length_101) + (vsize(length_102) + length_102) + (vsize(length_103) + length_103) + (vsize(length_104) + length_104) + (vsize(length_105) + length_105) + (vsize(length_106) + length_106) + (vsize(length_107) + length_107) + (vsize(length_108) + length_108) + (vsize(length_109) + length_109)
end
codec.w_Wide = function(buf_2, o_2, v)
	do
		local text = v.f0
		local length_110 = #text
		o_2 = vwrite(buf_2, o_2, length_110)
		buffer.writestring(buf_2, o_2, text)
		o_2 += length_110
	end
	do
		local text_1 = v.f1
		local length_111 = #text_1
		o_2 = vwrite(buf_2, o_2, length_111)
		buffer.writestring(buf_2, o_2, text_1)
		o_2 += length_111
	end
	do
		local text_2 = v.f2
		local length_112 = #text_2
		o_2 = vwrite(buf_2, o_2, length_112)
		buffer.writestring(buf_2, o_2, text_2)
		o_2 += length_112
	end
	do
		local text_3 = v.f3
		local length_113 = #text_3
		o_2 = vwrite(buf_2, o_2, length_113)
		buffer.writestring(buf_2, o_2, text_3)
		o_2 += length_113
	end
	do
		local text_4 = v.f4
		local length_114 = #text_4
		o_2 = vwrite(buf_2, o_2, length_114)
		buffer.writestring(buf_2, o_2, text_4)
		o_2 += length_114
	end
	do
		local text_5 = v.f5
		local length_115 = #text_5
		o_2 = vwrite(buf_2, o_2, length_115)
		buffer.writestring(buf_2, o_2, text_5)
		o_2 += length_115
	end
	do
		local text_6 = v.f6
		local length_116 = #text_6
		o_2 = vwrite(buf_2, o_2, length_116)
		buffer.writestring(buf_2, o_2, text_6)
		o_2 += length_116
	end
	do
		local text_7 = v.f7
		local length_117 = #text_7
		o_2 = vwrite(buf_2, o_2, length_117)
		buffer.writestring(buf_2, o_2, text_7)
		o_2 += length_117
	end
	do
		local text_8 = v.f8
		local length_118 = #text_8
		o_2 = vwrite(buf_2, o_2, length_118)
		buffer.writestring(buf_2, o_2, text_8)
		o_2 += length_118
	end
	do
		local text_9 = v.f9
		local length_119 = #text_9
		o_2 = vwrite(buf_2, o_2, length_119)
		buffer.writestring(buf_2, o_2, text_9)
		o_2 += length_119
	end
	do
		local text_10 = v.f10
		local length_120 = #text_10
		o_2 = vwrite(buf_2, o_2, length_120)
		buffer.writestring(buf_2, o_2, text_10)
		o_2 += length_120
	end
	do
		local text_11 = v.f11
		local length_121 = #text_11
		o_2 = vwrite(buf_2, o_2, length_121)
		buffer.writestring(buf_2, o_2, text_11)
		o_2 += length_121
	end
	do
		local text_12 = v.f12
		local length_122 = #text_12
		o_2 = vwrite(buf_2, o_2, length_122)
		buffer.writestring(buf_2, o_2, text_12)
		o_2 += length_122
	end
	do
		local text_13 = v.f13
		local length_123 = #text_13
		o_2 = vwrite(buf_2, o_2, length_123)
		buffer.writestring(buf_2, o_2, text_13)
		o_2 += length_123
	end
	do
		local text_14 = v.f14
		local length_124 = #text_14
		o_2 = vwrite(buf_2, o_2, length_124)
		buffer.writestring(buf_2, o_2, text_14)
		o_2 += length_124
	end
	do
		local text_15 = v.f15
		local length_125 = #text_15
		o_2 = vwrite(buf_2, o_2, length_125)
		buffer.writestring(buf_2, o_2, text_15)
		o_2 += length_125
	end
	do
		local text_16 = v.f16
		local length_126 = #text_16
		o_2 = vwrite(buf_2, o_2, length_126)
		buffer.writestring(buf_2, o_2, text_16)
		o_2 += length_126
	end
	do
		local text_17 = v.f17
		local length_127 = #text_17
		o_2 = vwrite(buf_2, o_2, length_127)
		buffer.writestring(buf_2, o_2, text_17)
		o_2 += length_127
	end
	do
		local text_18 = v.f18
		local length_128 = #text_18
		o_2 = vwrite(buf_2, o_2, length_128)
		buffer.writestring(buf_2, o_2, text_18)
		o_2 += length_128
	end
	do
		local text_19 = v.f19
		local length_129 = #text_19
		o_2 = vwrite(buf_2, o_2, length_129)
		buffer.writestring(buf_2, o_2, text_19)
		o_2 += length_129
	end
	do
		local text_20 = v.f20
		local length_130 = #text_20
		o_2 = vwrite(buf_2, o_2, length_130)
		buffer.writestring(buf_2, o_2, text_20)
		o_2 += length_130
	end
	do
		local text_21 = v.f21
		local length_131 = #text_21
		o_2 = vwrite(buf_2, o_2, length_131)
		buffer.writestring(buf_2, o_2, text_21)
		o_2 += length_131
	end
	do
		local text_22 = v.f22
		local length_132 = #text_22
		o_2 = vwrite(buf_2, o_2, length_132)
		buffer.writestring(buf_2, o_2, text_22)
		o_2 += length_132
	end
	do
		local text_23 = v.f23
		local length_133 = #text_23
		o_2 = vwrite(buf_2, o_2, length_133)
		buffer.writestring(buf_2, o_2, text_23)
		o_2 += length_133
	end
	do
		local text_24 = v.f24
		local length_134 = #text_24
		o_2 = vwrite(buf_2, o_2, length_134)
		buffer.writestring(buf_2, o_2, text_24)
		o_2 += length_134
	end
	do
		local text_25 = v.f25
		local length_135 = #text_25
		o_2 = vwrite(buf_2, o_2, length_135)
		buffer.writestring(buf_2, o_2, text_25)
		o_2 += length_135
	end
	do
		local text_26 = v.f26
		local length_136 = #text_26
		o_2 = vwrite(buf_2, o_2, length_136)
		buffer.writestring(buf_2, o_2, text_26)
		o_2 += length_136
	end
	do
		local text_27 = v.f27
		local length_137 = #text_27
		o_2 = vwrite(buf_2, o_2, length_137)
		buffer.writestring(buf_2, o_2, text_27)
		o_2 += length_137
	end
	do
		local text_28 = v.f28
		local length_138 = #text_28
		o_2 = vwrite(buf_2, o_2, length_138)
		buffer.writestring(buf_2, o_2, text_28)
		o_2 += length_138
	end
	do
		local text_29 = v.f29
		local length_139 = #text_29
		o_2 = vwrite(buf_2, o_2, length_139)
		buffer.writestring(buf_2, o_2, text_29)
		o_2 += length_139
	end
	do
		local text_30 = v.f30
		local length_140 = #text_30
		o_2 = vwrite(buf_2, o_2, length_140)
		buffer.writestring(buf_2, o_2, text_30)
		o_2 += length_140
	end
	do
		local text_31 = v.f31
		local length_141 = #text_31
		o_2 = vwrite(buf_2, o_2, length_141)
		buffer.writestring(buf_2, o_2, text_31)
		o_2 += length_141
	end
	do
		local text_32 = v.f32
		local length_142 = #text_32
		o_2 = vwrite(buf_2, o_2, length_142)
		buffer.writestring(buf_2, o_2, text_32)
		o_2 += length_142
	end
	do
		local text_33 = v.f33
		local length_143 = #text_33
		o_2 = vwrite(buf_2, o_2, length_143)
		buffer.writestring(buf_2, o_2, text_33)
		o_2 += length_143
	end
	do
		local text_34 = v.f34
		local length_144 = #text_34
		o_2 = vwrite(buf_2, o_2, length_144)
		buffer.writestring(buf_2, o_2, text_34)
		o_2 += length_144
	end
	do
		local text_35 = v.f35
		local length_145 = #text_35
		o_2 = vwrite(buf_2, o_2, length_145)
		buffer.writestring(buf_2, o_2, text_35)
		o_2 += length_145
	end
	do
		local text_36 = v.f36
		local length_146 = #text_36
		o_2 = vwrite(buf_2, o_2, length_146)
		buffer.writestring(buf_2, o_2, text_36)
		o_2 += length_146
	end
	do
		local text_37 = v.f37
		local length_147 = #text_37
		o_2 = vwrite(buf_2, o_2, length_147)
		buffer.writestring(buf_2, o_2, text_37)
		o_2 += length_147
	end
	do
		local text_38 = v.f38
		local length_148 = #text_38
		o_2 = vwrite(buf_2, o_2, length_148)
		buffer.writestring(buf_2, o_2, text_38)
		o_2 += length_148
	end
	do
		local text_39 = v.f39
		local length_149 = #text_39
		o_2 = vwrite(buf_2, o_2, length_149)
		buffer.writestring(buf_2, o_2, text_39)
		o_2 += length_149
	end
	do
		local text_40 = v.f40
		local length_150 = #text_40
		o_2 = vwrite(buf_2, o_2, length_150)
		buffer.writestring(buf_2, o_2, text_40)
		o_2 += length_150
	end
	do
		local text_41 = v.f41
		local length_151 = #text_41
		o_2 = vwrite(buf_2, o_2, length_151)
		buffer.writestring(buf_2, o_2, text_41)
		o_2 += length_151
	end
	do
		local text_42 = v.f42
		local length_152 = #text_42
		o_2 = vwrite(buf_2, o_2, length_152)
		buffer.writestring(buf_2, o_2, text_42)
		o_2 += length_152
	end
	do
		local text_43 = v.f43
		local length_153 = #text_43
		o_2 = vwrite(buf_2, o_2, length_153)
		buffer.writestring(buf_2, o_2, text_43)
		o_2 += length_153
	end
	do
		local text_44 = v.f44
		local length_154 = #text_44
		o_2 = vwrite(buf_2, o_2, length_154)
		buffer.writestring(buf_2, o_2, text_44)
		o_2 += length_154
	end
	do
		local text_45 = v.f45
		local length_155 = #text_45
		o_2 = vwrite(buf_2, o_2, length_155)
		buffer.writestring(buf_2, o_2, text_45)
		o_2 += length_155
	end
	do
		local text_46 = v.f46
		local length_156 = #text_46
		o_2 = vwrite(buf_2, o_2, length_156)
		buffer.writestring(buf_2, o_2, text_46)
		o_2 += length_156
	end
	do
		local text_47 = v.f47
		local length_157 = #text_47
		o_2 = vwrite(buf_2, o_2, length_157)
		buffer.writestring(buf_2, o_2, text_47)
		o_2 += length_157
	end
	do
		local text_48 = v.f48
		local length_158 = #text_48
		o_2 = vwrite(buf_2, o_2, length_158)
		buffer.writestring(buf_2, o_2, text_48)
		o_2 += length_158
	end
	do
		local text_49 = v.f49
		local length_159 = #text_49
		o_2 = vwrite(buf_2, o_2, length_159)
		buffer.writestring(buf_2, o_2, text_49)
		o_2 += length_159
	end
	do
		local text_50 = v.f50
		local length_160 = #text_50
		o_2 = vwrite(buf_2, o_2, length_160)
		buffer.writestring(buf_2, o_2, text_50)
		o_2 += length_160
	end
	do
		local text_51 = v.f51
		local length_161 = #text_51
		o_2 = vwrite(buf_2, o_2, length_161)
		buffer.writestring(buf_2, o_2, text_51)
		o_2 += length_161
	end
	do
		local text_52 = v.f52
		local length_162 = #text_52
		o_2 = vwrite(buf_2, o_2, length_162)
		buffer.writestring(buf_2, o_2, text_52)
		o_2 += length_162
	end
	do
		local text_53 = v.f53
		local length_163 = #text_53
		o_2 = vwrite(buf_2, o_2, length_163)
		buffer.writestring(buf_2, o_2, text_53)
		o_2 += length_163
	end
	do
		local text_54 = v.f54
		local length_164 = #text_54
		o_2 = vwrite(buf_2, o_2, length_164)
		buffer.writestring(buf_2, o_2, text_54)
		o_2 += length_164
	end
	do
		local text_55 = v.f55
		local length_165 = #text_55
		o_2 = vwrite(buf_2, o_2, length_165)
		buffer.writestring(buf_2, o_2, text_55)
		o_2 += length_165
	end
	do
		local text_56 = v.f56
		local length_166 = #text_56
		o_2 = vwrite(buf_2, o_2, length_166)
		buffer.writestring(buf_2, o_2, text_56)
		o_2 += length_166
	end
	do
		local text_57 = v.f57
		local length_167 = #text_57
		o_2 = vwrite(buf_2, o_2, length_167)
		buffer.writestring(buf_2, o_2, text_57)
		o_2 += length_167
	end
	do
		local text_58 = v.f58
		local length_168 = #text_58
		o_2 = vwrite(buf_2, o_2, length_168)
		buffer.writestring(buf_2, o_2, text_58)
		o_2 += length_168
	end
	do
		local text_59 = v.f59
		local length_169 = #text_59
		o_2 = vwrite(buf_2, o_2, length_169)
		buffer.writestring(buf_2, o_2, text_59)
		o_2 += length_169
	end
	do
		local text_60 = v.f60
		local length_170 = #text_60
		o_2 = vwrite(buf_2, o_2, length_170)
		buffer.writestring(buf_2, o_2, text_60)
		o_2 += length_170
	end
	do
		local text_61 = v.f61
		local length_171 = #text_61
		o_2 = vwrite(buf_2, o_2, length_171)
		buffer.writestring(buf_2, o_2, text_61)
		o_2 += length_171
	end
	do
		local text_62 = v.f62
		local length_172 = #text_62
		o_2 = vwrite(buf_2, o_2, length_172)
		buffer.writestring(buf_2, o_2, text_62)
		o_2 += length_172
	end
	do
		local text_63 = v.f63
		local length_173 = #text_63
		o_2 = vwrite(buf_2, o_2, length_173)
		buffer.writestring(buf_2, o_2, text_63)
		o_2 += length_173
	end
	do
		local text_64 = v.f64
		local length_174 = #text_64
		o_2 = vwrite(buf_2, o_2, length_174)
		buffer.writestring(buf_2, o_2, text_64)
		o_2 += length_174
	end
	do
		local text_65 = v.f65
		local length_175 = #text_65
		o_2 = vwrite(buf_2, o_2, length_175)
		buffer.writestring(buf_2, o_2, text_65)
		o_2 += length_175
	end
	do
		local text_66 = v.f66
		local length_176 = #text_66
		o_2 = vwrite(buf_2, o_2, length_176)
		buffer.writestring(buf_2, o_2, text_66)
		o_2 += length_176
	end
	do
		local text_67 = v.f67
		local length_177 = #text_67
		o_2 = vwrite(buf_2, o_2, length_177)
		buffer.writestring(buf_2, o_2, text_67)
		o_2 += length_177
	end
	do
		local text_68 = v.f68
		local length_178 = #text_68
		o_2 = vwrite(buf_2, o_2, length_178)
		buffer.writestring(buf_2, o_2, text_68)
		o_2 += length_178
	end
	do
		local text_69 = v.f69
		local length_179 = #text_69
		o_2 = vwrite(buf_2, o_2, length_179)
		buffer.writestring(buf_2, o_2, text_69)
		o_2 += length_179
	end
	do
		local text_70 = v.f70
		local length_180 = #text_70
		o_2 = vwrite(buf_2, o_2, length_180)
		buffer.writestring(buf_2, o_2, text_70)
		o_2 += length_180
	end
	do
		local text_71 = v.f71
		local length_181 = #text_71
		o_2 = vwrite(buf_2, o_2, length_181)
		buffer.writestring(buf_2, o_2, text_71)
		o_2 += length_181
	end
	do
		local text_72 = v.f72
		local length_182 = #text_72
		o_2 = vwrite(buf_2, o_2, length_182)
		buffer.writestring(buf_2, o_2, text_72)
		o_2 += length_182
	end
	do
		local text_73 = v.f73
		local length_183 = #text_73
		o_2 = vwrite(buf_2, o_2, length_183)
		buffer.writestring(buf_2, o_2, text_73)
		o_2 += length_183
	end
	do
		local text_74 = v.f74
		local length_184 = #text_74
		o_2 = vwrite(buf_2, o_2, length_184)
		buffer.writestring(buf_2, o_2, text_74)
		o_2 += length_184
	end
	do
		local text_75 = v.f75
		local length_185 = #text_75
		o_2 = vwrite(buf_2, o_2, length_185)
		buffer.writestring(buf_2, o_2, text_75)
		o_2 += length_185
	end
	do
		local text_76 = v.f76
		local length_186 = #text_76
		o_2 = vwrite(buf_2, o_2, length_186)
		buffer.writestring(buf_2, o_2, text_76)
		o_2 += length_186
	end
	do
		local text_77 = v.f77
		local length_187 = #text_77
		o_2 = vwrite(buf_2, o_2, length_187)
		buffer.writestring(buf_2, o_2, text_77)
		o_2 += length_187
	end
	do
		local text_78 = v.f78
		local length_188 = #text_78
		o_2 = vwrite(buf_2, o_2, length_188)
		buffer.writestring(buf_2, o_2, text_78)
		o_2 += length_188
	end
	do
		local text_79 = v.f79
		local length_189 = #text_79
		o_2 = vwrite(buf_2, o_2, length_189)
		buffer.writestring(buf_2, o_2, text_79)
		o_2 += length_189
	end
	do
		local text_80 = v.f80
		local length_190 = #text_80
		o_2 = vwrite(buf_2, o_2, length_190)
		buffer.writestring(buf_2, o_2, text_80)
		o_2 += length_190
	end
	do
		local text_81 = v.f81
		local length_191 = #text_81
		o_2 = vwrite(buf_2, o_2, length_191)
		buffer.writestring(buf_2, o_2, text_81)
		o_2 += length_191
	end
	do
		local text_82 = v.f82
		local length_192 = #text_82
		o_2 = vwrite(buf_2, o_2, length_192)
		buffer.writestring(buf_2, o_2, text_82)
		o_2 += length_192
	end
	do
		local text_83 = v.f83
		local length_193 = #text_83
		o_2 = vwrite(buf_2, o_2, length_193)
		buffer.writestring(buf_2, o_2, text_83)
		o_2 += length_193
	end
	do
		local text_84 = v.f84
		local length_194 = #text_84
		o_2 = vwrite(buf_2, o_2, length_194)
		buffer.writestring(buf_2, o_2, text_84)
		o_2 += length_194
	end
	do
		local text_85 = v.f85
		local length_195 = #text_85
		o_2 = vwrite(buf_2, o_2, length_195)
		buffer.writestring(buf_2, o_2, text_85)
		o_2 += length_195
	end
	do
		local text_86 = v.f86
		local length_196 = #text_86
		o_2 = vwrite(buf_2, o_2, length_196)
		buffer.writestring(buf_2, o_2, text_86)
		o_2 += length_196
	end
	do
		local text_87 = v.f87
		local length_197 = #text_87
		o_2 = vwrite(buf_2, o_2, length_197)
		buffer.writestring(buf_2, o_2, text_87)
		o_2 += length_197
	end
	do
		local text_88 = v.f88
		local length_198 = #text_88
		o_2 = vwrite(buf_2, o_2, length_198)
		buffer.writestring(buf_2, o_2, text_88)
		o_2 += length_198
	end
	do
		local text_89 = v.f89
		local length_199 = #text_89
		o_2 = vwrite(buf_2, o_2, length_199)
		buffer.writestring(buf_2, o_2, text_89)
		o_2 += length_199
	end
	do
		local text_90 = v.f90
		local length_200 = #text_90
		o_2 = vwrite(buf_2, o_2, length_200)
		buffer.writestring(buf_2, o_2, text_90)
		o_2 += length_200
	end
	do
		local text_91 = v.f91
		local length_201 = #text_91
		o_2 = vwrite(buf_2, o_2, length_201)
		buffer.writestring(buf_2, o_2, text_91)
		o_2 += length_201
	end
	do
		local text_92 = v.f92
		local length_202 = #text_92
		o_2 = vwrite(buf_2, o_2, length_202)
		buffer.writestring(buf_2, o_2, text_92)
		o_2 += length_202
	end
	do
		local text_93 = v.f93
		local length_203 = #text_93
		o_2 = vwrite(buf_2, o_2, length_203)
		buffer.writestring(buf_2, o_2, text_93)
		o_2 += length_203
	end
	do
		local text_94 = v.f94
		local length_204 = #text_94
		o_2 = vwrite(buf_2, o_2, length_204)
		buffer.writestring(buf_2, o_2, text_94)
		o_2 += length_204
	end
	do
		local text_95 = v.f95
		local length_205 = #text_95
		o_2 = vwrite(buf_2, o_2, length_205)
		buffer.writestring(buf_2, o_2, text_95)
		o_2 += length_205
	end
	do
		local text_96 = v.f96
		local length_206 = #text_96
		o_2 = vwrite(buf_2, o_2, length_206)
		buffer.writestring(buf_2, o_2, text_96)
		o_2 += length_206
	end
	do
		local text_97 = v.f97
		local length_207 = #text_97
		o_2 = vwrite(buf_2, o_2, length_207)
		buffer.writestring(buf_2, o_2, text_97)
		o_2 += length_207
	end
	do
		local text_98 = v.f98
		local length_208 = #text_98
		o_2 = vwrite(buf_2, o_2, length_208)
		buffer.writestring(buf_2, o_2, text_98)
		o_2 += length_208
	end
	do
		local text_99 = v.f99
		local length_209 = #text_99
		o_2 = vwrite(buf_2, o_2, length_209)
		buffer.writestring(buf_2, o_2, text_99)
		o_2 += length_209
	end
	do
		local text_100 = v.f100
		local length_210 = #text_100
		o_2 = vwrite(buf_2, o_2, length_210)
		buffer.writestring(buf_2, o_2, text_100)
		o_2 += length_210
	end
	do
		local text_101 = v.f101
		local length_211 = #text_101
		o_2 = vwrite(buf_2, o_2, length_211)
		buffer.writestring(buf_2, o_2, text_101)
		o_2 += length_211
	end
	do
		local text_102 = v.f102
		local length_212 = #text_102
		o_2 = vwrite(buf_2, o_2, length_212)
		buffer.writestring(buf_2, o_2, text_102)
		o_2 += length_212
	end
	do
		local text_103 = v.f103
		local length_213 = #text_103
		o_2 = vwrite(buf_2, o_2, length_213)
		buffer.writestring(buf_2, o_2, text_103)
		o_2 += length_213
	end
	do
		local text_104 = v.f104
		local length_214 = #text_104
		o_2 = vwrite(buf_2, o_2, length_214)
		buffer.writestring(buf_2, o_2, text_104)
		o_2 += length_214
	end
	do
		local text_105 = v.f105
		local length_215 = #text_105
		o_2 = vwrite(buf_2, o_2, length_215)
		buffer.writestring(buf_2, o_2, text_105)
		o_2 += length_215
	end
	do
		local text_106 = v.f106
		local length_216 = #text_106
		o_2 = vwrite(buf_2, o_2, length_216)
		buffer.writestring(buf_2, o_2, text_106)
		o_2 += length_216
	end
	do
		local text_107 = v.f107
		local length_217 = #text_107
		o_2 = vwrite(buf_2, o_2, length_217)
		buffer.writestring(buf_2, o_2, text_107)
		o_2 += length_217
	end
	do
		local text_108 = v.f108
		local length_218 = #text_108
		o_2 = vwrite(buf_2, o_2, length_218)
		buffer.writestring(buf_2, o_2, text_108)
		o_2 += length_218
	end
	do
		local text_109 = v.f109
		local length_219 = #text_109
		o_2 = vwrite(buf_2, o_2, length_219)
		buffer.writestring(buf_2, o_2, text_109)
		o_2 += length_219
	end
	return o_2
end
codec.r_Wide = function(buf_2, o_2)
	local object = {}
	do
		local length_220, o_3 = vread(buf_2, o_2)
		local text_110 = buffer.readstring(buf_2, o_3, length_220)
		o_2 = o_3 + length_220
		object.f0 = text_110
	end
	do
		local length_221, o_4 = vread(buf_2, o_2)
		local text_111 = buffer.readstring(buf_2, o_4, length_221)
		o_2 = o_4 + length_221
		object.f1 = text_111
	end
	do
		local length_222, o_5 = vread(buf_2, o_2)
		local text_112 = buffer.readstring(buf_2, o_5, length_222)
		o_2 = o_5 + length_222
		object.f2 = text_112
	end
	do
		local length_223, o_6 = vread(buf_2, o_2)
		local text_113 = buffer.readstring(buf_2, o_6, length_223)
		o_2 = o_6 + length_223
		object.f3 = text_113
	end
	do
		local length_224, o_7 = vread(buf_2, o_2)
		local text_114 = buffer.readstring(buf_2, o_7, length_224)
		o_2 = o_7 + length_224
		object.f4 = text_114
	end
	do
		local length_225, o_8 = vread(buf_2, o_2)
		local text_115 = buffer.readstring(buf_2, o_8, length_225)
		o_2 = o_8 + length_225
		object.f5 = text_115
	end
	do
		local length_226, o_9 = vread(buf_2, o_2)
		local text_116 = buffer.readstring(buf_2, o_9, length_226)
		o_2 = o_9 + length_226
		object.f6 = text_116
	end
	do
		local length_227, o_10 = vread(buf_2, o_2)
		local text_117 = buffer.readstring(buf_2, o_10, length_227)
		o_2 = o_10 + length_227
		object.f7 = text_117
	end
	do
		local length_228, o_11 = vread(buf_2, o_2)
		local text_118 = buffer.readstring(buf_2, o_11, length_228)
		o_2 = o_11 + length_228
		object.f8 = text_118
	end
	do
		local length_229, o_12 = vread(buf_2, o_2)
		local text_119 = buffer.readstring(buf_2, o_12, length_229)
		o_2 = o_12 + length_229
		object.f9 = text_119
	end
	do
		local length_230, o_13 = vread(buf_2, o_2)
		local text_120 = buffer.readstring(buf_2, o_13, length_230)
		o_2 = o_13 + length_230
		object.f10 = text_120
	end
	do
		local length_231, o_14 = vread(buf_2, o_2)
		local text_121 = buffer.readstring(buf_2, o_14, length_231)
		o_2 = o_14 + length_231
		object.f11 = text_121
	end
	do
		local length_232, o_15 = vread(buf_2, o_2)
		local text_122 = buffer.readstring(buf_2, o_15, length_232)
		o_2 = o_15 + length_232
		object.f12 = text_122
	end
	do
		local length_233, o_16 = vread(buf_2, o_2)
		local text_123 = buffer.readstring(buf_2, o_16, length_233)
		o_2 = o_16 + length_233
		object.f13 = text_123
	end
	do
		local length_234, o_17 = vread(buf_2, o_2)
		local text_124 = buffer.readstring(buf_2, o_17, length_234)
		o_2 = o_17 + length_234
		object.f14 = text_124
	end
	do
		local length_235, o_18 = vread(buf_2, o_2)
		local text_125 = buffer.readstring(buf_2, o_18, length_235)
		o_2 = o_18 + length_235
		object.f15 = text_125
	end
	do
		local length_236, o_19 = vread(buf_2, o_2)
		local text_126 = buffer.readstring(buf_2, o_19, length_236)
		o_2 = o_19 + length_236
		object.f16 = text_126
	end
	do
		local length_237, o_20 = vread(buf_2, o_2)
		local text_127 = buffer.readstring(buf_2, o_20, length_237)
		o_2 = o_20 + length_237
		object.f17 = text_127
	end
	do
		local length_238, o_21 = vread(buf_2, o_2)
		local text_128 = buffer.readstring(buf_2, o_21, length_238)
		o_2 = o_21 + length_238
		object.f18 = text_128
	end
	do
		local length_239, o_22 = vread(buf_2, o_2)
		local text_129 = buffer.readstring(buf_2, o_22, length_239)
		o_2 = o_22 + length_239
		object.f19 = text_129
	end
	do
		local length_240, o_23 = vread(buf_2, o_2)
		local text_130 = buffer.readstring(buf_2, o_23, length_240)
		o_2 = o_23 + length_240
		object.f20 = text_130
	end
	do
		local length_241, o_24 = vread(buf_2, o_2)
		local text_131 = buffer.readstring(buf_2, o_24, length_241)
		o_2 = o_24 + length_241
		object.f21 = text_131
	end
	do
		local length_242, o_25 = vread(buf_2, o_2)
		local text_132 = buffer.readstring(buf_2, o_25, length_242)
		o_2 = o_25 + length_242
		object.f22 = text_132
	end
	do
		local length_243, o_26 = vread(buf_2, o_2)
		local text_133 = buffer.readstring(buf_2, o_26, length_243)
		o_2 = o_26 + length_243
		object.f23 = text_133
	end
	do
		local length_244, o_27 = vread(buf_2, o_2)
		local text_134 = buffer.readstring(buf_2, o_27, length_244)
		o_2 = o_27 + length_244
		object.f24 = text_134
	end
	do
		local length_245, o_28 = vread(buf_2, o_2)
		local text_135 = buffer.readstring(buf_2, o_28, length_245)
		o_2 = o_28 + length_245
		object.f25 = text_135
	end
	do
		local length_246, o_29 = vread(buf_2, o_2)
		local text_136 = buffer.readstring(buf_2, o_29, length_246)
		o_2 = o_29 + length_246
		object.f26 = text_136
	end
	do
		local length_247, o_30 = vread(buf_2, o_2)
		local text_137 = buffer.readstring(buf_2, o_30, length_247)
		o_2 = o_30 + length_247
		object.f27 = text_137
	end
	do
		local length_248, o_31 = vread(buf_2, o_2)
		local text_138 = buffer.readstring(buf_2, o_31, length_248)
		o_2 = o_31 + length_248
		object.f28 = text_138
	end
	do
		local length_249, o_32 = vread(buf_2, o_2)
		local text_139 = buffer.readstring(buf_2, o_32, length_249)
		o_2 = o_32 + length_249
		object.f29 = text_139
	end
	do
		local length_250, o_33 = vread(buf_2, o_2)
		local text_140 = buffer.readstring(buf_2, o_33, length_250)
		o_2 = o_33 + length_250
		object.f30 = text_140
	end
	do
		local length_251, o_34 = vread(buf_2, o_2)
		local text_141 = buffer.readstring(buf_2, o_34, length_251)
		o_2 = o_34 + length_251
		object.f31 = text_141
	end
	do
		local length_252, o_35 = vread(buf_2, o_2)
		local text_142 = buffer.readstring(buf_2, o_35, length_252)
		o_2 = o_35 + length_252
		object.f32 = text_142
	end
	do
		local length_253, o_36 = vread(buf_2, o_2)
		local text_143 = buffer.readstring(buf_2, o_36, length_253)
		o_2 = o_36 + length_253
		object.f33 = text_143
	end
	do
		local length_254, o_37 = vread(buf_2, o_2)
		local text_144 = buffer.readstring(buf_2, o_37, length_254)
		o_2 = o_37 + length_254
		object.f34 = text_144
	end
	do
		local length_255, o_38 = vread(buf_2, o_2)
		local text_145 = buffer.readstring(buf_2, o_38, length_255)
		o_2 = o_38 + length_255
		object.f35 = text_145
	end
	do
		local length_256, o_39 = vread(buf_2, o_2)
		local text_146 = buffer.readstring(buf_2, o_39, length_256)
		o_2 = o_39 + length_256
		object.f36 = text_146
	end
	do
		local length_257, o_40 = vread(buf_2, o_2)
		local text_147 = buffer.readstring(buf_2, o_40, length_257)
		o_2 = o_40 + length_257
		object.f37 = text_147
	end
	do
		local length_258, o_41 = vread(buf_2, o_2)
		local text_148 = buffer.readstring(buf_2, o_41, length_258)
		o_2 = o_41 + length_258
		object.f38 = text_148
	end
	do
		local length_259, o_42 = vread(buf_2, o_2)
		local text_149 = buffer.readstring(buf_2, o_42, length_259)
		o_2 = o_42 + length_259
		object.f39 = text_149
	end
	do
		local length_260, o_43 = vread(buf_2, o_2)
		local text_150 = buffer.readstring(buf_2, o_43, length_260)
		o_2 = o_43 + length_260
		object.f40 = text_150
	end
	do
		local length_261, o_44 = vread(buf_2, o_2)
		local text_151 = buffer.readstring(buf_2, o_44, length_261)
		o_2 = o_44 + length_261
		object.f41 = text_151
	end
	do
		local length_262, o_45 = vread(buf_2, o_2)
		local text_152 = buffer.readstring(buf_2, o_45, length_262)
		o_2 = o_45 + length_262
		object.f42 = text_152
	end
	do
		local length_263, o_46 = vread(buf_2, o_2)
		local text_153 = buffer.readstring(buf_2, o_46, length_263)
		o_2 = o_46 + length_263
		object.f43 = text_153
	end
	do
		local length_264, o_47 = vread(buf_2, o_2)
		local text_154 = buffer.readstring(buf_2, o_47, length_264)
		o_2 = o_47 + length_264
		object.f44 = text_154
	end
	do
		local length_265, o_48 = vread(buf_2, o_2)
		local text_155 = buffer.readstring(buf_2, o_48, length_265)
		o_2 = o_48 + length_265
		object.f45 = text_155
	end
	do
		local length_266, o_49 = vread(buf_2, o_2)
		local text_156 = buffer.readstring(buf_2, o_49, length_266)
		o_2 = o_49 + length_266
		object.f46 = text_156
	end
	do
		local length_267, o_50 = vread(buf_2, o_2)
		local text_157 = buffer.readstring(buf_2, o_50, length_267)
		o_2 = o_50 + length_267
		object.f47 = text_157
	end
	do
		local length_268, o_51 = vread(buf_2, o_2)
		local text_158 = buffer.readstring(buf_2, o_51, length_268)
		o_2 = o_51 + length_268
		object.f48 = text_158
	end
	do
		local length_269, o_52 = vread(buf_2, o_2)
		local text_159 = buffer.readstring(buf_2, o_52, length_269)
		o_2 = o_52 + length_269
		object.f49 = text_159
	end
	do
		local length_270, o_53 = vread(buf_2, o_2)
		local text_160 = buffer.readstring(buf_2, o_53, length_270)
		o_2 = o_53 + length_270
		object.f50 = text_160
	end
	do
		local length_271, o_54 = vread(buf_2, o_2)
		local text_161 = buffer.readstring(buf_2, o_54, length_271)
		o_2 = o_54 + length_271
		object.f51 = text_161
	end
	do
		local length_272, o_55 = vread(buf_2, o_2)
		local text_162 = buffer.readstring(buf_2, o_55, length_272)
		o_2 = o_55 + length_272
		object.f52 = text_162
	end
	do
		local length_273, o_56 = vread(buf_2, o_2)
		local text_163 = buffer.readstring(buf_2, o_56, length_273)
		o_2 = o_56 + length_273
		object.f53 = text_163
	end
	do
		local length_274, o_57 = vread(buf_2, o_2)
		local text_164 = buffer.readstring(buf_2, o_57, length_274)
		o_2 = o_57 + length_274
		object.f54 = text_164
	end
	do
		local length_275, o_58 = vread(buf_2, o_2)
		local text_165 = buffer.readstring(buf_2, o_58, length_275)
		o_2 = o_58 + length_275
		object.f55 = text_165
	end
	do
		local length_276, o_59 = vread(buf_2, o_2)
		local text_166 = buffer.readstring(buf_2, o_59, length_276)
		o_2 = o_59 + length_276
		object.f56 = text_166
	end
	do
		local length_277, o_60 = vread(buf_2, o_2)
		local text_167 = buffer.readstring(buf_2, o_60, length_277)
		o_2 = o_60 + length_277
		object.f57 = text_167
	end
	do
		local length_278, o_61 = vread(buf_2, o_2)
		local text_168 = buffer.readstring(buf_2, o_61, length_278)
		o_2 = o_61 + length_278
		object.f58 = text_168
	end
	do
		local length_279, o_62 = vread(buf_2, o_2)
		local text_169 = buffer.readstring(buf_2, o_62, length_279)
		o_2 = o_62 + length_279
		object.f59 = text_169
	end
	do
		local length_280, o_63 = vread(buf_2, o_2)
		local text_170 = buffer.readstring(buf_2, o_63, length_280)
		o_2 = o_63 + length_280
		object.f60 = text_170
	end
	do
		local length_281, o_64 = vread(buf_2, o_2)
		local text_171 = buffer.readstring(buf_2, o_64, length_281)
		o_2 = o_64 + length_281
		object.f61 = text_171
	end
	do
		local length_282, o_65 = vread(buf_2, o_2)
		local text_172 = buffer.readstring(buf_2, o_65, length_282)
		o_2 = o_65 + length_282
		object.f62 = text_172
	end
	do
		local length_283, o_66 = vread(buf_2, o_2)
		local text_173 = buffer.readstring(buf_2, o_66, length_283)
		o_2 = o_66 + length_283
		object.f63 = text_173
	end
	do
		local length_284, o_67 = vread(buf_2, o_2)
		local text_174 = buffer.readstring(buf_2, o_67, length_284)
		o_2 = o_67 + length_284
		object.f64 = text_174
	end
	do
		local length_285, o_68 = vread(buf_2, o_2)
		local text_175 = buffer.readstring(buf_2, o_68, length_285)
		o_2 = o_68 + length_285
		object.f65 = text_175
	end
	do
		local length_286, o_69 = vread(buf_2, o_2)
		local text_176 = buffer.readstring(buf_2, o_69, length_286)
		o_2 = o_69 + length_286
		object.f66 = text_176
	end
	do
		local length_287, o_70 = vread(buf_2, o_2)
		local text_177 = buffer.readstring(buf_2, o_70, length_287)
		o_2 = o_70 + length_287
		object.f67 = text_177
	end
	do
		local length_288, o_71 = vread(buf_2, o_2)
		local text_178 = buffer.readstring(buf_2, o_71, length_288)
		o_2 = o_71 + length_288
		object.f68 = text_178
	end
	do
		local length_289, o_72 = vread(buf_2, o_2)
		local text_179 = buffer.readstring(buf_2, o_72, length_289)
		o_2 = o_72 + length_289
		object.f69 = text_179
	end
	do
		local length_290, o_73 = vread(buf_2, o_2)
		local text_180 = buffer.readstring(buf_2, o_73, length_290)
		o_2 = o_73 + length_290
		object.f70 = text_180
	end
	do
		local length_291, o_74 = vread(buf_2, o_2)
		local text_181 = buffer.readstring(buf_2, o_74, length_291)
		o_2 = o_74 + length_291
		object.f71 = text_181
	end
	do
		local length_292, o_75 = vread(buf_2, o_2)
		local text_182 = buffer.readstring(buf_2, o_75, length_292)
		o_2 = o_75 + length_292
		object.f72 = text_182
	end
	do
		local length_293, o_76 = vread(buf_2, o_2)
		local text_183 = buffer.readstring(buf_2, o_76, length_293)
		o_2 = o_76 + length_293
		object.f73 = text_183
	end
	do
		local length_294, o_77 = vread(buf_2, o_2)
		local text_184 = buffer.readstring(buf_2, o_77, length_294)
		o_2 = o_77 + length_294
		object.f74 = text_184
	end
	do
		local length_295, o_78 = vread(buf_2, o_2)
		local text_185 = buffer.readstring(buf_2, o_78, length_295)
		o_2 = o_78 + length_295
		object.f75 = text_185
	end
	do
		local length_296, o_79 = vread(buf_2, o_2)
		local text_186 = buffer.readstring(buf_2, o_79, length_296)
		o_2 = o_79 + length_296
		object.f76 = text_186
	end
	do
		local length_297, o_80 = vread(buf_2, o_2)
		local text_187 = buffer.readstring(buf_2, o_80, length_297)
		o_2 = o_80 + length_297
		object.f77 = text_187
	end
	do
		local length_298, o_81 = vread(buf_2, o_2)
		local text_188 = buffer.readstring(buf_2, o_81, length_298)
		o_2 = o_81 + length_298
		object.f78 = text_188
	end
	do
		local length_299, o_82 = vread(buf_2, o_2)
		local text_189 = buffer.readstring(buf_2, o_82, length_299)
		o_2 = o_82 + length_299
		object.f79 = text_189
	end
	do
		local length_300, o_83 = vread(buf_2, o_2)
		local text_190 = buffer.readstring(buf_2, o_83, length_300)
		o_2 = o_83 + length_300
		object.f80 = text_190
	end
	do
		local length_301, o_84 = vread(buf_2, o_2)
		local text_191 = buffer.readstring(buf_2, o_84, length_301)
		o_2 = o_84 + length_301
		object.f81 = text_191
	end
	do
		local length_302, o_85 = vread(buf_2, o_2)
		local text_192 = buffer.readstring(buf_2, o_85, length_302)
		o_2 = o_85 + length_302
		object.f82 = text_192
	end
	do
		local length_303, o_86 = vread(buf_2, o_2)
		local text_193 = buffer.readstring(buf_2, o_86, length_303)
		o_2 = o_86 + length_303
		object.f83 = text_193
	end
	do
		local length_304, o_87 = vread(buf_2, o_2)
		local text_194 = buffer.readstring(buf_2, o_87, length_304)
		o_2 = o_87 + length_304
		object.f84 = text_194
	end
	do
		local length_305, o_88 = vread(buf_2, o_2)
		local text_195 = buffer.readstring(buf_2, o_88, length_305)
		o_2 = o_88 + length_305
		object.f85 = text_195
	end
	do
		local length_306, o_89 = vread(buf_2, o_2)
		local text_196 = buffer.readstring(buf_2, o_89, length_306)
		o_2 = o_89 + length_306
		object.f86 = text_196
	end
	do
		local length_307, o_90 = vread(buf_2, o_2)
		local text_197 = buffer.readstring(buf_2, o_90, length_307)
		o_2 = o_90 + length_307
		object.f87 = text_197
	end
	do
		local length_308, o_91 = vread(buf_2, o_2)
		local text_198 = buffer.readstring(buf_2, o_91, length_308)
		o_2 = o_91 + length_308
		object.f88 = text_198
	end
	do
		local length_309, o_92 = vread(buf_2, o_2)
		local text_199 = buffer.readstring(buf_2, o_92, length_309)
		o_2 = o_92 + length_309
		object.f89 = text_199
	end
	do
		local length_310, o_93 = vread(buf_2, o_2)
		local text_200 = buffer.readstring(buf_2, o_93, length_310)
		o_2 = o_93 + length_310
		object.f90 = text_200
	end
	do
		local length_311, o_94 = vread(buf_2, o_2)
		local text_201 = buffer.readstring(buf_2, o_94, length_311)
		o_2 = o_94 + length_311
		object.f91 = text_201
	end
	do
		local length_312, o_95 = vread(buf_2, o_2)
		local text_202 = buffer.readstring(buf_2, o_95, length_312)
		o_2 = o_95 + length_312
		object.f92 = text_202
	end
	do
		local length_313, o_96 = vread(buf_2, o_2)
		local text_203 = buffer.readstring(buf_2, o_96, length_313)
		o_2 = o_96 + length_313
		object.f93 = text_203
	end
	do
		local length_314, o_97 = vread(buf_2, o_2)
		local text_204 = buffer.readstring(buf_2, o_97, length_314)
		o_2 = o_97 + length_314
		object.f94 = text_204
	end
	do
		local length_315, o_98 = vread(buf_2, o_2)
		local text_205 = buffer.readstring(buf_2, o_98, length_315)
		o_2 = o_98 + length_315
		object.f95 = text_205
	end
	do
		local length_316, o_99 = vread(buf_2, o_2)
		local text_206 = buffer.readstring(buf_2, o_99, length_316)
		o_2 = o_99 + length_316
		object.f96 = text_206
	end
	do
		local length_317, o_100 = vread(buf_2, o_2)
		local text_207 = buffer.readstring(buf_2, o_100, length_317)
		o_2 = o_100 + length_317
		object.f97 = text_207
	end
	do
		local length_318, o_101 = vread(buf_2, o_2)
		local text_208 = buffer.readstring(buf_2, o_101, length_318)
		o_2 = o_101 + length_318
		object.f98 = text_208
	end
	do
		local length_319, o_102 = vread(buf_2, o_2)
		local text_209 = buffer.readstring(buf_2, o_102, length_319)
		o_2 = o_102 + length_319
		object.f99 = text_209
	end
	do
		local length_320, o_103 = vread(buf_2, o_2)
		local text_210 = buffer.readstring(buf_2, o_103, length_320)
		o_2 = o_103 + length_320
		object.f100 = text_210
	end
	do
		local length_321, o_104 = vread(buf_2, o_2)
		local text_211 = buffer.readstring(buf_2, o_104, length_321)
		o_2 = o_104 + length_321
		object.f101 = text_211
	end
	do
		local length_322, o_105 = vread(buf_2, o_2)
		local text_212 = buffer.readstring(buf_2, o_105, length_322)
		o_2 = o_105 + length_322
		object.f102 = text_212
	end
	do
		local length_323, o_106 = vread(buf_2, o_2)
		local text_213 = buffer.readstring(buf_2, o_106, length_323)
		o_2 = o_106 + length_323
		object.f103 = text_213
	end
	do
		local length_324, o_107 = vread(buf_2, o_2)
		local text_214 = buffer.readstring(buf_2, o_107, length_324)
		o_2 = o_107 + length_324
		object.f104 = text_214
	end
	do
		local length_325, o_108 = vread(buf_2, o_2)
		local text_215 = buffer.readstring(buf_2, o_108, length_325)
		o_2 = o_108 + length_325
		object.f105 = text_215
	end
	do
		local length_326, o_109 = vread(buf_2, o_2)
		local text_216 = buffer.readstring(buf_2, o_109, length_326)
		o_2 = o_109 + length_326
		object.f106 = text_216
	end
	do
		local length_327, o_110 = vread(buf_2, o_2)
		local text_217 = buffer.readstring(buf_2, o_110, length_327)
		o_2 = o_110 + length_327
		object.f107 = text_217
	end
	do
		local length_328, o_111 = vread(buf_2, o_2)
		local text_218 = buffer.readstring(buf_2, o_111, length_328)
		o_2 = o_111 + length_328
		object.f108 = text_218
	end
	do
		local length_329, o_112 = vread(buf_2, o_2)
		local text_219 = buffer.readstring(buf_2, o_112, length_329)
		o_2 = o_112 + length_329
		object.f109 = text_219
	end
	return object, o_2
end
local wide = Flamework.createSerializer({
	serialize = function(v_1)
		local buf_3 = buffer.create(codec.s_Wide(v_1))
		codec.w_Wide(buf_3, 0, v_1)
		return buf_3
	end,
	deserialize = function(buf_4)
		local value, o_113 = codec.r_Wide(buf_4, 0)
		if o_113 ~= buffer.len(buf_4) then
			error("malformed payload")
		end
		return value
	end,
})
codec.s_Narrow = function(v_2)
	local length_330 = #(v_2.n0)
	local length_331 = #(v_2.n1)
	local length_332 = #(v_2.n2)
	local length_333 = #(v_2.n3)
	local length_334 = #(v_2.n4)
	local length_335 = #(v_2.n5)
	local length_336 = #(v_2.n6)
	local length_337 = #(v_2.n7)
	local length_338 = #(v_2.n8)
	local length_339 = #(v_2.n9)
	local length_340 = #(v_2.n10)
	local length_341 = #(v_2.n11)
	local length_342 = #(v_2.n12)
	local length_343 = #(v_2.n13)
	local length_344 = #(v_2.n14)
	local length_345 = #(v_2.n15)
	local length_346 = #(v_2.n16)
	local length_347 = #(v_2.n17)
	local length_348 = #(v_2.n18)
	local length_349 = #(v_2.n19)
	local length_350 = #(v_2.n20)
	local length_351 = #(v_2.n21)
	local length_352 = #(v_2.n22)
	local length_353 = #(v_2.n23)
	local length_354 = #(v_2.n24)
	local length_355 = #(v_2.n25)
	local length_356 = #(v_2.n26)
	local length_357 = #(v_2.n27)
	local length_358 = #(v_2.n28)
	local length_359 = #(v_2.n29)
	local length_360 = #(v_2.n30)
	local length_361 = #(v_2.n31)
	local length_362 = #(v_2.n32)
	local length_363 = #(v_2.n33)
	local length_364 = #(v_2.n34)
	local length_365 = #(v_2.n35)
	local length_366 = #(v_2.n36)
	local length_367 = #(v_2.n37)
	local length_368 = #(v_2.n38)
	local length_369 = #(v_2.n39)
	return vsize(length_330) + length_330 + (vsize(length_331) + length_331) + (vsize(length_332) + length_332) + (vsize(length_333) + length_333) + (vsize(length_334) + length_334) + (vsize(length_335) + length_335) + (vsize(length_336) + length_336) + (vsize(length_337) + length_337) + (vsize(length_338) + length_338) + (vsize(length_339) + length_339) + (vsize(length_340) + length_340) + (vsize(length_341) + length_341) + (vsize(length_342) + length_342) + (vsize(length_343) + length_343) + (vsize(length_344) + length_344) + (vsize(length_345) + length_345) + (vsize(length_346) + length_346) + (vsize(length_347) + length_347) + (vsize(length_348) + length_348) + (vsize(length_349) + length_349) + (vsize(length_350) + length_350) + (vsize(length_351) + length_351) + (vsize(length_352) + length_352) + (vsize(length_353) + length_353) + (vsize(length_354) + length_354) + (vsize(length_355) + length_355) + (vsize(length_356) + length_356) + (vsize(length_357) + length_357) + (vsize(length_358) + length_358) + (vsize(length_359) + length_359) + (vsize(length_360) + length_360) + (vsize(length_361) + length_361) + (vsize(length_362) + length_362) + (vsize(length_363) + length_363) + (vsize(length_364) + length_364) + (vsize(length_365) + length_365) + (vsize(length_366) + length_366) + (vsize(length_367) + length_367) + (vsize(length_368) + length_368) + (vsize(length_369) + length_369)
end
codec.w_Narrow = function(buf_5, o_114, v_2)
	local text_220 = v_2.n0
	local length_370 = #text_220
	o_114 = vwrite(buf_5, o_114, length_370)
	buffer.writestring(buf_5, o_114, text_220)
	o_114 += length_370
	local text_221 = v_2.n1
	local length_371 = #text_221
	o_114 = vwrite(buf_5, o_114, length_371)
	buffer.writestring(buf_5, o_114, text_221)
	o_114 += length_371
	local text_222 = v_2.n2
	local length_372 = #text_222
	o_114 = vwrite(buf_5, o_114, length_372)
	buffer.writestring(buf_5, o_114, text_222)
	o_114 += length_372
	local text_223 = v_2.n3
	local length_373 = #text_223
	o_114 = vwrite(buf_5, o_114, length_373)
	buffer.writestring(buf_5, o_114, text_223)
	o_114 += length_373
	local text_224 = v_2.n4
	local length_374 = #text_224
	o_114 = vwrite(buf_5, o_114, length_374)
	buffer.writestring(buf_5, o_114, text_224)
	o_114 += length_374
	local text_225 = v_2.n5
	local length_375 = #text_225
	o_114 = vwrite(buf_5, o_114, length_375)
	buffer.writestring(buf_5, o_114, text_225)
	o_114 += length_375
	local text_226 = v_2.n6
	local length_376 = #text_226
	o_114 = vwrite(buf_5, o_114, length_376)
	buffer.writestring(buf_5, o_114, text_226)
	o_114 += length_376
	local text_227 = v_2.n7
	local length_377 = #text_227
	o_114 = vwrite(buf_5, o_114, length_377)
	buffer.writestring(buf_5, o_114, text_227)
	o_114 += length_377
	local text_228 = v_2.n8
	local length_378 = #text_228
	o_114 = vwrite(buf_5, o_114, length_378)
	buffer.writestring(buf_5, o_114, text_228)
	o_114 += length_378
	local text_229 = v_2.n9
	local length_379 = #text_229
	o_114 = vwrite(buf_5, o_114, length_379)
	buffer.writestring(buf_5, o_114, text_229)
	o_114 += length_379
	local text_230 = v_2.n10
	local length_380 = #text_230
	o_114 = vwrite(buf_5, o_114, length_380)
	buffer.writestring(buf_5, o_114, text_230)
	o_114 += length_380
	local text_231 = v_2.n11
	local length_381 = #text_231
	o_114 = vwrite(buf_5, o_114, length_381)
	buffer.writestring(buf_5, o_114, text_231)
	o_114 += length_381
	local text_232 = v_2.n12
	local length_382 = #text_232
	o_114 = vwrite(buf_5, o_114, length_382)
	buffer.writestring(buf_5, o_114, text_232)
	o_114 += length_382
	local text_233 = v_2.n13
	local length_383 = #text_233
	o_114 = vwrite(buf_5, o_114, length_383)
	buffer.writestring(buf_5, o_114, text_233)
	o_114 += length_383
	local text_234 = v_2.n14
	local length_384 = #text_234
	o_114 = vwrite(buf_5, o_114, length_384)
	buffer.writestring(buf_5, o_114, text_234)
	o_114 += length_384
	local text_235 = v_2.n15
	local length_385 = #text_235
	o_114 = vwrite(buf_5, o_114, length_385)
	buffer.writestring(buf_5, o_114, text_235)
	o_114 += length_385
	local text_236 = v_2.n16
	local length_386 = #text_236
	o_114 = vwrite(buf_5, o_114, length_386)
	buffer.writestring(buf_5, o_114, text_236)
	o_114 += length_386
	local text_237 = v_2.n17
	local length_387 = #text_237
	o_114 = vwrite(buf_5, o_114, length_387)
	buffer.writestring(buf_5, o_114, text_237)
	o_114 += length_387
	local text_238 = v_2.n18
	local length_388 = #text_238
	o_114 = vwrite(buf_5, o_114, length_388)
	buffer.writestring(buf_5, o_114, text_238)
	o_114 += length_388
	local text_239 = v_2.n19
	local length_389 = #text_239
	o_114 = vwrite(buf_5, o_114, length_389)
	buffer.writestring(buf_5, o_114, text_239)
	o_114 += length_389
	local text_240 = v_2.n20
	local length_390 = #text_240
	o_114 = vwrite(buf_5, o_114, length_390)
	buffer.writestring(buf_5, o_114, text_240)
	o_114 += length_390
	local text_241 = v_2.n21
	local length_391 = #text_241
	o_114 = vwrite(buf_5, o_114, length_391)
	buffer.writestring(buf_5, o_114, text_241)
	o_114 += length_391
	local text_242 = v_2.n22
	local length_392 = #text_242
	o_114 = vwrite(buf_5, o_114, length_392)
	buffer.writestring(buf_5, o_114, text_242)
	o_114 += length_392
	local text_243 = v_2.n23
	local length_393 = #text_243
	o_114 = vwrite(buf_5, o_114, length_393)
	buffer.writestring(buf_5, o_114, text_243)
	o_114 += length_393
	local text_244 = v_2.n24
	local length_394 = #text_244
	o_114 = vwrite(buf_5, o_114, length_394)
	buffer.writestring(buf_5, o_114, text_244)
	o_114 += length_394
	local text_245 = v_2.n25
	local length_395 = #text_245
	o_114 = vwrite(buf_5, o_114, length_395)
	buffer.writestring(buf_5, o_114, text_245)
	o_114 += length_395
	local text_246 = v_2.n26
	local length_396 = #text_246
	o_114 = vwrite(buf_5, o_114, length_396)
	buffer.writestring(buf_5, o_114, text_246)
	o_114 += length_396
	local text_247 = v_2.n27
	local length_397 = #text_247
	o_114 = vwrite(buf_5, o_114, length_397)
	buffer.writestring(buf_5, o_114, text_247)
	o_114 += length_397
	local text_248 = v_2.n28
	local length_398 = #text_248
	o_114 = vwrite(buf_5, o_114, length_398)
	buffer.writestring(buf_5, o_114, text_248)
	o_114 += length_398
	local text_249 = v_2.n29
	local length_399 = #text_249
	o_114 = vwrite(buf_5, o_114, length_399)
	buffer.writestring(buf_5, o_114, text_249)
	o_114 += length_399
	local text_250 = v_2.n30
	local length_400 = #text_250
	o_114 = vwrite(buf_5, o_114, length_400)
	buffer.writestring(buf_5, o_114, text_250)
	o_114 += length_400
	local text_251 = v_2.n31
	local length_401 = #text_251
	o_114 = vwrite(buf_5, o_114, length_401)
	buffer.writestring(buf_5, o_114, text_251)
	o_114 += length_401
	local text_252 = v_2.n32
	local length_402 = #text_252
	o_114 = vwrite(buf_5, o_114, length_402)
	buffer.writestring(buf_5, o_114, text_252)
	o_114 += length_402
	local text_253 = v_2.n33
	local length_403 = #text_253
	o_114 = vwrite(buf_5, o_114, length_403)
	buffer.writestring(buf_5, o_114, text_253)
	o_114 += length_403
	local text_254 = v_2.n34
	local length_404 = #text_254
	o_114 = vwrite(buf_5, o_114, length_404)
	buffer.writestring(buf_5, o_114, text_254)
	o_114 += length_404
	local text_255 = v_2.n35
	local length_405 = #text_255
	o_114 = vwrite(buf_5, o_114, length_405)
	buffer.writestring(buf_5, o_114, text_255)
	o_114 += length_405
	local text_256 = v_2.n36
	local length_406 = #text_256
	o_114 = vwrite(buf_5, o_114, length_406)
	buffer.writestring(buf_5, o_114, text_256)
	o_114 += length_406
	local text_257 = v_2.n37
	local length_407 = #text_257
	o_114 = vwrite(buf_5, o_114, length_407)
	buffer.writestring(buf_5, o_114, text_257)
	o_114 += length_407
	local text_258 = v_2.n38
	local length_408 = #text_258
	o_114 = vwrite(buf_5, o_114, length_408)
	buffer.writestring(buf_5, o_114, text_258)
	o_114 += length_408
	local text_259 = v_2.n39
	local length_409 = #text_259
	o_114 = vwrite(buf_5, o_114, length_409)
	buffer.writestring(buf_5, o_114, text_259)
	o_114 += length_409
	return o_114
end
codec.r_Narrow = function(buf_5, o_114)
	local length_410, o_115 = vread(buf_5, o_114)
	local text_260 = buffer.readstring(buf_5, o_115, length_410)
	o_114 = o_115 + length_410
	local length_411, o_116 = vread(buf_5, o_114)
	local text_261 = buffer.readstring(buf_5, o_116, length_411)
	o_114 = o_116 + length_411
	local length_412, o_117 = vread(buf_5, o_114)
	local text_262 = buffer.readstring(buf_5, o_117, length_412)
	o_114 = o_117 + length_412
	local length_413, o_118 = vread(buf_5, o_114)
	local text_263 = buffer.readstring(buf_5, o_118, length_413)
	o_114 = o_118 + length_413
	local length_414, o_119 = vread(buf_5, o_114)
	local text_264 = buffer.readstring(buf_5, o_119, length_414)
	o_114 = o_119 + length_414
	local length_415, o_120 = vread(buf_5, o_114)
	local text_265 = buffer.readstring(buf_5, o_120, length_415)
	o_114 = o_120 + length_415
	local length_416, o_121 = vread(buf_5, o_114)
	local text_266 = buffer.readstring(buf_5, o_121, length_416)
	o_114 = o_121 + length_416
	local length_417, o_122 = vread(buf_5, o_114)
	local text_267 = buffer.readstring(buf_5, o_122, length_417)
	o_114 = o_122 + length_417
	local length_418, o_123 = vread(buf_5, o_114)
	local text_268 = buffer.readstring(buf_5, o_123, length_418)
	o_114 = o_123 + length_418
	local length_419, o_124 = vread(buf_5, o_114)
	local text_269 = buffer.readstring(buf_5, o_124, length_419)
	o_114 = o_124 + length_419
	local length_420, o_125 = vread(buf_5, o_114)
	local text_270 = buffer.readstring(buf_5, o_125, length_420)
	o_114 = o_125 + length_420
	local length_421, o_126 = vread(buf_5, o_114)
	local text_271 = buffer.readstring(buf_5, o_126, length_421)
	o_114 = o_126 + length_421
	local length_422, o_127 = vread(buf_5, o_114)
	local text_272 = buffer.readstring(buf_5, o_127, length_422)
	o_114 = o_127 + length_422
	local length_423, o_128 = vread(buf_5, o_114)
	local text_273 = buffer.readstring(buf_5, o_128, length_423)
	o_114 = o_128 + length_423
	local length_424, o_129 = vread(buf_5, o_114)
	local text_274 = buffer.readstring(buf_5, o_129, length_424)
	o_114 = o_129 + length_424
	local length_425, o_130 = vread(buf_5, o_114)
	local text_275 = buffer.readstring(buf_5, o_130, length_425)
	o_114 = o_130 + length_425
	local length_426, o_131 = vread(buf_5, o_114)
	local text_276 = buffer.readstring(buf_5, o_131, length_426)
	o_114 = o_131 + length_426
	local length_427, o_132 = vread(buf_5, o_114)
	local text_277 = buffer.readstring(buf_5, o_132, length_427)
	o_114 = o_132 + length_427
	local length_428, o_133 = vread(buf_5, o_114)
	local text_278 = buffer.readstring(buf_5, o_133, length_428)
	o_114 = o_133 + length_428
	local length_429, o_134 = vread(buf_5, o_114)
	local text_279 = buffer.readstring(buf_5, o_134, length_429)
	o_114 = o_134 + length_429
	local length_430, o_135 = vread(buf_5, o_114)
	local text_280 = buffer.readstring(buf_5, o_135, length_430)
	o_114 = o_135 + length_430
	local length_431, o_136 = vread(buf_5, o_114)
	local text_281 = buffer.readstring(buf_5, o_136, length_431)
	o_114 = o_136 + length_431
	local length_432, o_137 = vread(buf_5, o_114)
	local text_282 = buffer.readstring(buf_5, o_137, length_432)
	o_114 = o_137 + length_432
	local length_433, o_138 = vread(buf_5, o_114)
	local text_283 = buffer.readstring(buf_5, o_138, length_433)
	o_114 = o_138 + length_433
	local length_434, o_139 = vread(buf_5, o_114)
	local text_284 = buffer.readstring(buf_5, o_139, length_434)
	o_114 = o_139 + length_434
	local length_435, o_140 = vread(buf_5, o_114)
	local text_285 = buffer.readstring(buf_5, o_140, length_435)
	o_114 = o_140 + length_435
	local length_436, o_141 = vread(buf_5, o_114)
	local text_286 = buffer.readstring(buf_5, o_141, length_436)
	o_114 = o_141 + length_436
	local length_437, o_142 = vread(buf_5, o_114)
	local text_287 = buffer.readstring(buf_5, o_142, length_437)
	o_114 = o_142 + length_437
	local length_438, o_143 = vread(buf_5, o_114)
	local text_288 = buffer.readstring(buf_5, o_143, length_438)
	o_114 = o_143 + length_438
	local length_439, o_144 = vread(buf_5, o_114)
	local text_289 = buffer.readstring(buf_5, o_144, length_439)
	o_114 = o_144 + length_439
	local length_440, o_145 = vread(buf_5, o_114)
	local text_290 = buffer.readstring(buf_5, o_145, length_440)
	o_114 = o_145 + length_440
	local length_441, o_146 = vread(buf_5, o_114)
	local text_291 = buffer.readstring(buf_5, o_146, length_441)
	o_114 = o_146 + length_441
	local length_442, o_147 = vread(buf_5, o_114)
	local text_292 = buffer.readstring(buf_5, o_147, length_442)
	o_114 = o_147 + length_442
	local length_443, o_148 = vread(buf_5, o_114)
	local text_293 = buffer.readstring(buf_5, o_148, length_443)
	o_114 = o_148 + length_443
	local length_444, o_149 = vread(buf_5, o_114)
	local text_294 = buffer.readstring(buf_5, o_149, length_444)
	o_114 = o_149 + length_444
	local length_445, o_150 = vread(buf_5, o_114)
	local text_295 = buffer.readstring(buf_5, o_150, length_445)
	o_114 = o_150 + length_445
	local length_446, o_151 = vread(buf_5, o_114)
	local text_296 = buffer.readstring(buf_5, o_151, length_446)
	o_114 = o_151 + length_446
	local length_447, o_152 = vread(buf_5, o_114)
	local text_297 = buffer.readstring(buf_5, o_152, length_447)
	o_114 = o_152 + length_447
	local length_448, o_153 = vread(buf_5, o_114)
	local text_298 = buffer.readstring(buf_5, o_153, length_448)
	o_114 = o_153 + length_448
	local length_449, o_154 = vread(buf_5, o_114)
	local text_299 = buffer.readstring(buf_5, o_154, length_449)
	o_114 = o_154 + length_449
	local value_1 = {
		n0 = text_260,
		n1 = text_261,
		n2 = text_262,
		n3 = text_263,
		n4 = text_264,
		n5 = text_265,
		n6 = text_266,
		n7 = text_267,
		n8 = text_268,
		n9 = text_269,
		n10 = text_270,
		n11 = text_271,
		n12 = text_272,
		n13 = text_273,
		n14 = text_274,
		n15 = text_275,
		n16 = text_276,
		n17 = text_277,
		n18 = text_278,
		n19 = text_279,
		n20 = text_280,
		n21 = text_281,
		n22 = text_282,
		n23 = text_283,
		n24 = text_284,
		n25 = text_285,
		n26 = text_286,
		n27 = text_287,
		n28 = text_288,
		n29 = text_289,
		n30 = text_290,
		n31 = text_291,
		n32 = text_292,
		n33 = text_293,
		n34 = text_294,
		n35 = text_295,
		n36 = text_296,
		n37 = text_297,
		n38 = text_298,
		n39 = text_299,
	}
	return value_1, o_114
end
local narrow = Flamework.createSerializer({
	serialize = function(v_3)
		local buf_6 = buffer.create(codec.s_Narrow(v_3))
		codec.w_Narrow(buf_6, 0, v_3)
		return buf_6
	end,
	deserialize = function(buf_7)
		local value_2, o_155 = codec.r_Narrow(buf_7, 0)
		if o_155 ~= buffer.len(buf_7) then
			error("malformed payload")
		end
		return value_2
	end,
})
local wideList = Flamework.createSerializer({
	serialize = function(v_4)
		local buf_8 = buffer.create(codec.s_Wide(v_4[1]) + codec.s_Narrow(v_4[2]))
		local o_156 = 0
		o_156 = codec.w_Wide(buf_8, o_156, v_4[1])
		o_156 = codec.w_Narrow(buf_8, o_156, v_4[2])
		return buf_8
	end,
	deserialize = function(buf_9)
		local o_157 = 0
		local value_3, o_158 = codec.r_Wide(buf_9, o_157)
		local value_4, o_159 = codec.r_Narrow(buf_9, o_158)
		local value_5 = { value_3, value_4 }
		o_157 = o_159
		if o_157 ~= buffer.len(buf_9) then
			error("malformed payload")
		end
		return value_5
	end,
})
return {
	wide = wide,
	narrow = narrow,
	wideList = wideList,
}
