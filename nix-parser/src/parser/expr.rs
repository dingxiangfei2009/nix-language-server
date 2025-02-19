use std::iter::{self, FromIterator};

use codespan::Span;
use nom::branch::alt;
use nom::bytes::complete::take;
use nom::combinator::{map, opt};
use nom::multi::many0;
use nom::sequence::{pair, preceded};

use super::partial::{
    expect_terminated, map_partial, map_partial_spanned, pair_partial, verify_full, Partial,
};
use super::{tokens, IResult};
use crate::ast::{BinaryOp, Expr, ExprBinary, ExprFnApp, ExprIf, ExprProj, ExprUnary, UnaryOp};
use crate::error::{Errors, UnexpectedError};
use crate::lexer::{Token, Tokens};
use crate::{HasSpan, ToSpan};

mod atomic;
mod attr;
mod bind;
mod func;
mod stmt;
mod util;

pub fn expr(input: Tokens) -> IResult<Partial<Expr>> {
    preceded(many0(tokens::comment), function)(input)
}

fn function(input: Tokens) -> IResult<Partial<Expr>> {
    let function = map_partial(func::fn_decl, Expr::from);
    let with = map_partial(stmt::with, Expr::from);
    let assert = map_partial(stmt::assert, Expr::from);
    let let_in = map_partial(stmt::let_in, Expr::from);
    alt((function, with, assert, let_in, if_else))(input)
}

fn if_else(input: Tokens) -> IResult<Partial<Expr>> {
    let found = "keyword `then`";
    let cond = alt((util::error_expr_if(tokens::keyword_then, found), expr));
    let cond_then = expect_terminated(cond, tokens::keyword_then);
    let if_cond_then = preceded(tokens::keyword_if, cond_then);

    let found = "keyword `else`";
    let body = alt((util::error_expr_if(tokens::keyword_else, found), expr));
    let body_else = expect_terminated(body, tokens::keyword_else);

    let expr = alt((util::error_expr_if(tokens::eof, "<eof>"), expr));
    let block = pair_partial(if_cond_then, pair_partial(body_else, expr));
    let if_else = map_partial_spanned(block, |span, (cond, (body, fallback))| {
        Expr::If(Box::new(ExprIf::new(cond, body, fallback, span)))
    });

    alt((if_else, imply))(input)
}

fn imply(input: Tokens) -> IResult<Partial<Expr>> {
    let expr = pair(and, many0(preceded(tokens::op_imply, and)));
    map(expr, |(first, rest)| {
        rest.into_iter().fold(first, |lhs, rhs| {
            lhs.flat_map(|lhs| {
                rhs.map(|rhs| {
                    let span = Span::merge(lhs.span(), rhs.span());
                    Expr::Binary(Box::new(ExprBinary::new(BinaryOp::Impl, lhs, rhs, span)))
                })
            })
        })
    })(input)
}

fn and(input: Tokens) -> IResult<Partial<Expr>> {
    let expr = pair(or, many0(preceded(tokens::op_and, or)));
    map(expr, |(first, rest)| {
        rest.into_iter().fold(first, |lhs, rhs| {
            lhs.flat_map(|lhs| {
                rhs.map(|rhs| {
                    let span = Span::merge(lhs.span(), rhs.span());
                    Expr::Binary(Box::new(ExprBinary::new(BinaryOp::And, lhs, rhs, span)))
                })
            })
        })
    })(input)
}

fn or(input: Tokens) -> IResult<Partial<Expr>> {
    let expr = pair(equality, many0(preceded(tokens::op_or, equality)));
    map(expr, |(first, rest)| {
        rest.into_iter().fold(first, |lhs, rhs| {
            lhs.flat_map(|lhs| {
                rhs.map(|rhs| {
                    let span = Span::merge(lhs.span(), rhs.span());
                    Expr::Binary(Box::new(ExprBinary::new(BinaryOp::Or, lhs, rhs, span)))
                })
            })
        })
    })(input)
}

fn equality(input: Tokens) -> IResult<Partial<Expr>> {
    let eq = map(tokens::op_eq, |_| BinaryOp::Eq);
    let neq = map(tokens::op_neq, |_| BinaryOp::NotEq);
    let expr = pair(compare, opt(pair(alt((eq, neq)), compare)));
    map(expr, |(lhs, op)| match op {
        None => lhs,
        Some((op, rhs)) => lhs.flat_map(|lhs| {
            rhs.map(|rhs| {
                let span = Span::merge(lhs.span(), rhs.span());
                Expr::Binary(Box::new(ExprBinary::new(op, lhs, rhs, span)))
            })
        }),
    })(input)
}

fn compare(input: Tokens) -> IResult<Partial<Expr>> {
    let lte = map(tokens::op_lte, |_| BinaryOp::LessThanEq);
    let lt = map(tokens::op_lt, |_| BinaryOp::LessThan);
    let gte = map(tokens::op_gte, |_| BinaryOp::GreaterThanEq);
    let gt = map(tokens::op_gt, |_| BinaryOp::GreaterThan);
    let expr = pair(update, opt(pair(alt((lte, lt, gte, gt)), update)));
    map(expr, |(lhs, op)| match op {
        None => lhs,
        Some((op, rhs)) => lhs.flat_map(|lhs| {
            rhs.map(|rhs| {
                let span = Span::merge(lhs.span(), rhs.span());
                Expr::Binary(Box::new(ExprBinary::new(op, lhs, rhs, span)))
            })
        }),
    })(input)
}

