//! Union evaluation (`evaluation`, `testOf`, `objectKey`, `fit`), discrimination, width checks and type checks.

use std::rc::Rc;

use rbxts_plugin::ts::{P, Type};

use rbxts_luau::BinaryOperator::{self, And, Div, Eq, Ge, Gt, Le, Lt, Mod};
use rbxts_luau::{Expr, Stmt, create};

use super::codegen::{Ctx, Out, Place, push};
use super::lit::Lit;
use super::luau;
use super::order::{literal_key, print_literal};
use super::typeinfo::TableKey;
use super::{Evaluation, Fit, GResult, Gen, Kind, LengthWidth, Shape, Width, kind_key};

const VARINT_LIMIT: f64 = 34359738368.0;
const F32_MAX: f64 = 3.4028234663852886e38;

fn worse(a: Fit, b: Fit) -> Fit {
    if a >= b { a } else { b }
}

fn worst(values: &[Fit]) -> Fit {
    values.iter().fold(Fit::Whole, |r, v| worse(r, *v))
}

fn at_least_empty(value: Fit) -> Fit {
    if value == Fit::None { Fit::Whole } else { value }
}

/// A union member's test (`Test`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Test {
    Exact,
    Guard,
    Partial,
    Anything,
}

/// What a check's message says a value has to be.
pub(crate) struct Expectation {
    pub expected: String,
    pub show: bool,
}

/// `numberRange(kind)`: `[min, max, whole numbers only]`.
pub(crate) fn number_range(kind: &Kind) -> Option<(f64, f64, bool)> {
    match kind {
        Kind::Varint { .. } => Some((0.0, VARINT_LIMIT - 1.0, true)),
        Kind::Number { width, .. } => match width {
            Width::F64 => None,
            Width::F32 => Some((-F32_MAX, F32_MAX, false)),
            Width::U8 => Some((0.0, 255.0, true)),
            Width::I8 => Some((-128.0, 127.0, true)),
            Width::U16 => Some((0.0, 65535.0, true)),
            Width::I16 => Some((-32768.0, 32767.0, true)),
            Width::U32 => Some((0.0, 4294967295.0, true)),
            Width::I32 => Some((-2147483648.0, 2147483647.0, true)),
            Width::U64 => Some((0.0, 9007199254740992.0, true)),
            Width::I64 => Some((-9007199254740992.0, 9007199254740992.0, true)),
        },
        _ => None,
    }
}

/// `widthName(kind)`.
pub(crate) fn width_name(kind: &Kind) -> String {
    match kind {
        Kind::Number { width, .. } => width.name().into(),
        Kind::Varint { .. } => "varint".into(),
        Kind::String { length, .. } => match length {
            LengthWidth::U8 => "string8".into(),
            LengthWidth::U16 => "string16".into(),
            _ => "string32".into(),
        },
        Kind::Buffer { length, .. } => if *length == LengthWidth::U16 { "buffer16".into() } else { "buffer32".into() },
        other => other.name().into(),
    }
}

/// `primitiveType(kind)`.
pub(crate) fn primitive_type(kind: &Kind) -> Option<String> {
    Some(match kind {
        Kind::Number { .. } | Kind::Varint { .. } => "number".into(),
        Kind::String { .. } => "string".into(),
        Kind::Boolean => "boolean".into(),
        Kind::Buffer { .. } => "buffer".into(),
        Kind::Datatype(name) => (*name).into(),
        Kind::CFrame => "CFrame".into(),
        Kind::Enum(_) => "EnumItem".into(),
        Kind::Literals(values) => return constant_primitive(&values[0]),
        Kind::Constant(value) => return constant_primitive(value),
        _ => return None,
    })
}

fn constant_primitive(value: &Lit) -> Option<String> {
    Some(match value {
        Lit::Str(_) => "string".into(),
        Lit::Num(_) => "number".into(),
        Lit::Bool(_) => "boolean".into(),
        Lit::Nil | Lit::EnumItem { .. } => "EnumItem".into(),
    })
}

/// `fitsStatically(n, range)`.
pub(crate) fn fits_statically(n: f64, (minimum, maximum, whole): (f64, f64, bool)) -> bool {
    if !whole {
        return !(n.abs() > maximum && n.abs() < f64::INFINITY);
    }
    n >= minimum && n <= maximum && n % 1.0 == 0.0
}

fn literal_range(values: &[Lit]) -> Option<(f64, f64, bool)> {
    let numbers: Vec<f64> = values.iter().map(|v| crate::js::string_to_number(&literal_key(v))).collect();
    if numbers.iter().any(|n| n.is_nan()) {
        return None;
    }
    let min = numbers.iter().copied().fold(f64::INFINITY, f64::min);
    let max = numbers.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    Some((min, max, numbers.iter().all(|n| n.fract() == 0.0 && n.is_finite())))
}

