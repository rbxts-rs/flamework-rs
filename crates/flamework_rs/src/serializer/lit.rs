//! A literal type's value ([`Lit`]): what `Kind::Constant` and `Kind::Literals` hold, what the generator ranks, prints
//! in layout texts and compares (`getLiteral`'s expressions in the TS generator).

use rbxts_luau::Expr;

use super::luau;

/// A literal: a string, a number, a boolean, `nil`, or a Roblox enum item (`Enum.Material.Plastic`).
#[derive(Clone, Debug, PartialEq)]
pub enum Lit {
    Str(String),
    Num(f64),
    Bool(bool),
    Nil,
    /// `Enum.<group>.<item>`
    EnumItem { group: String, item: String },
}

impl Lit {
    /// `literalType`: Luau's `type()` of the value (an enum item is no literal type: `None`).
    pub fn type_name(&self) -> Option<&'static str> {
        match self {
            Lit::Str(_) => Some("string"),
            Lit::Num(_) => Some("number"),
            Lit::Bool(_) => Some("boolean"),
            Lit::Nil => Some("nil"),
            Lit::EnumItem { .. } => None,
        }
    }

    /// The literal as Luau.
    pub fn to_luau(&self) -> Expr {
        match self {
            Lit::Str(s) => luau::str(s),
            Lit::Num(n) => luau::num(*n),
            Lit::Bool(b) => rbxts_luau::create::bool(*b),
            Lit::Nil => rbxts_luau::create::nil(),
            Lit::EnumItem { group, item } => luau::prop(luau::prop(rbxts_luau::create::id("Enum"), group), item),
        }
    }

    /// The literal a generated value is, if it is one: `nil`, a boolean or a number (`-7` included). Strings are only
    /// ever read back from a call site's arguments (`Gen::literal`).
    pub fn of(e: &Expr) -> Option<Lit> {
        Some(match e {
            Expr::Nil => Lit::Nil,
            Expr::True => Lit::Bool(true),
            Expr::False => Lit::Bool(false),
            _ => Lit::Num(luau::literal_number(e)?),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(e: &Expr) -> String {
        rbxts_luau::render_expr(e, &rbxts_luau::LateStrings::default())
    }

    #[test]
    fn literals_render_as_luau() {
        let literals = [
            Lit::Str("a\"b\u{e9}".into()),
            Lit::Str(String::new()),
            Lit::Num(7.0),
            Lit::Num(-7.5),
            Lit::Num(-0.0),
            Lit::Num(1e21),
            Lit::Num(f64::INFINITY),
            Lit::Num(f64::NEG_INFINITY),
            Lit::Bool(true),
            Lit::Bool(false),
            Lit::Nil,
            Lit::EnumItem { group: "Material".into(), item: "Plastic".into() },
        ];
        let texts: Vec<String> = literals.iter().map(|l| render(&l.to_luau())).collect();
        assert_eq!(
            texts,
            [
                r#""a\"b\u{00E9}""#,
                "\"\"",
                "7",
                "-7.5",
                "0",
                "1e+21",
                "1e999",
                "-1e999",
                "true",
                "false",
                "nil",
                "Enum.Material.Plastic"
            ]
        );
    }

    #[test]
    fn values_read_back_as_literals() {
        assert_eq!(Lit::of(&luau::num(-3.0)), Some(Lit::Num(-3.0)));
        assert_eq!(Lit::of(&rbxts_luau::create::nil()), Some(Lit::Nil));
        assert_eq!(Lit::of(&rbxts_luau::create::bool(true)), Some(Lit::Bool(true)));
        assert_eq!(Lit::of(&rbxts_luau::create::id("x")), None);
        assert_eq!(Lit::of(&Lit::EnumItem { group: "A".into(), item: "B".into() }.to_luau()), None);
    }
}
