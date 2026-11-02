// The wire-diff driver: pairs two `wire-harvest` folders (an old build's and a new build's of the same project), runs
// the Lune checks on every pair (wirediff.luau, split between workers) and prints a summary. See README.md.
//
// usage: node tools/wirediff/wirediff.cjs <old harvest dir> <new harvest dir> [options]
// options:
//   --seed N          the run's seed (default 1); each pair's values are seeded from it and the pair's path.
//   --random N        random values per pair (default 200), besides every edge value.
//   --w4 N            values per pair whose buffer is also malformed (W4) and, if versioned, given wrong headers
//                     (W5) (default 32).
//   --invalid N       invalid values per pair (W3, default 40).
//   --realm R         Server or Client: what `RunService` answers, for side-limited checks (default Server).
//   --jobs N          Lune workers (default 4).
//   --timeout S       one worker's time limit in seconds (default 1800).
//   --examples N      failures shown per check (default 5).
//   --redact          print counts only: no paths, values or messages (for a private corpus).
//   --json FILE       write every pair's result there.
//   --lune PATH       the lune executable (default: lune on PATH).
// Prints the pairs (and modules only one side has), values, and per check the failures with the first examples.
// Exits 0 with no failure, 1 with failures (or a pair only one side has, or a worker that died), 2 on a usage error.
"use strict";
const fs = require("fs");
const os = require("os");
const path = require("path");
const cp = require("child_process");

const argv = process.argv.slice(2);
const flags = new Set(["--redact"]);
const options = {};
const positional = [];
for (let i = 0; i < argv.length; i++) {
	if (flags.has(argv[i])) options[argv[i]] = true;
	else if (argv[i].startsWith("--")) options[argv[i]] = argv[++i];
	else positional.push(argv[i]);
}
const [oldDir, newDir] = positional;
if (!oldDir || !newDir || positional.length > 2) {
	console.error("usage: node tools/wirediff/wirediff.cjs <old harvest dir> <new harvest dir> [options]");
	process.exit(2);
}
for (const dir of [oldDir, newDir]) {
	if (!fs.existsSync(dir) || !fs.statSync(dir).isDirectory()) {
		console.error(`wirediff: ${dir} is not a folder`);
		process.exit(2);
	}
}
const number = (name, fallback) => (options[name] === undefined ? fallback : Number(options[name]));
const seed = number("--seed", 1);
const jobs = Math.max(1, number("--jobs", 4));
const timeout = number("--timeout", 1800);
const shown = number("--examples", 5);
const redact = !!options["--redact"];
const lune = options["--lune"] || "lune";
const tool = __dirname;

function modules(root) {
	const found = [];
	const walk = (dir) => {
		for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
			const p = path.join(dir, e.name);
			if (e.isDirectory()) walk(p);
			else if (e.name.endsWith(".luau")) found.push(path.relative(root, p).split(path.sep).join("/"));
		}
	};
	walk(root);
	return found;
}

const oldModules = new Set(modules(oldDir));
const newModules = new Set(modules(newDir));
const pairs = [...oldModules].filter((m) => newModules.has(m)).sort();
const onlyOld = [...oldModules].filter((m) => !newModules.has(m)).sort();
const onlyNew = [...newModules].filter((m) => !oldModules.has(m)).sort();

// Workers get pairs round-robin by size, so the big modules spread.
const sized = pairs.map((p) => ({ path: p, size: fs.statSync(path.join(oldDir, p)).size }));
sized.sort((a, b) => b.size - a.size);
const chunks = Array.from({ length: Math.min(jobs, Math.max(sized.length, 1)) }, () => []);
sized.forEach((p, i) => chunks[i % chunks.length].push(p.path));

const work = fs.mkdtempSync(path.join(os.tmpdir(), "wirediff-"));
const jobOptions = {
	seed,
	random: number("--random", 200),
	w4: number("--w4", 32),
	invalid: number("--invalid", 40),
	realm: options["--realm"] || "Server",
	t: path.join(tool, "vendor", "t.lua"),
};

