use std::fmt::{Display, Formatter, Result as FmtResult};

use codespan::Span;

use self::tokens::{Comment, Ident, Literal};
use crate::HasSpan;

pub mod tokens;

mod macros;

/// A source file with a top-level doc comment.
#[derive(Clone, Debug, PartialEq)]
pub struct SourceFile {
    comment: Option<Comment>,
    expr: Box<Expr>,
}

impl SourceFile {
    pub fn new(comment: Option<Comment>, expr: Box<Expr>) -> Self {
        SourceFile { comment, expr }
    }

    pub fn comment(&self) -> Option<&Comment> {
        self.comment.as_ref()
    }

    pub fn expr(&self) -> &Expr {
        &self.expr
    }
}

impl Display for SourceFile {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        if let Some(ref comment) = &self.comment {
            write!(fmt, "{}\n{}", comment, self.expr)
        } else {
            write!(fmt, "{}", self.expr)
        }
    }
}

impl HasSpan for SourceFile {
    fn span(&self) -> Span {
        let first = self.comment.as_ref().map(|c| c.span()).unwrap_or_default();
        let second = self.expr.span();
        Span::merge(first, second)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    /// A parenthesized expression.
    ///
    /// This type of expression is only used to aid with serialization of the AST back into a token
    /// string.
    Paren(ExprParen),
    /// `foo`
    Ident(Ident),
    /// `${foo}`
    Interpolation(ExprInterpolation),
    /// `12`, `4.0`, `false`, `"foo"`, `''bar''`, `./foo/bar`, `null`, `http://www.example.com`
    Literal(Literal),
    /// `[1 2 3 4]`
    List(ExprList),
    /// `"foo"`, `''bar''`, `"baz ${quux}"`
    String(ExprString),
    /// `{ foo = "hello"; bar = 123; }`
    Set(ExprSet),

    /// `-12`
    /// `!15.0`
    Unary(ExprUnary),
    /// `1 + 1`, `true && false`, `"foo" + hello + "bar"`, `"foo ${hello} bar"`
    Binary(ExprBinary),

    /// `let { foo = "bar"; }`
    Let(ExprLet),
    /// `rec { foo = "bar"; }`
    Rec(ExprRec),
    /// `x.y`
    Proj(ExprProj),

    /// `if true then "success" else "failure"`
    If(ExprIf),
    /// `foo.bar or "failed"`
    Or(ExprOr),
    /// `assert true != false; true`
    Assert(ExprAssert),
    /// `with foo; foo.attr`
    With(ExprWith),

    /// `let foo = "bar"; in foo`
    LetIn(ExprLetIn),
    /// `foo: 1 + 2`, `{ x, y }: x + y`, `{ x, y } @ foo: x + y`
    FnDecl(ExprFnDecl),
    /// `foo one`
    FnApp(ExprFnApp),

    /// An invalid unparseable expression.
    Error(Span),
    /// Trap for halting the parser in place.
    Trap(Span),
}

impl Display for Expr {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        match *self {
            Expr::Paren(ref e) => write!(fmt, "{}", e),
            Expr::Ident(ref e) => write!(fmt, "{}", e),
            Expr::Interpolation(ref e) => write!(fmt, "{}", e),
            Expr::Literal(ref e) => write!(fmt, "{}", e),
            Expr::List(ref e) => write!(fmt, "{}", e),
            Expr::String(ref e) => write!(fmt, "{}", e),
            Expr::Set(ref e) => write!(fmt, "{}", e),

            Expr::Unary(ref e) => write!(fmt, "{}", e),
            Expr::Binary(ref e) => write!(fmt, "{}", e),

            Expr::Let(ref e) => write!(fmt, "{}", e),
            Expr::Rec(ref e) => write!(fmt, "{}", e),
            Expr::Proj(ref e) => write!(fmt, "{}", e),

