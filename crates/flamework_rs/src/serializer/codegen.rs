//! The code: the top level (`buildSerializer`, `buildDecoder`, `encodeElements`), the size pass, the writes and the
//! reads, with the cursor that folds constant offsets and the compact form (`fitted`). Everything is built as Luau
//! ([`super::luau`]), indexes 1-based where the TS generator's were 0-based.

use std::cell::RefCell;
use std::rc::Rc;

use rbxts_luau::BinaryOperator as B;
use rbxts_luau::{AssignmentOperator, ElseBody, Expr, IfStatement, Stmt, create};
use tsrs_checker::TypeFlags;

use super::checks::{fits_statically, number_range};
use super::classify::datatype;
use super::hoist::{MALFORMED, Role};
use super::limits::{self, Choice, holds_locals};
use super::luau::{self, Part, is_literal, is_name};
use super::typeinfo::{key_access, key_name, key_segment, local_name};
use super::{EncodingSite, GResult, Gen, Id, Kind, LengthWidth, Layout, Shape, Width};

const HEADER_SIZE: u64 = 5;
const BLOB_SIZE: i64 = 4;
const ZERO_SIZE_COUNT_MAX: f64 = 65535.0;
const HALF: f64 = 4294967296.0;

/// A statement list several builders append to (a TS array shared by reference).
pub(crate) type Out = Rc<RefCell<Vec<Stmt>>>;

pub(crate) fn out() -> Out {
    Rc::new(RefCell::new(Vec::new()))
}

pub(crate) fn push(out: &Out, s: Stmt) {
    out.borrow_mut().push(s);
}

pub(crate) fn take(out: &Out) -> Vec<Stmt> {
    std::mem::take(&mut *out.borrow_mut())
}

fn num(n: f64) -> Expr {
    luau::num(n)
}

/// `target += value`.
fn add_assign(target: Expr, value: Expr) -> Stmt {
    luau::compound(target, AssignmentOperator::Add, value)
}

/// `if test then body end`.
fn if_then(test: Expr, body: Vec<Stmt>) -> Stmt {
    luau::if_chain(vec![(test, body)], Vec::new())
}

/// `if test then body else <rest>` of an if-chain built from its end: an `if` goes on as `elseif`, a `do` block is the
/// last `else` block.
fn if_else(test: Expr, body: Vec<Stmt>, rest: Stmt) -> Stmt {
    let else_body = match rest {
        Stmt::If(inner) => ElseBody::If(inner),
        Stmt::Do(block) => ElseBody::Block(block.statements),
        other => ElseBody::Block(vec![other]),
    };
    Stmt::If(Box::new(IfStatement { condition: test, statements: body, else_body }))
}

/// [`if_else`], or [`if_then`] without the rest.
fn if_chain_on(test: Expr, body: Vec<Stmt>, rest: Option<Stmt>) -> Stmt {
    match rest {
        Some(rest) => if_else(test, body, rest),
        None => if_then(test, body),
    }
}

/// `function(ids) body end`.
pub(crate) fn function(ids: &[Id], body: Vec<Stmt>) -> Expr {
    let params: Vec<Expr> = ids.iter().map(|id| id.e()).collect();
    luau::function(&params, body)
}

/// Where a value sits, for the checks' messages (`Place`).
#[derive(Clone, Debug, Default)]
pub(crate) struct Place {
    pub path: Option<String>,
    pub site: Option<String>,
    pub where_: Option<Id>,
    pub owner: Option<String>,
    pub args: bool,
    pub tested: bool,
    pub compared: Option<String>,
}

