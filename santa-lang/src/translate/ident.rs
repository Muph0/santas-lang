use std::{collections::HashMap, sync::Arc};

use crate::translate::loc::SourceStr;

/// Wrapper around a identifier hashmap that produces correct located errors
pub struct Identifiers {
    data: HashMap<SourceStr, usize>,
}
impl Identifiers {
    pub fn new() -> Self {
        Self {
            data: Default::default(),
        }
    }

    #[must_use]
    pub fn define(&mut self, ident: &SourceStr, value: usize) -> Result<(), super::Error> {
        let conflict = self.data.get_key_value(ident);
        match conflict {
            None => {
                self.data.insert(ident.clone(), value);
                Ok(())
            }
            Some((existing, _)) => Err(super::Error {
                source_name: ident.source_name.clone(),
                loc: Some(ident.loc.clone()),
                code: super::ECode::IdentifierConflict(existing.clone()),
            }),
        }
    }

    #[must_use]
    pub fn get(&self, ident: &SourceStr) -> Result<usize, super::Error> {
        let found = self.data.get(ident);
        match found {
            Some(some) => Ok(*some),
            None => Err(super::Error {
                source_name: ident.source_name.clone(),
                loc: Some(ident.loc.clone()),
                code: super::ECode::UnknownIdentifier(ident.string.clone()),
            }),
        }
    }
}
