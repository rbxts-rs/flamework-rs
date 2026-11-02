// Compares a Flamework build with a reference build of the same project, file by file and byte for byte: every file
// under out/, plus flamework.build and include/flamework/{globs,paths,config}.json.
//
// usage: node tools/compare.cjs <ours dir> <ref dir> [--normalize] [--diff N] [--report FILE]
//   <ours dir>, <ref dir>: project roots (each holds out/ and, when written, flamework.build and include/flamework).
//   --normalize: Luau files (.lua, .luau) that differ are compared again with their locals renamed to canonical
//     names (tools/normalize.cjs); those that are equal then count as identical, and as "names only". The
//     differences shown and reported are those of the normalised texts.
//   --diff N: print the first N differing files around their first difference.
//   --report FILE: write the summary and every difference's first differing line to FILE.
// Prints `identical A, differing B, only ours C, only ref D` (with --normalize, then `, names only E`: how many of the
// identical files differ in their locals' names only), then the files only one side has.
// Also a module: `compare(ours, ref, { normalize, diff, report })` returns the lists (identical, namesOnly,
// differing, onlyOurs, onlyRef) and the summary line.
"use strict";
const fs = require("fs");
const path = require("path");
const { normalize: normalizeText } = require("./normalize.cjs");

const EXTRAS = ["flamework.build", "include/flamework/globs.json", "include/flamework/paths.json", "include/flamework/config.json"];

function walk(dir, base = dir, out = []) {
	if (!fs.existsSync(dir)) return out;
	for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
		const p = path.join(dir, e.name);
		if (e.isDirectory()) walk(p, base, out);
		else out.push(path.relative(base, p).replace(/\\/g, "/"));
	}
	return out;
}

function compare(ours, ref, { normalize = false, diff = 0, report, log = console.log } = {}) {
	const files = new Set([
		...walk(path.join(ours, "out")).map((f) => "out/" + f),
		...walk(path.join(ref, "out")).map((f) => "out/" + f),
	]);
	for (const extra of EXTRAS) {
		if (fs.existsSync(path.join(ours, extra)) || fs.existsSync(path.join(ref, extra))) files.add(extra);
	}

	const result = { identical: [], namesOnly: [], differing: [], onlyOurs: [], onlyRef: [] };
	const lines = [];
	let shown = 0;
	for (const file of [...files].sort()) {
		const a = path.join(ours, file);
		const b = path.join(ref, file);
		if (!fs.existsSync(a)) {
			result.onlyRef.push(file);
			continue;
		}
		if (!fs.existsSync(b)) {
			result.onlyOurs.push(file);
			continue;
		}
		const ba = fs.readFileSync(a);
		const bb = fs.readFileSync(b);
		if (ba.equals(bb)) {
			result.identical.push(file);
			continue;
		}
		// Decoded only to normalise and to show where the files differ.
		let ta = ba.toString("utf8");
		let tb = bb.toString("utf8");
		if (normalize && /\.luau?$/.test(file)) {
			ta = normalizeText(ta);
			tb = normalizeText(tb);
			if (ta === tb) {
				result.identical.push(file);
				result.namesOnly.push(file);
				continue;
			}
		}
		result.differing.push(file);
		const la = ta.split("\n");
		const lb = tb.split("\n");
		let i = 0;
		while (i < la.length && i < lb.length && la[i] === lb[i]) i++;
		lines.push(`${file}:${i + 1}\n  ours: ${(la[i] ?? "<eof>").trim().slice(0, 220)}\n  ref:  ${(lb[i] ?? "<eof>").trim().slice(0, 220)}`);
		if (shown < diff) {
			shown++;
			log(`---- ${file} (first difference at line ${i + 1})`);
			for (let j = Math.max(0, i - 2); j < Math.min(Math.max(la.length, lb.length), i + 6); j++) {
				if (la[j] !== lb[j]) {
					log(`- ref : ${lb[j] ?? "<eof>"}`);
					log(`+ ours: ${la[j] ?? "<eof>"}`);
				} else log(`        ${la[j]}`);
			}
		}
	}
	const summary =
		`identical ${result.identical.length}, differing ${result.differing.length}, only ours ${result.onlyOurs.length}, only ref ${result.onlyRef.length}` +
		(normalize ? `, names only ${result.namesOnly.length}` : "");
	if (report) {
		fs.writeFileSync(
			report,
			[summary, ...lines, ...result.onlyOurs.map((f) => "only ours: " + f), ...result.onlyRef.map((f) => "only ref: " + f)].join("\n") + "\n",
		);
	}
	return { ...result, summary };
}

function option(args, name) {
	const i = args.indexOf(name);
	return i >= 0 ? args[i + 1] : undefined;
}

if (require.main === module) {
	const [ours, ref, ...rest] = process.argv.slice(2);
	if (!ours || !ref) {
		console.error("usage: node tools/compare.cjs <ours dir> <ref dir> [--normalize] [--diff N] [--report FILE]");
		process.exit(2);
	}
	const result = compare(ours, ref, {
		normalize: rest.includes("--normalize"),
		diff: Number(option(rest, "--diff") ?? 0),
		report: option(rest, "--report"),
	});
	console.log(result.summary);
	for (const f of result.onlyOurs) console.log(`only ours: ${f}`);
	for (const f of result.onlyRef) console.log(`only ref: ${f}`);
}

module.exports = { compare };
