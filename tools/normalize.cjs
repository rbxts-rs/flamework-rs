// Renames the generated locals of a Luau file to canonical names, so that two builds whose generated code differs
// only in the names its generated locals got (`buf` / `buf_1`, `_exp` / `_exp_2`, `value_3` / `value_1`) compare
// equal.
//
// usage: node tools/normalize.cjs <file.luau> [<source.ts>]   prints the file normalised.
// Also a module: `normalize(text, keep)` returns the text normalised; `sourceIdentifiers(tsText)` gives the `keep`
// set of a TS source file.
//
// What is renamed: a name the file declares (`local`, a `for` variable, a function's parameter, `local function`)
// that is not in `keep`, wherever it occurs as a name (not a field name after `.` or `:`, not a table constructor's
// key `{ key = ... }`, never inside a string or a comment). Everything else stays as written: keywords, names the file
// never declares (globals such as `Vector3`, `buffer`), and the names in `keep`. `keep` should be the identifiers of
// the file's TS source (`sourceIdentifiers`): the compiler names generated locals so that they are none of those, so
// the user's own locals (`player`) keep their names and a renamed one (`player` / `player_1`) is still reported.
// Without a source, `keep` is empty and every declared name is renamed.
//
// How: a renamed name's class is its name without a trailing number (`buf_12` and `buf` are `buf`, `_exp2` is
// `_exp`), and it becomes `<class>#<n>`, where n counts the distinct renamed names of that class in the order they
// first occur in the file (the first `buf`-class name seen is `buf#0` wherever it appears, the second `buf#1`, ...).
// One exception: a renamed name of the `_` class (`_`, `_1`, ...) that is only ever declared, never used, becomes
// `_#`: it names what is never read (a loop's counter, a skipped value), and the compiler's own `_` may share its name
// with a generated one in one build only. A `_`-class name that is used is renamed like any other.
//
// So two files normalise to the same text exactly when one is the other with its renamed names consistently swapped
// for names of the same class, and with never-used `_`-class names swapped freely: a constant, a call, the order of the
// code, a global, a name in `keep`, a key, a field, or which local a use reads (`buf` where the other reads `buf_1`)
// still differ. The whole file is one scope, which is stricter than Luau's scoping: a name reused in two functions of
// one build and kept apart in the other differs. `tests/support/expected.rs` is a port of this file (keep the two
// alike).
"use strict";

const KEYWORDS = new Set([
	"and", "break", "continue", "do", "else", "elseif", "end", "export", "false", "for", "function", "if", "in", "local",
	"nil", "not", "or", "repeat", "return", "then", "true", "type", "typeof", "until", "while",
]);

