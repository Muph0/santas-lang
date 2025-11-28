# Language reference

The program module consists of multiple files. The module may contain multiple
workshop blocks and one Santa block.

    workshop MyWorkshop1: ;
    workshop MyWorkshop2: ;
    ...

    Santa will: ;

You describe the workshop layout and then tell Santa what to do.

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

The santa block may contain one or more `ToDo`s.

> `Santa` `will` `:` ToDo list `;`

### ToDo items

- `setup` *shop_name* `for` `elf` *elf_name*? `(` number list `)` </br>
  - Create new workshop for an elf with starting stack equal to the given number list.

- `setup` *source_elf* `.` *source_port* `->` *target_elf* `.` *target_port* </br>
  - Connect two elves' workshop ports together with a pipe.

- `monitor` *elf* `.` *port* `:` ToDo list `;` </br>
  - Santa connects a pipe to the given port and when a sheet of paper arrives through
this pipe, he executes the ToDo list in this monitor block.

- `receive` *var* ( `from` *elf* `.` *port* )
  - Receive a sheet from the monitored port. You can later refer to it by chosen
  identifier *var*.

- `send` *var* `to`

- `deliver` *var*
  - Print the value of *var* to the screen as a single ASCII character.


---

## Execution model

The santa is prettty lazy, so he will only follow simple instructions without
any loops or branches.

### Elves

Each elf has their own workshop; they carry a stack of paper and a pencil, with
which they can perform some arithmetic.

The elf movement goes like this:

- Elves start on an `e_` tile (their spawn point and facing direction).
- They walk in a *straight line*, executing instructions in the order they step
  on them.

- Movement continues until:
  - A **direction tile** (`m^`, `mv`, `m<`, `m>`) changes their path.
  - A **conditional tile** (`?=`, `?>`, `?<`) diverts them based on the stack’s
    top value. Satisfied condition turns them right, otherwise they go left.
    Either way the top value is consumed.

- Unless redirected, they march endlessly forward, faithfully carrying out Santa’s plan.

### Pipes

TODO

---

## Instruction tiles reference

A tile always consist of two printable characters. Terminology:
- *Push* - elf puts a blank sheet of paper on top of their stack and writes something on it.
- *Pop* - elf removes the top sheet and does something with it.

| Characters | Meaning | Stack (before → after) |
|-----------|---------|------------------------|
| `..`, `␣␣` | Empty tile | -- |
| `m^`, `mv`, `m<`, `m>` | Move elf (set direction up, down, left, right) | -- |
| `e^`, `ev`, `e<`, `e>` | Elf spawn point (with direction). | -- |
| `C<c>` | Push character `c`. | `a b` → `a b <c>` |
| `<d1><d0>` | Push two‑digit number `d1d0`. | `a b` → `a b <d1d0>` |
| `D<n>` | Duplicate sheet at depth `n` (0 = top) and place on top. | `D1`: `a b c` → `a b c b` |
| `R<n>` | Remove sheet at depth `n` (0 = top). | `R1`: `a b c` → `a c` |
| `S<n>` | Swap sheet at depth `n` with sheet on top. | `S1`: `a b c` → `a c b` |
| `I<c>` | Wait for incoming sheet `n` from port `c` and put it on top. | `I1`: `a b` → `a b n` |
| `O<c>` | Pop `n` and send it down port `c`. | `Ox`: `a b n` → `a b` |
| `Hm` | Hammock. Elf falls asleep here, to wait for the next christmas. | -- |
| `?=` | Pop `n` from the stack, go right if `n` = 0, left otherwise. | `a b` → `a` |
| `?>` | Pop `n` from the stack, go right if `n` > 0, left otherwise.  | `a b` → `a` |
| `?<` | Pop `n` from the stack, go right if `n` < 0, left otherwise.  | `a b` → `a` |
| `+_`, `-_`, `*_`, `/_`, `%_` | Arithmetic on top two items; consumes both | `+_`: `a b` → `(a+b)` |
| `+<n>`, `-<n>`, `*<n>`, `/ <n>`, `%<n>` | Arithmetic with constant `<n>`; consumes top | `+<n>`: `a b` → `a (b+<n>)` |
