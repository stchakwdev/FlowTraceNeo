# ledger-core tests

The fixture tests replay `rust/fixtures/` (see its README). This note covers the property suite,
the review regressions and the parity table check (task C7).

## Property tests: `properties.rs`

Fifteen properties over books built through the engine. The first four port the fast-check
properties of `flowtrace/src/lib/ledger/engine.test.ts:322-380`; the others state the invariants
of `Docs/flowtrace-ledger-plan.md` section 5 (balance in base currency and per fund, immutable
posted entries, closed and locked periods, grant dates and awards) plus rejections that leave no
trace, random command sequences, statement import, auto-match and the CSV round trip. Each test's
doc comment cites the TS test or rule it states. Strategies live in `common/strategies.rs`,
books, actors and the shared invariant check `books_hold` in `common/books.rs`.

Each property runs 1000 cases by default (about 4.5 s for the file in a debug build).
`PROPTEST_CASES` changes that for every property in the run:

```
cd rust
cargo test -p ledger-core --test properties                       # 1000 cases each
PROPTEST_CASES=20000 cargo test -p ledger-core --test properties  # a soak run, about 85 s
PROPTEST_CASES=50 cargo test -p ledger-core --test properties     # a quick local loop
```

When a property fails, proptest shrinks the input and prints the minimal case, and saves its seed
to `tests/properties.proptest-regressions`; later runs replay saved seeds first. Turn the minimal
case into a named `#[test]` (below), then delete the seed file or commit it with the fix. Config
and strategy API: <https://docs.rs/proptest/1.12.0/proptest/test_runner/struct.Config.html> and
<https://docs.rs/proptest/1.12.0/proptest/strategy/trait.Strategy.html>.

Shrinking stays useful because strategies draw plain data (amounts, sides, account codes) and
build engine inputs last; ids that exist only once the books do (an entry to reverse, a draft to
edit) are drawn as `proptest::sample::Index` and resolved against the books when the case runs.

## Review regressions: `review_regressions.rs`

One `#[test]` per `it` of `flowtrace/src/lib/ledger/review-regressions.test.ts`, named as the TS
test in snake case (apostrophes dropped, other punctuation an underscore) with a doc comment
citing its lines: `cannot_change_an_accounts_type_code_or_normal_balance` is
`review-regressions.test.ts:50-57`, and so on for all eleven. Each builds its books as the TS file
does: `common::books::regression_books()` is its `books()`.

To add a regression test (a bug found in review, or a minimal case from a failing property):

1. Name the test after the behaviour that must hold, in snake case; if it ports a TS test, use
   the TS title and cite `<file>.ts:<lines>`.
2. Build the books with `regression_books()`, `fresh()` (`engine.test.ts:24-40`) or
   `org_books()`, and drive them through `ledger_core` only (engine methods, or
   `sanitize_command` then `apply_command` as a client would).
3. Assert what TS asserts; where a fixture already replays the case, the test may assert through
   `common::replay` instead.

## Parity table: `parity_table.rs`

Fails unless `rust/docs/parity.md` has exactly one row per export of the TS ledger modules and
per public method of the TS `Ledger` class (94 rows), found by grepping the TS files.
