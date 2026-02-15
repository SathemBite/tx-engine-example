# tx-engine-example

Small payment engine for a coding challenge.

## Run

```bash
cargo run -- data/transactions.csv
```

```bash
cargo run -- /path/to/transactions.csv
```

## Docs

- `ASSUMPTIONS.md`
- `AI_USAGE.md`

## Errors

- CSV I/O or parsing errors are fatal.
- Invalid business events are non-fatal and skipped.

## Tests

```bash
cargo test
```
