use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};

use codespan::{FileId, Span};
use codespan_reporting::diagnostic::{Diagnostic, Label};

use super::ToDiagnostic;
use crate::ToSpan;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IncorrectDelimError {
    pub unmatched_delim: (char, Span),
    pub candidate_span: Option<Span>,
    pub unclosed_span: Option<Span>,
}

impl IncorrectDelimError {
    pub fn new<S>(delim: char, span: S, candidate: Option<S>, unclosed: Option<S>) -> Self
    where
        S: ToSpan,
    {
        IncorrectDelimError {
            unmatched_delim: (delim, span.to_span()),
            candidate_span: candidate.map(|s| s.to_span()),
            unclosed_span: unclosed.map(|s| s.to_span()),
        }
    }
}

impl Display for IncorrectDelimError {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        write!(
            fmt,
            "incorrect close delimiter: `{}`",
            self.unmatched_delim.0
        )
    }
}

impl Error for IncorrectDelimError {}

impl ToDiagnostic for IncorrectDelimError {
    fn to_diagnostic(&self, file: FileId) -> Diagnostic {
        let primary = Label::new(file, self.unmatched_delim.1, "incorrect close delimiter");
        let mut diagnostic = Diagnostic::new_error(self.to_string(), primary);

        if let Some(span) = self.candidate_span {
            let candidate = Label::new(file, span, "close delimiter possibly meant for this");
            diagnostic.secondary_labels.push(candidate);
        }

        if let Some(span) = self.unclosed_span {
            let unclosed = Label::new(file, span, "unmatched delimiter");
            diagnostic.secondary_labels.push(unclosed);
        }

        diagnostic
    }
}
