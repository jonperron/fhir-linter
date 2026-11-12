## 1. Registry

- [x] 1.1 Add `Registry::child_element` with backbone, choice and unknown cases

## 2. FHIRPath data model

- [x] 2.1 Add `resource_to_typed_value` and `ChildElement`
- [x] 2.2 Add `Value::Primitive`, merge JSON `_x` and XML primitives with children
- [x] 2.3 Unwrap primitives in operators and functions; fix `hasValue()`
- [x] 2.4 Exclude `resourceType` and JSON nulls from items; complex `=` primitive is false
- [x] 2.5 Type official suite inputs with the Registry; raise floor to 829

## 3. Validator

- [x] 3.1 Evaluate invariants on typed values via `child_element`
- [x] 3.2 Add invariant regression tests (app-5, task-inv-1, bdl-10, org-1)
