# CLAUDE.md — poe-wayfinder-core

The domain. Text in, item out. Item in, trade query out.

Read `~/.claude/CLAUDE.md` then `../CLAUDE.md`. Both apply.

## The one rule that matters

**No I/O in this crate.** No network. No filesystem. No clipboard. No window.
No clock beyond what a caller passes in.

That is why every test here runs without the game and without a socket. The
reference has no parser tests that run outside Electron. Ours do.

If you need data from outside, declare a trait in `src/adapter/` and let
`poe-wayfinder-app` implement it. `GameData` is the example.

## Layout

| Path | Holds |
|---|---|
| `src/types/` | plain data. No behaviour beyond small helpers. |
| `src/controller/parse/` | the parser |
| `src/controller/stat_match/` | text to stat id |
| `src/controller/calc/` | base and quality math |
| `src/controller/filter/` | item to trade query |
| `src/adapter/` | traits only. Zero implementations. |
| `src/util/` | pure helpers |

## How the parser works

Read `../STUDY.md` section 1 first.

The parser splits clipboard text on lines equal to `--------`, then runs an
ordered list of stages. Each stage returns one of three outcomes.

| Outcome | Meaning |
|---|---|
| `SectionParsed` | the section is consumed and removed |
| `SectionSkipped` | not this section. Try the next one. |
| `ParserSkipped` | this stage does not apply. Next stage. |

A section is consumed at most once. That is why the modifier stage appears
five times in the pipeline. Each occurrence eats a different section.

## PoE1 and PoE2

One parser. 35 of 50 stages are shared. See `../DESIGN.md` section 2.

`src/controller/parse/shared/` holds the 35. `poe1/` holds 11. `poe2/` holds
15. Two builders assemble an ordered `Vec<Stage>` per game.

Never fork the crate per game. The delta is 30 percent.

## Build and test

```sh
forge build
forge test-all
```

Coverage target is near 100 percent of hand written code.