impl Gen<'_, '_, '_> {
    // --- evaluation ----------------------------------------------------------------------------------------

    /// `evaluation(union)`.
    pub(crate) fn evaluation(&mut self, union: &Rc<Kind>) -> GResult<Rc<Evaluation>> {
        let key = kind_key(union);
        if let Some(entry) = self.g.evaluations.get(&key) {
            return Ok(entry.clone());
        }
        let Kind::Union { alternatives, .. } = &**union else { panic!("evaluation of a union") };
        let indices: Vec<usize> = (0..alternatives.len()).collect();
        let mut tests = Vec::new();
        for &i in &indices {
            tests.push(self.test_of(union, i)?);
        }
        let mut takes_cache = std::collections::HashMap::new();
        let mut takes = |gn: &mut Self, a: usize, b: usize| -> GResult<bool> {
            if let Some(v) = takes_cache.get(&(a, b)) {
                return Ok(*v);
            }
            let v = gn.fit(&alternatives[a].shape, &alternatives[b].shape)? == Fit::Lossy;
            takes_cache.insert((a, b), v);
            Ok(v)
        };

        let guarded: Vec<usize> = indices
            .iter()
            .copied()
            .filter(|&i| tests[i] == Test::Guard)
            .chain(indices.iter().copied().filter(|&i| tests[i] == Test::Partial))
            .collect();
        let mut remaining = guarded.clone();
        let mut ordered = Vec::new();
        while !remaining.is_empty() {
            let mut next = None;
            for &a in &remaining {
                let mut loses = false;
                for &b in &remaining {
                    if b != a && takes(self, a, b)? && !takes(self, b, a)? {
                        loses = true;
                        break;
                    }
                }
                if !loses {
                    next = Some(a);
                    break;
                }
            }
            let next = next.unwrap_or(remaining[0]);
            ordered.push(next);
            remaining.retain(|&i| i != next);
        }

        let mut order: Vec<usize> = indices.iter().copied().filter(|&i| tests[i] == Test::Exact).collect();
        order.extend(&ordered);
        order.extend(indices.iter().copied().filter(|&i| tests[i] == Test::Anything));
        let last = *order.last().unwrap();
        let loose = matches!(tests[last], Test::Guard | Test::Partial);
        let table_only = if loose && self.describe(&alternatives[last].shape)?.is_table() { Some(last) } else { None };
        let entry = Rc::new(Evaluation { order, table_only });
        self.g.evaluations.insert(key, entry.clone());

        let mut involved = Vec::new();
        for (position, &earlier) in ordered.iter().enumerate() {
            for &later in &ordered[position + 1..] {
                if takes(self, earlier, later)? {
                    if !involved.contains(&earlier) {
                        involved.push(earlier);
                    }
                    if !involved.contains(&later) {
                        involved.push(later);
                    }
                }
            }
        }
        if !involved.is_empty() {
            let members: Vec<usize> = ordered.iter().copied().filter(|i| involved.contains(i)).collect();
            self.warn_indistinct(union, &members)?;
        }
        Ok(entry)
    }

    fn test_of(&mut self, union: &Rc<Kind>, index: usize) -> GResult<Test> {
        let Kind::Union { alternatives, .. } = &**union else { unreachable!() };
        let kind = self.describe(&alternatives[index].shape)?;
        Ok(match &*kind {
            Kind::Blob { typeof_name } => if typeof_name.is_none() { Test::Anything } else { Test::Exact },
            Kind::Object { fields } => {
                if self.object_key(union, &kind)?.is_some() {
                    Test::Exact
                } else {
                    let mut required = false;
                    for f in fields {
                        if self.is_required(&f.shape)? {
                            required = true;
                            break;
                        }
                    }
                    if required { Test::Guard } else { Test::Partial }
                }
            }
            Kind::Array(_) | Kind::Set(_) | Kind::Map(..) | Kind::List { .. } | Kind::Optional(_) | Kind::Union { .. } => {
                Test::Guard
            }
            _ => Test::Exact,
        })
    }

    pub(crate) fn is_required(&mut self, shape: &Shape) -> GResult<bool> {
        Ok(!matches!(&*self.describe(shape)?, Kind::Optional(_) | Kind::Nothing))
    }

    /// `objectKey(union, kind)`: the discriminant compared, or a required field only this member declares.
    pub(crate) fn object_key(&mut self, union: &Rc<Kind>, kind: &Rc<Kind>) -> GResult<Option<(String, TableKey, Option<Lit>)>> {
        let Kind::Union { alternatives, .. } = &**union else { unreachable!() };
        let Kind::Object { fields } = &**kind else { return Ok(None) };
        let discriminant = self.discriminant_of(union)?;
        if let Some(d) = &discriminant
            && let Some(field) = fields.iter().find(|f| &f.name == d)
        {
            let constant = self.describe(&field.shape)?;
            let Kind::Constant(value) = &*constant else { unreachable!() };
            return Ok(Some((field.name.clone(), field.key.clone(), Some(value.clone()))));
        }
        let mut others = Vec::new();
        for other in alternatives {
            let k = self.describe(&other.shape)?;
            if !Rc::ptr_eq(&k, kind) {
                others.push(k);
            }
        }
        if others.iter().any(|o| o.is_table() && !matches!(&**o, Kind::Object { .. })) {
            return Ok(None);
        }
        for candidate in fields {
            if !self.is_required(&candidate.shape)? {
                continue;
            }
            let unique = others.iter().all(|o| match &**o {
                Kind::Object { fields } => !fields.iter().any(|f| f.name == candidate.name),
                _ => true,
            });
            if unique {
                return Ok(Some((candidate.name.clone(), candidate.key.clone(), None)));
            }
        }
        Ok(None)
    }

    /// `discriminantOf(union)`.
    fn discriminant_of(&mut self, union: &Rc<Kind>) -> GResult<Option<String>> {
        let key = kind_key(union);
        if let Some(d) = self.g.discriminants.get(&key) {
            return Ok(d.clone());
        }
        let Kind::Union { alternatives, .. } = &**union else { unreachable!() };
        let mut objects = Vec::new();
        for a in alternatives {
            let k = self.describe(&a.shape)?;
            if matches!(&*k, Kind::Object { .. }) {
                objects.push(k);
            }
        }
        let mut discriminant = None;
        if let Some(first) = objects.first() {
            let Kind::Object { fields } = &**first else { unreachable!() };
            'fields: for field in fields {
                let mut seen = Vec::new();
                for object in &objects {
                    let Kind::Object { fields } = &**object else { unreachable!() };
                    let Some(candidate) = fields.iter().find(|f| f.name == field.name) else { continue 'fields };
                    let kind = self.describe(&candidate.shape)?;
                    let Kind::Constant(value) = &*kind else { continue 'fields };
                    let text = print_literal(value);
                    if seen.contains(&text) {
                        continue 'fields;
                    }
                    seen.push(text);
                }
                discriminant = Some(field.name.clone());
                break;
            }
        }
        self.g.discriminants.insert(key, discriminant.clone());
        Ok(discriminant)
    }

    /// `discriminantField(union, index)`.
    pub(crate) fn discriminant_field(&mut self, union: &Rc<Kind>, index: usize) -> GResult<Option<String>> {
        let Kind::Union { alternatives, .. } = &**union else { unreachable!() };
        let kind = self.describe(&alternatives[index].shape)?;
        if !matches!(&*kind, Kind::Object { .. }) || self.evaluation(union)?.table_only == Some(index) {
            return Ok(None);
        }
        Ok(self.object_key(union, &kind)?.and_then(|(name, _, value)| value.map(|_| name)))
    }

    /// `warnIndistinct(union, members)`.
    fn warn_indistinct(&mut self, union: &Rc<Kind>, members: &[usize]) -> GResult<()> {
        let Kind::Union { alternatives, ty, .. } = &**union else { unreachable!() };
        let key = match ty {
            Some(t) => super::ShapeKey::Type(*t),
            None => super::ShapeKey::Kind(kind_key(union)),
        };
        if !self.g.warned_unions.insert(key) {
            return Ok(());
        }
        let mut names = Vec::new();
        for &i in members {
            names.push(format!("'{}'", self.alternative_name(&alternatives[i])?));
        }
        let written = match ty {
            Some(t) if super::typeinfo::alias_symbol(*t).is_some() => self.type_to_string(*t),
            _ => {
                let mut all = Vec::new();
                for a in alternatives {
                    all.push(self.alternative_name(a)?);
                }
                all.join(" | ")
            }
        };
        self.warn(&format!(
            "the union '{written}' has members a value cannot tell apart: {}. A value that fits more than one is written as the first of them in this order, without the parts only the others declare",
            names.join(", ")
        ));
        Ok(())
    }

    /// `warn(message)`: once per text and file.
    pub(crate) fn warn(&mut self, message: &str) {
        #[cfg(feature = "wire-harvest")]
        if self.g.companion {
            return;
        }
        let pos = self.cx.utf16_pos(self.g.diagnostic_node);
        let id = self.shared.file_id(&self.cx.file().path);
        let text = format!("{id}:{}:{} - {message}", pos.line + 1, pos.character + 1);
        if self.g.warned.insert(text) {
            self.cx.warning(self.g.diagnostic_node, message.to_owned());
        }
    }

    // --- fit ----------------------------------------------------------------------------------------------

    /// `fit(a, b)`.
    pub(crate) fn fit(&mut self, a: &Shape, b: &Shape) -> GResult<Fit> {
        if a.same(b) {
            return Ok(Fit::Whole);
        }
        let left = self.describe(a)?;
        let right = self.describe(b)?;
        if Rc::ptr_eq(&left, &right) {
            return Ok(Fit::Whole);
        }
        let key = (kind_key(&left), kind_key(&right));
        if let Some(known) = self.g.fits.get(&key) {
            return Ok(*known);
        }
        self.g.fits.insert(key, Fit::Whole);
        let result = self.compute_fit(&left, &right)?;
        self.g.fits.insert(key, result);
        Ok(result)
    }

    fn compute_fit(&mut self, a: &Rc<Kind>, b: &Rc<Kind>) -> GResult<Fit> {
        if let Kind::Optional(inner) = &**b {
            let present = self.fit(&Shape::Kind(a.clone()), inner)?;
            return Ok(if matches!(&**a, Kind::Optional(_) | Kind::Nothing) { worse(present, Fit::Whole) } else { present });
        }
        if matches!(&**b, Kind::Nothing) {
            return Ok(if matches!(&**a, Kind::Optional(_) | Kind::Nothing) { Fit::Whole } else { Fit::None });
        }
        if let Kind::Optional(inner) = &**a {
            return self.fit(inner, &Shape::Kind(b.clone()));
        }
        if let Kind::Union { alternatives, .. } = &**b {
            let mut result = Fit::None;
            for alt in alternatives {
                result = worse(result, self.fit(&Shape::Kind(a.clone()), &alt.shape)?);
            }
            return Ok(result);
        }
        if let Kind::Union { alternatives, .. } = &**a {
            let mut result = Fit::None;
            for alt in alternatives {
                result = worse(result, self.fit(&alt.shape, &Shape::Kind(b.clone()))?);
            }
            return Ok(result);
        }
        if let Kind::Blob { typeof_name } = &**a {
            return Ok(if typeof_name.is_none() || matches!(&**b, Kind::Blob { .. }) { Fit::Whole } else { Fit::None });
        }
        if matches!(&**b, Kind::Blob { .. }) {
            return Ok(if a.is_table() { Fit::Lossy } else { Fit::Whole });
        }
        if let Some(primitive) = self.primitive_fit(a, b) {
            return Ok(primitive);
        }

        // Tables. A set is a map from its elements to `true`.
        let as_map = |gn: &mut Self, kind: &Kind| -> Option<(Shape, Shape)> {
            match kind {
                Kind::Map(k, v) => Some((k.clone(), v.clone())),
                Kind::Set(e) => Some((e.clone(), gn.true_shape())),
                _ => None,
            }
        };
        let map_a = as_map(self, a);
        let map_b = as_map(self, b);

        if let Kind::Object { fields } = &**a {
            if let Kind::Object { .. } = &**b {
                return self.object_fit(a, b);
            }
            if let Some((key_b, value_b)) = &map_b {
                for field in fields {
                    if !self.is_required(&field.shape)? {
                        continue;
                    }
                    if !self.key_fits(key_b, &field.key)? || self.fit(&field.shape, value_b)? == Fit::None {
                        return Ok(Fit::None);
                    }
                }
                return Ok(Fit::Lossy);
            }
            for field in fields {
                if self.is_required(&field.shape)? {
                    return Ok(Fit::None);
                }
            }
            return Ok(Fit::Lossy);
        }

        if let Some((key_a, value_a)) = &map_a {
            if let Kind::Object { fields } = &**b {
                let mut result = Fit::Whole;
                for field in fields {
                    let value = if self.key_fits(key_a, &field.key)? { self.fit(value_a, &field.shape)? } else { Fit::None };
                    if value == Fit::None {
                        if self.is_required(&field.shape)? {
                            return Ok(Fit::None);
                        }
                        continue;
                    }
                    result = worse(result, value);
                }
                return Ok(result);
            }
            if let Some((key_b, value_b)) = &map_b {
                let k = self.fit(key_a, key_b)?;
                let v = self.fit(value_a, value_b)?;
                return Ok(at_least_empty(worst(&[k, v])));
            }
            if let Kind::Array(element) = &**b {
                return Ok(if self.key_fits(key_a, &TableKey::Num(1.0))? {
                    at_least_empty(self.fit(value_a, element)?)
                } else {
                    Fit::Whole
                });
            }
            if matches!(&**b, Kind::List { .. }) {
                return if self.key_fits(key_a, &TableKey::Num(1.0))? { self.list_into(b, value_a) } else { Ok(Fit::None) };
            }
            return Ok(Fit::None);
        }

        if let Kind::Array(element_a) = &**a {
            if let Kind::Array(element_b) = &**b {
                return Ok(at_least_empty(self.fit(element_a, element_b)?));
            }
            if matches!(&**b, Kind::List { .. }) {
                return self.list_into(b, element_a);
            }
            if let Some((key_b, value_b)) = &map_b {
                return Ok(if self.key_fits(key_b, &TableKey::Num(1.0))? {
                    at_least_empty(self.fit(element_a, value_b)?)
                } else {
                    Fit::Whole
                });
            }
            if let Kind::Object { fields } = &**b {
                for field in fields {
                    if self.is_required(&field.shape)? {
                        return Ok(Fit::None);
                    }
                }
                return Ok(Fit::Whole);
            }
            return Ok(Fit::None);
        }

        if let Kind::List { elements, rest, after } = &**a {
            let after_a: Vec<Shape> = after.clone().unwrap_or_default();
            if let Kind::List { elements: elements_b, rest: rest_b, after: after_b } = &**b {
                let after_b: Vec<Shape> = after_b.clone().unwrap_or_default();
                if elements_b.len() != elements.len() || rest.is_none() != rest_b.is_none() || after_b.len() != after_a.len() {
                    return Ok(Fit::None);
                }
                let mut fits = Vec::new();
                for (x, y) in elements.iter().zip(elements_b) {
                    fits.push(self.fit(x, y)?);
                }
                if let (Some(x), Some(y)) = (rest, rest_b) {
                    fits.push(at_least_empty(self.fit(x, y)?));
                }
                for (x, y) in after_a.iter().zip(&after_b) {
                    fits.push(self.fit(x, y)?);
                }
                return Ok(if fits.contains(&Fit::None) { Fit::None } else { worst(&fits) });
            }
            if let Kind::Array(element) = &**b {
                let mut fits = Vec::new();
                for x in elements.iter().chain(&after_a) {
                    fits.push(self.fit(x, element)?);
                }
                return Ok(if fits.contains(&Fit::None) { Fit::None } else { worst(&fits) });
            }
            return Ok(Fit::None);
        }
        Ok(Fit::None)
    }

    /// `TRUE`: a set's values, seen as a map's.
    fn true_shape(&mut self) -> Shape {
        if let Some(t) = &self.g.true_kind {
            return Shape::Kind(t.clone());
        }
        let k = self.kind(Kind::Constant(Lit::Bool(true)));
        self.g.true_kind = Some(k.clone());
        Shape::Kind(k)
    }

    fn object_fit(&mut self, a: &Rc<Kind>, b: &Rc<Kind>) -> GResult<Fit> {
        let (Kind::Object { fields: fa }, Kind::Object { fields: fb }) = (&**a, &**b) else { unreachable!() };
        let mut result = Fit::Whole;
        for field in fa {
            let Some(other) = fb.iter().find(|c| c.name == field.name) else {
                if self.is_required(&field.shape)? {
                    return Ok(Fit::None);
                }
                continue;
            };
            let value = self.fit(&field.shape, &other.shape)?;
            if value == Fit::None {
                return Ok(Fit::None);
            }
            result = worse(result, value);
        }
        for field in fb {
            if matches!(&*self.describe(&field.shape)?, Kind::Nothing) {
                continue;
            }
            if !fa.iter().any(|c| c.name == field.name) {
                return Ok(Fit::Lossy);
            }
        }
        Ok(result)
    }

    fn list_into(&mut self, list: &Rc<Kind>, element: &Shape) -> GResult<Fit> {
        let Kind::List { elements, rest, after } = &**list else { unreachable!() };
        let mut fits = Vec::new();
        for shape in elements.iter().chain(after.iter().flatten()) {
            fits.push(self.fit(element, shape)?);
        }
        if let Some(rest) = rest {
            fits.push(at_least_empty(self.fit(element, rest)?));
        }
        Ok(if fits.contains(&Fit::None) { Fit::None } else { worst(&fits) })
    }

    /// `keyFits(key, name)`.
    fn key_fits(&mut self, key: &Shape, name: &TableKey) -> GResult<bool> {
        let kind = self.describe(key)?;
        let written = match name {
            TableKey::Str(s) => super::typeinfo::json_string(s),
            TableKey::Num(n) => crate::js::number_to_string(*n),
        };
        Ok(match &*kind {
            Kind::String { .. } => matches!(name, TableKey::Str(_)),
            Kind::Number { .. } | Kind::Varint { .. } => matches!(name, TableKey::Num(_)),
            Kind::Constant(value) => literal_key(value) == written,
            Kind::Literals(values) => values.iter().any(|v| literal_key(v) == written),
            Kind::Optional(inner) => self.key_fits(inner, name)?,
            Kind::Union { alternatives, .. } => {
                for a in alternatives {
                    if self.key_fits(&a.shape, name)? {
                        return Ok(true);
                    }
                }
                false
            }
            Kind::Blob { typeof_name } => typeof_name.is_none(),
            _ => false,
        })
    }

    /// `primitiveFit(a, b)`.
    fn primitive_fit(&mut self, a: &Kind, b: &Kind) -> Option<Fit> {
        let left = primitive_type(a);
        let right = primitive_type(b);
        if left.is_none() && right.is_none() {
            return None;
        }
        if left.is_none() || right.is_none() || left != right {
            return Some(Fit::None);
        }
        let values = |kind: &Kind| -> Option<Vec<Lit>> {
            match kind {
                Kind::Constant(v) => Some(vec![v.clone()]),
                Kind::Literals(vs) => Some(vs.clone()),
                _ => None,
            }
        };
        let left_values = values(a);
        let right_values = values(b);
        if let (Some(lv), Some(rv)) = (&left_values, &right_values) {
            let keys: Vec<String> = lv.iter().map(literal_key).collect();
            return Some(if rv.iter().any(|v| keys.contains(&literal_key(v))) { Fit::Whole } else { Fit::None });
        }
        if left_values.is_some() {
            return Some(Fit::Whole);
        }
        if let (Kind::Enum(x), Kind::Enum(y)) = (a, b) {
            return Some(if x == y { Fit::Whole } else { Fit::None });
        }
        if let (Kind::Datatype(x), Kind::Datatype(y)) = (a, b) {
            return Some(if x == y { Fit::Whole } else { Fit::None });
        }
        if let Some(holds) = number_range(a) {
            let range = match &right_values {
                Some(rv) => literal_range(rv),
                None => number_range(b),
            };
            let Some((minimum, maximum, whole)) = range else { return Some(Fit::Lossy) };
            return Some(if minimum >= holds.0 && maximum <= holds.1 && (whole || !holds.2) { Fit::Whole } else { Fit::Lossy });
        }
        Some(Fit::Whole)
    }

    // --- width checks --------------------------------------------------------------------------------------

    /// `checkedWidth(kind, ctx)`.
    pub(crate) fn checked_width(&self, kind: &Kind, unchecked: bool) -> Option<String> {
        if self.g.checks.category == "none" || unchecked {
            return None;
        }
        let implicit = match kind {
            Kind::Number { width, implicit } => {
                if *width == Width::F64 {
                    return None;
                }
                *implicit
            }
            Kind::Varint { implicit } => *implicit,
            Kind::String { length, implicit } | Kind::Buffer { length, implicit } => {
                if matches!(length, LengthWidth::V | LengthWidth::U32) {
                    return None;
                }
                *implicit
            }
            _ => return None,
        };
        if self.g.checks.category == "implicit" && !implicit {
            return None;
        }
        Some(width_name(kind))
    }

    /// `numericFallback(union, ctx)`.
    pub(crate) fn numeric_fallback(&mut self, union: &Rc<Kind>, unchecked: bool) -> GResult<Option<(usize, String)>> {
        let Kind::Union { alternatives, whole, .. } = &**union else { unreachable!() };
        if whole.is_some() {
            return Ok(None);
        }
        let mut widths: Vec<String> = Vec::new();
        for a in alternatives {
            let kind = self.describe(&a.shape)?;
            if number_range(&kind).is_some() && !widths.contains(&width_name(&kind)) {
                widths.push(width_name(&kind));
            }
        }
        let mut index = None;
        for i in self.evaluation(union)?.order.clone() {
            let kind = self.describe(&alternatives[i].shape)?;
            if matches!(&*kind, Kind::Number { width: Width::F64, .. }) {
                return Ok(None);
            }
            if matches!(&*kind, Kind::Blob { typeof_name: None }) {
                return Ok(None);
            }
            if index.is_none() && number_range(&kind).is_some() && self.checked_width(&kind, unchecked).is_some() {
                index = Some(i);
            }
        }
        Ok(index.map(|i| (i, widths.join(" | "))))
    }

    /// `hasChecks(root)`.
    pub(crate) fn has_checks(&mut self, root: &Shape) -> GResult<bool> {
        let mut seen = Vec::new();
        if self.has_checks_walk(root, root, &mut seen)? {
            return Ok(true);
        }
        self.has_type_checks(root, false)
    }

    fn has_checks_walk(&mut self, root: &Shape, shape: &Shape, seen: &mut Vec<super::ShapeKey>) -> GResult<bool> {
        if seen.contains(&shape.key()) {
            return Ok(false);
        }
        seen.push(shape.key());
        if !shape.same(root)
            && let Shape::Type(t) = shape
            && self.always_hoisted(*t)?
        {
            return Ok(false);
        }
        let kind = self.describe(shape)?;
        Ok(match &*kind {
            Kind::Number { .. } | Kind::Varint { .. } | Kind::String { .. } | Kind::Buffer { .. } => {
                self.checked_width(&kind, false).is_some()
            }
            Kind::Optional(inner) => self.has_checks_walk(root, inner, seen)?,
            Kind::Array(e) | Kind::Set(e) => self.has_checks_walk(root, e, seen)?,
            Kind::Map(k, v) => self.has_checks_walk(root, k, seen)? || self.has_checks_walk(root, v, seen)?,
            Kind::List { elements, rest, after } => {
                for e in elements {
                    if self.has_checks_walk(root, e, seen)? {
                        return Ok(true);
                    }
                }
                if let Some(r) = rest
                    && self.has_checks_walk(root, r, seen)?
                {
                    return Ok(true);
                }
                for e in after.iter().flatten() {
                    if self.has_checks_walk(root, e, seen)? {
                        return Ok(true);
                    }
                }
                false
            }
            Kind::Object { fields } => {
                for f in fields {
                    if self.has_checks_walk(root, &f.shape, seen)? {
                        return Ok(true);
                    }
                }
                false
            }
            Kind::Union { alternatives, .. } => {
                if self.numeric_fallback(&kind, false)?.is_some() {
                    return Ok(true);
                }
                for a in alternatives {
                    let ak = self.describe(&a.shape)?;
                    if number_range(&ak).is_none() && self.has_checks_walk(root, &a.shape, seen)? {
                        return Ok(true);
                    }
                }
                false
            }
            _ => false,
        })
    }

    /// `hasTypeChecks(root, pass)`: `size` true for the size pass.
    pub(crate) fn has_type_checks(&mut self, root: &Shape, size: bool) -> GResult<bool> {
        if !self.g.checks.types {
            return Ok(false);
        }
        let mut seen = Vec::new();
        let mut seen_tested = Vec::new();
        self.type_checks_walk(root, root, false, size, &mut seen, &mut seen_tested)
    }

    fn type_checks_walk(
        &mut self,
        root: &Shape,
        shape: &Shape,
        tested: bool,
        size: bool,
        seen: &mut Vec<super::ShapeKey>,
        seen_tested: &mut Vec<super::ShapeKey>,
    ) -> GResult<bool> {
        let visited = if tested { &mut *seen_tested } else { &mut *seen };
        if visited.contains(&shape.key()) {
            return Ok(false);
        }
        visited.push(shape.key());
        if !shape.same(root)
            && let Shape::Type(t) = shape
            && self.always_hoisted(*t)?
        {
            return Ok(false);
        }
        let kind = self.describe(shape)?;
        if !tested && self.type_expectation(&kind)?.is_some() {
            let fixed = self.layout_of(shape)?.size.is_some();
            if fixed != size {
                return Ok(true);
            }
        }
        Ok(match &*kind {
            Kind::Optional(inner) => self.type_checks_walk(root, inner, tested, size, seen, seen_tested)?,
            Kind::Array(e) | Kind::Set(e) => self.type_checks_walk(root, e, false, size, seen, seen_tested)?,
            Kind::Map(k, v) => {
                self.type_checks_walk(root, k, false, size, seen, seen_tested)?
                    || self.type_checks_walk(root, v, false, size, seen, seen_tested)?
            }
            Kind::List { elements, rest, after } => {
                for e in elements.iter().chain(rest.iter()).chain(after.iter().flatten()) {
                    if self.type_checks_walk(root, e, false, size, seen, seen_tested)? {
                        return Ok(true);
                    }
                }
                false
            }
            Kind::Object { fields } => {
                for f in fields {
                    if self.type_checks_walk(root, &f.shape, false, size, seen, seen_tested)? {
                        return Ok(true);
                    }
                }
                false
            }
            Kind::Union { alternatives, .. } => {
                for a in alternatives {
                    if self.type_checks_walk(root, &a.shape, true, size, seen, seen_tested)? {
                        return Ok(true);
                    }
                }
                false
            }
            _ => false,
        })
    }

    /// `alwaysHoisted(type)`.
    pub(crate) fn always_hoisted(&mut self, ty: P<Type>) -> GResult<bool> {
        if self.g.hoisted.iter().any(|(t, _)| *t == ty) {
            return Ok(true);
        }
        if !self.can_hoist(ty)? || self.hoist_name(ty).is_none() {
            return Ok(false);
        }
        let structure = self.describe(&Shape::Type(ty))?;
        Ok(!matches!(&*structure, Kind::Array(_) | Kind::Set(_) | Kind::Map(..)))
    }

    // --- type checks ---------------------------------------------------------------------------------------

    /// `typeCheckIn(shape, pass, place)`: `size` true for the size pass.
    pub(crate) fn type_check_in(&mut self, shape: &Shape, size: bool, place: &Place) -> GResult<Option<Rc<Kind>>> {
        if !self.g.checks.types || place.tested || place.args {
            return Ok(None);
        }
        if self.layout_of(shape)?.size.is_some() == size {
            return Ok(None);
        }
        let kind = self.describe(shape)?;
        Ok(if self.type_expectation(&kind)?.is_some() { Some(kind) } else { None })
    }

    /// `typeExpectation(kind)`.
    pub(crate) fn type_expectation(&mut self, kind: &Rc<Kind>) -> GResult<Option<Expectation>> {
        let plain = |s: &str| Some(Expectation { expected: s.to_owned(), show: false });
        Ok(match &**kind {
            Kind::Number { .. } | Kind::Varint { .. } => plain("number"),
            Kind::String { .. } => plain("string"),
            Kind::Boolean => plain("boolean"),
            Kind::Buffer { .. } => plain("buffer"),
            Kind::Datatype(name) => plain(name),
            Kind::CFrame => plain("CFrame"),
            Kind::Enum(name) => Some(Expectation { expected: format!("Enum.{name}"), show: true }),
            Kind::Constant(value) => Some(Expectation { expected: literal_key(value), show: true }),
            Kind::Literals(values) => {
                Some(Expectation { expected: values.iter().map(literal_key).collect::<Vec<_>>().join(" | "), show: true })
            }
            Kind::Blob { typeof_name } => typeof_name.as_deref().and_then(plain),
            Kind::Array(_) | Kind::Set(_) | Kind::Map(..) | Kind::List { .. } | Kind::Object { .. } => plain("table"),
            Kind::Union { .. } => Some(Expectation { expected: self.union_text(kind)?, show: false }),
            _ => None,
        })
    }

    /// `unionText(union)`.
    fn union_text(&mut self, union: &Rc<Kind>) -> GResult<String> {
        let Kind::Union { alternatives, ty, .. } = &**union else { unreachable!() };
        if let Some(t) = ty
            && super::typeinfo::alias_symbol(*t).is_some()
        {
            return Ok(self.type_to_string(*t));
        }
        let mut names = Vec::new();
        for a in alternatives {
            names.push(self.alternative_name(a)?);
        }
        Ok(names.join(" | "))
    }

    /// `typeTest(kind, value)`.
    fn type_test(&mut self, kind: &Kind, value: Expr) -> Expr {
        match kind {
            Kind::Number { .. } | Kind::Varint { .. } => self.type_of_is(value, "number"),
            Kind::String { .. } => self.type_of_is(value, "string"),
            Kind::Boolean => self.type_of_is(value, "boolean"),
            Kind::Buffer { .. } => self.type_of_is(value, "buffer"),
            Kind::Datatype(name) => self.type_of_is(value, name),
            Kind::CFrame => self.type_of_is(value, "CFrame"),
            Kind::Enum(name) => self.enum_test(name, value),
            Kind::Constant(constant) => {
                self.check_globals_in(constant);
                luau::bin(value, Eq, constant.to_luau())
            }
            Kind::Blob { typeof_name } => self.type_of_is(value, typeof_name.as_deref().unwrap_or("")),
            _ => self.type_of_is(value, "table"),
        }
    }

    /// `enumTest(name, value)`: `typeof(v) == "EnumItem" and v.EnumType == Enum.<name>`.
    pub(crate) fn enum_test(&mut self, name: &str, value: Expr) -> Expr {
        let is_item = self.type_of_is(value.clone(), "EnumItem");
        let enum_global = self.global_ref("Enum");
        luau::bin(is_item, And, luau::bin(luau::prop(value, "EnumType"), Eq, luau::prop(enum_global, name)))
    }

    /// `failed(test)`: `a ~= b` for `a == b`, else `not (test)`.
    pub(crate) fn failed(test: Expr) -> Expr {
        match test {
            Expr::Binary(binary) if binary.operator == Eq => {
                let binary = *binary;
                luau::bin(binary.left, BinaryOperator::Ne, binary.right)
            }
            other => luau::not(other),
        }
    }

    /// `callTypeCheck(kind, value, place)`: `codec.checkType(expected, value, where[, true])`.
    pub(crate) fn call_type_check(&mut self, kind: &Rc<Kind>, value: Expr, place: &Place) -> GResult<Expr> {
        let expectation = self.type_expectation(kind)?.expect("a kind with a test");
        let where_ = self.where_of(place);
        let mut args = vec![luau::str(&expectation.expected), value, where_];
        if expectation.show {
            args.push(create::bool(true));
        }
        let helper = self.type_check_helper();
        Ok(luau::call(helper, args))
    }

    /// `typeCheckStatement(kind, value, place)`: `if <failed test> then <checkType> end`.
    pub(crate) fn type_check_statement(&mut self, kind: &Rc<Kind>, value: Expr, place: &Place) -> GResult<Stmt> {
        let test = self.type_test(kind, value.clone());
        let call = self.call_type_check(kind, value, place)?;
        Ok(luau::if_chain(vec![(Self::failed(test), vec![Stmt::Call(call)])], Vec::new()))
    }

    /// `emitTypeCheck(kind, value, out, place)`.
    pub(crate) fn emit_type_check(&mut self, kind: &Rc<Kind>, value: Expr, out: &Out, place: &Place) -> GResult<()> {
        if let Some(literal) = self.literal(&value)
            && let Some(ty) = literal.type_name()
        {
            if !literal_passes(kind, &literal, ty) {
                let call = self.call_type_check(kind, value, place)?;
                push(out, Stmt::Call(call));
            }
            return Ok(());
        }
        if matches!(&**kind, Kind::Constant(_)) || self.is_name(&value) {
            let statement = self.type_check_statement(kind, value, place)?;
            push(out, statement);
            return Ok(());
        }
        let v = self.uid("v");
        let statement = self.type_check_statement(kind, v.e(), place)?;
        push(out, luau::do_(vec![luau::local(&v.e(), Some(value)), statement]));
        Ok(())
    }

    /// `tablePrecheck(union, value, out, place)`.
    pub(crate) fn table_precheck(&mut self, union: &Rc<Kind>, value: Expr, out: &Out, place: &Place) -> GResult<()> {
        let Kind::Union { alternatives, .. } = &**union else { unreachable!() };
        let mut kinds = Vec::new();
        for a in alternatives {
            kinds.push(self.describe(&a.shape)?);
        }
        if !kinds.iter().all(|k| k.is_table()) {
            return Ok(());
        }
        let table_only = self.evaluation(union)?.table_only;
        let mut keyed = false;
        for (i, k) in kinds.iter().enumerate() {
            if matches!(&**k, Kind::Object { .. }) && Some(i) != table_only && self.object_key(union, k)?.is_some() {
                keyed = true;
                break;
            }
        }
        if !keyed {
            return Ok(());
        }
        let test = self.type_of_is(value.clone(), "table");
        let call = self.call_type_check(union, value, place)?;
        push(out, luau::if_chain(vec![(Self::failed(test), vec![Stmt::Call(call)])], Vec::new()));
        Ok(())
    }

    /// `fitsRange(n, range)`: `n >= min and n <= max and n % 1 == 0`, or for a float width
    /// `not (math.abs(n) > max and math.abs(n) < math.huge)`.
    pub(crate) fn fits_range(&mut self, n: Expr, (minimum, maximum, whole): (f64, f64, bool)) -> Expr {
        if !whole {
            return luau::not(self.outside_float(n, maximum));
        }
        luau::bin(
            luau::bin(luau::bin(n.clone(), Ge, luau::num(minimum)), And, luau::bin(n.clone(), Le, luau::num(maximum))),
            And,
            luau::bin(luau::bin(n, Mod, luau::num(1.0)), Eq, luau::num(0.0)),
        )
    }

    /// `math.abs(n) > max and math.abs(n) < math.huge`.
    fn outside_float(&mut self, n: Expr, maximum: f64) -> Expr {
        let a = self.math_call("abs", vec![n.clone()]);
        let b = self.math_call("abs", vec![n]);
        let huge = luau::prop(self.global_ref("math"), "huge");
        luau::bin(luau::bin(a, Gt, luau::num(maximum)), And, luau::bin(b, Lt, huge))
    }

    /// `failsRange(n, range)`.
    pub(crate) fn fails_range(&mut self, n: Expr, range: (f64, f64, bool)) -> Expr {
        if !range.2 {
            return self.outside_float(n, range.1);
        }
        luau::not(self.fits_range(n, range))
    }

    /// `isWhole(n)`: `n < 2^35 and 1 / n > 0 and n % 1 == 0`.
    pub(crate) fn is_whole(n: Expr) -> Expr {
        luau::bin(
            luau::bin(
                luau::bin(n.clone(), Lt, luau::num(VARINT_LIMIT)),
                And,
                luau::bin(luau::bin(luau::num(1.0), Div, n.clone()), Gt, luau::num(0.0)),
            ),
            And,
            luau::bin(luau::bin(n, Mod, luau::num(1.0)), Eq, luau::num(0.0)),
        )
    }

    /// `callCheck(width, value, ctx, unit)`: `codec.checkWidth(width, value, where[, unit])`.
    pub(crate) fn call_check(&mut self, width: &str, value: Expr, ctx: &Ctx, unit: Option<&str>) -> Expr {
        let where_ = self.where_of(&ctx.place);
        let mut args = vec![luau::str(width), value, where_];
        if let Some(unit) = unit {
            args.push(luau::str(unit));
        }
        let helper = self.check_helper();
        luau::call(helper, args)
    }

    /// `whereOf(place)` as text known when building, or `Err` with the code that builds it when the code runs (a
    /// hoisted function's `where` parameter, and its path).
    pub(crate) fn where_text(&self, place: &Place) -> Result<String, Expr> {
        if let Some(w) = place.where_ {
            return Err(match place.path.as_deref() {
                Some(path) if !path.is_empty() => luau::bin(w.e(), BinaryOperator::Concat, luau::str(path)),
                _ => w.e(),
            });
        }
        let path = match place.path.as_deref() {
            None | Some("") => "value",
            Some(p) => p,
        };
        Ok(match &place.site {
            Some(site) => format!("'{site}' {path}"),
            None => path.to_owned(),
        })
    }

    /// `whereOf(place)`.
    pub(crate) fn where_of(&self, place: &Place) -> Expr {
        match self.where_text(place) {
            Ok(text) => luau::str(&text),
            Err(e) => e,
        }
    }

    /// `passedWhere(place)`.
    pub(crate) fn passed_where(&self, place: &Place) -> Expr {
        if let Some(owner) = &place.owner {
            return luau::str(&format!("{owner}{}", place.path.as_deref().unwrap_or("")));
        }
        self.where_of(place)
    }

    /// `discriminate(union, index, value)`.
    pub(crate) fn discriminate(&mut self, union: &Rc<Kind>, index: usize, value: Expr) -> GResult<Expr> {
        if self.evaluation(union)?.table_only == Some(index) {
            return Ok(self.type_of_is(value, "table"));
        }
        let Kind::Union { alternatives, .. } = &**union else { unreachable!() };
        let alternative = alternatives[index].clone();
        let kind = self.describe(&alternative.shape)?;
        if matches!(&*kind, Kind::Object { .. })
            && let Some((_, key, constant)) = self.object_key(union, &kind)?
        {
            let field = super::typeinfo::key_access(value.clone(), &key);
            let test = match constant {
                Some(c) => {
                    self.check_globals_in(&c);
                    luau::bin(field, Eq, c.to_luau())
                }
                None => luau::bin(field, BinaryOperator::Ne, create::nil()),
            };
            let mut tables = true;
            for other in alternatives {
                if !self.describe(&other.shape)?.is_table() {
                    tables = false;
                    break;
                }
            }
            return Ok(if tables { test } else { luau::bin(self.type_of_is(value, "table"), And, test) });
        }
        Ok(match &*kind {
            Kind::Number { .. } | Kind::Varint { .. } => match number_range(&kind) {
                None => self.type_of_is(value, "number"),
                Some(range) => {
                    let is_number = self.type_of_is(value.clone(), "number");
                    let fits = self.fits_range(value, range);
                    luau::bin(is_number, And, fits)
                }
            },
            Kind::String { .. } => self.type_of_is(value, "string"),
            Kind::Boolean => self.type_of_is(value, "boolean"),
            Kind::Buffer { .. } => self.type_of_is(value, "buffer"),
            Kind::Datatype(name) => self.type_of_is(value, name),
            Kind::CFrame => self.type_of_is(value, "CFrame"),
            Kind::Enum(name) => self.enum_test(name, value),
            Kind::Literals(_) => {
                let (_, index) = self.literal_tables_for(&kind);
                luau::bin(luau::index(index, value), BinaryOperator::Ne, create::nil())
            }
            Kind::Constant(c) => {
                self.check_globals_in(c);
                luau::bin(value, Eq, c.to_luau())
            }
            Kind::Nothing => luau::bin(value, Eq, create::nil()),
            Kind::Blob { typeof_name } => match typeof_name {
                Some(name) => self.type_of_is(value, name),
                None => create::bool(true),
            },
            _ => {
                let Some(ty) = alternative.ty else { panic!("Flamework: cannot discriminate a synthetic {}", kind.name()) };
                let guard = self.guard_for(ty)?;
                luau::call(guard, vec![value])
            }
        })
    }
}

/// `literalPasses(kind, value, type)`.
fn literal_passes(kind: &Kind, value: &Lit, ty: &str) -> bool {
    if let Kind::Constant(c) = kind {
        return ty != "nil" && print_literal(value) == print_literal(c);
    }
    primitive_type(kind).as_deref() == Some(ty)
        && matches!(kind, Kind::Number { .. } | Kind::Varint { .. } | Kind::String { .. } | Kind::Boolean)
}
