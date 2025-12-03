pub mod ir;
pub mod logger;
pub mod parse;
pub mod runtime;
pub mod translate;

pub use parse::parse;

trait RecoverResult<T, E> {
    fn recover(self, t: T, errors: &mut Vec<E>) -> T;
}
impl<T, E> RecoverResult<T, E> for Result<T, E> {
    fn recover(self, t: T, errors: &mut Vec<E>) -> T {
        match self {
            Ok(ok) => ok,
            Err(e) => {
                errors.push(e);
                t
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::runtime::{Instr::*, *};

    const PRINT: Port = 123;

    #[test]
    pub fn fizzbuzz() {
        crate::logger::init(log::LevelFilter::Debug);
        todo!("hangs");

        #[rustfmt::skip]
        let fizzbuzz = Room::new_testing(vec![      // 100
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
        let print = Room::new_testing(vec![ // num: =-1->Fizz, =-2->Buzz, else print num
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

        let santa = Vec::from([
            SantaCode::SetupElf {
                name: None,
                room: 0,
                stack: vec![100],
            },
            SantaCode::SetupElf {
                name: None,
                room: 1,
                stack: vec![100],
            },
            SantaCode::Monitor {
                port: (1, PRINT),
                block_len: 2,
            },
            SantaCode::Receive(1, PRINT),
            SantaCode::Deliver(3),
        ]);

        let unit = Unit {
            rooms: vec![fizzbuzz, print],
            santa,
        };

        let mut rt = Runtime::new(&unit);
        rt.run(RunCommand::RunToEnd).unwrap();
    }
}

use std::ops::Drop;

pub struct DropGuard<F: FnMut() + 'static> {
    action: Option<F>,
}

impl<F: FnMut() + 'static> DropGuard<F> {
    /// Create a new DropGuard with a given closure
    pub fn new(f: F) -> Self {
        DropGuard { action: Some(f) }
    }

    /// Create a new DropGuard with no closure
    pub fn new_empty() -> Self {
        DropGuard { action: None }
    }

    /// Reset the closure to a new one
    pub fn reset<G>(mut self, g: G) -> DropGuard<G>
    where
        G: FnMut() + 'static,
    {
        self.clear();
        DropGuard { action: Some(g) }
    }

    /// Clear the closure (nothing will run on drop)
    pub fn clear(&mut self) {
        self.action = None;
    }
}

impl<F: FnMut() + 'static> Drop for DropGuard<F> {
    fn drop(&mut self) {
        if let Some(action) = self.action.as_mut() {
            action();
        }
    }
}
