//! Thin builders over [`rbxts_luau::create`] for the serializer's direct Luau emission (DESIGN §2.1).
//!
//! The renderer already adds the parentheses precedence needs (`needs_parentheses`), so a builder adds them only where
//! Luau needs them on a callee or a receiver ([`indexable`]). Locals and parameters are placeholders
//! ([`crate::luau_names::Names::fresh`]), declared here through [`declared`], so every declaration goes through one
//! place.
//!
//! Spellings: numbers as JavaScript prints them (`1e999` for an infinity), strings and template text as roblox-ts
//! spelled a TS transformer's (`synthesized_string`, `template_text`), which TypeScript 7.1 spells the same way.

use rbxts_luau::{
    AssignmentOperator, BinaryOperator, ElseBody, Expr, InterpolatedStringPart, ReturnStatement, Stmt, UnaryOperator,
    create,
};

use crate::luau_names::declared;

/// `convertToIndexableExpression`: `e` as a callee or receiver, parenthesized unless it already is one.
pub(crate) fn indexable(e: Expr) -> Expr {
    if e.is_indexable() { e } else { create::paren(e) }
}

/// `callee(args)`.
pub(crate) fn call(callee: Expr, args: Vec<Expr>) -> Expr {
    create::call(indexable(callee), args)
}

/// `object:name(args)`.
pub(crate) fn method(object: Expr, name: &str, args: Vec<Expr>) -> Expr {
    create::method_call(indexable(object), name, args)
}

/// `object.key`, or `object["key"]` when `key` is no Luau identifier (a keyword, `a-b`, …).
pub(crate) fn prop(object: Expr, key: &str) -> Expr {
    if rbxts_luau::is_valid_identifier(key) {
        create::property(indexable(object), key)
    } else {
        create::computed_index(indexable(object), str(key))
    }
}

/// `object[key]`.
pub(crate) fn index(object: Expr, key: Expr) -> Expr {
    create::computed_index(indexable(object), key)
}

/// `#e`.
pub(crate) fn len(e: Expr) -> Expr {
    create::unary(UnaryOperator::Len, e)
}

/// `not e`.
pub(crate) fn not(e: Expr) -> Expr {
    create::unary(UnaryOperator::Not, e)
}

/// `left op right`.
pub(crate) fn bin(left: Expr, op: BinaryOperator, right: Expr) -> Expr {
    create::binary(left, op, right)
}

/// `if condition then when_true else when_false`.
pub(crate) fn if_expr(condition: Expr, when_true: Expr, when_false: Expr) -> Expr {
    create::if_expr(condition, when_true, when_false)
}

/// A number: its JavaScript spelling (`String(value)`), `1e999` for an infinity, a negation of the magnitude for a
/// negative value, and `0 / 0` for NaN (which has no literal).
pub(crate) fn num(value: f64) -> Expr {
    if value.is_nan() {
        create::binary(num(0.0), BinaryOperator::Div, num(0.0))
    } else if value < 0.0 {
        create::unary(UnaryOperator::Neg, num(-value))
    } else if value.is_infinite() {
        create::number_text("1e999")
    } else {
        create::number_text(crate::js::number_to_string(value))
    }
}

/// A string literal with the value `text`, spelled as roblox-ts spelled a TS transformer's
/// ([`rbxts_plugin::synthesized_string`]: `"\u{00E9}"`).
pub(crate) fn str(text: &str) -> Expr {
    rbxts_plugin::synthesized_string(text)
}

/// A part of an interpolated string ([`interp`]).
pub(crate) enum Part {
    /// Text, as its value (escaped here).
    Text(String),
    Expr(Expr),
}

