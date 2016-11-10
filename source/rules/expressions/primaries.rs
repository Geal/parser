// Tagua VM
//
//
// New BSD License
//
// Copyright © 2016-2016, Ivan Enderlin.
// All rights reserved.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are met:
//     * Redistributions of source code must retain the above copyright
//       notice, this list of conditions and the following disclaimer.
//     * Redistributions in binary form must reproduce the above copyright
//       notice, this list of conditions and the following disclaimer in the
//       documentation and/or other materials provided with the distribution.
//     * Neither the name of the Hoa nor the names of its contributors may be
//       used to endorse or promote products derived from this software without
//       specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
// ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDERS AND CONTRIBUTORS BE
// LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
// CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
// SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
// INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
// CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
// ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
// POSSIBILITY OF SUCH DAMAGE.

//! Group of primary expression rules.
//!
//! The list of all primary expressions is provided by the PHP Language
//! Specification in the [Grammar chapter, Expressions
//! section](https://github.com/php/php-langspec/blob/master/spec/19-grammar.md#primary-expressions).

use std::result::Result as StdResult;
use super::expression;
use super::super::literals::literal;
use super::super::super::internal::fold_into_vector;
use super::super::tokens::{
    qualified_name,
    variable
};
use super::super::super::ast::{
    Expression,
    Literal,
    Name,
    Variable
};
use super::super::super::tokens;

named!(
    pub primary<Expression>,
    alt!(
        variable       => { variable_mapper }
      | qualified_name => { qualified_name_mapper }
      | literal        => { literal_mapper }
      | intrinsic
    )
);

#[inline(always)]
fn variable_mapper<'a>(variable: Variable<'a>) -> Expression<'a> {
    Expression::Variable(variable)
}

#[inline(always)]
fn qualified_name_mapper<'a>(name: Name<'a>) -> Expression<'a> {
    Expression::Name(name)
}

#[inline(always)]
fn literal_mapper<'a>(literal: Literal) -> Expression<'a> {
    Expression::Literal(literal)
}

named!(
    pub intrinsic<Expression>,
    alt!(
        intrinsic_construct
      | intrinsic_operator
    )
);

named!(
    intrinsic_construct<Expression>,
    alt!(
        intrinsic_echo
      | intrinsic_unset
    )
);

named!(
    intrinsic_operator<Expression>,
    call!(intrinsic_empty)
);

named!(
    intrinsic_echo<Expression>,
    chain!(
        accumulator: map_res!(
            preceded!(
                keyword!(tokens::ECHO),
                first!(expression)
            ),
            into_vector_mapper
        ) ~
        result: fold_many0!(
            preceded!(
                first!(tag!(tokens::COMMA)),
                first!(expression)
            ),
            accumulator,
            fold_into_vector
        ),
        || { echo_mapper(result) }
    )
);

#[inline(always)]
fn into_vector_mapper<T>(item: T) -> StdResult<Vec<T>, ()> {
    Ok(vec![item])
}

#[inline(always)]
fn echo_mapper<'a>(expressions: Vec<Expression<'a>>) -> Expression<'a> {
    Expression::Echo(expressions)
}

named!(
    intrinsic_unset<Expression>,
    chain!(
        accumulator: map_res!(
            preceded!(
                keyword!(tokens::UNSET),
                preceded!(
                    first!(tag!(tokens::LEFT_PARENTHESIS)),
                    first!(variable)
                )
            ),
            into_vector_mapper
        ) ~
        result: terminated!(
            fold_many0!(
                preceded!(
                    first!(tag!(tokens::COMMA)),
                    first!(variable)
                ),
                accumulator,
                fold_into_vector
            ),
            first!(tag!(tokens::RIGHT_PARENTHESIS))
        ),
        || { unset_mapper(result) }
    )
);

#[inline(always)]
fn unset_mapper<'a>(variables: Vec<Variable<'a>>) -> Expression<'a> {
    Expression::Unset(variables)
}

named!(
    intrinsic_empty<Expression>,
    map_res!(
        preceded!(
            keyword!(tokens::EMPTY),
            preceded!(
                first!(tag!(tokens::LEFT_PARENTHESIS)),
                terminated!(
                    first!(expression),
                    first!(tag!(tokens::RIGHT_PARENTHESIS))
                )
            )
        ),
        empty_mapper
    )
);

#[inline(always)]
fn empty_mapper<'a>(expression: Expression<'a>) -> StdResult<Expression<'a>, ()> {
    Ok(Expression::Empty(Box::new(expression)))
}


