#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub span: Span,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>, span: Span) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            span,
        }
    }

    pub fn warning(message: impl Into<String>, span: Span) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            span,
        }
    }
}

pub fn report(source: &str, diagnostics: &[Diagnostic]) {
    for d in diagnostics {
        let prefix = match d.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };

        let before = &source[..d.span.start.min(source.len())];
        let line = before.lines().count().max(1);
        let col = before.lines().last().map(|l| l.len() + 1).unwrap_or(1);

        eprintln!("{}: [{}:{}] {}", prefix, line, col, d.message);
    }
}