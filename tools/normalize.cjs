// Renames the locals of a Luau file to canonical names, so that two builds whose generated code differs only in the
// names its locals got (`buf` / `buf_1`, `_exp` / `_exp_2`, `value_3` / `value_1`) compare equal.
//
// usage: node tools/normalize.cjs <file.luau>   prints the file normalised.
// Also a module: `normalize(text)` returns the text normalised.
//
// How: every identifier is renamed, except Luau's keywords, field names (after `.` or `:`) and the keys of a table
// constructor (`{ key = ... }`); strings and comments are left as they are. An identifier's class is its name without
// a trailing number (`buf_12` and `buf` are `buf`, `_exp2` is `_exp`), and it becomes `<class>#<n>`, where n counts
// the distinct names of that class in the order they first occur in the file (the first `buf`-class name seen is
// `buf#0` wherever it appears, the second `buf#1`, ...). So two files normalise to the same text exactly when one is
// the other with names consistently swapped for names of the same class: a constant, a call, the order of the code,
// or which local a use reads (`buf` where the other reads `buf_1`) still differ. The whole file is one scope, which is
// stricter than Luau's scoping: a name reused in two functions of one build and kept apart in the other differs.
// One exception: the names of the `_` class (`_`, `_1`, ...) all become `_`. They name what is never read (a loop's
// counter, a skipped value), and the compiler's own `_` may share its name with a generated one in one build only.
"use strict";

const KEYWORDS = new Set([
	"and", "break", "continue", "do", "else", "elseif", "end", "export", "false", "for", "function", "if", "in", "local",
	"nil", "not", "or", "repeat", "return", "then", "true", "type", "typeof", "until", "while",
]);

// The file as tokens: `{ kind, text }` with kind `id`, `space` (whitespace and comments), `str`, `num` or `op`. An
// interpolated string's expressions are tokens of their own between `str` pieces.
function tokenize(text) {
	const tokens = [];
	let i = 0;
	const n = text.length;
	const longBracket = (at) => {
		// `[[`, `[=[`, ...: the level, or -1.
		if (text[at] !== "[") return -1;
		let j = at + 1;
		while (text[j] === "=") j++;
		return text[j] === "[" ? j - at - 1 : -1;
	};
	const closeLong = (from, level) => {
		const close = "]" + "=".repeat(level) + "]";
		const end = text.indexOf(close, from);
		return end < 0 ? n : end + close.length;
	};
	const quoted = (from, quote) => {
		let j = from + 1;
		while (j < n && text[j] !== quote && text[j] !== "\n") {
			if (text[j] === "\\") j++;
			j++;
		}
		return Math.min(j + 1, n);
	};
	// Inside an interpolated string, the depth of `{` at which each one started.
	const interpolations = [];
	let braces = 0;
	const interpolatedPiece = (from) => {
		// From just after "`" or "}": text up to the next unescaped "{" or "`".
		let j = from;
		while (j < n && text[j] !== "`" && text[j] !== "{") {
			if (text[j] === "\\") j++;
			j++;
		}
		return j;
	};
	while (i < n) {
		const c = text[i];
		if (/\s/.test(c)) {
			let j = i;
			while (j < n && /\s/.test(text[j])) j++;
			tokens.push({ kind: "space", text: text.slice(i, j) });
			i = j;
		} else if (text.startsWith("--", i)) {
			const level = longBracket(i + 2);
			let j;
			if (level >= 0) j = closeLong(i + 2, level);
			else {
				j = text.indexOf("\n", i);
				if (j < 0) j = n;
			}
			tokens.push({ kind: "space", text: text.slice(i, j) });
			i = j;
		} else if (c === '"' || c === "'") {
			const j = quoted(i, c);
			tokens.push({ kind: "str", text: text.slice(i, j) });
			i = j;
		} else if (c === "[" && longBracket(i) >= 0) {
			const j = closeLong(i, longBracket(i));
			tokens.push({ kind: "str", text: text.slice(i, j) });
			i = j;
		} else if (c === "`" || (c === "}" && interpolations.length > 0 && interpolations[interpolations.length - 1] === braces)) {
			// The start of an interpolated string, or the end of one of its expressions.
			if (c === "}") interpolations.pop();
			const j = interpolatedPiece(i + 1);
			if (j < n && text[j] === "{") {
				interpolations.push(braces);
				tokens.push({ kind: "str", text: text.slice(i, j + 1) });
				i = j + 1;
			} else {
				tokens.push({ kind: "str", text: text.slice(i, Math.min(j + 1, n)) });
				i = Math.min(j + 1, n);
			}
		} else if (/[A-Za-z_]/.test(c)) {
			let j = i + 1;
			while (j < n && /[A-Za-z0-9_]/.test(text[j])) j++;
			tokens.push({ kind: "id", text: text.slice(i, j) });
			i = j;
		} else if (/[0-9]/.test(c) || (c === "." && /[0-9]/.test(text[i + 1] ?? ""))) {
			let j = i + 1;
			while (j < n && (/[0-9A-Za-z_.]/.test(text[j]) || ((text[j] === "+" || text[j] === "-") && /[eE]/.test(text[j - 1]) && !/^0[xX]/.test(text.slice(i, j))))) j++;
			tokens.push({ kind: "num", text: text.slice(i, j) });
			i = j;
		} else {
			const three = text.slice(i, i + 3);
			const two = text.slice(i, i + 2);
			let op = c;
			if (three === "..." || three === "..=" || three === "//=") op = three;
			else if (["..", "==", "~=", "<=", ">=", "+=", "-=", "*=", "/=", "%=", "^=", "::", "->", "//"].includes(two)) op = two;
			if (op === "{") braces++;
			else if (op === "}") braces--;
			tokens.push({ kind: "op", text: op });
			i += op.length;
		}
	}
	return tokens;
}

