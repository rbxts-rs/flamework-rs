// Writes tests/fixture_limits/src (DESIGN §5.2, "Limit cases"): types wide enough to take a generated function past
// Luau's limits, and sends whose packing lands among many live locals, for `tests/limits.rs`. Run `node generate.cjs`
// here after changing it.
//   strings.ts, bytes.ts, vectors.ts: `Wide<N>` with N string / u8 / Vector3 fields, N in WIDTHS;
//   unions.ts: unions of N object members, N in MEMBERS;
//   network.ts: events of N string arguments, N in ARGUMENTS;
//   sends.ts: each event sent as a whole statement from functions already holding H locals, H in HELD, and the same
//   inside three nested loops (whose variables count in H: `_` and `a` of the `for … of`, `i`); returned sends; and a
//   send among 199 live locals, which leaves its packing no room there;
//   slots.ts: more file slots than Luau's 200 locals in one main chunk (literal lists and indexes, guards telling
//   union members apart, enum tables, dedup guards; flamework.config.json's dedup limit is 2), which the compiler
//   spills into one table, and a whole-statement send after them.
"use strict";
const fs = require("fs");
const path = require("path");

const WIDTHS = [40, 60, 80, 100, 110, 150, 250];
const MEMBERS = [50, 100, 150, 200, 250];
const ARGUMENTS = [100, 150, 200, 250];
const HELD = [0, 100, 170, 195];
const SLOTS = { literals: 80, unions: 40, dedup: 40 };
const HEADER = 'import { Flamework, Serialization } from "@flamework-experimental/core";\n';

function wide(file, about, field) {
	let text = `${HEADER}\n/** ${about} */\n`;
	for (const n of WIDTHS) {
		const fields = Array.from({ length: n }, (_, i) => `\tf${i}: ${field};\n`).join("");
		text += `\ninterface Wide${n} {\n${fields}}\nexport const wide${n} = Flamework.createSerializer<Wide${n}>();\n`;
	}
	return [file, text];
}

function unions() {
	let text = `${HEADER}\n/** Unions of object members told apart by a literal field. */\n`;
	for (const n of MEMBERS) {
		const members = Array.from({ length: n }, (_, i) => `\t| { kind: "m${i}"; a: string; b: Serialization.u8 }\n`).join("");
		text += `\ntype Union${n} =\n${members};\nexport const union${n} = Flamework.createSerializer<Union${n}>();\n`;
	}
	return ["unions.ts", text];
}

function network() {
	let text = 'import { Networking } from "@flamework-experimental/networking";\n\n/** Events of N string arguments. */\n';
	text += "export interface ServerEvents {\n";
	for (const n of ARGUMENTS) {
		const parameters = Array.from({ length: n }, (_, i) => `\t\ta${i}: string,\n`).join("");
		text += `\tw${n}(\n${parameters}\t): void;\n`;
	}
	text += "}\n\nexport interface ClientEvents {}\n\n";
	text += "export const GlobalEvents = Networking.createEvent<ServerEvents, ClientEvents>();\n";
	text += "export const Events = GlobalEvents.createClient({});\n";
	return ["network.ts", text];
}

function sends() {
	let text = 'import { Events } from "./network";\n\ndeclare function compute(): number;\ndeclare const text: string;\n';
	text += "declare const list: number[];\ndeclare const count: number;\n";
	text += "\n/** Whole-statement sends from functions already holding H locals, plainly and in three nested loops. */\n";
	const held = (h) => Array.from({ length: h }, (_, i) => `\tconst l${i} = compute();\n`).join("");
	const send = (n, indent) => `${indent}Events.w${n}.fire(${Array.from({ length: n }, () => "text").join(", ")});\n`;
	for (const n of ARGUMENTS) {
		for (const h of HELD) {
			text += `\nexport function send${n}held${h}() {\n${held(h)}${send(n, "\t")}}\n`;
			text += `\nexport function send${n}held${h}loops() {\n${held(Math.max(0, h - 3))}`;
			text += "\tfor (const a of list) {\n\t\tfor (let i = 0; i < count; i++) {\n\t\t\twhile (compute() > a + i) {\n";
			text += send(n, "\t\t\t\t");
			text += "\t\t\t}\n\t\t}\n\t}\n}\n";
		}
		text += `\nexport function return${n}held195() {\n${held(195)}\treturn ${send(n, "").trimEnd().slice(0, -1)};\n}\n`;
	}
	text += `\nexport function send100held199() {\n${held(199)}${send(100, "\t")}}\n`;
	return ["sends.ts", text];
}

function slots() {
	let text = 'import { Flamework } from "@flamework-experimental/core";\nimport { Events } from "./network";\n';
	text += "\ndeclare const text: string;\n";
	text += "\n/** Literal groups: a list and an index each. */\nexport const literals = [\n";
	for (let i = 0; i < SLOTS.literals; i++) text += `\tFlamework.createSerializer<"a${i}" | "b${i}" | "c${i}">(),\n`;
	text += "];\n\n/** Unions told apart by a guard. */\nexport const unions = [\n";
	for (let i = 0; i < SLOTS.unions; i++) {
		text += `\tFlamework.createSerializer<Partial<{ p${i}: number }> | Partial<{ q${i}: string }>>(),\n`;
	}
	text += "];\n\n/** Enum tables. */\nexport const enums = [\n";
	for (const name of ["Material", "KeyCode", "UserInputType", "Font", "PartType"]) {
		text += `\tFlamework.createSerializer<Enum.${name}>(),\n`;
	}
	text += "];\n\n/** Dedup guards in the main chunk: each interface met twice. */\n";
	for (let i = 0; i < SLOTS.dedup; i++) text += `interface D${i} {\n\tx${i}: number;\n}\n`;
	text += "export const guards = [\n";
	for (let i = 0; i < SLOTS.dedup; i++) text += `\tFlamework.createGuard<{ a: D${i}; b: D${i} }>(),\n`;
	text += "];\n\n/** A whole-statement send in the main chunk, after them. */\n";
	text += `Events.w100.fire(${Array.from({ length: 100 }, () => "text").join(", ")});\n`;
	return ["slots.ts", text];
}

const src = path.join(__dirname, "src");
fs.mkdirSync(src, { recursive: true });
for (const [file, text] of [
	wide("strings.ts", "String fields: variable sizes, a running size and a cursor.", "string"),
	wide("bytes.ts", "u8 fields: a fixed layout.", "Serialization.u8"),
	wide("vectors.ts", "Vector3 fields: three reads and a constructor each.", "Vector3"),
	unions(),
	network(),
	sends(),
	slots(),
]) {
	fs.writeFileSync(path.join(src, file), text);
}
console.log(`wrote ${src}`);
