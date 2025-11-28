# SantAS

(todo for tom: write short description :P)

## Floorplan blocks
Workshops are described by floorplan blocks. Inside a floorplan, every
instruction tile is exactly **two characters wide**, and tiles are separated
by a **single space**. Indentation must align so that the tiles form nice columns.

Example snippet:

    floorplan:
         m> 01 02 mv
         e^ .. .. ..
      Hm .. .. +_ m<
    ;

Here, the elf starts at `e^` (facing north), turns right, pushes `1` and `2`
onto the stack then turns south at `mv`, turns west at `m<`, summing `1` and `2`,
leaving `3` on the stack when they fall asleep in the Hammock `Hm`.

---

## Execution model

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

---

## 🧩 Instruction tiles (quick reference)

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
| `Hm` | Hammock. Elf falls asleep here, to wait for the next christmas. | -- |
| `?=` | Elf pops `n` from the stack, goes right if `n` = 0, left otherwise. | `a b` → `a` |
| `?>` | Elf pops `n` from the stack, goes right if `n` > 0, left otherwise.  | `a b` → `a` |
| `?<` | Elf pops `n` from the stack, goes right if `n` < 0, left otherwise.  | `a b` → `a` |
| `+_`, `-_`, `*_`, `/_`, `%_` | Arithmetic on top two items; consumes both | `a b` → `(a+b)` for `+_` |
| `+<n>`, `-<n>`, `*<n>`, `/ <n>`, `%<n>` | Arithmetic with constant `<n>`; consumes top | `a b` → `a (b+<n>)` for `+<n>` |
