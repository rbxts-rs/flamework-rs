//! Expected outputs: compared byte for byte, or written with `UPDATE_EXPECTED=1`; and TS5 references, compared with
//! the generated locals' names normalised (a port of `tools/normalize.cjs`: keep the two alike).

use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Whether this run writes the expected outputs (`UPDATE_EXPECTED=1`) instead of comparing them.
pub fn updating() -> bool {
    std::env::var_os("UPDATE_EXPECTED").is_some()
}

/// `got` against the expected file at `path`: where it first differs, described (`None` when equal). With
/// `UPDATE_EXPECTED=1`, writes `got` there instead.
pub fn check(name: &str, path: &Path, got: &str) -> Option<String> {
    if updating() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, got).unwrap();
        return None;
    }
    let want = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{name}: {e}")).replace("\r\n", "\n");
    first_difference(name, got, &want)
}

/// Where `got` first differs from `want`, described (`None` when equal).
pub fn first_difference(name: &str, got: &str, want: &str) -> Option<String> {
    if got == want {
        return None;
    }
    let line = got
        .lines()
        .zip(want.lines())
        .position(|(a, b)| a != b)
        .unwrap_or_else(|| got.lines().count().min(want.lines().count()));
    Some(format!(
        "{name}: first difference at line {}\n  ours: {}\n  want: {}",
        line + 1,
        got.lines().nth(line).unwrap_or("<eof>"),
        want.lines().nth(line).unwrap_or("<eof>")
    ))
}

/// A Luau file with its generated locals renamed to canonical names, as `tools/normalize.cjs` renames them (see
/// there): a name the file declares (`local`, a `for` variable, a parameter, `local function`) that is not in `keep`
/// (the identifiers of the file's TS source, [`source_identifiers`]) becomes `<class>#<n>` wherever it stands as a name
/// (not a field name, not a table constructor's key; never in a string or a comment), its class being its name
/// without a trailing number and `n` the order in which the class's renamed names first occur; a `_`-class name that
/// is never used, only declared, becomes `_#`. Keywords, globals and the names in `keep` stay as written. For
/// comparing with a TS5 reference, whose generated locals are named differently.
pub fn normalize(text: &str, keep: &HashSet<String>) -> String {
    let tokens = tokenize(text);
    let marks = marks(&tokens, keep);
    // `_`-class names used somewhere other than where they are declared
    let used: HashSet<&str> = (0..tokens.len())
        .filter(|&i| marks[i].renamed && !marks[i].declaration && class_of(tokens[i].1) == "_")
        .map(|i| tokens[i].1)
        .collect();
    let mut classes: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut out = String::with_capacity(text.len());
    for (i, &(_, token)) in tokens.iter().enumerate() {
        if !marks[i].renamed {
            out.push_str(token);
            continue;
        }
        let class = class_of(token);
        if class == "_" && !used.contains(token) {
            out.push_str("_#");
            continue;
        }
        let names = classes.entry(class).or_default();
        let n = names.iter().position(|&name| name == token).unwrap_or_else(|| {
            names.push(token);
            names.len() - 1
        });
        out.push_str(&format!("{class}#{n}"));
    }
    out
}

/// A name's class: the name without a trailing number (and the `_` before it), `_` when nothing is left.
fn class_of(name: &str) -> &str {
    let class = name.trim_end_matches(|c: char| c.is_ascii_digit());
    let class = if class.len() < name.len() { class.strip_suffix('_').unwrap_or(class) } else { class };
    if class.is_empty() { "_" } else { class }
}