            Expr::If(ref e) => write!(fmt, "{}", e),
            Expr::Or(ref e) => write!(fmt, "{}", e),
            Expr::Assert(ref e) => write!(fmt, "{}", e),
            Expr::With(ref e) => write!(fmt, "{}", e),

            Expr::LetIn(ref e) => write!(fmt, "{}", e),
            Expr::FnDecl(ref e) => write!(fmt, "{}", e),
            Expr::FnApp(ref e) => write!(fmt, "{}", e),

            Expr::Error(_) => write!(fmt, "<error>"),
            Expr::Trap(_) => write!(fmt, "trap"),
        }
    }
}

impl HasSpan for Expr {
    fn span(&self) -> Span {
        match *self {
            Expr::Paren(ref e) => e.span(),
            Expr::Ident(ref e) => e.span(),
            Expr::Interpolation(ref e) => e.span(),
            Expr::Literal(ref e) => e.span(),
            Expr::List(ref e) => e.span(),
            Expr::String(ref e) => e.span(),
            Expr::Set(ref e) => e.span(),

            Expr::Unary(ref e) => e.span(),
            Expr::Binary(ref e) => e.span(),

            Expr::Let(ref e) => e.span(),
            Expr::Rec(ref e) => e.span(),
            Expr::Proj(ref e) => e.span(),

            Expr::If(ref e) => e.span(),
            Expr::Or(ref e) => e.span(),
            Expr::Assert(ref e) => e.span(),
            Expr::With(ref e) => e.span(),

            Expr::LetIn(ref e) => e.span(),
            Expr::FnDecl(ref e) => e.span(),
            Expr::FnApp(ref e) => e.span(),

            Expr::Error(ref e) => *e,
            Expr::Trap(ref e) => *e,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ExprParen {
    expr: Box<Expr>,
    span: Span,
}

impl ExprParen {
    pub fn new(expr: Box<Expr>, span: Span) -> Self {
        ExprParen { expr, span }
    }

    pub fn expr(&self) -> &Expr {
        &self.expr
    }
}

impl Display for ExprParen {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        write!(fmt, "({})", self.expr)
    }
}

impl HasSpan for ExprParen {
    fn span(&self) -> Span {
        self.span
    }
}

impl PartialEq for ExprParen {
    fn eq(&self, other: &Self) -> bool {
        self.expr == other.expr
    }
}

#[derive(Clone, Debug)]
pub struct ExprInterpolation {
    inner: Box<Expr>,
    span: Span,
}

impl ExprInterpolation {
    pub fn new(inner: Box<Expr>, span: Span) -> Self {
        ExprInterpolation { inner, span }
    }

    pub fn inner(&self) -> &Expr {
        &self.inner
    }
}

impl Display for ExprInterpolation {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        write!(fmt, "${{{}}}", self.inner)
    }
}

impl HasSpan for ExprInterpolation {
    fn span(&self) -> Span {
        self.span
    }
}

impl PartialEq for ExprInterpolation {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

#[derive(Clone, Debug)]
pub struct ExprList {
    elems: Vec<Expr>,
    span: Span,
}

impl ExprList {
    pub fn new(elems: Vec<Expr>, span: Span) -> Self {
        ExprList { elems, span }
    }

    pub fn elems(&self) -> &[Expr] {
        &self.elems[..]
    }
}

impl Display for ExprList {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        let elems: Vec<_> = self.elems.iter().map(ToString::to_string).collect();
        write!(fmt, "[{}]", elems.join(" "))
    }
}

impl HasSpan for ExprList {
    fn span(&self) -> Span {
        self.span
    }
}

impl PartialEq for ExprList {
    fn eq(&self, other: &Self) -> bool {
        self.elems == other.elems
    }
}

#[derive(Clone, Debug)]
pub struct ExprSet {
    binds: Vec<Bind>,
    span: Span,
}

impl ExprSet {
    pub fn new(binds: Vec<Bind>, span: Span) -> Self {
        ExprSet { binds, span }
    }

