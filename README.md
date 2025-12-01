# SantAS

Thematic programing language created for the occasion of [Advent of Code 2025](https://adventofcode.com/2025/about).

If you feel particularly adventurous, you can immerse yourself in a programming language
set (roughly) in the AoC universe. Embrace the true challenge of the AoC and write programs
as literal blueprints and todo-lists for Santa and his little helpers.

## User manual

*A word of warning: this is absolutely not production-level
software, use at your own risk!*

Clone this repository, and run `cargo run` in the root of it.
You can use cargo to put `santac` on PATH with `cargo install --path santa-comp`.

- Read the [language reference](./Reference.md)

You can then run your programs with

    santac -i my_program.sasm

## Roadmap

- [x] stack machine runtime
- [x] floorplan to stack machine program translation
- [x] Santa code runtime
- [x] Santa code translation
- [x] dynamic elf creation
- [x] file IO
- [ ] indirect piping between elves
- [ ] translation to LLVM
- [ ] JIT interpreter
- [ ] ...compiler?

## Project structure

```sh
  .
  ├───examples            # find various examples in the SantAS language
  ├───santa-comp          # executable for interpreting the source code
  │   └───src
  └───santa-lang          # library with language support
      └───src
          ├───parse       # grammar and AST representation
          ├───runtime     # runtime focused on easy debugging over performance
          └───translate   # algorithms that convert AST to runtime representation
```
