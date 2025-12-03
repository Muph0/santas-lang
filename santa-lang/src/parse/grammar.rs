use std::fmt::Debug;

use peg::str::LineCol;

use crate::ir::{Instr, Int};

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
        = (__ NL())+ r1:plan_row(None) rs:plan_row(Some(&r1))* _ { ShopBlock::make_plan(r1, rs) }

    rule plan_row(first: Option<&PlanRow<&'input str>>) -> PlanRow<&'input str>
        = s:slice(<i:indent_any() ts:(plan_tile() ** " ") {(i,ts)}>) (__ NL())+ {?
            PlanRow { text: s.1, indent: s.0.0, tiles: s.0.1 }.matches(first)
        }

    pub rule plan_tile() -> Tile<&'input str> =
        t:slice(<plan_tile_kind()>) { Tile { text: t.1, kind: t.0 } }

    rule plan_tile_kind() -> TileKind
        = ("  " / "..") { TileKind::Empty }
        / "m" d:dir() { TileKind::Move(d) }
        / "e" d:dir() { TileKind::Elf(d) }
        / "C" c:tile_ch() { TileKind::Instr(Instr::Push(c as Int)) }
        / d1:digit() d0:digit() { TileKind::Instr(Instr::Push(d1 as Int * 10 + d0 as Int)) }
        / "D" d:digit() { TileKind::Instr(Instr::Dup(d)) }
        / "E" d:digit() { TileKind::Instr(Instr::Erase(d)) }
        / "S" d:digit() { TileKind::Instr(Instr::Swap(d)) }
        / "I" d:tile_param() { TileKind::Instr(Instr::In(d as u16)) }
        / "O" d:tile_param() { TileKind::Instr(Instr::Out(d as u16)) }
        / "R" d:digit() { TileKind::Instr(Instr::Read(d as u8)) }
        / "W" d:digit() { TileKind::Instr(Instr::Write(d as u8)) }
        / "Hm" { TileKind::Instr(Instr::Hammock) }
        / "?=" { TileKind::IsZero }
        / "?>" { TileKind::IsPos }
        / "?<" { TileKind::IsNeg }
        / "?s" { TileKind::IsEmpty }
        / "!s" { TileKind::Instr(Instr::StackLen) }
        / "*-" { TileKind::Instr(Instr::ArithC(runtime::Op::Mul, -1)) }
        / op:arith_op() "_" { TileKind::Instr(Instr::Arith(op)) }
        / op:arith_op() d:digit() { TileKind::Instr(Instr::ArithC(op, d as Int)) }
        // s:$(tile_ch()*<2>) { TileKind::Unknown(s) }

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
        = word("setup") shop:ident() word("for") h:helper_type() name:ident()? "(" stack:val_expr()* ")"
            { match h {
                HelperType::Elf => ToDo::SetupElf { name, stack, shop },
                HelperType::Raindeer => todo!("raindeer"),
            } }
        / word("setup") src:connection("STDIN") "->" dst:connection("STDOUT")
            { ToDo::Connect { src, dst } }
        / word("monitor") target:helper_port() ":" _ ts:todo_item()* _ ";" _
            { ToDo::Monitor { target, todos: ts } }
        / word("receive") vs:list(<ident()>) src:(word("from") p:helper_port() {p})?
            { ToDo::Receive { vars: vs, src } }
        / word("send") vs:list(<val_expr()>) dst:(word("to") p:helper_port() {p})?
            { ToDo::Send { values: vs, dst } }
        / word("deliver") e:val_expr() { ToDo::Deliver { e } }


    rule helper_type() -> HelperType
        = word("elf") { HelperType::Elf }
        // word("raindeer") { HelperType::Raindeer }

    rule connection(std: &'static str) -> Connection<&'input str>
        = word("FILE") "(" name:strlit() ")" _ { Connection::File(name) }
        // word(std) { Connection::Std }
        / p:helper_port() { Connection::Port(p.0, p.1) }

    rule helper_port() -> (&'input str, char)
        = name:ident() "." _ port:tile_param() _ { (name, port as u8 as char) }

    rule val_expr() -> Expr<&'input str>
        = v:numInt() { Expr::Number(v) }
        / id:ident() { Expr::Var(id) }

    rule list<T>(x: rule<T>) -> Vec<T>
        = "(" many:x() ** _ ")" { many }
        / one:x() { vec![one] }

    rule word(expect: &'static str) -> &'input str
        = i:alnum() {? if i == expect { Ok(i) } else { Err(expect)} }

    rule strlit() -> &'input str
        = _ "\"" s:$([^'"']*) "\"" _ { s }

    rule num128() -> i128 = _ n:$(['0'..='9']+) _ {? n.parse().or(Err("i128")) }
    rule numInt() -> Int
        = _ n:$(['0'..='9']+) _ {? n.parse().or(Err("Int")) }
        / _ "-" _ n:numInt() { -n }

    rule ident() -> &'input str
        = alnum()
        / expected!("identifier")

    rule alnum() -> &'input str = _ s:$(quiet!{['a'..='z'|'A'..='Z'|'_']['a'..='z'|'A'..='Z'|'_'|'0'..='9']*}) _ {s}

    rule slice<T>(x: rule<T>) -> (T, &'input str)
        = s:position!() t:x() e:position!() pair:#{|input, _|
            peg::RuleResult::Matched(e, (t, &input[s..e]))
        } {pair}

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

    rule NL() = "\n" / "\r\n"

    rule __ -> usize = s:$(quiet!{[' ' | '\r' | '\x0b' | '\x0c' | '\t']* ("#" [^'\n']*)? }) { s.len() }
    rule _ -> usize = s:$( __ ** NL() ) { s.len() }
}}

enum HelperType {
    Elf,
    Raindeer,
}

impl<'i> ShopBlock<&'i str> {
    fn empty_plan() -> Self {
        Self::Plan {
            width: 0,
            height: 0,
            map: vec![],
        }
    }
    fn make_plan(r1: PlanRow<&'i str>, mut rows: Vec<PlanRow<&'i str>>) -> Self {
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

        let empty = rows
            .iter()
            .flat_map(|r| r.tiles.iter())
            .filter_map(|t| match t.text == "  " {
                true => Some(t.text),
                _ => None,
            })
            .next()
            .unwrap_or_else(|| &rows[0].text[0..2]);

        let height = rows.len();
        let mut map = Vec::new();
        map.resize(
            width * height,
            Tile {
                text: empty,
                kind: TileKind::Empty,
            },
        );

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
    use super::*;

    fn t<S>(s: S, kind: TileKind) -> Tile<S> {
        Tile { text: s, kind }
    }

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

        use TileKind::*;
        let expected = Shop {
            name: "test",
            blocks: vec![ShopBlock::Plan {
                width: 3,
                height: 2,
                map: vec![
                    t("..", Empty),
                    t("..", Empty),
                    t("..", Empty),
                    t("..", Empty),
                    t("..", Empty),
                    t("  ", Empty),
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
                       .. 00
                    ;
                ;
            ",
        );

        let shop = match shop {
            Err(e) => panic!("{e}"),
            Ok(s) => s,
        };

        use TileKind::*;
        let expected = Shop {
            name: "test",
            blocks: vec![ShopBlock::Plan {
                width: 3,
                height: 2,
                map: vec![
                    t("e>", Elf(Direction::Right)),
                    t("..", Empty),
                    t("mv", Move(Direction::Down)),
                    t("  ", Empty),
                    t("..", Empty),
                    t("00", Instr(runtime::Instr::Push(0))),
                ],
            }],
        };

        pretty_assertions::assert_eq!(expected, shop);
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
            crate::ir::{Instr::*, Op::*},
            Direction::*,
            TileKind::*,
        };
        let expected = Shop {
            name: "weird_Hm",
            blocks: vec![ShopBlock::Plan {
                width: 7,
                height: 5,
                #[rustfmt::skip]
                map: vec![
                    t("  ", Empty), t("mv", Move(Down)), t("  ", Empty), t("S1", Instr(Swap(1))), t("-1", Instr(ArithC(Sub, 1))), t("m<", Move(Left)), t("  ", Empty),
                    t("  ", Empty), t("  ", Empty), t("  ", Empty), t("m>", Move(Right)), t("  ", Empty), t("  ", Empty), t("Hm", Instr(Hammock)),
                    t("e>", Elf(Right)), t("m>", Move(Right)), t("D1", Instr(Dup(1))), t("?>", IsPos), t("  ", Empty), t("S1", Instr(Swap(1))), t("  ", Empty),
                    t("  ", Empty), t("  ", Empty), t("  ", Empty), t("m>", Move(Right)), t("D0", Instr(Dup(0))), t("?>", IsPos), t("  ", Empty),
                    t("  ", Empty), t("  ", Empty), t("  ", Empty), t("m^", Move(Up)), t("-1", Instr(ArithC(Sub, 1))), t("m<", Move(Left)), t("  ", Empty),
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
                    stack: vec![Expr::Number(1), Expr::Number(2), Expr::Number(3)],
                },
                ToDo::SetupElf {
                    shop: "prod",
                    name: Some("Bob".into()),
                    stack: vec![],
                },
                ToDo::Connect {
                    src: Connection::Port("Josh".into(), 'a'),
                    dst: Connection::Port("Bob".into(), 1 as char),
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
                            stack: vec![Expr::Number(4), Expr::Number(5)],
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

    #[test]
    fn parse_comment() {
        crate::logger::init(log::LevelFilter::Trace);
        let shop = santasm::shop(
            "
                workshop test: # hello
                    floorplan:  # test
                    e> .. mv   # test
                       .. 00   # test
                    ;
                ;
            ",
        );

        let shop = match shop {
            Err(e) => panic!("{e}"),
            Ok(s) => s,
        };

        use TileKind::*;
        let expected = Shop {
            name: "test",
            blocks: vec![ShopBlock::Plan {
                width: 4,
                height: 2,
                map: vec![
                    t("e>", Elf(Direction::Right)),
                    t("..", Empty),
                    t("mv", Move(Direction::Down)),
                    t("  ", Empty),
                    t("  ", Empty),
                    t("..", Empty),
                    t("00", Instr(runtime::Instr::Push(0))),
                    t("  ", Empty),
                ],
            }],
        };

        pretty_assertions::assert_eq!(expected, shop);
    }

    #[test]
    fn parse_tile() {
        let tile_r = santasm::plan_tile("e>");

        let tile = match tile_r {
            Err(e) => panic!("{e}"),
            Ok(s) => s,
        };

        let expected = Tile {
            text: "e>",
            kind: TileKind::Elf(Direction::Right),
        };

        pretty_assertions::assert_eq!(expected, tile);
    }
}
