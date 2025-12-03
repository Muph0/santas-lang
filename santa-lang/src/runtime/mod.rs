use std::{
    collections::{HashMap, VecDeque},
    fmt, fs, io, usize,
};

use crate::DropGuard;
pub use crate::ir::*;
pub use pipe::*;

mod pipe;

#[derive(Debug)]
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
    /// Output of the santa's deliver command
    pub output: Out,
    /// IO files
    in_files: Vec<OutputPipe<Int>>,
    out_files: Vec<OutFile>,
}

#[derive(Debug, Clone)]
pub enum Out {
    Std,
    Buffer(String),
}

#[derive(Debug)]
pub struct Elf {
    /// Instruction pointer
    ip: ElfLine,
    room: RoomId,
    id: ElfId,
    name: String,
    stack: Vec<Int>,
    sleeve: Box<[Int; 10]>,
    inputs: HashMap<Port, InputPipe<Int>>,
    outputs: HashMap<Port, OutputPipe<Int>>,
    finished: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum RunCommand {
    /// Run to the end without stopping.
    RunToEnd,
    /// Continue to next breakpoint.
    Continue,
    /// Step n steps.
    Step(usize),
}

#[derive(Debug, Clone)]
pub enum RunOk {
    /// Result of a Step command, says how many steps were taken.
    Stepped(usize),
    /// A breakpoint was hit.
    Breakpoint,
    Done,
}

#[derive(Debug, Clone)]
pub struct Error<'u> {
    unit: &'u Unit,
    ip: usize,
    room: Option<RoomId>,
    culprit: Turn,
    code: ECode,
    stack: Vec<Int>,
}
#[derive(Debug, Clone)]
pub enum ECode {
    InvalidIndex(usize),
    InvalidInstr,
    DivisionByZero,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Turn {
    Santa { ip: usize, until: usize },
    Elf(ElfId),
}
impl Turn {
    fn unwrap_elfid(&self) -> usize {
        match self {
            Turn::Santa { .. } => panic!(),
            Turn::Elf(id) => *id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Event {
    Yield,
    Dequeue,
    Breakpoint,
    Write(Port),
}

struct OutFile {
    pipe: InputPipe<Int>,
    writer: Box<dyn io::Write>,
}
impl fmt::Debug for OutFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OutFile").field("pipe", &self.pipe).finish()
    }
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
            schedule: VecDeque::from([Turn::Santa {
                ip: 0,
                until: unit.santa.len(),
            }]),
            monitors: Default::default(),
            output: Out::Std,

            in_files: Vec::new(),
            out_files: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new(self.unit);
    }

    pub fn run(&mut self, cmd: RunCommand) -> Result<RunOk, Error> {
        let mut last = None;
        let mut steps = 0u64;

        let result = loop {
            let Some(mut next) = self.schedule.pop_front() else {
                break Ok(RunOk::Done);
            };
            if Some(next) != last {
                match next {
                    Turn::Elf(id) => log::debug!("Scheduling {next:?} {:?}", self.elves[&id].name),
                    _ => log::debug!("Scheduling {next:?}"),
                };
                last = Some(next);
            }

            let result = match &mut next {
                Turn::Santa { ip, until } => self.step_santa(ip, until),
                Turn::Elf(id) => self.step_elf(*id),
            };

            let evt = match result {
                Ok(ev) => ev,
                Err(ecode) => {
                    let (ip, stack, room) = match next {
                        Turn::Santa { ip, .. } => (ip, vec![], None),
                        Turn::Elf(id) => {
                            let elf = &self.elves[&id];
                            (elf.ip, elf.stack.clone(), Some(elf.room))
                        }
                    };
                    let error = Error {
                        unit: self.unit,
                        ip,
                        room,
                        culprit: next,
                        code: ecode,
                        stack,
                    };
                    self.reset();
                    break Err(error);
                }
            };

            if evt.is_some() {
                log::trace!("evt={evt:?}");
            }

            // requeue
            match evt {
                Some(Event::Dequeue) => match next {
                    Turn::Elf(id) => {
                        self.elves.remove(&id);
                    }
                    _ => {}
                },
                Some(Event::Yield | Event::Write(_)) => self.schedule.push_back(next),
                _ => self.schedule.push_front(next), // else repeat the same `next`
            }

            // event side effect
            match evt {
                Some(Event::Breakpoint) => todo!("breakpoint"),
                Some(Event::Write(port)) => {
                    let key = (next.unwrap_elfid(), port);
                    if let Some(mon) = self.monitors.get(&key) {
                        self.santa_ip = mon.1 + 1;
                        self.schedule.push_front(Turn::Santa {
                            ip: mon.1 + 1,
                            until: self.unit.santa[mon.1].unwrap_monitor().1,
                        });
                    }
                }
                _ => {}
            }

            steps += 1;
            if steps % (1 << 10) == 0 {
                self.flush_outs();
            }

            match cmd {
                RunCommand::Step(n) if steps as usize >= n => {
                    return Ok(RunOk::Stepped(steps as usize));
                }
                _ => {}
            }
        };

        self.flush_outs();
        result
    }