/// `` `text{e}text` ``: adjacent text parts are joined and empty ones dropped, text is spelled as a template's
/// ([`template_text`]); a plain string ([`str`]) when no part is an expression.
pub(crate) fn interp(parts: Vec<Part>) -> Expr {
    let mut joined: Vec<Part> = Vec::new();
    for part in parts {
        match (part, joined.last_mut()) {
            (Part::Text(text), Some(Part::Text(last))) => last.push_str(&text),
            (part, _) => joined.push(part),
        }
    }
    if let [] | [Part::Text(_)] = joined.as_slice() {
        let text = match joined.pop() {
            Some(Part::Text(text)) => text,
            _ => String::new(),
        };
        return str(&text);
    }
    let parts = joined
        .into_iter()
        .filter_map(|part| match part {
            Part::Text(text) if text.is_empty() => None,
            Part::Text(text) => Some(InterpolatedStringPart::Text(template_text(&text))),
            Part::Expr(e) => Some(InterpolatedStringPart::Expression(e)),
        })
        .collect();
    create::interpolated_string(parts)
}

/// `local name = value` (`local name` without a value); `name` is a placeholder or an identifier.
pub(crate) fn local(name: &Expr, value: Option<Expr>) -> Stmt {
    create::local(declared(name), value)
}

/// `local a, b = x, y`.
pub(crate) fn locals(names: &[Expr], values: Vec<Expr>) -> Stmt {
    create::locals(names.iter().map(declared).collect(), values)
}

/// `target = value`.
pub(crate) fn assign(target: Expr, value: Expr) -> Stmt {
    create::assignment(target, AssignmentOperator::Assign, value)
}

/// `target op= value` (`+=`, `..=`, `*=`, …).
pub(crate) fn compound(target: Expr, op: AssignmentOperator, value: Expr) -> Stmt {
    create::assignment(target, op, value)
}

/// `if c1 then … elseif c2 then … else … end`: one branch per condition, `otherwise` (when not empty) last.
pub(crate) fn if_chain(branches: Vec<(Expr, Vec<Stmt>)>, otherwise: Vec<Stmt>) -> Stmt {
    let mut else_body = ElseBody::Block(otherwise);
    let mut branches = branches.into_iter().rev();
    let (condition, statements) = branches.next().expect("an if statement has a condition");
    let mut statement = rbxts_luau::IfStatement { condition, statements, else_body };
    for (condition, statements) in branches {
        else_body = ElseBody::If(Box::new(statement));
        statement = rbxts_luau::IfStatement { condition, statements, else_body };
    }
    Stmt::If(Box::new(statement))
}

/// `for id = start, end do body end`.
pub(crate) fn for_range(id: &Expr, start: Expr, end: Expr, body: Vec<Stmt>) -> Stmt {
    create::numeric_for(declared(id), start, end, None, body)
}

/// `for a, b in iterable do body end` (`ids` may hold `_`).
pub(crate) fn for_in(ids: &[Expr], iterable: Expr, body: Vec<Stmt>) -> Stmt {
    create::for_in(ids.iter().map(declared).collect(), iterable, body)
}

/// `while condition do body end`.
pub(crate) fn while_(condition: Expr, body: Vec<Stmt>) -> Stmt {
    create::while_stmt(condition, body)
}

/// `do body end`.
pub(crate) fn do_(body: Vec<Stmt>) -> Stmt {
    create::do_stmt(body)
}

/// `function(params) body end`.
pub(crate) fn function(params: &[Expr], body: Vec<Stmt>) -> Expr {
    create::function(params.iter().map(declared).collect(), false, body)
}

/// `return a, b, …`.
pub(crate) fn returns(values: Vec<Expr>) -> Stmt {
    Stmt::Return(ReturnStatement { expressions: values })
}

/// A name (a local, a placeholder, a global): reading it again reads the same value, so it is never bound to a local.
pub(crate) fn is_name(e: &Expr) -> bool {
    matches!(e, Expr::Identifier(_) | Expr::TemporaryIdentifier(_))
}

