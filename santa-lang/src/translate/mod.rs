//! This module takes care of translating parsed files to a runtime repr
//!
//! Translation tasks are eg
//! - symbol resolution
//! - elf program linearization

use peg::{error::ParseError, str::LineCol};
use std::{collections::HashMap, fmt, fs, path::PathBuf, sync::Arc};

use crate::RecoverResult;
use crate::ir::{Instr, Room, SantaCode, Unit, to_port};
use crate::parse::{Expr, ShopBlock, Tile, ToDo, TranslationUnit};
use crate::translate::ident::Identifiers;
use loc::{LineMap, SourceStr};

mod elf;
mod ident;
mod loc;

pub use loc::Loc;

#[derive(Debug, Clone)]
pub enum TranslationInput {
    File(PathBuf),
    Buffer { name: Option<String>, text: String },
}

#[derive(Debug, Clone)]
pub struct Error {
    pub source_name: Arc<str>,
    pub loc: Option<Loc>,
    pub code: ECode,
}
#[derive(Debug, Clone)]
pub enum ECode {
    Io(Arc<std::io::Error>),
    Parse(peg::error::ExpectedSet),
    DuplicateShop(SourceStr),
    MissingPlan,
    MultiplePlans,
    MultiplePrograms,
    MissingElfStart,
    MultipleElfStarts,
    UnknownTile(SourceStr),
    ElfWallHit(usize, usize),
    IdentifierConflict(SourceStr),
    UnknownIdentifier(Arc<str>),
}

pub fn translate(inputs: Vec<TranslationInput>) -> Result<Unit, Vec<Error>> {
    let mut errors = Vec::new();

    let unit = read_into_unit(inputs, &mut errors);
    if errors.is_empty() == false {
        return Err(errors);
    }

    // check which shops are instantiated
    let mut elf_shop_names = Vec::new();
    walk_todos(&unit.todos, &mut |td| match td {
        ToDo::SetupElf { shop, .. } => elf_shop_names.push(shop.string.clone()),
        _ => {}
    });

    // translate
    let mut rooms = Vec::new();
    let mut scode = Vec::new();
    let mut identifiers = Identifiers::new();

    for (sh_name, sh) in unit.workshops {
        let mut plans = sh.blocks.iter().filter_map(|blk| blk.as_plan());

        let Some(plan) = plans.next() else {
            errors.push(Error::at(&sh_name, ECode::MissingPlan));
            continue;
        };
        if plans.next().is_some() {
            errors.push(Error::at(&sh_name, ECode::MultiplePlans));
        }

        let room_opt = elf::translate_plan(&sh_name, plan, &mut errors);
        if let Some(room) = room_opt {
            identifiers.define(&sh_name, rooms.len());
            rooms.push(room);
        }
    }

    emit_todos(&unit.todos, &mut scode, &mut identifiers, &mut errors, None);

    match errors.is_empty() {
        false => Err(errors),
        true => Ok(Unit {
            rooms,
            santa: scode,
        }),
    }
}

