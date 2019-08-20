use std::iter::FromIterator;

use codespan::ByteSpan;
use nom::branch::alt;
use nom::bytes::complete::{tag, take_until};
use nom::character::complete::anychar;
use nom::combinator::{all_consuming, map, recognize};
use nom::error::{ErrorKind, VerboseError, VerboseErrorKind};
use nom::multi::{many0, many1, many_till};
use nom::sequence::{pair, terminated};
use nom::Slice;

use super::{IResult, Span};
use crate::ToByteSpan;

#[derive(Clone, Debug, PartialEq)]
pub struct Partial<'a, T> {
    value: Option<T>,
    errors: VerboseError<Span<'a>>,
}

impl<'a, T> Partial<'a, T> {
    /// Constructs a new `Partial<T>` with the given initial value.
    pub fn new(value: Option<T>) -> Self {
        Partial {
            value,
            errors: VerboseError { errors: Vec::new() },
        }
    }

    /// Constructs a new `Partial<T>` with the given initial value and a stack of errors.
    pub fn with_errors(value: Option<T>, errors: VerboseError<Span<'a>>) -> Self {
        Partial { value, errors }
    }

    /// Returns whether this partial value contains errors.
    pub fn has_errors(&self) -> bool {
        !self.errors.errors.is_empty()
    }

    /// Returns the errors associated with the partial value, if any.
    pub fn errors(&self) -> Option<VerboseError<Span<'a>>> {
        if self.has_errors() {
            Some(self.errors.clone())
        } else {
            None
        }
    }

    /// Appends the given error to the error stack contained in this partial value.
    pub fn extend_errors(&mut self, error: VerboseError<Span<'a>>) {
        self.errors.errors.extend(error.errors);
    }

    /// Returns the contained partial value, if any.
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
    /// # use nix_parser::parser::Partial;
    /// # use nom::error::VerboseError;
    /// # use nom_locate::LocatedSpan;
    /// # fn main() -> Result<(), VerboseError<LocatedSpan<&'static str>>> {
    /// let partial_string = Partial::from(String::from("Hello, world!"));
    /// let partial_len = partial_string.map(|s| s.len());
    /// // We assert here that the contained partial value has no errors.
    /// let full_len = partial_len.verify()?;
    ///
    /// assert_eq!(full_len, 13);
    /// # Ok(())
    /// # }
    /// ```
    pub fn map<U, F>(self, f: F) -> Partial<'a, U>
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
    pub fn flat_map<U, F>(mut self, f: F) -> Partial<'a, U>
    where
        F: FnOnce(T) -> Partial<'a, U>,
    {
        if let Some(value) = self.value {
            let mut partial = f(value);
            self.errors.errors.extend(partial.errors.errors);
            partial.errors = self.errors;
            partial
        } else {
            Partial::with_errors(None, self.errors)
        }
    }

    pub fn map_err<F>(self, f: F) -> Partial<'a, T>
    where
        F: FnOnce(VerboseError<Span<'a>>) -> VerboseError<Span<'a>>,
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

    /// Transforms the `Partial<T>` into a `Result<T, VerboseError<Span>>`, asserting that the
    /// contained value exists and has no errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use nix_parser::parser::Partial;
    /// # use nom::error::VerboseError;
    /// # use nom_locate::LocatedSpan;
    /// # fn main() -> Result<(), VerboseError<LocatedSpan<&'static str>>> {
    /// let partial = Partial::new(Some(123));
    /// assert_eq!(Ok(123), partial.verify());
    ///
    /// let partial: Partial<u32> = Partial::new(None);
    /// assert!(partial.verify().is_err());
    /// # Ok(())
    /// # }
    /// ```
    pub fn verify(self) -> Result<T, VerboseError<Span<'a>>> {
        match self.value {
            Some(_) if self.has_errors() => Err(self.errors),
            Some(value) => Ok(value),
            None => Err(self.errors),
        }
    }
}

/// Extend the contents of a `Partial<Vec<T>>` from an iterator of `Partial<T>`.
impl<'a, T> Extend<Partial<'a, T>> for Partial<'a, Vec<T>> {
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = Partial<'a, T>>,
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

impl<'a, T> From<T> for Partial<'a, T> {
    fn from(value: T) -> Self {
        Partial::new(Some(value))
    }
}

impl<'a, T> From<Option<T>> for Partial<'a, T> {
    fn from(value: Option<T>) -> Self {
        Partial::new(value)
    }
}

/// Collect an iterator of `Partial<T>` into a `Partial<Vec<T>>`.
impl<'a, T> FromIterator<Partial<'a, T>> for Partial<'a, Vec<T>> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = Partial<'a, T>>,
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

pub fn expect_terminated<'a, O1, O2, F, G>(
    f: F,
    term: G,
) -> impl Fn(Span<'a>) -> IResult<Partial<O1>>
where
    F: Fn(Span<'a>) -> IResult<Partial<O1>>,
    G: Fn(Span<'a>) -> IResult<O2>,
{
    move |input| match terminated(&f, &term)(input) {
        Ok((remaining, partial)) => Ok((remaining, partial)),
        Err(nom::Err::Error(e)) => {
            let (remaining, mut partial) = f(input)?;
            partial.extend_errors(e);
            Ok((remaining, partial))
        }
        Err(e) => Err(e),
    }
}

pub fn map_partial<'a, O1, O2, P, F>(partial: P, f: F) -> impl Fn(Span<'a>) -> IResult<Partial<O2>>
where
    P: Fn(Span<'a>) -> IResult<Partial<O1>>,
    F: Fn(O1) -> O2,
{
    move |input| {
        let (input, partial) = partial(input)?;
        Ok((input, partial.map(&f)))
    }
}

pub fn map_partial_spanned<'a, O1, O2, P, F>(
    input: Span<'a>,
    partial: P,
    f: F,
) -> impl Fn(Span<'a>) -> IResult<Partial<O2>>
where
    P: Fn(Span<'a>) -> IResult<Partial<O1>>,
    F: Fn(ByteSpan, O1) -> O2,
{
    move |input| {
        let (remainder, partial) = partial(input)?;
        let partial_len = remainder.offset - input.offset;
        let span = input.slice(..partial_len).to_byte_span();
        Ok((remainder, partial.map(|p| f(span, p))))
    }
}

pub fn map_err_partial<'a, O1, O2, F, G>(
    partial: F,
    skip_to: G,
    error: ErrorKind,
) -> impl Fn(Span<'a>) -> IResult<Partial<O1>>
where
    F: Fn(Span<'a>) -> IResult<Partial<O1>>,
    G: Fn(Span<'a>) -> IResult<O2>,
{
    move |input| match partial(input) {
        Ok((remaining, value)) => Ok((remaining, value)),
        Err(nom::Err::Failure(e)) | Err(nom::Err::Error(e)) => {
            let (remaining, failed) = recognize(many_till(anychar, &skip_to))(input)?;
            let mut partial = Partial::with_errors(None, e);
            partial.extend_errors(VerboseError {
                errors: vec![(failed, VerboseErrorKind::Nom(error))],
            });
            Ok((remaining, partial))
        }
        Err(e) => Err(e),
    }
}

pub fn verify_full<'a, O, F>(f: F) -> impl Fn(Span<'a>) -> IResult<O>
where
    F: Fn(Span<'a>) -> IResult<Partial<O>>,
{
    move |input| {
        let (input, partial) = f(input)?;
        partial
            .verify()
            .map(move |value| (input, value))
            .map_err(nom::Err::Failure)
    }
}