const started = Date.now();
const runs = chunks.map((chunk, i) => {
	const jobFile = path.join(work, `job${i}.json`);
	const resultFile = path.join(work, `results${i}.json`);
	const job = {
		options: jobOptions,
		pairs: chunk.map((p) => ({ path: p, old: path.resolve(oldDir, p), new: path.resolve(newDir, p) })),
	};
	fs.writeFileSync(jobFile, JSON.stringify(job));
	return new Promise((resolve) => {
		const child = cp.spawn(lune, ["run", path.join(tool, "wirediff.luau"), jobFile, resultFile], {
			cwd: tool,
			stdio: ["ignore", "pipe", "pipe"],
		});
		let last = "";
		let stderr = "";
		child.stdout.on("data", (d) => {
			const lines = d.toString().split(/\r?\n/).filter((l) => l.startsWith("pair "));
			if (lines.length) last = lines[lines.length - 1];
		});
		child.stderr.on("data", (d) => (stderr += d.toString()));
		const timer = setTimeout(() => child.kill(), timeout * 1000);
		child.on("close", (code) => {
			clearTimeout(timer);
			let results = null;
			try {
				results = JSON.parse(fs.readFileSync(resultFile, "utf8"));
			} catch {}
			resolve({ index: i, code, results, last, stderr, pairs: chunk.length });
		});
	});
});

Promise.all(runs).then((done) => {
	const seconds = ((Date.now() - started) / 1000).toFixed(1);
	const results = [];
	let failed = onlyOld.length + onlyNew.length > 0;
	for (const run of done) {
		if (!run.results) {
			failed = true;
			// The pair that was running when the worker died (its line was printed before it started).
			const at = redact ? run.last.replace(/^(pair \d+\/\d+).*/, "$1") : run.last;
			console.log(`worker ${run.index} died (exit ${run.code}) after ${at || "no pair"}`);
			if (!redact && run.stderr) console.log(run.stderr.split("\n").slice(-6).join("\n"));
			continue;
		}
		results.push(...run.results);
	}
	const checks = ["W0", "W1", "W2", "W3", "W4", "W5"];
	const total = { values: 0, rejected: 0, invalid: 0, malformed: 0, headers: 0, lossy: 0 };
	const failures = Object.fromEntries(checks.map((c) => [c, { pairs: 0, cases: 0, examples: [] }]));
	const statuses = {};
	const kinds = {};
	for (const r of results) {
		statuses[r.status] = (statuses[r.status] || 0) + 1;
		if (r.kind) kinds[r.kind] = (kinds[r.kind] || 0) + 1;
		for (const k of Object.keys(total)) total[k] += r[k] || 0;
		for (const c of checks) {
			const n = (r.failures || {})[c] || 0;
			if (n > 0) {
				failures[c].pairs++;
				failures[c].cases += n;
			}
		}
		for (const e of Array.isArray(r.examples) ? r.examples : []) {
			if (failures[e.check].examples.length < shown) failures[e.check].examples.push({ path: r.path, ...e });
		}
	}
	const list = (o) => Object.entries(o).map(([k, v]) => `${k} ${v}`).join(", ");
	console.log(`pairs ${pairs.length} (${list(kinds)}), only old ${onlyOld.length}, only new ${onlyNew.length}; ${seconds} s, ${chunks.length} workers`);
	console.log(`status: ${list(statuses)}`);
	console.log(`values ${total.values} (refused by the encoder ${total.rejected}, lossy round trips ${total.lossy}), invalid ${total.invalid}, malformed ${total.malformed}, wrong headers ${total.headers}`);
	for (const c of checks) {
		const f = failures[c];
		console.log(`${c}: ${f.cases} failures in ${f.pairs} pairs`);
		if (f.cases > 0) failed = true;
		if (redact) continue;
		for (const e of f.examples) {
			console.log(`  ${e.path}: ${e.what}`);
			if (e.value !== undefined) console.log(`    value ${e.value}`);
			if (e.old !== undefined) console.log(`    old   ${e.old}`);
			if (e.new !== undefined) console.log(`    new   ${e.new}`);
		}
	}
	if (!redact) {
		for (const m of onlyOld.slice(0, shown)) console.log(`only old: ${m}`);
		for (const m of onlyNew.slice(0, shown)) console.log(`only new: ${m}`);
		const odd = results.filter((r) => r.status !== "ok");
		for (const r of odd.slice(0, shown)) console.log(`${r.status}: ${r.path}${r.reason ? ` (${r.reason})` : ""}`);
	}
	if (options["--json"]) fs.writeFileSync(options["--json"], JSON.stringify(results, null, "\t"));
	fs.rmSync(work, { recursive: true, force: true });
	process.exit(failed ? 1 : 0);
});
