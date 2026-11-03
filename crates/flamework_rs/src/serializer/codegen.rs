//! The code: the top level (`buildSerializer`, `buildDecoder`, `encodeElements`), the size pass, the writes and the
//! reads, with the cursor that folds constant offsets and the compact form (`fitted`).

use std::cell::RefCell;
use std::rc::Rc;

use tsrs_checker::TypeFlags;

use super::checks::{fits_statically, literal_type, number_range};
use super::classify::datatype;
use super::hoist::{MALFORMED, Role};
use super::ir::{self, Bind, Body, E, Id, Key, Op, S, Ty};
use super::limits::{self, Choice, Measuring, holds_locals};
use super::typeinfo::{key_access, key_name, key_segment, local_name};
use super::{EncodingSite, GResult, Gen, Kind, LengthWidth, Layout, Shape, Width};

const HEADER_SIZE: u64 = 5;
const BLOB_SIZE: i64 = 4;
const ZERO_SIZE_COUNT_MAX: f64 = 65535.0;
const HALF: f64 = 4294967296.0;

/// A statement list several builders append to (a TS array shared by reference).
pub(crate) type Out = Rc<RefCell<Vec<S>>>;

pub(crate) fn out() -> Out {
    Rc::new(RefCell::new(Vec::new()))
}

pub(crate) fn push(out: &Out, s: S) {
    out.borrow_mut().push(s);
}

