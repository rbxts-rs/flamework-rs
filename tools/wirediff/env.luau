--!nolint
--[[
	The Roblox globals a harvested module touches, on Lune.

	- Datatypes and Instances are Lune's own (`@lune/roblox`), whose `typeof` names them as the engine does.
	- `Enum` items are tables kept per item, so that they compare and index by identity as the engine's do (Lune makes a
	  new userdata at every access); `typeof` names them `EnumItem` and their enum `Enum`, `Value` comes from Lune's enum
	  database, which also lists an enum's items for `GetEnumItems` (after the runtime harness's `roblox.luau`).
	- `t` is @rbxts/t (vendor/t.lua), loaded with the same `typeof`.
	- `game:GetService("RunService")` answers `IsServer` / `IsClient` for the realm the run is in, `warn` records its
	  message, and `__fwguard(<key>)` is the test a companion encoder uses in place of a guard (the layout's values).
	- An opaque value of any other `typeof` name (`Values` makes one for a `blob(<name>)`) is a table `typeof` names so.
]]

local fs = require("@lune/fs")
local luau = require("@lune/luau")
local roblox = require("@lune/roblox") :: any

local Env = {}

local opaque: { [any]: string } = setmetatable({}, { __mode = "k" }) :: any

local function typeOf(value: any): string
	return opaque[value] or typeof(value)
end
Env.typeOf = typeOf

local enumCache: { [string]: any } = {}
local Enum = setmetatable({}, {
	__index = function(_, enumName: string)
		local existing = enumCache[enumName]
		if existing then
			return existing
		end
		local ok, real = pcall(function()
			return roblox.Enum[enumName]
		end)
		if not ok then
			real = nil
		end
		local items: { [string]: any } = {}
		local list: { any }? = nil
		local enumType
		local itemMeta = {
			__tostring = function(item)
				return `Enum.{enumName}.{item.Name}`
			end,
		}
		local function item(itemName: string): any
			if items[itemName] == nil then
				local found = nil
				if real ~= nil then
					pcall(function()
						found = real[itemName]
					end)
				end
				if found == nil then
					error(`Enum.{enumName} has no item {itemName}`, 3)
				end
				local value = setmetatable({ Name = itemName, Value = found.Value, EnumType = enumType }, itemMeta)
				opaque[value] = "EnumItem"
				items[itemName] = value
			end
			return items[itemName]
		end
		enumType = setmetatable({
			GetEnumItems = function(): { any }
				if list == nil then
					if real == nil then
						error(`Lune's enum database has no Enum.{enumName}`, 2)
					end
					local result = {}
					for _, realItem in real:GetEnumItems() do
						table.insert(result, item(realItem.Name))
					end
					list = result
				end
				return table.clone(list :: { any })
			end,
		}, {
			__index = function(_, itemName: string)
				return item(itemName)
			end,
			__tostring = function()
				return "Enum." .. enumName
			end,
		})
		opaque[enumType] = "Enum"
		enumCache[enumName] = enumType
		return enumType
	end,
})

local DATATYPES = {
	"Vector3", "Vector2", "Vector3int16", "Vector2int16", "CFrame", "Color3", "UDim", "UDim2", "NumberRange", "Rect",
	"BrickColor", "Ray", "Region3", "Region3int16", "NumberSequence", "NumberSequenceKeypoint", "ColorSequence",
	"ColorSequenceKeypoint", "PhysicalProperties", "Faces", "Axes", "Font",
}

--- The environment's shared parts. `realm` is "Server" or "Client"; `t` is loaded from `tPath`.
function Env.create(realm: string, tPath: string): any
	local base: { [string]: any } = {}
	base.typeof = typeOf
	base.Enum = Enum
	for _, name in DATATYPES do
		if roblox[name] ~= nil then
			base[name] = roblox[name]
		end
	end
	base.Instance = { new = roblox.Instance.new }
	local runService = {
		IsServer = function()
			return realm == "Server"
		end,
		IsClient = function()
			return realm == "Client"
		end,
	}
	base.game = {
		GetService = function(_, name: string)
			if name == "RunService" then
				return runService
			end
			error(`wirediff: no service {name}`, 2)
		end,
	}
	local warnings: { string } = {}
	base.warn = function(...)
		local parts = {}
		for i = 1, select("#", ...) do
			table.insert(parts, tostring((select(i, ...))))
		end
		table.insert(warnings, table.concat(parts, " "))
	end

	local tEnv = table.clone(base)
	local loaded = luau.load(fs.readFile(tPath), { debugName = "t", environment = tEnv })()
	base.t = loaded.t

	local env = {}
	env.base = base
	env.Enum = Enum
	for _, name in DATATYPES do
		env[name] = base[name]
	end
	--- The warnings recorded since the last call.
	function env.takeWarnings(): { string }
		local taken = table.clone(warnings)
		table.clear(warnings)
		return taken
	end
	function env.newInstance(className: string): any
		return roblox.Instance.new(className)
	end
	--- A table `typeof` names `name`.
	function env.opaqueValue(name: string): any
		local value = setmetatable({}, {
			__tostring = function()
				return name
			end,
		})
		opaque[value] = name
		return value
	end
	return env
end

--- Loads a module's source in a fresh environment over `env.base` (globals the module's own `local`s don't hide;
--- `t_1` to `t_9` are `t` as well, the import's other names). `guard(key)` serves `__fwguard`.
function Env.load(env: any, source: string, guard: (string) -> any): (boolean, any)
	local moduleEnv = table.clone(env.base)
	moduleEnv.__fwguard = guard
	-- Lune's injected globals take the environment's metatable, so the import's other names are fields.
	for i = 1, 9 do
		moduleEnv["t_" .. i] = env.base.t
	end
	local ok, chunk = pcall(luau.load, source, { debugName = "module", environment = moduleEnv })
	if not ok then
		return false, `compile: {chunk}`
	end
	return pcall(chunk)
end

return Env
