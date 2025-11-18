mod grammar;

use std::{borrow::Cow, collections::HashMap, hash::Hash, sync::Arc};

use crate::{Int, runtime};

/// The `TranslationUnit` is generic over a string type `S`,
/// allowing flexibility in how text is represented (e.g. `&str`, `String`, `Cow<'_, str>`).
/// Parsing may borrow strings from input, but you can later call `convert()`
/// to transform all `S` values into another representation (such as fully owned `String`).
#[derive(Debug, Default, Clone)]
pub struct TranslationUnit<S: Hash + Clone> {
    pub name: String,
    pub workshops: HashMap<S, Shop<S>>,
    pub todos: Vec<ToDo<S>>,
}

#[derive(Debug, Clone)]
pub struct Shop<S> {
    pub name: S,
    pub block: ShopBlock<S>,
}

#[derive(Debug, Clone)]
pub enum ShopBlock<S> {
    Plan {
        width: usize,
        height: usize,
        map: Vec<Tile<S>>,
    },
    Program(Vec<runtime::Instr>),
}

type Indent = (char, usize);

#[derive(Debug, Clone)]
pub struct PlanRow<S> {
    pub indent: Indent,
    pub tiles: Vec<Tile<S>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tile<S> {
    Empty,
    Move(Direction),
    Elf(Direction),
    Question,
    Instr(runtime::Instr),
    Unknown(S),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToDo<S> {
    SendElf { name: Option<S>, stack: Vec<Int> },
}

impl<S: Hash + Clone> TranslationUnit<S> {
    pub fn convert<R: Hash + Clone + Eq>(self, f: impl Fn(S) -> R) -> TranslationUnit<R> {
        TranslationUnit {
            name: self.name.clone(),
            workshops: self
                .workshops
                .into_iter()
                .map(|(k, v)| (f(k), v.convert(&f)))
                .collect(),
            todos: self.todos.into_iter().map(|t| t.convert(&f)).collect(),
        }
    }
}
impl<S> Shop<S> {
    pub fn convert<R>(self, f: impl Fn(S) -> R) -> Shop<R> {
        Shop {
            name: f(self.name),
            block: self.block.convert(f),
        }
    }
}
impl<S> ShopBlock<S> {
    pub fn convert<R>(self, f: impl Fn(S) -> R) -> ShopBlock<R> {
        match self {
            ShopBlock::Plan { width, height, map } => ShopBlock::Plan {
                width,
                height,
                map: map.into_iter().map(|t| t.convert(&f)).collect(),
            },
            ShopBlock::Program(instrs) => ShopBlock::Program(instrs),
        }
    }
}
impl<S> Tile<S> {
    pub fn convert<R>(self, f: impl Fn(S) -> R) -> Tile<R> {
        match self {
            Tile::Empty => Tile::Empty,
            Tile::Move(d) => Tile::Move(d),
            Tile::Elf(d) => Tile::Elf(d),
            Tile::Question => Tile::Question,
            Tile::Instr(i) => Tile::Instr(i),
            Tile::Unknown(s) => Tile::Unknown(f(s)),
        }
    }
}
impl<S> ToDo<S> {
    pub fn convert<R>(self, f: impl Fn(S) -> R) -> ToDo<R> {
        match self {
            ToDo::SendElf { name, stack } => ToDo::SendElf {
                name: name.map(f),
                stack,
            },
        }
    }
}

#[test]
fn demonstrate_convert() {
    let unit = {
        let src = String::from("shop_name,elf_name");
        let names: Vec<&str> = src.split(",").collect(); // represents parsed names

        let unit = TranslationUnit {
            name: "name".to_string(),
            workshops: HashMap::from([(
                names[0],
                Shop {
                    name: names[0],
                    block: ShopBlock::Program(vec![]),
                },
            )]),
            todos: vec![ToDo::SendElf {
                name: Some(names[1]),
                stack: vec![],
            }],
        };

        // src.clear(); // fails, src is borrowed by unit

        unit.convert::<Arc<str>>(|s| Arc::from(s))
    };

    println!("{unit:?}");
}
