//! This module takes care of translating parsed files to a runtime repr

use std::hash::Hash;

use crate::{Runtime, parse::TranslationUnit};

pub fn translate<S: Clone + Hash + Eq>(unit: TranslationUnit<S>) -> Runtime {
    todo!("translation")
}
