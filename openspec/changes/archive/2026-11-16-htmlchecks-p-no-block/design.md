## Context
HTML 4.01 `p` holds only inline content.

## Goals / Non-Goals
Reject block elements (p, h1-h6, ul, ol, pre, dl, div, blockquote, hr, table, address) inside `p`. No other content-model checks.

## Decisions
`map` resets the rule because MAP holds %block.

## Risks
Documents with sloppy nesting now fail.
