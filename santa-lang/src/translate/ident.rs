use std::{collections::HashMap, sync::Arc};

use crate::translate::loc::SourceStr;

pub struct Identifiers {
    data: HashMap<Arc<str>, usize>,
}
impl Identifiers {
    pub fn define(ident: &SourceStr, value: usize) -> Result<(), super::Error> {
        todo!()
    }

    pub fn resolve(ident: &SourceStr) -> Result<usize, super::Error> {
        todo!()
    }
}
