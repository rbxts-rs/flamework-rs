-- Compiled with roblox-ts v3.0.0
local TS = require(game:GetService("ReplicatedStorage"):WaitForChild("rbxts_include"):WaitForChild("RuntimeLib"))
local Reflect = TS.import(script, game:GetService("ReplicatedStorage"), "rbxts_include", "node_modules", "@flamework-experimental", "core", "out").Reflect
local t = TS.import(script, game:GetService("ReplicatedStorage"), "rbxts_include", "node_modules", "@rbxts", "t", "lib", "ts").t
local _components = TS.import(script, game:GetService("ReplicatedStorage"), "rbxts_include", "node_modules", "@flamework-experimental", "components", "out")
local BaseComponent = _components.BaseComponent
local Component = _components.Component
-- No argument at all: the config is made whole.
local BareComponent
do
	local super = BaseComponent
	BareComponent = setmetatable({}, {
		__tostring = function()
			return "BareComponent"
		end,
		__index = super,
	})
	BareComponent.__index = BareComponent
	function BareComponent.new(...)
		local self = setmetatable({}, BareComponent)
		return self:constructor(...) or self
	end
	function BareComponent:constructor(...)
		super.constructor(self, ...)
	end
	do
		-- (Flamework) BareComponent metadata
		Reflect.defineMetadata(BareComponent, "identifier", "shared/config@BareComponent")
		Reflect.defineMetadata(BareComponent, "flamework:module", script)
		Reflect.defineMetadata(BareComponent, "flamework:implements", {})
		Reflect.defineMetadata(BareComponent, "flamework:parameters", { "$c:baseComponent@ComponentMetadata" })
		Reflect.defineMetadata(BareComponent, "flamework:dependencies", { {
			id = "$c:baseComponent@ComponentMetadata",
		} })
	end
	BareComponent = Component({
		attributes = {
			health = t.number,
		},
		instanceShape = {
			isA = { "Model" },
		},
	})(BareComponent) or BareComponent
end
-- An empty object.
local EmptyConfigComponent
do
	local super = BaseComponent
	EmptyConfigComponent = setmetatable({}, {
		__tostring = function()
			return "EmptyConfigComponent"
		end,
		__index = super,
	})
	EmptyConfigComponent.__index = EmptyConfigComponent
	function EmptyConfigComponent.new(...)
		local self = setmetatable({}, EmptyConfigComponent)
		return self:constructor(...) or self
	end
	function EmptyConfigComponent:constructor(...)
		super.constructor(self, ...)
	end
	do
		-- (Flamework) EmptyConfigComponent metadata
		Reflect.defineMetadata(EmptyConfigComponent, "identifier", "shared/config@EmptyConfigComponent")
		Reflect.defineMetadata(EmptyConfigComponent, "flamework:module", script)
		Reflect.defineMetadata(EmptyConfigComponent, "flamework:implements", {})
		Reflect.defineMetadata(EmptyConfigComponent, "flamework:parameters", { "$c:baseComponent@ComponentMetadata" })
		Reflect.defineMetadata(EmptyConfigComponent, "flamework:dependencies", { {
			id = "$c:baseComponent@ComponentMetadata",
		} })
	end
	EmptyConfigComponent = Component({
		attributes = {
			health = t.number,
		},
		instanceShape = {
			isA = { "Instance" },
		},
	})(EmptyConfigComponent) or EmptyConfigComponent
end
-- A guard written by hand for one attribute: the generated one for it is left out.
local CustomAttributeComponent
do
	local super = BaseComponent
	CustomAttributeComponent = setmetatable({}, {
		__tostring = function()
			return "CustomAttributeComponent"
		end,
		__index = super,
	})
	CustomAttributeComponent.__index = CustomAttributeComponent
	function CustomAttributeComponent.new(...)
		local self = setmetatable({}, CustomAttributeComponent)
		return self:constructor(...) or self
	end
	function CustomAttributeComponent:constructor(...)
		super.constructor(self, ...)
	end
	do
		-- (Flamework) CustomAttributeComponent metadata
		Reflect.defineMetadata(CustomAttributeComponent, "identifier", "shared/config@CustomAttributeComponent")
		Reflect.defineMetadata(CustomAttributeComponent, "flamework:module", script)
		Reflect.defineMetadata(CustomAttributeComponent, "flamework:implements", {})
		Reflect.defineMetadata(CustomAttributeComponent, "flamework:parameters", { "$c:baseComponent@ComponentMetadata" })
		Reflect.defineMetadata(CustomAttributeComponent, "flamework:dependencies", { {
			id = "$c:baseComponent@ComponentMetadata",
		} })
	end
	CustomAttributeComponent = Component({
		tag = "F6Custom",
		defaults = {
			health = 100,
		},
		attributes = {
			health = t.numberConstrained(0, 100),
			name = t.string,
		},
		instanceShape = {
			isA = { "Part" },
		},
	})(CustomAttributeComponent) or CustomAttributeComponent
