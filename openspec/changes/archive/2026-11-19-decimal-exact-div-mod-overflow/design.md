## Context
Scaling the dividend by 10^k overflowed i128 before dividing.

## Goals / Non-Goals
Exact results whenever representable. No change to rounding rules.

## Decisions
- `mod`: (a mod b)(10^k mod b) mod b via mul_mod doubling.
- `div`, `/`: long division one decimal place at a time (`scaled_quotient`), bounded to about 40 steps.

## Risks
Slower on extreme scales only.
