use std::iter::FromIterator;

use codespan::Span;
use nom::bytes::complete::take;
use nom::sequence::{preceded, terminated};
use nom::InputLength;

use super::{tokens, IResult};
use crate::error::{Error, Errors};
use crate::lexer::Tokens;
use crate::ToSpan;

#[derive(Clone, Debug, PartialEq)]
pub struct Partial<T> {
    value: Option<T>,
    errors: Errors,
}

impl<T> Partial<T> {
    /// Constructs a new `Partial<T>` with the given initial value.
    #[inline]
    pub fn new(value: Option<T>) -> Self {
        Partial {
            value,
            errors: Errors::new(),
        }
    }

    /// Constructs a new `Partial<T>` with the given initial value and a stack of errors.
    #[inline]
    pub fn with_errors(value: Option<T>, errors: Errors) -> Self {
        Partial { value, errors }
    }

    /// Returns whether this partial value contains errors.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Returns the errors associated with the partial value, if any.
    #[inline]
    pub fn errors(&self) -> Option<Errors> {
        if self.has_errors() {
            Some(self.errors.clone())
        } else {
            None
        }
    }

    /// Appends the given error to the error stack contained in this partial value.
    pub fn extend_errors<I: IntoIterator<Item = Error>>(&mut self, error: I) {
        self.errors.extend(error);
    }

    /// Returns the contained partial value, if any.
    #[inline]
    pub fn value(&self) -> Option<&T> {
        self.value.as_ref()
    }

    /// Maps a `Partial<T>` to `Partial<U>` by applying a function to a contained value.
    ///
    /// This transformation is applied regardless of whether this `Partial<T>` contains errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use nix_parser::error::Errors;
    /// # use nix_parser::parser::Partial;
    /// # fn main() -> Result<(), Errors> {
    /// let partial_string = Partial::from(String::from("Hello, world!"));
    /// let partial_len = partial_string.map(|s| s.len());
    /// // We assert here that the contained partial value has no errors.
    /// let full_len = partial_len.verify()?;
    ///
    /// assert_eq!(full_len, 13);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn map<U, F>(self, f: F) -> Partial<U>
    where
        F: FnOnce(T) -> U,
    {
        Partial {
            value: self.value.map(f),
            errors: self.errors,
        }
    }

    /// Calls `f` if there exists a contained value, otherwise returns the stored errors instead.
    ///
    /// Any errors produced by `f` are appended to the errors already inside `self`.
    #[inline]
    pub fn flat_map<U, F>(mut self, f: F) -> Partial<U>
    where
        F: FnOnce(T) -> Partial<U>,
    {
        if let Some(value) = self.value {
            let mut partial = f(value);
            self.errors.extend(partial.errors);
            partial.errors = self.errors;
            partial
        } else {
            Partial::with_errors(None, self.errors)
        }
    }

    pub fn map_err<F>(self, f: F) -> Partial<T>
    where
        F: FnOnce(Errors) -> Errors,
    {
        let errors = if self.has_errors() {
            f(self.errors)
        } else {
            self.errors
        };

        Partial {
            value: self.value,
            errors,
        }
    }

    /// Transforms the `Partial<T>` into a `Result<T, VerboseError<LocatedSpan>>`, asserting that
    /// the contained value exists and has no errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use nix_parser::parser::Partial;
    /// let partial = Partial::new(Some(123));
    /// assert_eq!(Ok(123), partial.verify());
    ///
    /// let partial: Partial<u32> = Partial::new(None);
    /// assert!(partial.verify().is_err());
    /// ```
    #[inline]
    pub fn verify(self) -> Result<T, Errors> {
        match self.value {
            Some(_) if self.has_errors() => Err(self.errors),
            Some(value) => Ok(value),
            None => Err(self.errors),
        }
    }
}

/// Extend the contents of a `Partial<Vec<T>>` from an iterator of `Partial<T>`.
impl<T> Extend<Partial<T>> for Partial<Vec<T>> {
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = Partial<T>>,
    {
        let iter = iter.into_iter();

        if let (Some(values), (_, Some(bound))) = (self.value.as_mut(), iter.size_hint()) {
            let additional = bound.saturating_sub(values.len());
            values.reserve(additional);
        }

        for partial in iter {
            if let Some(errors) = partial.errors() {
                self.extend_errors(errors);
            }

            if let (Some(values), Some(value)) = (self.value.as_mut(), partial.value) {
                values.push(value);
            }
        }
    }
}

