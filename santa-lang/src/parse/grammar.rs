use std::fmt::Debug;

use peg::str::LineCol;

use crate::{Instr, Int};

use super::*;

pub type Error = peg::error::ParseError<LineCol>;
pub type Result<T> = std::result::Result<T, Error>;

// pub struct Span<T> {
//     start: usize,
//     end: usize,
//     t: T,
// }

pub fn parse(input: &str) -> Result<TranslationUnit<&str>> {
    log::trace!("parsing\n{input:?}");

    let mut unit = TranslationUnit::default();
    santasm::unit(input, &mut unit).map(|_| unit)
}

#[cfg(test)]
pub(crate) fn parse_plan(input: &str) -> Result<ShopBlock<&str>> {
    santasm::plan(input)
}

// Top-level rules have side effects, they populate the translation unit.
// Low-level rules should be pure.
peg::parser! { grammar santasm() for str {

    pub rule unit(u: &mut TranslationUnit<&'input str>)
        = (s:shop() { u.workshops.insert(s.name, s); }) unit(u)
        / santa_block(u) unit(u)
        / _ {}

    pub rule shop() -> Shop<&'input str>
        = word("workshop") name:ident() ":" _ blocks:shop_block()* _ ";" _ { Shop { name, blocks } }

    rule shop_block() -> ShopBlock<&'input str>
        = word("floorplan") ":" p:plan()? _ ";" _ { p.unwrap_or(ShopBlock::empty_plan()) }

    pub rule plan() -> ShopBlock<&'input str>
        = (__ NL()) r1:plan_row(None) rs:plan_row(Some(&r1))* _ { ShopBlock::make_plan(r1, rs) }

    rule plan_row(first: Option<&PlanRow<&'input str>>) -> PlanRow<&'input str>
        = (__ NL())* i:indent_any() tiles:(plan_tile() ** " ") NL() {? PlanRow { indent: i, tiles }.matches(first) }

    rule plan_tile() -> Tile<&'input str>
        = ("  " / "..") { Tile::Empty }
        / "m" d:dir() { Tile::Move(d) }
        / "e" d:dir() { Tile::Elf(d) }
        / "P" n:tile_param() { Tile::Instr(Instr::Push(n)) }
        / d1:digit() d0:digit() { Tile::Instr(Instr::Push(d1 as Int * 10 + d0 as Int)) }
        / "D" d:digit() { Tile::Instr(Instr::Dup(d)) }
        / "E" d:digit() { Tile::Instr(Instr::Erase(d)) }
        / "S" d:digit() { Tile::Instr(Instr::Swap(d)) }
        / "Hm" { Tile::Instr(Instr::Hammock) }
        / "?=" { Tile::IsZero }
        / "?>" { Tile::IsPos }
        / "?<" { Tile::IsNeg }
        / op:arith_op() "_" { Tile::Instr(Instr::Arith(op)) }
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
        = word("setup") shop:ident() word("for") h:helper_type() name:ident()? "(" stack:numInt()* ")"
            { match h {
                HelperType::Elf => ToDo::SetupElf { name, stack, shop },
                HelperType::Raindeer => todo!("raindeer"),
            } }
        / word("setup") src:helper_port() "->" dst:helper_port()
            { ToDo::Connect { src, dst } }
        / word("monitor") target:helper_port() ":" _ ts:todo_item()* _ ";" _
            { ToDo::Monitor { target, todos: ts } }
        / word("receive") vs:list(<ident()>) src:(word("from") p:helper_port() {p})?
            { ToDo::Receive { vars: vs, src } }
        / word("send") vs:list(<val_expr()>) dst:(word("to") p:helper_port() {p})?
            { ToDo::Send { values: vs, dst } }

    rule helper_type() -> HelperType
        = word("elf") { HelperType::Elf }
        // word("raindeer") { HelperType::Raindeer }

    rule helper_port() -> (&'input str, char)
        = name:ident() "." _ port:tile_ch() _ { (name, port) }

    rule val_expr() -> Expr<&'input str>
        = v:numInt() { Expr::Number(v) }
        / id:ident() { Expr::Var(id) }

    rule list<T>(x: rule<T>) -> Vec<T>
        = "(" many:x() ** _ ")" { many }
        / one:x() { vec![one] }

    rule word(expect: &'static str) -> &'input str
        = i:alnum() {? if i == expect { Ok(i) } else { Err(expect)} }

    rule num128() -> i128 = _ n:$(['0'..='9']+) _ {? n.parse().or(Err("i128")) }
    rule numInt() -> Int = _ n:$(['0'..='9']+) _ {? n.parse().or(Err("Int")) }

    rule ident() -> &'input str
        = alnum()
        / expected!("identifier")

    rule alnum() -> &'input str = _ s:$(quiet!{['a'..='z'|'A'..='Z'|'_']['a'..='z'|'A'..='Z'|'_'|'0'..='9']*}) _ {s}

    // rule loc<T>(x: rule<T>) -> (T, Loc)
    //     = start:position!() t:x() end:position!() { Span { start, end, t } }

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
    rule NL() = "\n" / "\r\n"

    rule __ -> usize = s:$(quiet!{[' ' | '\r' | '\x0b' | '\x0c' | '\t']*}) { s.len() }
    rule _ -> usize = s:$(quiet!{[' ' | '\r' | '\x0b' | '\x0c' | '\t' | '\n']*}) { s.len() }
}}

enum HelperType {
    Elf,
    Raindeer,
}

impl<S: Clone + Debug> ShopBlock<S> {
    fn empty_plan() -> Self {
        Self::Plan {
            width: 0,
            height: 0,
            map: vec![],
        }
    }
    fn make_plan(r1: PlanRow<S>, mut rows: Vec<PlanRow<S>>) -> Self {
        rows.insert(0, r1);

        for r in rows.iter() {
            log::trace!("row {r:?}");
        }

        let leftmost_ind = rows.iter().map(|row| row.indent.1).min().unwrap();

        let width = rows
            .iter()
            .map(|row| row.tiles.len() + (row.indent.1 - leftmost_ind) / 3)
            .max()
            .unwrap();

        let height = rows.len();
        let mut map = Vec::new();
        map.resize(width * height, Tile::Empty);

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
        let shop = santasm::shop(
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

        let expected = Shop {
            name: "test",
            blocks: vec![ShopBlock::Plan {
                width: 0,
                height: 0,
                map: vec![],
            }],
        };

        pretty_assertions::assert_eq!(shop, expected);
    }

    #[test]
    fn parse_empty_tiles() {
        let shop = santasm::shop(
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

        let expected = Shop {
            name: "test",
            blocks: vec![ShopBlock::Plan {
                width: 3,
                height: 2,
                map: vec![
                    Tile::Empty,
                    Tile::Empty,
                    Tile::Empty,
                    Tile::Empty,
                    Tile::Empty,
                    Tile::Empty,
                ],
            }],
        };

        pretty_assertions::assert_eq!(shop, expected);
    }

    #[test]
    fn parse_shifted_indent() {
        let shop = santasm::shop(
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

        let expected = Shop {
            name: "test",
            blocks: vec![ShopBlock::Plan {
                width: 3,
                height: 2,
                map: vec![
                    Tile::Elf(Direction::Right),
                    Tile::Empty,
                    Tile::Move(Direction::Down),
                    Tile::Empty,
                    Tile::Empty,
                    Tile::Instr(runtime::Instr::Push(0)),
                ],
            }],
        };

        pretty_assertions::assert_eq!(shop, expected);
    }

    #[test]
    fn parse_weird_hm() {
        crate::logger::init(log::LevelFilter::Trace);
        let shop = santasm::shop(
            "
                workshop weird_Hm:
                    floorplan:
                    mv    S1 -1 m<
                          m>       Hm
                 e> m> D1 ?>    S1
                          m> D0 ?>
                          m^ -1 m<
                    ;
                ;
            ",
        );

        let shop = match shop {
            Err(e) => panic!("{e}"),
            Ok(s) => s,
        };

        use {
            crate::{Instr::*, Op::*},
            Direction::*,
            Tile::*,
        };
        let expected = Shop {
            name: "weird_Hm",
            blocks: vec![ShopBlock::Plan {
                width: 7,
                height: 5,
                #[rustfmt::skip]
                map: vec![
                    Empty, Move(Down), Empty, Instr(Swap(1)), Instr(ArithC(Sub, 1)), Move(Left), Empty,
                    Empty, Empty, Empty, Move(Right), Empty, Empty, Instr(Hammock),
                    Elf(Right), Move(Right), Instr(Dup(1)), IsPos, Empty, Instr(Swap(1)), Empty,
                    Empty, Empty, Empty, Move(Right), Instr(Dup(0)), IsPos, Empty,
                    Empty, Empty, Empty, Move(Up), Instr(ArithC(Sub, 1)), Move(Left), Empty,
                ],
            }],
        };

        pretty_assertions::assert_eq!(shop, expected);
    }

    #[test]
    fn parse_santa_block() {
        let mut tu = TranslationUnit::default();
        let r = santasm::santa_block(
            "
                Santa will:
                    setup toys for elf Josh (1 2 3)
                    setup prod for elf Bob ()

                    setup Josh.a -> Bob.1

                    monitor Josh.b:
                        receive (a b)
                        receive x
                        send (a 1234)
                        setup sweets for elf Alice (4 5)
                    ;
                ;
            ",
            &mut tu,
        );

        if let Err(e) = r {
            panic!("{e}")
        };

        let expected = TranslationUnit {
            workshops: Default::default(),
            todos: vec![
                ToDo::SetupElf {
                    shop: "toys",
                    name: Some("Josh".into()),
                    stack: vec![1, 2, 3],
                },
                ToDo::SetupElf {
                    shop: "prod",
                    name: Some("Bob".into()),
                    stack: vec![],
                },
                ToDo::Connect {
                    src: ("Josh".into(), 'a'),
                    dst: ("Bob".into(), '1'),
                },
                ToDo::Monitor {
                    target: ("Josh".into(), 'b'),
                    todos: vec![
                        ToDo::Receive {
                            src: None,
                            vars: vec!["a", "b"],
                        },
                        ToDo::Receive {
                            src: None,
                            vars: vec!["x"],
                        },
                        ToDo::Send {
                            dst: None,
                            values: vec![Expr::Var("a"), Expr::Number(1234)],
                        },
                        ToDo::SetupElf {
                            shop: "sweets",
                            name: Some("Alice".into()),
                            stack: vec![4, 5],
                        },
                    ],
                },
            ],
        };

        pretty_assertions::assert_eq!(expected, tu);
    }

    #[test]
    fn unit_parse_empty() {
        let mut u = TranslationUnit::default();
        santasm::unit("    \n\n  \r\n\r\n   \t  ", &mut u).unwrap();
    }

    #[test]
    fn unit_parse_empty_shops() {
        let mut u = TranslationUnit::default();
        santasm::unit(
            "

            workshop w1:; workshop w2:;

            workshop w3:;

            ",
            &mut u,
        )
        .unwrap();
    }
}
