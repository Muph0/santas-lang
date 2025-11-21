//! This mod contains structs modelling the contents of a parsed file

use std::{collections::HashMap, hash::Hash};

use crate::{Int, runtime, translate::Loc};

mod grammar;
pub use grammar::*;

/// The `TranslationUnit` is generic over a string type `S`,
/// allowing flexibility in how text is represented (e.g. `&str`, `String`, `Cow<'_, str>`).
/// Parsing may borrow strings from input, but you can later call `convert()`
/// to transform all `S` values into another representation (such as fully owned `String`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranslationUnit<S: Clone + Eq + Hash> {
    pub workshops: HashMap<S, Shop<S>>,
    pub todos: Vec<ToDo<S>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shop<S> {
    pub name: S,
    pub blocks: Vec<ShopBlock<S>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShopBlock<S> {
    Plan {
        width: usize,
        height: usize,
        map: Vec<Tile<S>>,
    },
    Program(Vec<runtime::Instr>),
}

type Indent = (char, usize);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanRow<S> {
    pub indent: Indent,
    pub tiles: Vec<Tile<S>>,
}

/// Elf walks on tiles in a straight line, unless
/// - Move or Is___ tells him to change direction
/// - Instr::Hammock tells him to halt
/// - He walks into a wall or Unknown, which is error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tile<S> {
    Empty,
    Move(Direction),
    /// Starting point, has no effect if walked on later.
    Elf(Direction),
    /// IsX means if top of stack is X, go right, otherwise go left
    IsZero,
    IsNeg,
    IsPos,
    Instr(runtime::Instr),
    Unknown(S),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Right,
    Down,
    Left,
    Up,
}
impl Direction {
    pub fn left(self) -> Self {
        use Direction::*;
        match self {
            Up => Left,
            Left => Down,
            Down => Right,
            Right => Up,
        }
    }
    pub fn right(self) -> Self {
        use Direction::*;
        match self {
            Up => Right,
            Right => Down,
            Down => Left,
            Left => Up,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToDo<S> {
    /// Connect output of one shop to input of another shop.
    SetupElf {
        shop: S,
        name: Option<S>,
        stack: Vec<Int>,
    },
    /// Connect output of one shop to input of another shop.
    Connect { src: (S, char), dst: (S, char) },
    /// Monitor a pipe and do stuff with incoming message.
    Monitor {
        target: (S, char),
        todos: Vec<ToDo<S>>,
    },
    Receive {
        src: Option<(S, char)>,
        vars: Vec<S>,
    },
    Send {
        dst: Option<(S, char)>,
        values: Vec<Expr<S>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr<S> {
    Number(Int),
    Var(S),
}

impl<S: Clone + Eq + Hash> Default for TranslationUnit<S> {
    fn default() -> Self {
        Self {
            workshops: Default::default(),
            todos: Default::default(),
        }
    }
}

impl<S: Clone + Hash + Eq> TranslationUnit<S> {
    pub fn convert<R: Hash + Clone + Eq>(self, f: &impl Fn(S) -> R) -> TranslationUnit<R> {
        TranslationUnit {
            workshops: self
                .workshops
                .into_iter()
                .map(|(k, v)| (f(k), v.convert(&f)))
                .collect(),
            todos: self.todos.into_iter().map(|t| t.convert(f)).collect(),
        }
    }
}
impl<S> Shop<S> {
    pub fn convert<R>(self, f: &impl Fn(S) -> R) -> Shop<R> {
        Shop {
            name: f(self.name),
            blocks: self.blocks.into_iter().map(|b| b.convert(f)).collect(),
        }
    }
}
impl<S> ShopBlock<S> {
    pub fn convert<R>(self, f: &impl Fn(S) -> R) -> ShopBlock<R> {
        match self {
            ShopBlock::Plan { width, height, map } => ShopBlock::Plan {
                width,
                height,
                map: map.into_iter().map(|t| t.convert(f)).collect(),
            },
            ShopBlock::Program(instrs) => ShopBlock::Program(instrs),
        }
    }
}
impl<S> Tile<S> {
    pub fn convert<R>(self, f: impl Fn(S) -> R) -> Tile<R> {
        use Tile::*;
        match self {
            Empty => Empty,
            Move(d) => Move(d),
            Elf(d) => Elf(d),
            IsNeg => IsNeg,
            IsPos => IsPos,
            IsZero => IsZero,
            Instr(i) => Instr(i),
            Unknown(s) => Unknown(f(s)),
        }
    }
}
impl<S> ToDo<S> {
    pub fn convert<R>(self, f: &impl Fn(S) -> R) -> ToDo<R> {
        use ToDo::*;
        match self {
            SetupElf { name, stack, shop } => SetupElf {
                name: name.map(f),
                shop: f(shop),
                stack,
            },
            Connect { src, dst } => Connect {
                src: (f(src.0), src.1),
                dst: (f(dst.0), dst.1),
            },
            Monitor { target, todos } => Monitor {
                target: (f(target.0), target.1),
                todos: todos.into_iter().map(|x| x.convert(f)).collect(),
            },
            Receive { src, vars } => Receive {
                src: src.map(|x| (f(x.0), x.1)),
                vars: vars.into_iter().map(|x| f(x)).collect(),
            },
            Send { dst, values } => Send {
                dst: dst.map(|x| (f(x.0), x.1)),
                values: values.into_iter().map(|x| x.convert(f)).collect(),
            },
        }
    }
}
impl<S> Expr<S> {
    pub fn convert<R>(self, f: &impl Fn(S) -> R) -> Expr<R> {
        match self {
            Expr::Number(n) => Expr::Number(n),
            Expr::Var(s) => Expr::Var(f(s)),
        }
    }
}

#[test]
fn demonstrate_convert() {
    use std::sync::Arc;

    let unit = {
        let src = String::from("shop_name,elf_name");
        let names: Vec<&str> = src.split(",").collect(); // represents parsed names

        let unit = TranslationUnit {
            workshops: HashMap::from([(
                names[0],
                Shop {
                    name: names[0],
                    blocks: vec![],
                },
            )]),
            todos: vec![ToDo::SetupElf {
                shop: names[0],
                name: Some(names[1]),
                stack: vec![],
            }],
        };

        // src.clear(); // fails, src is borrowed by unit

        unit.convert(&|s| Arc::<str>::from(s))
    };

    println!("{unit:?}");
}
