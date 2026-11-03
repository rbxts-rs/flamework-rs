//! A literal type's value ([`Lit`]): what `Kind::Constant` and `Kind::Literals` hold, what the generator ranks, prints
//! in layout texts and compares (`getLiteral`'s expressions in the TS generator).

use rbxts_luau::Expr;

use super::ir::{self, E, ExtLiteral};
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
    /// The literal a generated value is, if it is one (`isLiteral`, with its value): a string, a number (`-7`
    /// included), a boolean, `undefined`, or a call site's literal argument. An enum item is never read back.
    pub fn of(e: &E) -> Option<Lit> {
        Some(match e {
            E::Str(s) => Lit::Str(s.clone()),
            E::Num(n) => Lit::Num(*n),
            E::Neg(inner) => match &**inner {
                E::Num(n) => Lit::Num(-*n),
                _ => return None,
            },
            E::Bool(b) => Lit::Bool(*b),
            E::Undefined => Lit::Nil,
            E::Ext(ext) => match ext.literal.as_ref()? {
                ExtLiteral::Num(n) => Lit::Num(*n),
                ExtLiteral::Str(s) => Lit::Str(s.clone()),
                ExtLiteral::Bool(b) => Lit::Bool(*b),
            },
            _ => return None,
        })
    }

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

    /// The literal as the generator's own node, as `getLiteral` built it (`f.number`, `f.string`, …,
    /// `Enum["Material"]["Plastic"]`), for emission through the IR until it is gone.
    pub fn to_ir(&self) -> E {
        match self {
            Lit::Str(s) => ir::string(s),
            Lit::Num(n) => ir::num(*n),
            Lit::Bool(b) => ir::bool(*b),
            Lit::Nil => ir::nil(),
            Lit::EnumItem { group, item } => {
                ir::elem(ir::elem(ir::global("Enum"), ir::string(group)), ir::string(item))
            }
        }
    }

    /// The literal as Luau, rendering exactly as [`Lit::to_ir`] lowers (the literal tables', and the direct emission's
    /// from round B2).
    pub fn to_luau(&self) -> Expr {
        match self {
            Lit::Str(s) => luau::str(s),
            Lit::Num(n) => luau::num(*n),
            Lit::Bool(b) => rbxts_luau::create::bool(*b),
            Lit::Nil => rbxts_luau::create::nil(),
            Lit::EnumItem { group, item } => luau::prop(luau::prop(rbxts_luau::create::id("Enum"), group), item),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::super::ir::Ids;
    use super::super::lower::{LowerCx, Lowerer};
    use super::*;

    struct NoNames;

    impl LowerCx for NoNames {
        fn temp_id(&mut self, _: &str) -> rbxts_luau::TemporaryIdentifier {
            unreachable!("a literal has no temporaries")
        }

        fn local_name(&mut self, _: &str) -> String {
            unreachable!("a literal has no locals")
        }
    }

    fn render(e: &Expr) -> String {
        rbxts_luau::render_expr(e, &rbxts_luau::LateStrings::default())
    }

    #[test]
    fn luau_renders_as_the_lowered_ir() {
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
        let ids = Ids::default();
        let mut names = HashMap::new();
        let mut cx = NoNames;
        let mut lowerer = Lowerer { ids: &ids, names: &mut names, cx: &mut cx };
        let mut texts = Vec::new();
        for literal in &literals {
            let mut prereqs = Vec::new();
            let lowered = lowerer.expr(&literal.to_ir(), &mut prereqs);
            assert!(prereqs.is_empty());
            assert_eq!(render(&literal.to_luau()), render(&lowered), "{literal:?}");
            texts.push(render(&lowered));
        }
        assert_eq!(
            texts,
            [
                "\"a\\\"b\\u{00E9}\"",
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
        assert_eq!(Lit::of(&ir::num(-3.0)), Some(Lit::Num(-3.0)));
        assert_eq!(Lit::of(&ir::string("x")), Some(Lit::Str("x".into())));
        assert_eq!(Lit::of(&ir::nil()), Some(Lit::Nil));
        let ext = ir::ExtValue::new(false, Some(ExtLiteral::Bool(true)));
        assert_eq!(Lit::of(&E::Ext(ext)), Some(Lit::Bool(true)));
        assert_eq!(Lit::of(&E::Ext(ir::ExtValue::new(true, None))), None);
        assert_eq!(Lit::of(&ir::global("x")), None);
        assert_eq!(Lit::of(&Lit::EnumItem { group: "A".into(), item: "B".into() }.to_ir()), None);
    }
}
