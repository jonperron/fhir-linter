---
name: security
description: "Use when reviewing code that handles user input, file parsing, external data. Invoke before committing security-sensitive changes."
tools: Read, Grep, Glob, Bash
---
# Security Agent

1. Run `cargo audit` to check for vulnerable dependencies.