/// A literal: `nil`, a boolean, a string or a number (`-7` included).
pub(crate) fn is_literal(e: &Expr) -> bool {
    matches!(e, Expr::Nil | Expr::True | Expr::False | Expr::String(_)) || literal_number(e).is_some()
}

/// A number literal's value (`7`, `-7`), as `getLiteralNumberValue` reads it.
pub(crate) fn literal_number(e: &Expr) -> Option<f64> {
    match e {
        Expr::Number(n) => Some(crate::js::string_to_number(&n.value)),
        Expr::Unary(u) if u.operator == UnaryOperator::Neg => literal_number(&u.expression).map(|v| -v),
        _ => None,
    }
}

/// `e + n` with the constant folded: `e` itself for 0, a literal's sum, `i - 1` plus one as `i`, `x + 2` plus one as
/// `x + 3`, and `e - m` for a negative `n`. A binary's constant is folded only when both are whole (sizes and offsets),
/// so floating-point sums keep their order of evaluation.
pub(crate) fn add(e: Expr, n: f64) -> Expr {
    if n == 0.0 {
        return e;
    }
    if let Some(value) = literal_number(&e) {
        return num(value + n);
    }
    if let Expr::Binary(b) = &e
        && matches!(b.operator, BinaryOperator::Add | BinaryOperator::Sub)
        && let Some(right) = literal_number(&b.right)
        && is_whole(right)
        && is_whole(n)
    {
        let right = if b.operator == BinaryOperator::Sub { -right } else { right };
        return offset(b.left.clone(), right + n);
    }
    offset(e, n)
}

/// `left + right` with [`add`]'s folding when either side is a literal number (`0 + x` is `x`).
pub(crate) fn plus(left: Expr, right: Expr) -> Expr {
    if literal_number(&left) == Some(0.0) {
        return right;
    }
    match literal_number(&right) {
        Some(n) => add(left, n),
        None => bin(left, BinaryOperator::Add, right),
    }
}

fn is_whole(n: f64) -> bool {
    n.fract() == 0.0 && n.abs() < 9007199254740992.0
}

fn offset(e: Expr, n: f64) -> Expr {
    if n == 0.0 {
        e
    } else if n > 0.0 {
        bin(e, BinaryOperator::Add, num(n))
    } else {
        bin(e, BinaryOperator::Sub, num(-n))
    }
}

// --- string spelling ------------------------------------------------------------------------------------------------

/// A template's text as roblox-ts spelled a TS transformer's (TypeScript's printer, backtick-escaped, with `${`
/// escaped, then `createStringFromLiteral`).
pub(crate) fn template_text(text: &str) -> String {
    let escaped = ts_escape(text, '`').replace("${", "\\${");
    from_literal_source(&escaped, false)
}

/// TypeScript's `escapeNonAsciiString(text, quote)`.
fn ts_escape(text: &str, quote: char) -> String {
    let units: Vec<u16> = text.encode_utf16().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < units.len() {
        let u = units[i];
        let next = units.get(i + 1).copied();
        let escape = |out: &mut String, u: u16| out.push_str(&format!("\\u{:04X}", u));
        match u {
            0x0D if quote == '`' && next == Some(0x0A) => {
                out.push_str("\\r\\n");
                i += 2;
                continue;
            }
            0 => {
                if next.is_some_and(|n| (0x30..=0x39).contains(&n)) {
                    out.push_str("\\x00");
                } else {
                    out.push_str("\\0");
                }
            }
            0x09 => out.push_str("\\t"),
            0x0B => out.push_str("\\v"),
            0x0C => out.push_str("\\f"),
            0x08 => out.push_str("\\b"),
            0x0D => out.push_str("\\r"),
            0x0A if quote == '"' => out.push_str("\\n"),
            0x5C => out.push_str("\\\\"),
            0x22 if quote == '"' => out.push_str("\\\""),
            0x60 if quote == '`' => out.push_str("\\`"),
            0x2028 | 0x2029 | 0x85 => escape(&mut out, u),
            u if u < 0x20 => escape(&mut out, u),
            u if u > 0x7F => escape(&mut out, u),
            u => out.push(char::from_u32(u32::from(u)).unwrap()),
        }
        i += 1;
    }
    out
}

