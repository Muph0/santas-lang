use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    fmt,
    sync::Arc,
};

pub use crate::ir::*;
pub use pipe::*;

mod pipe;

pub struct Runtime<'u> {
    unit: &'u Unit,
    /// Instruction pointer.
    pub santa_ip: SantaLine,
    /// Each santa code line can produce a value.
    santa_result: Vec<usize>,
    /// Auto-increment id for new elves
    next_elf_id: ElfId,
    /// Stores active elves. They get deleted when they finish.
    pub elves: HashMap<ElfId, Elf>,
    /// Queue for elf scheduling.
    schedule: VecDeque<Turn>,
    /// Each monitor is a pair of (pipe, santa_handler_ptr)
    monitors: HashMap<(ElfId, Port), (InputPipe<Int>, SantaLine)>,
}

pub struct Elf {
    /// Instruction pointer
    ip: ElfLine,
    room: RoomId,
    id: ElfId,
    name: String,
    stack: Vec<Int>,
    inputs: HashMap<Port, InputPipe<Int>>,
    outputs: HashMap<Port, OutputPipe<Int>>,
    finished: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum RunCommand {
    /// Run to the end without stopping.
    Run,
    /// Continue to next breakpoint.
    Continue,
    /// Step n steps.
    Step(usize),
}

#[derive(Debug, Clone, Copy)]
pub enum RunResult {
    /// Result of a Step command, says how many steps were taken.
    Stepped(usize),
    /// A breakpoint was hit.
    Breakpoint,
    Done,
}

#[derive(Debug)]
pub struct ElfError<'p> {
    unit: &'p Unit,
    code: Error,
    ip: usize,
    stack: Vec<Int>,
    room_id: usize,
}

