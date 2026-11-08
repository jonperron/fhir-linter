# Changelog

## [0.4.1](https://github.com/jonperron/fihr-linter/compare/fhir-linter-v0.4.0...fhir-linter-v0.4.1) (2026-11-08)


### Bug Fixes

* **definitions:** load core profiles from profiles-others.json ([b83c499](https://github.com/jonperron/fihr-linter/commit/b83c499a0a22d62b4982c5ae38b6538b26fd2e3c))
* **definitions:** load core profiles from profiles-others.json ([366a8cc](https://github.com/jonperron/fihr-linter/commit/366a8ccd07e15da5fc42eb9aed143213f6bf3eab))
* **parser:** parse integers beyond i64 as decimals ([efba82d](https://github.com/jonperron/fihr-linter/commit/efba82d6b283a0c7f553fc1aa2dcd670b8c8470e))
* **parser:** parse integers beyond i64 as decimals ([cfb5729](https://github.com/jonperron/fihr-linter/commit/cfb57291a5b18c16e6589e07aa5e9545bae2316c))
* **validator:** address audit should-fix findings ([977d45d](https://github.com/jonperron/fihr-linter/commit/977d45deb95dc0c34acac9db3f8e477e06fa3791))
* **validator:** apply the primitive extension rule at every depth ([513016e](https://github.com/jonperron/fihr-linter/commit/513016ef2e8e702fda6f9c6f88089c0a143b7dc5))
* **validator:** check cardinality of choice elements ([4e61e8b](https://github.com/jonperron/fihr-linter/commit/4e61e8b07298a79fe32aa8fa16068179d75c2b19))
* **validator:** enforce strict temporal and 32-bit integer formats ([ec28548](https://github.com/jonperron/fihr-linter/commit/ec28548e3983fde0f10e71cf496bd770197a5d84))
* **validator:** report skipped invariants as information ([3071256](https://github.com/jonperron/fihr-linter/commit/30712567d1a71115516ec21233239415a9f0ea86))
* **validator:** require extension shape for primitive presence ([4df7c9f](https://github.com/jonperron/fihr-linter/commit/4df7c9f0674321c091836e09a6ed5a319013bd3c))
* **validator:** tighten primitive extension and choice handling ([816de22](https://github.com/jonperron/fihr-linter/commit/816de22439ad178ef9acc01eebd64e41369094eb))
* **validator:** type-check resource id and fhirpath system types ([38ed94e](https://github.com/jonperron/fihr-linter/commit/38ed94e869a0d703558f9b88a0e2ab3027c5ac86))
* **validator:** validate choice elements from their declared types ([ea96e77](https://github.com/jonperron/fihr-linter/commit/ea96e778ede742c0a5dcd17636cf21b079f90be0))
* **validator:** validate choice elements from their declared types ([dc82c37](https://github.com/jonperron/fihr-linter/commit/dc82c3718660799cae3a51fe9072b22fa818942a))
* **validator:** validate extension and modifierExtension elements ([d24e99c](https://github.com/jonperron/fihr-linter/commit/d24e99c77e1b34dc1292cda7b8e95dc16b33ee8f))
* **validator:** validate resource ids and required choice elements ([aeb054a](https://github.com/jonperron/fihr-linter/commit/aeb054aba3c164268ebfdc683eb1be59cc778a71))


### Performance Improvements

* **validator:** index snapshot elements by path ([d35a39b](https://github.com/jonperron/fihr-linter/commit/d35a39b98f7cf642990c92a10c567f347ee610c5))

## [0.4.0](https://github.com/jonperron/fihr-linter/compare/fhir-linter-v0.3.0...fhir-linter-v0.4.0) (2026-11-03)


### Features

* **python:** expose validate and lint via pyo3 ([e3452f0](https://github.com/jonperron/fihr-linter/commit/e3452f048ead3068520538a3bfb1c2591e1d2f73))
* **python:** phase 8 python bindings ([8e6ae2a](https://github.com/jonperron/fihr-linter/commit/8e6ae2ac93d41bf387e09d05c6406857f11b27b4))
* **typescript:** add node addon bindings with validate and lint ([b1b4510](https://github.com/jonperron/fihr-linter/commit/b1b45104b9834b3651201c4e4674448cf9a27c58))
* **typescript:** node addon bindings (phase 9) ([03c1c9e](https://github.com/jonperron/fihr-linter/commit/03c1c9e2300b87af29f2824761a16ed2e00bc0d0))
* update readme ([6d6a579](https://github.com/jonperron/fihr-linter/commit/6d6a5791bc49df609894b3c66433f3ba800bd344))
* update readme ([dfa1ace](https://github.com/jonperron/fihr-linter/commit/dfa1acefb8bda9eeba5b99c724d4cdd8635c710b))


### Bug Fixes

* **deps:** bump quick-xml to 0.41 ([54b7a18](https://github.com/jonperron/fihr-linter/commit/54b7a182252b524cbe6cb5f0eb0a9608dfcd60a2))
* **python:** build abi3 wheels for all python &gt;= 3.10 ([fa23b40](https://github.com/jonperron/fihr-linter/commit/fa23b40f1b32c8df4055ca97e991c17fc779ac8a))
* **python:** share one registry per resolved definitions path ([013ee68](https://github.com/jonperron/fihr-linter/commit/013ee68595997e107d7becdde5be16cfc83bd9d0))
* **typescript:** address cross-review findings ([f2c14f5](https://github.com/jonperron/fihr-linter/commit/f2c14f5bfc7e7538c4ba418f8b4c102fd4cf0597))


### Performance Improvements

* **python:** release the GIL during validate and lint ([edb2d6c](https://github.com/jonperron/fihr-linter/commit/edb2d6c6d8fc8bb7b5435e72a2a1749a86a86964))

## [0.3.0](https://github.com/jonperron/fihr-linter/compare/fhir-linter-v0.2.0...fhir-linter-v0.3.0) (2026-11-02)


### Features

* **fhir-linter:** add rule registry, TOML config and first rules ([ed25744](https://github.com/jonperron/fihr-linter/commit/ed25744fa5017d08f2afb37069ce8a9cc1ece701))
* **fhir-validator:** add references and profile layers ([815504d](https://github.com/jonperron/fihr-linter/commit/815504d3e7d6e7f12d7b94b8ac13d256640e0c8d))
* **fhir-validator:** complete phase 5 validator layers ([a4b3362](https://github.com/jonperron/fihr-linter/commit/a4b3362e46aac0a5f503840c695156392565236d))
* **fhir-validator:** implement terminology layer ([de23f1b](https://github.com/jonperron/fihr-linter/commit/de23f1b07baa69bdc311b09a628a125a11d4a95d))
* **fhir-validator:** validate nested elements ([b043b79](https://github.com/jonperron/fihr-linter/commit/b043b79e03b818c96f1d157bc528c4567d6276ba))
* nested validation (phase 5) and linter (phase 6) ([a859adc](https://github.com/jonperron/fihr-linter/commit/a859adc9b0c104c0a555bd5f2a64e8a98254427c))


### Bug Fixes

* **fhir-fhirpath:** harden evaluator (panics, equality, regex) ([1bfc557](https://github.com/jonperron/fihr-linter/commit/1bfc5576c7e50d0be13157391f78649187230a5b))
* **fhir-fhirpath:** harden evaluator panics, set equality and regex ([274eaf3](https://github.com/jonperron/fihr-linter/commit/274eaf3dd81f9994102745b7aeb21b1ec092ddd7))
* **fhir-fhirpath:** harden parser string, bool and nesting handling ([96805eb](https://github.com/jonperron/fihr-linter/commit/96805ebc20c2309ddc9ecc954662f81ec2c3ff7b))
* **fhir-fhirpath:** harden parser string, bool and nesting handling ([9751612](https://github.com/jonperron/fihr-linter/commit/9751612ea1d4036b224dc416d2c1eedc7f1aeb3f))
* **fhir-fhirpath:** make nesting guard skip comments and backticks ([3381cfc](https://github.com/jonperron/fihr-linter/commit/3381cfc3d5861c8cea81c472adf1f8ea14cbcbb8))
* **fhir-fhirpath:** restore descendants order and tolerate base64 whitespace ([3e222be](https://github.com/jonperron/fihr-linter/commit/3e222beec5510ce2e58e6ed33e5181533bd68aec))
* **fhir-fhirpath:** tighten parser limits to prevent stack overflow ([868f773](https://github.com/jonperron/fihr-linter/commit/868f773b86c228b57091db35883a0f449a0ca067))
* **fhir-fhirpath:** tighten parser limits to prevent stack overflow ([504559e](https://github.com/jonperron/fihr-linter/commit/504559efac4650194aa3656696eafa3b7ab3ba49))
* **fhir-fhirpath:** use checked i64 arithmetic for integers ([82eb980](https://github.com/jonperron/fihr-linter/commit/82eb9808fd6b3c91aeb37f83355b864b302dbe43))
* **fhir-fhirpath:** use checked i64 arithmetic for integers ([3bf0ead](https://github.com/jonperron/fihr-linter/commit/3bf0eadfb0a7a6e2990073209e40dea94ecd2f79))
* **fhir-parser:** cap nesting depth and harden definitions loader ([fbb6e1e](https://github.com/jonperron/fihr-linter/commit/fbb6e1e09159f29bc1d1636ffd3f05dee43f5193))
* **fhir-parser:** cap nesting depth and harden definitions loader ([8c37ca4](https://github.com/jonperron/fihr-linter/commit/8c37ca4d8383dda8ef9b7b571f3294eba5ad200b))
* **fhir-validator:** address phase 5 review findings ([415c1ad](https://github.com/jonperron/fihr-linter/commit/415c1ad7acde523b5f03d1e714f7866370df422b))
* **fhir-validator:** handle contentReference and primitive extensions ([69402ec](https://github.com/jonperron/fihr-linter/commit/69402ecee9d35ccfaa3aee346ca28554a2c98b8f))
* **fhir-validator:** handle contentReference and primitive extensions ([58c08db](https://github.com/jonperron/fihr-linter/commit/58c08db50825842d11cb9a55f7ba99642a452d79))
* **fhir-validator:** non-ascii panic, dedupe, INVARIANT_PARSE ([2cccbb2](https://github.com/jonperron/fihr-linter/commit/2cccbb21c878f5e7c09caa03068980dfc2736972))
* **fhir-validator:** prevent non-ascii panic and warn on unparsable invariants ([7bdccbd](https://github.com/jonperron/fihr-linter/commit/7bdccbd625cf2378049ed9bccd73bbe4d54f5d9d))
* **fhir-validator:** skip unparseable invariants silently ([fcc3789](https://github.com/jonperron/fihr-linter/commit/fcc37896b08de072f2de49d5493cc9f4970f2d57))
* **fhir-validator:** skip unparseable invariants silently ([5dd7973](https://github.com/jonperron/fihr-linter/commit/5dd7973bbf210f7a2a04e3488e4016646fe64517))

## [0.2.0](https://github.com/jonperron/fihr-linter/compare/fhir-linter-v0.1.0...fhir-linter-v0.2.0) (2026-11-01)


### Features

* **agents:** bootstrap agents ([88e5fae](https://github.com/jonperron/fihr-linter/commit/88e5faeef6a1012eb797e4da42836879c53e4f4d))
* **agents:** bootstrap agents ([ff96ec7](https://github.com/jonperron/fihr-linter/commit/ff96ec7c0b7169fcef1071e6328f6d2032f60171))
* **fhir-definitions:** registry and model ([435dbad](https://github.com/jonperron/fihr-linter/commit/435dbad29cc5ec2b02bc5655fdb94c575cc690da))
* **fhir-definitions:** registry and model ([3bb5b3e](https://github.com/jonperron/fihr-linter/commit/3bb5b3e9ba3a9c708c7288ea994478d041f7420a))
* **fhir-fhirpath:** implement FHIRPath 2.0 evaluator ([cf6bcfa](https://github.com/jonperron/fihr-linter/commit/cf6bcfa8aec791fddee75767949ea2eef28734df))
* **fhir-fhirpath:** upgrade evaluator to FHIRPath 3.0 ([78ef577](https://github.com/jonperron/fihr-linter/commit/78ef5779eff21948cec7df6987901edae9d9cd13))
* **fhir-fhirpath:** upgrade evaluator to FHIRPath 3.0 ([ca061d1](https://github.com/jonperron/fihr-linter/commit/ca061d1b1d4c4b248b22bd2e122d814db3d7c5e6))
* **fhir-parser:** implement JSON parser with source spans ([64feb71](https://github.com/jonperron/fihr-linter/commit/64feb71d9284000f59fe05407872ceace2d1367e))
* **fhir-parser:** implement JSON parser with source spans ([583c008](https://github.com/jonperron/fihr-linter/commit/583c008560fb0d70dc17981f33a413140e12f7d7))
* **fhir-parser:** implement XML parser ([729a416](https://github.com/jonperron/fihr-linter/commit/729a416d2efb953e421ae4e1769f6606a5032f64))
* **fhir-parser:** implement XML parser (phase 3) ([5ce89b1](https://github.com/jonperron/fihr-linter/commit/5ce89b19e2ab3eff92461925327835862c931b06))
* **fhir-validator:** implement core validator ([771a126](https://github.com/jonperron/fihr-linter/commit/771a1263462741b62d7ff263e20db0284713c3ed))


### Bug Fixes

* **fhir-definitions,scripts:** reject malformed inputs at parse boundaries ([020012d](https://github.com/jonperron/fihr-linter/commit/020012d45fe7fae6d056d76d6c0f5553fc414ef7))
* **fhir-parser:** preserve non-value attributes and reject trailing elements ([a74e6fa](https://github.com/jonperron/fihr-linter/commit/a74e6fa7049535abcddc0907a7e597e65004586a))
* **security:** harden download script and forbid unsafe code ([fea6011](https://github.com/jonperron/fihr-linter/commit/fea6011837e2e77948409dfe34726881b6555d32))
* **typo:** fix types and json parser ([a1e4bc5](https://github.com/jonperron/fihr-linter/commit/a1e4bc5d486104192c7724e2663fe121b0322a78))
* **typo:** fix types and json parser ([99245d1](https://github.com/jonperron/fihr-linter/commit/99245d102cd616099af37f5cfce1cdf1d93aca78))
