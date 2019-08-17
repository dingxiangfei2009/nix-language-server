use nom::branch::alt;
use nom::character::complete::{char, multispace0};
use nom::combinator::{cut, map, opt};
use nom::multi::many1;
use nom::sequence::{delimited, pair, preceded, separated_pair, terminated};

use super::expr;
use crate::ast::{Bind, BindInherit, BindInheritExpr, BindSimple};
use crate::parser::partial::verify_full;
use crate::parser::{tokens, IResult, Span};
use crate::ToByteSpan;

pub fn bind(input: Span) -> IResult<Bind> {
    let inherit = map(inherit, Bind::Inherit);
    let inherit_expr = map(inherit_expr, Bind::InheritExpr);
    let simple = map(simple, Bind::Simple);
    alt((inherit, inherit_expr, simple))(input)
}

fn simple(input: Span) -> IResult<BindSimple> {
    let comment = opt(terminated(tokens::comment, multispace0));

    let lhs = terminated(tokens::ident_path, tokens::space);
    let equals = pair(cut(char('=')), tokens::space);
    let rhs = terminated(map(verify_full(expr), Box::new), cut(char(';')));
    let bind = pair(comment, separated_pair(lhs, equals, rhs));

    map(bind, |(comment, (name, expr))| {
        BindSimple::new(comment, name, expr, input.to_byte_span())
    })(input)
}

fn inherit(input: Span) -> IResult<BindInherit> {
    let comment = opt(terminated(tokens::comment, multispace0));
    let key_inherit = pair(tokens::keyword_inherit, tokens::space);
    let name = terminated(tokens::identifier, tokens::space);
    let bind = preceded(comment, delimited(key_inherit, many1(name), cut(char(';'))));
    map(bind, |names| BindInherit::new(names, input.to_byte_span()))(input)
}

fn inherit_expr(input: Span) -> IResult<BindInheritExpr> {
    let comment = opt(terminated(tokens::comment, multispace0));
    let key_inherit = pair(tokens::keyword_inherit, tokens::space);

    let open_paren = pair(char('('), tokens::space);
    let close_paren = pair(cut(char(')')), tokens::space);
    let verified = verify_full(expr);
    let expr = map(delimited(open_paren, verified, close_paren), Box::new);

    let name = terminated(tokens::identifier, tokens::space);
    let inherit = delimited(key_inherit, pair(expr, many1(name)), cut(char(';')));
    let bind = preceded(comment, inherit);

    map(bind, |(expr, names)| {
        BindInheritExpr::new(expr, names, input.to_byte_span())
    })(input)
}

#[cfg(test)]
mod tests {
    use nom::combinator::all_consuming;

    use super::*;
    use crate::ast::tokens::{Comment, Ident, IdentPath, Literal};
    use crate::ast::Expr;
    use crate::ToByteSpan;

    #[test]
    fn simple_binds() {
        let string = Span::new("foo.bar = true;");
        let (_, uncommented) = all_consuming(simple)(string).unwrap();
        assert_eq!(
            uncommented,
            BindSimple::new(
                None,
                IdentPath::from(vec!["foo", "bar"]),
                Box::new(Expr::Literal(Literal::from(true))),
                Span::new("").to_byte_span(),
            )
        );

        let string = Span::new("# hello world \n #this is a   doc comment   \n  foo.bar = true;");
        let (_, commented) = all_consuming(simple)(string).unwrap();
        assert_eq!(
            commented,
            BindSimple::new(
                Some(Comment::from(" hello world \nthis is a   doc comment   ")),
                IdentPath::from(vec!["foo", "bar"]),
                Box::new(Expr::Literal(Literal::from(true))),
                Span::new("").to_byte_span(),
            )
        );
    }
}
