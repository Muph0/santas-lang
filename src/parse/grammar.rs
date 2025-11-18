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

pub fn parse(input: &str) -> Result<TranslationUnit> {
    let unit = match elf_parse::unit(input) {
        Ok(unit) => unit,
        Err(e) => return Err(Error::Parse(e)),
    };

    Ok(unit)
}

peg::parser! { grammar elf_parse() for str {
    pub rule unit() -> TranslationUnit<'input> = {todo!()}

    pub rule shop() -> Shop<'input>
        = kw("workshop") name:ident() ":" _ block:shop_block() _ ";" _ { Shop { name, block } }

    rule shop_block() -> ShopBlock<'input>
        = kw("floorplan") ":" p:plan()? _ ";" _ { p.unwrap_or(ShopBlock::empty_plan()) }

    rule plan() -> ShopBlock<'input>
        = (__ "\n") r1:plan_row(None) rs:plan_row(Some(&r1))* { ShopBlock::make_plan(r1, rs) }

    rule plan_row(first: Option<&PlanRow>) -> PlanRow<'input>
        = (__ "\n")* i:indent_any() tiles:(plan_tile() ** " ") "\n" {? PlanRow { indent: i, tiles }.matches(first) }

    rule plan_tile() -> Tile<'input>
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

    rule digit() -> usize = d:['0'..='9'] { d as usize - '0' as usize }

    rule tile_ch() -> char = [^'\n']

    rule kw(expect: &'static str)
        = i:ident() {? if i == expect { Ok(()) } else { Err(expect)} }

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

impl<'i> ShopBlock<'i> {
    fn empty_plan() -> Self {
        Self::Plan {
            width: 0,
            height: 0,
            map: vec![],
        }
    }
    fn make_plan(r1: PlanRow<'i>, mut rows: Vec<PlanRow<'i>>) -> Self {
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

impl<'i> PlanRow<'i> {
    fn matches(self, expect: Option<&PlanRow>) -> std::result::Result<Self, &'static str> {
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
        let shop = elf_parse::shop(
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
            ShopBlock::Program(instrs) => panic!(),
            ShopBlock::Plan { width, height, map } => {
                assert_eq!(width, 0);
                assert_eq!(height, 0);
                assert_eq!(map.len(), 0);
            }
        }
    }

    #[test]
    fn parse_empty_tiles() {
        let shop = elf_parse::shop(
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
            ShopBlock::Program(instrs) => panic!(),
            ShopBlock::Plan { width, height, map } => {
                assert_eq!(width, 3);
                assert_eq!(height, 2);
                map.iter().for_each(|t| assert_eq!(t, &Tile::Empty));
            }
        }
    }

    #[test]
    fn parse_shifted_indent() {
        let shop = elf_parse::shop(
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
            ShopBlock::Program(instrs) => panic!(),
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
}
