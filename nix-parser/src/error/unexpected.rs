use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};

use codespan::{FileId, Span};
use codespan_reporting::diagnostic::{Diagnostic, Label};

use super::ToDiagnostic;
use crate::ToSpan;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnexpectedError {
    pub token: String,
    pub span: Span,
}

impl UnexpectedError {
    pub fn new<T, S>(token: T, span: S) -> Self
    where
        T: Into<String>,
        S: ToSpan,
    {
        UnexpectedError {
            token: token.into(),
            span: span.to_span(),
        }
    }
}

impl Display for UnexpectedError {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        write!(fmt, "unexpected token: {}", self.token)
    }
}

impl Error for UnexpectedError {}

impl ToDiagnostic for UnexpectedError {
    fn to_diagnostic(&self, file: FileId) -> Diagnostic {
        let label = Label::new(file, self.span, "found unexpected token here");
        Diagnostic::new_error(self.to_string(), label)
    }
}