/// `createStringFromLiteral` on a literal's text between its delimiters.
fn from_literal_source(text: &str, is_string_literal: bool) -> String {
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    let bytes = text.as_bytes();
    // Surrogate pairs as one code point.
    let mut combined = String::with_capacity(text.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            if bytes.get(i + 1) == Some(&b'\\') {
                combined.push_str("\\\\");
                i += 2;
                continue;
            }
            if let Some((high, low)) = surrogate_pair_at(bytes, i + 1) {
                let code_point = char::decode_utf16([high, low]).next().and_then(Result::ok).map_or(0, u32::from);
                combined.push_str(&format!("\\u{{{code_point:x}}}"));
                i += 12;
                continue;
            }
        }
        let c = text[i..].chars().next().unwrap();
        combined.push(c);
        i += c.len_utf8();
    }
    let bytes = combined.as_bytes();
    let mut out = String::with_capacity(combined.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'\\' || i + 1 >= bytes.len() {
            let c = combined[i..].chars().next().unwrap();
            out.push(c);
            i += c.len_utf8();
            continue;
        }
        if bytes[i + 1] == b'u' && bytes.len() >= i + 6 && bytes[i + 2..i + 6].iter().all(u8::is_ascii_hexdigit) {
            out.push_str("\\u{");
            out.push_str(&combined[i + 2..i + 6]);
            out.push('}');
            i += 6;
            continue;
        }
        let character = combined[i + 1..].chars().next().unwrap();
        let escape_len = 1 + character.len_utf8();
        if matches!(character, '\n' | '\u{2028}' | '\u{2029}') {
        } else if "0bfnrtvux\\\"'".contains(character) || (character == '`' && !is_string_literal) {
            out.push_str(&combined[i..i + escape_len]);
        } else {
            out.push(character);
        }
        i += escape_len;
    }
    out
}

fn surrogate_pair_at(bytes: &[u8], i: usize) -> Option<(u16, u16)> {
    let s = bytes.get(i..i + 11)?;
    if s[0] != b'u' || s[5] != b'\\' || s[6] != b'u' {
        return None;
    }
    let high = &s[1..5];
    let low = &s[7..11];
    if !high.iter().chain(low).all(u8::is_ascii_hexdigit) {
        return None;
    }
    let ok_high = matches!(high[0], b'd' | b'D') && matches!(high[1], b'8' | b'9' | b'a' | b'A' | b'b' | b'B');
    let ok_low = matches!(low[0], b'd' | b'D') && matches!(low[1], b'c'..=b'f' | b'C'..=b'F');
    if !(ok_high && ok_low) {
        return None;
    }
    let parse = |h: &[u8]| u16::from_str_radix(std::str::from_utf8(h).unwrap(), 16).unwrap();
    Some((parse(high), parse(low)))
}

#[cfg(test)]
mod tests {
    use rbxts_luau::create::id;

    use super::*;
    use crate::luau_names::Names;

    fn text(e: &Expr) -> String {
        rbxts_luau::render_expr(e, &rbxts_luau::LateStrings::default())
    }

    fn lines(statements: &[Stmt]) -> String {
        rbxts_luau::render_ast(statements)
    }

    #[test]
    fn calls_and_receivers_are_parenthesized_only_when_luau_needs_it() {
        let f = function(&[], vec![]);
        assert_eq!(text(&call(f, vec![num(1.0)])), "(function() end)(1)");
        assert_eq!(text(&call(prop(id("buffer"), "len"), vec![id("b")])), "buffer.len(b)");
        assert_eq!(text(&method(str("x"), "rep", vec![num(2.0)])), "(\"x\"):rep(2)");
        assert_eq!(text(&method(id("cf"), "GetComponents", vec![])), "cf:GetComponents()");
        assert_eq!(text(&index(bin(id("a"), BinaryOperator::Or, id("b")), num(1.0))), "(a or b)[1]");
    }