fn emit_todos(
    todos: &[ToDo<SourceStr>],
    scode: &mut Vec<SantaCode>,
    identifiers: &mut Identifiers,
    errors: &mut Vec<Error>,
    parent_monitor: Option<usize>,
) {
    for td in todos {
        match td {
            ToDo::SetupElf { shop, name, stack } => {
                if let Some(n) = &name {
                    identifiers.define(&n, scode.len());
                }
                let mut init_stack = Vec::new();
                for expr in stack {
                    let line = match expr {
                        Expr::Number(constant) => {
                            scode.push(SantaCode::Const(*constant));
                            scode.len() - 1
                        },
                        Expr::Var(id) => identifiers.get(id).recover(0, errors),
                    };
                    init_stack.push(line);
                }
                scode.push(SantaCode::SetupElf {
                    name: name.as_ref().map(|s| s.string.to_string()), // TODO Arc::clone
                    room: identifiers.get(shop).recover(0, errors),
                    init_stack,
                });
            }
            ToDo::Connect { src, dst } => {
                use crate::parse::Connection::*;
                match (src, dst) {
                    (Port(src_id, src_port), Port(dst_id, dst_port)) => {
                        let src_elf = identifiers.get(src_id).recover(0, errors);
                        let dst_elf = identifiers.get(dst_id).recover(0, errors);
                        scode.push(SantaCode::Connect {
                            src: (src_elf, to_port(*src_port)),
                            dst: (dst_elf, to_port(*dst_port)),
                        });
                    }
                    (File(name), Port(dst_id, dst_port)) => {
                        let dst_elf = identifiers.get(dst_id).recover(0, errors);
                        scode.push(SantaCode::OpenRead {
                            file: name.string.clone(),
                            dst: (dst_elf, to_port(*dst_port)),
                        });
                    }
                    (Port(src_id, src_port), File(name)) => {
                        let src_elf = identifiers.get(src_id).recover(0, errors);
                        scode.push(SantaCode::OpenWrite {
                            src: (src_elf, to_port(*src_port)),
                            file: name.string.clone(),
                        });
                    }
                    _ => todo!("{src:?} -> {dst:?}"),
                }
            }
            ToDo::Monitor { target, todos } => {
                let elfid = identifiers.get(&target.0).recover(0, errors);
                let block_start = scode.len();
                scode.push(SantaCode::Monitor {
                    port: (elfid, to_port(target.1)),
                    block_len: 0,
                });
                emit_todos(todos, scode, identifiers, errors, Some(block_start));
                let block_end = scode.len();
                scode[block_start] = SantaCode::Monitor {
                    port: (elfid, to_port(target.1)),
                    block_len: block_end - block_start,
                };
            }
            ToDo::Receive { src, vars } => {
                let port = match (src, parent_monitor) {
                    (Some(src), _) => (identifiers.get(&src.0).recover(0, errors), to_port(src.1)),
                    (None, Some(par)) => {
                        let SantaCode::Monitor { port, .. } = &scode[par] else {
                            panic!("bug: parent block is not monitor")
                        };
                        *port
                    }
                    (None, None) => todo!("error: receive used outside of monitor block"),
                };

                for v in vars {
                    identifiers.define(v, scode.len()).recover((), errors);
                    scode.push(SantaCode::Receive(port.0, port.1));
                }
            }
            ToDo::Send { dst, values } => {
                let port = match (dst, parent_monitor) {
                    (Some(dst), _) => (identifiers.get(&dst.0).recover(0, errors), to_port(dst.1)),
                    (None, Some(par)) => {
                        let SantaCode::Monitor { port, .. } = &scode[par] else {
                            panic!("bug: parent block is not monitor")
                        };
                        *port
                    }
                    (None, None) => todo!("error: receive used outside of monitor block"),
                };

                for v in values {
                    let ip = match v {
                        Expr::Number(n) => {
                            scode.push(SantaCode::Const(*n));
                            scode.len() - 1
                        },
                        Expr::Var(v) => identifiers.get(v).recover(0, errors),
                    };
                    scode.push(SantaCode::Send(port.0, port.1, ip));
                }
            }
            ToDo::Deliver { e } => {
                let ip = match e {
                    Expr::Number(n) => {
                        scode.push(SantaCode::Const(*n));
                        scode.len() - 1
                    },
                    Expr::Var(v) => identifiers.get(v).recover(0, errors),
                };
                scode.push(SantaCode::Deliver(ip));
            }
        }
    }
}

fn read_into_unit(
    inputs: Vec<TranslationInput>,
    errors: &mut Vec<Error>,
) -> TranslationUnit<SourceStr> {
    let mut unit = TranslationUnit::default();

    for input in inputs {
        let source_name: Arc<str>;
        let source: String;

        match input {
            TranslationInput::File(path_buf) => {
                source_name = path_buf.to_string_lossy().into();

                let read = fs::read_to_string(path_buf);
                match read {
                    Ok(s) => source = s,
                    Err(e) => {
                        errors.push(Error {
                            source_name,
                            loc: None,
                            code: ECode::Io(e.into()),
                        });
                        continue;
                    }
                }
            }
            TranslationInput::Buffer { name, text } => {
                source_name = name.unwrap_or_else(|| "anonymous".into()).into();
                source = text;
            }
        }

        let map = LineMap::new(&source_name, &source);

        let new_unit = match crate::parse(&source) {
            Ok(u) => u,
            Err(e) => {
                errors.push(Error::from_parse(&source_name, e));
                continue;
            }
        };

        unit.import_from(new_unit, errors, &map);
    }
    unit
}