end
-- Written with a string key and in another order, after the field the shape follows.
local OrderedConfigComponent
do
	local super = BaseComponent
	OrderedConfigComponent = setmetatable({}, {
		__tostring = function()
			return "OrderedConfigComponent"
		end,
		__index = super,
	})
	OrderedConfigComponent.__index = OrderedConfigComponent
	function OrderedConfigComponent.new(...)
		local self = setmetatable({}, OrderedConfigComponent)
		return self:constructor(...) or self
	end
	function OrderedConfigComponent:constructor(...)
		super.constructor(self, ...)
	end
	do
		-- (Flamework) OrderedConfigComponent metadata
		Reflect.defineMetadata(OrderedConfigComponent, "identifier", "shared/config@OrderedConfigComponent")
		Reflect.defineMetadata(OrderedConfigComponent, "flamework:module", script)
		Reflect.defineMetadata(OrderedConfigComponent, "flamework:implements", {})
		Reflect.defineMetadata(OrderedConfigComponent, "flamework:parameters", { "$c:baseComponent@ComponentMetadata" })
		Reflect.defineMetadata(OrderedConfigComponent, "flamework:dependencies", { {
			id = "$c:baseComponent@ComponentMetadata",
		} })
	end
	OrderedConfigComponent = Component({
		tag = "F6Ordered",
		attributes = {
			name = t.string,
			health = t.number,
		},
		instanceShape = {
			isA = { "Part" },
		},
	})(OrderedConfigComponent) or OrderedConfigComponent
end
-- The same attributes as its base: nothing to check twice.
local BaseAttributeComponent
do
	local super = BaseComponent
	BaseAttributeComponent = setmetatable({}, {
		__tostring = function()
			return "BaseAttributeComponent"
		end,
		__index = super,
	})
	BaseAttributeComponent.__index = BaseAttributeComponent
	function BaseAttributeComponent.new(...)
		local self = setmetatable({}, BaseAttributeComponent)
		return self:constructor(...) or self
	end
	function BaseAttributeComponent:constructor(...)
		super.constructor(self, ...)
	end
	do
		-- (Flamework) BaseAttributeComponent metadata
		Reflect.defineMetadata(BaseAttributeComponent, "identifier", "shared/config@BaseAttributeComponent")
		Reflect.defineMetadata(BaseAttributeComponent, "flamework:module", script)
		Reflect.defineMetadata(BaseAttributeComponent, "flamework:implements", {})
		Reflect.defineMetadata(BaseAttributeComponent, "flamework:parameters", { "$c:baseComponent@ComponentMetadata" })
		Reflect.defineMetadata(BaseAttributeComponent, "flamework:dependencies", { {
			id = "$c:baseComponent@ComponentMetadata",
		} })
	end
	BaseAttributeComponent = Component({
		tag = "F6Base",
		attributes = {
			shared = t.number,
			own = t.string,
		},
		instanceShape = {
			isA = { "BasePart" },
		},
	})(BaseAttributeComponent) or BaseAttributeComponent