pub(crate) fn take(out: &Out) -> Vec<S> {
    std::mem::take(&mut *out.borrow_mut())
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

/// `left + right` with constants folded (`add`).
pub(crate) fn add(left: E, right: f64) -> E {
    if right == 0.0 {
        return left;
    }
    if let E::Num(n) = left {
        return ir::num(n + right);
    }
    ir::binary(left, Op::Add, ir::num(right))
}

fn add_expr(left: E, right: E) -> E {
    if left.is_zero() {
        return right;
    }
    if let E::Num(n) = right {
        return add(left, n);
    }
    ir::binary(left, Op::Add, right)
}

/// A byte total under construction (`Sum`).
#[derive(Default)]
struct Sum {
    constant: f64,
    terms: Vec<E>,
}

impl Sum {
    fn add(&mut self, part: E) {
        match part {
            E::Num(n) => self.constant += n,
            other => self.terms.push(other),
        }
    }

    fn add_number(&mut self, n: f64) {
        self.constant += n;
    }

    fn build(self) -> E {
        let mut total: Option<E> = None;
        for term in self.terms {
            total = Some(match total {
                Some(t) => ir::binary(t, Op::Add, term),
                None => term,
            });
        }
        match total {
            Some(t) => add(t, self.constant),
            None => ir::num(self.constant),
        }
    }
}

/// A part of `interpolate`.
pub(crate) enum Part {
    Text(String),
    Expr(E),
}

/// `interpolate(parts)`: a template string, or a plain string without values.
pub(crate) fn interpolate(parts: Vec<Part>) -> E {
    let mut head = String::new();
    let mut spans: Vec<(E, String)> = Vec::new();
    for part in parts {
        match part {
            Part::Expr(e) => spans.push((e, String::new())),
            Part::Text(t) => match spans.last_mut() {
                Some(last) => last.1.push_str(&t),
                None => head.push_str(&t),
            },
        }
    }
    if spans.is_empty() {
        return ir::string(&head);
    }
    E::Template(head, spans)
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
    pub statements: Vec<S>,
    pub payload: Option<Id>,
    pub blobs: Option<Id>,
}

/// A value of a call site (`values[i]`).
#[derive(Clone)]
pub(crate) enum SiteValues {
    List(Vec<E>),
    Table(E),
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
    pub(crate) fn build_serializer(&mut self, ty: Shape, versioned: Option<(u32, String)>) -> GResult<E> {
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
        let value = self.parameter("v", Ty::Unknown);
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
        let serialize = ir::arrow(vec![ir::param(value)], Body::Block(serialize_body));

        let buf = self.uid("buf", Ty::Buffer);
        let blobs = if layout.blobs { Some(self.uid("blobs", Ty::Blobs)) } else { None };
        let params: Vec<Id> = std::iter::once(buf).chain(blobs).collect();
        let body = self.fitted(
            &mut |gn| {
                let statements = out();
                let result = gn.decode_body(&ty, layout, buf, blobs, &statements, header.as_ref())?;
                push(&statements, S::Return(Some(result)));
                Ok(take(&statements))
            },
            &params,
            Some(&named),
            None,
            0,
        )?;
        let deserialize = ir::arrow(params.iter().map(|&p| ir::param(p)).collect(), Body::Block(body));

        let mut fields = Vec::new();
        if let Some(h) = &header {
            fields.push((Key::Str("version".into()), ir::num(f64::from(h.version))));
        }
        fields.push((Key::Str("serialize".into()), serialize));
        fields.push((Key::Str("deserialize".into()), deserialize));
        Ok(ir::object(fields))
    }

    /// `buildDecoder(list)`: `None` when the list carries nothing.
    pub(crate) fn build_decoder(&mut self, list: Shape) -> GResult<Option<E>> {
        let kind = self.describe(&list)?;
        if self.carries_nothing(&kind)? {
            return Ok(None);
        }
        let layout = self.layout_of(&list)?;
        self.count_uses(&list)?;
        let buf = self.uid("buf", Ty::Buffer);
        let blobs = if layout.blobs { Some(self.uid("blobs", Ty::Blobs)) } else { None };
        let params: Vec<Id> = std::iter::once(buf).chain(blobs).collect();
        let body = self.fitted(
            &mut |gn| {
                let statements = out();
                let result = gn.decode_body(&list, layout, buf, blobs, &statements, None)?;
                push(&statements, S::Return(Some(result)));
                Ok(take(&statements))
            },
            &params,
            Some("these arguments"),
            None,
            0,
        )?;
        Ok(Some(ir::paren(ir::arrow(params.iter().map(|&p| ir::param(p)).collect(), Body::Block(body)))))
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
        let after_values: Vec<E> = if !after.is_empty() { values[values.len() - after.len()..].to_vec() } else { vec![] };
        let rest_values: Vec<E> = if values.len() > elements.len() + after_values.len() {
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
            (true, Some(E::Id(id))) => vec![*id],
            _ => Vec::new(),
        };
        let chosen_in = chosen.clone();
        let tail_chosen = chosen.clone();
        let mut tail = move |_gen: &mut Self| -> Vec<S> {
            let (buf, blobs) = *tail_chosen.borrow().last().expect("the build ran");
            let mut list = vec![ir::id(buf)];
            if let Some(b) = blobs {
                list.push(ir::id(b));
            }
            vec![S::Return(Some(ir::array(list)))]
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
        values: &[E],
        rest_values: &[E],
        after_values: &[E],
        count_bytes: &[u64],
    ) -> GResult<(Id, Option<Id>)> {
        let element_value = |i: usize| values.get(i).cloned().unwrap_or_else(ir::nil);
        let size_place = |i: usize| Place {
            path: Some(if site.result { "result".into() } else { format!("[{i}]") }),
            site: site.name.clone(),
            ..Place::default()
        };
        let mut total = Sum::default();
        let parts: Vec<(Shape, E, Place)> =
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
        let fixed = matches!(size, E::Num(_));
        let buf = self.uid("buf", Ty::Buffer);
        let create = self.buffer_call("create", vec![size]);
        push(statements, S::Const(Bind::One(buf), create));
        let blobs = if layout.blobs { Some(self.uid("blobs", Ty::Blobs)) } else { None };
        if let Some(b) = blobs {
            let array = self.construct("Array", vec![]);
            push(statements, S::Const(Bind::One(b), array));
        }
        let variable = if fixed { None } else { Some(self.uid("o", Ty::Number)) };
        if let Some(v) = variable {
            push(statements, S::Let(v, Some(ir::num(0.0))));
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
                let write = self.buffer_call("writeu8", vec![ir::id(buf), at, ir::num(byte as f64)]);
                push(statements, ir::stmt(write));
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
    fn encode_body(&mut self, shape: &Shape, layout: Layout, value: Id, path: &str, header: Option<&Header>) -> GResult<Vec<S>> {
        let body = out();
        let place = Place { path: Some(path.to_owned()), ..Place::default() };
        let (buf, blobs) = self.encode_into(shape, layout, ir::id(value), &body, &place, header)?;
        let result = match blobs {
            Some(b) => self.tuple(vec![ir::id(buf), ir::id(b)]),
            None => ir::id(buf),
        };
        push(&body, S::Return(Some(result)));
        Ok(take(&body))
    }

    /// `encodeInto(shape, layout, value, body, where, header)`.
    fn encode_into(
        &mut self,
        shape: &Shape,
        layout: Layout,
        value: E,
        body: &Out,
        place: &Place,
        header: Option<&Header>,
    ) -> GResult<(Id, Option<Id>)> {
        let size = self.emit_size(shape, value.clone(), body, place)?;
        let buf = self.uid("buf", Ty::Buffer);
        let total = if header.is_some() { add(size, HEADER_SIZE as f64) } else { size };
        let create = self.buffer_call("create", vec![total]);
        push(body, S::Const(Bind::One(buf), create));
        if let Some(h) = header {
            let a = self.buffer_call("writeu8", vec![ir::id(buf), ir::num(0.0), ir::num(f64::from(h.version))]);
            push(body, ir::stmt(a));
            let b = self.buffer_call("writeu32", vec![ir::id(buf), ir::num(1.0), ir::num(f64::from(h.hash))]);
            push(body, ir::stmt(b));
        }
        let start = if header.is_some() { HEADER_SIZE as i64 } else { 0 };
        let blobs = if layout.blobs { Some(self.uid("blobs", Ty::Blobs)) } else { None };
        if let Some(b) = blobs {
            let array = self.construct("Array", vec![]);
            push(body, S::Const(Bind::One(b), array));
        }
        let top = match shape {
            Shape::Type(t) => self.hoist(*t)?,
            Shape::Kind(_) => None,
        };
        if let Some(top) = top {
            let mut args = vec![ir::id(buf), ir::num(start as f64), value];
            if let Some(b) = blobs {
                args.push(ir::id(b));
            }
            if top.checks {
                let where_place = Place { path: place.path.clone(), site: place.site.clone(), ..Place::default() };
                args.push(self.where_of(&where_place));
            }
            let call = self.call_hoisted(&top, Role::W, args);
            push(body, ir::stmt(call));
        } else {
            let variable = if layout.size.is_none() { Some(self.uid("o", Ty::Number)) } else { None };
            if let Some(v) = variable {
                push(body, S::Let(v, Some(ir::num(start as f64))));
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
    ) -> GResult<E> {
        if let Some(h) = header {
            self.check_header(buf, h, body);
        }
        let start = if header.is_some() { HEADER_SIZE as i64 } else { 0 };
        let length = self.buffer_call("len", vec![ir::id(buf)]);
        if layout.zeros {
            let zeros = self.zero_tally();
            push(body, ir::assign(ir::id(zeros), ir::num(0.0)));
        }
        let top = match shape {
            Shape::Type(t) => self.hoist(*t)?,
            Shape::Kind(_) => None,
        };
        if let Some(top) = top {
            let value = self.uid("value", Ty::Unknown);
            let end = self.uid("o", Ty::Number);
            let mut args = vec![ir::id(buf), ir::num(start as f64)];
            if let Some(b) = blobs {
                args.push(ir::id(b));
            }
            let call = self.call_hoisted(&top, Role::R, args);
            push(body, S::Const(Bind::Tuple(vec![value, end]), call));
            let raise = self.raise(MALFORMED);
            push(body, ir::if_stmt(ir::binary(ir::id(end), Op::Ne, length), vec![raise], None));
            return Ok(ir::id(value));
        }
        let variable = if layout.size.is_none() { Some(self.uid("o", Ty::Number)) } else { None };
        if let Some(v) = variable {
            push(body, S::Let(v, Some(ir::num(start as f64))));
        }
        let ctx = Ctx::new(buf, blobs, variable, if variable.is_some() { 0 } else { start }, body.clone(), Place::default());
        let mut result = self.emit_read(shape, &ctx)?;
        if !result.is_identifier() && !result.is_literal() {
            result = self.bind(body, result, "value", None);
        }
        if let Some(v) = variable {
            self.sync(&ctx);
            let raise = self.raise(MALFORMED);
            push(body, ir::if_stmt(ir::binary(ir::id(v), Op::Ne, length), vec![raise], None));
        } else {
            let raise = self.raise(MALFORMED);
            let expected = ir::num((layout.size.unwrap() as i64 + start) as f64);
            push(body, ir::if_stmt(ir::binary(length, Op::Ne, expected), vec![raise], None));
        }
        Ok(result)
    }

    /// `checkHeader(buf, header, body)`.
    fn check_header(&mut self, buf: Id, header: &Header, body: &Out) {
        let Header { version, hash, name } = header;
        let len = self.buffer_call("len", vec![ir::id(buf)]);
        let raise = self.raise(&format!(
            "[Flamework] this buffer is too short for the header of {name}, version {version}: no versioned serializer wrote it"
        ));
        push(body, ir::if_stmt(ir::binary(len, Op::Lt, ir::num(HEADER_SIZE as f64)), vec![raise], None));

        let written = |gn: &mut Self| gn.buffer_call("readu8", vec![ir::id(buf), ir::num(0.0)]);
        let condition = ir::binary(written(self), Op::Ne, ir::num(f64::from(*version)));
        let message = interpolate(vec![
            Part::Text("[Flamework] this buffer is version ".into()),
            Part::Expr(written(self)),
            Part::Text(format!(
                ", and this serializer of {name} reads version {version}: read each version with its own serializer (Serialization.versionOf tells which), and a buffer written without a version with one without"
            )),
        ]);
        let raise = self.raise_with(message);
        push(body, ir::if_stmt(condition, vec![raise], None));

        let read = self.buffer_call("readu32", vec![ir::id(buf), ir::num(1.0)]);
        let raise = self.raise(&format!(
            "[Flamework] {name}'s layout changed since this buffer was written as version {version}; bump the version and keep a reader for the old one"
        ));
        push(body, ir::if_stmt(ir::binary(read, Op::Ne, ir::num(f64::from(*hash))), vec![raise], None));
    }

    // --- cursor ----------------------------------------------------------------------------------------------

    /// `at(ctx, extra)`.
    pub(crate) fn at(&self, ctx: &Ctx, extra: i64) -> E {
        let cursor = ctx.cursor.borrow();
        let offset = cursor.offset + extra;
        match cursor.base {
            Some(base) => add(ir::id(base), offset as f64),
            None => ir::num(offset as f64),
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
            push(&ctx.out, ir::add_assign(ir::id(variable), ir::num(offset as f64)));
        } else {
            push(&ctx.out, ir::assign(ir::id(variable), self.at(ctx, 0)));
        }
        let mut c = ctx.cursor.borrow_mut();
        c.base = Some(variable);
        c.offset = 0;
    }

    /// `advanceBy(ctx, amount)`.
    fn advance_by(&self, ctx: &Ctx, amount: E) {
        let (base, variable, offset) = {
            let c = ctx.cursor.borrow();
            (c.base, c.variable, c.offset)
        };
        let variable = variable.expect("Flamework: a variable-size write inside a fixed layout");
        if base == Some(variable) {
            push(&ctx.out, ir::add_assign(ir::id(variable), add_expr_offset(amount, offset)));
        } else {
            push(&ctx.out, ir::assign(ir::id(variable), add_expr(self.at(ctx, 0), amount)));
        }
        let mut c = ctx.cursor.borrow_mut();
        c.base = Some(variable);
        c.offset = 0;
    }

    /// `branch(ctx, build)`.
    fn branch(&mut self, ctx: &Ctx, build: &mut dyn FnMut(&mut Self, &Ctx) -> GResult<()>) -> GResult<Vec<S>> {
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

    /// `bind(out, value, hint, type)`.
    pub(crate) fn bind(&mut self, out_list: &Out, value: E, hint: &str, ty: Option<Ty>) -> E {
        if ty.is_none() {
            if value.is_identifier() && !self.is_parameter_reference(&value) {
                return value;
            }
            if let E::As(inner, _) = &value
                && inner.skip_casts().is_identifier()
            {
                return value;
            }
        }
        let declared = ty.unwrap_or_else(|| self.type_of_value(&value));
        let id = self.uid(hint, declared);
        push(out_list, S::Const(Bind::One(id), value));
        ir::id(id)
    }

    // --- compact functions -----------------------------------------------------------------------------------

    /// `fitted(build, parameters, what, tail, outer)`: built plainly, and again compactly when the plain build does not
    /// fit Luau's limits with [`limits::MARGIN`] to spare (DESIGN §3.2, [`limits`]).
    #[allow(clippy::type_complexity)]
    pub(crate) fn fitted(
        &mut self,
        build: &mut dyn FnMut(&mut Self) -> GResult<Vec<S>>,
        parameters: &[Id],
        what: Option<&str>,
        tail: Option<&mut dyn FnMut(&mut Self) -> Vec<S>>,
        outer: u32,
    ) -> GResult<Vec<S>> {
        self.fitted_choice(build, parameters, what, tail, outer).map(|(body, _)| body)
    }

    /// [`Self::fitted`], saying which build it kept: `true` for the compact rebuild, `false` for the first build.
    /// `outer` is the locals live around the code (a call site's), `tail` statements measured after the body, and
    /// `what` names the value in a refusal (a call site, `None`, keeps its compact build instead).
    #[allow(clippy::type_complexity)]
    pub(crate) fn fitted_choice(
        &mut self,
        build: &mut dyn FnMut(&mut Self) -> GResult<Vec<S>>,
        parameters: &[Id],
        what: Option<&str>,
        mut tail: Option<&mut dyn FnMut(&mut Self) -> Vec<S>>,
        outer: u32,
    ) -> GResult<(Vec<S>, bool)> {
        let previous = self.g.compact;
        self.g.compact = false;
        let result = (|| {
            let mut measuring = Measuring::default();
            let mut measure = |gn: &mut Self, measuring: &mut Measuring, body: &[S]| {
                let mut all = body.to_vec();
                if let Some(tail) = tail.as_mut() {
                    all.extend(tail(gn));
                }
                measuring.function(&gn.g.ids, parameters, &all)
            };
            let body = build(self)?;
            let recording = limits::record::on();
            let mut plain = measure(self, &mut measuring, &body);
            if !recording && plain.surely_fits(limits::MARGIN + outer) {
                return Ok((body, false));
            }
            // what the file hoisted so far ahead of its functions, in `take_hoisted`'s order
            let hoisted = |gn: &Self| [gn.g.declarations.len(), gn.g.tables.len()];
            let prelude = |gn: &Self, measuring: &mut Measuring| {
                let statements = [gn.g.declarations.as_slice(), gn.g.tables.as_slice()].concat();
                Rc::new(measuring.lower(&gn.g.ids, &statements))
            };
            let before = hoisted(self);
            plain.prelude = prelude(self, &mut measuring);
            let keep = limits::keeps_plain(&plain, outer).map_err(|error| self.does_not_compile(what, error))?;
            if keep {
                record(what, outer, &plain, None, false);
                return Ok((body, false));
            }
            self.g.compact = true;
            let rebuilt = build(self)?;
            let mut compact = measure(self, &mut measuring, &rebuilt);
            if hoisted(self) != before {
                // what the rebuild hoisted
                plain.prelude = prelude(self, &mut measuring);
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
    fn add_sizes(&mut self, out_list: &Out, total: &mut Sum, parts: Vec<(Shape, E, Place)>) -> GResult<()> {
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
            if matches!(size, E::Num(_)) && body.borrow().is_empty() {
                total.add(size);
                continue;
            }
            let r = match running {
                Some(r) => r,
                None => {
                    let r = self.uid("size", Ty::Number);
                    push(out_list, S::Let(r, Some(ir::num(0.0))));
                    running = Some(r);
                    r
                }
            };
            push(&body, ir::add_assign(ir::id(r), size));
            self.push_scoped(out_list, take(&body));
        }
        if let Some(r) = running {
            total.add(ir::id(r));
        }
        Ok(())
    }

    /// `pushScoped(out, body)`.
    fn push_scoped(&self, out_list: &Out, body: Vec<S>) {
        if holds_locals(&self.g.ids, &body) {
            push(out_list, S::Block(body));
        } else {
            out_list.borrow_mut().extend(body);
        }
    }

    // --- size ------------------------------------------------------------------------------------------------

    /// `emitSize(shape, value, out, place)`.
    pub(crate) fn emit_size(&mut self, shape: &Shape, value: E, out_list: &Out, place: &Place) -> GResult<E> {
        let layout = self.layout_of(shape)?;
        if let Some(size) = layout.size {
            return Ok(ir::num(size as f64));
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
            Kind::String { length, .. } => {
                let size = ir::macro_call(ir::cast(value, Ty::String), "size", vec![]);
                Ok(self.size_with_length(out_list, *length, size))
            }
            Kind::Buffer { length, .. } => {
                let len = self.buffer_call("len", vec![ir::cast(value, Ty::Buffer)]);
                Ok(self.size_with_length(out_list, *length, len))
            }
            Kind::Varint { .. } => {
                let v = self.varint_helpers();
                Ok(ir::call(ir::id(v.size), vec![ir::cast(value, Ty::Number)]))
            }
            Kind::Optional(inner) => {
                if value.is_nil_literal() {
                    return Ok(ir::num(1.0));
                }
                let inner_layout = self.layout_of(inner)?;
                let v = self.bind(out_list, value, "v", None);
                if let Some(size) = inner_layout.size {
                    return Ok(ir::cond(ir::binary(v, Op::Ne, ir::nil()), ir::num(1.0 + size as f64), ir::num(1.0)));
                }
                let total = self.uid("size", Ty::Number);
                push(out_list, S::Let(total, Some(ir::num(1.0))));
                let body = out();
                let size = self.emit_size(inner, v.clone(), &body, place)?;
                push(&body, ir::add_assign(ir::id(total), size));
                push(out_list, ir::if_stmt(ir::binary(v, Op::Ne, ir::nil()), take(&body), None));
                Ok(ir::id(total))
            }
            Kind::Array(element) => {
                let array = self.bind(out_list, ir::cast(value, Ty::Array), "array", None);
                let count = self.bind(out_list, ir::macro_call(array.clone(), "size", vec![]), "n", None);
                let vsize = self.varint_helpers().size;
                let prefix = ir::call(ir::id(vsize), vec![count.clone()]);
                let element_layout = self.layout_of(element)?;
                if let Some(size) = element_layout.size {
                    return Ok(counted_size(prefix, count, size));
                }
                let total = self.uid("size", Ty::Number);
                let index = self.uid("i", Ty::Number);
                push(out_list, S::Let(total, Some(prefix)));
                let body = out();
                let hole = if self.allows_nil(element)? { None } else { Some((place.clone(), "the array".to_owned())) };
                let item = self.element_at(array, index, &body, hole);
                let size = self.emit_size(element, item, &body, &place.within("[]", false))?;
                push(&body, ir::add_assign(ir::id(total), size));
                let range = self.range(ir::num(1.0), count);
                push(out_list, S::ForOf(Bind::One(index), range, take(&body)));
                Ok(ir::id(total))
            }
            Kind::Set(element) => {
                let set = self.bind(out_list, ir::cast(value, Ty::Set), "set", None);
                let element_layout = self.layout_of(element)?;
                let item = self.uid("item", Ty::Defined);
                let element = element.clone();
                self.counted_in_pass(out_list, set, Bind::One(item), &mut |gn, body| match element_layout.size {
                    Some(size) => Ok(ir::num(size as f64)),
                    None => gn.emit_size(&element, ir::id(item), body, &place.within("[]", false)),
                })
            }
            Kind::Map(key_shape, value_shape) => {
                let map = self.bind(out_list, ir::cast(value, Ty::Map), "map", None);
                let key = self.uid("key", Ty::Unknown);
                let entry = self.uid("entry", Ty::Unknown);
                let (key_shape, value_shape) = (key_shape.clone(), value_shape.clone());
                self.counted_in_pass(out_list, map, Bind::Tuple(vec![key, entry]), &mut |gn, body| {
                    let k = gn.emit_size(&key_shape, ir::id(key), body, &place.within("<key>", false))?;
                    let v = gn.emit_size(&value_shape, ir::id(entry), body, &place.within("<value>", false))?;
                    Ok(add_expr(k, v))
                })
            }
            Kind::List { elements, rest, after } => {
                let list = self.bind(out_list, ir::cast(value, Ty::Array), "list", None);
                let mut total = Sum::default();
                let parts = elements
                    .iter()
                    .enumerate()
                    .map(|(i, e)| (e.clone(), ir::elem(list.clone(), ir::num(i as f64)), place.within(&format!("[{i}]"), false)))
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
                        let at = add(ir::id(count), (elements.len() + i) as f64);
                        (e.clone(), ir::elem(list.clone(), at), place.within("[]", false))
                    })
                    .collect();
                self.add_sizes(out_list, &mut total, parts)?;
                let vsize = self.varint_helpers().size;
                let prefix = ir::call(ir::id(vsize), vec![ir::id(count)]);
                if let Some(size) = rest_layout.size {
                    total.add(counted_size(prefix, ir::id(count), size));
                    return Ok(total.build());
                }
                let sum = self.uid("size", Ty::Number);
                let index = self.uid("i", Ty::Number);
                total.add(prefix);
                push(out_list, S::Let(sum, Some(total.build())));
                let body = out();
                let element = if self.allows_nil(rest)? {
                    ir::elem(list.clone(), ir::binary(ir::id(index), Op::Sub, ir::num(1.0)))
                } else {
                    let container = tuple_name(place);
                    self.element_at(list.clone(), index, &body, Some((place.clone(), container)))
                };
                let size = self.emit_size(rest, element, &body, &place.within("[]", false))?;
                push(&body, ir::add_assign(ir::id(sum), size));
                let range = self.range(ir::num((elements.len() + 1) as f64), add(ir::id(count), elements.len() as f64));
                push(out_list, S::ForOf(Bind::One(index), range, take(&body)));
                Ok(ir::id(sum))
            }
            Kind::Object { fields } => {
                let object = self.bind(out_list, ir::cast(value, Ty::Record), "object", None);
                let mut total = Sum::default();
                let parts = fields
                    .iter()
                    .map(|f| (f.shape.clone(), key_access(object.clone(), &f.key), place.within(&key_segment(&f.key), false)))
                    .collect();
                self.add_sizes(out_list, &mut total, parts)?;
                Ok(total.build())
            }
            Kind::Union { alternatives, whole, .. } => {
                let v = self.bind(out_list, value, "v", None);
                let total = self.uid("size", Ty::Number);
                push(out_list, S::Let(total, Some(ir::num(1.0))));
                if typed {
                    self.table_precheck(&kind, v.clone(), out_list, place)?;
                }
                let mut chain: Option<S> = if typed {
                    let call = self.call_type_check(&kind, v.clone(), place)?;
                    Some(S::Block(vec![ir::stmt(call)]))
                } else {
                    None
                };
                let member = Place { tested: true, ..place.clone() };
                if let Some((index, _)) = self.numeric_fallback(&kind, false)? {
                    let shape = alternatives[index].shape.clone();
                    let member_size = self.layout_of(&shape)?.size;
                    let body = out();
                    let size = match member_size {
                        Some(s) => ir::num(s as f64),
                        None => self.emit_size(&shape, v.clone(), &body, &member)?,
                    };
                    push(&body, ir::add_assign(ir::id(total), size));
                    let test = self.type_of_is(v.clone(), "number");
                    chain = Some(ir::if_stmt(test, take(&body), chain.map(else_of)));
                }
                let order = self.evaluation(&kind)?.order.clone();
                for &i in order.iter().rev() {
                    let alternative = alternatives[i].clone();
                    let layout = self.layout_of(&alternative.shape)?;
                    let body = out();
                    let size = if *whole == Some(i) {
                        let n = ir::cast(v.clone(), Ty::Number);
                        let vsize = self.varint_helpers().size;
                        ir::cond(Self::is_whole(n.clone()), ir::call(ir::id(vsize), vec![n]), ir::num(8.0))
                    } else if let Some(s) = layout.size {
                        ir::num(s as f64)
                    } else {
                        self.emit_size(&alternative.shape, v.clone(), &body, &member)?
                    };
                    if !size.is_zero() {
                        push(&body, ir::add_assign(ir::id(total), size));
                    }
                    if body.borrow().is_empty() && chain.is_none() {
                        continue;
                    }
                    let test = self.discriminate(&kind, i, v.clone())?;
                    chain = Some(ir::if_stmt(test, take(&body), chain.map(else_of)));
                }
                if let Some(c) = chain {
                    push(out_list, c);
                }
                Ok(ir::id(total))
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

    /// `elementAt(list, index, out, hole)`.
    fn element_at(&mut self, list: E, index: Id, out_list: &Out, hole: Option<(Place, String)>) -> E {
        let item = self.uid("item", Ty::Unknown);
        push(out_list, S::Const(Bind::One(item), ir::elem(list, ir::binary(ir::id(index), Op::Sub, ir::num(1.0)))));
        if let Some((place, container)) = hole {
            let mut parts = vec![Part::Text(format!("[Flamework] {container} has no value at "))];
            parts.extend(self.hole_where(&place));
            parts.push(Part::Text("[".into()));
            parts.push(Part::Expr(ir::binary(ir::id(index), Op::Sub, ir::num(1.0))));
            parts.push(Part::Text("]".into()));
            let message = interpolate(parts);
            let raise = self.raise_with(message);
            push(out_list, ir::if_stmt(ir::binary(ir::id(item), Op::Eq, ir::nil()), vec![raise], None));
        }
        ir::id(item)
    }

    /// `holeWhere(place)`.
    fn hole_where(&self, place: &Place) -> Vec<Part> {
        if let Some(w) = place.where_ {
            return match &place.path {
                Some(p) if !p.is_empty() => vec![Part::Expr(ir::id(w)), Part::Text(p.clone())],
                _ => vec![Part::Expr(ir::id(w))],
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
        match self.where_of(place) {
            E::Str(s) => vec![Part::Text(s)],
            other => vec![Part::Expr(other)],
        }
    }

    // --- write -----------------------------------------------------------------------------------------------

    /// `emitWrite(shape, value, ctx)`.
    pub(crate) fn emit_write(&mut self, shape: &Shape, value: E, ctx: &Ctx) -> GResult<()> {
        if let Shape::Type(t) = shape
            && let Some(info) = self.hoist(*t)?
        {
            let mut args = vec![ir::id(ctx.buf), self.at(ctx, 0), value];
            if info.layout.blobs {
                args.push(ir::id(ctx.blobs.unwrap()));
            }
            if info.checks {
                args.push(self.passed_where(&ctx.place));
            }
            let call = self.call_hoisted(&info, Role::W, args);
            let variable = ctx.variable().unwrap();
            push(&ctx.out, ir::assign(ir::id(variable), call));
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
                    let write = gn.buffer_call(&format!("write{}", width.name()), vec![ir::id(target.buf), at, ir::cast(n, Ty::Number)]);
                    push(&target.out, ir::stmt(write));
                    target.cursor.borrow_mut().offset += i64::from(width.size());
                })
            }
            Kind::Varint { .. } => {
                self.write_number(&kind, value, ctx, &mut |gn, n, target| gn.write_varint(target, ir::cast(n, Ty::Number)))
            }
            Kind::Boolean => {
                let mut write = |gn: &mut Self, b: E, target: &Ctx| {
                    let at = gn.at(target, 0);
                    let flag = ir::cond(ir::cast(b, Ty::Boolean), ir::num(1.0), ir::num(0.0));
                    let call = gn.buffer_call("writeu8", vec![ir::id(target.buf), at, flag]);
                    push(&target.out, ir::stmt(call));
                    target.cursor.borrow_mut().offset += 1;
                };
                if typed {
                    if literal_type(&value).is_none() {
                        return self.write_typed(&kind, value, ctx, &mut write);
                    }
                    self.emit_type_check(&kind, value.clone(), &ctx.out, &ctx.place)?;
                }
                write(self, value, ctx);
                Ok(())
            }
            Kind::String { length: width, .. } => {
                let text = self.bind(&ctx.out, ir::cast(value, Ty::String), "text", None);
                let length = self.bind(&ctx.out, ir::macro_call(text.clone(), "size", vec![]), "length", None);
                let checked = self.checked_width(&kind, ctx.unchecked);
                self.write_length(ctx, *width, length.clone(), "string", checked.as_deref());
                let at = self.at(ctx, 0);
                let write = self.buffer_call("writestring", vec![ir::id(ctx.buf), at, text]);
                push(&ctx.out, ir::stmt(write));
                self.advance_by(ctx, length);
                Ok(())
            }
            Kind::Buffer { length: width, .. } => {
                let bytes = self.bind(&ctx.out, ir::cast(value, Ty::Buffer), "bytes", None);
                let len = self.buffer_call("len", vec![bytes.clone()]);
                let length = self.bind(&ctx.out, len, "length", None);
                let checked = self.checked_width(&kind, ctx.unchecked);
                self.write_length(ctx, *width, length.clone(), "buffer", checked.as_deref());
                let at = self.at(ctx, 0);
                let copy = self.buffer_call("copy", vec![ir::id(ctx.buf), at, bytes]);
                push(&ctx.out, ir::stmt(copy));
                self.advance_by(ctx, length);
                Ok(())
            }
            Kind::Constant(_) | Kind::Nothing => Ok(()),
            Kind::Literals(values) => {
                let (_, index) = self.literal_tables_for(&kind);
                let v = self.bind(&ctx.out, value, "v", None);
                let get = ir::macro_call(index, "get", vec![ir::cast(v.clone(), Ty::Defined)]);
                let slot = self.bind(&ctx.out, get, "index", Some(Ty::Number));
                let refuse = self.raise("value is not one of the literals its type allows");
                let body = if typed {
                    let call = self.call_type_check(&kind, v, &ctx.place)?;
                    vec![ir::stmt(call), refuse]
                } else {
                    vec![refuse]
                };
                push(&ctx.out, ir::if_stmt(ir::binary(slot.clone(), Op::Eq, ir::nil()), body, None));
                let width = if values.len() > 0xff { Width::U16 } else { Width::U8 };
                let at = self.at(ctx, 0);
                let write = self.buffer_call(&format!("write{}", width.name()), vec![ir::id(ctx.buf), at, slot]);
                push(&ctx.out, ir::stmt(write));
                ctx.cursor.borrow_mut().offset += i64::from(width.size());
                Ok(())
            }
            Kind::Blob { .. } => {
                let blob = self.bind(&ctx.out, value, "blob", None);
                let blobs = ctx.blobs.unwrap();
                let at = self.at(ctx, 0);
                let size = ir::macro_call(ir::id(blobs), "size", vec![]);
                let mut present = vec![
                    ir::stmt(ir::macro_call(ir::id(blobs), "push", vec![ir::cast(blob.clone(), Ty::Defined)])),
                    ir::stmt(self.buffer_call("writeu32", vec![ir::id(ctx.buf), at.clone(), size])),
                ];
                if typed {
                    present.insert(0, self.type_check_statement(&kind, blob.clone(), &ctx.place)?);
                }
                let absent = vec![ir::stmt(self.buffer_call("writeu32", vec![ir::id(ctx.buf), at, ir::num(0.0)]))];
                push(&ctx.out, ir::if_stmt(ir::binary(blob, Op::Ne, ir::nil()), present, Some(ir::Else::Block(absent))));
                ctx.cursor.borrow_mut().offset += BLOB_SIZE;
                Ok(())
            }
            Kind::Datatype(name) => {
                let (_, fields) = datatype(name).unwrap();
                let hint = name.to_lowercase();
                let value = self.bind(&ctx.out, ir::cast(value, Ty::Datatype), &hint, None);
                for (width, names) in fields {
                    let at = self.at(ctx, 0);
                    let path = names.iter().fold(value.clone(), |current, n| ir::prop(current, n));
                    let write = self.buffer_call(&format!("write{}", width.name()), vec![ir::id(ctx.buf), at, path]);
                    push(&ctx.out, ir::stmt(write));
                    ctx.cursor.borrow_mut().offset += i64::from(width.size());
                }
                Ok(())
            }
            Kind::CFrame => {
                let components: Vec<Id> = (0..12).map(|i| self.uid(&format!("c{i}"), Ty::Number)).collect();
                push(&ctx.out, S::Const(Bind::Tuple(components.clone()), ir::method(ir::cast(value, Ty::CFrame), "GetComponents", vec![])));
                for c in components {
                    let at = self.at(ctx, 0);
                    let write = self.buffer_call("writef32", vec![ir::id(ctx.buf), at, ir::id(c)]);
                    push(&ctx.out, ir::stmt(write));
                    ctx.cursor.borrow_mut().offset += 4;
                }
                Ok(())
            }
            Kind::Enum(_) => {
                let at = self.at(ctx, 0);
                let write = self.buffer_call("writeu16", vec![ir::id(ctx.buf), at, ir::prop(ir::cast(value, Ty::EnumItem), "Value")]);
                push(&ctx.out, ir::stmt(write));
                ctx.cursor.borrow_mut().offset += 2;
                Ok(())
            }
            Kind::Optional(inner) => {
                if value.is_nil_literal() {
                    let at = self.at(ctx, 0);
                    let write = self.buffer_call("writeu8", vec![ir::id(ctx.buf), at, ir::num(0.0)]);
                    push(&ctx.out, ir::stmt(write));
                    ctx.cursor.borrow_mut().offset += 1;
                    return Ok(());
                }
                let v = self.bind(&ctx.out, value, "v", None);
                let at = self.at(ctx, 0);
                let flag = ir::cond(ir::binary(v.clone(), Op::Ne, ir::nil()), ir::num(1.0), ir::num(0.0));
                let write = self.buffer_call("writeu8", vec![ir::id(ctx.buf), at, flag]);
                push(&ctx.out, ir::stmt(write));
                ctx.cursor.borrow_mut().offset += 1;
                let inner = inner.clone();
                let v2 = v.clone();
                let body = self.branch(ctx, &mut |gn, child| gn.emit_write(&inner, v2.clone(), child))?;
                push(&ctx.out, ir::if_stmt(ir::binary(v, Op::Ne, ir::nil()), body, None));
                Ok(())
            }
            Kind::Array(element) => {
                let array = self.bind(&ctx.out, ir::cast(value, Ty::Array), "array", None);
                self.write_varint(ctx, ir::macro_call(array.clone(), "size", vec![]));
                let hole = if self.hole_in_write(element)? { Some((ctx.place.clone(), "the array".to_owned())) } else { None };
                let index = self.uid("i", Ty::Number);
                let range = self.range(ir::num(1.0), ir::macro_call(array.clone(), "size", vec![]));
                let element = element.clone();
                let body = self.branch(ctx, &mut |gn, child| {
                    let item = gn.element_at(array.clone(), index, &child.out, hole.clone());
                    gn.emit_write(&element, item, &child.within("[]", false))
                })?;
                push(&ctx.out, S::ForOf(Bind::One(index), range, body));
                Ok(())
            }
            Kind::Set(element) => {
                let set = self.bind(&ctx.out, ir::cast(value, Ty::Set), "set", None);
                let element = element.clone();
                self.write_counted(ctx, set, None, &mut |gn, item, child| gn.emit_write(&element, ir::id(item), &child.within("[]", false)))
            }
            Kind::Map(key_shape, value_shape) => {
                let map = self.bind(&ctx.out, ir::cast(value, Ty::Map), "map", None);
                let key = self.uid("key", Ty::Unknown);
                let entry = self.uid("entry", Ty::Unknown);
                let (key_shape, value_shape) = (key_shape.clone(), value_shape.clone());
                self.write_counted(ctx, map, Some(Bind::Tuple(vec![key, entry])), &mut |gn, _, child| {
                    gn.emit_write(&key_shape, ir::id(key), &child.within("<key>", false))?;
                    gn.emit_write(&value_shape, ir::id(entry), &child.within("<value>", false))
                })
            }
            Kind::List { elements, rest, after } => {
                let list = self.bind(&ctx.out, ir::cast(value, Ty::Array), "list", None);
                for (i, element) in elements.iter().enumerate() {
                    self.emit_scoped_write(element, ir::elem(list.clone(), ir::num(i as f64)), &ctx.within(&format!("[{i}]"), false))?;
                }
                if let Some(rest) = rest {
                    let after: Vec<Shape> = after.clone().unwrap_or_default();
                    let count = self.rest_count(&ctx.out, list.clone(), elements.len() + after.len());
                    self.write_varint(ctx, ir::id(count));
                    let index = self.uid("i", Ty::Number);
                    let hole = if self.hole_in_write(rest)? { Some((ctx.place.clone(), tuple_name(&ctx.place))) } else { None };
                    let range = self.range(ir::num((elements.len() + 1) as f64), add(ir::id(count), elements.len() as f64));
                    let rest = rest.clone();
                    let body = self.branch(ctx, &mut |gn, child| {
                        let element = match &hole {
                            Some(h) => gn.element_at(list.clone(), index, &child.out, Some(h.clone())),
                            None => ir::elem(list.clone(), ir::binary(ir::id(index), Op::Sub, ir::num(1.0))),
                        };
                        gn.emit_write(&rest, element, &child.within("[]", false))
                    })?;
                    push(&ctx.out, S::ForOf(Bind::One(index), range, body));
                    for (position, shape) in after.iter().enumerate() {
                        let at = add(ir::id(count), (elements.len() + position) as f64);
                        self.emit_scoped_write(shape, ir::elem(list.clone(), at), &ctx.within("[]", false))?;
                    }
                }
                Ok(())
            }
            Kind::Object { fields } => {
                let object = self.bind(&ctx.out, ir::cast(value, Ty::Record), "object", None);
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
        value: E,
        ctx: &Ctx,
        typed: bool,
    ) -> GResult<()> {
        let v = self.bind(&ctx.out, value, "v", None);
        let layout = self.layout_of(&Shape::Kind(kind.clone()))?;
        let is_fixed_layout = ctx.variable().is_none();
        let start = ctx.cursor.borrow().offset;
        if typed {
            self.table_precheck(kind, v.clone(), &ctx.out, &ctx.place)?;
        }
        let refuse = self.raise("value matches none of the union's members");
        let mut chain = if typed {
            let call = self.call_type_check(kind, v.clone(), &ctx.place)?;
            S::Block(vec![ir::stmt(call), refuse])
        } else {
            S::Block(vec![refuse])
        };
        let checks_types = self.g.checks.types;
        let write_member = |gn: &mut Self, child: &Ctx, i: usize| -> GResult<()> {
            let at = gn.at(child, 0);
            let tag = gn.buffer_call("writeu8", vec![ir::id(child.buf), at, ir::num(i as f64)]);
            push(&child.out, ir::stmt(tag));
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
            let check = self.call_check(&widths, ir::cast(v.clone(), Ty::Number), ctx, None);
            let test = ir::binary(is_number, Op::And, check);
            let body = self.branch(ctx, &mut |gn, child| write_member(gn, child, index))?;
            chain = ir::if_stmt(test, body, Some(else_of(chain)));
        }

        let order = self.evaluation(kind)?.order.clone();
        let count = alternatives.len();
        for &i in order.iter().rev() {
            let body = self.branch(ctx, &mut |gn, child| {
                if whole != Some(i) {
                    return write_member(gn, child, i);
                }
                let n = ir::cast(v.clone(), Ty::Number);
                let n2 = n.clone();
                let whole_body = gn.branch(child, &mut |gn, inner| {
                    let at = gn.at(inner, 0);
                    let tag = gn.buffer_call("writeu8", vec![ir::id(inner.buf), at, ir::num(count as f64)]);
                    push(&inner.out, ir::stmt(tag));
                    inner.cursor.borrow_mut().offset += 1;
                    gn.write_varint(inner, n2.clone());
                    Ok(())
                })?;
                let other = gn.branch(child, &mut |gn, inner| write_member(gn, inner, i))?;
                push(&child.out, ir::if_stmt(Self::is_whole(n), whole_body, Some(ir::Else::Block(other))));
                Ok(())
            })?;
            let test = self.discriminate(kind, i, v.clone())?;
            chain = ir::if_stmt(test, body, Some(else_of(chain)));
        }
        push(&ctx.out, chain);
        if is_fixed_layout && let Some(size) = layout.size {
            ctx.cursor.borrow_mut().offset = start + size as i64;
        }
        Ok(())
    }

    /// `emitScopedWrite(shape, value, place)`.
    fn emit_scoped_write(&mut self, shape: &Shape, value: E, ctx: &Ctx) -> GResult<()> {
        if !self.g.checks.types && !self.g.compact {
            return self.emit_write(shape, value, ctx);
        }
        let block = ctx.with_out(out());
        self.emit_write(shape, value, &block)?;
        let statements = take(&block.out);
        if holds_locals(&self.g.ids, &statements) {
            push(&ctx.out, S::Block(statements));
        } else {
            ctx.out.borrow_mut().extend(statements);
        }
        Ok(())
    }

    /// `holeInWrite(element)`.
    fn hole_in_write(&mut self, element: &Shape) -> GResult<bool> {
        Ok(!self.allows_nil(element)? && self.layout_of(element)?.size.is_some())
    }

    /// `restCount(out, list, fixed)`.
    fn rest_count(&mut self, out_list: &Out, list: E, fixed: usize) -> Id {
        let count = self.uid("count", Ty::Number);
        let size = ir::macro_call(list, "size", vec![]);
        let value = if fixed == 0 {
            size
        } else {
            let length = ir::binary(size, Op::Sub, ir::num(fixed as f64));
            self.math_call("max", vec![length, ir::num(0.0)])
        };
        push(out_list, S::Const(Bind::One(count), value));
        count
    }

    /// `writeLength(ctx, width, length, what, checked)`.
    fn write_length(&mut self, ctx: &Ctx, width: LengthWidth, length: E, what: &str, checked: Option<&str>) {
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
                    vec![ir::stmt(call), refuse]
                }
                None => vec![refuse],
            };
            push(&ctx.out, ir::if_stmt(ir::binary(length.clone(), Op::Gt, ir::num(max)), body, None));
        }
        let at = self.at(ctx, 0);
        let write = self.buffer_call(&format!("write{}", fixed.name()), vec![ir::id(ctx.buf), at, length]);
        push(&ctx.out, ir::stmt(write));
        ctx.cursor.borrow_mut().offset += i64::from(fixed.size());
    }

    /// `writeCounted(ctx, collection, body, binding)`.
    fn write_counted(
        &mut self,
        ctx: &Ctx,
        collection: E,
        binding: Option<Bind>,
        body: &mut dyn FnMut(&mut Self, Id, &Ctx) -> GResult<()>,
    ) -> GResult<()> {
        let count = self.bind(&ctx.out, ir::macro_call(collection.clone(), "size", vec![]), "n", None);
        self.write_varint(ctx, count);
        let item = self.uid("item", Ty::Defined);
        let statements = self.branch(ctx, &mut |gn, child| body(gn, item, child))?;
        push(&ctx.out, S::ForOf(binding.unwrap_or(Bind::One(item)), collection, statements));
        Ok(())
    }

    /// `writeNumber(kind, value, ctx, write)`.
    fn write_number(
        &mut self,
        kind: &Rc<Kind>,
        value: E,
        ctx: &Ctx,
        write: &mut dyn FnMut(&mut Self, E, &Ctx),
    ) -> GResult<()> {
        let width = self.checked_width(kind, ctx.unchecked);
        let range = number_range(kind);
        let typed = self.type_check_in(&Shape::Kind(kind.clone()), false, &ctx.place)?.is_some();
        let ranged = width.is_some() && range.is_some();
        if !ranged && !typed {
            write(self, value, ctx);
            return Ok(());
        }
        if let Some(known) = value.literal_number() {
            if ranged && !fits_statically(known, range.unwrap()) {
                let call = self.call_check(width.as_deref().unwrap(), value.clone(), ctx, None);
                push(&ctx.out, ir::stmt(call));
            }
            write(self, value, ctx);
            return Ok(());
        }
        let block = ctx.with_out(out());
        let n = if value.is_identifier() && !(typed && self.is_parameter_reference(&value)) {
            value
        } else {
            self.bind(&block.out, value, "n", None)
        };
        let target = if block.out.borrow().is_empty() { ctx.clone() } else { block.clone() };
        if typed {
            let statement = self.type_check_statement(kind, n.clone(), &ctx.place)?;
            push(&target.out, statement);
        }
        if ranged {
            let fails = self.fails_range(ir::cast(n.clone(), Ty::Number), range.unwrap());
            let call = self.call_check(width.as_deref().unwrap(), ir::cast(n.clone(), Ty::Number), ctx, None);
            push(&target.out, ir::if_stmt(fails, vec![ir::stmt(call)], None));
        }
        write(self, n, &target);
        if Rc::ptr_eq(&target.out, &block.out) {
            let statements = take(&block.out);
            push(&ctx.out, S::Block(statements));
        }
        Ok(())
    }

    /// `writeTyped(kind, value, ctx, write)`.
    fn write_typed(&mut self, kind: &Rc<Kind>, value: E, ctx: &Ctx, write: &mut dyn FnMut(&mut Self, E, &Ctx)) -> GResult<()> {
        let block = ctx.with_out(out());
        let v = if value.is_identifier() && !self.is_parameter_reference(&value) {
            value
        } else {
            self.bind(&block.out, value, "v", None)
        };
        let target = if block.out.borrow().is_empty() { ctx.clone() } else { block.clone() };
        let statement = self.type_check_statement(kind, v.clone(), &ctx.place)?;
        push(&target.out, statement);
        write(self, v, &target);
        if Rc::ptr_eq(&target.out, &block.out) {
            let statements = take(&block.out);
            push(&ctx.out, S::Block(statements));
        }
        Ok(())
    }

    /// `writeVarint(ctx, n)`: `o = vwrite(buf, o, n)`.
    fn write_varint(&mut self, ctx: &Ctx, n: E) {
        let variable = ctx.variable().expect("Flamework: a varint inside a fixed layout");
        let vwrite = self.varint_helpers().write;
        let at = self.at(ctx, 0);
        push(&ctx.out, ir::assign(ir::id(variable), ir::call(ir::id(vwrite), vec![ir::id(ctx.buf), at, n])));
        let mut c = ctx.cursor.borrow_mut();
        c.base = Some(variable);
        c.offset = 0;
    }

    /// `readVarint(ctx, hint)`: `const [n, o2] = vread(buf, o)`.
    fn read_varint(&mut self, ctx: &Ctx, hint: &str) -> Id {
        assert!(ctx.variable().is_some(), "Flamework: a varint inside a fixed layout");
        let n = self.uid(hint, Ty::Number);
        let next = self.uid("o", Ty::Number);
        let vread = self.varint_helpers().read;
        let at = self.at(ctx, 0);
        push(&ctx.out, S::Const(Bind::Tuple(vec![n, next]), ir::call(ir::id(vread), vec![ir::id(ctx.buf), at])));
        let mut c = ctx.cursor.borrow_mut();
        c.base = Some(next);
        c.offset = 0;
        n
    }

    /// `writeWide(ctx, width, value)`.
    fn write_wide(&mut self, ctx: &Ctx, width: Width, value: E) {
        let high = if width == Width::U64 { "writeu32" } else { "writei32" };
        if let Some(known) = value.skip_casts().literal_number()
            && known.is_finite()
        {
            let upper = (known / HALF).floor();
            let at = self.at(ctx, 0);
            let a = self.buffer_call("writeu32", vec![ir::id(ctx.buf), at, ir::num(known - upper * HALF)]);
            push(&ctx.out, ir::stmt(a));
            let at4 = self.at(ctx, 4);
            let b = self.buffer_call(high, vec![ir::id(ctx.buf), at4, ir::num(upper)]);
            push(&ctx.out, ir::stmt(b));
            ctx.cursor.borrow_mut().offset += i64::from(width.size());
            return;
        }
        let block = ctx.with_out(out());
        let n = if value.skip_casts().is_identifier() { value } else { self.bind(&block.out, value, "n", None) };
        let target = if block.out.borrow().is_empty() { ctx.clone() } else { block.clone() };
        let number = ir::cast(n, Ty::Number);
        let at = self.at(&target, 0);
        let low = self.buffer_call("writeu32", vec![ir::id(target.buf), at, ir::binary(number.clone(), Op::Mod, ir::num(HALF))]);
        push(&target.out, ir::stmt(low));
        let upper = self.math_call("floor", vec![ir::binary(number, Op::Div, ir::num(HALF))]);
        let at4 = self.at(&target, 4);
        let high_write = self.buffer_call(high, vec![ir::id(target.buf), at4, upper]);
        push(&target.out, ir::stmt(high_write));
        target.cursor.borrow_mut().offset += i64::from(width.size());
        if Rc::ptr_eq(&target.out, &block.out) {
            let statements = take(&block.out);
            push(&ctx.out, S::Block(statements));
        }
    }

    /// `readWide(ctx, width)`.
    fn read_wide(&mut self, ctx: &Ctx, width: Width) -> E {
        let at4 = self.at(ctx, 4);
        let high = self.buffer_call(if width == Width::U64 { "readu32" } else { "readi32" }, vec![ir::id(ctx.buf), at4]);
        let at = self.at(ctx, 0);
        let low = self.buffer_call("readu32", vec![ir::id(ctx.buf), at]);
        ctx.cursor.borrow_mut().offset += i64::from(width.size());
        ir::binary(ir::binary(high, Op::Mul, ir::num(HALF)), Op::Add, low)
    }

    /// `sizeWithLength(out, width, length)`.
    fn size_with_length(&mut self, out_list: &Out, width: LengthWidth, length: E) -> E {
        if let Some(fixed) = width.width() {
            return add(length, f64::from(fixed.size()));
        }
        let bound = self.bind(out_list, length, "length", None);
        let vsize = self.varint_helpers().size;
        add_expr(ir::call(ir::id(vsize), vec![bound.clone()]), bound)
    }

    /// `countedInPass(out, collection, binding, element)`.
    fn counted_in_pass(
        &mut self,
        out_list: &Out,
        collection: E,
        binding: Bind,
        element: &mut dyn FnMut(&mut Self, &Out) -> GResult<E>,
    ) -> GResult<E> {
        let total = self.uid("size", Ty::Number);
        let count = self.uid("n", Ty::Number);
        push(out_list, S::Let(total, Some(ir::num(0.0))));
        push(out_list, S::Let(count, Some(ir::num(0.0))));
        let body = out();
        push(&body, ir::add_assign(ir::id(count), ir::num(1.0)));
        let size = element(self, &body)?;
        push(&body, ir::add_assign(ir::id(total), size));
        push(out_list, S::ForOf(binding, collection, take(&body)));
        let vsize = self.varint_helpers().size;
        push(out_list, ir::add_assign(ir::id(total), ir::call(ir::id(vsize), vec![ir::id(count)])));
        Ok(ir::id(total))
    }

    // --- read ------------------------------------------------------------------------------------------------

    /// `emitRead(shape, ctx)`.
    pub(crate) fn emit_read(&mut self, shape: &Shape, ctx: &Ctx) -> GResult<E> {
        if let Shape::Type(t) = shape
            && let Some(info) = self.hoist(*t)?
        {
            let mut args = vec![ir::id(ctx.buf), self.at(ctx, 0)];
            if info.layout.blobs {
                args.push(ir::id(ctx.blobs.unwrap()));
            }
            let value = self.uid("value", Ty::Unknown);
            let next = self.uid("o", Ty::Number);
            let call = self.call_hoisted(&info, Role::R, args);
            push(&ctx.out, S::Const(Bind::Tuple(vec![value, next]), call));
            let mut c = ctx.cursor.borrow_mut();
            c.base = Some(next);
            c.offset = 0;
            return Ok(ir::id(value));
        }
        let kind = self.describe(shape)?;
        match &*kind {
            Kind::Number { width, .. } => {
                if width.is_wide() {
                    return Ok(self.read_wide(ctx, *width));
                }
                let at = self.at(ctx, 0);
                let read = self.buffer_call(&format!("read{}", width.name()), vec![ir::id(ctx.buf), at]);
                ctx.cursor.borrow_mut().offset += i64::from(width.size());
                Ok(read)
            }
            Kind::Varint { .. } => Ok(ir::id(self.read_varint(ctx, "n"))),
            Kind::Boolean => {
                let at = self.at(ctx, 0);
                let read = self.buffer_call("readu8", vec![ir::id(ctx.buf), at]);
                ctx.cursor.borrow_mut().offset += 1;
                Ok(ir::binary(read, Op::Ne, ir::num(0.0)))
            }
            Kind::String { length: width, .. } => {
                let length = self.read_length(ctx, *width);
                let at = self.at(ctx, 0);
                let read = self.buffer_call("readstring", vec![ir::id(ctx.buf), at, length.clone()]);
                let text = self.bind(&ctx.out, read, "text", Some(Ty::String));
                self.advance_by(ctx, length);
                Ok(text)
            }
            Kind::Buffer { length: width, .. } => {
                let length = self.read_length(ctx, *width);
                let len = self.buffer_call("len", vec![ir::id(ctx.buf)]);
                let remaining = ir::binary(len, Op::Sub, self.at(ctx, 0));
                let raise = self.raise(MALFORMED);
                push(&ctx.out, ir::if_stmt(ir::binary(length.clone(), Op::Gt, remaining), vec![raise], None));
                let create = self.buffer_call("create", vec![length.clone()]);
                let bytes = self.bind(&ctx.out, create, "bytes", Some(Ty::Buffer));
                let at = self.at(ctx, 0);
                let copy = self.buffer_call("copy", vec![bytes.clone(), ir::num(0.0), ir::id(ctx.buf), at, length.clone()]);
                push(&ctx.out, ir::stmt(copy));
                self.advance_by(ctx, length);
                Ok(bytes)
            }
            Kind::Constant(value) => {
                self.check_globals_in(value);
                Ok(value.to_ir())
            }
            Kind::Nothing => Ok(ir::nil()),
            Kind::Literals(values) => {
                let (list, _) = self.literal_tables_for(&kind);
                let width = if values.len() > 0xff { Width::U16 } else { Width::U8 };
                let at = self.at(ctx, 0);
                let index = self.buffer_call(&format!("read{}", width.name()), vec![ir::id(ctx.buf), at]);
                ctx.cursor.borrow_mut().offset += i64::from(width.size());
                let value = self.bind(&ctx.out, ir::elem(list, index), "literal", Some(Ty::Unknown));
                let raise = self.raise(MALFORMED);
                push(&ctx.out, ir::if_stmt(ir::binary(value.clone(), Op::Eq, ir::nil()), vec![raise], None));
                Ok(value)
            }
            Kind::Blob { .. } => {
                let at = self.at(ctx, 0);
                let index = self.buffer_call("readu32", vec![ir::id(ctx.buf), at]);
                ctx.cursor.borrow_mut().offset += BLOB_SIZE;
                Ok(ir::elem(ir::id(ctx.blobs.unwrap()), ir::binary(index, Op::Sub, ir::num(1.0))))
            }
            Kind::Datatype(name) => {
                let (_, fields) = datatype(name).unwrap();
                let mut args = Vec::new();
                for (width, _) in fields {
                    let at = self.at(ctx, 0);
                    args.push(self.buffer_call(&format!("read{}", width.name()), vec![ir::id(ctx.buf), at]));
                    ctx.cursor.borrow_mut().offset += i64::from(width.size());
                }
                Ok(self.construct(name, args))
            }
            Kind::CFrame => {
                let mut args = Vec::new();
                for _ in 0..12 {
                    let at = self.at(ctx, 0);
                    args.push(self.buffer_call("readf32", vec![ir::id(ctx.buf), at]));
                    ctx.cursor.borrow_mut().offset += 4;
                }
                Ok(self.construct("CFrame", args))
            }
            Kind::Enum(name) => {
                let at = self.at(ctx, 0);
                let index = self.buffer_call("readu16", vec![ir::id(ctx.buf), at]);
                ctx.cursor.borrow_mut().offset += 2;
                let table = self.enum_table_for(name);
                Ok(ir::elem(table, index))
            }
            Kind::Optional(inner) => {
                let at = self.at(ctx, 0);
                let read = self.buffer_call("readu8", vec![ir::id(ctx.buf), at]);
                let present = self.bind(&ctx.out, ir::binary(read, Op::Ne, ir::num(0.0)), "present", None);
                ctx.cursor.borrow_mut().offset += 1;
                let value = self.uid("value", Ty::Unknown);
                push(&ctx.out, S::Let(value, None));
                let inner = inner.clone();
                let body = self.branch(ctx, &mut |gn, child| {
                    let read = gn.emit_read(&inner, child)?;
                    push(&child.out, ir::assign(ir::id(value), read));
                    Ok(())
                })?;
                push(&ctx.out, ir::if_stmt(present, body, None));
                Ok(ir::id(value))
            }
            Kind::Array(element) => {
                let min = self.layout_of(element)?.min;
                let count = self.read_count(ctx, min);
                let created = self.construct("Array", vec![count.clone()]);
                let array = self.bind(&ctx.out, created, "array", Some(Ty::Array));
                let index = self.uid("i", Ty::Number);
                let element = element.clone();
                let range = self.range(ir::num(1.0), count);
                let body = self.branch(ctx, &mut |gn, child| {
                    let read = gn.emit_read(&element, child)?;
                    push(&child.out, ir::assign(ir::elem(array.clone(), ir::binary(ir::id(index), Op::Sub, ir::num(1.0))), read));
                    Ok(())
                })?;
                push(&ctx.out, S::ForOf(Bind::One(index), range, body));
                Ok(array)
            }
            Kind::Set(element) => {
                let min = self.layout_of(element)?.min;
                let count = self.read_count(ctx, min);
                let created = self.construct("Set", vec![]);
                let set = self.bind(&ctx.out, created, "set", Some(Ty::Set));
                let loop_id = self.uid("_", Ty::Number);
                let element = element.clone();
                let range = self.range(ir::num(1.0), count);
                let body = self.branch(ctx, &mut |gn, child| {
                    let read = gn.emit_read(&element, child)?;
                    push(&child.out, ir::stmt(ir::macro_call(set.clone(), "add", vec![ir::cast(read, Ty::Defined)])));
                    Ok(())
                })?;
                push(&ctx.out, S::ForOf(Bind::One(loop_id), range, body));
                Ok(set)
            }
            Kind::Map(key_shape, value_shape) => {
                let min = self.layout_of(key_shape)?.min + self.layout_of(value_shape)?.min;
                let count = self.read_count(ctx, min);
                let created = self.construct("Map", vec![]);
                let map = self.bind(&ctx.out, created, "map", Some(Ty::Map));
                let loop_id = self.uid("_", Ty::Number);
                let (key_shape, value_shape) = (key_shape.clone(), value_shape.clone());
                let range = self.range(ir::num(1.0), count);
                let body = self.branch(ctx, &mut |gn, child| {
                    let key = gn.read_into(&key_shape, child, "key")?;
                    let value = gn.read_into(&value_shape, child, "entry")?;
                    push(&child.out, ir::stmt(ir::macro_call(map.clone(), "set", vec![key, value])));
                    Ok(())
                })?;
                push(&ctx.out, S::ForOf(Bind::One(loop_id), range, body));
                Ok(map)
            }
            Kind::List { elements, rest, after } => self.read_list(elements, rest.as_ref(), after.as_deref(), ctx),
            Kind::Object { fields } => {
                if self.g.compact {
                    let object = self.bind(&ctx.out, ir::object(vec![]), "object", Some(Ty::Record));
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
                Ok(ir::object(entries))
            }
            Kind::Union { alternatives, whole, .. } => {
                let layout = self.layout_of(&Shape::Kind(kind.clone()))?;
                let is_fixed_layout = ctx.variable().is_none();
                let at = self.at(ctx, 0);
                let read = self.buffer_call("readu8", vec![ir::id(ctx.buf), at]);
                let tag = self.bind(&ctx.out, read, "tag", None);
                ctx.cursor.borrow_mut().offset += 1;
                let start = ctx.cursor.borrow().offset;
                let value = self.uid("value", Ty::Unknown);
                push(&ctx.out, S::Let(value, None));
                let raise = self.raise(MALFORMED);
                let mut chain = S::Block(vec![raise]);
                if whole.is_some() {
                    let body = self.branch(ctx, &mut |gn, child| {
                        let n = gn.read_varint(child, "n");
                        push(&child.out, ir::assign(ir::id(value), ir::id(n)));
                        Ok(())
                    })?;
                    chain = ir::if_stmt(ir::binary(tag.clone(), Op::Eq, ir::num(alternatives.len() as f64)), body, Some(else_of(chain)));
                }
                for i in (0..alternatives.len()).rev() {
                    let shape = alternatives[i].shape.clone();
                    let body = self.branch(ctx, &mut |gn, child| {
                        let read = gn.emit_read(&shape, child)?;
                        push(&child.out, ir::assign(ir::id(value), read));
                        Ok(())
                    })?;
                    chain = ir::if_stmt(ir::binary(tag.clone(), Op::Eq, ir::num(i as f64)), body, Some(else_of(chain)));
                }
                push(&ctx.out, chain);
                if is_fixed_layout && let Some(size) = layout.size {
                    ctx.cursor.borrow_mut().offset = start + size as i64 - 1;
                }
                Ok(ir::id(value))
            }
        }
    }

    fn read_list(&mut self, elements: &[Shape], rest: Option<&Shape>, after: Option<&[Shape]>, ctx: &Ctx) -> GResult<E> {
        let list;
        let count;
        if self.g.compact {
            let built = self.bind(&ctx.out, ir::array(vec![]), "list", Some(Ty::Array));
            for (i, element) in elements.iter().enumerate() {
                self.read_scoped(element, ctx, ir::elem(built.clone(), ir::num(i as f64)))?;
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
            let Some(rest) = rest else { return Ok(ir::cast(ir::array(values), Ty::Array)) };
            let min = self.layout_of(rest)?.min;
            count = self.read_count(ctx, min);
            list = self.bind(&ctx.out, ir::array(values), "list", Some(Ty::Array));
        }
        let rest = rest.unwrap().clone();
        let index = self.uid("i", Ty::Number);
        let range = self.range(ir::num(1.0), count.clone());
        let fixed = elements.len();
        let body = self.branch(ctx, &mut |gn, child| {
            let read = gn.emit_read(&rest, child)?;
            push(&child.out, ir::assign(ir::elem(list.clone(), add(ir::id(index), fixed as f64 - 1.0)), read));
            Ok(())
        })?;
        push(&ctx.out, S::ForOf(Bind::One(index), range, body));
        for (position, shape) in after.unwrap_or(&[]).iter().enumerate() {
            let at = ir::elem(list.clone(), add(count.clone(), (fixed + position) as f64));
            if self.g.compact {
                self.read_scoped(shape, ctx, at)?;
                continue;
            }
            let element = self.read_into(shape, ctx, "arg")?;
            push(&ctx.out, ir::assign(at, element));
        }
        Ok(list)
    }

    /// `readScoped(shape, ctx, target)`.
    fn read_scoped(&mut self, shape: &Shape, ctx: &Ctx, target: E) -> GResult<()> {
        let inner = ctx.with_out(out());
        let read = self.emit_read(shape, &inner)?;
        push(&inner.out, ir::assign(target, read));
        if inner.variable().is_some() {
            self.sync(&inner);
        }
        let statements = take(&inner.out);
        self.push_scoped(&ctx.out, statements);
        Ok(())
    }

    /// `readInto(shape, ctx, hint)`.
    fn read_into(&mut self, shape: &Shape, ctx: &Ctx, hint: &str) -> GResult<E> {
        let value = self.emit_read(shape, ctx)?;
        if ctx.variable().is_none() || value.is_identifier() || value.is_literal() {
            return Ok(value);
        }
        let name = local_name(self.cx, hint);
        Ok(self.bind(&ctx.out, value, &name, None))
    }

    /// `readLength(ctx, width)`.
    fn read_length(&mut self, ctx: &Ctx, width: LengthWidth) -> E {
        let Some(fixed) = width.width() else { return ir::id(self.read_varint(ctx, "length")) };
        let at = self.at(ctx, 0);
        let read = self.buffer_call(&format!("read{}", fixed.name()), vec![ir::id(ctx.buf), at]);
        let length = self.bind(&ctx.out, read, "length", None);
        ctx.cursor.borrow_mut().offset += i64::from(fixed.size());
        length
    }

    /// `readCount(ctx, minimumElementSize)`.
    fn read_count(&mut self, ctx: &Ctx, minimum: u64) -> E {
        let count = self.read_varint(ctx, "count");
        self.sync(ctx);
        if minimum == 0 {
            let tally = self.zero_tally();
            push(&ctx.out, ir::add_assign(ir::id(tally), ir::id(count)));
            let raise = self.raise(MALFORMED);
            push(&ctx.out, ir::if_stmt(ir::binary(ir::id(tally), Op::Gt, ir::num(ZERO_SIZE_COUNT_MAX)), vec![raise], None));
            return ir::id(count);
        }
        let len = self.buffer_call("len", vec![ir::id(ctx.buf)]);
        let remaining = ir::binary(len, Op::Sub, ir::id(ctx.variable().unwrap()));
        let needed = if minimum > 1 { ir::binary(ir::id(count), Op::Mul, ir::num(minimum as f64)) } else { ir::id(count) };
        let raise = self.raise(MALFORMED);
        push(&ctx.out, ir::if_stmt(ir::binary(needed, Op::Gt, remaining), vec![raise], None));
        ir::id(count)
    }
}

/// A versioned serializer's header.
pub(crate) struct Header {
    pub version: u32,
    pub hash: u32,
    pub name: String,
}

/// `else` of an if-chain: an `if` goes on as `else if`, anything else is the last block.
fn else_of(s: S) -> ir::Else {
    match s {
        S::If(..) => ir::Else::If(s),
        S::Block(list) => ir::Else::Block(list),
        other => ir::Else::Block(vec![other]),
    }
}

/// `add(amount, offset)` as `advanceBy` folds it.
fn add_expr_offset(amount: E, offset: i64) -> E {
    add(amount, offset as f64)
}

/// `countedSize(prefix, count, size)`.
fn counted_size(prefix: E, count: E, size: u64) -> E {
    if size == 0 {
        return prefix;
    }
    add_expr(prefix, ir::binary(count, Op::Mul, ir::num(size as f64)))
}

/// `tupleName(place)`.
fn tuple_name(place: &Place) -> String {
    if place.args { "the argument list".into() } else { "the tuple".into() }
}
