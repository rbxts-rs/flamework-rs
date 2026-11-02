--!nolint
--[[
	The wire-diff worker: runs checks W1-W5 on pairs of harvested modules (an old build's and a new build's of the
	same site) and writes what it found as JSON. `wirediff.cjs` pairs the folders, splits the pairs between workers and
	sums up; see README.md.

	usage: lune run wirediff.luau <job.json> <results.json>
	job.json: { options: { seed, random, w4, invalid, realm, t }, pairs: [ { old, new, path } ] }

	Per pair, the values are every edge value of the layout and `random` random ones (seeded by the run's seed and the
	pair's path); then:
	- W1: both encoders give the same buffer and blob list (or both raise the same message);
	- W2: each decoder reads both buffers, and the four results agree (each build's own reading is its normal form of
	  the value; a normal form other than the value itself is counted as `lossy`, not a failure);
	- W3: `invalid` invalid values (wrong type, out of width, non-integer, a hole, an unknown literal, a missing
	  field): both encoders raise the same message, or both accept with the same bytes;
	- W4: for the first `w4` values, every truncation of the buffer (at most 256), 64 random byte flips and an
	  appended byte: both decoders give the same result or raise the same message;
	- W5, versioned serializers: the header's version byte, then a changed version, each changed hash byte and every
	  short buffer: both decoders refuse them with the same message.
	Messages are compared without their `chunk:line:` prefix; the warnings a call makes count as part of its result.
]]

local fs = require("@lune/fs")
local process = require("@lune/process")
local serde = require("@lune/serde")

local Env = require("./env")
local Values = require("./values")

local jobPath, resultsPath = process.args[1], process.args[2]
local job = serde.decode("json", fs.readFile(jobPath))
local options = job.options
local env = Env.create(options.realm or "Server", options.t)
local typeOf = Env.typeOf

-- --- comparing ----------------------------------------------------------------------------------------------------

local ids: { [any]: number } = setmetatable({}, { __mode = "k" }) :: any
local nextId = 0
local function identity(v: any): number
	if ids[v] == nil then
		nextId += 1
		ids[v] = nextId
	end
	return ids[v]
end

local PARTS = {
	Vector3 = { "X", "Y", "Z" },
	Vector2 = { "X", "Y" },
	Vector3int16 = { "X", "Y", "Z" },
	Vector2int16 = { "X", "Y" },
	Color3 = { "R", "G", "B" },
	UDim = { "Scale", "Offset" },
	UDim2 = { "X", "Y" },
	NumberRange = { "Min", "Max" },
	Rect = { "Min", "Max" },
	BrickColor = { "Number" },
}

--- A canonical text of a value: equal texts for deep-equal values, numbers by their bits (NaN and -0 included),
--- Instances and other references by identity.
local function canon(v: any, depth: number?): string
	local d = depth or 0
	local t = typeOf(v)
	if t == "nil" then
		return "nil"
	elseif t == "boolean" then
		return tostring(v)
	elseif t == "number" then
		return "n" .. string.pack("<d", v)
	elseif t == "string" then
		return "s" .. #v .. ":" .. v
	elseif t == "buffer" then
		return "B" .. buffer.len(v) .. ":" .. buffer.tostring(v)
	elseif t == "EnumItem" or t == "Enum" then
		return "E:" .. tostring(v)
	elseif t == "table" then
		if d > 64 then
			return "<deep>"
		end
		local entries = {}
		for k, x in v do
			table.insert(entries, canon(k, d + 1) .. "=" .. canon(x, d + 1))
		end
		table.sort(entries)
		return "{" .. table.concat(entries, ",") .. "}"
	elseif t == "CFrame" then
		local parts = {}
		for _, n in { v:GetComponents() } do
			table.insert(parts, canon(n))
		end
		return "CFrame(" .. table.concat(parts, ",") .. ")"
	elseif PARTS[t] then
		local parts = {}
		for _, name in PARTS[t] do
			table.insert(parts, canon(v[name], d + 1))
		end
		return t .. "(" .. table.concat(parts, ",") .. ")"
	end
	return t .. "#" .. identity(v)
end

--- A short readable form of a value, for examples.
local function show(v: any, depth: number?, limit: number?): string
	local d = depth or 0
	local t = typeOf(v)
	local text
	if t == "string" then
		local cut = limit or 40
		local s = if #v > cut then string.sub(v, 1, cut) .. `...({#v} bytes)` else v
		text = '"' .. string.gsub(s, "[%c\128-\255\"\\]", function(c)
			return string.format("\\x%02x", string.byte(c))
		end) .. '"'
	elseif t == "number" then
		text = if v ~= v then "NaN" elseif v == 0 and 1 / v < 0 then "-0" else string.format("%.17g", v)
	elseif t == "buffer" then
		local n = math.min(buffer.len(v), 24)
		local hex = {}
		for i = 0, n - 1 do
			table.insert(hex, string.format("%02x", buffer.readu8(v, i)))
		end
		text = `buffer({buffer.len(v)}: {table.concat(hex)}{if buffer.len(v) > n then "..." else ""})`
	elseif t == "table" then
		if d > 4 then
			return "{...}"
		end
		local parts = {}
		for k, x in v do
			table.insert(parts, `[{show(k, d + 1)}] = {show(x, d + 1)}`)
			if #parts >= 8 then
				table.insert(parts, "...")
				break
			end
		end
		text = "{" .. table.concat(parts, ", ") .. "}"
	elseif t == "nil" or t == "boolean" or t == "EnumItem" then
		text = tostring(v)
	else
		text = t .. "(" .. tostring(v) .. ")"
	end
	if #text > 300 then
		text = string.sub(text, 1, 300) .. "..."
	end
	return text
end

--- An error message without the `chunk:line:` prefixes Luau puts on it.
local function message(e: any): string
	local s
	if type(e) == "string" then
		s = e
	elseif typeOf(e) == "error" then
		s = tostring(e)
	else
		return "<" .. typeOf(e) .. ">"
	end
	s = string.gsub(s, "\nstack traceback:.*$", "")
	while true do
		local stripped = string.gsub(s, '^%[string "[^"]*"%]:%d+: ', "", 1)
		stripped = string.gsub(stripped, "^[^\n]-%.luau:%d+: ", "", 1)
		if stripped == s then
			return s
		end
		s = stripped
	end
end

--- A call's outcome: its first two results, or the message it raised, and the warnings it made.
local function call(f: any, ...): any
	env.takeWarnings()
	local r = table.pack(pcall(f, ...))
	local warnings = env.takeWarnings()
	if not r[1] then
		return { ok = false, err = message(r[2]), warnings = warnings }
	end
	return { ok = true, a = r[2], b = r[3], warnings = warnings }
end

local function outcomeText(o: any): string
	local warned = table.concat(o.warnings, "\n")
	if o.ok then
		return "ok|" .. canon(o.a) .. "|" .. canon(o.b) .. "|" .. warned
	end
	return "error|" .. o.err .. "|" .. warned
end

local function showOutcome(o: any): string
	local warned = if #o.warnings > 0 then " warned " .. show(o.warnings) else ""
	if o.ok then
		return "ok " .. show(o.a) .. (if o.b ~= nil then " blobs " .. show(o.b) else "") .. warned
	end
	return "error " .. show(o.err, 0, 200) .. warned
end

-- --- modules ------------------------------------------------------------------------------------------------------

--- A loaded module's encoder and decoder, in one calling convention: `enc(value)` gives `payload, blobs` and
--- `dec(payload, blobs)` the value (an argument list packed with its count).
local function codec(m: any): any
	if m.kind == "serializer" then
		local s = m.serializer
		return {
			version = s.version,
			enc = function(v)
				return s.serialize(v)
			end,
			dec = function(buf, blobs)
				return s.deserialize(buf, blobs)
			end,
		}
	end
	local function unpackResult(r: any)
		if r == nil then
			return nil
		end
		return r[1], r[2]
	end
	local enc
	if m.kind == "decoder" then
		enc = function(args)
			return unpackResult(m.encode(args))
		end
	elseif m.kind == "site" then
		enc = function(args)
			return unpackResult(m.pack(args))
		end
	else
		enc = function(args)
			return unpackResult(m.pack(args[1]))
		end
	end
	return {
		enc = enc,
		dec = function(buf, blobs)
			return m.deserialize(buf, blobs)
		end,
		list = true,
	}
end

local guards = Values.guard({ env = env })

--- An argument list without its count, as a decoder gives it back.
local function withoutCount(v: any): any
	if type(v) ~= "table" then
		return v
	end
	local copy = table.clone(v)
	copy.n = nil
	return copy
end

local function load(path: string): (boolean, any)
	local ok, source = pcall(fs.readFile, path)
	if not ok then
		return false, "read: " .. tostring(source)
	end
	return Env.load(env, source, guards)
end

-- --- one pair -----------------------------------------------------------------------------------------------------

local CHECKS = { "W0", "W1", "W2", "W3", "W4", "W5" }
local MAX_EXAMPLES = 3

local function runPair(pair: any): any
	local result = { path = pair.path, values = 0, rejected = 0, headers = 0, invalid = 0, malformed = 0, lossy = 0, failures = {}, examples = {} }
	for _, c in CHECKS do
		result.failures[c] = 0
	end
	local function fail(check: string, what: string, value: any, old: any?, new: any?)
		result.failures[check] += 1
		if #result.examples < MAX_EXAMPLES * #CHECKS then
			local count = 0
			for _, e in result.examples do
				if e.check == check then
					count += 1
				end
			end
			if count < MAX_EXAMPLES then
				table.insert(result.examples, {
					check = check,
					what = what,
					value = show(value),
					old = if old ~= nil then showOutcome(old) else nil,
					new = if new ~= nil then showOutcome(new) else nil,
				})
			end
		end
	end

	local okOld, old = load(pair.old)
	local okNew, new = load(pair.new)
	if not okOld or not okNew then
		local a = if okOld then "loaded" else message(old)
		local b = if okNew then "loaded" else message(new)
		result.status = "load"
		result.reason = show(if okOld then b else a)
		if a ~= b then
			fail("W0", "one module loads and the other does not, or they fail differently", nil, { ok = false, err = a, warnings = {} }, { ok = false, err = b, warnings = {} })
		end
		return result
	end
	result.kind = old.kind
	for _, field in { "kind", "layout", "key", "arity" } do
		if old[field] ~= new[field] then
			fail("W0", `the modules' {field} differs`, nil, { ok = true, a = old[field], warnings = {} }, { ok = true, a = new[field], warnings = {} })
		end
	end
	if canon(old.fixed) ~= canon(new.fixed) then
		fail("W0", "the call sites' literal arguments differ", nil)
	end
	if result.failures.W0 > 0 then
		result.status = "mismatch"
		return result
	end
	local oc, nc = codec(old), codec(new)
	if oc.version ~= nc.version then
		fail("W0", "the serializers' versions differ", nil)
		result.status = "mismatch"
		return result
	end

	local parsed, root = pcall(Values.parse, old.key)
	if not parsed then
		result.status = "unparsed"
		result.reason = show(tostring(root))
		return result
	end
	local r = Values.rng(Values.hash(pair.path, options.seed))
	local ctx = { env = env, r = r }

	-- A call site's arguments: its count, and the literal arguments its packing was built for.
	local rest = nil
	if old.arity and root.t == "list" and root.rest then
		rest = math.max(old.arity - #root.elements - #root.after, 0)
	end
	local function asArguments(v: any): any
		if not oc.list or type(v) ~= "table" then
			return v
		end
		local n = Values.counts[v] or #v
		for i, box in old.fixed or {} do
			v[i] = box[1]
			n = math.max(n, i)
		end
		if old.arity then
			for i = old.arity + 1, math.max(n, #v) do
				v[i] = nil
			end
		end
		v.n = old.arity or n
		return v
	end

	local values = {}
	local okEdges, edges = pcall(Values.edges, ctx, root)
	if okEdges then
		for _, v in edges do
			table.insert(values, v)
		end
	else
		result.edgeError = show(tostring(edges))
	end
	if rest ~= nil then
		-- the edges' rest counts are not the site's
		values = {}
	end
	for _ = 1, options.random do
		local ok, v = pcall(Values.random, ctx, root, rest)
		if ok then
			table.insert(values, v)
		end
	end
	if #values == 0 then
		result.status = "nothing generated"
		result.reason = result.edgeError
		return result
	end

	local w4Done = 0
	for _, raw in values do
		local v = asArguments(if raw == Values.NIL then nil else raw)
		result.values += 1
		local o, n = call(oc.enc, v), call(nc.enc, v)
		if outcomeText(o) ~= outcomeText(n) then
			fail("W1", "the encoders disagree", v, o, n)
			continue
		end
		if not o.ok or typeof(o.a) ~= "buffer" then
			result.rejected += 1
			continue
		end
		local oldBuf, newBuf, blobs = o.a, n.a, o.b

		-- W2
		local oo, on = call(oc.dec, oldBuf, blobs), call(oc.dec, newBuf, n.b)
		local no, nn = call(nc.dec, oldBuf, blobs), call(nc.dec, newBuf, n.b)
		local too, tnn = outcomeText(oo), outcomeText(nn)
		if too ~= outcomeText(on) then
			fail("W2", "the old decoder reads the two buffers differently", v, oo, on)
		elseif tnn ~= outcomeText(no) then
			fail("W2", "the new decoder reads the two buffers differently", v, no, nn)
		elseif too ~= tnn then
			fail("W2", "the decoders disagree", v, oo, nn)
		elseif oo.ok and canon(oo.a) ~= canon(if oc.list then withoutCount(v) else v) then
			result.lossy += 1
			if options.debug and result.lossy <= 2 then
				print("LOSSY", pair.path, show(v), "->", show(oo.a))
			end
		end

		if w4Done < options.w4 then
			w4Done += 1
			local len = buffer.len(oldBuf)
			local malformed = {}
			local function cut(n: number)
				local b = buffer.create(n)
				buffer.copy(b, 0, oldBuf, 0, n)
				table.insert(malformed, b)
			end
			if len <= 256 then
				for i = 0, len - 1 do
					cut(i)
				end
			else
				for i = 0, 63 do
					cut(i)
					cut(len - 64 + i)
				end
				for _ = 1, 128 do
					cut(r.int(64, len - 65))
				end
			end
			if len > 0 then
				for _ = 1, 64 do
					local b = buffer.create(len)
					buffer.copy(b, 0, oldBuf)
					local at = r.int(0, len - 1)
					buffer.writeu8(b, at, bit32.bxor(buffer.readu8(b, at), r.int(1, 255)))
					table.insert(malformed, b)
				end
			end
			local appended = buffer.create(len + 1)
			buffer.copy(appended, 0, oldBuf)
			buffer.writeu8(appended, len, r.int(0, 255))
			table.insert(malformed, appended)
			for _, m in malformed do
				result.malformed += 1
				local mo, mn = call(oc.dec, m, blobs), call(nc.dec, m, blobs)
				if outcomeText(mo) ~= outcomeText(mn) then
					fail("W4", "the decoders read a malformed buffer differently", m, mo, mn)
				end
			end

			-- W5
			if oc.version ~= nil then
				if len < 5 or buffer.readu8(oldBuf, 0) ~= oc.version then
					fail("W5", "the buffer does not start with the version", oldBuf)
				end
				local headers = {}
				for i = 0, math.min(len, 5) - 1 do
					local b = buffer.create(i)
					buffer.copy(b, 0, oldBuf, 0, i)
					table.insert(headers, b)
				end
				for i = 0, math.min(len, 5) - 1 do
					local b = buffer.create(len)
					buffer.copy(b, 0, oldBuf)
					buffer.writeu8(b, i, if i == 0 then (oc.version + 1) % 256 else bit32.bxor(buffer.readu8(b, i), 255))
					table.insert(headers, b)
				end
				for _, h in headers do
					result.headers += 1
					local ho, hn = call(oc.dec, h, blobs), call(nc.dec, h, blobs)
					if outcomeText(ho) ~= outcomeText(hn) then
						fail("W5", "the decoders read a wrong header differently", h, ho, hn)
					elseif ho.ok then
						fail("W5", "a wrong header was read", h, ho, hn)
					end
				end
			end
		end
	end

	-- W3
	for k = 1, options.invalid do
		local kind = Values.MUTATIONS[(k - 1) % #Values.MUTATIONS + 1]
		local ok, base = pcall(Values.random, ctx, root, rest)
		if ok then
			local invalid, made = Values.invalid(r, root, base, kind)
			if made then
				invalid = asArguments(invalid)
				result.invalid += 1
				local o, n = call(oc.enc, invalid), call(nc.enc, invalid)
				if outcomeText(o) ~= outcomeText(n) then
					fail("W3", `the encoders treat an invalid value ({kind}) differently`, invalid, o, n)
				end
			end
		end
	end
	result.status = "ok"
	return result
end

local results = {}
for i, pair in job.pairs do
	print(`pair {i}/{#job.pairs} {pair.path}`)
	local ok, result = pcall(runPair, pair)
	if not ok then
		result = { path = pair.path, status = "crashed", reason = show(tostring(result)), failures = {}, examples = {} }
	end
	table.insert(results, result)
end
fs.writeFile(resultsPath, serde.encode("json", results))
print("done")
