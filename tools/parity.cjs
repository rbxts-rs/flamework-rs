// Flamework parity on a whole project: builds a copy of the project with `rbxtsc-fw` and compares the output with a
// reference build of it (TypeScript 5 roblox-ts with Flamework's TS transformer), file by file (tools/compare.cjs).
//
// usage: node tools/parity.cjs <rbxtsc-fw exe> <project dir> <reference out dir> <work dir> [options] [-- build args]
//   <project dir>: the project to build. It is copied to <work dir>/proj first, without its top-level `out` and
//     `include` and without any `*buildinfo` file, so the build is clean; links (symbolic links, junctions, such as
//     a linked `node_modules` package) are recreated as junctions to the same targets, not copied.
//   <reference out dir>: the reference build's `out` folder. It is linked as <work dir>/ref/out.
//   <work dir>: scratch space, emptied first and removed at the end (unless --keep).
// options:
//   --ref-extras DIR   a folder holding the reference's flamework.build and include/flamework, linked in beside its out;
//                      without it those files count as "only ours".
//   --save-out DIR     copy the build's out folder to DIR (replacing it).
//   --report FILE      write every difference's first differing line (tools/compare.cjs --report).
//   --diff N           print the first N differing files around their first difference.
//   --timeout SECONDS  the build's time limit (default 600).
//   --tsconfig FILE    the project's tsconfig to build, relative to the project (default: its tsconfig.json), such as a
//                      TypeScript 7 variant beside a TypeScript 5 project's own.
//   --keep             keep <work dir>.
//   -- ARGS            extra arguments for `rbxtsc-fw build -p <project>`.
// Prints `build exit N`, then `identical A, differing B, only ours C, only ref D`. Exits 0 when the build exited 0,
// 1 when it failed (the comparison still runs), 2 on a usage error.
"use strict";
const fs = require("fs");
const path = require("path");
const cp = require("child_process");
const { compare } = require("./compare.cjs");

const argv = process.argv.slice(2);
const dash = argv.indexOf("--");
const args = dash >= 0 ? argv.slice(0, dash) : argv;
const buildArgs = dash >= 0 ? argv.slice(dash + 1) : [];
const flags = new Set(["--keep"]);
const options = {};
const positional = [];
for (let i = 0; i < args.length; i++) {
	if (flags.has(args[i])) options[args[i]] = true;
	else if (args[i].startsWith("--")) options[args[i]] = args[++i];
	else positional.push(args[i]);
}
const [exe, project, refOut, work] = positional;
if (!exe || !project || !refOut || !work || positional.length > 4) {
	console.error("usage: node tools/parity.cjs <rbxtsc-fw exe> <project dir> <reference out dir> <work dir> [options] [-- build args]");
	process.exit(2);
}
for (const [what, p] of [["exe", exe], ["project", project], ["reference out", refOut]]) {
	if (!fs.existsSync(p)) {
		console.error(`parity: the ${what} ${p} does not exist`);
		process.exit(2);
	}
}

// The work dir is emptied: refuse one that holds, or lies inside, the project or the reference.
function inside(child, parent) {
	const relative = path.relative(path.resolve(parent), path.resolve(child));
	return relative === "" || (!relative.startsWith("..") && !path.isAbsolute(relative));
}
for (const p of [project, refOut, options["--ref-extras"]].filter(Boolean)) {
	if (inside(p, work) || inside(work, p)) {
		console.error(`parity: the work dir ${work} must not hold or lie inside ${p}`);
		process.exit(2);
	}
}

// Links are removed as links, so emptying a folder never reaches into what a junction points at.
function unlinkLinks(dir) {
	let entries;
	try {
		entries = fs.readdirSync(dir, { withFileTypes: true });
	} catch {
		return;
	}
	for (const e of entries) {
		const p = path.join(dir, e.name);
		if (e.isSymbolicLink()) {
			try {
				fs.rmdirSync(p);
			} catch {
				fs.unlinkSync(p);
			}
		} else if (e.isDirectory()) unlinkLinks(p);
	}
}
function rm(p) {
	unlinkLinks(p);
	fs.rmSync(p, { recursive: true, force: true, maxRetries: 5 });
}
function link(target, at) {
	fs.symlinkSync(path.resolve(target), at, "junction");
}
function copy(from, to, skipTop = []) {
	fs.mkdirSync(to, { recursive: true });
	for (const e of fs.readdirSync(from, { withFileTypes: true })) {
		if (skipTop.includes(e.name) || e.name.endsWith("buildinfo")) continue;
		const a = path.join(from, e.name);
		const b = path.join(to, e.name);
		if (e.isSymbolicLink()) link(path.resolve(from, fs.readlinkSync(a)), b);
		else if (e.isDirectory()) copy(a, b);
		else fs.copyFileSync(a, b);
	}
}

const proj = path.join(work, "proj");
const ref = path.join(work, "ref");
rm(work);
copy(project, proj, ["out", "include"]);
fs.mkdirSync(ref, { recursive: true });
link(refOut, path.join(ref, "out"));
if (options["--ref-extras"]) {
	const extras = options["--ref-extras"];
	if (fs.existsSync(path.join(extras, "flamework.build"))) fs.copyFileSync(path.join(extras, "flamework.build"), path.join(ref, "flamework.build"));
	if (fs.existsSync(path.join(extras, "include", "flamework"))) {
		fs.mkdirSync(path.join(ref, "include"), { recursive: true });
		link(path.join(extras, "include", "flamework"), path.join(ref, "include", "flamework"));
	}
}

const timeout = Number(options["--timeout"] ?? 600) * 1000;
const build = cp.spawnSync(path.resolve(exe), ["build", "-p", options["--tsconfig"] ?? ".", ...buildArgs], { cwd: proj, encoding: "utf8", timeout, maxBuffer: 1 << 28 });
if (build.error) console.log(`build error: ${build.error.message}`);
console.log(`build exit ${build.status}`);
if (build.status !== 0) {
	const output = `${build.stdout ?? ""}${build.stderr ?? ""}`.split("\n");
	console.log(output.slice(0, 40).join("\n"));
}

const result = compare(proj, ref, { diff: Number(options["--diff"] ?? 0), report: options["--report"] });
console.log(result.summary);

if (options["--save-out"]) {
	rm(options["--save-out"]);
	if (fs.existsSync(path.join(proj, "out"))) copy(path.join(proj, "out"), options["--save-out"]);
}
if (!options["--keep"]) rm(work);
process.exit(build.status === 0 ? 0 : 1);