end
local DerivedAttributeComponent
do
	local super = BaseAttributeComponent
	DerivedAttributeComponent = setmetatable({}, {
		__tostring = function()
			return "DerivedAttributeComponent"
		end,
		__index = super,
	})
	DerivedAttributeComponent.__index = DerivedAttributeComponent
	function DerivedAttributeComponent.new(...)
		local self = setmetatable({}, DerivedAttributeComponent)
		return self:constructor(...) or self
	end
	function DerivedAttributeComponent:constructor(...)
		super.constructor(self, ...)
	end
	do
		-- (Flamework) DerivedAttributeComponent metadata
		Reflect.defineMetadata(DerivedAttributeComponent, "identifier", "shared/config@DerivedAttributeComponent")
		Reflect.defineMetadata(DerivedAttributeComponent, "flamework:module", script)
		Reflect.defineMetadata(DerivedAttributeComponent, "flamework:implements", {})
		Reflect.defineMetadata(DerivedAttributeComponent, "flamework:parameters", { "$c:baseComponent@ComponentMetadata" })
		Reflect.defineMetadata(DerivedAttributeComponent, "flamework:dependencies", { {
			id = "$c:baseComponent@ComponentMetadata",
		} })
	end
	DerivedAttributeComponent = Component({
		tag = "F6Derived",
		attributes = {},
	})(DerivedAttributeComponent) or DerivedAttributeComponent
end
-- A narrower instance than its base, and an attribute of its own.
local AbstractBase
do
	local super = BaseComponent
	AbstractBase = setmetatable({}, {
		__tostring = function()
			return "AbstractBase"
		end,
		__index = super,
	})
	AbstractBase.__index = AbstractBase
	function AbstractBase:constructor(...)
		super.constructor(self, ...)
	end
end
local NarrowComponent
do
	local super = AbstractBase
	NarrowComponent = setmetatable({}, {
		__tostring = function()
			return "NarrowComponent"
		end,
		__index = super,
	})
	NarrowComponent.__index = NarrowComponent
	function NarrowComponent.new(...)
		local self = setmetatable({}, NarrowComponent)
		return self:constructor(...) or self
	end
	function NarrowComponent:constructor(...)
		super.constructor(self, ...)
	end
	do
		-- (Flamework) NarrowComponent metadata
		Reflect.defineMetadata(NarrowComponent, "identifier", "shared/config@NarrowComponent")
		Reflect.defineMetadata(NarrowComponent, "flamework:module", script)
		Reflect.defineMetadata(NarrowComponent, "flamework:implements", {})
		Reflect.defineMetadata(NarrowComponent, "flamework:parameters", { "$c:baseComponent@ComponentMetadata" })
		Reflect.defineMetadata(NarrowComponent, "flamework:dependencies", { {
			id = "$c:baseComponent@ComponentMetadata",
		} })
	end
	NarrowComponent = Component({
		tag = "F6Narrow",
		attributes = {
			level = t.number,
			extra = t.boolean,
		},
	})(NarrowComponent) or NarrowComponent
end
-- No instance type: `Instance` is the base's own, so there is no shape.
local PlainComponent
do
	local super = BaseComponent
	PlainComponent = setmetatable({}, {
		__tostring = function()
			return "PlainComponent"
		end,
		__index = super,
	})
	PlainComponent.__index = PlainComponent
	function PlainComponent.new(...)
		local self = setmetatable({}, PlainComponent)
		return self:constructor(...) or self
	end
	function PlainComponent:constructor(...)
		super.constructor(self, ...)
	end
	do
		-- (Flamework) PlainComponent metadata
		Reflect.defineMetadata(PlainComponent, "identifier", "shared/config@PlainComponent")
		Reflect.defineMetadata(PlainComponent, "flamework:module", script)
		Reflect.defineMetadata(PlainComponent, "flamework:implements", {})
		Reflect.defineMetadata(PlainComponent, "flamework:parameters", { "$c:baseComponent@ComponentMetadata" })
		Reflect.defineMetadata(PlainComponent, "flamework:dependencies", { {
			id = "$c:baseComponent@ComponentMetadata",
		} })
	end
	PlainComponent = Component({
		tag = "F6Plain",
		attributes = {
			amount = t.optional(t.number),
			mode = t.literal("a", "b"),
		},
		instanceShape = {
			isA = { "Instance" },
		},
	})(PlainComponent) or PlainComponent