/// The identifiers of a TS source file (its words outside comments, strings and template text): the `keep` set of the
/// Luau compiled from it (`tools/normalize.cjs`'s `sourceIdentifiers`).
pub fn source_identifiers(text: &str) -> HashSet<String> {
    let bytes = text.as_bytes();
    let n = bytes.len();
    let mut out = HashSet::new();
    // the `{` depth at which each open template's `${` started
    let mut templates: Vec<i64> = Vec::new();
    let mut braces = 0i64;
    // From inside a template's text: to just after its closing "`" (popping it) or its next `${` (keeping it open).
    let template_text = |from: usize, templates: &mut Vec<i64>| -> usize {
        let mut j = from;
        while j < n {
            if bytes[j] == b'\\' {
                j += 2;
            } else if bytes[j] == b'`' {
                templates.pop();
                return j + 1;
            } else if bytes[j] == b'$' && bytes.get(j + 1) == Some(&b'{') {
                return j + 2;
            } else {
                j += 1;
            }
        }
        templates.pop();
        n
    };
    let mut i = 0;
    while i < n {
        let c = bytes[i];
        if bytes[i..].starts_with(b"//") {
            i = text[i..].find('\n').map_or(n, |j| i + j);
        } else if bytes[i..].starts_with(b"/*") {
            i = text[i + 2..].find("*/").map_or(n, |j| i + 2 + j + 2);
        } else if c == b'"' || c == b'\'' {
            let mut j = i + 1;
            while j < n && bytes[j] != c && bytes[j] != b'\n' {
                j += if bytes[j] == b'\\' { 2 } else { 1 };
            }
            i = (j + 1).min(n);
        } else if c == b'`' {
            templates.push(braces);
            i = template_text(i + 1, &mut templates);
        } else if c == b'}' && templates.last() == Some(&braces) {
            i = template_text(i + 1, &mut templates);
        } else if c.is_ascii_alphabetic() || c == b'_' || c == b'$' {
            let start = i;
            i += 1;
            while i < n && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'$') {
                i += 1;
            }
            out.insert(text[start..i].to_owned());
        } else if c.is_ascii_digit() {
            i += 1;
            while i < n && (bytes[i].is_ascii_alphanumeric() || matches!(bytes[i], b'_' | b'$' | b'.')) {
                i += 1;
            }
        } else {
            match c {
                b'{' => braces += 1,
                b'}' => braces -= 1,
                _ => {}
            }
            i += 1;
        }
    }
    out
}

const KEYWORDS: [&str; 25] = [
    "and", "break", "continue", "do", "else", "elseif", "end", "export", "false", "for", "function", "if", "in", "local",
    "nil", "not", "or", "repeat", "return", "then", "true", "type", "typeof", "until", "while",
];

/// The tokens after which `if` starts an if-expression, not an if-statement (with the `then` and `else` of an
/// if-expression).
const EXPRESSION_BEFORE: [&str; 31] = [
    "=", "(", ",", "{", "[", "+", "-", "*", "/", "//", "%", "^", "..", "==", "~=", "<", "<=", ">", ">=", "+=", "-=",
    "*=", "/=", "//=", "%=", "^=", "..=", "return", "and", "or", "not",
];

/// What `normalize` knows of a token: whether it stands as a name (an identifier, not a keyword, a field name or a
/// table constructor's key), whether the file declares it there, and whether `normalize` renames it.
#[derive(Clone, Copy, Default)]
struct Mark {
    name: bool,
    declaration: bool,
    renamed: bool,
}

