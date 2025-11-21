//! This module takes care of translating parsed files to a runtime repr

use peg::{error::ParseError, str::LineCol};
use std::{collections::HashMap, fmt, fs, path::PathBuf, sync::Arc};

use crate::{
    Elf, Instr, ElfProgram, Program,
    parse::{ShopBlock, Tile, ToDo, TranslationUnit},
};
use loc::{LineMap, SourceStr};

mod loc;
mod elf;

pub use loc::Loc;
use elf::*;

pub enum TranslationInput {
    File(PathBuf),
}

pub fn translate(inputs: Vec<TranslationInput>) -> Result<Program, Vec<Error>> {
    let mut errors = Vec::new();

    let unit = read_into_unit(inputs, &mut errors);
    if errors.is_empty() == false {
        return Err(errors);
    }

    // translate
    let mut elves = Vec::new();
    let mut rooms = HashMap::new();

    for td in unit.todos {
        match td {
            ToDo::SetupElf { shop, name, stack } => {
                let Some(shop) = unit.workshops.get(&shop) else {
                    todo!("error: no workshop named {shop:?}");
                };

                let mut plans = shop.blocks.iter().filter_map(|blk| blk.as_plan());
                let mut programs = shop.blocks.iter().filter_map(|blk| blk.as_program());

                if programs.clone().count() == 0 {
                    if plans.clone().count() > 1 {
                        errors.push(Error::at(&shop.name, ECode::MultiplePlans));
                    }
                    if let Some(plan) = plans.next() {
                        let prog = translate_plan(&shop.name, plan, &mut errors);
                        if let Some(p) = prog {
                            rooms.insert(shop.name.string.clone(), ElfProgram::new(p));
                        }
                    } else {
                        errors.push(Error::at(&shop.name, ECode::MissingPlan));
                        continue;
                    }
                }

                let room = rooms.get(&shop.name.string).unwrap();

                let elf = Elf::new(room.clone(), vec![todo!()]);
            }
            ToDo::Connect { src, dst } => todo!(),
            ToDo::Monitor { target, todos } => todo!(),
            ToDo::Receive { src, vars } => todo!(),
            ToDo::Send { dst, values } => todo!(),
        }
    }

    match errors.is_empty() {
        true => Ok(Program::new(elves)),
        false => Err(errors),
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
