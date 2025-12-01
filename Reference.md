# Language reference

SantAS (Santa Assembly) is a Christmas-Object‑Oriented Programming language designed for
festive workload distribution. Instead of carrying out every instruction himself,
Santa delegates execution to a scalable number of elves. Programs are organized
into workshops, which define the tasks to be performed, while Santa acts as the
overseer, coordinating and supervising the overall flow.

## Execution model

1. The program **starts** with the Santa. He performs his ToDo block and can spawn
multiple elves and connect their ports with pipes. Then he waits for elves to complete,
optionally monitoring some outgoing ports.
2. During **execution**, elves work in their workshops, reading messages from incoming
ports, and sending messages to outgoing ports. They can also send message to Santa,
writing to a port that he monitors.
3. The program **stops** when all elves fall asleep. Elf falls asleep in the *Hammock*
or when reading from a closed port. When an elf falls asleep, all the outgoing ports
close. This causes connected incoming ports to close, if after this there are
no more open connected open ports left.

### Elves

Workshop is a grid of [two-letter tiles](#instruction-tiles-reference).
Each elf has their own workshop; they carry a stack of paper and a pencil, with
which they can perform some arithmetic. They can operate on the top 10 sheets.

They can also write up to 10 numbers on their *sleeve*. Using instructions
`R0`..`R9` and `W0`..`W9` they can read and write to/from one of the 10 slots.

The elf walks through the workshop like this:

- Elves start on an `e_` tile (their spawn point and facing direction).
- They walk in a *straight line*, executing instructions in the order they step
  on them.

- Movement continues until:
  - A **direction tile** (`m^`, `mv`, `m<`, `m>`) changes their path.
  - A **conditional tile** (`?=`, `?>`, `?<`, `?s`) diverts them based on the check.
    True turns them right, otherwise they go left.


- Unless redirected, they march endlessly forward, faithfully carrying out Santa’s plan.

### Ports and pipes

Workshops can communicate through pipes. Each elf in a workshop has two sets
of *input* and *output* ports, and Santa can connect them to other workshop via
a pipe by the `setup .. -> ..` ToDo.

Inputs and outputs are treated separately, so `I1` reads from input port 1,
while `O1` writes to output port 1, which is completely separate.
You can connect them with `setup Elf.1 -> Elf.1`.

See the [stream_add example](./examples/stream_add.sasm) file pls.

#### Files

It is possible to read from or write to files by setting up a file as an input or output of a pipe. 

See the [copy_file example](./examples/copy_file.sasm) file.

---

## Syntax

The program source consists of one or more files. The translation treats them as
if they were concatenated into one string. Altogether, the module may contain multiple
workshop blocks and one Santa block.

    workshop MyWorkshop1: ... ;
    workshop MyWorkshop2: ... ;
    ...

    Santa will:
      setup MyWorkshop1 for elf Cringle ()
      ...
    ;

In practice, you describe the workshop layouts and then tell Santa what to do.

## Workshop description

Workshops are described by floorplan blocks. Inside a floorplan, every
instruction tile is exactly **two characters wide**, and tiles are separated
by a **single space**. Indentation must align so that the tiles form nice columns.

Example snippet:

    workshop MyWorkshop1:
      floorplan:
            m> 01 02 mv
            e^ .. .. ..
         Hm .. .. +_ m<
      ;
    ;

Here, the elf starts at `e^` (facing north), turns right, pushes `1` and `2`
onto the stack then turns south at `mv`, turns west at `m<`, summing `1` and `2`,
leaving `3` on the stack when they fall asleep in the Hammock `Hm`.

## Santa code

The Santa block may contain one or more `ToDo`s.

> `Santa` `will` `:` ToDo list `;`

### ToDo items

- `setup` *shop_name* `for` `elf` *elf_name*? `(` number list `)` </br>
  - Create new workshop for an elf with starting stack equal to the given number list.

- `setup` *source_elf* `.` *source_port* `->` *target_elf* `.` *target_port* </br>
  - Connect two elves' workshop ports together with a pipe.

- `monitor` *elf* `.` *port* `:` ToDo list `;` </br>
  - Santa connects a pipe to the given port and when a sheet of paper arrives through
this pipe, he executes the ToDo list in this monitor block.

- `receive` *var* ( `from` *elf* `.` *port* )?
  - Receive a sheet from the monitored port. You can later refer to it by chosen
  identifier *var*.
  - The `from` part is optional, defaults to the monitored port if left out.

- `send` *var* `to`

- `deliver` *var*
  - Print the value of *var* to the screen as a single ASCII character.


---

## Instruction tiles reference

A tile always consist of two printable characters. Terminology:
- *Push* - elf puts a blank sheet of paper on top of their stack and writes something on it.
- *Pop* - elf removes the top sheet and does something with it.

| Characters | Meaning | Stack (before → after) |
|-----------|---------|------------------------|
| `..`, `  ` | Empty tile | -- |
| `m^`, `mv`, `m<`, `m>` | Move elf (set direction up, down, left, right) | -- |
| `e^`, `ev`, `e<`, `e>` | Elf spawn point (with direction). | -- |
| `C<c>` | Push character `c`. | `a b` → `a b <c>` |
| `<d1><d0>` | Push two‑digit number `d1d0`. | `a b` → `a b <d1d0>` |
| `D<n>` | Duplicate sheet at depth `n` (0 = top) and place on top. | `D1`: `a b c` → `a b c b` |
| `E<n>` | Remove sheet at depth `n` (0 = top). | `E1`: `a b c` → `a c` |
| `S<n>` | Swap sheet at depth `n` with sheet on top. | `S1`: `a b c` → `a c b` |
| `I<c>` | Wait for incoming sheet `n` from port `c` and put it on top. | `I1`: `a b` → `a b n` |
| `O<c>` | Pop a number and send it down port `c`. | `Ox`: `a b n` → `a b` |
| `W<n>` | Pop a number and write it on the sleeve slot `n`. | TODO |
| `R<n>` | Read sleeve slot `n` and push it on the stack. | TODO |
| `Hm` | Hammock. Elf falls asleep here, to wait for the next christmas. | -- |
| `?=` | Pop `n` from the stack, go right if `n` = 0, left otherwise. | `a b` → `a` |
| `?>` | Pop `n` from the stack, go right if `n` > 0, left otherwise. | `a b` → `a` |
| `?<` | Pop `n` from the stack, go right if `n` < 0, left otherwise. | `a b` → `a` |
| `?s` | Elf goes right when the stack is empty. | -- |
| `!s` | Push lenght of stack on top. | TODO |
| `+_`, `-_`, `*_`, `/_`, `%_` | Arithmetic on top two items; consumes both | `+_`: `a b` → `(a+b)` |
| `+<n>`, `-<n>`, `*<n>`, `/ <n>`, `%<n>` | Arithmetic with constant `<n>`; consumes top | `+<n>`: `a b` → `a (b+<n>)` |
