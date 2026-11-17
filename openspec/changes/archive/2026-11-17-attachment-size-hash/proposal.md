## Why

Attachment.size and Attachment.hash describe `data`; the R5 spec and the Java validator reject a mismatch. The validator did not check it.

## What Changes

- Add ATTACHMENT_001 (size mismatch) and ATTACHMENT_002 (SHA-1 hash mismatch), error severity, for Attachments at any depth.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `fhir-validator`: new requirement for Attachment size/hash consistency.

## Impact

`crates/fhir-validator/src/attachment.rs`, `nested.rs` (`descend`), new SHA-1 dependency.
