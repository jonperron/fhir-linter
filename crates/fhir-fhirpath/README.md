# fhir-fhirpath

FHIRPath 2.0 expression evaluator used for FHIR R5 invariants.

## Third-party data

`data/ucum-essence.xml` is an unmodified copy of the UCUM essence file,
read at runtime to convert Quantity units. It is distributed under the
UCUM License, Version 1.1; see `data/UCUM-NOTICE.txt` for the copyright
notice, licence reference and warranty disclaimer. Both files ship in
the crate package.
