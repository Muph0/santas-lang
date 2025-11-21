use std::{collections::HashMap, mem, sync::Arc};

use super::{Runtime, pipe::*};

pub type Int = i64;
pub type Port = u32;
pub type ElfId = u16;

#[derive(Debug)]
pub struct Room {
    pub(super) program: Vec<Instr>,
}

pub struct Elf {
    pub(super) room: Arc<Room>,
    pub(super) name: Option<&'static str>,
    pub(super) instr: usize,
    pub(super) stack: Vec<Int>,
    pub(super) inputs: HashMap<Port, InputPipe<Int>>,
    pub(super) outputs: HashMap<Port, Output>,
    pub(super) finished: bool,
}

#[derive(Default)]
pub(super) struct Output {
    pub(super) pipe: OutputPipe<Int>,
    pub(super) monitor: Option<Arc<dyn Fn(&mut Runtime, Int)>>,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instr {
    #[default]
    Nop,
    Push(Int),
    Dup(usize),   // push n-th from top to the top
    Erase(usize), // remove n-th from top
    Tuck(usize),  // insert top before n-th from top
    Swap(usize),  // swap top with n-th from top
    JmpPtr(usize), // jump to usize
    IfPosPtr(usize), // if top>0, jump to usize
    IfNzPtr(usize), // if top!=0, jump to usize
    Arith(Op),
    ArithC(Op, Int),
    In(Port),
    Out(Port),
    Hammock,

    // human-friendly branches only used in tests
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

#[derive(Debug)]
pub enum Error {
    InvalidIndex(usize),
    InvalidInstr,
    DivisionByZero,
}

impl Room {
    pub fn new(mut program: Vec<Instr>) -> Arc<Self> {
        let mut labels: HashMap<&str, usize> = HashMap::new();
        for (i, instr) in program.iter().enumerate() {
            if let Instr::Label(name) = instr {
                let conflict = labels.insert(*name, i);
                assert!(conflict.is_none(), "Duplicate label {name:?}, line {i}");
            }
        }
        for (i, instr) in program.iter_mut().enumerate() {
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
        Self { program }.into()
    }
}

impl Elf {
    pub fn new(room: Arc<Room>, stack: Vec<Int>) -> Self {
        Self {
            room,
            name: None,
            instr: 0,
            stack,
            inputs: Default::default(),
            outputs: Default::default(),
            finished: false,
        }
    }

    pub fn connect(&mut self, out_port: Port, (other, other_input): (&mut Elf, Port)) {
        let output = self
            .outputs
            .entry(out_port)
            .or_insert_with(|| Output::default());

        other
            .inputs
            .entry(other_input)
            .and_modify(|input| input.connect(&mut output.pipe))
            .or_insert_with(|| InputPipe::new_connected(&mut output.pipe));
    }

    pub fn monitor(&mut self, port: Port, monitor: impl Fn(&mut Runtime, Int) + 'static) {
        self.outputs
            .entry(port)
            .or_insert_with(|| Output::default())
            .monitor
            .replace(Arc::new(monitor));
    }

    pub(super) fn top_idx(&self, from_top: usize) -> Result<usize, Error> {
        let stack_len = self.stack.len();
        match from_top < stack_len {
            true => Ok(stack_len - from_top - 1),
            false => Err(Error::InvalidIndex(from_top)),
        }
    }
    pub(super) fn top_val(&self, from_top: usize) -> Result<Int, Error> {
        Ok(self.stack[self.top_idx(from_top)?])
    }
}
