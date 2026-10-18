# AGENTS.md — rules for AI agents working in this repo

This file governs any AI coding agent (Claude, ZCode, Codex, …) working in
the 1974-speed-race repository. Humans are subject to CONTRIBUTING.md;
agents are subject to this file. Where they conflict, this file wins for
agents.

## 1. Commit and push autonomously

**You SHALL commit and push as you see fit, without asking permission.**
Every completed, compiling unit of work must be committed and pushed to
`origin` before you report back. Do not stop to ask "shall I commit?" —
the answer is always yes. Commit early, commit often, push every commit.

- Write real commit messages (imperative mood, say *why*, not just *what*).
- Never leave work uncommitted at the end of a turn.
- Never push broken builds. Before every push, all three gates must pass:
  1. `cargo build --release` — **zero warnings** (note: incremental
     builds suppress repeat warnings; use `touch src/main.rs` first if
     unsure)
  2. `cargo clippy --release --all-targets` — zero warnings
  3. `cargo test --release` — all tests green
- Force pushes are the one exception: never `--force` without an explicit
  human instruction.

## 2. Verify by simulation, not by hope

This codebase deliberately separates game logic (`Game::step`) from
rendering. When changing gameplay, add or extend a headless simulation
test in `src/main.rs` (`mod tests`) that drives `step()` directly and
asserts on outcomes, and run it with `cargo test --release`. Visual
changes may be eyeballed via screenshots, but behavior changes must be
backed by a passing sim.

## 3. Keep the dependency budget at zero-plus-one

The only permitted dependency is `minifb`. Everything else — fonts,
sprites, pseudo-randomness (see `hash()`), input, timing — stays in
`std`. Do not add crates without a documented, compelling reason in the
commit message.

## 4. House style

- Single-binary project: game code lives in `src/main.rs`, organized in
  clearly-commented sections. Keep it that way unless it outgrows ~1500
  lines, at which point propose a module split in a commit.
- Arcade-authentic presentation: monochrome sprites tinted by the
  cellophane overlay bands (red / amber / green). New visual elements
  should obey the overlay model, not bypass it.
- Sprites are ASCII-art strings (`#` white, `o` grey, `.` transparent)
  parsed at startup. Draw new art in that format.
- No `unwrap()` outside `main()`'s window setup; game-loop code must
  never panic.

## 5. Scope discipline

Small, complete changes. A commit that touches rendering shouldn't
silently retune gameplay, and vice versa. Difficulty tuning changes
should note the affected dials (cut frequency, cut speed, spawn rate,
`diff` ramp) in the commit message so they can be bisected later.