end
-- An optional child of a child, a class union and a component deeper in the tree are left to the guard.
local GuardedComponent
do
	local super = BaseComponent
	GuardedComponent = setmetatable({}, {
		__tostring = function()
			return "GuardedComponent"
		end,
		__index = super,
	})
	GuardedComponent.__index = GuardedComponent
	function GuardedComponent.new(...)
		local self = setmetatable({}, GuardedComponent)
		return self:constructor(...) or self
	end
	function GuardedComponent:constructor(...)
		super.constructor(self, ...)
	end
	do
		-- (Flamework) GuardedComponent metadata
		Reflect.defineMetadata(GuardedComponent, "identifier", "shared/config@GuardedComponent")
		Reflect.defineMetadata(GuardedComponent, "flamework:module", script)
		Reflect.defineMetadata(GuardedComponent, "flamework:implements", {})
		Reflect.defineMetadata(GuardedComponent, "flamework:parameters", { "$c:baseComponent@ComponentMetadata" })
		Reflect.defineMetadata(GuardedComponent, "flamework:dependencies", { {
			id = "$c:baseComponent@ComponentMetadata",
		} })
	end
	GuardedComponent = Component({
		tag = "F6Guarded",
		attributes = {},
		instanceShape = {
			isA = { "Model" },
			children = {
				Body = {
					isA = { "BasePart", "Folder" },
				},
				Extra = {
					isA = { "Folder" },
					children = {
						Inner = {
							isA = { "Part" },
						},
					},
				},
			},
		},
	})(GuardedComponent) or GuardedComponent
end
-- Attributes with every kind of value an attribute takes.
local ValuesComponent
do
	local super = BaseComponent
	ValuesComponent = setmetatable({}, {
		__tostring = function()
			return "ValuesComponent"
		end,
		__index = super,
	})
	ValuesComponent.__index = ValuesComponent
	function ValuesComponent.new(...)
		local self = setmetatable({}, ValuesComponent)
		return self:constructor(...) or self
	end
	function ValuesComponent:constructor(...)
		super.constructor(self, ...)
	end
	do
		-- (Flamework) ValuesComponent metadata
		Reflect.defineMetadata(ValuesComponent, "identifier", "shared/config@ValuesComponent")
		Reflect.defineMetadata(ValuesComponent, "flamework:module", script)
		Reflect.defineMetadata(ValuesComponent, "flamework:implements", {})
		Reflect.defineMetadata(ValuesComponent, "flamework:parameters", { "$c:baseComponent@ComponentMetadata" })
		Reflect.defineMetadata(ValuesComponent, "flamework:dependencies", { {
			id = "$c:baseComponent@ComponentMetadata",
		} })
	end
	ValuesComponent = Component({
		tag = "F6Values",
		attributes = {
			v2 = t.Vector2,
			v3 = t.Vector3,
			color = t.Color3,
			udim = t.UDim2,
			range = t.NumberRange,
			cframe = t.CFrame,
			text = t.literal("off", "on"),
			number = t.literalList({ 1, 2, 3 }),
			either = t.optional(t.union(t.string, t.number)),
		},
		instanceShape = {
			isA = { "Model" },
			children = {
				Primary = {
					isA = { "Part" },
				},
				Folder = {
					isA = { "Folder" },
					children = {
						Nested = {
							isA = { "Model" },
							children = {
								Leaf = {
									isA = { "MeshPart" },
								},
							},
						},
					},
				},
			},
		},
	})(ValuesComponent) or ValuesComponent
end
-- Written attributes and a link: the link goes first, the written attributes after it.
local LinkedCustomComponent
do
	local super = BaseComponent
	LinkedCustomComponent = setmetatable({}, {
		__tostring = function()
			return "LinkedCustomComponent"
		end,
		__index = super,
	})
	LinkedCustomComponent.__index = LinkedCustomComponent
	function LinkedCustomComponent.new(...)
		local self = setmetatable({}, LinkedCustomComponent)
		return self:constructor(...) or self
	end
	function LinkedCustomComponent:constructor(...)
		super.constructor(self, ...)
	end
	do
		-- (Flamework) LinkedCustomComponent metadata
		Reflect.defineMetadata(LinkedCustomComponent, "identifier", "shared/config@LinkedCustomComponent")
		Reflect.defineMetadata(LinkedCustomComponent, "flamework:module", script)
		Reflect.defineMetadata(LinkedCustomComponent, "flamework:implements", {})
		Reflect.defineMetadata(LinkedCustomComponent, "flamework:parameters", { "$c:baseComponent@ComponentMetadata" })
		Reflect.defineMetadata(LinkedCustomComponent, "flamework:dependencies", { {
			id = "$c:baseComponent@ComponentMetadata",
		} })
	end
	LinkedCustomComponent = Component({
		tag = "F6LinkedCustom",
		instanceGuard = t.instanceIsA("Model"),
		links = { {
			kind = "attribute",
			name = "target",
			optional = false,
			shape = {
				isA = { "Part" },
			},
		} },
		attributes = {
			name = t.string,
			target = t.typeof("InstanceHandle"),
		},
	})(LinkedCustomComponent) or LinkedCustomComponent
