use peg::{RuleResult, str::LineCol};

use crate::{Instr, Int};

use super::*;

#[derive(Debug)]
pub enum Error {
    Parse(peg::error::ParseError<LineCol>),
}
pub type Result<T> = std::result::Result<T, Error>;

pub struct Loc<T> {
    start: usize,
    end: usize,
    t: T,
}

pub fn parse(name: String, input: &str) -> Result<TranslationUnit<&str>> {
    let mut unit = TranslationUnit {
        name: name,
        ..Default::default()
    };

    match elf::unit(input, &mut unit) {
        Ok(_) => {}
        Err(e) => return Err(Error::Parse(e)),
    };

    Ok(unit)
}

// Top-level rules have side effects, they populate the translation unit.
// Low-level rules should be pure.
peg::parser! { grammar elf() for str {

    pub rule unit(u: &mut TranslationUnit<&'input str>)
        = (s:shop() { u.workshops.insert(s.name, s); }) unit(u)
        // (s:santa_block() { u.santa }) TODO

    pub rule shop() -> Shop<&'input str>
        = word("workshop") name:ident() ":" _ block:shop_block() _ ";" _ { Shop { name, block } }

    rule shop_block() -> ShopBlock<&'input str>
        = word("floorplan") ":" p:plan()? _ ";" _ { p.unwrap_or(ShopBlock::empty_plan()) }

    rule plan() -> ShopBlock<&'input str>
        = (__ "\n") r1:plan_row(None) rs:plan_row(Some(&r1))* { ShopBlock::make_plan(r1, rs) }

    rule plan_row(first: Option<&PlanRow<&'input str>>) -> PlanRow<&'input str>
        = (__ "\n")* i:indent_any() tiles:(plan_tile() ** " ") "\n" {? PlanRow { indent: i, tiles }.matches(first) }

    rule plan_tile() -> Tile<&'input str>
        = ("  " / "..") { Tile::Empty }
        / "m" d:dir() { Tile::Move(d) }
        / "e" d:dir() { Tile::Elf(d) }
        / "P" n:tile_param() { Tile::Instr(Instr::Push(n)) }
        / "D" d:digit() { Tile::Instr(Instr::Dup(d)) }
        / "E" d:digit() { Tile::Instr(Instr::Erase(d)) }
        / op:arith_op() "." { Tile::Instr(Instr::Arith(op)) }
        / op:arith_op() d:digit() { Tile::Instr(Instr::ArithC(op, d as Int)) }
        // s:$(tile_ch()*<2>) { Tile::Unknown(s) }

    rule dir() -> Direction
        = "^" { Direction::Up }
        / "v" { Direction::Down }
        / "<" { Direction::Left }
        / ">" { Direction::Right }

    rule arith_op() -> runtime::Op
        = "+" { runtime::Op::Add }
        / "-" { runtime::Op::Sub }
        / "*" { runtime::Op::Mul }
        / "/" { runtime::Op::Div }
        / "%" { runtime::Op::Mod }

    rule tile_param() -> Int
        = d:digit() { d as Int }
        / c:tile_ch() { c as Int }

    rule tile_ch() -> char = [^'\n']
    rule digit() -> usize = d:['0'..='9'] { d as usize - '0' as usize }

    pub rule santa_block(u: &mut TranslationUnit<&'input str>)
        = word("Santa") word("will") ":" _ ts:todo_item()* _ ";" _ {
            u.todos.extend(ts);
        }

    rule todo_item() -> ToDo<&'input str>
        = word("setup") w:ident() word("for") h:helper_type() name:ident()? "(" stack:((n:number() {n as Int}) ** _) ")" {
            match h {
                HelperType::Elf => ToDo::SendElf { name, stack },
                HelperType::Raindeer => todo!("raindeer"),
            }
        }
        / word("setup") src:helper_port() "->" dst:helper_port() {
            ToDo::Connect { src, dst, }
        }

    rule helper_type() -> HelperType
        = word("elf") { HelperType::Elf }
        // word("raindeer") { HelperType::Raindeer }

    rule helper_port() -> (&'input str, char)
        = name:ident() "." _ port:tile_ch() _ { (name, port) }


    rule word(expect: &'static str) -> &'input str
        = i:ident() {? if i == expect { Ok(i) } else { Err(expect)} }

    rule number() -> i128
        = _ n:$(['0'..='9']+) _ {? n.parse().or(Err("number")) }

    rule ident() -> &'input str
        = _ s:$(quiet!{['a'..='z'|'A'..='Z'|'_']['a'..='z'|'A'..='Z'|'_'|'0'..='9']*}) _ { s }
        / expected!("identifier")

    rule loc<T>(x: rule<T>) -> Loc<T>
        = start:position!() t:x() end:position!() { Loc { start, end, t } }

    rule indent(expect: Option<Indent>) -> Indent
        = i:indent_any() {?
            match expect {
                None => Ok(i),
                Some(ex) if i == ex => Ok(i),
                Some(_) => Err("same indentation"),
            }
        }

    rule indent_any() -> Indent
        = s:$(quiet!{[' ']*})  ![' '|'\t'] { (' ', s.len()) }
        / s:$(quiet!{['\t']+}) ![' '|'\t'] { ('\t', s.len()) }
        / expected!("uniform indentation")

    rule todo() = { todo!() }

    rule __ -> usize = s:$(quiet!{[' ' | '\t']*}) { s.len() }
    rule _ -> usize = s:$(quiet!{[' ' | '\n' | '\t']*}) { s.len() }
}}

enum HelperType {
    Elf,
    Raindeer,
}

impl<S: Clone> ShopBlock<S> {
    fn empty_plan() -> Self {
        Self::Plan {
            width: 0,
            height: 0,
            map: vec![],
        }
    }
    fn make_plan(r1: PlanRow<S>, mut rows: Vec<PlanRow<S>>) -> Self {
        rows.insert(0, r1);

        let width = rows.iter().map(|row| row.tiles.len()).max().unwrap();
        let height = rows.len();
        let mut map = Vec::new();
        map.resize(width * height, Tile::Empty);

        let leftmost_ind = rows.iter().map(|row| row.indent.1).min().unwrap();

        for (y, row) in rows.into_iter().enumerate() {
            for (x_padded, tile) in row.tiles.into_iter().enumerate() {
                let x = x_padded + (row.indent.1 - leftmost_ind) / 3;
                map[x + y * width] = tile;
            }
        }

        Self::Plan { width, height, map }
    }
}

impl<S> PlanRow<S> {
    fn matches(self, expect: Option<&PlanRow<S>>) -> std::result::Result<Self, &'static str> {
        let ind = self.indent;
        match expect {
            None => Ok(self),
            Some(other) if ind == other.indent => Ok(self),
            Some(o) if ind.0 == ' ' && ind.1.abs_diff(o.indent.1) % 3 == 0 => Ok(self),
            Some(_) => Err("row with same indentation"),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::Instr;

    use super::*;

    #[test]
    fn parse_empty_shop() {
        let shop = elf::shop(
            "
                workshop test:
                    floorplan: ;
                ;
            ",
        );

        let shop = match shop {
            Err(e) => panic!("{e}"),
            Ok(s) => s,
        };

        match shop.block {
            ShopBlock::Program(_) => panic!(),
            ShopBlock::Plan { width, height, map } => {
                assert_eq!(width, 0);
                assert_eq!(height, 0);
                assert_eq!(map.len(), 0);
            }
        }
    }

    #[test]
    fn parse_empty_tiles() {
        let shop = elf::shop(
            "
                workshop test:
                    floorplan:
                    .. .. ..
                    .. ..
                    ;
                ;
            ",
        );

        let shop = match shop {
            Err(e) => panic!("{e}"),
            Ok(s) => s,
        };

        match shop.block {
            ShopBlock::Program(_) => panic!(),
            ShopBlock::Plan { width, height, map } => {
                assert_eq!(width, 3);
                assert_eq!(height, 2);
                map.iter().for_each(|t| assert_eq!(t, &Tile::Empty));
            }
        }
    }

    #[test]
    fn parse_shifted_indent() {
        let shop = elf::shop(
            "
                workshop test:
                    floorplan:
                    e> .. mv
                       .. P0
                    ;
                ;
            ",
        );

        let shop = match shop {
            Err(e) => panic!("{e}"),
            Ok(s) => s,
        };

        match shop.block {
            ShopBlock::Program(_) => panic!(),
            ShopBlock::Plan { width, height, map } => {
                assert_eq!(width, 3);
                assert_eq!(height, 2);
                assert_eq!(map[0], Tile::Elf(Direction::Right));
                assert_eq!(map[2], Tile::Move(Direction::Down));
                assert_eq!(map[4], Tile::Empty);
                assert_eq!(map[5], Tile::Instr(Instr::Push(0)));
            }
        }
    }

    #[test]
    fn parse_santa_block() {
        let mut tu = TranslationUnit::default();
        let r = elf::santa_block(
            "
                Santa will:
                    setup toys for elf Josh (1 2 3)
                    setup toys for elf Bob ()

                    setup Josh.a -> Bob.1

                    monitor Josh.b:
                        setup toys for elf Alice (4 5)
                    ;
                ;
            ",
            &mut tu,
        );

        if let Err(e) = r {
            panic!("{e}")
        };

        // Now check the todos inside the TranslationUnit
        assert_eq!(tu.todos.len(), 4);

        match &tu.todos[0] {
            ToDo::SendElf { name, stack } => {
                assert_eq!(*name, Some("Josh"));
                assert_eq!(*stack, vec![1, 2, 3]);
            }
            _ => panic!("expected SendElf for Josh"),
        }

        match &tu.todos[1] {
            ToDo::SendElf { name, stack } => {
                assert_eq!(*name, Some("Bob"));
                assert!(stack.is_empty());
            }
            _ => panic!("expected SendElf for Bob"),
        }

        match &tu.todos[2] {
            ToDo::Connect { src, dst } => {
                assert_eq!(*src, ("Josh", 'a'));
                assert_eq!(*dst, ("Bob", '1'));
            }
            _ => panic!("expected Connect"),
        }

        match &tu.todos[3] {
            ToDo::Monitor {
                target,
                port,
                todos,
            } => {
                assert_eq!(*target, "Josh");
                assert_eq!(*port, 'b');
                assert_eq!(todos.len(), 1);
                match &todos[0] {
                    ToDo::SendElf { name, stack } => {
                        assert_eq!(*name, Some("Alice"));
                        assert_eq!(*stack, vec![4, 5]);
                    }
                    _ => panic!("expected nested SendElf for Alice"),
                }
            }
            _ => panic!("expected Monitor"),
        }
    }
}
