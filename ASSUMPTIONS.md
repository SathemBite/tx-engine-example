# Assumptions

Short version of what this implementation assumes:

1. One client = one asset account.
2. `client` is `u16`, `tx` is `u32`.
3. `tx` is treated as globally unique (duplicate `tx` is skipped).
4. New client records are created on `deposit` and on successful `withdrawal`.
5. `dispute/resolve/chargeback` for an unknown client are skipped.
6. `dispute` is allowed only for `deposit`.
7. `dispute` may make `available` negative; we follow the spec math literally.
8. `resolve` and `chargeback` require an active dispute.
9. After `chargeback`, account is locked and future events are skipped.
10. CSV input is trimmed; empty `amount` is allowed for non-amount ops.
11. Output amounts are printed with 4 decimal places.
12. Output row order is not guaranteed.