    pub fn binds(&self) -> &[Bind] {
        &self.binds[..]
    }
}

impl Display for ExprSet {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        let binds: Vec<_> = self.binds.iter().map(ToString::to_string).collect();
        write!(fmt, "{{{}}}", binds.join(" "))
    }
}

impl HasSpan for ExprSet {
    fn span(&self) -> Span {
        self.span
    }
}

impl PartialEq for ExprSet {
    fn eq(&self, other: &Self) -> bool {
        self.binds == other.binds
    }
}

#[derive(Clone, Debug)]
pub struct ExprString(Vec<StringFragment>, Span);

impl ExprString {
    pub fn new(fragments: Vec<StringFragment>, span: Span) -> Self {
        ExprString(fragments, span)
    }
}

impl Display for ExprString {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        // FIXME: Should record whether this string is a single or multi string so we can properly
        // escape the string here.
        let segments: Vec<_> = self.0.iter().map(ToString::to_string).collect();
        write!(fmt, "\"{}\"", segments.concat())
    }
}

impl HasSpan for ExprString {
    fn span(&self) -> Span {
        self.1
    }
}

impl PartialEq for ExprString {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[derive(Clone, Debug)]
pub enum StringFragment {
    Literal(String, Span),
    Interpolation(ExprInterpolation),
}

impl Display for StringFragment {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        match *self {
            StringFragment::Literal(ref text, _) => write!(fmt, "{}", text),
            StringFragment::Interpolation(ref expr) => write!(fmt, "{}", expr),
        }
    }
}

impl HasSpan for StringFragment {
    fn span(&self) -> Span {
        match *self {
            StringFragment::Literal(_, ref span) => *span,
            StringFragment::Interpolation(ref expr) => expr.span(),
        }
    }
}

