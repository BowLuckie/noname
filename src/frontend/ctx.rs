use crate::frontend::{Diagnostic, Interner, Literals};

use super::Severity;

#[derive(Default, Debug)]
pub struct Ctx {
    pub interner: Interner,
    pub literals: Literals,
    pub diags: Vec<Diagnostic>,
}

impl Ctx {
    pub fn has_error(&self) -> bool {
        for diag in &self.diags {
            if let Severity::Error = diag.severity {
                return true;
            }
        }

        false
    }

    pub fn error_count(&self) -> usize {
        self.diags
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count()
    }
}