// The file's tokens, each identifier that `normalize` renames marked `local: true`.
function locals(text) {
	const tokens = tokenize(text);
	const significant = tokens.map((t, index) => index).filter((index) => tokens[index].kind !== "space");
	const stack = []; // open brackets, and blocks (`function`, `if`, `do`, `repeat`) up to their `end` / `until`
	let s = 0; // position in `significant`
	for (let index = 0; index < tokens.length; index++) {
		const token = tokens[index];
		if (token.kind === "space") continue;
		const previous = s > 0 ? tokens[significant[s - 1]] : undefined;
		const next = s + 1 < significant.length ? tokens[significant[s + 1]] : undefined;
		s++;
		if (token.kind === "op") {
			if (token.text === "{" || token.text === "(" || token.text === "[") stack.push(token.text);
			else if (token.text === "}" || token.text === ")" || token.text === "]") stack.pop();
		}
		if (token.kind === "id") {
			if (["function", "if", "do", "repeat"].includes(token.text)) stack.push(token.text);
			else if (token.text === "end" || token.text === "until") stack.pop();
		}
		if (token.kind !== "id" || KEYWORDS.has(token.text)) continue;
		const field = previous && previous.kind === "op" && (previous.text === "." || previous.text === ":");
		const key =
			stack[stack.length - 1] === "{" &&
			next && next.kind === "op" && next.text === "=" &&
			previous && previous.kind === "op" && (previous.text === "{" || previous.text === "," || previous.text === ";");
		token.local = !field && !key;
	}
	return tokens;
}

// The text with its locals renamed (see the header).
function normalize(text) {
	const classes = new Map(); // class -> Map(name -> index)
	const out = [];
	for (const token of locals(text)) {
		if (!token.local) {
			out.push(token.text);
			continue;
		}
		const name = token.text;
		const cls = name.replace(/_?[0-9]+$/, "") || "_";
		if (cls === "_") {
			out.push("_");
			continue;
		}
		let names = classes.get(cls);
		if (!names) classes.set(cls, (names = new Map()));
		if (!names.has(name)) names.set(name, names.size);
		out.push(`${cls}#${names.get(name)}`);
	}
	return out.join("");
}

if (require.main === module) {
	const file = process.argv[2];
	if (!file) {
		console.error("usage: node tools/normalize.cjs <file.luau>");
		process.exit(2);
	}
	process.stdout.write(normalize(require("fs").readFileSync(file, "utf8")));
}

module.exports = { normalize, locals, tokenize };