impl Place {
    /// `within(place, segment, root)`.
    pub(crate) fn within(&self, segment: &str, root: bool) -> Place {
        Place {
            path: Some(if root { segment.to_owned() } else { format!("{}{segment}", self.path.as_deref().unwrap_or("")) }),
            args: false,
            tested: false,
            compared: None,
            ..self.clone()
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Cursor {
    pub variable: Option<Id>,
    pub base: Option<Id>,
    pub offset: i64,
}

/// Where the code being built goes (`Ctx`): the cursor and the statement list are shared by the copies, as the
/// TS spreads shared them.
#[derive(Clone, Debug)]
pub(crate) struct Ctx {
    pub buf: Id,
    pub blobs: Option<Id>,
    pub cursor: Rc<RefCell<Cursor>>,
    pub out: Out,
    pub place: Place,
    pub unchecked: bool,
}

impl Ctx {
    pub(crate) fn new(buf: Id, blobs: Option<Id>, variable: Option<Id>, offset: i64, out: Out, place: Place) -> Ctx {
        Ctx {
            buf,
            blobs,
            cursor: Rc::new(RefCell::new(Cursor { variable, base: variable, offset })),
            out,
            place,
            unchecked: false,
        }
    }

    fn within(&self, segment: &str, root: bool) -> Ctx {
        Ctx { place: self.place.within(segment, root), ..self.clone() }
    }

    fn with_out(&self, out: Out) -> Ctx {
        Ctx { out, ..self.clone() }
    }

    fn variable(&self) -> Option<Id> {
        self.cursor.borrow().variable
    }
}

/// A byte total under construction (`Sum`).
#[derive(Default)]
struct Sum {
    constant: f64,
    terms: Vec<Expr>,
}

impl Sum {
    fn add(&mut self, part: Expr) {
        match luau::literal_number(&part) {
            Some(n) => self.constant += n,
            None => self.terms.push(part),
        }
    }

    fn add_number(&mut self, n: f64) {
        self.constant += n;
    }

    fn build(self) -> Expr {
        let mut total: Option<Expr> = None;
        for term in self.terms {
            total = Some(match total {
                Some(t) => luau::bin(t, B::Add, term),
                None => term,
            });
        }
        match total {
            Some(t) => luau::add(t, self.constant),
            None => num(self.constant),
        }
    }
}

/// The bytes of a varint known when building (`staticVarint`).
fn static_varint(mut n: u64) -> Vec<u64> {
    let mut bytes = Vec::new();
    while n >= 128 {
        bytes.push(n % 128 + 128);
        n /= 128;
    }
    bytes.push(n);
    bytes
}

/// The payload and blob list of each build of an encoding, in build order (`encodings`): the one `fitted` kept is
/// the encoding's (`kept(statements)`).
type Chosen = Rc<RefCell<Vec<(Id, Option<Id>)>>>;

/// The payload and blob list of the build `fitted_choice` kept: the compact rebuild's, or the first build's.
fn kept(chosen: &Chosen, rebuilt: bool) -> (Id, Option<Id>) {
    let builds = chosen.borrow();
    if rebuilt { *builds.last().expect("the rebuild ran") } else { builds[0] }
}

/// Records a `fitted` decision for the limit tests ([`limits::record`]), while they record.
fn record(
    what: Option<&str>,
    outer: u32,
    plain: &limits::Function,
    compact: Option<&limits::Function>,
    kept_compact: bool,
) {
    if !limits::record::on() {
        return;
    }
    let sources = |f: &limits::Function| [f.probe_source(limits::MARGIN + outer), f.probe_source(outer)];
    limits::record::push(limits::record::Decision {
        what: what.map(str::to_owned),
        outer,
        plain: sources(plain),
        compact: compact.map(sources),
        kept_compact,
        plain_locals: plain.locals(),
    });
}

/// A call site's packing, before naming.
pub(crate) struct Encoding {
    pub statements: Vec<Stmt>,
    pub payload: Option<Id>,
    pub blobs: Option<Id>,
}

/// A value of a call site (`values[i]`).
#[derive(Clone)]
pub(crate) enum SiteValues {
    List(Vec<Expr>),
    Table(Expr),
}

impl Gen<'_, '_, '_> {
    // --- top level -------------------------------------------------------------------------------------------

    /// `shapeText(shape)`.
    fn shape_text(&mut self, shape: &Shape) -> String {
        match shape {
            Shape::Kind(_) => "this value".into(),
            Shape::Type(t) => format!("the type '{}'", self.type_text(*t)),
        }
    }

    /// `buildSerializer(type, versioned)`.
    pub(crate) fn build_serializer(&mut self, ty: Shape, versioned: Option<(u32, String)>) -> GResult<Expr> {
        let layout = self.layout_of(&ty)?;
        self.count_uses(&ty)?;
        let header = match versioned {
            Some((version, name)) => {
                let text = self.layout_text(&ty)?;
                let hash = crate::sha::layout_hash(&text);
                #[cfg(feature = "layout-dump")]
                super::dump_layout(&name, &text, hash);
                Some(Header { version, hash, name })
            }
            None => None,
        };
        let value = self.uid("v");
        let named = self.shape_text(&ty);
        let serialize_body = self.fitted(
            &mut |gn| {
                let path = gn.root_path(&ty)?;
                gn.encode_body(&ty, layout, value, &path, header.as_ref())
            },
            &[value],
            Some(&named),
            None,
            0,
        )?;
        let serialize = function(&[value], serialize_body);

        let buf = self.uid("buf");
        let blobs = if layout.blobs { Some(self.uid("blobs")) } else { None };
        let params: Vec<Id> = std::iter::once(buf).chain(blobs).collect();
        let body = self.fitted(
            &mut |gn| {
                let statements = out();
                let result = gn.decode_body(&ty, layout, buf, blobs, &statements, header.as_ref())?;
                push(&statements, create::return_stmt(result));
                Ok(take(&statements))
            },
            &params,
            Some(&named),
            None,
            0,
        )?;
        let deserialize = function(&params, body);

        let mut fields = Vec::new();
        if let Some(h) = &header {
            fields.push((luau::str("version"), num(f64::from(h.version))));
        }
        fields.push((luau::str("serialize"), serialize));
        fields.push((luau::str("deserialize"), deserialize));
        Ok(create::map(fields))
    }

    /// `buildDecoder(list)`: `None` when the list carries nothing.
    pub(crate) fn build_decoder(&mut self, list: Shape) -> GResult<Option<Expr>> {
        let kind = self.describe(&list)?;
        if self.carries_nothing(&kind)? {
            return Ok(None);
        }
        let layout = self.layout_of(&list)?;
        self.count_uses(&list)?;
        let buf = self.uid("buf");
        let blobs = if layout.blobs { Some(self.uid("blobs")) } else { None };
        let params: Vec<Id> = std::iter::once(buf).chain(blobs).collect();
        let body = self.fitted(
            &mut |gn| {
                let statements = out();
                let result = gn.decode_body(&list, layout, buf, blobs, &statements, None)?;
                push(&statements, create::return_stmt(result));
                Ok(take(&statements))
            },
            &params,
            Some("these arguments"),
            None,
            0,
        )?;
        Ok(Some(function(&params, body)))
    }

    /// `encodeElements(list, values, site)`.
    pub(crate) fn encode_elements(&mut self, list: Shape, values: SiteValues, site: EncodingSite) -> GResult<Encoding> {
        let kind = self.describe(&list)?;
        if self.carries_nothing(&kind)? {
            return Ok(Encoding { statements: vec![], payload: None, blobs: None });
        }
        let Kind::List { elements, rest, after } = &*kind else { unreachable!() };
        let layout = self.layout_of(&list)?;
        self.count_uses(&list)?;
        let chosen: Chosen = Rc::new(RefCell::new(Vec::new()));

        let values = match values {
            SiteValues::Table(table) => {
                let chosen_in = chosen.clone();
                let (statements, rebuilt) = self.fitted_choice(
                    &mut |gn| {
                        let body = out();
                        let place = Place { path: Some(String::new()), site: site.name.clone(), args: true, ..Place::default() };
                        let encoding = gn.encode_into(&list, layout, table.clone(), &body, &place, None)?;
                        chosen_in.borrow_mut().push(encoding);
                        Ok(take(&body))
                    },
                    &[],
                    None,
                    None,
                    site.outer,
                )?;
                let (buf, blobs) = kept(&chosen, rebuilt);
                return Ok(Encoding { statements, payload: Some(buf), blobs });
            }
            SiteValues::List(values) => values,
        };

        let after: Vec<Shape> = after.clone().unwrap_or_default();
        let after_values: Vec<Expr> = if !after.is_empty() { values[values.len() - after.len()..].to_vec() } else { vec![] };
        let rest_values: Vec<Expr> = if values.len() > elements.len() + after_values.len() {
            values[elements.len()..values.len() - after_values.len()].to_vec()
        } else {
            vec![]
        };
        if !rest_values.is_empty() && rest.is_none() {
            return Err(self.fail("more arguments than the list has elements"));
        }
        for _ in 1..rest_values.len().max(1) {
            self.count_uses(rest.as_ref().unwrap())?;
        }
        let count_bytes = static_varint(rest_values.len() as u64);
        // A function's result is packed in a function of its own, whose one parameter is the value.
        let own: Vec<Id> = match (site.result, values.first()) {
            (true, Some(first)) => crate::luau_names::placeholder_index(first).map(|i| Id(i as u32)).into_iter().collect(),
            _ => Vec::new(),
        };
        let chosen_in = chosen.clone();
        let tail_chosen = chosen.clone();
        let mut tail = move |_gen: &mut Self| -> Vec<Stmt> {
            let (buf, blobs) = *tail_chosen.borrow().last().expect("the build ran");
            let list = std::iter::once(buf).chain(blobs).map(Id::e).collect();
            vec![create::return_stmt(create::array(list))]
        };
        let elements = elements.clone();
        let rest = rest.clone();
        let (statements, rebuilt) = self.fitted_choice(
            &mut |gn| {
                let body = out();
                let encoding = gn.encode_values(
                    &elements,
                    rest.as_ref(),
                    &after,
                    layout,
                    &site,
                    &body,
                    &values,
                    &rest_values,
                    &after_values,
                    &count_bytes,
                )?;
                chosen_in.borrow_mut().push(encoding);
                Ok(take(&body))
            },
            &own,
            None,
            if site.result { Some(&mut tail) } else { None },
            site.outer,
        )?;
        let (buf, blobs) = kept(&chosen, rebuilt);
        Ok(Encoding { statements, payload: Some(buf), blobs })
    }

    /// `encodeValues(...)`.
    #[allow(clippy::too_many_arguments)]
    fn encode_values(
        &mut self,
        elements: &[Shape],
        rest: Option<&Shape>,
        after: &[Shape],
        layout: Layout,
        site: &EncodingSite,
        statements: &Out,
        values: &[Expr],
        rest_values: &[Expr],
        after_values: &[Expr],
        count_bytes: &[u64],
    ) -> GResult<(Id, Option<Id>)> {
        let element_value = |i: usize| values.get(i).cloned().unwrap_or_else(create::nil);
        let size_place = |i: usize| Place {
            path: Some(if site.result { "result".into() } else { format!("[{i}]") }),
            site: site.name.clone(),
            ..Place::default()
        };
        let mut total = Sum::default();
        let parts: Vec<(Shape, Expr, Place)> =
            elements.iter().enumerate().map(|(i, e)| (e.clone(), element_value(i), size_place(i))).collect();
        self.add_sizes(statements, &mut total, parts)?;
        if let Some(rest) = rest {
            total.add_number(count_bytes.len() as f64);
            let start = elements.len();
            let parts = rest_values.iter().enumerate().map(|(i, v)| (rest.clone(), v.clone(), size_place(start + i))).collect();
            self.add_sizes(statements, &mut total, parts)?;
            let parts = after
                .iter()
                .enumerate()
                .map(|(i, e)| (e.clone(), after_values[i].clone(), size_place(start + rest_values.len() + i)))
                .collect();
            self.add_sizes(statements, &mut total, parts)?;
        }

        let size = total.build();
        let fixed = luau::literal_number(&size).is_some();
        let buf = self.uid("buf");
        let create = self.buffer_call("create", vec![size]);
        push(statements, luau::local(&buf.e(), Some(create)));
        let blobs = if layout.blobs { Some(self.uid("blobs")) } else { None };
        if let Some(b) = blobs {
            let array = self.construct("Array", vec![]);
            push(statements, luau::local(&b.e(), Some(array)));
        }
        let variable = if fixed { None } else { Some(self.uid("o")) };
        if let Some(v) = variable {
            push(statements, luau::local(&v.e(), Some(num(0.0))));
        }
        let ctx = Ctx::new(buf, blobs, variable, 0, statements.clone(), Place { site: site.name.clone(), ..Place::default() });
        let argument = |i: usize| ctx.within(&if site.result { "result".into() } else { format!("[{i}]") }, true);
        let compact = self.g.compact;
        for (i, element) in elements.iter().enumerate() {
            let c = argument(i);
            if compact { self.emit_scoped_write(element, element_value(i), &c)? } else { self.emit_write(element, element_value(i), &c)? }
        }
        if let Some(rest) = rest {
            for &byte in count_bytes {
                let at = self.at(&ctx, 0);
                let write = self.buffer_call("writeu8", vec![buf.e(), at, num(byte as f64)]);
                push(statements, Stmt::Call(write));
                ctx.cursor.borrow_mut().offset += 1;
            }
            let start = elements.len();
            for (i, value) in rest_values.iter().enumerate() {
                let c = argument(start + i);
                if compact { self.emit_scoped_write(rest, value.clone(), &c)? } else { self.emit_write(rest, value.clone(), &c)? }
            }
            for (i, element) in after.iter().enumerate() {
                let c = argument(start + rest_values.len() + i);
                if compact {
                    self.emit_scoped_write(element, after_values[i].clone(), &c)?
                } else {
                    self.emit_write(element, after_values[i].clone(), &c)?
                }
            }
        }
        Ok((buf, blobs))
    }

    /// `encodeBody(shape, layout, value, path, header)`.
    fn encode_body(&mut self, shape: &Shape, layout: Layout, value: Id, path: &str, header: Option<&Header>) -> GResult<Vec<Stmt>> {
        let body = out();
        let place = Place { path: Some(path.to_owned()), ..Place::default() };
        let (buf, blobs) = self.encode_into(shape, layout, value.e(), &body, &place, header)?;
        let result = match blobs {
            Some(b) => luau::returns(vec![buf.e(), b.e()]),
            None => create::return_stmt(buf.e()),
        };
        push(&body, result);
        Ok(take(&body))
    }

    /// `encodeInto(shape, layout, value, body, where, header)`.
    fn encode_into(
        &mut self,
        shape: &Shape,
        layout: Layout,
        value: Expr,
        body: &Out,
        place: &Place,
        header: Option<&Header>,
    ) -> GResult<(Id, Option<Id>)> {
        let size = self.emit_size(shape, value.clone(), body, place)?;
        let buf = self.uid("buf");
        let total = if header.is_some() { luau::add(size, HEADER_SIZE as f64) } else { size };
        let create = self.buffer_call("create", vec![total]);
        push(body, luau::local(&buf.e(), Some(create)));
        if let Some(h) = header {
            let a = self.buffer_call("writeu8", vec![buf.e(), num(0.0), num(f64::from(h.version))]);
            push(body, Stmt::Call(a));
            let b = self.buffer_call("writeu32", vec![buf.e(), num(1.0), num(f64::from(h.hash))]);
            push(body, Stmt::Call(b));
        }
        let start = if header.is_some() { HEADER_SIZE as i64 } else { 0 };
        let blobs = if layout.blobs { Some(self.uid("blobs")) } else { None };
        if let Some(b) = blobs {
            let array = self.construct("Array", vec![]);
            push(body, luau::local(&b.e(), Some(array)));
        }
        let top = match shape {
            Shape::Type(t) => self.hoist(*t)?,
            Shape::Kind(_) => None,
        };
        if let Some(top) = top {
            let mut args = vec![buf.e(), num(start as f64), value];
            if let Some(b) = blobs {
                args.push(b.e());
            }
            if top.checks {
                let where_place = Place { path: place.path.clone(), site: place.site.clone(), ..Place::default() };
                args.push(self.where_of(&where_place));
            }
            let call = self.call_hoisted(&top, Role::W, args);
            push(body, Stmt::Call(call));
        } else {
            let variable = if layout.size.is_none() { Some(self.uid("o")) } else { None };
            if let Some(v) = variable {
                push(body, luau::local(&v.e(), Some(num(start as f64))));
            }
            let ctx = Ctx::new(buf, blobs, variable, if variable.is_some() { 0 } else { start }, body.clone(), Place {
                path: place.path.clone(),
                site: place.site.clone(),
                args: place.args,
                ..Place::default()
            });
            self.emit_write(shape, value, &ctx)?;
        }
        Ok((buf, blobs))
    }

    /// `decodeBody(shape, layout, buf, blobs, body, header)`.
    fn decode_body(
        &mut self,
        shape: &Shape,
        layout: Layout,
        buf: Id,
        blobs: Option<Id>,
        body: &Out,
        header: Option<&Header>,
    ) -> GResult<Expr> {
        if let Some(h) = header {
            self.check_header(buf, h, body);
        }
        let start = if header.is_some() { HEADER_SIZE as i64 } else { 0 };
        let length = self.buffer_call("len", vec![buf.e()]);
        if layout.zeros {
            let zeros = self.zero_tally();
            push(body, luau::assign(zeros.e(), num(0.0)));
        }
        let top = match shape {
            Shape::Type(t) => self.hoist(*t)?,
            Shape::Kind(_) => None,
        };
        if let Some(top) = top {
            let value = self.uid("value");
            let end = self.uid("o");
            let mut args = vec![buf.e(), num(start as f64)];
            if let Some(b) = blobs {
                args.push(b.e());
            }
            let call = self.call_hoisted(&top, Role::R, args);
            push(body, luau::locals(&[value.e(), end.e()], vec![call]));
            let raise = self.raise(MALFORMED);
            push(body, if_then(luau::bin(end.e(), B::Ne, length), vec![raise]));
            return Ok(value.e());
        }
        let variable = if layout.size.is_none() { Some(self.uid("o")) } else { None };
        if let Some(v) = variable {
            push(body, luau::local(&v.e(), Some(num(start as f64))));
        }
        let ctx = Ctx::new(buf, blobs, variable, if variable.is_some() { 0 } else { start }, body.clone(), Place::default());
        let mut result = self.emit_read(shape, &ctx)?;
        if !is_name(&result) && !is_literal(&result) {
            result = self.bind(body, result, "value");
        }
        if let Some(v) = variable {
            self.sync(&ctx);
            let raise = self.raise(MALFORMED);
            push(body, if_then(luau::bin(v.e(), B::Ne, length), vec![raise]));
        } else {
            let raise = self.raise(MALFORMED);
            let expected = num((layout.size.unwrap() as i64 + start) as f64);
            push(body, if_then(luau::bin(length, B::Ne, expected), vec![raise]));
        }
        Ok(result)
    }

    /// `checkHeader(buf, header, body)`.
    fn check_header(&mut self, buf: Id, header: &Header, body: &Out) {
        let Header { version, hash, name } = header;
        let len = self.buffer_call("len", vec![buf.e()]);
        let raise = self.raise(&format!(
            "[Flamework] this buffer is too short for the header of {name}, version {version}: no versioned serializer wrote it"
        ));
        push(body, if_then(luau::bin(len, B::Lt, num(HEADER_SIZE as f64)), vec![raise]));

        let written = |gn: &mut Self| gn.buffer_call("readu8", vec![buf.e(), num(0.0)]);
        let condition = luau::bin(written(self), B::Ne, num(f64::from(*version)));
        let message = luau::interp(vec![
            Part::Text("[Flamework] this buffer is version ".into()),
            Part::Expr(written(self)),
            Part::Text(format!(
                ", and this serializer of {name} reads version {version}: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without"
            )),
        ]);
        let raise = self.raise_with(message);
        push(body, if_then(condition, vec![raise]));

        let read = self.buffer_call("readu32", vec![buf.e(), num(1.0)]);
        let raise = self.raise(&format!(
            "[Flamework] {name}'s layout changed since this buffer was written as version {version}; bump the version and keep a reader for the old one"
        ));
        push(body, if_then(luau::bin(read, B::Ne, num(f64::from(*hash))), vec![raise]));
    }

    // --- cursor ----------------------------------------------------------------------------------------------

    /// `at(ctx, extra)`.
    pub(crate) fn at(&self, ctx: &Ctx, extra: i64) -> Expr {
        let cursor = ctx.cursor.borrow();
        let offset = cursor.offset + extra;
        match cursor.base {
            Some(base) => luau::add(base.e(), offset as f64),
            None => num(offset as f64),
        }
    }

    /// `sync(ctx)`.
    pub(crate) fn sync(&self, ctx: &Ctx) {
        let (base, variable, offset) = {
            let c = ctx.cursor.borrow();
            (c.base, c.variable, c.offset)
        };
        if base == variable && offset == 0 {
            return;
        }
        let variable = variable.expect("Flamework: a variable-size write inside a fixed layout");
        if base == Some(variable) {
            push(&ctx.out, add_assign(variable.e(), num(offset as f64)));
        } else {
            push(&ctx.out, luau::assign(variable.e(), self.at(ctx, 0)));
        }
        let mut c = ctx.cursor.borrow_mut();
        c.base = Some(variable);
        c.offset = 0;
    }

    /// `advanceBy(ctx, amount)`.
    fn advance_by(&self, ctx: &Ctx, amount: Expr) {
        let (base, variable, offset) = {
            let c = ctx.cursor.borrow();
            (c.base, c.variable, c.offset)
        };
        let variable = variable.expect("Flamework: a variable-size write inside a fixed layout");
        if base == Some(variable) {
            push(&ctx.out, add_assign(variable.e(), luau::add(amount, offset as f64)));
        } else {
            push(&ctx.out, luau::assign(variable.e(), luau::plus(self.at(ctx, 0), amount)));
        }
        let mut c = ctx.cursor.borrow_mut();
        c.base = Some(variable);
        c.offset = 0;
    }

    /// `branch(ctx, build)`.
    fn branch(&mut self, ctx: &Ctx, build: &mut dyn FnMut(&mut Self, &Ctx) -> GResult<()>) -> GResult<Vec<Stmt>> {
        if let Some(variable) = ctx.variable() {
            self.sync(ctx);
            let child = Ctx {
                cursor: Rc::new(RefCell::new(Cursor { variable: Some(variable), base: Some(variable), offset: 0 })),
                out: out(),
                ..ctx.clone()
            };
            build(self, &child)?;
            self.sync(&child);
            return Ok(take(&child.out));
        }
        let copy = ctx.cursor.borrow().clone();
        let child = Ctx { cursor: Rc::new(RefCell::new(copy)), out: out(), ..ctx.clone() };
        build(self, &child)?;
        Ok(take(&child.out))
    }

    /// `bind(out, value, hint)`: the value read once into a new local, or the name itself.
    pub(crate) fn bind(&mut self, out_list: &Out, value: Expr, hint: &str) -> Expr {
        if self.is_name(&value) {
            return value;
        }
        let id = self.uid(hint);
        push(out_list, luau::local(&id.e(), Some(value)));
        id.e()
    }

    // --- compact functions -----------------------------------------------------------------------------------

    /// `fitted(build, parameters, what, tail, outer)`: built plainly, and again compactly when the plain build does not
    /// fit Luau's limits with [`limits::MARGIN`] to spare (DESIGN §3.2, [`limits`]).
    #[allow(clippy::type_complexity)]
    pub(crate) fn fitted(
        &mut self,
        build: &mut dyn FnMut(&mut Self) -> GResult<Vec<Stmt>>,
        parameters: &[Id],
        what: Option<&str>,
        tail: Option<&mut dyn FnMut(&mut Self) -> Vec<Stmt>>,
        outer: u32,
    ) -> GResult<Vec<Stmt>> {
        self.fitted_choice(build, parameters, what, tail, outer).map(|(body, _)| body)
    }

    /// [`Self::fitted`], saying which build it kept: `true` for the compact rebuild, `false` for the first build.
    /// `outer` is the locals live around the code (a call site's), `tail` statements measured after the body, and
    /// `what` names the value in a refusal (a call site, `None`, keeps its compact build instead).
    #[allow(clippy::type_complexity)]
    pub(crate) fn fitted_choice(
        &mut self,
        build: &mut dyn FnMut(&mut Self) -> GResult<Vec<Stmt>>,
        parameters: &[Id],
        what: Option<&str>,
        mut tail: Option<&mut dyn FnMut(&mut Self) -> Vec<Stmt>>,
        outer: u32,
    ) -> GResult<(Vec<Stmt>, bool)> {
        let previous = self.g.compact;
        self.g.compact = false;
        let result = (|| {
            let mut measure = |gn: &mut Self, body: &[Stmt]| {
                let mut all = body.to_vec();
                if let Some(tail) = tail.as_mut() {
                    all.extend(tail(gn));
                }
                limits::Function::new(parameters, all)
            };
            let body = build(self)?;
            let recording = limits::record::on();
            let mut plain = measure(self, &body);
            if !recording && plain.surely_fits(limits::MARGIN + outer) {
                return Ok((body, false));
            }
            // what the file hoisted so far ahead of its functions, in `take_hoisted`'s order
            let hoisted = |gn: &Self| [gn.g.declarations.len(), gn.g.tables.len()];
            let prelude = |gn: &Self| Rc::new([gn.g.declarations.as_slice(), gn.g.tables.as_slice()].concat());
            let before = hoisted(self);
            plain.prelude = prelude(self);
            let keep = limits::keeps_plain(&plain, outer).map_err(|error| self.does_not_compile(what, error))?;
            if keep {
                record(what, outer, &plain, None, false);
                return Ok((body, false));
            }
            self.g.compact = true;
            let rebuilt = build(self)?;
            let mut compact = measure(self, &rebuilt);
            if hoisted(self) != before {
                // what the rebuild hoisted
                plain.prelude = prelude(self);
            }
            compact.prelude = plain.prelude.clone();
            let choice = limits::choose(&plain, &compact, outer).map_err(|error| self.does_not_compile(what, error))?;
            record(what, outer, &plain, Some(&compact), choice != Choice::Plain);
            match choice {
                Choice::Plain => Ok((body, false)),
                Choice::Compact => Ok((rebuilt, true)),
                Choice::Refused(reason) => match what {
                    Some(what) => Err(self.fail(&format!(
                        "the code for {what} {reason}, built one field at a time or not. Split the type into smaller named types"
                    ))),
                    None => Ok((rebuilt, true)),
                },
            }
        })();
        self.g.compact = previous;
        result
    }

    /// A generated function Luau's compiler refuses for something other than its limits: a bug in the generator.
    fn does_not_compile(&mut self, what: Option<&str>, error: rbxts_luau_check::CompileError) -> rbxts_plugin::Abort {
        let what = what.unwrap_or("this call");
        let diagnostic = rbxts_plugin::PluginDiagnostic::error(
            self.g.diagnostic_node,
            format!("Flamework's generated code for {what} does not compile: Luau says \"{}\".", error.message),
        )
        .with_hint("This is a bug in Flamework: please report it, with the file.");
        self.cx.report(diagnostic).expect("an error")
    }

    /// `addSizes(out, total, parts)`.
    fn add_sizes(&mut self, out_list: &Out, total: &mut Sum, parts: Vec<(Shape, Expr, Place)>) -> GResult<()> {
        if !self.g.compact {
            for (shape, value, place) in parts {
                let size = self.emit_size(&shape, value, out_list, &place)?;
                total.add(size);
            }
            return Ok(());
        }
        let mut running: Option<Id> = None;
        for (shape, value, place) in parts {
            let body = out();
            let size = self.emit_size(&shape, value, &body, &place)?;
            if luau::literal_number(&size).is_some() && body.borrow().is_empty() {
                total.add(size);
                continue;
            }
            let r = match running {
                Some(r) => r,
                None => {
                    let r = self.uid("size");
                    push(out_list, luau::local(&r.e(), Some(num(0.0))));
                    running = Some(r);
                    r
                }
            };
            push(&body, add_assign(r.e(), size));
            self.push_scoped(out_list, take(&body));
        }
        if let Some(r) = running {
            total.add(r.e());
        }
        Ok(())
    }

    /// `pushScoped(out, body)`.
    fn push_scoped(&self, out_list: &Out, body: Vec<Stmt>) {
        if holds_locals(&body) {
            push(out_list, luau::do_(body));
        } else {
            out_list.borrow_mut().extend(body);
        }
    }

    // --- size ------------------------------------------------------------------------------------------------

    /// `emitSize(shape, value, out, place)`.
    pub(crate) fn emit_size(&mut self, shape: &Shape, value: Expr, out_list: &Out, place: &Place) -> GResult<Expr> {
        let layout = self.layout_of(shape)?;
        if let Some(size) = layout.size {
            return Ok(num(size as f64));
        }
        if let Shape::Type(t) = shape
            && let Some(info) = self.hoist(*t)?
        {
            let args = if info.size_checks { vec![value, self.passed_where(place)] } else { vec![value] };
            return Ok(self.call_hoisted(&info, Role::S, args));
        }
        let kind = self.describe(shape)?;
        let typed = self.type_check_in(shape, true, place)?.is_some();
        if typed && !matches!(&*kind, Kind::Union { .. }) {
            self.emit_type_check(&kind, value.clone(), out_list, place)?;
        }
        match &*kind {
            Kind::String { length, .. } => Ok(self.size_with_length(out_list, *length, luau::len(value))),
            Kind::Buffer { length, .. } => {
                let len = self.buffer_call("len", vec![value]);
                Ok(self.size_with_length(out_list, *length, len))
            }
            Kind::Varint { .. } => {
                let v = self.varint_helpers();
                Ok(luau::call(v.size.e(), vec![value]))
            }
            Kind::Optional(inner) => {
                if matches!(value, Expr::Nil) {
                    return Ok(num(1.0));
                }
                let inner_layout = self.layout_of(inner)?;
                let v = self.bind(out_list, value, "v");
                if let Some(size) = inner_layout.size {
                    return Ok(luau::if_expr(luau::bin(v, B::Ne, create::nil()), num(1.0 + size as f64), num(1.0)));
                }
                let total = self.uid("size");
                push(out_list, luau::local(&total.e(), Some(num(1.0))));
                let body = out();
                let size = self.emit_size(inner, v.clone(), &body, place)?;
                push(&body, add_assign(total.e(), size));
                push(out_list, if_then(luau::bin(v, B::Ne, create::nil()), take(&body)));
                Ok(total.e())
            }
            Kind::Array(element) => {
                let array = self.bind(out_list, value, "array");
                let count = self.bind(out_list, luau::len(array.clone()), "n");
                let vsize = self.varint_helpers().size;
                let prefix = luau::call(vsize.e(), vec![count.clone()]);
                let element_layout = self.layout_of(element)?;
                if let Some(size) = element_layout.size {
                    return Ok(counted_size(prefix, count, size));
                }
                let total = self.uid("size");
                let index = self.uid("i");
                push(out_list, luau::local(&total.e(), Some(prefix)));
                let body = out();
                let hole = if self.allows_nil(element)? { None } else { Some((place.clone(), "the array".to_owned())) };
                let item = self.element_at(array, index, &body, hole);
                let size = self.emit_size(element, item, &body, &place.within("[]", false))?;
                push(&body, add_assign(total.e(), size));
                let range = self.for_range(index, num(1.0), count, take(&body));
                push(out_list, range);
                Ok(total.e())
            }
            Kind::Set(element) => {
                let set = self.bind(out_list, value, "set");
                let element_layout = self.layout_of(element)?;
                let item = self.uid("item");
                let element = element.clone();
                self.counted_in_pass(out_list, set, &[item], &mut |gn, body| match element_layout.size {
                    Some(size) => Ok(num(size as f64)),
                    None => gn.emit_size(&element, item.e(), body, &place.within("[]", false)),
                })
            }
            Kind::Map(key_shape, value_shape) => {
                let map = self.bind(out_list, value, "map");
                let key = self.uid("key");
                let entry = self.uid("entry");
                let (key_shape, value_shape) = (key_shape.clone(), value_shape.clone());
                self.counted_in_pass(out_list, map, &[key, entry], &mut |gn, body| {
                    let k = gn.emit_size(&key_shape, key.e(), body, &place.within("<key>", false))?;
                    let v = gn.emit_size(&value_shape, entry.e(), body, &place.within("<value>", false))?;
                    Ok(luau::plus(k, v))
                })
            }
            Kind::List { elements, rest, after } => {
                let list = self.bind(out_list, value, "list");
                let mut total = Sum::default();
                let parts = elements
                    .iter()
                    .enumerate()
                    .map(|(i, e)| (e.clone(), luau::index(list.clone(), num((i + 1) as f64)), place.within(&format!("[{i}]"), false)))
                    .collect();
                self.add_sizes(out_list, &mut total, parts)?;
                let Some(rest) = rest else { return Ok(total.build()) };
                let rest_layout = self.layout_of(rest)?;
                let after: Vec<Shape> = after.clone().unwrap_or_default();
                let count = self.rest_count(out_list, list.clone(), elements.len() + after.len());
                let parts = after
                    .iter()
                    .enumerate()
                    .map(|(i, e)| {
                        let at = luau::add(count.e(), (elements.len() + i + 1) as f64);
                        (e.clone(), luau::index(list.clone(), at), place.within("[]", false))
                    })
                    .collect();
                self.add_sizes(out_list, &mut total, parts)?;
                let vsize = self.varint_helpers().size;
                let prefix = luau::call(vsize.e(), vec![count.e()]);
                if let Some(size) = rest_layout.size {
                    total.add(counted_size(prefix, count.e(), size));
                    return Ok(total.build());
                }
                let sum = self.uid("size");
                let index = self.uid("i");
                total.add(prefix);
                push(out_list, luau::local(&sum.e(), Some(total.build())));
                let body = out();
                let element = if self.allows_nil(rest)? {
                    luau::index(list.clone(), index.e())
                } else {
                    let container = tuple_name(place);
                    self.element_at(list.clone(), index, &body, Some((place.clone(), container)))
                };
                let size = self.emit_size(rest, element, &body, &place.within("[]", false))?;
                push(&body, add_assign(sum.e(), size));
                let range = self.for_range(
                    index,
                    num((elements.len() + 1) as f64),
                    luau::add(count.e(), elements.len() as f64),
                    take(&body),
                );
                push(out_list, range);
                Ok(sum.e())
            }
            Kind::Object { fields } => {
                let object = self.bind(out_list, value, "object");
                let mut total = Sum::default();
                let parts = fields
                    .iter()
                    .map(|f| (f.shape.clone(), key_access(object.clone(), &f.key), place.within(&key_segment(&f.key), false)))
                    .collect();
                self.add_sizes(out_list, &mut total, parts)?;
                Ok(total.build())
            }
            Kind::Union { alternatives, whole, .. } => {
                let v = self.bind(out_list, value, "v");
                let total = self.uid("size");
                push(out_list, luau::local(&total.e(), Some(num(1.0))));
                if typed {
                    self.table_precheck(&kind, v.clone(), out_list, place)?;
                }
                let mut chain: Option<Stmt> = if typed {
                    let call = self.call_type_check(&kind, v.clone(), place)?;
                    Some(luau::do_(vec![Stmt::Call(call)]))
                } else {
                    None
                };
                let member = Place { tested: true, ..place.clone() };
                if let Some((index, _)) = self.numeric_fallback(&kind, false)? {
                    let shape = alternatives[index].shape.clone();
                    let member_size = self.layout_of(&shape)?.size;
                    let body = out();
                    let size = match member_size {
                        Some(s) => num(s as f64),
                        None => self.emit_size(&shape, v.clone(), &body, &member)?,
                    };
                    push(&body, add_assign(total.e(), size));
                    let test = self.type_of_is(v.clone(), "number");
                    chain = Some(if_chain_on(test, take(&body), chain));
                }
                let order = self.evaluation(&kind)?.order.clone();
                for &i in order.iter().rev() {
                    let alternative = alternatives[i].clone();
                    let layout = self.layout_of(&alternative.shape)?;
                    let body = out();
                    let size = if *whole == Some(i) {
                        let vsize = self.varint_helpers().size;
                        luau::if_expr(Self::is_whole(v.clone()), luau::call(vsize.e(), vec![v.clone()]), num(8.0))
                    } else if let Some(s) = layout.size {
                        num(s as f64)
                    } else {
                        self.emit_size(&alternative.shape, v.clone(), &body, &member)?
                    };
                    if luau::literal_number(&size) != Some(0.0) {
                        push(&body, add_assign(total.e(), size));
                    }
                    if body.borrow().is_empty() && chain.is_none() {
                        continue;
                    }
                    let test = self.discriminate(&kind, i, v.clone())?;
                    chain = Some(if_chain_on(test, take(&body), chain));
                }
                if let Some(c) = chain {
                    push(out_list, c);
                }
                Ok(total.e())
            }
            other => panic!("Flamework: '{}' has a fixed size", other.name()),
        }
    }

    /// `allowsNil(shape)`.
    fn allows_nil(&mut self, shape: &Shape) -> GResult<bool> {
        let kind = self.describe(shape)?;
        if matches!(&*kind, Kind::Optional(_) | Kind::Nothing) {
            return Ok(true);
        }
        let Shape::Type(t) = shape else { return Ok(false) };
        if t.flags().intersects(TypeFlags::Any | TypeFlags::Unknown) {
            return Ok(true);
        }
        if t.flags().intersects(TypeFlags::TypeVariable) {
            let constraint = self.cx.checker().get_base_constraint_of_type_exported(*t);
            return match constraint {
                None => Ok(true),
                Some(c) if c == *t => Ok(true),
                Some(c) => self.allows_nil(&Shape::Type(c)),
            };
        }
        Ok(false)
    }

    /// `elementAt(list, index, out, hole)`: `local item = list[index]`, refused when nil with `hole`.
    fn element_at(&mut self, list: Expr, index: Id, out_list: &Out, hole: Option<(Place, String)>) -> Expr {
        let item = self.uid("item");
        push(out_list, luau::local(&item.e(), Some(luau::index(list, index.e()))));
        if let Some((place, container)) = hole {
            let mut parts = vec![Part::Text(format!("[Flamework] {container} has no value at "))];
            parts.extend(self.hole_where(&place));
            parts.push(Part::Text("[".into()));
            parts.push(Part::Expr(luau::bin(index.e(), B::Sub, num(1.0))));
            parts.push(Part::Text("]".into()));
            let message = luau::interp(parts);
            let raise = self.raise_with(message);
            push(out_list, if_then(luau::bin(item.e(), B::Eq, create::nil()), vec![raise]));
        }
        item.e()
    }

    /// `holeWhere(place)`.
    fn hole_where(&self, place: &Place) -> Vec<Part> {
        if let Some(w) = place.where_ {
            return match &place.path {
                Some(p) if !p.is_empty() => vec![Part::Expr(w.e()), Part::Text(p.clone())],
                _ => vec![Part::Expr(w.e())],
            };
        }
        if let Some(owner) = &place.owner {
            return vec![Part::Text(format!("{owner}{}", place.path.as_deref().unwrap_or("")))];
        }
        if place.args {
            return vec![Part::Text(match &place.site {
                Some(site) => format!("'{site}' "),
                None => String::new(),
            })];
        }
        match self.where_text(place) {
            Ok(text) => vec![Part::Text(text)],
            Err(e) => vec![Part::Expr(e)],
        }
    }

    // --- write -----------------------------------------------------------------------------------------------

    /// `emitWrite(shape, value, ctx)`.
    pub(crate) fn emit_write(&mut self, shape: &Shape, value: Expr, ctx: &Ctx) -> GResult<()> {
        if let Shape::Type(t) = shape
            && let Some(info) = self.hoist(*t)?
        {
            let mut args = vec![ctx.buf.e(), self.at(ctx, 0), value];
            if info.layout.blobs {
                args.push(ctx.blobs.unwrap().e());
            }
            if info.checks {
                args.push(self.passed_where(&ctx.place));
            }
            let call = self.call_hoisted(&info, Role::W, args);
            let variable = ctx.variable().unwrap();
            push(&ctx.out, luau::assign(variable.e(), call));
            let mut c = ctx.cursor.borrow_mut();
            c.base = Some(variable);
            c.offset = 0;
            return Ok(());
        }
        let kind = self.describe(shape)?;
        let typed = self.type_check_in(shape, false, &ctx.place)?.is_some();
        if typed && kind.tested_ahead() {
            self.emit_type_check(&kind, value.clone(), &ctx.out, &ctx.place)?;
        }
        match &*kind {
            Kind::Number { width, .. } => {
                let width = *width;
                self.write_number(&kind, value, ctx, &mut |gn, n, target| {
                    if width.is_wide() {
                        gn.write_wide(target, width, n);
                        return;
                    }
                    let at = gn.at(target, 0);
                    let write = gn.buffer_call(&format!("write{}", width.name()), vec![target.buf.e(), at, n]);
                    push(&target.out, Stmt::Call(write));
                    target.cursor.borrow_mut().offset += i64::from(width.size());
                })
            }
            Kind::Varint { .. } => self.write_number(&kind, value, ctx, &mut |gn, n, target| gn.write_varint(target, n)),
            Kind::Boolean => {
                let mut write = |gn: &mut Self, b: Expr, target: &Ctx| {
                    let at = gn.at(target, 0);
                    let flag = luau::if_expr(b, num(1.0), num(0.0));
                    let call = gn.buffer_call("writeu8", vec![target.buf.e(), at, flag]);
                    push(&target.out, Stmt::Call(call));
                    target.cursor.borrow_mut().offset += 1;
                };
                if typed {
                    if self.literal(&value).and_then(|l| l.type_name()).is_none() {
                        return self.write_typed(&kind, value, ctx, &mut write);
                    }
                    self.emit_type_check(&kind, value.clone(), &ctx.out, &ctx.place)?;
                }
                write(self, value, ctx);
                Ok(())
            }
            Kind::String { length: width, .. } => {
                let text = self.bind(&ctx.out, value, "text");
                let length = self.bind(&ctx.out, luau::len(text.clone()), "length");
                let checked = self.checked_width(&kind, ctx.unchecked);
                self.write_length(ctx, *width, length.clone(), "string", checked.as_deref());
                let at = self.at(ctx, 0);
                let write = self.buffer_call("writestring", vec![ctx.buf.e(), at, text]);
                push(&ctx.out, Stmt::Call(write));
                self.advance_by(ctx, length);
                Ok(())
            }
            Kind::Buffer { length: width, .. } => {
                let bytes = self.bind(&ctx.out, value, "bytes");
                let len = self.buffer_call("len", vec![bytes.clone()]);
                let length = self.bind(&ctx.out, len, "length");
                let checked = self.checked_width(&kind, ctx.unchecked);
                self.write_length(ctx, *width, length.clone(), "buffer", checked.as_deref());
                let at = self.at(ctx, 0);
                let copy = self.buffer_call("copy", vec![ctx.buf.e(), at, bytes]);
                push(&ctx.out, Stmt::Call(copy));
                self.advance_by(ctx, length);
                Ok(())
            }
            Kind::Constant(_) | Kind::Nothing => Ok(()),
            Kind::Literals(values) => {
                let (_, index) = self.literal_tables_for(&kind);
                let v = self.bind(&ctx.out, value, "v");
                let slot = self.bind(&ctx.out, luau::index(index, v.clone()), "index");
                let refuse = self.raise("value is not one of the literals its type allows");
                let body = if typed {
                    let call = self.call_type_check(&kind, v, &ctx.place)?;
                    vec![Stmt::Call(call), refuse]
                } else {
                    vec![refuse]
                };
                push(&ctx.out, if_then(luau::bin(slot.clone(), B::Eq, create::nil()), body));
                let width = if values.len() > 0xff { Width::U16 } else { Width::U8 };
                let at = self.at(ctx, 0);
                let write = self.buffer_call(&format!("write{}", width.name()), vec![ctx.buf.e(), at, slot]);
                push(&ctx.out, Stmt::Call(write));
                ctx.cursor.borrow_mut().offset += i64::from(width.size());
                Ok(())
            }
            Kind::Blob { .. } => {
                let blob = self.bind(&ctx.out, value, "blob");
                let blobs = ctx.blobs.unwrap();
                let at = self.at(ctx, 0);
                let insert = luau::call(luau::prop(create::id("table"), "insert"), vec![blobs.e(), blob.clone()]);
                let mut present = vec![
                    Stmt::Call(insert),
                    Stmt::Call(self.buffer_call("writeu32", vec![ctx.buf.e(), at.clone(), luau::len(blobs.e())])),
                ];
                if typed {
                    present.insert(0, self.type_check_statement(&kind, blob.clone(), &ctx.place)?);
                }
                let absent = vec![Stmt::Call(self.buffer_call("writeu32", vec![ctx.buf.e(), at, num(0.0)]))];
                push(&ctx.out, luau::if_chain(vec![(luau::bin(blob, B::Ne, create::nil()), present)], absent));
                ctx.cursor.borrow_mut().offset += BLOB_SIZE;
                Ok(())
            }
            Kind::Datatype(name) => {
                let (_, fields) = datatype(name).unwrap();
                let hint = name.to_lowercase();
                let value = self.bind(&ctx.out, value, &hint);
                for (width, names) in fields {
                    let at = self.at(ctx, 0);
                    let path = names.iter().fold(value.clone(), |current, n| luau::prop(current, n));
                    let write = self.buffer_call(&format!("write{}", width.name()), vec![ctx.buf.e(), at, path]);
                    push(&ctx.out, Stmt::Call(write));
                    ctx.cursor.borrow_mut().offset += i64::from(width.size());
                }
                Ok(())
            }
            Kind::CFrame => {
                let components: Vec<Id> = (0..12).map(|i| self.uid(&format!("c{i}"))).collect();
                let names: Vec<Expr> = components.iter().map(|c| c.e()).collect();
                push(&ctx.out, luau::locals(&names, vec![luau::method(value, "GetComponents", vec![])]));
                for c in components {
                    let at = self.at(ctx, 0);
                    let write = self.buffer_call("writef32", vec![ctx.buf.e(), at, c.e()]);
                    push(&ctx.out, Stmt::Call(write));
                    ctx.cursor.borrow_mut().offset += 4;
                }
                Ok(())
            }
            Kind::Enum(_) => {
                let at = self.at(ctx, 0);
                let write = self.buffer_call("writeu16", vec![ctx.buf.e(), at, luau::prop(value, "Value")]);
                push(&ctx.out, Stmt::Call(write));
                ctx.cursor.borrow_mut().offset += 2;
                Ok(())
            }
            Kind::Optional(inner) => {
                if matches!(value, Expr::Nil) {
                    let at = self.at(ctx, 0);
                    let write = self.buffer_call("writeu8", vec![ctx.buf.e(), at, num(0.0)]);
                    push(&ctx.out, Stmt::Call(write));
                    ctx.cursor.borrow_mut().offset += 1;
                    return Ok(());
                }
                let v = self.bind(&ctx.out, value, "v");
                let at = self.at(ctx, 0);
                let flag = luau::if_expr(luau::bin(v.clone(), B::Ne, create::nil()), num(1.0), num(0.0));
                let write = self.buffer_call("writeu8", vec![ctx.buf.e(), at, flag]);
                push(&ctx.out, Stmt::Call(write));
                ctx.cursor.borrow_mut().offset += 1;
                let inner = inner.clone();
                let v2 = v.clone();
                let body = self.branch(ctx, &mut |gn, child| gn.emit_write(&inner, v2.clone(), child))?;
                push(&ctx.out, if_then(luau::bin(v, B::Ne, create::nil()), body));
                Ok(())
            }
            Kind::Array(element) => {
                let array = self.bind(&ctx.out, value, "array");
                self.write_varint(ctx, luau::len(array.clone()));
                let hole = if self.hole_in_write(element)? { Some((ctx.place.clone(), "the array".to_owned())) } else { None };
                let index = self.uid("i");
                let element = element.clone();
                let array_in = array.clone();
                let body = self.branch(ctx, &mut |gn, child| {
                    let item = gn.element_at(array_in.clone(), index, &child.out, hole.clone());
                    gn.emit_write(&element, item, &child.within("[]", false))
                })?;
                let range = self.for_range(index, num(1.0), luau::len(array), body);
                push(&ctx.out, range);
                Ok(())
            }
            Kind::Set(element) => {
                let set = self.bind(&ctx.out, value, "set");
                let element = element.clone();
                self.write_counted(ctx, set, None, &mut |gn, item, child| gn.emit_write(&element, item.e(), &child.within("[]", false)))
            }
            Kind::Map(key_shape, value_shape) => {
                let map = self.bind(&ctx.out, value, "map");
                let key = self.uid("key");
                let entry = self.uid("entry");
                let (key_shape, value_shape) = (key_shape.clone(), value_shape.clone());
                self.write_counted(ctx, map, Some(vec![key, entry]), &mut |gn, _, child| {
                    gn.emit_write(&key_shape, key.e(), &child.within("<key>", false))?;
                    gn.emit_write(&value_shape, entry.e(), &child.within("<value>", false))
                })
            }
            Kind::List { elements, rest, after } => {
                let list = self.bind(&ctx.out, value, "list");
                for (i, element) in elements.iter().enumerate() {
                    let item = luau::index(list.clone(), num((i + 1) as f64));
                    self.emit_scoped_write(element, item, &ctx.within(&format!("[{i}]"), false))?;
                }
                if let Some(rest) = rest {
                    let after: Vec<Shape> = after.clone().unwrap_or_default();
                    let count = self.rest_count(&ctx.out, list.clone(), elements.len() + after.len());
                    self.write_varint(ctx, count.e());
                    let index = self.uid("i");
                    let hole = if self.hole_in_write(rest)? { Some((ctx.place.clone(), tuple_name(&ctx.place))) } else { None };
                    let rest = rest.clone();
                    let list_in = list.clone();
                    let body = self.branch(ctx, &mut |gn, child| {
                        let element = match &hole {
                            Some(h) => gn.element_at(list_in.clone(), index, &child.out, Some(h.clone())),
                            None => luau::index(list_in.clone(), index.e()),
                        };
                        gn.emit_write(&rest, element, &child.within("[]", false))
                    })?;
                    let range =
                        self.for_range(index, num((elements.len() + 1) as f64), luau::add(count.e(), elements.len() as f64), body);
                    push(&ctx.out, range);
                    for (position, shape) in after.iter().enumerate() {
                        let at = luau::add(count.e(), (elements.len() + position + 1) as f64);
                        self.emit_scoped_write(shape, luau::index(list.clone(), at), &ctx.within("[]", false))?;
                    }
                }
                Ok(())
            }
            Kind::Object { fields } => {
                let object = self.bind(&ctx.out, value, "object");
                for field in fields {
                    let place = ctx.within(&key_segment(&field.key), false);
                    let compared =
                        if ctx.place.compared.as_deref() == Some(field.name.as_str()) { Ctx { place: Place { tested: true, ..place.place.clone() }, ..place } } else { place };
                    self.emit_scoped_write(&field.shape, key_access(object.clone(), &field.key), &compared)?;
                }
                Ok(())
            }
            Kind::Union { alternatives, whole, .. } => self.write_union(&kind, alternatives.clone(), *whole, value, ctx, typed),
        }
    }

    fn write_union(
        &mut self,
        kind: &Rc<Kind>,
        alternatives: Vec<super::Alternative>,
        whole: Option<usize>,
        value: Expr,
        ctx: &Ctx,
        typed: bool,
    ) -> GResult<()> {
        let v = self.bind(&ctx.out, value, "v");
        let layout = self.layout_of(&Shape::Kind(kind.clone()))?;
        let is_fixed_layout = ctx.variable().is_none();
        let start = ctx.cursor.borrow().offset;
        if typed {
            self.table_precheck(kind, v.clone(), &ctx.out, &ctx.place)?;
        }
        let refuse = self.raise("value matches none of the union's members");
        let mut chain = if typed {
            let call = self.call_type_check(kind, v.clone(), &ctx.place)?;
            luau::do_(vec![Stmt::Call(call), refuse])
        } else {
            luau::do_(vec![refuse])
        };
        let checks_types = self.g.checks.types;
        let write_member = |gn: &mut Self, child: &Ctx, i: usize| -> GResult<()> {
            let at = gn.at(child, 0);
            let tag = gn.buffer_call("writeu8", vec![child.buf.e(), at, num(i as f64)]);
            push(&child.out, Stmt::Call(tag));
            child.cursor.borrow_mut().offset += 1;
            let shape = alternatives[i].shape.clone();
            let ranged = number_range(&*gn.describe(&shape)?).is_some();
            let compared = if checks_types { gn.discriminant_field(kind, i)? } else { None };
            let mut member = Ctx { place: Place { tested: true, compared, ..child.place.clone() }, ..child.clone() };
            if ranged {
                member.unchecked = true;
            }
            gn.emit_write(&shape, v.clone(), &member)
        };

        if let Some((index, widths)) = self.numeric_fallback(kind, ctx.unchecked)? {
            let is_number = self.type_of_is(v.clone(), "number");
            let check = self.call_check(&widths, v.clone(), ctx, None);
            let test = luau::bin(is_number, B::And, check);
            let body = self.branch(ctx, &mut |gn, child| write_member(gn, child, index))?;
            chain = if_else(test, body, chain);
        }

        let order = self.evaluation(kind)?.order.clone();
        let count = alternatives.len();
        for &i in order.iter().rev() {
            let body = self.branch(ctx, &mut |gn, child| {
                if whole != Some(i) {
                    return write_member(gn, child, i);
                }
                let n = v.clone();
                let whole_body = gn.branch(child, &mut |gn, inner| {
                    let at = gn.at(inner, 0);
                    let tag = gn.buffer_call("writeu8", vec![inner.buf.e(), at, num(count as f64)]);
                    push(&inner.out, Stmt::Call(tag));
                    inner.cursor.borrow_mut().offset += 1;
                    gn.write_varint(inner, n.clone());
                    Ok(())
                })?;
                let other = gn.branch(child, &mut |gn, inner| write_member(gn, inner, i))?;
                push(&child.out, luau::if_chain(vec![(Self::is_whole(v.clone()), whole_body)], other));
                Ok(())
            })?;
            let test = self.discriminate(kind, i, v.clone())?;
            chain = if_else(test, body, chain);
        }
        push(&ctx.out, chain);
        if is_fixed_layout && let Some(size) = layout.size {
            ctx.cursor.borrow_mut().offset = start + size as i64;
        }
        Ok(())
    }

    /// `emitScopedWrite(shape, value, place)`.
    fn emit_scoped_write(&mut self, shape: &Shape, value: Expr, ctx: &Ctx) -> GResult<()> {
        if !self.g.checks.types && !self.g.compact {
            return self.emit_write(shape, value, ctx);
        }
        let block = ctx.with_out(out());
        self.emit_write(shape, value, &block)?;
        let statements = take(&block.out);
        self.push_scoped(&ctx.out, statements);
        Ok(())
    }

    /// `holeInWrite(element)`.
    fn hole_in_write(&mut self, element: &Shape) -> GResult<bool> {
        Ok(!self.allows_nil(element)? && self.layout_of(element)?.size.is_some())
    }

    /// `restCount(out, list, fixed)`: `local count = #list`, less the fixed elements.
    fn rest_count(&mut self, out_list: &Out, list: Expr, fixed: usize) -> Id {
        let count = self.uid("count");
        let size = luau::len(list);
        let value = if fixed == 0 {
            size
        } else {
            let length = luau::bin(size, B::Sub, num(fixed as f64));
            self.math_call("max", vec![length, num(0.0)])
        };
        push(out_list, luau::local(&count.e(), Some(value)));
        count
    }

    /// `writeLength(ctx, width, length, what, checked)`.
    fn write_length(&mut self, ctx: &Ctx, width: LengthWidth, length: Expr, what: &str, checked: Option<&str>) {
        let Some(fixed) = width.width() else {
            self.write_varint(ctx, length);
            return;
        };
        if width != LengthWidth::U32 {
            let max = if width == LengthWidth::U8 { 255.0 } else { 65535.0 };
            let refuse = self.raise(&format!("{what} is longer than its {} length prefix allows", width.name()));
            let body = match checked {
                Some(c) => {
                    let call = self.call_check(c, length.clone(), ctx, Some(" bytes"));
                    vec![Stmt::Call(call), refuse]
                }
                None => vec![refuse],
            };
            push(&ctx.out, if_then(luau::bin(length.clone(), B::Gt, num(max)), body));
        }
        let at = self.at(ctx, 0);
        let write = self.buffer_call(&format!("write{}", fixed.name()), vec![ctx.buf.e(), at, length]);
        push(&ctx.out, Stmt::Call(write));
        ctx.cursor.borrow_mut().offset += i64::from(fixed.size());
    }

    /// `writeCounted(ctx, collection, body, binding)`: the collection counted (`local n = 0; for _ in c do n += 1 end`),
    /// the count written, then `for <binding> in c do <body> end`.
    fn write_counted(
        &mut self,
        ctx: &Ctx,
        collection: Expr,
        binding: Option<Vec<Id>>,
        body: &mut dyn FnMut(&mut Self, Id, &Ctx) -> GResult<()>,
    ) -> GResult<()> {
        let count = self.uid("n");
        push(&ctx.out, luau::local(&count.e(), Some(num(0.0))));
        let counting = vec![add_assign(count.e(), num(1.0))];
        push(&ctx.out, luau::for_in(&[create::id("_")], collection.clone(), counting));
        self.write_varint(ctx, count.e());
        let item = self.uid("item");
        let statements = self.branch(ctx, &mut |gn, child| body(gn, item, child))?;
        let ids: Vec<Expr> = binding.unwrap_or_else(|| vec![item]).into_iter().map(Id::e).collect();
        push(&ctx.out, luau::for_in(&ids, collection, statements));
        Ok(())
    }

    /// `writeNumber(kind, value, ctx, write)`.
    fn write_number(
        &mut self,
        kind: &Rc<Kind>,
        value: Expr,
        ctx: &Ctx,
        write: &mut dyn FnMut(&mut Self, Expr, &Ctx),
    ) -> GResult<()> {
        let width = self.checked_width(kind, ctx.unchecked);
        let range = number_range(kind);
        let typed = self.type_check_in(&Shape::Kind(kind.clone()), false, &ctx.place)?.is_some();
        let ranged = width.is_some() && range.is_some();
        if !ranged && !typed {
            write(self, value, ctx);
            return Ok(());
        }
        if let Some(known) = self.literal_number(&value) {
            if ranged && !fits_statically(known, range.unwrap()) {
                let call = self.call_check(width.as_deref().unwrap(), value.clone(), ctx, None);
                push(&ctx.out, Stmt::Call(call));
            }
            write(self, value, ctx);
            return Ok(());
        }
        let block = ctx.with_out(out());
        let n = if self.is_name(&value) { value } else { self.bind(&block.out, value, "n") };
        let target = if block.out.borrow().is_empty() { ctx.clone() } else { block.clone() };
        if typed {
            let statement = self.type_check_statement(kind, n.clone(), &ctx.place)?;
            push(&target.out, statement);
        }
        if ranged {
            let fails = self.fails_range(n.clone(), range.unwrap());
            let call = self.call_check(width.as_deref().unwrap(), n.clone(), ctx, None);
            push(&target.out, if_then(fails, vec![Stmt::Call(call)]));
        }
        write(self, n, &target);
        if Rc::ptr_eq(&target.out, &block.out) {
            let statements = take(&block.out);
            push(&ctx.out, luau::do_(statements));
        }
        Ok(())
    }

    /// `writeTyped(kind, value, ctx, write)`.
    fn write_typed(&mut self, kind: &Rc<Kind>, value: Expr, ctx: &Ctx, write: &mut dyn FnMut(&mut Self, Expr, &Ctx)) -> GResult<()> {
        let block = ctx.with_out(out());
        let v = if self.is_name(&value) { value } else { self.bind(&block.out, value, "v") };
        let target = if block.out.borrow().is_empty() { ctx.clone() } else { block.clone() };
        let statement = self.type_check_statement(kind, v.clone(), &ctx.place)?;
        push(&target.out, statement);
        write(self, v, &target);
        if Rc::ptr_eq(&target.out, &block.out) {
            let statements = take(&block.out);
            push(&ctx.out, luau::do_(statements));
        }
        Ok(())
    }

    /// `writeVarint(ctx, n)`: `o = vwrite(buf, o, n)`.
    fn write_varint(&mut self, ctx: &Ctx, n: Expr) {
        let variable = ctx.variable().expect("Flamework: a varint inside a fixed layout");
        let vwrite = self.varint_helpers().write;
        let at = self.at(ctx, 0);
        push(&ctx.out, luau::assign(variable.e(), luau::call(vwrite.e(), vec![ctx.buf.e(), at, n])));
        let mut c = ctx.cursor.borrow_mut();
        c.base = Some(variable);
        c.offset = 0;
    }

    /// `readVarint(ctx, hint)`: `local n, o2 = vread(buf, o)`.
    fn read_varint(&mut self, ctx: &Ctx, hint: &str) -> Id {
        assert!(ctx.variable().is_some(), "Flamework: a varint inside a fixed layout");
        let n = self.uid(hint);
        let next = self.uid("o");
        let vread = self.varint_helpers().read;
        let at = self.at(ctx, 0);
        push(&ctx.out, luau::locals(&[n.e(), next.e()], vec![luau::call(vread.e(), vec![ctx.buf.e(), at])]));
        let mut c = ctx.cursor.borrow_mut();
        c.base = Some(next);
        c.offset = 0;
        n
    }

    /// `writeWide(ctx, width, value)`.
    fn write_wide(&mut self, ctx: &Ctx, width: Width, value: Expr) {
        let high = if width == Width::U64 { "writeu32" } else { "writei32" };
        if let Some(known) = self.literal_number(&value)
            && known.is_finite()
        {
            let upper = (known / HALF).floor();
            let at = self.at(ctx, 0);
            let a = self.buffer_call("writeu32", vec![ctx.buf.e(), at, num(known - upper * HALF)]);
            push(&ctx.out, Stmt::Call(a));
            let at4 = self.at(ctx, 4);
            let b = self.buffer_call(high, vec![ctx.buf.e(), at4, num(upper)]);
            push(&ctx.out, Stmt::Call(b));
            ctx.cursor.borrow_mut().offset += i64::from(width.size());
            return;
        }
        let block = ctx.with_out(out());
        let n = if self.is_name(&value) { value } else { self.bind(&block.out, value, "n") };
        let target = if block.out.borrow().is_empty() { ctx.clone() } else { block.clone() };
        let at = self.at(&target, 0);
        let low = self.buffer_call("writeu32", vec![target.buf.e(), at, luau::bin(n.clone(), B::Mod, num(HALF))]);
        push(&target.out, Stmt::Call(low));
        let upper = self.math_call("floor", vec![luau::bin(n, B::Div, num(HALF))]);
        let at4 = self.at(&target, 4);
        let high_write = self.buffer_call(high, vec![target.buf.e(), at4, upper]);
        push(&target.out, Stmt::Call(high_write));
        target.cursor.borrow_mut().offset += i64::from(width.size());
        if Rc::ptr_eq(&target.out, &block.out) {
            let statements = take(&block.out);
            push(&ctx.out, luau::do_(statements));
        }
    }

    /// `readWide(ctx, width)`.
    fn read_wide(&mut self, ctx: &Ctx, width: Width) -> Expr {
        let at4 = self.at(ctx, 4);
        let high = self.buffer_call(if width == Width::U64 { "readu32" } else { "readi32" }, vec![ctx.buf.e(), at4]);
        let at = self.at(ctx, 0);
        let low = self.buffer_call("readu32", vec![ctx.buf.e(), at]);
        ctx.cursor.borrow_mut().offset += i64::from(width.size());
        luau::bin(luau::bin(high, B::Mul, num(HALF)), B::Add, low)
    }

    /// `sizeWithLength(out, width, length)`.
    fn size_with_length(&mut self, out_list: &Out, width: LengthWidth, length: Expr) -> Expr {
        if let Some(fixed) = width.width() {
            return luau::add(length, f64::from(fixed.size()));
        }
        let bound = self.bind(out_list, length, "length");
        let vsize = self.varint_helpers().size;
        luau::plus(luau::call(vsize.e(), vec![bound.clone()]), bound)
    }

    /// `countedInPass(out, collection, binding, element)`.
    fn counted_in_pass(
        &mut self,
        out_list: &Out,
        collection: Expr,
        binding: &[Id],
        element: &mut dyn FnMut(&mut Self, &Out) -> GResult<Expr>,
    ) -> GResult<Expr> {
        let total = self.uid("size");
        let count = self.uid("n");
        push(out_list, luau::local(&total.e(), Some(num(0.0))));
        push(out_list, luau::local(&count.e(), Some(num(0.0))));
        let body = out();
        push(&body, add_assign(count.e(), num(1.0)));
        let size = element(self, &body)?;
        push(&body, add_assign(total.e(), size));
        let ids: Vec<Expr> = binding.iter().map(|id| id.e()).collect();
        push(out_list, luau::for_in(&ids, collection, take(&body)));
        let vsize = self.varint_helpers().size;
        push(out_list, add_assign(total.e(), luau::call(vsize.e(), vec![count.e()])));
        Ok(total.e())
    }

    // --- read ------------------------------------------------------------------------------------------------

    /// `emitRead(shape, ctx)`.
    pub(crate) fn emit_read(&mut self, shape: &Shape, ctx: &Ctx) -> GResult<Expr> {
        if let Shape::Type(t) = shape
            && let Some(info) = self.hoist(*t)?
        {
            let mut args = vec![ctx.buf.e(), self.at(ctx, 0)];
            if info.layout.blobs {
                args.push(ctx.blobs.unwrap().e());
            }
            let value = self.uid("value");
            let next = self.uid("o");
            let call = self.call_hoisted(&info, Role::R, args);
            push(&ctx.out, luau::locals(&[value.e(), next.e()], vec![call]));
            let mut c = ctx.cursor.borrow_mut();
            c.base = Some(next);
            c.offset = 0;
            return Ok(value.e());
        }
        let kind = self.describe(shape)?;
        match &*kind {
            Kind::Number { width, .. } => {
                if width.is_wide() {
                    return Ok(self.read_wide(ctx, *width));
                }
                let at = self.at(ctx, 0);
                let read = self.buffer_call(&format!("read{}", width.name()), vec![ctx.buf.e(), at]);
                ctx.cursor.borrow_mut().offset += i64::from(width.size());
                Ok(read)
            }
            Kind::Varint { .. } => Ok(self.read_varint(ctx, "n").e()),
            Kind::Boolean => {
                let at = self.at(ctx, 0);
                let read = self.buffer_call("readu8", vec![ctx.buf.e(), at]);
                ctx.cursor.borrow_mut().offset += 1;
                Ok(luau::bin(read, B::Ne, num(0.0)))
            }
            Kind::String { length: width, .. } => {
                let length = self.read_length(ctx, *width);
                let at = self.at(ctx, 0);
                let read = self.buffer_call("readstring", vec![ctx.buf.e(), at, length.clone()]);
                let text = self.bind(&ctx.out, read, "text");
                self.advance_by(ctx, length);
                Ok(text)
            }
            Kind::Buffer { length: width, .. } => {
                let length = self.read_length(ctx, *width);
                let len = self.buffer_call("len", vec![ctx.buf.e()]);
                let remaining = luau::bin(len, B::Sub, self.at(ctx, 0));
                let raise = self.raise(MALFORMED);
                push(&ctx.out, if_then(luau::bin(length.clone(), B::Gt, remaining), vec![raise]));
                let create = self.buffer_call("create", vec![length.clone()]);
                let bytes = self.bind(&ctx.out, create, "bytes");
                let at = self.at(ctx, 0);
                let copy = self.buffer_call("copy", vec![bytes.clone(), num(0.0), ctx.buf.e(), at, length.clone()]);
                push(&ctx.out, Stmt::Call(copy));
                self.advance_by(ctx, length);
                Ok(bytes)
            }
            Kind::Constant(value) => {
                self.check_globals_in(value);
                Ok(value.to_luau())
            }
            Kind::Nothing => Ok(create::nil()),
            Kind::Literals(values) => {
                let (list, _) = self.literal_tables_for(&kind);
                let width = if values.len() > 0xff { Width::U16 } else { Width::U8 };
                let at = self.at(ctx, 0);
                let index = self.buffer_call(&format!("read{}", width.name()), vec![ctx.buf.e(), at]);
                ctx.cursor.borrow_mut().offset += i64::from(width.size());
                let value = self.bind(&ctx.out, luau::index(list, luau::add(index, 1.0)), "literal");
                let raise = self.raise(MALFORMED);
                push(&ctx.out, if_then(luau::bin(value.clone(), B::Eq, create::nil()), vec![raise]));
                Ok(value)
            }
            Kind::Blob { .. } => {
                let at = self.at(ctx, 0);
                let index = self.buffer_call("readu32", vec![ctx.buf.e(), at]);
                ctx.cursor.borrow_mut().offset += BLOB_SIZE;
                Ok(luau::index(ctx.blobs.unwrap().e(), index))
            }
            Kind::Datatype(name) => {
                let (_, fields) = datatype(name).unwrap();
                let mut args = Vec::new();
                for (width, _) in fields {
                    let at = self.at(ctx, 0);
                    args.push(self.buffer_call(&format!("read{}", width.name()), vec![ctx.buf.e(), at]));
                    ctx.cursor.borrow_mut().offset += i64::from(width.size());
                }
                Ok(self.construct(name, args))
            }
            Kind::CFrame => {
                let mut args = Vec::new();
                for _ in 0..12 {
                    let at = self.at(ctx, 0);
                    args.push(self.buffer_call("readf32", vec![ctx.buf.e(), at]));
                    ctx.cursor.borrow_mut().offset += 4;
                }
                Ok(self.construct("CFrame", args))
            }
            Kind::Enum(name) => {
                let at = self.at(ctx, 0);
                let index = self.buffer_call("readu16", vec![ctx.buf.e(), at]);
                ctx.cursor.borrow_mut().offset += 2;
                let table = self.enum_table_for(name);
                Ok(luau::index(table, index))
            }
            Kind::Optional(inner) => {
                let at = self.at(ctx, 0);
                let read = self.buffer_call("readu8", vec![ctx.buf.e(), at]);
                let present = self.bind(&ctx.out, luau::bin(read, B::Ne, num(0.0)), "present");
                ctx.cursor.borrow_mut().offset += 1;
                let value = self.uid("value");
                push(&ctx.out, luau::local(&value.e(), None));
                let inner = inner.clone();
                let body = self.branch(ctx, &mut |gn, child| {
                    let read = gn.emit_read(&inner, child)?;
                    push(&child.out, luau::assign(value.e(), read));
                    Ok(())
                })?;
                push(&ctx.out, if_then(present, body));
                Ok(value.e())
            }
            Kind::Array(element) => {
                let min = self.layout_of(element)?.min;
                let count = self.read_count(ctx, min);
                let created = self.construct("Array", vec![count.clone()]);
                let array = self.bind(&ctx.out, created, "array");
                let index = self.uid("i");
                let element = element.clone();
                let array_in = array.clone();
                let body = self.branch(ctx, &mut |gn, child| {
                    let read = gn.emit_read(&element, child)?;
                    push(&child.out, luau::assign(luau::index(array_in.clone(), index.e()), read));
                    Ok(())
                })?;
                let range = self.for_range(index, num(1.0), count, body);
                push(&ctx.out, range);
                Ok(array)
            }
            Kind::Set(element) => {
                let min = self.layout_of(element)?.min;
                let count = self.read_count(ctx, min);
                let created = self.construct("Set", vec![]);
                let set = self.bind(&ctx.out, created, "set");
                let loop_id = self.uid("_");
                let element = element.clone();
                let set_in = set.clone();
                let body = self.branch(ctx, &mut |gn, child| {
                    let read = gn.emit_read(&element, child)?;
                    push(&child.out, luau::assign(luau::index(set_in.clone(), read), create::bool(true)));
                    Ok(())
                })?;
                let range = self.for_range(loop_id, num(1.0), count, body);
                push(&ctx.out, range);
                Ok(set)
            }
            Kind::Map(key_shape, value_shape) => {
                let min = self.layout_of(key_shape)?.min + self.layout_of(value_shape)?.min;
                let count = self.read_count(ctx, min);
                let map = self.bind(&ctx.out, create::map(Vec::<(Expr, Expr)>::new()), "map");
                let loop_id = self.uid("_");
                let (key_shape, value_shape) = (key_shape.clone(), value_shape.clone());
                let map_in = map.clone();
                let body = self.branch(ctx, &mut |gn, child| {
                    let key = gn.read_into(&key_shape, child, "key")?;
                    let value = gn.read_into(&value_shape, child, "entry")?;
                    push(&child.out, luau::assign(luau::index(map_in.clone(), key), value));
                    Ok(())
                })?;
                let range = self.for_range(loop_id, num(1.0), count, body);
                push(&ctx.out, range);
                Ok(map)
            }
            Kind::List { elements, rest, after } => self.read_list(elements, rest.as_ref(), after.as_deref(), ctx),
            Kind::Object { fields } => {
                if self.g.compact {
                    let object = self.bind(&ctx.out, create::map(Vec::<(Expr, Expr)>::new()), "object");
                    for field in fields {
                        let target = key_access(object.clone(), &field.key);
                        self.read_scoped(&field.shape, ctx, target)?;
                    }
                    return Ok(object);
                }
                let mut entries = Vec::new();
                for field in fields {
                    let read = self.read_into(&field.shape, ctx, &field.name)?;
                    entries.push((key_name(&field.key), read));
                }
                Ok(create::map(entries))
            }
            Kind::Union { alternatives, whole, .. } => {
                let layout = self.layout_of(&Shape::Kind(kind.clone()))?;
                let is_fixed_layout = ctx.variable().is_none();
                let at = self.at(ctx, 0);
                let read = self.buffer_call("readu8", vec![ctx.buf.e(), at]);
                let tag = self.bind(&ctx.out, read, "tag");
                ctx.cursor.borrow_mut().offset += 1;
                let start = ctx.cursor.borrow().offset;
                let value = self.uid("value");
                push(&ctx.out, luau::local(&value.e(), None));
                let raise = self.raise(MALFORMED);
                let mut chain = luau::do_(vec![raise]);
                if whole.is_some() {
                    let body = self.branch(ctx, &mut |gn, child| {
                        let n = gn.read_varint(child, "n");
                        push(&child.out, luau::assign(value.e(), n.e()));
                        Ok(())
                    })?;
                    chain = if_else(luau::bin(tag.clone(), B::Eq, num(alternatives.len() as f64)), body, chain);
                }
                for i in (0..alternatives.len()).rev() {
                    let shape = alternatives[i].shape.clone();
                    let body = self.branch(ctx, &mut |gn, child| {
                        let read = gn.emit_read(&shape, child)?;
                        push(&child.out, luau::assign(value.e(), read));
                        Ok(())
                    })?;
                    chain = if_else(luau::bin(tag.clone(), B::Eq, num(i as f64)), body, chain);
                }
                push(&ctx.out, chain);
                if is_fixed_layout && let Some(size) = layout.size {
                    ctx.cursor.borrow_mut().offset = start + size as i64 - 1;
                }
                Ok(value.e())
            }
        }
    }

    fn read_list(&mut self, elements: &[Shape], rest: Option<&Shape>, after: Option<&[Shape]>, ctx: &Ctx) -> GResult<Expr> {
        let list;
        let count;
        if self.g.compact {
            let built = self.bind(&ctx.out, create::array(vec![]), "list");
            for (i, element) in elements.iter().enumerate() {
                self.read_scoped(element, ctx, luau::index(built.clone(), num((i + 1) as f64)))?;
            }
            let Some(rest) = rest else { return Ok(built) };
            list = built;
            let min = self.layout_of(rest)?.min;
            count = self.read_count(ctx, min);
        } else {
            let mut values = Vec::new();
            for element in elements {
                values.push(self.read_into(element, ctx, "arg")?);
            }
            let Some(rest) = rest else { return Ok(create::array(values)) };
            let min = self.layout_of(rest)?.min;
            count = self.read_count(ctx, min);
            list = self.bind(&ctx.out, create::array(values), "list");
        }
        let rest = rest.unwrap().clone();
        let index = self.uid("i");
        let fixed = elements.len();
        let list_in = list.clone();
        let body = self.branch(ctx, &mut |gn, child| {
            let read = gn.emit_read(&rest, child)?;
            push(&child.out, luau::assign(luau::index(list_in.clone(), luau::add(index.e(), fixed as f64)), read));
            Ok(())
        })?;
        let range = self.for_range(index, num(1.0), count.clone(), body);
        push(&ctx.out, range);
        for (position, shape) in after.unwrap_or(&[]).iter().enumerate() {
            let at = luau::index(list.clone(), luau::add(count.clone(), (fixed + position + 1) as f64));
            if self.g.compact {
                self.read_scoped(shape, ctx, at)?;
                continue;
            }
            let element = self.read_into(shape, ctx, "arg")?;
            push(&ctx.out, luau::assign(at, element));
        }
        Ok(list)
    }

    /// `readScoped(shape, ctx, target)`.
    fn read_scoped(&mut self, shape: &Shape, ctx: &Ctx, target: Expr) -> GResult<()> {
        let inner = ctx.with_out(out());
        let read = self.emit_read(shape, &inner)?;
        push(&inner.out, luau::assign(target, read));
        if inner.variable().is_some() {
            self.sync(&inner);
        }
        let statements = take(&inner.out);
        self.push_scoped(&ctx.out, statements);
        Ok(())
    }

    /// `readInto(shape, ctx, hint)`.
    fn read_into(&mut self, shape: &Shape, ctx: &Ctx, hint: &str) -> GResult<Expr> {
        let value = self.emit_read(shape, ctx)?;
        if ctx.variable().is_none() || is_name(&value) || is_literal(&value) {
            return Ok(value);
        }
        let name = local_name(self.cx, hint);
        Ok(self.bind(&ctx.out, value, &name))
    }

    /// `readLength(ctx, width)`.
    fn read_length(&mut self, ctx: &Ctx, width: LengthWidth) -> Expr {
        let Some(fixed) = width.width() else { return self.read_varint(ctx, "length").e() };
        let at = self.at(ctx, 0);
        let read = self.buffer_call(&format!("read{}", fixed.name()), vec![ctx.buf.e(), at]);
        let length = self.bind(&ctx.out, read, "length");
        ctx.cursor.borrow_mut().offset += i64::from(fixed.size());
        length
    }

    /// `readCount(ctx, minimumElementSize)`.
    fn read_count(&mut self, ctx: &Ctx, minimum: u64) -> Expr {
        let count = self.read_varint(ctx, "count");
        self.sync(ctx);
        if minimum == 0 {
            let tally = self.zero_tally();
            push(&ctx.out, add_assign(tally.e(), count.e()));
            let raise = self.raise(MALFORMED);
            push(&ctx.out, if_then(luau::bin(tally.e(), B::Gt, num(ZERO_SIZE_COUNT_MAX)), vec![raise]));
            return count.e();
        }
        let len = self.buffer_call("len", vec![ctx.buf.e()]);
        let remaining = luau::bin(len, B::Sub, ctx.variable().unwrap().e());
        let needed = if minimum > 1 { luau::bin(count.e(), B::Mul, num(minimum as f64)) } else { count.e() };
        let raise = self.raise(MALFORMED);
        push(&ctx.out, if_then(luau::bin(needed, B::Gt, remaining), vec![raise]));
        count.e()
    }
}

/// A versioned serializer's header.
pub(crate) struct Header {
    pub version: u32,
    pub hash: u32,
    pub name: String,
}

/// `countedSize(prefix, count, size)`.
fn counted_size(prefix: Expr, count: Expr, size: u64) -> Expr {
    if size == 0 {
        return prefix;
    }
    luau::plus(prefix, luau::bin(count, B::Mul, num(size as f64)))
}

/// `tupleName(place)`.
fn tuple_name(place: &Place) -> String {
    if place.args { "the argument list".into() } else { "the tuple".into() }
}
