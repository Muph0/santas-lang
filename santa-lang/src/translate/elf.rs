use std::collections::{HashMap, VecDeque};

use crate::{
    Instr,
    parse::{Direction, Tile},
    translate::{ECode, Error, loc::SourceStr},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ElfState {
    pub x: usize,
    pub y: usize,
    pub dir: Direction,
}

impl ElfState {
    pub fn new(x: usize, y: usize, dir: Direction) -> Self {
        Self { x, y, dir }
    }

    pub fn step_fwd(mut self) -> Self {
        use Direction::*;
        match self.dir {
            Up => self.y = self.y.wrapping_sub(1),
            Down => self.y += 1,
            Left => self.x = self.x.wrapping_sub(1),
            Right => self.x += 1,
        }
        self
    }

    pub fn with_dir(self, dir: Direction) -> Self {
        Self {
            x: self.x,
            y: self.y,
            dir,
        }
    }

    /// Turn left, then move forward.
    pub fn step_left(self) -> Self {
        let turned = Self {
            dir: self.dir.left(),
            ..self
        };
        turned.step_fwd()
    }

    /// Turn right, then move forward.
    pub fn step_right(self) -> Self {
        let turned = Self {
            dir: self.dir.right(),
            ..self
        };
        turned.step_fwd()
    }
}

fn xy(w: usize, h: usize) -> impl Iterator<Item = (usize, usize)> {
    (0..w * h).map(move |i| (i % w, i / w))
}

pub fn translate_plan(
    shop_name: &SourceStr,
    plan: (usize, usize, &[Tile<SourceStr>]),
    errors: &mut Vec<Error>,
) -> Option<Vec<Instr>> {
    let (w, h, tiles) = plan;

    // find start
    let mut elf_starts = xy(w, h).filter_map(|(x, y)| {
        tiles[x + y * w]
            .as_elf_start()
            .map(|d| ElfState::new(x, y, d))
    });
    let Some(elf_start) = elf_starts.next() else {
        errors.push(Error::at(shop_name, ECode::MissingElfStart));
        return None;
    };
    if elf_starts.next().is_some() {
        errors.push(Error::at(shop_name, ECode::MultipleElfStarts));
        return None;
    }

    // emitted code
    let mut emit = Vec::new();

    // map visited tile to instruction index emitted after that tile
    let mut visited = HashMap::<ElfState, usize>::new();

    // state: elf, and optionally where we came from (to fill in jump target later)
    let mut bfs = VecDeque::<(ElfState, Option<usize>)>::from([(elf_start, None)]);

    while let Some((elf, from)) = bfs.pop_back() {
        if let Some(f) = from {
            log::trace!("pop {elf:?}, from={f:?}");
        } else {
            log::trace!("pop {elf:?}");
        }

        // if we hit a visited tile, emit Jump and bail
        if let Some(iptr) = visited.get(&elf) {
            emit.push(Instr::JmpPtr(*iptr));
            continue;
        }

        if !(elf.x < w && elf.y < h) {
            log::debug!("elf walks into a wall {elf:?}");
            errors.push(Error::at(shop_name, ECode::ElfWallHit(elf.x, elf.y)));
            continue;
        }

        // save current instruction pointer to the tile
        visited.insert(elf, emit.len());

        // if we jumped in from somewhere, now is a good time to fill in the target pointer
        // because we are AT the target pointer
        if let Some(from) = from {
            let emit_len = emit.len();
            match &mut emit[from] {
                Instr::JmpPtr(target) | Instr::IfPosPtr(target) | Instr::IfNzPtr(target) => {
                    *target = emit_len;
                }
                _ => panic!("bug: jumped from non-jump instr during translation"),
            }
        }

        let mut next = elf.step_fwd();
        let idx = elf.x + elf.y * w;

        log::trace!("tile {:?}", tiles[idx]);
        match &tiles[idx] {
            Tile::Empty | Tile::Elf(_) => {}
            Tile::Move(dir) => next = elf.with_dir(*dir).step_fwd(),
            Tile::IsZero => {
                let true_elf = elf.step_right();
                let false_elf = elf.step_left();
                next = true_elf; // true now, false branch will be processed later
                bfs.push_back((false_elf, Some(emit.len()))); // we save "where from" on the stack because
                emit.push(Instr::IfNzPtr(emit.len() + 1)); // we dont know where to jump yet (default to here+1=nop)
            }
            Tile::IsNeg => {
                next = elf.step_right();
                emit.push(Instr::ArithC(crate::Op::Add, 1));
                bfs.push_back((elf.step_left(), Some(emit.len())));
                emit.push(Instr::IfPosPtr(emit.len() + 1));
            }
            Tile::IsPos => {
                next = elf.step_left();
                bfs.push_back((elf.step_right(), Some(emit.len())));
                emit.push(Instr::IfPosPtr(emit.len() + 1));
            }
            Tile::Instr(instr) => {
                emit.push(*instr);
                if *instr == Instr::Hammock {
                    continue;
                }
            }
            Tile::Unknown(s) => {
                errors.push(Error::at(shop_name, ECode::UnknownTile(s.clone())));
            }
        }

        bfs.push_back((next, None));
    }

    Some(emit)
}

impl<S> Tile<S> {
    fn as_elf_start(&self) -> Option<Direction> {
        match self {
            Tile::Elf(direction) => Some(*direction),
            _ => None,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::translate::{Loc, loc::LineMap};
    use crate::{Op, parse::parse_plan};

    fn check_program(tiles: &str, expect_program: &[Instr]) {
        crate::logger::init(log::LevelFilter::Trace);
        let shop_name = SourceStr {
            source_name: "test_file".into(),
            string: "test_shop".into(),
            loc: Loc::new(1, 1, 1),
        };
        let map = LineMap::new(&shop_name.source_name, tiles);
        let result = parse_plan(tiles);

        if let Err(e) = result {
            panic!("{e}");
        }

        let plan = result.unwrap().convert(&|s| map.map_slice(s));
        let mut errors = Vec::new();

        let program = translate_plan(&shop_name, plan.as_plan().unwrap(), &mut errors).unwrap();

        if !errors.is_empty() {
            errors.iter().for_each(|e| println!("{e}"));
        }

        assert!(errors.is_empty());
        pretty_assertions::assert_eq!(expect_program, &program);
    }

    use Instr::*;

    #[test]
    fn translate_simple() {
        check_program(
            "
            e> P1 .. mv
            Hm       m<
            ",
            &[Push(1), Hammock],
        );
    }

    #[test]
    fn translate_ifz() {
        check_program(
            "
               m> P2 mv
            e> ?=    m> Hm
               m> P1 m^
            ",
            &[IfNzPtr(3), Push(1), Hammock, Push(2), JmpPtr(2)],
        );
    }

    #[test]
    fn translate_if_pos() {
        check_program(
            "
               m> P2 mv
            e> ?>    m> Hm
               m> P1 m^
            ",
            &[IfPosPtr(3), Push(2), Hammock, Push(1), JmpPtr(2)],
        );
    }

    #[test]
    fn translate_if_neg() {
        check_program(
            "
               m> P2 mv
            e> ?<    m> Hm
               m> P1 m^
            ",
            &[
                ArithC(Op::Add, 1),
                IfPosPtr(4),
                Push(1),
                Hammock,
                Push(2),
                JmpPtr(3),
            ],
        );
    }

    #[test]
    fn translate_loop_nested() {
        check_program(
            "
               mv    S1 -1 m<
                     m>       Hm
            e> m> D1 ?>    S1
                     m> D0 ?>
                     m^ -1 m<
            ",
            &[
                Dup(1),
                IfPosPtr(3),
                Hammock,
                Dup(0),
                IfPosPtr(9),
                Swap(1),
                ArithC(Op::Sub, 1),
                Swap(1),
                JmpPtr(0),
                ArithC(Op::Sub, 1),
                JmpPtr(3),
            ],
        );
        todo!("check correctness of expected program");
    }
}