// The tokens after which `if` starts an if-expression, not an if-statement (with the `then` and `else` of an
// if-expression, marked as they are met).
const EXPRESSION_BEFORE = new Set([
	"=", "(", ",", "{", "[", "+", "-", "*", "/", "//", "%", "^", "..", "==", "~=", "<", "<=", ">", ">=",
	"+=", "-=", "*=", "/=", "//=", "%=", "^=", "..=", "return", "and", "or", "not",
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

// The file's tokens, each identifier marked `name: true` where it stands as a name (not a keyword, a field name or a
// table constructor's key), `declaration: true` where the file declares it, and `renamed: true` where `normalize`
// renames it (a name the file declares somewhere, not in `keep`).
function locals(text, keep = new Set()) {
	const tokens = tokenize(text);
	const at = tokens.map((t, index) => index).filter((index) => tokens[index].kind !== "space");
	const sig = at.map((index) => tokens[index]);
	const stack = []; // open brackets, and blocks (`function`, `if`, `do`, `repeat`) up to their `end` / `until`
	const ifExpressions = []; // open if-expressions: the stack depth they are at, and whether a `then` is due
	for (let s = 0; s < sig.length; s++) {
		const token = sig[s];
		const previous = sig[s - 1];
		const next = sig[s + 1];
		const open = ifExpressions[ifExpressions.length - 1];
		const atOpen = open !== undefined && open.depth === stack.length;
		if (token.kind === "op") {
			if (token.text === "{" || token.text === "(" || token.text === "[") stack.push(token.text);
			else if (token.text === "}" || token.text === ")" || token.text === "]") stack.pop();
		} else if (token.kind === "id") {
			if (token.text === "if") {
				const expression =
					previous !== undefined &&
					(previous.ifExpressionPart || ((previous.kind === "op" || previous.kind === "id") && EXPRESSION_BEFORE.has(previous.text)));
				if (expression) ifExpressions.push({ depth: stack.length, then: true });
				else stack.push("if");
			} else if (token.text === "then" && atOpen && open.then) {
				open.then = false;
				token.ifExpressionPart = true;
			} else if (token.text === "elseif" && atOpen && !open.then) {
				open.then = true;
			} else if (token.text === "else" && atOpen && !open.then) {
				ifExpressions.pop();
				token.ifExpressionPart = true;
			} else if (["function", "do", "repeat"].includes(token.text)) stack.push(token.text);
			else if (token.text === "end" || token.text === "until") stack.pop();
		}
		if (token.kind !== "id" || KEYWORDS.has(token.text)) continue;
		const field = previous && previous.kind === "op" && (previous.text === "." || previous.text === ":");
		const key =
			stack[stack.length - 1] === "{" &&
			next && next.kind === "op" && next.text === "=" &&
			previous && previous.kind === "op" && (previous.text === "{" || previous.text === "," || previous.text === ";");
		token.name = !field && !key;
	}

	// Declarations: `local a, b`, `local function f`, `for a, b`, and every function's parameters.
	const isOp = (t, text) => t !== undefined && t.kind === "op" && t.text === text;
	const declare = (t) => {
		if (t !== undefined && t.kind === "id" && t.name) t.declaration = true;
	};
	// From a `:` after a declared name: past its type, to the `,` or `=` (or `in`, or the line's end) that ends it.
	const skipType = (s) => {
		let depth = 0;
		for (s++; s < sig.length; s++) {
			const t = sig[s];
			if (t.kind === "op" && "({[".includes(t.text)) depth++;
			else if (t.kind === "op" && ")}]".includes(t.text)) depth--;
			else if (depth === 0 && (isOp(t, ",") || isOp(t, "=") || (t.kind === "id" && t.text === "in"))) return s;
			// A new line at depth 0 after a complete type ends it.
			const gap = tokens[at[s] + 1];
			if (depth === 0 && gap !== undefined && gap.kind === "space" && gap.text.includes("\n") && !["|", "&", "->", "?", ":", ","].includes(t.text)) {
				return s + 1;
			}
		}
		return s;
	};
	for (let s = 0; s < sig.length; s++) {
		const token = sig[s];
		if (token.kind !== "id") continue;
		if ((token.text === "local" && sig[s + 1]?.text !== "function") || token.text === "for") {
			let j = s + 1;
			while (j < sig.length) {
				declare(sig[j]);
				j++;
				if (isOp(sig[j], ":")) j = skipType(j);
				if (!isOp(sig[j], ",")) break;
				j++;
			}
		} else if (token.text === "function") {
			if (sig[s - 1]?.text === "local") declare(sig[s + 1]);
			let j = s + 1;
			while (j < sig.length && !isOp(sig[j], "(")) j++;
			let depth = 0;
			for (; j < sig.length; j++) {
				const t = sig[j];
				if (t.kind === "op" && "({[<".includes(t.text)) depth++;
				else if (t.kind === "op" && ")}]>".includes(t.text)) {
					if (--depth === 0) break;
				} else if (depth === 1 && (isOp(sig[j - 1], "(") || isOp(sig[j - 1], ","))) declare(t);
			}
		}
	}
	const declared = new Set(sig.filter((t) => t.declaration).map((t) => t.text));
	for (const t of sig) if (t.name && declared.has(t.text) && !keep.has(t.text)) t.renamed = true;
	return tokens;
}

const classOf = (name) => name.replace(/_?[0-9]+$/, "") || "_";

// The text with its generated locals renamed (see the header). `keep`: names kept as written, the identifiers of the
// file's TS source (`sourceIdentifiers`).
function normalize(text, keep = new Set()) {
	const tokens = locals(text, keep);
	// `_`-class names used somewhere other than where they are declared.
	const used = new Set(tokens.filter((t) => t.renamed && !t.declaration && classOf(t.text) === "_").map((t) => t.text));
	const classes = new Map(); // class -> Map(name -> index)
	const out = [];
	for (const token of tokens) {
		if (!token.renamed) {
			out.push(token.text);
			continue;
		}
		const name = token.text;
		const cls = classOf(name);
		if (cls === "_" && !used.has(name)) {
			out.push("_#");
			continue;
		}
		let names = classes.get(cls);
		if (!names) classes.set(cls, (names = new Map()));
		if (!names.has(name)) names.set(name, names.size);
		out.push(`${cls}#${names.get(name)}`);
	}
	return out.join("");
}

// The identifiers of a TS source file (its words outside comments, strings and template text): the `keep` set of the
// Luau compiled from it.
function sourceIdentifiers(text) {
	const out = new Set();
	const n = text.length;
	const templates = []; // the `{` depth at which each open template's `${` started
	let braces = 0;
	let i = 0;
	// From inside a template's text: to just after its closing "`" (popping it) or its next `${` (keeping it open).
	const templateText = (from) => {
		let j = from;
		while (j < n) {
			if (text[j] === "\\") j += 2;
			else if (text[j] === "`") {
				templates.pop();
				return j + 1;
			} else if (text[j] === "$" && text[j + 1] === "{") return j + 2;
			else j++;
		}
		templates.pop();
		return n;
	};
	while (i < n) {
		const c = text[i];
		if (text.startsWith("//", i)) {
			const j = text.indexOf("\n", i);
			i = j < 0 ? n : j;
		} else if (text.startsWith("/*", i)) {
			const j = text.indexOf("*/", i + 2);
			i = j < 0 ? n : j + 2;
		} else if (c === '"' || c === "'") {
			let j = i + 1;
			while (j < n && text[j] !== c && text[j] !== "\n") j += text[j] === "\\" ? 2 : 1;
			i = Math.min(j + 1, n);
		} else if (c === "`") {
			templates.push(braces);
			i = templateText(i + 1);
		} else if (c === "}" && templates.length > 0 && templates[templates.length - 1] === braces) {
			i = templateText(i + 1);
		} else if (/[A-Za-z_$]/.test(c)) {
			let j = i + 1;
			while (j < n && /[A-Za-z0-9_$]/.test(text[j])) j++;
			out.add(text.slice(i, j));
			i = j;
		} else if (/[0-9]/.test(c)) {
			let j = i + 1;
			while (j < n && /[A-Za-z0-9_$.]/.test(text[j])) j++;
			i = j;
		} else {
			if (c === "{") braces++;
			else if (c === "}") braces--;
			i++;
		}
	}
	return out;
}

if (require.main === module) {
	const [file, source] = process.argv.slice(2);
	if (!file) {
		console.error("usage: node tools/normalize.cjs <file.luau> [<source.ts>]");
		process.exit(2);
	}
	const fs = require("fs");
	const keep = source ? sourceIdentifiers(fs.readFileSync(source, "utf8")) : new Set();
	process.stdout.write(normalize(fs.readFileSync(file, "utf8"), keep));
}

module.exports = { normalize, locals, tokenize, sourceIdentifiers };