    fn step_santa(&mut self, ip: &mut usize, until: &usize) -> Result<Option<Event>, ECode> {
        let Some(code) = self.unit.santa.get(self.santa_ip) else {
            return Ok(Some(Event::Dequeue));
        };

        let mut next_ip = self.santa_ip + 1;
        let (_g, ip) = (&self.santa_ip, self.santa_ip);

        let trace_code: SantaCode = code.clone();
        let trace = DropGuard::new(move || {
            log::trace!("santa: {ip:4} | {trace_code:?}");
        });

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
                    sleeve: Box::new([0; 10]),
                    inputs: Default::default(),
                    outputs: Default::default(),
                    finished: false,
                };
                self.next_elf_id += 1;

                self.schedule.push_back(Turn::Elf(new.id));
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
            SantaCode::OpenRead { file, dst } => {
                let content = fs::read_to_string(file.as_ref()).unwrap();
                let elfid = self.santa_result[dst.0];
                if let Some(elf) = self.elves.get_mut(&elfid) {
                    // this will produce closed pipe
                    let input = elf.ensure_input(dst.1, &mut OutputPipe::new());
                    for c in content.chars() {
                        input.write_direct(c as Int);
                    }
                } else {
                    panic!("bug: unknown elf {elfid}");
                }
                None
            }
            SantaCode::OpenWrite { src, file } => {
                let wr = io::BufWriter::new(fs::File::create(&**file).expect(&file));
                let elfid = self.santa_result[src.0];
                if let Some(elf) = self.elves.get_mut(&elfid) {
                    let file_pipe = InputPipe::new_connected(elf.ensure_output(src.1));
                    self.out_files.push(OutFile {
                        pipe: file_pipe,
                        writer: Box::new(wr),
                    });
                } else {
                    panic!("bug: unknown elf {elfid}\n{self:?}");
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

                let monitor = self.monitors.get_mut(&(elf_id, *port)).unwrap();

                match monitor.0.try_read() {
                    Err(InputError::Closed) => Some(Event::Dequeue), // reading closed input hangs forever
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
            SantaCode::Deliver(line) => {
                let c = self.santa_result[*line] as u8 as char;
                match &mut self.output {
                    Out::Std => print!("{}", c),
                    Out::Buffer(buf) => buf.push(c),
                };
                None
            }
        };

        let trace_code = code.clone();
        let result = self.santa_result[self.santa_ip];
        let trace = trace.reset(move || {
            log::trace!(
                "santa: {ip:4} | {:18} -> {result}",
                format!("{trace_code:?}")
            );
        });

        _ = _g;
        self.santa_ip = next_ip;
        Ok(event)
    }

    fn step_elf(&mut self, id: ElfId) -> Result<Option<Event>, ECode> {
        use Instr::*;
        let unit = self.unit;
        let Some(elf) = self.elves.get_mut(&id) else {
            todo!("no elf {id}");
        };

        let code_opt = unit.rooms[elf.room].elf_program.get(elf.ip);
        let code = code_opt.cloned().unwrap_or(Hammock);

        let mut event = None;
        let mut next_ip = elf.ip + 1;
        let _g = &elf.ip; // you should write to next_instr instead

        match code {
            Nop | Label(_) => {}
            Push(value) => elf.stack.push(value),
            Dup(i) => elf.stack.push(elf.top_val(i)?),
            Erase(i) => {
                elf.stack.remove(elf.top_idx(i)?);
            }
            Tuck(i) => {
                let index = elf.top_idx(i)?;
                let top = elf.stack.pop().unwrap();
                elf.stack.insert(index, top);
            }
            Swap(i) => {
                let top_i = elf.top_idx(0)?;
                let index = elf.top_idx(i)?;
                elf.stack.swap(top_i, index);
            }
            Jmp(_) | IfPos(_) | IfNz(_) => return Err(ECode::InvalidInstr),
            JmpPtr(target) => next_ip = target,
            IfPosPtr(target) => {
                if elf.top_val(0)? > 0 {
                    next_ip = target
                }
                elf.stack.pop();
            }
            IfNzPtr(target) => {
                if elf.top_val(0)? != 0 {
                    next_ip = target
                }
                elf.stack.pop();
            }
            IfEmptyPtr(target) => {
                if elf.stack.is_empty() {
                    next_ip = target;
                }
            }
            Arith(op) => {
                let result = op.invoke(elf.top_val(1)?, elf.top_val(0)?)?;
                elf.stack.pop();
                elf.stack.pop();
                elf.stack.push(result);
            }
            ArithC(op, c) => {
                let result = op.invoke(elf.top_val(0)?, c)?;
                elf.stack.pop();
                elf.stack.push(result);
            }
            In(port) => match elf.inputs.get_mut(&port).map(|p| p.try_read()) {
                Some(Ok(value)) => elf.stack.push(value),
                Some(Err(InputError::Empty)) => {
                    next_ip = elf.ip; // wait here for input
                    event = Some(Event::Yield);
                }
                None | Some(Err(InputError::Closed)) => {
                    elf.finished = true;
                }
            },
            Out(port) => {
                let top = elf.top_val(0)?;
                elf.stack.pop();
                if let Some(output) = elf.outputs.get(&port) {
                    output.write(top);
                    event = Some(Event::Write(port));
                } else {
                    log::warn!("Elf {:?} writes to unused port {port:?}", elf.name);
                }
            }
            Read(slot) => {
                elf.stack.push(elf.sleeve[slot as usize]);
            }
            Write(slot) => {
                elf.sleeve[slot as usize] = elf.top_val(0)?;
                elf.stack.pop();
            }
            StackLen => {
                elf.stack.push(elf.stack.len() as Int);
            }
            Hammock => {
                elf.finished = true;
            }
        };

        if elf.finished {
            event = Some(Event::Dequeue);
        }

        log::trace!(
            "elf {} > {:>3} | {:<25}{:?}",
            elf.name,
            elf.ip,
            format!("{:?}", code),
            &elf.stack[elf.stack.len().saturating_sub(10)..]
        );

        _ = _g;
        elf.ip = next_ip;
        Ok(event)
    }

    fn flush_outs(&mut self) {
        for f in self.out_files.iter_mut() {
            while let Ok(v) = f.pipe.try_read() {
                let c = v as u8 as char; // TODO: better encoding
                write!(&mut f.writer, "{c}").unwrap();
            }
        }
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

    pub fn top_idx(&self, from_top: usize) -> Result<usize, ECode> {
        let stack_len = self.stack.len();
        match from_top < stack_len {
            true => Ok(stack_len - from_top - 1),
            false => Err(ECode::InvalidIndex(from_top)),
        }
    }
    pub fn top_val(&self, from_top: usize) -> Result<Int, ECode> {
        Ok(self.stack[self.top_idx(from_top)?])
    }
}

impl Op {
    fn invoke(&self, a: i64, b: i64) -> Result<Int, ECode> {
        return Ok(match self {
            Op::Add => a + b,
            Op::Sub => a - b,
            Op::Mul => a * b,
            Op::Div if b == 0 => return Err(ECode::DivisionByZero),
            Op::Div => a / b,
            Op::Mod => a % b,
        });
    }
}

impl<'u> fmt::Display for Error<'u> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Elf encountered a problem and doesn't know what to do: ")?;
        match self.code {
            ECode::InvalidIndex(i) => writeln!(f, "invalid index {i}"),
            ECode::InvalidInstr => writeln!(f, "invalid instruction"),
            ECode::DivisionByZero => writeln!(f, "division by zero"),
        }?;

        if let Some(room) = self.room.map(|i| &self.unit.rooms[i]) {
            let (x, y) = room.ip_to_tile[&self.ip].clone();
            write!(f, "  pos=({x},{y})")?;
        }
        writeln!(f, "  stack: {:?}", self.stack)?;

        Ok(())
    }
}
