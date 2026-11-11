# Changelog

## [0.5.2](https://github.com/jonperron/fihr-linter/compare/fhir-linter-v0.5.1...fhir-linter-v0.5.2) (2026-11-11)


### Bug Fixes

* **ci:** restrict publishing to main and keep queued retries ([aee7dcb](https://github.com/jonperron/fihr-linter/commit/aee7dcb1c37c1222a8d5ce770927964306475d58))
* **fhirpath:** navigate choice elements by their base name ([2414ded](https://github.com/jonperron/fihr-linter/commit/2414ded6b310782c04a10daa757bcb2e8377a90c))
* **fhirpath:** return undefined function for unimplemented stubs ([650df9b](https://github.com/jonperron/fihr-linter/commit/650df9bc43abfc1a8945eb12b707e0bd1fbc9812))
* **fhirpath:** route the is operator through the is() function logic ([aaf64c3](https://github.com/jonperron/fihr-linter/commit/aaf64c3f6c3d590b7487571c321bf79dc1726a07))
* **fhirpath:** undefined stubs, choice navigation and is operator ([#65](https://github.com/jonperron/fihr-linter/issues/65)) ([b995921](https://github.com/jonperron/fihr-linter/commit/b9959211aa61b11c96e6d8af558acd9b311e11f8))
* **parser:** accept a leading utf-8 bom in json ([b197375](https://github.com/jonperron/fihr-linter/commit/b1973753ccf06af8222079331ba977963be51e86))
* **parser:** flag control characters in json and xml strings ([6b48955](https://github.com/jonperron/fihr-linter/commit/6b4895513d00d1141a49afcc136519a3da8b7187))
* **parser:** json -0, control characters, utf-8 bom and byte offsets ([#68](https://github.com/jonperron/fihr-linter/issues/68)) ([48560a5](https://github.com/jonperron/fihr-linter/commit/48560a5d027d46a0c3869cddf4942017b0ba8c05))
* **parser:** keep a literal json -0 as a decimal ([255923d](https://github.com/jonperron/fihr-linter/commit/255923d1163b3a1f029cf8432013ed221b2cdd6d))
* **parser:** reject literal xml control characters anywhere ([61b6dd6](https://github.com/jonperron/fihr-linter/commit/61b6dd6a69c5175109d25a83ffeac2645ad46a32))
* **release:** publish to registries from the top-level workflow ([cd9aac0](https://github.com/jonperron/fihr-linter/commit/cd9aac0675165f250c6884003e3eaa30c91c808e))
* **release:** publish to registries from the top-level workflow ([#69](https://github.com/jonperron/fihr-linter/issues/69)) ([d4cb432](https://github.com/jonperron/fihr-linter/commit/d4cb4329f863de8aff8c84931c552bec775253c2))
* **validator:** check extensible bindings as warnings ([195bcca](https://github.com/jonperron/fihr-linter/commit/195bccadf2b5b93493022e95d6c239b347528098))
* **validator:** clearer text-only terminology message ([361425d](https://github.com/jonperron/fihr-linter/commit/361425d438454ad69c63318ff5c4bcec6389c1fe))
* **validator:** compare reference type with a fhir url reference ([ea7d305](https://github.com/jonperron/fihr-linter/commit/ea7d305cf9cd50ab3eae6efe3da1d4c8442887dc))
* **validator:** empty arrays, code format and date-only zones ([ce141b8](https://github.com/jonperron/fihr-linter/commit/ce141b8379f06df9a5556104a73bfde5c675d2a1))
* **validator:** follow the r5 regexes for code and dateTime zones ([bdb10d4](https://github.com/jonperron/fihr-linter/commit/bdb10d4626c80ccfc654accc505eda93cd0250c4))
* **validator:** read a single xml coding in terminology checks ([02fd4e6](https://github.com/jonperron/fihr-linter/commit/02fd4e61a5d01aa9bb50df4ce8855f154e864b3a))
* **validator:** reject a bare hash reference at the root ([a80a9bc](https://github.com/jonperron/fihr-linter/commit/a80a9bc752f76f5596e6efd4122dc0d825445f25))
* **validator:** reject text-only codeable concept under required binding ([62accfa](https://github.com/jonperron/fihr-linter/commit/62accfa4a16eae1e3247a5b38f2cde6665855b6f))
* **validator:** terminology and reference audit fixes (V2, V4, V5, V7, V11) ([#70](https://github.com/jonperron/fihr-linter/issues/70)) ([1b8122e](https://github.com/jonperron/fihr-linter/commit/1b8122e765ccf95c2c7b979e16a5242243bb7c0d))
* **validator:** treat a coding without system as not a member ([87b88a4](https://github.com/jonperron/fihr-linter/commit/87b88a46f9748d2b66b04e409f2fe9bc49e6c9f3))
* **validator:** treat empty or all-null arrays as absent ([7b69847](https://github.com/jonperron/fihr-linter/commit/7b69847a4d7f4e3022405bd07c4fa81baa315862))
* **validator:** treat only a uri scheme as an absolute reference ([36eaf40](https://github.com/jonperron/fihr-linter/commit/36eaf400267a33c744f117053c4d7adaa255993f))
* **validator:** warn on a malformed relative reference ([51097eb](https://github.com/jonperron/fihr-linter/commit/51097eba12a7a4f034fb4e8a9dd7b65e00588c6a))
* **validator:** warn on control characters in the resource id ([610de3f](https://github.com/jonperron/fihr-linter/commit/610de3f8d0c447e793a13e309dead2c77492d158))

## [0.5.1](https://github.com/jonperron/fihr-linter/compare/fhir-linter-v0.5.0...fhir-linter-v0.5.1) (2026-11-10)


### Bug Fixes

* **linter:** close narrative, identifier and profile rule gaps ([83fb381](https://github.com/jonperron/fihr-linter/commit/83fb381c310e4653f7d94f21403d33549048c617))
* **linter:** limit bundle missing-identifier to persistent types ([3992212](https://github.com/jonperron/fihr-linter/commit/3992212ad5dc6ba94adc4028339d54f95c3dacae))
* **linter:** locate identifier and profile findings at meta ([bdf8094](https://github.com/jonperron/fihr-linter/commit/bdf8094537f91794ff4c7f99523e26bb1c1220ec))
* **linter:** report narrative with status empty as absent ([3b8eac5](https://github.com/jonperron/fihr-linter/commit/3b8eac5bca0d959eea1deca52fe2e6f3e256b09e))
* **linter:** scan narrative xhtml properly and read xml primitives ([5e80ae3](https://github.com/jonperron/fihr-linter/commit/5e80ae37c2cb743c3a86b92524dce7dd67839b63))
* **linter:** treat a markup-only narrative div as empty ([a06b2bd](https://github.com/jonperron/fihr-linter/commit/a06b2bd887c5123961bbf5d0113929a837337026))
* **linter:** treat empty identifier and profile arrays as missing ([fb208f5](https://github.com/jonperron/fihr-linter/commit/fb208f55c680637b69074c12c2dc405ba461656c))
* **release:** publish npm to github packages and python to pypi ([c4fb6e6](https://github.com/jonperron/fihr-linter/commit/c4fb6e6afa3284b0780cd7a728ec16d3aff6443c))
* **release:** publish npm to github packages and python to pypi ([5fd6151](https://github.com/jonperron/fihr-linter/commit/5fd615176ddffc634bbcf83b426d5e97f6e3ec94))

## [0.5.0](https://github.com/jonperron/fihr-linter/compare/fhir-linter-v0.4.1...fhir-linter-v0.5.0) (2026-11-09)


### Features

* **fhirpath:** parse backtick-delimited identifiers ([05baada](https://github.com/jonperron/fihr-linter/commit/05baada7c82f1ed4dfb60dcf47132ae8b301c1ea))
* **fhirpath:** parse backtick-delimited identifiers ([9610844](https://github.com/jonperron/fihr-linter/commit/9610844ffb53099152ef5c54bc8e92682153abec))
* **release:** attach cli, wheels and npm tarball to github releases ([bca1c64](https://github.com/jonperron/fihr-linter/commit/bca1c649470bf28d5cabba981c25aa3679cc580d))


### Bug Fixes

* **ci:** match crate Cargo.toml files in ci path filters ([8535218](https://github.com/jonperron/fihr-linter/commit/85352187698d16edf51fdeb393a53cf13e50b453))
* **ci:** match crate Cargo.toml files in ci path filters ([bfe82b9](https://github.com/jonperron/fihr-linter/commit/bfe82b9b84d35dc730cab3d28359908172ffa2c5))
* **ci:** pin release-please action to a commit sha ([7f044c4](https://github.com/jonperron/fihr-linter/commit/7f044c40c353bc31cc31941c6e1b472e9b600274))
* **ci:** pin release-please action to a commit sha ([04b74c6](https://github.com/jonperron/fihr-linter/commit/04b74c67d597900bd6b4d92a4e0932f3b0e5353f))
* **fhirpath:** add is() and as() function forms ([e9194d3](https://github.com/jonperron/fihr-linter/commit/e9194d3e2bf4d51b62a15c7cd651b7491bfec155))
* **fhirpath:** add is() and as() function forms ([e444051](https://github.com/jonperron/fihr-linter/commit/e444051850a91db4f0fd3756edf93c8fad1f86e6))
* **fhirpath:** allow expressions to start with the context type name ([5cb716f](https://github.com/jonperron/fihr-linter/commit/5cb716f9ef2cd4e1e51b7114f8cf93112ca27c68))
* **fhirpath:** allow expressions to start with the context type name ([174ecb0](https://github.com/jonperron/fihr-linter/commit/174ecb071a1a8e4bbb7db4e3043dfab913bafc9b))
* **fhirpath:** restrict type name prefix to the first path segment ([84c4e35](https://github.com/jonperron/fihr-linter/commit/84c4e35084aaf2c1f17c24bf220b9246992e0d33))
* **fhirpath:** return empty collection for out-of-range index ([d193b62](https://github.com/jonperron/fihr-linter/commit/d193b628d42c4b58311479d7e0e27acc8d2a7097))
* **fhirpath:** return empty collection for out-of-range index ([f0cd4cf](https://github.com/jonperron/fihr-linter/commit/f0cd4cfff3d4ce192eb6674f2b2be666366b638e))
* **release:** keep cargo.lock in sync with the release version ([04f525b](https://github.com/jonperron/fihr-linter/commit/04f525b019bc7cc8a34eae627e4ca1fbaa7cdade))
* **release:** leave the stub cli out of release assets ([9ce52a1](https://github.com/jonperron/fihr-linter/commit/9ce52a1edacfff3980b54c284be5c6a2bb9feb0e))
* **release:** pin the rust toolchain and manylinux image ([7a70e72](https://github.com/jonperron/fihr-linter/commit/7a70e72f6cf5440b608bfb271197ebe1a1ed310c))
* **release:** publish the release only after its assets are uploaded ([db50792](https://github.com/jonperron/fihr-linter/commit/db507927d5d940db0e664978eebd7a9739684960))
* **validator:** enforce json array shape of repeating elements ([e8a2bab](https://github.com/jonperron/fihr-linter/commit/e8a2babca98c45bf81c0a09fb4eb56b7b620e808))
* **validator:** enforce json array shape of repeating elements ([4c0c072](https://github.com/jonperron/fihr-linter/commit/4c0c0725a7aad4200b4e981e74d0897e0a037400))
* **validator:** name non-concrete types in structure_000 message ([376340c](https://github.com/jonperron/fihr-linter/commit/376340c3cfe79d1ef7dce23dbdb55cb0eda924ee))
* **validator:** reject abstract and non-resource resource types ([26321fa](https://github.com/jonperron/fihr-linter/commit/26321fa5ee12a627f41885a8861e782634835909))
* **validator:** reject abstract and non-resource resource types ([6043d21](https://github.com/jonperron/fihr-linter/commit/6043d21dab2b9d5540204beb2cda399ac2928bcc))
* **validator:** reject unknown _name keys and walk primitive extensions ([d25d6ee](https://github.com/jonperron/fihr-linter/commit/d25d6eeb8b0d0705dc9caff08a1f8a56e4a88672))
* **validator:** reject unknown _name keys and walk primitive extensions ([da504d8](https://github.com/jonperron/fihr-linter/commit/da504d8401b342386a5316c6d857d49b89ac756f))
* **validator:** skip choice slices in cardinality check ([be0e075](https://github.com/jonperron/fihr-linter/commit/be0e075559fd6ba44c4e8d4cd95b8feaafd01558))
* **validator:** skip slice elements in profile cardinality check ([365c3bb](https://github.com/jonperron/fihr-linter/commit/365c3bb1a9258f08f1977f2e34bf886531605880))
* **validator:** skip slice elements in profile cardinality check ([0442d2b](https://github.com/jonperron/fihr-linter/commit/0442d2b08b609037c621977e368bc084dbad2f7a))
* **validator:** type XML primitive extensions and keep empty primitives ([6607001](https://github.com/jonperron/fihr-linter/commit/6607001642869986b466624758a39a59efd4f2ac))
* **validator:** type XML primitives and split their extensions ([d0c56b4](https://github.com/jonperron/fihr-linter/commit/d0c56b49fd7222e8b91a7d371b8625ca67cf2ba2))
* **validator:** type XML primitives and split their extensions ([4ee84b2](https://github.com/jonperron/fihr-linter/commit/4ee84b228f39a96671d08066d93b31e788d9dc5e))

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