fn update(input: Tokens) -> IResult<Partial<Expr>> {
    let expr = pair(sum, many0(preceded(tokens::op_update, sum)));
    map(expr, |(first, rest)| {
        let exprs = Partial::from_iter(iter::once(first).chain(rest));
        exprs.map(|mut exprs| {
            let last = exprs.pop().unwrap();
            exprs.into_iter().rev().fold(last, |lhs, rhs| {
                let span = Span::merge(rhs.span(), lhs.span());
                Expr::Binary(Box::new(ExprBinary::new(BinaryOp::Update, lhs, rhs, span)))
            })
        })
    })(input)
}

fn sum(input: Tokens) -> IResult<Partial<Expr>> {
    let add = map(tokens::op_add, |_| BinaryOp::Add);
    let sub = map(tokens::op_sub, |_| BinaryOp::Sub);
    let expr = pair(product, many0(pair(alt((add, sub)), product)));
    map(expr, |(first, rest)| {
        rest.into_iter().fold(first, |lhs, (op, rhs)| {
            lhs.flat_map(|lhs| {
                rhs.map(|rhs| {
                    let span = Span::merge(lhs.span(), rhs.span());
                    Expr::Binary(Box::new(ExprBinary::new(op, lhs, rhs, span)))
                })
            })
        })
    })(input)
}

fn product(input: Tokens) -> IResult<Partial<Expr>> {
    let mul = map(tokens::op_mul, |_| BinaryOp::Mul);
    let div = map(tokens::op_div, |_| BinaryOp::Div);
    let expr = pair(concat, many0(pair(alt((mul, div)), concat)));
    map(expr, |(first, rest)| {
        rest.into_iter().fold(first, |lhs, (op, rhs)| {
            lhs.flat_map(|lhs| {
                rhs.map(|rhs| {
                    let span = Span::merge(lhs.span(), rhs.span());
                    Expr::Binary(Box::new(ExprBinary::new(op, lhs, rhs, span)))
                })
            })
        })
    })(input)
}

fn concat(input: Tokens) -> IResult<Partial<Expr>> {
    let expr = pair(unary, many0(preceded(tokens::op_concat, unary)));
    map(expr, |(first, rest)| {
        let exprs = Partial::from_iter(iter::once(first).chain(rest));
        exprs.map(|mut exprs| {
            let last = exprs.pop().unwrap();
            exprs.into_iter().rev().fold(last, |lhs, rhs| {
                let span = Span::merge(rhs.span(), lhs.span());
                Expr::Binary(Box::new(ExprBinary::new(BinaryOp::Concat, lhs, rhs, span)))
            })
        })
    })(input)
}

fn unary(input: Tokens) -> IResult<Partial<Expr>> {
    let neg = map(tokens::op_sub, |_| UnaryOp::Neg);
    let not = map(tokens::op_not, |_| UnaryOp::Not);
    let unary = pair_partial(map(opt(alt((neg, not))), Partial::from), fn_app);
    let expr = map_partial_spanned(unary, |span, (unary, expr)| match unary {
        Some(op) => Expr::Unary(Box::new(ExprUnary::new(op, expr, span))),
        None => expr,
    });
    alt((expr, error))(input)
}

fn fn_app(input: Tokens) -> IResult<Partial<Expr>> {
    map(pair(project, many0(project)), |(first, rest)| {
        rest.into_iter().fold(first, |lhs, rhs| {
            lhs.flat_map(|lhs| {
                rhs.map(|rhs| {
                    let span = Span::merge(lhs.span(), rhs.span());
                    Expr::FnApp(Box::new(ExprFnApp::new(lhs, rhs, span)))
                })
            })
        })
    })(input)
}

fn project(input: Tokens) -> IResult<Partial<Expr>> {
    let path = preceded(tokens::dot, verify_full(attr::attr_path));
    let expr = pair(atomic, opt(path));
    map(expr, |(base, path)| match path {
        None => base,
        Some(path) => base.map(|base| {
            let span = Span::merge(base.span(), path.span());
            Expr::Proj(Box::new(ExprProj::new(base, path, None, span)))
        }),
    })(input)
}

fn atomic(input: Tokens) -> IResult<Partial<Expr>> {
    let paren = map_partial(atomic::paren, Expr::from);
    let inter = map_partial(atomic::interpolation, Expr::from);
    let set = map_partial(atomic::set, Expr::from);
    let rec_set = map_partial(atomic::rec_set, Expr::from);
    let let_set = map_partial(atomic::let_set, Expr::from);
    let list = map_partial(atomic::list, Expr::from);
    let string = map_partial(atomic::string, Expr::from);
    let literal = map_partial(atomic::literal, Expr::from);
    let ident = map_partial(atomic::identifier, Expr::from);
    alt((
        ident, paren, set, list, string, inter, literal, rec_set, let_set,
    ))(input)
}

fn error(input: Tokens) -> IResult<Partial<Expr>> {
    let (remaining, tokens) = take(1usize)(input)?;
    let mut errors = Errors::new();
    errors.push(UnexpectedError::new(
        tokens.current().description(),
        tokens.to_span(),
    ));
    if let Token::Eof(_) = tokens.current() {
        Err(nom::Err::Error(errors))
    } else {
        let error = Expr::Error(tokens.to_span());
        Ok((remaining, Partial::with_errors(Some(error), errors)))
    }
}