#[derive(Debug, Clone, Copy)]
enum Turn {
    Santa,
    Elf(ElfId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Event {
    Yield,
    Dequeue,
    Breakpoint,
    Write(Port, Int),
}

#[rustfmt::skip]
const ELF_NAMES: [&str; 256] = [
    "Alabaster", "Archibald", "Applejack", "Amberglow", "Astra", "Auburn", "Aurora", "Amity", "Aurelian", "Azura", "Aspen",
    "Bells", "Blitzie", "Bounder", "Bubble", "Buddy", "Bramble", "Biscuit", "Beryl", "Brio", "Blythe",
    "Cherry", "Cookie", "Cocoa", "Crinkle", "Cuddles", "Charm", "Clover", "Candlenut", "Celestia", "Crispin",
    "Dabble", "Dandy", "Doodle", "Dingle", "Dongle", "Dazzle", "Drizzle", "Dulcie", "Dewdrop", "Dandelion",
    "Ellie", "Elmo", "Evergreen", "Ember", "Echo", "Edelweiss", "Elfina", "Euphoria", "Elara", "Eos",
    "Flurry", "Frosty", "Frostfern", "Frostine", "Figgy", "Flicker", "Frangle", "Fable", "Frolic", "Feather", "Fiora",
    "Glimmer", "Glitter", "Gingersnap", "Glee", "Gossamer", "Gusty", "Giddy", "Glowbug", "Galatea", "Glimora", "Glintleaf",
    "Holly", "Happy", "Harmony", "Hobnob", "Hugsy", "Hickory", "Hazel", "Humphrey", "Halcyon", "Hesper",
    "Icicle", "Ivy", "Inky", "Iris", "Iggle", "Isolde", "Iota", "Illumina", "Indigo", "Iolana",
    "Jimmy", "Jingle", "Jolly", "Jovial", "Jester", "Jubilee", "Jasmine", "Joviette", "Juniper", "Jovani",
    "Kandy", "Kip", "Knickers", "Kringle", "Kookie", "Kismet", "Keenan", "Kettle", "Kalliope", "Korrin",
    "Lolly", "Lumi", "Lucky", "Larkspur", "Luster", "Lilac", "Lively", "Linden", "Lyric", "Liora",
    "Maple", "Merry", "Misty", "Muffin", "Myrth", "Mallow", "Moonbeam", "Moonwhisper", "Moppet", "Mirabel", "Mystara",
    "Nibbles", "Nutmeg", "Nuzzle", "Nifty", "Nectar", "Noodle", "Nimble", "Nimora", "Nerissa", "Noxie",
    "Olaf", "Opal", "Orin", "Orca", "Onyx", "Olive", "Octavia", "Ocarina", "Odette", "Orchid",
    "Pepper", "Peppermint", "Pinecone", "Pippin", "Purdy", "Puddle", "Pixie", "Pansy", "Primrose", "Pavonine",
    "Quincy", "Quibble", "Quill", "Quirky", "Quaver", "Quartz", "Quokka", "Quenby", "Quarra", "Quintessa",
    "Ripplo", "Rolo", "Rudy", "Ruffles", "Rusty", "Razzle", "Ramble", "Rhyme", "Riven", "Roscoe",
    "Shinny", "Snowdrop", "Snowflake", "Snappy", "Sparkleberry", "Sprinkle", "Sugarplum", "Starbright", "Solstice", "Sylphie", "Sylvaris",
    "Tinsel", "Twinkle", "Taffy", "Tango", "Tiptoe", "Truffle", "Tulip", "Tinker", "Thistle", "Tauriel", "Thalindra",
    "Vixen", "Vivi", "Velvet", "Vireo", "Vesper", "Verity", "Valen", "Valkyra", "Viridian", "Vallora",
    "Wunorse", "Waffle", "Winky", "Whimsy", "Wobble", "Wander", "Wisp", "Wisteria", "Willow", "Wyrda",
    "Xander", "Xylo", "Xenia", "Xavi", "Xylia", "Xanadu", "Xerra", "Xiomara", "Xeraphine", "Xylora",
    "Yule", "Yara", "Yanni", "Yippee", "Yarrow", "Yodel", "Yvette", "Yonder", "Ysabel", "Ysolde",
    "Zanzwi", "Zulu", "Zigzag", "Zippy", "Zinna", "Zephyr", "Zelda", "Zodiac", "Zarina", "Zyra",
];

impl<'u> Runtime<'u> {
    pub fn new(unit: &'u Unit) -> Self {
        Self {
            unit,
            santa_ip: 0,
            santa_result: vec![0; unit.santa.len()],
            next_elf_id: 0,
            elves: Default::default(),
            schedule: VecDeque::from([Turn::Santa]),
            monitors: Default::default(),
        }
    }

    pub fn run(&mut self, cmd: RunCommand) -> RunResult {
        while self.schedule.len() > 0 {
            let next = self.schedule.pop_front().unwrap();

            let evt = match next {
                Turn::Santa => self.step_santa(),
                Turn::Elf(id) => self.step_elf(id),
            };

            match evt {
                _ => {}
            }

            match evt {
                Some(Event::Yield) => self.schedule.push_back(next),
                Some(Event::Dequeue) => {} // no requeue
                _ => self.schedule.push_front(next), // else repeat the same `next`
            }
        }
        RunResult::Done
    }

    fn step_santa(&mut self) -> Option<Event> {
        let Some(code) = self.unit.santa.get(self.santa_ip) else {
            return Some(Event::Dequeue);
        };

        let mut next_ip = self.santa_ip + 1;
        let _g = &self.santa_ip;

        let event = match code {
            SantaCode::SetupElf { name, room, stack } => {
                let new = Elf {
                    ip: 0,
                    room: *room,
                    id: self.next_elf_id,
                    name: name.clone().unwrap_or_else(|| {
                        ELF_NAMES[self.next_elf_id % ELF_NAMES.len()].to_string()
                    }),
                    stack: stack.clone(),
                    inputs: Default::default(),
                    outputs: Default::default(),
                    finished: false,
                };
                self.next_elf_id += 1;

                self.santa_result[self.santa_ip] = new.id;
                self.elves.insert(new.id, new);
                None
            }
            SantaCode::Connect { src, dst } => {
                let src_eid = self.santa_result[src.0];
                let dst_eid = self.santa_result[dst.0];

                if let [Some(src_elf), Some(dst_elf)] =
                    self.elves.get_disjoint_mut([&src_eid, &dst_eid])
                {
                    let mut output = src_elf.ensure_output(src.1);
                    dst_elf.ensure_input(dst.1, &mut output);
                } else if src_eid == dst_eid {
                    let elf = self.elves.get_mut(&src_eid).unwrap();
                    let port = src.1;
                    let output = elf
                        .outputs
                        .entry(port)
                        .or_insert_with(|| OutputPipe::default());

                    let port = dst.1;
                    elf.inputs
                        .entry(port)
                        .and_modify(|input| input.connect(output))
                        .or_insert_with(|| InputPipe::new_connected(output));
                } else {
                    panic!("SantaCode::Connect {{ {src:?}, {dst:?} }}")
                }
                None
            }
            SantaCode::Monitor { port, block_len } => {
                let elf_id = self.santa_result[port.0];
                let port = port.1;
                let elf = self
                    .elves
                    .get_mut(&elf_id)
                    .unwrap_or_else(|| panic!("{port:?}, {block_len}"));
                let output = elf.ensure_output(port);

                let v = (InputPipe::new_connected(output), self.santa_ip);
                let conflict = self.monitors.insert((elf_id, port), v);

                assert!(conflict.is_none(), "port=({elf_id}, {port})");
                next_ip = self.santa_ip + *block_len;
                None
            }
            SantaCode::Receive(elf_line, port) => {
                let elf_id = self.santa_result[*elf_line];

                let monitor = &self.monitors[&(elf_id, *port)];

                match monitor.0.try_read() {
                    Err(InputError::Closed) => Some(Event::Dequeue), // reading closed input semantically hangs forever
                    Err(InputError::Empty) => {
                        next_ip = self.santa_ip; // will re-read in next cycle
                        Some(Event::Yield)
                    }
                    Ok(recvd) => {
                        self.santa_result[self.santa_ip] = recvd as _;
                        None
                    }
                }
            }
            SantaCode::Send(_, _, _) => todo!(),
            SantaCode::SendConst(_, _, _) => todo!(),
        };

        _ = _g;
        self.santa_ip = next_ip;
        event
    }
    fn step_elf(&mut self, id: ElfId) -> Option<Event> {}

    pub fn run_loop(&mut self) -> Result<(), ElfError> {
        while self.schedule.len() > 0 {
            let elf_id = self.schedule.iter().next().unwrap();
            let elf = self.elves.get_mut(&elf_id).unwrap();

            match elf.step() {
                Ok(None) => {}
                Ok(Some(Event::Write(port, value))) => {
                    if let Some(monitor) = self.elves[&elf_id].outputs[&port].monitor.clone() {
                        monitor(self, value);
                    }
                    let front = self.schedule.pop_front().unwrap();
                    self.schedule.push_back(front);
                }
                Ok(Some(Event::Yield)) => {
                    let front = self.schedule.pop_front().unwrap();
                    self.schedule.push_back(front);
                }
                Err(e) => {
                    return Err(ElfError {
                        code: e,
                        ip: elf.instr,
                        stack: elf.stack.clone(),
                        room: elf.room.clone(),
                    });
                }
            }

            self.elves.retain(|_, e| e.finished == false);
        }

        Ok(())
    }
}

impl Elf {
    fn ensure_output(&mut self, port: Port) -> &mut OutputPipe<i64> {
        self.outputs
            .entry(port)
            .or_insert_with(|| OutputPipe::default())
    }
    fn ensure_input(&mut self, port: Port, connect: &mut OutputPipe<Int>) -> &mut InputPipe<Int> {
        self.inputs
            .entry(port)
            .and_modify(|input| input.connect(connect))
            .or_insert_with(|| InputPipe::new_connected(connect))
    }

    fn step(&mut self, unit: &Unit) -> Result<Option<Event>, Error> {
        use Instr::*;
        if self.finished {
            return Ok(None);
        }

        let code_opt = self.room.program.get(self.instr).cloned();
        let code = code_opt.unwrap_or(Hammock);

        let mut event = None;
        let mut next_instr = self.instr + 1;
        let guard_instr = &self.instr; // you should write to next_instr instead

        match code {
            Nop | Label(_) => {}
            Push(value) => self.init_stack.push(value),
            Dup(i) => self.init_stack.push(self.top_val(i)?),
            Erase(i) => {
                self.init_stack.remove(self.top_idx(i)?);
            }
            Tuck(i) => {
                let index = self.top_idx(i)?;
                let top = self.init_stack.pop().unwrap();
                self.init_stack.insert(index, top);
            }
            Swap(i) => {
                let top_i = self.top_idx(0)?;
                let index = self.top_idx(i)?;
                self.init_stack.swap(top_i, index);
            }
            Jmp(_) | IfPos(_) | IfNz(_) => return Err(Error::InvalidInstr),
            JmpPtr(target) => next_instr = target,
            IfPosPtr(target) => {
                if self.top_val(0)? > 0 {
                    next_instr = target
                }
                self.init_stack.pop();
            }
            IfNzPtr(target) => {
                if self.top_val(0)? != 0 {
                    next_instr = target
                }
                self.init_stack.pop();
            }
            Arith(op) => {
                let result = op.invoke(self.top_val(1)?, self.top_val(0)?)?;
                self.init_stack.pop();
                self.init_stack.pop();
                self.init_stack.push(result);
            }
            ArithC(op, c) => {
                let result = op.invoke(self.top_val(0)?, c)?;
                self.init_stack.pop();
                self.init_stack.push(result);
            }
            In(port) => match self.inputs.get_mut(&port).map(|p| p.try_read()) {
                Some(Ok(value)) => self.init_stack.push(value),
                Some(Err(InputError::Empty)) => {
                    next_instr = self.instr; // wait here for input
                    event = Some(Event::Yield);
                }
                None | Some(Err(InputError::Closed)) => {
                    self.finished = true;
                }
            },
            Out(port) => {
                if let Some(output) = self.outputs.get(&port) {
                    let top = self.top_val(0)?;
                    output.pipe.write(top);
                    self.init_stack.pop();
                    event = Some(Event::Write(port, top));
                }
            }
            Hammock => {
                self.finished = true;
            }
        };

        log::trace!(
            "elf {} > {:>3} | {:<25}{:?}",
            self.name.unwrap_or("Unk?"),
            self.instr,
            format!("{:?}", code),
            &self.init_stack[self.init_stack.len().saturating_sub(10)..]
        );

        _ = guard_instr;
        self.instr = next_instr;
        Ok(event)
    }
}

impl Op {
    fn invoke(&self, a: i64, b: i64) -> Result<Int, Error> {
        return Ok(match self {
            Op::Add => a + b,
            Op::Sub => a - b,
            Op::Mul => a * b,
            Op::Div if b == 0 => return Err(Error::DivisionByZero),
            Op::Div => a / b,
            Op::Mod => a % b,
        });
    }
}

impl fmt::Display for ElfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Elf encountered a problem and doesn't know what to do: ")?;
        match self.code {
            Error::InvalidIndex(i) => writeln!(f, "invalid index {i}"),
            Error::InvalidInstr => writeln!(f, "invalid instruction"),
            Error::DivisionByZero => writeln!(f, "division by zero"),
        }?;

        writeln!(f, "  stack: {:?}", self.stack)?;

        let program_peek = self.room.elf_program.iter().enumerate();
        for (i, instr) in program_peek.skip(self.ip.saturating_sub(2)).take(5) {
            let caret = if i == self.ip { '>' } else { ' ' };
            writeln!(f, "{:>5} | {instr:?}", format!("{caret} {i}"))?;
        }

        Ok(())
    }
}
