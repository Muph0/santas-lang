mod grammar;

use std::{borrow::Cow, collections::HashMap};

use crate::runtime;

#[derive(Debug, Clone)]
pub struct TranslationUnit<'i> {
    pub name: &'i str,
    pub workshops: HashMap<&'i str, Shop<'i>>,
}

#[derive(Debug, Clone)]
pub struct Shop<'i> {
    pub name: &'i str,
    pub block: ShopBlock<'i>,
}

#[derive(Debug, Clone)]
pub enum ShopBlock<'i> {
    Plan {
        width: usize,
        height: usize,
        map: Vec<Tile<'i>>,
    },
    Program(Vec<runtime::Instr>),
}

type Indent = (char, usize);

#[derive(Debug, Clone)]
pub struct PlanRow<'i> {
    pub indent: Indent,
    pub tiles: Vec<Tile<'i>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tile<'i> {
    Empty,
    Instr(runtime::Instr),
    Unknown(&'i str),
}
