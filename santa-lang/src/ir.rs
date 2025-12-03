use std::{collections::HashMap, sync::Arc};

use crate::parse::Tile;

#[derive(Debug)]
/// Represents a executable code unit
pub struct Unit {
    pub rooms: Vec<Room>,
    pub santa: Vec<SantaCode>,
}

pub type Int = i64;
pub type Port = u16;
pub type ElfId = usize;
pub type RoomId = usize;
pub type SantaLine = usize;
pub type ElfLine = usize;

#[derive(Debug, Clone)]
pub enum SantaCode {
    SetupElf {
        name: Option<String>,
        room: RoomId,
        stack: Vec<Int>,
    },
    Connect {
        src: (SantaLine, Port),
        dst: (SantaLine, Port),
    },
    OpenRead {
        file: Arc<str>,
        dst: (SantaLine, Port),
    },
    OpenWrite {
        src: (SantaLine, Port),
        file: Arc<str>,
    },
    Monitor {
        port: (SantaLine, Port),
        block_len: usize,
    },
    /// from (elf, port)
    Receive(SantaLine, Port),
    /// send (elf, port, expr)
    Send(SantaLine, Port, SantaLine),
    SendConst(SantaLine, Port, Int),
    Deliver(SantaLine),
}
impl SantaCode {
    pub(crate) fn unwrap_monitor(&self) -> ((SantaLine, Port), usize) {
        match self {
            SantaCode::Monitor { port, block_len } => (*port, *block_len),
            _ => panic!("{self:?}"),
        }
    }
}

pub struct PortIdent {
    elf: SantaLine,
    /// If `indirect`, port is
    port: SantaLine,
}

#[derive(Debug)]
pub struct Room {
    /// Mapping: ip -> x,y
    pub ip_to_tile: HashMap<usize, (usize, usize)>,
    /// (width, height) tuple
    pub size: (usize, usize),
    pub tiles: Vec<Tile<Arc<str>>>,
    pub elf_program: Vec<Instr>,
}
impl Room {
    pub fn get_tile(&self, x:usize,y:usize) -> &Tile<Arc<str>> {
        debug_assert!(x < self.size.0 && y < self.size.1);
        &self.tiles[x + y * self.size.0]
    }
}


#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instr {
    #[default]
    Nop,
    Push(Int),
    Dup(usize),        // push n-th from top to the top
    Erase(usize),      // remove n-th from top
    Tuck(usize),       // insert top before n-th from top
    Swap(usize),       // swap top with n-th from top
    JmpPtr(ElfLine),   // jump to usize
    IfPosPtr(ElfLine), // if top>0, jump to usize
    IfNzPtr(ElfLine),  // if top!=0, jump to usize
    IfEmptyPtr(ElfLine), // if stack is empty, jump
    Arith(Op),
    ArithC(Op, Int),
    StackLen,
    Read(u8),  // read sleeve slot, push on top
    Write(u8), // write to sleeve slot, consuming top
    In(Port),
    Out(Port),
    Hammock,

    // human-friendly branches, only used in tests
    Label(&'static str),
    Jmp(&'static str),
    IfPos(&'static str),
    IfNz(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

pub fn to_port(src: char) -> Port {
    src as u16
}

impl Room {
    #[cfg(test)]
    pub fn new_testing(mut elf_program: Vec<Instr>) -> Self {
        use std::mem;

        use crate::parse::TileKind;

        let mut labels: HashMap<&str, usize> = HashMap::new();
        for (i, instr) in elf_program.iter().enumerate() {
            if let Instr::Label(name) = instr {
                let conflict = labels.insert(*name, i);
                assert!(conflict.is_none(), "Duplicate label {name:?}, line {i}");
            }
        }
        for (i, instr) in elf_program.iter_mut().enumerate() {
            let resolve = |name: &str| {
                *labels
                    .get(name)
                    .unwrap_or_else(|| panic!("Undefined label {name:?} line {i}"))
            };

            *instr = match mem::take(instr) {
                Instr::Jmp(name) => Instr::JmpPtr(resolve(name)),
                Instr::IfPos(name) => Instr::IfPosPtr(resolve(name)),
                Instr::IfNz(name) => Instr::IfNzPtr(resolve(name)),
                x => x,
            }
        }
        Self {
            ip_to_tile: Default::default(),
            size: (1,1),
            tiles: vec![Tile{ text: "  ".into(), kind: TileKind::Empty }],
            elf_program,
        }
    }
}
