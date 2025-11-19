mod runtime;
mod parse;

pub use runtime::*;

#[cfg(test)]
mod test {
    use crate::runtime::{Instr::*, *};

    const PRINT: Port = 123;

    #[test]
    pub fn test1() {
        simple_logger::init_with_level(log::Level::Debug).unwrap();

        #[rustfmt::skip]
        let fizzbuzz = Room::new(vec![      // 100
            Push(1),                        // 100 1
            Label("loop"),
                Dup(1),                     // 100 1 100
                Dup(1),                     // 100 1 100 1
                Arith(Op::Sub),             // 100 1 99
                ArithC(Op::Add, 1),         // 100 1 100

                IfPos("continue"),          // 100 1
                    Hammock,                // 100 100
                Label("continue"),

                Push(0),                    // 100 1 0
                Dup(1),                     // 100 1 0 1
                ArithC(Op::Mod, 3),         // 100 1 0 1

                IfPos("no fizz"),           // 100 1 0
                    Push(-1),               // 100 3 0 -1
                    Out(PRINT),             // 100 3 0
                    Erase(0), Push(1),      // 100 3 1
                Label("no fizz"),

                Dup(1),                     // 100 1 0 1
                ArithC(Op::Mod, 5),         // 100 1 0 1

                IfPos("no buzz"),           // 100 1 0
                    Push(-2),               // 100 5 0 -2
                    Out(PRINT),             // 100 5 0
                    Erase(0), Push(1),      // 100 5 1
                Label("no buzz"),           // 100 1 0

                IfPos("no number"),         // 100 1
                    Dup(0),                 // 100 1 1
                    Out(PRINT),             // 100 1
                Label("no number"),

                Push(-3),
                Out(PRINT),

                ArithC(Op::Add, 1),         // 100 2
            Jmp("loop"),
        ]);

        #[rustfmt::skip]
        let print = Room::new(vec![ // num: =-1->Fizz, =-2->Buzz, else print num
            Label("start"),
            In(1),                          // num

            Dup(0),
            ArithC(Op::Add, 1), // if num == -1
            IfNz("not fizz"),
                Push('z' as Int),
                Push('z' as Int),
                Push('i' as Int),
                Push('F' as Int),
                Out(PRINT),
                Out(PRINT),
                Out(PRINT),
                Out(PRINT),
                Jmp("start"),
            Label("not fizz"),

            Dup(0),
            ArithC(Op::Add, 2), // if num == -2
            IfNz("not buzz"),
                Push('z' as Int),
                Push('z' as Int),
                Push('u' as Int),
                Push('B' as Int),
                Out(PRINT),
                Out(PRINT),
                Out(PRINT),
                Out(PRINT),
                Jmp("start"),
            Label("not buzz"),

            Dup(0),
            ArithC(Op::Add, 3), // if num == -3
            IfNz("not endl"),
                Push('\n' as Int),
                Out(PRINT),
                Jmp("start"),
            Label("not endl"),

            Push(-1),                           // num -1
            Swap(1),                            // -1 num
            Label("prep_digits"),
                Dup(0),                         // -1 num num
                ArithC(Op::Mod, 10),            // -1 num num%10
                Swap(1),                        // -1 d_0 num
                ArithC(Op::Div, 10),            // -1 d_0 num/10
                Dup(0),                         // -1 d_0 num/10 num/10
                IfNz("prep_digits"),            // -1 d_0 num/10

            Erase(0),                           // -1 d_0 .. d_k-1 d_k
            Label("print_digits"),
                ArithC(Op::Add, '0' as Int),    // -1 d_0 .. d_k-1 d_k+'0'
                Out(PRINT),                     // -1 d_0 .. d_k-1 -> prints d_k+'0'
                // find -1
                Dup(0),
                ArithC(Op::Add, 1),
                IfNz("print_digits"),

            Jmp("start"),
        ]);

        let mut elf_fizzbuzz = Elf::new(fizzbuzz, vec![100]);
        let mut elf_print = Elf::new(print, vec![]);

        elf_fizzbuzz.connect(PRINT, (&mut elf_print, 1));
        elf_print.monitor(PRINT, |_rt, n| {
            print!("{}", n as u8 as char);
        });

        let mut rt = Runtime::new(vec![elf_fizzbuzz, elf_print]);

        rt.run_loop().unwrap_or_else(|e| panic!("{e}"))
    }
}