#[cfg(test)]
mod tests {
    use super::{
        intrinsic,
        intrinsic_construct,
        intrinsic_echo,
        intrinsic_empty,
        intrinsic_operator,
        intrinsic_unset,
        primary
    };
    use super::super::expression;
    use super::super::super::super::ast::{
        Expression,
        Literal,
        Name,
        Variable
    };
    use super::super::super::super::internal::{
        Error,
        ErrorKind,
        Result
    };

    #[test]
    fn case_primary_variable() {
        let input  = b"$foo";
        let output = Result::Done(&b""[..], Expression::Variable(Variable(&b"foo"[..])));

        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_primary_qualified_name() {
        let input  = b"Foo\\Bar";
        let output = Result::Done(&b""[..], Expression::Name(Name::Qualified(vec![&b"Foo"[..], &b"Bar"[..]])));

        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_primary_literal() {
        let input  = b"'Hello, World!'";
        let output = Result::Done(&b""[..], Expression::Literal(Literal::String(b"Hello, World!".to_vec())));

        assert_eq!(primary(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_echo_one_expression() {
        let input  = b"echo /* baz */ 'foobar'";
        let output = Result::Done(
            &b""[..],
            Expression::Echo(
                vec![
                    Expression::Literal(Literal::String(b"foobar".to_vec()))
                ]
            )
        );

        assert_eq!(intrinsic_echo(input), output);
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_echo_many_expressions() {
        let input  = b"echo /* baz */ 'foobar',\t $bazqux, \n  42";
        let output = Result::Done(
            &b""[..],
            Expression::Echo(
                vec![
                    Expression::Literal(Literal::String(b"foobar".to_vec())),
                    Expression::Variable(Variable(&b"bazqux"[..])),
                    Expression::Literal(Literal::Integer(42i64))
                ]
            )
        );

        assert_eq!(intrinsic_echo(input), output);
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_invalid_intrinsic_echo_expression_missing() {
        let input  = b"echo;";
        let output = Result::Error(Error::Position(ErrorKind::Alt, &b"echo;"[..]));

        assert_eq!(intrinsic_echo(input), Result::Error(Error::Position(ErrorKind::Alt, &b";"[..])));
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_unset_one_variable() {
        let input  = b"unset($foo)";
        let output = Result::Done(
            &b""[..],
            Expression::Unset(
                vec![
                    Variable(&b"foo"[..])
                ]
            )
        );

        assert_eq!(intrinsic_unset(input), output);
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_unset_many_variables() {
        let input  = b"unset($foo, $bar)";
        let output = Result::Done(
            &b""[..],
            Expression::Unset(
                vec![
                    Variable(&b"foo"[..]),
                    Variable(&b"bar"[..])
                ]
            )
        );

        assert_eq!(intrinsic_unset(input), output);
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_invalid_intrinsic_unset_zero_variable() {
        let input  = b"unset()";
        let output = Result::Error(Error::Position(ErrorKind::Alt, &b"unset()"[..]));

        assert_eq!(intrinsic_unset(input), Result::Error(Error::Position(ErrorKind::Tag, &b")"[..])));
        assert_eq!(intrinsic_construct(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_empty_string() {
        let input  = b"empty('foo')";
        let output = Result::Done(
            &b""[..],
            Expression::Empty(
                Box::new(
                    Expression::Literal(
                        Literal::String(b"foo".to_vec())
                    )
                )
            )
        );

        assert_eq!(intrinsic_empty(input), output);
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_intrinsic_empty_integer() {
        let input  = b"empty(42)";
        let output = Result::Done(
            &b""[..],
            Expression::Empty(
                Box::new(
                    Expression::Literal(
                        Literal::Integer(42i64)
                    )
                )
            )
        );

        assert_eq!(intrinsic_empty(input), output);
        assert_eq!(intrinsic_operator(input), output);
        assert_eq!(intrinsic(input), output);
        assert_eq!(expression(input), output);
    }

    #[test]
    fn case_invalid_intrinsic_empty_expression_missing() {
        let input    = b"empty()";
        let output_a = Result::Error(Error::Position(ErrorKind::Alt, &b")"[..]));
        let output_b = Result::Error(Error::Position(ErrorKind::Alt, &b"empty()"[..]));

        assert_eq!(intrinsic_empty(input), output_a);
        assert_eq!(intrinsic_operator(input), output_a);
        assert_eq!(intrinsic(input), output_b);
        assert_eq!(expression(input), output_b);
    }
}
