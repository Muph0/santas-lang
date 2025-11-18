use peg::{RuleResult, str::LineCol};

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
        = kw("workshop") name:ident() ":" _ block:shop_block() _ "." _ { Shop { name, block } }

    rule shop_block() -> ShopBlock<'input>
        = kw("floorplan") ":" __ "\n" p:plan() _ "." _ { p }

    rule plan() -> ShopBlock<'input>
        = r1:plan_row(None) rs:plan_row(Some(&r1))* { ShopBlock::make_plan(r1, rs) }

    rule plan_row(first: Option<&PlanRow>) -> PlanRow<'input>
        = i:indent_any() tiles:(plan_tile() ** " ") "\n" {? PlanRow { indent: i, tiles }.matches(first) }

    rule plan_tile() -> Tile<'input>
        = ("  " / "..") { Tile::Empty }
        / #{|input, pos|
            if pos + 2 < input.len() {
                RuleResult::Matched(2, Tile::Unknown(&input[pos..pos+2]))
            } else {
                RuleResult::Failed
            }
        }

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
        = s:$([' ']*)  ![' '|'\t'] { (' ', s.len()) }
        / s:$(['\t']+) ![' '|'\t'] { ('\t', s.len()) }
        / expected!("uniform indentation")

    rule todo() = { todo!() }

    rule __ -> usize = s:$(quiet!{[' ' | '\t']*}) { s.len() }
    rule _ -> usize = s:$(quiet!{[' ' | '\n' | '\t']*}) { s.len() }
}}

impl<'i> ShopBlock<'i> {
    fn make_plan(r1: PlanRow<'i>, mut rows: Vec<PlanRow<'i>>) -> Self {
        rows.insert(0, r1);

        let width = rows.iter().map(|row| row.tiles.len()).max().unwrap();
        let height = rows.len();
        let mut map = Vec::new();
        map.resize(width * height, Tile::Empty);

        for (y,row) in rows.into_iter().enumerate() {
            for (x, tile) in row.tiles.into_iter().enumerate() {
                map[x + y * width] = tile;
            }
        }

        Self::Plan {
            width,
            height,
            map,
        }
    }
}

impl<'i> PlanRow<'i> {
    fn matches(self, expect: Option<&PlanRow>) -> std::result::Result<Self, &'static str> {
        match expect {
            None => Ok(self),
            Some(other) if self.indent != other.indent => Err("row with same indentation"),
            Some(_) => Ok(self),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_empty_shop() {
        let shop = elf_parse::shop(
            "
                workshop test:
                    floorplan:
                    .. .. ..
                    .. ..
                    .
                .
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
            },
        }
    }
}