/// Each token's [`Mark`] (`tools/normalize.cjs`'s `locals`).
fn marks(tokens: &[(Token, &str)], keep: &HashSet<String>) -> Vec<Mark> {
    let at: Vec<usize> = (0..tokens.len()).filter(|&i| tokens[i].0 != Token::Space).collect();
    let mut marks = vec![Mark::default(); tokens.len()];
    // the `then` and `else` of an if-expression
    let mut part = vec![false; tokens.len()];
    // open brackets, and blocks (`function`, `if`, `do`, `repeat`) up to their `end` / `until`
    let mut stack: Vec<&str> = Vec::new();
    // open if-expressions: the stack depth they are at, and whether a `then` is due
    let mut ifs: Vec<(usize, bool)> = Vec::new();
    let op = |t: Option<(Token, &str)>, texts: &[&str]| matches!(t, Some((Token::Op, x)) if texts.contains(&x));
    for s in 0..at.len() {
        let i = at[s];
        let (kind, token) = tokens[i];
        let previous = s.checked_sub(1).map(|p| at[p]);
        let next = at.get(s + 1).map(|&n| tokens[n]);
        let open = ifs.last().copied().filter(|&(depth, _)| depth == stack.len());
        match (kind, token) {
            (Token::Op, "{" | "(" | "[") => stack.push(token),
            (Token::Op, "}" | ")" | "]") | (Token::Id, "end" | "until") => {
                stack.pop();
            }
            (Token::Id, "if") => {
                let expression = previous.is_some_and(|p| {
                    part[p]
                        || (tokens[p].0 == Token::Str && tokens[p].1.ends_with('{'))
                        || (matches!(tokens[p].0, Token::Op | Token::Id)
                            && EXPRESSION_BEFORE.contains(&tokens[p].1))
                });
                if expression {
                    ifs.push((stack.len(), true));
                } else {
                    stack.push(token);
                }
            }
            (Token::Id, "then") if open.is_some_and(|(_, then)| then) => {
                ifs.last_mut().unwrap().1 = false;
                part[i] = true;
            }
            (Token::Id, "elseif") if open.is_some_and(|(_, then)| !then) => ifs.last_mut().unwrap().1 = true,
            (Token::Id, "else") if open.is_some_and(|(_, then)| !then) => {
                ifs.pop();
                part[i] = true;
            }
            (Token::Id, "function" | "do" | "repeat") => stack.push(token),
            _ => {}
        }
        if kind != Token::Id || KEYWORDS.contains(&token) {
            continue;
        }
        let previous = previous.map(|p| tokens[p]);
        marks[i].name = !op(previous, &[".", ":"])
            && !(stack.last() == Some(&"{") && op(next, &["="]) && op(previous, &["{", ",", ";"]));
    }

    // Declarations: `local a, b`, `local function f`, `for a, b`, and every function's parameters.
    let text = |s: usize| at.get(s).map(|&i| tokens[i]);
    let is_op = |s: usize, x: &str| op(text(s), &[x]);
    let mut declare = |s: usize| {
        if let Some(&i) = at.get(s)
            && marks[i].name
        {
            marks[i].declaration = true;
        }
    };
    // From a `:` after a declared name: past its type, to the `,` or `=` (or `in`, or the line's end) that ends it.
    let skip_type = |mut s: usize| -> usize {
        let mut depth = 0i64;
        s += 1;
        while s < at.len() {
            let (kind, t) = tokens[at[s]];
            if kind == Token::Op && matches!(t, "(" | "{" | "[") {
                depth += 1;
            } else if kind == Token::Op && matches!(t, ")" | "}" | "]") {
                depth -= 1;
            } else if depth == 0 && matches!((kind, t), (Token::Op, "," | "=") | (Token::Id, "in")) {
                return s;
            }
            // A new line at depth 0 after a complete type ends it.
            let gap = tokens.get(at[s] + 1);
            if depth == 0
                && gap.is_some_and(|&(kind, gap)| kind == Token::Space && gap.contains('\n'))
                && !matches!(t, "|" | "&" | "->" | "?" | ":" | ",")
            {
                return s + 1;
            }
            s += 1;
        }
        s
    };
    for s in 0..at.len() {
        let (kind, token) = tokens[at[s]];
        if kind != Token::Id {
            continue;
        }
        if (token == "local" && text(s + 1).map(|t| t.1) != Some("function")) || token == "for" {
            let mut j = s + 1;
            while j < at.len() {
                declare(j);
                j += 1;
                if is_op(j, ":") {
                    j = skip_type(j);
                }
                if !is_op(j, ",") {
                    break;
                }
                j += 1;
            }
        } else if token == "function" {
            if s > 0 && text(s - 1).map(|t| t.1) == Some("local") {
                declare(s + 1);
            }
            let mut j = s + 1;
            while j < at.len() && !is_op(j, "(") {
                j += 1;
            }
            let mut depth = 0i64;
            while j < at.len() {
                let (kind, t) = tokens[at[j]];
                if kind == Token::Op && matches!(t, "(" | "{" | "[" | "<") {
                    depth += 1;
                } else if kind == Token::Op && matches!(t, ")" | "}" | "]" | ">") {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                } else if depth == 1 && (is_op(j - 1, "(") || is_op(j - 1, ",")) {
                    declare(j);
                }
                j += 1;
            }
        }
    }
    let declared: HashSet<&str> = (0..tokens.len()).filter(|&i| marks[i].declaration).map(|i| tokens[i].1).collect();
    for (i, mark) in marks.iter_mut().enumerate() {
        mark.renamed = mark.name && declared.contains(tokens[i].1) && !keep.contains(tokens[i].1)
            && tokens[i].1 != "self";
    }
    marks
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Token {
    Id,
    Space,
    Str,
    Num,
    Op,
}

/// The file as tokens (`tools/normalize.cjs`'s `tokenize`).
fn tokenize(text: &str) -> Vec<(Token, &str)> {
    let bytes = text.as_bytes();
    let n = bytes.len();
    let mut tokens = Vec::new();
    let long_bracket = |at: usize| -> Option<usize> {
        if bytes.get(at) != Some(&b'[') {
            return None;
        }
        let mut j = at + 1;
        while bytes.get(j) == Some(&b'=') {
            j += 1;
        }
        (bytes.get(j) == Some(&b'[')).then_some(j - at - 1)
    };
    let close_long = |from: usize, level: usize| -> usize {
        let close = format!("]{}]", "=".repeat(level));
        text[from..].find(&close).map_or(n, |i| from + i + close.len())
    };
    let interpolated_piece = |from: usize| -> usize {
        let mut j = from;
        while j < n && bytes[j] != b'`' && bytes[j] != b'{' {
            if bytes[j] == b'\\' {
                j += 1;
            }
            j += 1;
        }
        j.min(n)
    };
    let mut interpolations: Vec<i32> = Vec::new();
    let mut braces = 0i32;
    let mut i = 0;
    while i < n {
        let c = bytes[i];
        let start = i;
        let kind;
        if c.is_ascii_whitespace() {
            while i < n && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            kind = Token::Space;
        } else if text[i..].starts_with("--") {
            i = match long_bracket(i + 2) {
                Some(level) => close_long(i + 2, level),
                None => text[i..].find('\n').map_or(n, |j| i + j),
            };
            kind = Token::Space;
        } else if c == b'"' || c == b'\'' {
            i += 1;
            while i < n && bytes[i] != c && bytes[i] != b'\n' {
                if bytes[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
            i = (i + 1).min(n);
            kind = Token::Str;
        } else if let Some(level) = long_bracket(i) {
            i = close_long(i, level);
            kind = Token::Str;
        } else if c == b'`' || (c == b'}' && interpolations.last() == Some(&braces)) {
            if c == b'}' {
                interpolations.pop();
            }
            let j = interpolated_piece(i + 1);
            if j < n && bytes[j] == b'{' {
                interpolations.push(braces);
            }
            i = (j + 1).min(n);
            kind = Token::Str;
        } else if c.is_ascii_alphabetic() || c == b'_' {
            while i < n && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            kind = Token::Id;
        } else if c.is_ascii_digit() || (c == b'.' && bytes.get(i + 1).is_some_and(u8::is_ascii_digit)) {
            let hex = text[i..].starts_with("0x") || text[i..].starts_with("0X");
            i += 1;
            while i < n
                && (bytes[i].is_ascii_alphanumeric()
                    || bytes[i] == b'_'
                    || bytes[i] == b'.'
                    || ((bytes[i] == b'+' || bytes[i] == b'-') && !hex && matches!(bytes[i - 1], b'e' | b'E')))
            {
                i += 1;
            }
            kind = Token::Num;
        } else {
            let three = text.get(i..i + 3).unwrap_or("");
            let two = text.get(i..i + 2).unwrap_or("");
            let len = if ["...", "..=", "//="].contains(&three) {
                3
            } else if ["..", "==", "~=", "<=", ">=", "+=", "-=", "*=", "/=", "%=", "^=", "::", "->", "//"].contains(&two) {
                2
            } else {
                text[i..].chars().next().map_or(1, char::len_utf8)
            };
            match c {
                b'{' if len == 1 => braces += 1,
                b'}' if len == 1 => braces -= 1,
                _ => {}
            }
            i += len;
            kind = Token::Op;
        }
        tokens.push((kind, &text[start..i]));
    }
    tokens
}
