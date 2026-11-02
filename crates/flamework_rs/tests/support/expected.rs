//! Expected outputs: compared byte for byte, or written with `UPDATE_EXPECTED=1`; and TS5 references, compared with
//! the generated locals' names normalised.

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

/// A Luau file with its locals renamed to canonical names, as `tools/normalize.cjs` renames them (see there): every
/// identifier but a keyword, a field name or a table constructor's key becomes `<class>#<n>`, its class being its name
/// without a trailing number and `n` the order in which the class's names first occur; the `_` class becomes `_`.
/// Strings and comments are kept. For comparing with a TS5 reference, whose generated locals are named differently.
pub fn normalize(text: &str) -> String {
    let tokens = tokenize(text);
    let significant: Vec<usize> = (0..tokens.len()).filter(|&i| tokens[i].0 != Token::Space).collect();
    let mut stack: Vec<&str> = Vec::new();
    let mut classes: std::collections::HashMap<String, Vec<String>> = Default::default();
    let mut out = String::with_capacity(text.len());
    for (s, &index) in significant.iter().enumerate() {
        // whitespace and comments before this token, as they are
        let start = if s == 0 { 0 } else { significant[s - 1] + 1 };
        for t in &tokens[start..index] {
            out.push_str(t.1);
        }
        let (kind, token) = tokens[index];
        let previous = s.checked_sub(1).map(|p| tokens[significant[p]]);
        let next = significant.get(s + 1).map(|&n| tokens[n]);
        match (kind, token) {
            (Token::Op, "{" | "(" | "[") => stack.push(token),
            (Token::Op, "}" | ")" | "]") | (Token::Id, "end" | "until") => {
                stack.pop();
            }
            (Token::Id, "function" | "if" | "do" | "repeat") => stack.push(token),
            _ => {}
        }
        let op = |t: Option<(Token, &str)>, texts: &[&str]| matches!(t, Some((Token::Op, x)) if texts.contains(&x));
        let local = kind == Token::Id
            && !KEYWORDS.contains(&token)
            && !op(previous, &[".", ":"])
            && !(stack.last() == Some(&"{") && op(next, &["="]) && op(previous, &["{", ",", ";"]));
        if !local {
            out.push_str(token);
            continue;
        }
        let class = token.trim_end_matches(|c: char| c.is_ascii_digit());
        let class = if class.len() < token.len() { class.strip_suffix('_').unwrap_or(class) } else { class };
        let class = if class.is_empty() { "_" } else { class };
        if class == "_" {
            out.push('_');
            continue;
        }
        let names = classes.entry(class.to_owned()).or_default();
        let n = names.iter().position(|name| name == token).unwrap_or_else(|| {
            names.push(token.to_owned());
            names.len() - 1
        });
        out.push_str(&format!("{class}#{n}"));
    }
    if let Some(&last) = significant.last() {
        for t in &tokens[last + 1..] {
            out.push_str(t.1);
        }
    } else {
        out.push_str(text);
    }
    out
}

const KEYWORDS: [&str; 25] = [
    "and", "break", "continue", "do", "else", "elseif", "end", "export", "false", "for", "function", "if", "in", "local",
    "nil", "not", "or", "repeat", "return", "then", "true", "type", "typeof", "until", "while",
];

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