impl PartialEq for StringFragment {
    fn eq(&self, other: &Self) -> bool {
        use StringFragment::*;
        match (self, other) {
            (Literal(ref lhs, _), Literal(ref rhs, _)) => lhs == rhs,
            (Interpolation(ref lhs), Interpolation(ref rhs)) => lhs == rhs,
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UnaryOp {
    /// The unary `-` operator.
    Neg,
    /// The unary `!` operator.
    Not,
}

impl Display for UnaryOp {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        match *self {
            UnaryOp::Neg => fmt.write_str("-"),
            UnaryOp::Not => fmt.write_str("!"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ExprUnary {
    op: UnaryOp,
    expr: Box<Expr>,
    span: Span,
}

impl ExprUnary {
    pub fn new(op: UnaryOp, expr: Box<Expr>, span: Span) -> Self {
        ExprUnary { op, expr, span }
    }

    pub fn op(&self) -> UnaryOp {
        self.op
    }

    pub fn expr(&self) -> &Expr {
        &self.expr
    }
}

impl Display for ExprUnary {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        write!(fmt, "{}{}", self.op, self.expr)
    }
}

impl HasSpan for ExprUnary {
    fn span(&self) -> Span {
        self.span
    }
}

impl PartialEq for ExprUnary {
    fn eq(&self, other: &Self) -> bool {
        self.op == other.op && self.expr == other.expr
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BinaryOp {
    /// The binary `+` operator.
    Add,
    /// The binary `-` operator.
    Sub,
    /// The binary `*` operator.
    Mul,
    /// The binary `/` operator.
    Div,
    /// The binary `==` operator.
    Eq,
    /// The binary `!=` operator.
    NotEq,
    /// The binary `<` operator.
    LessThan,
    /// The binary `<=` operator.
    LessThanEq,
    /// The binary `>` operator.
    GreaterThan,
    /// The binary `>=` operator.
    GreaterThanEq,
    /// The binary `&&` operator.
    And,
    /// The binary `||` operator.
    Or,
    /// The binary `++` operator.
    Concat,
    /// The binary `//` operator.
    Update,
    /// The binary `?` operator.
    HasAttr,
    /// The binary `->` operator.
    Impl,
}

impl Display for BinaryOp {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        match *self {
            BinaryOp::Add => fmt.write_str("+"),
            BinaryOp::Sub => fmt.write_str("-"),
            BinaryOp::Mul => fmt.write_str("*"),
            BinaryOp::Div => fmt.write_str("/"),
            BinaryOp::Eq => fmt.write_str("=="),
            BinaryOp::NotEq => fmt.write_str("!="),
            BinaryOp::LessThan => fmt.write_str("<"),
            BinaryOp::LessThanEq => fmt.write_str("<="),
            BinaryOp::GreaterThan => fmt.write_str(">"),
            BinaryOp::GreaterThanEq => fmt.write_str(">="),
            BinaryOp::And => fmt.write_str("&&"),
            BinaryOp::Or => fmt.write_str("||"),
            BinaryOp::Concat => fmt.write_str("++"),
            BinaryOp::Update => fmt.write_str("//"),
            BinaryOp::HasAttr => fmt.write_str("?"),
            BinaryOp::Impl => fmt.write_str("->"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ExprBinary {
    op: BinaryOp,
    lhs: Box<Expr>,
    rhs: Box<Expr>,
    span: Span,
}

impl ExprBinary {
    pub fn new(op: BinaryOp, lhs: Box<Expr>, rhs: Box<Expr>, span: Span) -> Self {
        ExprBinary { op, lhs, rhs, span }
    }

    pub fn op(&self) -> BinaryOp {
        self.op
    }

    pub fn left(&self) -> &Expr {
        &self.lhs
    }

    pub fn right(&self) -> &Expr {
        &self.rhs
    }
}

impl Display for ExprBinary {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        write!(fmt, "{} {} {}", self.lhs, self.op, self.rhs)
    }
}

impl HasSpan for ExprBinary {
    fn span(&self) -> Span {
        self.span
    }
}

impl PartialEq for ExprBinary {
    fn eq(&self, other: &Self) -> bool {
        self.op == other.op && self.lhs == other.lhs && self.rhs == other.rhs
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Bind {
    Simple(BindSimple),
    Inherit(BindInherit),
    InheritExpr(BindInheritExpr),
}

impl Display for Bind {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        match *self {
            Bind::Simple(ref b) => write!(fmt, "{}", b),
            Bind::Inherit(ref b) => write!(fmt, "{}", b),
            Bind::InheritExpr(ref b) => write!(fmt, "{}", b),
        }
    }
}

impl HasSpan for Bind {
    fn span(&self) -> Span {
        match *self {
            Bind::Simple(ref b) => b.span(),
            Bind::Inherit(ref b) => b.span(),
            Bind::InheritExpr(ref b) => b.span(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct BindSimple {
    comment: Option<Comment>,
    attr: AttrPath,
    expr: Box<Expr>,
    span: Span,
}

impl BindSimple {
    pub fn new(comment: Option<Comment>, attr: AttrPath, expr: Box<Expr>, span: Span) -> Self {
        BindSimple {
            comment,
            attr,
            expr,
            span,
        }
    }

    pub fn comment(&self) -> Option<&Comment> {
        self.comment.as_ref()
    }

    pub fn attr(&self) -> &AttrPath {
        &self.attr
    }

    pub fn expr(&self) -> &Expr {
        &self.expr
    }
}

impl Display for BindSimple {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        if let Some(ref comment) = self.comment {
            write!(fmt, "{}{} = {};", comment, self.attr, self.expr)
        } else {
            write!(fmt, "{} = {};", self.attr, self.expr)
        }
    }
}

impl HasSpan for BindSimple {
    fn span(&self) -> Span {
        self.span
    }
}

impl PartialEq for BindSimple {
    fn eq(&self, other: &Self) -> bool {
        self.attr == other.attr && self.expr == other.expr && self.comment == other.comment
    }
}

#[derive(Clone, Debug)]
pub struct BindInherit {
    names: Vec<Ident>,
    span: Span,
}

impl BindInherit {
    pub fn new(names: Vec<Ident>, span: Span) -> Self {
        BindInherit { names, span }
    }

    pub fn names(&self) -> &[Ident] {
        &self.names[..]
    }
}

impl Display for BindInherit {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        let names: Vec<_> = self.names.iter().map(ToString::to_string).collect();
        write!(fmt, "inherit {};", names.join(" "))
    }
}

impl HasSpan for BindInherit {
    fn span(&self) -> Span {
        self.span
    }
}

impl PartialEq for BindInherit {
    fn eq(&self, other: &Self) -> bool {
        self.names == other.names
    }
}

#[derive(Clone, Debug)]
pub struct BindInheritExpr {
    expr: Box<Expr>,
    names: Vec<Ident>,
    span: Span,
}

impl BindInheritExpr {
    pub fn new(expr: Box<Expr>, names: Vec<Ident>, span: Span) -> Self {
        BindInheritExpr { expr, names, span }
    }

    pub fn expr(&self) -> &Expr {
        &self.expr
    }

    pub fn names(&self) -> &[Ident] {
        &self.names[..]
    }
}

impl Display for BindInheritExpr {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        let names: Vec<_> = self.names.iter().map(ToString::to_string).collect();
        write!(fmt, "inherit ({}) {};", self.expr, names.join(" "))
    }
}

impl HasSpan for BindInheritExpr {
    fn span(&self) -> Span {
        self.span
    }
}

impl PartialEq for BindInheritExpr {
    fn eq(&self, other: &Self) -> bool {
        self.expr == other.expr && self.names == other.names
    }
}

#[derive(Clone, Debug)]
pub struct ExprLet {
    binds: Vec<Bind>,
    span: Span,
}

impl ExprLet {
    pub fn new(binds: Vec<Bind>, span: Span) -> Self {
        ExprLet { binds, span }
    }

    pub fn binds(&self) -> &[Bind] {
        &self.binds[..]
    }
}

impl Display for ExprLet {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        let binds: Vec<_> = self.binds.iter().map(ToString::to_string).collect();
        write!(fmt, "let {{{}}}", binds.join(" "))
    }
}

impl HasSpan for ExprLet {
    fn span(&self) -> Span {
        self.span
    }
}

impl PartialEq for ExprLet {
    fn eq(&self, other: &Self) -> bool {
        self.binds == other.binds
    }
}

#[derive(Clone, Debug)]
pub struct ExprRec {
    binds: Vec<Bind>,
    span: Span,
}

impl ExprRec {
    pub fn new(binds: Vec<Bind>, span: Span) -> Self {
        ExprRec { binds, span }
    }

    pub fn binds(&self) -> &[Bind] {
        &self.binds[..]
    }
}

impl Display for ExprRec {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        let binds: Vec<_> = self.binds.iter().map(ToString::to_string).collect();
        write!(fmt, "rec {{{}}}", binds.join(" "))
    }
}

impl HasSpan for ExprRec {
    fn span(&self) -> Span {
        self.span
    }
}

impl PartialEq for ExprRec {
    fn eq(&self, other: &Self) -> bool {
        self.binds == other.binds
    }
}

#[derive(Clone, Debug)]
pub struct AttrPath(Vec<AttrSegment>, Span);

impl AttrPath {
    pub fn new(segments: Vec<AttrSegment>) -> Self {
        let span = segments
            .first()
            .map(|s| s.span())
            .and_then(|first| segments.last().map(|s| (first, s.span())))
            .map(|(first, second)| Span::merge(first, second))
            .unwrap_or_default();

        AttrPath(segments, span)
    }
}

impl Display for AttrPath {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        let segments: Vec<_> = self.0.iter().map(ToString::to_string).collect();
        write!(fmt, "{}", segments.join("."))
    }
}

impl HasSpan for AttrPath {
    fn span(&self) -> Span {
        self.1
    }
}

impl PartialEq for AttrPath {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[derive(Clone, Debug)]
pub enum AttrSegment {
    Ident(Ident),
    Interpolation(ExprInterpolation),
    String(ExprString),
}

impl Display for AttrSegment {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        match *self {
            AttrSegment::Ident(ref ident) => write!(fmt, "{}", ident),
            AttrSegment::Interpolation(ref expr) => write!(fmt, "{}", expr),
            AttrSegment::String(ref expr) => write!(fmt, "{}", expr),
        }
    }
}

impl HasSpan for AttrSegment {
    fn span(&self) -> Span {
        match *self {
            AttrSegment::Ident(ref ident) => ident.span(),
            AttrSegment::Interpolation(ref expr) => expr.span(),
            AttrSegment::String(ref expr) => expr.span(),
        }
    }
}

impl PartialEq for AttrSegment {
    fn eq(&self, other: &Self) -> bool {
        use AttrSegment::*;
        match (self, other) {
            (Ident(ref lhs), Ident(ref rhs)) => lhs == rhs,
            (Interpolation(ref lhs), Interpolation(ref rhs)) => lhs == rhs,
            (String(ref lhs), String(ref rhs)) => lhs == rhs,
            _ => false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ExprProj {
    base: Box<Expr>,
    attr: AttrPath,
    fallback: Option<Box<Expr>>,
    span: Span,
}

impl ExprProj {
    pub fn new(base: Box<Expr>, attr: AttrPath, fallback: Option<Box<Expr>>, span: Span) -> Self {
        ExprProj {
            base,
            attr,
            fallback,
            span,
        }
    }

    pub fn base(&self) -> &Expr {
        &self.base
    }

    pub fn attr(&self) -> &AttrPath {
        &self.attr
    }

    pub fn fallback(&self) -> Option<&Expr> {
        self.fallback.as_ref().map(|e| e.as_ref())
    }
}

impl Display for ExprProj {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        if let Some(ref val) = self.fallback.as_ref() {
            write!(fmt, "{}.{} or {}", self.base, self.attr, val)
        } else {
            write!(fmt, "{}.{}", self.base, self.attr)
        }
    }
}

impl HasSpan for ExprProj {
    fn span(&self) -> Span {
        self.span
    }
}

impl PartialEq for ExprProj {
    fn eq(&self, other: &Self) -> bool {
        self.base == other.base && self.attr == other.attr
    }
}

#[derive(Clone, Debug)]
pub struct ExprIf {
    cond: Box<Expr>,
    body: Box<Expr>,
    fallback: Box<Expr>,
    span: Span,
}

impl ExprIf {
    pub fn new(cond: Box<Expr>, body: Box<Expr>, fallback: Box<Expr>, span: Span) -> Self {
        ExprIf {
            cond,
            body,
            fallback,
            span,
        }
    }

    pub fn condition(&self) -> &Expr {
        &self.cond
    }

    pub fn body(&self) -> &Expr {
        &self.body
    }

    pub fn fallback(&self) -> &Expr {
        &self.fallback
    }
}

impl Display for ExprIf {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        write!(
            fmt,
            "if {} then {} else {}",
            self.cond, self.body, self.fallback
        )
    }
}

impl HasSpan for ExprIf {
    fn span(&self) -> Span {
        self.span
    }
}

impl PartialEq for ExprIf {
    fn eq(&self, other: &Self) -> bool {
        self.cond == other.cond && self.body == other.body && self.fallback == other.fallback
    }
}

#[derive(Clone, Debug)]
pub struct ExprOr {
    expr: Box<Expr>,
    fallback: Box<Expr>,
    span: Span,
}

impl ExprOr {
    pub fn new(expr: Box<Expr>, fallback: Box<Expr>, span: Span) -> Self {
        ExprOr {
            expr,
            fallback,
            span,
        }
    }

    pub fn expr(&self) -> &Expr {
        &self.expr
    }

    pub fn fallback(&self) -> &Expr {
        &self.fallback
    }
}

impl Display for ExprOr {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        write!(fmt, "{} or {}", self.expr, self.fallback)
    }
}

impl HasSpan for ExprOr {
    fn span(&self) -> Span {
        self.span
    }
}

impl PartialEq for ExprOr {
    fn eq(&self, other: &Self) -> bool {
        self.expr == other.expr && self.fallback == other.fallback
    }
}

#[derive(Clone, Debug)]
pub struct ExprAssert {
    cond: Box<Expr>,
    expr: Box<Expr>,
    span: Span,
}

impl ExprAssert {
    pub fn new(cond: Box<Expr>, expr: Box<Expr>, span: Span) -> Self {
        ExprAssert { cond, expr, span }
    }

    pub fn condition(&self) -> &Expr {
        &self.cond
    }

    pub fn expr(&self) -> &Expr {
        &self.expr
    }
}

impl Display for ExprAssert {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        write!(fmt, "assert {}; {}", self.cond, self.expr)
    }
}

impl HasSpan for ExprAssert {
    fn span(&self) -> Span {
        self.span
    }
}

impl PartialEq for ExprAssert {
    fn eq(&self, other: &Self) -> bool {
        self.cond == other.cond && self.expr == other.expr
    }
}

#[derive(Clone, Debug)]
pub struct ExprWith {
    with: Box<Expr>,
    expr: Box<Expr>,
    span: Span,
}

impl ExprWith {
    pub fn new(with: Box<Expr>, expr: Box<Expr>, span: Span) -> Self {
        ExprWith { with, expr, span }
    }

    pub fn with(&self) -> &Expr {
        &self.with
    }

    pub fn expr(&self) -> &Expr {
        &self.expr
    }
}

impl Display for ExprWith {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        write!(fmt, "with {}; {}", self.with, self.expr)
    }
}

impl HasSpan for ExprWith {
    fn span(&self) -> Span {
        self.span
    }
}

impl PartialEq for ExprWith {
    fn eq(&self, other: &Self) -> bool {
        self.with == other.with && self.expr == other.expr
    }
}

#[derive(Clone, Debug)]
pub struct ExprLetIn {
    binds: Vec<Bind>,
    body: Box<Expr>,
    span: Span,
}

impl ExprLetIn {
    pub fn new(binds: Vec<Bind>, body: Box<Expr>, span: Span) -> Self {
        ExprLetIn { binds, body, span }
    }

    pub fn binds(&self) -> &[Bind] {
        &self.binds[..]
    }

    pub fn body(&self) -> &Expr {
        &self.body
    }
}

impl Display for ExprLetIn {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        let binds: Vec<_> = self.binds.iter().map(ToString::to_string).collect();
        write!(fmt, "let {} in {}", binds.join(" "), self.body)
    }
}

impl HasSpan for ExprLetIn {
    fn span(&self) -> Span {
        self.span
    }
}

impl PartialEq for ExprLetIn {
    fn eq(&self, other: &Self) -> bool {
        self.binds == other.binds && self.body == other.body
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExprFnDecl {
    Simple(FnDeclSimple),
    Formals(FnDeclFormals),
}

impl Display for ExprFnDecl {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        match *self {
            ExprFnDecl::Simple(ref s) => write!(fmt, "{}", s),
            ExprFnDecl::Formals(ref f) => write!(fmt, "{}", f),
        }
    }
}

impl HasSpan for ExprFnDecl {
    fn span(&self) -> Span {
        match *self {
            ExprFnDecl::Simple(ref d) => d.span(),
            ExprFnDecl::Formals(ref d) => d.span(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FnDeclSimple {
    name: Ident,
    body: Box<Expr>,
    span: Span,
}

impl FnDeclSimple {
    pub fn new(name: Ident, body: Box<Expr>, span: Span) -> Self {
        FnDeclSimple { name, body, span }
    }

    pub fn name(&self) -> &Ident {
        &self.name
    }

    pub fn body(&self) -> &Expr {
        &self.body
    }
}

impl Display for FnDeclSimple {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        write!(fmt, "{}: {}", self.name, self.body)
    }
}

impl HasSpan for FnDeclSimple {
    fn span(&self) -> Span {
        self.span
    }
}

impl PartialEq for FnDeclSimple {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.body == other.body
    }
}

#[derive(Clone, Debug)]
pub struct Formal {
    name: Ident,
    default: Option<Box<Expr>>,
    span: Span,
}

impl Formal {
    pub fn new(name: Ident, default: Option<Box<Expr>>, span: Span) -> Self {
        Formal {
            name,
            default,
            span,
        }
    }

    pub fn name(&self) -> &Ident {
        &self.name
    }

    pub fn default(&self) -> Option<&Expr> {
        self.default.as_ref().map(|e| e.as_ref())
    }
}

impl Display for Formal {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        let default = self
            .default
            .as_ref()
            .map(|e| format!(" ? {}", e))
            .unwrap_or_default();
        write!(fmt, "{}{}", self.name, default)
    }
}

impl HasSpan for Formal {
    fn span(&self) -> Span {
        self.span
    }
}

impl PartialEq for Formal {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.default == other.default
    }
}

#[derive(Clone, Debug)]
pub struct FnDeclFormals {
    formals: Vec<Formal>,
    ellipsis: Option<Span>,
    extra: Option<Ident>,
    body: Box<Expr>,
    span: Span,
}

impl FnDeclFormals {
    pub fn new(
        formals: Vec<Formal>,
        ellipsis: Option<Span>,
        extra: Option<Ident>,
        body: Box<Expr>,
        span: Span,
    ) -> Self {
        FnDeclFormals {
            formals,
            ellipsis,
            extra,
            body,
            span,
        }
    }
}

impl Display for FnDeclFormals {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        let args: Vec<_> = self
            .formals
            .iter()
            .map(ToString::to_string)
            .chain(self.ellipsis.as_ref().map(|_| "...".to_string()))
            .collect();
        let extra = self
            .extra
            .as_ref()
            .map(|ident| format!("@{}", ident))
            .unwrap_or_default();
        write!(fmt, "{{{}}}{}:", args.join(", "), extra)
    }
}

impl HasSpan for FnDeclFormals {
    fn span(&self) -> Span {
        self.span
    }
}

impl PartialEq for FnDeclFormals {
    fn eq(&self, other: &Self) -> bool {
        self.formals == other.formals
            && (self.ellipsis.is_some() && other.ellipsis.is_some())
            && self.body == other.body
    }
}

#[derive(Clone, Debug)]
pub struct ExprFnApp {
    function: Box<Expr>,
    argument: Box<Expr>,
    span: Span,
}

impl ExprFnApp {
    pub fn new(function: Box<Expr>, argument: Box<Expr>, span: Span) -> Self {
        ExprFnApp {
            function,
            argument,
            span,
        }
    }

    pub fn function(&self) -> &Expr {
        &self.function
    }

    pub fn argument(&self) -> &Expr {
        &self.argument
    }
}

impl Display for ExprFnApp {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        write!(fmt, "{} {}", self.function, self.argument)
    }
}

impl HasSpan for ExprFnApp {
    fn span(&self) -> Span {
        self.span
    }
}

impl PartialEq for ExprFnApp {
    fn eq(&self, other: &Self) -> bool {
        self.function == other.function && self.argument == other.argument
    }
}
