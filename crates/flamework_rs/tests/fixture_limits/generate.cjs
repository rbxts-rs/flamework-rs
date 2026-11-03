// Writes tests/fixture_limits/src (DESIGN §5.2, "Limit cases", the serializer's): types wide enough to take a
// generated function past Luau's limits, for `tests/limits.rs`. Run `node generate.cjs` here after changing it.
//   strings.ts, bytes.ts, vectors.ts: `Wide<N>` with N string / u8 / Vector3 fields, N in WIDTHS;
//   unions.ts: unions of N object members, N in MEMBERS.
"use strict";
const fs = require("fs");
const path = require("path");

const WIDTHS = [40, 60, 80, 100, 110, 150, 250];
const MEMBERS = [50, 100, 150, 200, 250];
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

const src = path.join(__dirname, "src");
fs.mkdirSync(src, { recursive: true });
for (const [file, text] of [
	wide("strings.ts", "String fields: variable sizes, a running size and a cursor.", "string"),
	wide("bytes.ts", "u8 fields: a fixed layout.", "Serialization.u8"),
	wide("vectors.ts", "Vector3 fields: three reads and a constructor each.", "Vector3"),
	unions(),
]) {
	fs.writeFileSync(path.join(src, file), text);
}
console.log(`wrote ${src}`);