    #[test]
    fn keys_are_dotted_when_they_are_identifiers() {
        assert_eq!(text(&prop(id("v"), "name")), "v.name");
        assert_eq!(text(&prop(id("v"), "end")), "v[\"end\"]");
        assert_eq!(text(&prop(id("v"), "a-b")), "v[\"a-b\"]");
        assert_eq!(text(&prop(id("v"), "\u{e9}")), r#"v["\u{00E9}"]"#);
        assert_eq!(text(&index(id("t"), id("k"))), "t[k]");
    }

    #[test]
    fn the_renderer_adds_precedence_parentheses() {
        let sum = bin(id("a"), BinaryOperator::Add, id("b"));
        assert_eq!(text(&bin(sum.clone(), BinaryOperator::Mul, id("c"))), "(a + b) * c");
        assert_eq!(text(&bin(id("c"), BinaryOperator::Sub, sum.clone())), "c - (a + b)");
        assert_eq!(text(&bin(sum.clone(), BinaryOperator::Sub, id("c"))), "a + b - c");
        assert_eq!(text(&len(prop(id("v"), "name"))), "#v.name");
        assert_eq!(text(&len(bin(id("a"), BinaryOperator::Concat, id("b")))), "#(a .. b)");
        assert_eq!(text(&not(bin(id("a"), BinaryOperator::And, id("b")))), "not (a and b)");
        let choice = if_expr(bin(id("n"), BinaryOperator::Lt, num(128.0)), num(1.0), num(2.0));
        assert_eq!(text(&choice), "if n < 128 then 1 else 2");
        assert_eq!(text(&bin(sum, BinaryOperator::Add, choice)), "a + b + (if n < 128 then 1 else 2)");
    }

    #[test]
    fn numbers_take_javascripts_spelling() {
        let values = [0.0, -0.0, 1.0, 0.5, -3.0, 1e21, 1e-7, 2f64.powi(53), f64::INFINITY, f64::NEG_INFINITY, f64::NAN];
        let spelled: Vec<String> = values.iter().map(|&n| text(&num(n))).collect();
        let expected = ["0", "0", "1", "0.5", "-3", "1e+21", "1e-7", "9007199254740992", "1e999", "-1e999", "0 / 0"];
        assert_eq!(spelled, expected);
        assert_eq!(text(&bin(id("a"), BinaryOperator::Sub, num(-1.0))), "a - -1");
        assert_eq!(literal_number(&num(-2.5)), Some(-2.5));
        assert_eq!(literal_number(&id("x")), None);
    }

    #[test]
    fn strings_take_the_synthesized_spelling() {
        assert_eq!(text(&str("plain")), "\"plain\"");
        assert_eq!(text(&str("a\"b\\c\n\t\u{e9}\u{1f600}")), r#""a\"b\\c\n\t\u{00E9}\u{1f600}""#);
        let parts = vec![
            Part::Text("n".into()),
            Part::Expr(bin(id("v"), BinaryOperator::Add, num(0.0))),
            Part::Text(String::new()),
            Part::Expr(id("w")),
            Part::Text("`{x}\u{e9}".into()),
        ];
        assert_eq!(text(&interp(parts)), r"`n{v + 0}{w}\`\{x\}\u{00E9}`");
    }

    #[test]
    fn template_text_takes_the_printers_escapes() {
        assert_eq!(ts_escape("a\"b\\c\n\u{e9}", '"'), r#"a\"b\\c\n\u00E9"#);
        assert_eq!(from_literal_source(r#"a\"b\u00E9"#, true), r#"a\"b\u{00E9}"#);
        assert_eq!(template_text("x`${y}"), r"x\`${y}");
    }

    #[test]
    fn statements_declare_placeholders() {
        let mut names = Names::default();
        let [n, i, k, v, buf] = ["n", "i", "k", "v", "buf"].map(|base| names.fresh(base));
        let increment = compound(n.clone(), AssignmentOperator::Add, num(1.0));
        let decrement = compound(n.clone(), AssignmentOperator::Sub, num(1.0));
        let mark = assign(index(id("seen"), v.clone()), rbxts_luau::create::bool(true));
        let mut body = vec![
            local(&n, Some(num(0.0))),
            local(&buf, None),
            locals(&[k.clone(), v.clone()], vec![id("a"), id("b")]),
            for_range(&i, num(1.0), len(id("list")), vec![increment]),
            for_in(&[id("_"), v.clone()], id("list"), vec![mark]),
            while_(bin(n.clone(), BinaryOperator::Gt, num(0.0)), vec![decrement]),
            do_(vec![local(&k, Some(num(1.0)))]),
            assign(buf.clone(), function(std::slice::from_ref(&v), vec![rbxts_luau::create::return_stmt(v.clone())])),
        ];
        names.assign_with(&mut body, None, |base| base.to_owned());
        assert_eq!(
            lines(&body),
            "local n = 0\nlocal buf\nlocal k, v = a, b\nfor i = 1, #list do\n\tn += 1\nend\nfor _, v in list do\n\tseen[v] = true\n\
             end\nwhile n > 0 do\n\tn -= 1\nend\ndo\n\tlocal k = 1\nend\nbuf = function(v)\n\treturn v\nend\n"
        );
    }

    #[test]
    fn if_chains_nest_as_elseif() {
        let ret = |n: f64| vec![rbxts_luau::create::return_stmt(num(n))];
        let lt = |n: f64| bin(id("n"), BinaryOperator::Lt, num(n));
        let chain = if_chain(vec![(lt(128.0), ret(1.0)), (lt(16384.0), ret(2.0))], ret(3.0));
        assert_eq!(
            lines(&[chain]),
            "if n < 128 then\n\treturn 1\nelseif n < 16384 then\n\treturn 2\nelse\n\treturn 3\nend\n"
        );
        let single = if_chain(vec![(id("ok"), vec![])], vec![]);
        assert_eq!(lines(&[single]), "if ok then\nend\n");
    }

    #[test]
    fn add_folds_constants() {
        let x = || id("x");
        assert_eq!(text(&add(x(), 0.0)), "x");
        assert_eq!(text(&add(x(), 3.0)), "x + 3");
        assert_eq!(text(&add(x(), -3.0)), "x - 3");
        assert_eq!(text(&add(num(2.0), 3.0)), "5");
        assert_eq!(text(&add(num(-2.0), 1.0)), "-1");
        assert_eq!(text(&add(bin(x(), BinaryOperator::Sub, num(1.0)), 1.0)), "x");
        assert_eq!(text(&add(bin(x(), BinaryOperator::Add, num(2.0)), 1.0)), "x + 3");
        assert_eq!(text(&add(bin(x(), BinaryOperator::Sub, num(3.0)), 1.0)), "x - 2");
        assert_eq!(text(&add(bin(x(), BinaryOperator::Add, num(1.0)), -4.0)), "x - 3");
        // not whole: the sums keep their order
        assert_eq!(text(&add(bin(x(), BinaryOperator::Add, num(0.1)), 0.2)), "x + 0.1 + 0.2");
        assert_eq!(text(&add(bin(x(), BinaryOperator::Mul, num(2.0)), 1.0)), "x * 2 + 1");
        assert_eq!(text(&plus(num(0.0), x())), "x");
        assert_eq!(text(&plus(x(), num(4.0))), "x + 4");
        assert_eq!(text(&plus(x(), id("y"))), "x + y");
    }
}
