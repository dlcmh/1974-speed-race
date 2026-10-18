# Contributing to 1974-speed-race

A dependency-light Rust tribute to Speed Race (Taito, 1974). PRs welcome.

## Ground rules

- **Rust + one dependency.** `minifb` is the only allowed crate. Fonts,
  sprites, pseudo-RNG, input handling: all `std`. PRs adding dependencies
  will be asked to justify them in the description.
- **Single binary, single file (for now).** Code lives in `src/main.rs`,
  organized by the section-banner comments. Follow the existing style;
  don't reformat unrelated code.
- **No panics in the game loop.** `unwrap()` is only acceptable in `main()`
  at window setup.

## Gameplay changes need a simulation test

The game logic (`Game::step`) is deliberately decoupled from rendering so
it can run headless. If your PR changes behavior (traffic AI, difficulty,
scoring, physics), add or extend a test in `mod tests` that simulates
frames and asserts the outcome, e.g. the existing `rivals_change_lanes`
test. Then:

```sh
cargo test --release
```

must pass. If your change affects difficulty, name the dials you touched
(`cut` frequency/speed, spawn rate, `diff` ramp) in the PR description so
the change is bisectable.

## Visual changes

Follow the arcade-authentic model: everything is drawn monochrome
(white / grey / black) and the cellophane overlay bands tint it. New
sprites are ASCII-art strings — `#` white, `o` grey, `.` transparent —
in the `Sprite::parse` format.

## Commit messages

Imperative mood, explain the *why* when it isn't obvious:

```
Widen rival AI decision window so lane cuts actually fire
```

## License

By contributing you agree that your contributions are licensed under the
MIT License found in `LICENSE`.