end
-- Attributes written as anything but an object literal give way to the generated guards.
local guards = {
	name = t.string,
}
local HeldGuardsComponent
do
	local super = BaseComponent
	HeldGuardsComponent = setmetatable({}, {
		__tostring = function()
			return "HeldGuardsComponent"
		end,
		__index = super,
	})
	HeldGuardsComponent.__index = HeldGuardsComponent
	function HeldGuardsComponent.new(...)
		local self = setmetatable({}, HeldGuardsComponent)
		return self:constructor(...) or self
	end
	function HeldGuardsComponent:constructor(...)
		super.constructor(self, ...)
	end
	do
		-- (Flamework) HeldGuardsComponent metadata
		Reflect.defineMetadata(HeldGuardsComponent, "identifier", "shared/config@HeldGuardsComponent")
		Reflect.defineMetadata(HeldGuardsComponent, "flamework:module", script)
		Reflect.defineMetadata(HeldGuardsComponent, "flamework:implements", {})
		Reflect.defineMetadata(HeldGuardsComponent, "flamework:parameters", { "$c:baseComponent@ComponentMetadata" })
		Reflect.defineMetadata(HeldGuardsComponent, "flamework:dependencies", { {
			id = "$c:baseComponent@ComponentMetadata",
		} })
	end
	HeldGuardsComponent = Component({
		tag = "F6HeldGuards",
		attributes = {
			name = t.string,
			count = t.number,
		},
		instanceShape = {
			isA = { "Instance" },
		},
	})(HeldGuardsComponent) or HeldGuardsComponent
end
-- A computed name: the written fields cannot be addressed one by one, so the literal is reordered.
local ComputedNameComponent
do
	local super = BaseComponent
	ComputedNameComponent = setmetatable({}, {
		__tostring = function()
			return "ComputedNameComponent"
		end,
		__index = super,
	})
	ComputedNameComponent.__index = ComputedNameComponent
	function ComputedNameComponent.new(...)
		local self = setmetatable({}, ComputedNameComponent)
		return self:constructor(...) or self
	end
	function ComputedNameComponent:constructor(...)
		super.constructor(self, ...)
	end
	do
		-- (Flamework) ComputedNameComponent metadata
		Reflect.defineMetadata(ComputedNameComponent, "identifier", "shared/config@ComputedNameComponent")
		Reflect.defineMetadata(ComputedNameComponent, "flamework:module", script)
		Reflect.defineMetadata(ComputedNameComponent, "flamework:implements", {})
		Reflect.defineMetadata(ComputedNameComponent, "flamework:parameters", { "$c:baseComponent@ComponentMetadata" })
		Reflect.defineMetadata(ComputedNameComponent, "flamework:dependencies", { {
			id = "$c:baseComponent@ComponentMetadata",
		} })
	end
	ComputedNameComponent = Component({
		tag = "F6Computed",
		refreshAttributes = true,
		attributes = {
			name = t.string,
			count = t.number,
		},
		instanceShape = {
			isA = { "Part" },
		},
	})(ComputedNameComponent) or ComputedNameComponent
end
return {
	BareComponent = BareComponent,
	EmptyConfigComponent = EmptyConfigComponent,
	CustomAttributeComponent = CustomAttributeComponent,
	OrderedConfigComponent = OrderedConfigComponent,
	BaseAttributeComponent = BaseAttributeComponent,
	DerivedAttributeComponent = DerivedAttributeComponent,
	NarrowComponent = NarrowComponent,
	PlainComponent = PlainComponent,
	GuardedComponent = GuardedComponent,
	ValuesComponent = ValuesComponent,
	LinkedCustomComponent = LinkedCustomComponent,
	HeldGuardsComponent = HeldGuardsComponent,
	ComputedNameComponent = ComputedNameComponent,
}
