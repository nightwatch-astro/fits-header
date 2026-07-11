# Rust References

Use these in this order:

1. Local crate conventions and existing code patterns
2. Context7 for crate-specific docs
3. Canonical Rust docs for ownership, trait design, and API idioms

## Context7

- Use Context7 when the question depends on the current crate version or ecosystem tool.
- Start with the exact crate docs for the repo, then narrow to the topic:
  - `tokio`
  - `serde`
  - `axum`
  - `clap`
  - any crate actually used in the project

## Canonical Docs

- The Rust Book: https://doc.rust-lang.org/book/
- Rust API Guidelines: https://rust-lang.github.io/api-guidelines/
- Rust Reference: https://doc.rust-lang.org/reference/

## Steering Notes

- Prefer types that make ownership and invariants obvious.
- Prefer iterator and trait composition over repetitive imperative boilerplate when readability improves.
- Make fallible behavior explicit with `Result` and meaningful error types.
- Design public APIs to be unsurprising in the wider Rust ecosystem.