impl<T> From<T> for Partial<T> {
    fn from(value: T) -> Self {
        Partial::new(Some(value))
    }
}

impl<T> From<Option<T>> for Partial<T> {
    fn from(value: Option<T>) -> Self {
        Partial::new(value)
    }
}

/// Collect an iterator of `Partial<T>` into a `Partial<Vec<T>>`.
impl<T> FromIterator<Partial<T>> for Partial<Vec<T>> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = Partial<T>>,
    {
        let iter = iter.into_iter();

        let (_, capacity) = iter.size_hint();
        let mut values = Vec::with_capacity(capacity.unwrap_or(0));
        let mut partials = Partial::new(None);

        for partial in iter {
            if let Some(errors) = partial.errors() {
                partials.extend_errors(errors);
            }

            if let Some(value) = partial.value {
                values.push(value);
            }
        }

        partials.value = Some(values);
        partials
    }
}

/// Combinator which runs the given partial parser and then expects on a terminator.
///
/// If the terminator is missing, an unclosed delimiter error will be appended to the `Partial`,
/// and parsing will be allowed to continue as though the terminator existed.
pub fn expect_terminated<'a, O1, O2, F, G>(
    f: F,
    term: G,
) -> impl Fn(Tokens<'a>) -> IResult<Partial<O1>>
where
    F: Fn(Tokens<'a>) -> IResult<Partial<O1>>,
    G: Fn(Tokens<'a>) -> IResult<O2>,
{
    move |input| match terminated(&f, &term)(input) {
        Ok((remaining, partial)) => Ok((remaining, partial)),
        Err(nom::Err::Error(err)) => {
            let (remaining, mut partial) = f(input)?;
            partial.extend_errors(err);
            Ok((remaining, partial))
        }
        Err(err) => Err(err),
    }
}

/// Combinator which behaves like `nom::combinator::map()`, except it is a shorthand for:
///
/// ```rust,ignore
/// map(partial, |partial| partial.map(&f))
/// ```
pub fn map_partial<'a, O1, O2, P, F>(
    partial: P,
    f: F,
) -> impl Fn(Tokens<'a>) -> IResult<Partial<O2>>
where
    P: Fn(Tokens<'a>) -> IResult<Partial<O1>>,
    F: Fn(O1) -> O2,
{
    move |input| {
        let (input, partial) = partial(input)?;
        Ok((input, partial.map(&f)))
    }
}

/// Combinator which combines the functionality of `map_partial()` and `map_spanned()`.
///
/// This is like `map_partial()` except it also includes a `Span` based on the consumed input.
pub fn map_partial_spanned<'a, O1, O2, P, F>(
    partial: P,
    f: F,
) -> impl Fn(Tokens<'a>) -> IResult<Partial<O2>>
where
    P: Fn(Tokens<'a>) -> IResult<Partial<O1>>,
    F: Fn(Span, O1) -> O2,
{
    move |input| {
        let (remainder, partial) = partial(input)?;
        let span = if remainder.input_len() > 0 {
            Span::new(input.to_span().start(), remainder.to_span().start())
        } else {
            input.to_span()
        };
        Ok((remainder, partial.map(|p| f(span, p))))
    }
}

/// Combinator which applies the partial parser `f` until the parser `g` produces a result,
/// returning a `Partial<Vec<_>>` of the results of `f`.
///
/// If the terminator is missing, an unclosed delimiter error will be appended to the `Partial`,
/// and parsing will be allowed to continue as through the terminator existed.
pub fn many_till_partial<'a, O1, O2, F, G>(
    f: F,
    g: G,
) -> impl Fn(Tokens<'a>) -> IResult<Partial<Vec<O1>>>
where
    F: Fn(Tokens<'a>) -> IResult<Partial<O1>>,
    G: Fn(Tokens<'a>) -> IResult<O2>,
{
    move |input| {
        let mut partials = Vec::new();
        let mut errors = Errors::new();
        let mut input = input;

        loop {
            match g(input) {
                Ok(_) => {
                    let mut partial: Partial<_> = partials.into_iter().collect();
                    partial.extend_errors(errors);
                    return Ok((input, partial));
                }
                Err(nom::Err::Failure(_)) | Err(nom::Err::Error(_)) => match f(input) {
                    Err(nom::Err::Failure(err)) | Err(nom::Err::Error(err)) => {
                        if tokens::eof(input).is_ok() {
                            let partial: Partial<_> = partials.into_iter().collect();
                            return Ok((input, partial));
                        } else if let Ok((remainder, _)) = take::<_, _, Errors>(1usize)(input) {
                            errors.extend(err);
                            input = remainder;
                        }
                    }
                    Err(err) => return Err(err),
                    Ok((remainder, elem)) => {
                        partials.push(elem);
                        input = remainder;
                    }
                },
                Err(err) => return Err(err),
            }
        }
    }
}

/// Combinator which gets the result from the first partial parser, then gets the result from the
/// second partial parser, and produces a partial value containing a tuple of the two results.
///
/// This is effectively shorthand for:
///
/// ```rust,ignore
/// map(pair(first, second), |(f, g)| f.flat_map(|f| g.map(|g| (f, g))))
/// ```
pub fn pair_partial<'a, O1, O2, F, G>(
    first: F,
    second: G,
) -> impl Fn(Tokens<'a>) -> IResult<Partial<(O1, O2)>>
where
    F: Fn(Tokens<'a>) -> IResult<Partial<O1>>,
    G: Fn(Tokens<'a>) -> IResult<Partial<O2>>,
{
    move |input| {
        let (input, f) = first(input)?;
        let (remaining, g) = second(input)?;
        Ok((remaining, f.flat_map(|f| g.map(|g| (f, g)))))
    }
}

/// Combinator which produces a list of partial elements `f` separated by parser `sep`.
///
/// This parser behaves like `nom::multi::separated_list`, except that it expects some terminator
/// `term` at the end of the list so it knows when to soft-bail.
///
/// If the terminator is missing, an unclosed delimiter error will be appended to the `Partial`,
/// and parsing will be allowed to continue as through the terminator existed.
///
/// This parser is essentially shorthand for:
///
/// ```rust,ignore
/// let (remaining, (first, rest)) = pair(&f, many_till_partial(preceded(sep, &f), term))(input)?;
/// let partial = first.flat_map(|f| rest.map(|r| std::iter::once(f).chain(r).collect()));
/// ```
pub fn separated_list_partial<'a, O1, O2, O3, F, G, H>(
    sep: G,
    term: H,
    f: F,
) -> impl Fn(Tokens<'a>) -> IResult<Partial<Vec<O1>>>
where
    F: Fn(Tokens<'a>) -> IResult<Partial<O1>>,
    G: Fn(Tokens<'a>) -> IResult<O2>,
    H: Fn(Tokens<'a>) -> IResult<O3>,
{
    move |input| {
        let mut partials = Vec::new();
        let mut errors = Errors::new();
        let mut input = input;

        match f(input) {
            Err(nom::Err::Error(_)) => return Ok((input, partials.into_iter().collect())),
            Err(nom::Err::Failure(err)) => {
                return Err(nom::Err::Error(err));
            }
            Err(err) => return Err(err),
            Ok((remaining, partial)) => {
                input = remaining;
                partials.push(partial);
            }
        }

        loop {
            match term(input) {
                Ok(_) => {
                    let mut partial: Partial<_> = partials.into_iter().collect();
                    partial.extend_errors(errors);
                    return Ok((input, partial));
                }
                Err(nom::Err::Failure(_)) | Err(nom::Err::Error(_)) => {
                    match preceded(&sep, &f)(input) {
                        Err(nom::Err::Failure(err)) | Err(nom::Err::Error(err)) => {
                            if tokens::eof(input).is_ok() {
                                let partial: Partial<_> = partials.into_iter().collect();
                                return Ok((input, partial));
                            } else if let Ok((remainder, _)) = take::<_, _, Errors>(1usize)(input) {
                                errors.extend(err);
                                input = remainder;
                            }
                        }
                        Err(err) => return Err(err),
                        Ok((remainder, elem)) => {
                            partials.push(elem);
                            input = remainder;
                        }
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }
}

/// Combinator which asserts that a given partial parser produces a value and contains no errors.
pub fn verify_full<'a, O, F>(f: F) -> impl Fn(Tokens<'a>) -> IResult<O>
where
    F: Fn(Tokens<'a>) -> IResult<Partial<O>>,
{
    move |input| {
        let (input, partial) = f(input)?;
        partial
            .verify()
            .map(move |value| (input, value))
            .map_err(nom::Err::Error)
    }
}