fn walk_todos<S>(list: &[ToDo<S>], visit: &mut impl FnMut(&ToDo<S>)) {
    for i in list {
        visit(i);
        match i {
            ToDo::Monitor { todos, .. } => walk_todos(todos, visit),
            _ => {}
        }
    }
}

impl TranslationUnit<SourceStr> {
    /// Accumulate all units into a single unit
    fn import_from(
        &mut self,
        other: TranslationUnit<&str>,
        errors: &mut Vec<Error>,
        map: &loc::LineMap,
    ) {
        for (k, shop) in other.workshops {
            let key = map.map_slice(k);
            let conflict = self
                .workshops
                .insert(key.clone(), shop.convert(&|s| map.map_slice(s)));

            if let Some(c) = conflict {
                errors.push(map.error_at(k, ECode::DuplicateShop(c.name)));
            }
        }

        for td in other.todos {
            self.todos.push(td.convert(&|s| map.map_slice(s)));
        }
    }
}

impl<S> ShopBlock<S> {
    fn as_plan(&self) -> Option<(usize, usize, &[Tile<S>])> {
        match self {
            ShopBlock::Plan { width, height, map } => Some((*width, *height, map.as_slice())),
            _ => None,
        }
    }
    fn as_program(&self) -> Option<&[Instr]> {
        match self {
            ShopBlock::Program(code) => Some(code.as_slice()),
            _ => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut locations: Vec<_> = self.loc.iter().collect();

        match &self.code {
            ECode::Io(e) => {
                write!(f, "{e}: {}", self.source_name)?;
                locations.clear();
            }
            ECode::Parse(expected_set) => write!(f, "expected {expected_set}")?,
            ECode::DuplicateShop(shop) => {
                write!(f, "duplicate shop definition: {}", shop.string)?;
                locations.push(&shop.loc);
            }
            ECode::MissingPlan => write!(f, "missing floorplan block")?,
            ECode::MultiplePlans => write!(f, "multiple floorplan blocks found")?,
            ECode::MultiplePrograms => write!(f, "multiple program blocks found")?,
            ECode::MissingElfStart => write!(f, "missing elf starting tile")?,
            ECode::MultipleElfStarts => write!(f, "multiple elf starting tiles")?,
            ECode::UnknownTile(s) => {
                write!(f, "Unknown tile '{}'", s.string)?;
                locations.clear();
                locations.push(&s.loc);
            }
            ECode::ElfWallHit(x, y) => write!(f, "elf walks into a wall on tile {x},{y}")?,
            ECode::IdentifierConflict(existing) => {
                write!(f, "identifier redefined: {}", existing.display_at())?
            }
            ECode::UnknownIdentifier(id) => write!(f, "unknown identifier \"{id}\"")?,
        }

        if let Some(loc) = &self.loc {
            write!(f, "\n - {}:{}:{}", self.source_name, loc.line, loc.col)?;
        }
        Ok(())
    }
}
impl Error {
    fn from_parse(source_name: &Arc<str>, e: ParseError<LineCol>) -> Self {
        Self {
            source_name: source_name.clone(),
            loc: Some(Loc {
                line: e.location.line as u32,
                col: e.location.column as u32,
                len: 1,
            }),
            code: ECode::Parse(e.expected),
        }
    }
    fn at(token: &SourceStr, code: ECode) -> Self {
        Self {
            source_name: token.source_name.clone(),
            loc: Some(token.loc.clone()),
            code,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        ir::Unit,
        translate::{Error, TranslationInput},
    };

    fn make_unit(src: &str) -> Result<Unit, Vec<Error>> {
        super::translate(vec![TranslationInput::Buffer {
            name: None,
            text: src.into(),
        }])
    }

    #[test]
    fn loopback_port() {
        let unit = make_unit(
            "

            workshop emit_stack:
                floorplan:
                    mv m<
                    ev O1
                    m> ?s
                       m> I1 Oo mv
                       m^       m<
                ;
            ;

            Santa will:
                setup emit_stack for elf Baba (1 2 3 4 5 6 7)
                setup Baba.1 -> Baba.1
            ;

            ",
        );

        unit.unwrap();
    }
}
