# Debugging programs

TL;DR: use `--trace`

You can then produce step-by-step execution trace of your elves, like this one

TODO: elaborate

    debug: Scheduling Elf(1) "Printer"
    trace: elf Printer >  20 | JmpPtr(0)                []
    trace: elf Printer >   0 | In(110)                  [0]
    trace: elf Printer >   1 | Dup(0)                   [0, 0]
    trace: elf Printer >   2 | ArithC(Add, 1)           [0, 1]
    trace: elf Printer >   3 | IfPosPtr(22)             [0]
    trace: elf Printer >  22 | Push(45)                 [0, 45]
    trace: elf Printer >  23 | Out(111)                 [0]
    trace: evt=Some(Write(111))
    debug: Scheduling Santa { ip: 4, until: 3 }
    trace: santa:    4 | Receive(1, 111)    -> 45
    trace: santa:    5 | Deliver(4)         -> 0
    trace: evt=Some(Dequeue)
    debug: Scheduling Elf(1) "Printer"
    trace: elf Printer >  24 | JmpPtr(21)               [0]
    trace: elf Printer >  21 | JmpPtr(4)                [0]
    trace: elf Printer >   4 | Dup(0)                   [0, 0]
    trace: elf Printer >   5 | Push(10)                 [0, 0, 10]
    trace: elf Printer >   6 | Arith(Mod)               [0, 0]
    trace: elf Printer >   7 | Swap(1)                  [0, 0]
    trace: elf Printer >   8 | Push(10)                 [0, 0, 10]
    trace: elf Printer >   9 | Arith(Div)               [0, 0]
    trace: elf Printer >  10 | Dup(0)                   [0, 0, 0]
    trace: elf Printer >  11 | IfNzPtr(21)              [0, 0]
    trace: elf Printer >  12 | Erase(0)                 [0]
    trace: elf Printer >  13 | Push(48)                 [0, 48]
    trace: elf Printer >  14 | Arith(Add)               [48]
    trace: elf Printer >  15 | Out(111)                 []
