use std::{collections::HashMap, fmt, sync::Arc};

pub use ir::*;
pub use pipe::*;

pub struct Runtime {
    //rooms: Vec<Arc<Room>>,
    elves: HashMap<ElfId, Elf>,
    max_elf_id: ElfId,
}

mod ir;
mod pipe;

#[derive(Debug)]
pub struct ElfError {
    code: Error,
    instr: usize,
    stack: Vec<Int>,
    room: Arc<Room>,
}

enum Event {
    Yield,
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

impl Elf {
    fn step(&mut self) -> Result<Option<Event>, Error> {
        use ir::Instr::*;
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
            Push(value) => self.stack.push(value),
            Dup(i) => self.stack.push(self.top_val(i)?),
            Erase(i) => {
                self.stack.remove(self.top_idx(i)?);
            }
            Tuck(i) => {
                let index = self.top_idx(i)?;
                let top = self.stack.pop().unwrap();
                self.stack.insert(index, top);
            }
            Swap(i) => {
                let top_i = self.top_idx(0)?;
                let index = self.top_idx(i)?;
                self.stack.swap(top_i, index);
            }
            Jmp(_) | IfPos(_) | IfNz(_) => return Err(Error::InvalidInstr),
            JmpPtr(target) => next_instr = target,
            IfPosPtr(target) => {
                if self.top_val(0)? > 0 {
                    next_instr = target
                }
                self.stack.pop();
            }
            IfNzPtr(target) => {
                if self.top_val(0)? != 0 {
                    next_instr = target
                }
                self.stack.pop();
            }
            Arith(op) => {
                let result = op.invoke(self.top_val(1)?, self.top_val(0)?)?;
                self.stack.pop();
                self.stack.pop();
                self.stack.push(result);
            }
            ArithC(op, c) => {
                let result = op.invoke(self.top_val(0)?, c)?;
                self.stack.pop();
                self.stack.push(result);
            }
            In(port) => match self.inputs.get_mut(&port).map(|p| p.try_read()) {
                Some(Ok(value)) => self.stack.push(value),
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
                    self.stack.pop();
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
            &self.stack[self.stack.len().saturating_sub(10)..]
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

impl Runtime {
    pub fn new(elf_list: Vec<Elf>) -> Self {
        let mut rooms = vec![];
        let mut elves = HashMap::new();
        for (i, mut elf) in elf_list.into_iter().enumerate() {
            let nid = i.wrapping_mul(10007).wrapping_add(101) % ELF_NAMES.len();
            elf.name = Some(ELF_NAMES[nid]);

            rooms.push(elf.room.clone());
            elves.insert(i as ElfId, elf);
        }
        Self {
            max_elf_id: elves.keys().max().cloned().unwrap_or(1),
            //rooms,
            elves,
        }
    }

    pub fn run_loop(&mut self) -> Result<(), ElfError> {
        let mut elf_id: ElfId = 0;

        while self.elves.len() > 0 {
            let elf = 'block: {
                for i in 0..=self.max_elf_id {
                    let next_id = (elf_id + i) % (self.max_elf_id + 1);
                    if let Some(elf) = self.elves.get_mut(&next_id) {
                        elf_id = next_id;
                        break 'block elf;
                    }
                }
                panic!()
            };

            match elf.step() {
                Ok(None) => {}
                Ok(Some(Event::Write(port, value))) => {
                    if let Some(monitor) = self.elves[&elf_id].outputs[&port].monitor.clone() {
                        monitor(self, value);
                    }
                    elf_id += 1;
                }
                Ok(Some(Event::Yield)) => {
                    elf_id += 1;
                }
                Err(e) => {
                    return Err(ElfError {
                        code: e,
                        instr: elf.instr,
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

impl fmt::Display for ElfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Elf encountered a problem and doesn't know what to do: ")?;
        match self.code {
            Error::InvalidIndex(i) => writeln!(f, "invalid index {i}"),
            Error::InvalidInstr => writeln!(f, "invalid instruction"),
            Error::DivisionByZero => writeln!(f, "division by zero"),
        }?;

        writeln!(f, "  stack: {:?}", self.stack)?;

        let program_peek = self.room.program.iter().enumerate();
        for (i, instr) in program_peek.skip(self.instr.saturating_sub(2)).take(5) {
            let caret = if i == self.instr { '>' } else { ' ' };
            writeln!(f, "{:>5} | {instr:?}", format!("{caret} {i}"))?;
        }

        Ok(())
    }
}
