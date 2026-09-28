## [0.11.0](https://github.com/0x676e67/wreq-python/compare/v0.10.2..v0.11.0) - 2026-04-13

### Features

- *(http1/http2)* Implement `__str__` for types ([#559](https://github.com/0x676e67/wreq/issues/559)) - ([e85ac65](https://github.com/0x676e67/wreq-python/commit/e85ac657f63cfb798061f7a27e9e69b82bfb70a8))

### Refactor

- *(tls)* Standardize TLS config fields with `tls_` prefix ([#556](https://github.com/0x676e67/wreq/issues/556)) - ([5504542](https://github.com/0x676e67/wreq-python/commit/5504542fcc92356459fe186e434de53f738f2c91))

### Documentation

- *(python)* Prefer `Mapping` over `Dict` for abstract type hints - ([d7ab30e](https://github.com/0x676e67/wreq-python/commit/d7ab30e65a168c7663e4bd29146bc7cfbabc32a4))

### Miscellaneous Tasks

- Change license from GPLv3 to Apache-2.0 - ([7e4851a](https://github.com/0x676e67/wreq-python/commit/7e4851a260542a24decd7433f97190c06868de17))

### Bench

- *(python)* Use `pyperf` strict mode over custom logic ([#561](https://github.com/0x676e67/wreq/issues/561)) - ([2eca676](https://github.com/0x676e67/wreq-python/commit/2eca676bac064b098c895d9c789bf4b29c5e6ffd))

### Build

- *(ci)* Fix free-threaded wheel builds on Windows via maturin ([#560](https://github.com/0x676e67/wreq/issues/560)) - ([5d39a3a](https://github.com/0x676e67/wreq-python/commit/5d39a3a988a1515639745a10a90f51df6a23502e))
- *(deps)* Bump wreq version to latest ([#562](https://github.com/0x676e67/wreq/issues/562)) - ([c33b4b4](https://github.com/0x676e67/wreq-python/commit/c33b4b4cfa4b0c310ce0227afecddd209ca71c6f))
- *(deps)* Update `http` dependency version to 1.4.0 ([#553](https://github.com/0x676e67/wreq/issues/553)) - ([af16c66](https://github.com/0x676e67/wreq-python/commit/af16c668aa1bfe3bdc8a5113fae1f4370ad7283f))
- *(deps)* Update `tokio` dependency version to 1.51.0 ([#552](https://github.com/0x676e67/wreq/issues/552)) - ([2ee6fda](https://github.com/0x676e67/wreq-python/commit/2ee6fda3de65ac9f88de9ef3a835e121a3e78c07))
- *(deps)* Update `arc-swap` dependency to version 1.9.0 ([#551](https://github.com/0x676e67/wreq/issues/551)) - ([e479f49](https://github.com/0x676e67/wreq-python/commit/e479f49ae4d26c1f70e56ada52b416b7fc32a820))
- *(deps)* Update `pyo3` dependency version to 0.28.3 ([#550](https://github.com/0x676e67/wreq/issues/550)) - ([94e1d30](https://github.com/0x676e67/wreq-python/commit/94e1d303feb87be66e24d4a7225e9415228435a5))
- *(deps)* Update bench optional dependencies - ([1685d14](https://github.com/0x676e67/wreq-python/commit/1685d14572a59097d0e07af97d6b35f22c0fa8f2))

## New Contributors ❤️

* @tjaycodes4you made their first contribution in [#557](https://github.com/0x676e67/wreq-python/pull/557)

## [0.10.2](https://github.com/0x676e67/wreq-python/compare/v0.10.1..v0.10.2) - 2026-03-27

### Features

- *(client)* Implement graceful shutdown for Client ([#543](https://github.com/0x676e67/wreq/issues/543)) - ([e844ff3](https://github.com/0x676e67/wreq-python/commit/e844ff30bdb3cabd40ff07974a18f96c4620eb66))

### Bug Fixes

- *(response)* Ensure response remains cached when valid ([#544](https://github.com/0x676e67/wreq/issues/544)) - ([5f7e3ba](https://github.com/0x676e67/wreq-python/commit/5f7e3ba259569871cc56b3c09e654d3df60efe2d))

### Miscellaneous Tasks

- *(buffer)* Add `skip_from_py_object` to `BufferView` pyclass - ([8cae362](https://github.com/0x676e67/wreq-python/commit/8cae36263ca247886c987a5a171f0f74d3406b14))
- *(client)* Replace cancel with close in async exit handling - ([5b09884](https://github.com/0x676e67/wreq-python/commit/5b09884964e1218153f268adb9309f1199a03737))


## [0.10.1](https://github.com/0x676e67/wreq-python/compare/v0.10.0..v0.10.1) - 2026-03-27

### Documentation

- *(ws)* Update response attributes documentation - ([3858d42](https://github.com/0x676e67/wreq-python/commit/3858d4200458e63dc4f309dfc9b0d56d04ffeaeb))
- *(ws)* Expose local_addr in response docs ([#542](https://github.com/0x676e67/wreq/issues/542)) - ([6f92e6c](https://github.com/0x676e67/wreq-python/commit/6f92e6ced24051bba368fda25eb71dce50e572dd))
- Migrate to Zensical for documentation ([#540](https://github.com/0x676e67/wreq/issues/540)) - ([b41a913](https://github.com/0x676e67/wreq-python/commit/b41a91336c39c80bc1350128b65a5e0340b6ce9f))
- Update documentation URL in pyproject.toml - ([5dc16c6](https://github.com/0x676e67/wreq-python/commit/5dc16c6a17010f37819f13fb8d03b50e39ddf9c2))

### Build

- *(deps)* Streamline dependency management ([#541](https://github.com/0x676e67/wreq/issues/541)) - ([382c154](https://github.com/0x676e67/wreq-python/commit/382c154e66cd55df85aaff4bd01c2f63a91608c3))


## [0.10.0](https://github.com/0x676e67/wreq-python/compare/v3.0.0-rc22..v0.10.0) - 2026-03-26

### Features

- *(client)* Implement context manager protocol ([#533](https://github.com/0x676e67/wreq/issues/533)) - ([3f76605](https://github.com/0x676e67/wreq-python/commit/3f76605792d6217734b9a59aa52414f98bde09f9))
- *(response)* Clear cached data when response is dropped ([#535](https://github.com/0x676e67/wreq/issues/535)) - ([c65346e](https://github.com/0x676e67/wreq-python/commit/c65346e447dfca44a49583793c6654c76ce0fbae))

### Documentation

- *(python)* Change return type of  `__enter__` method to `Any` ([#534](https://github.com/0x676e67/wreq/issues/534)) - ([7d592f9](https://github.com/0x676e67/wreq-python/commit/7d592f9304f6731e27d1ee0265a928cf1b3a7b41))

### Miscellaneous Tasks

- Update commit parsers in cliff.toml - ([e60d510](https://github.com/0x676e67/wreq-python/commit/e60d510d8ca7152c5bc398a07d1083dc5551c651))

### Build

- *(deps)* Bump actions/attest-build-provenance from 3 to 4 ([#529](https://github.com/0x676e67/wreq/issues/529)) - ([b32777b](https://github.com/0x676e67/wreq-python/commit/b32777b083c33e3d3ec22a48bbff2235b01149a7))
- *(deps)* Bump actions/download-artifact from 7 to 8 ([#528](https://github.com/0x676e67/wreq/issues/528)) - ([4b0ea46](https://github.com/0x676e67/wreq-python/commit/4b0ea466645b538c3e0852031d113b59614c6e04))
- *(deps)* Bump actions/upload-artifact from 6 to 7 ([#527](https://github.com/0x676e67/wreq/issues/527)) - ([23366c7](https://github.com/0x676e67/wreq-python/commit/23366c749c2015fd61272435c1466bc6cba2b7fe))


## [3.0.0-rc22](https://github.com/0x676e67/wreq-python/compare/v3.0.0-rc21..v3.0.0-rc22) - 2026-02-24

### Refactor

- *(response)* Merge `text` and `text_with_charset` into a single method ([#524](https://github.com/0x676e67/wreq/issues/524)) - ([542a610](https://github.com/0x676e67/wreq-python/commit/542a610ebf1c67e93958dcfbcca854e0310a896f))


## [3.0.0-rc21](https://github.com/0x676e67/wreq-python/compare/v3.0.0-rc20..v3.0.0-rc21) - 2026-02-18

### Features

- *(emulation)* Update emulation device enums from wreq-util rc.10 ([#516](https://github.com/0x676e67/wreq/issues/516)) - ([eb25f00](https://github.com/0x676e67/wreq-python/commit/eb25f0099cf1381848330cd3c3924f4cadb90012))
- Add sync shortcut request method ([#514](https://github.com/0x676e67/wreq/issues/514)) - ([274959a](https://github.com/0x676e67/wreq-python/commit/274959a6037552f2b13b11bfcb68abc828334933))

### Refactor

- *(request)* Simplify multipart body extraction ([#510](https://github.com/0x676e67/wreq/issues/510)) - ([8ccacb2](https://github.com/0x676e67/wreq-python/commit/8ccacb27b57bb85d3dc7bc055c29a3d96b03417b))
- *(ws)* Simplify Message creation ([#517](https://github.com/0x676e67/wreq/issues/517)) - ([c35c289](https://github.com/0x676e67/wreq-python/commit/c35c2893a127b4ecb368ab2a1cb8c7d1c758f2e1))

### Documentation

- *(response)* Clarify close() method behavior and recommend context managers ([#507](https://github.com/0x676e67/wreq/issues/507)) - ([daa2871](https://github.com/0x676e67/wreq-python/commit/daa2871dfd126ad01a0cb27003e98ea38541428b))
- *(stream)* Improve comments on GIL acquisition - ([8a2a127](https://github.com/0x676e67/wreq-python/commit/8a2a127dfe0c967b8f5537a143e816990c3ca31b))
- Fix api docs ([#509](https://github.com/0x676e67/wreq/issues/509)) - ([49a5861](https://github.com/0x676e67/wreq-python/commit/49a58615992b5367b2f1ad919629baff435d63c0))
- Update documentation site - ([3d0b0c2](https://github.com/0x676e67/wreq-python/commit/3d0b0c2f71b59d12a726b779679cc165feab12f0))
- Add documentation site ([#498](https://github.com/0x676e67/wreq/issues/498)) - ([c0d0f18](https://github.com/0x676e67/wreq-python/commit/c0d0f18a6e9160e9ee0979249a1b0a794586abd7))

### Performance

- *(rt)* Use native async for context management ([#511](https://github.com/0x676e67/wreq/issues/511)) - ([30c0136](https://github.com/0x676e67/wreq-python/commit/30c013660cc96362c985639386246163aabe43f1))
- *(stream)* Refactor `Streamer` for compatibility with frozen async state ([#513](https://github.com/0x676e67/wreq/issues/513)) - ([83a0782](https://github.com/0x676e67/wreq-python/commit/83a0782b909a43635ca365c45a5b08e9b517bae2))

### Miscellaneous Tasks

- *(proxy)* Fmt code - ([cc381f5](https://github.com/0x676e67/wreq-python/commit/cc381f555ed2fe39d9e462fc361687623b3db0c2))
- Update `wreq-util` features in Cargo.toml ([#521](https://github.com/0x676e67/wreq/issues/521)) - ([95de438](https://github.com/0x676e67/wreq-python/commit/95de43865620ddbb0ee385ac5fdbe81edc581cd5))
- Update docs theme - ([069462b](https://github.com/0x676e67/wreq-python/commit/069462bbf813d1e9b031331beaf88208436fa8ee))
- Update docs module ([#508](https://github.com/0x676e67/wreq/issues/508)) - ([f4d9bf2](https://github.com/0x676e67/wreq-python/commit/f4d9bf2a2b5ecec0f87b0d290bee8c1045676d67))
- Fix docs format - ([2f91ea9](https://github.com/0x676e67/wreq-python/commit/2f91ea9570dcc9cf6b064db8792984a11a77345c))

### Build

- *(deps)* Update arc-swap dependency to version 1.8.0 - ([302cd9b](https://github.com/0x676e67/wreq-python/commit/302cd9b5d25ec67a58c3e523bc65d0eb08b607fa))
- *(deps)* Update indexmap dependency to version 2.13.0 ([#503](https://github.com/0x676e67/wreq/issues/503)) - ([95a460f](https://github.com/0x676e67/wreq-python/commit/95a460f52071d02ce8683988cbb2f3b472737f49))
- *(deps)* Bump actions/setup-python from 5 to 6 ([#500](https://github.com/0x676e67/wreq/issues/500)) - ([b46bee4](https://github.com/0x676e67/wreq-python/commit/b46bee4ce3b83c5bd10a7de5d524574e3ba57948))
- *(deps)* Bump actions/checkout from 4 to 6 ([#499](https://github.com/0x676e67/wreq/issues/499)) - ([40b2335](https://github.com/0x676e67/wreq-python/commit/40b23356382667d21f362474a3a9cb722a50c48d))

## New Contributors ❤️

* @steppaovo made their first contribution in [#516](https://github.com/0x676e67/wreq-python/pull/516)

## [3.0.0-rc20](https://github.com/0x676e67/wreq-python/compare/v3.0.0-rc17..v3.0.0-rc20) - 2026-01-17

### Features

- *(client)* Expose client cookie jar access ([#477](https://github.com/0x676e67/wreq/issues/477)) - ([034d0e3](https://github.com/0x676e67/wreq-python/commit/034d0e365204e07afaa3f3a5ec50b3ee91d3e81e))
- *(cookie)* Remove deprecated cookie add APIs ([#492](https://github.com/0x676e67/wreq/issues/492)) - ([76788ac](https://github.com/0x676e67/wreq-python/commit/76788ac562bb39a40c119168d8af25601ca36514))
- *(cookie)* Consolidate cookie methods into a unified `add()` ([#479](https://github.com/0x676e67/wreq/issues/479)) - ([5c44f4f](https://github.com/0x676e67/wreq-python/commit/5c44f4f2756c51abf214d6120f28fc9ca12ddf9b))
- *(http)* Implement rich comparison for `StatusCode` with integers ([#490](https://github.com/0x676e67/wreq/issues/490)) - ([65af91e](https://github.com/0x676e67/wreq-python/commit/65af91e257112fd48212422825d8cd3b0e420660))
- *(request)* Allow setting compressed and uncompressed cookies ([#491](https://github.com/0x676e67/wreq/issues/491)) - ([b268564](https://github.com/0x676e67/wreq-python/commit/b2685648326ac3174177de6a0a733d01ceffebc9))
- *(runtime)* Add async task cancellation support ([#495](https://github.com/0x676e67/wreq/issues/495)) - ([23dc0c4](https://github.com/0x676e67/wreq-python/commit/23dc0c4d8dfdabb73069613e640b0e2d2256459e))

### Refactor

- *(response)* Refactor `TlsInfo` extension wrapper ([#481](https://github.com/0x676e67/wreq/issues/481)) - ([8195fb6](https://github.com/0x676e67/wreq-python/commit/8195fb6d887438a35e6b5eba4aa06c51be664fc7))

### Documentation

- *(client)* Update client configuration parameters ([#482](https://github.com/0x676e67/wreq/issues/482)) - ([4c01ab1](https://github.com/0x676e67/wreq-python/commit/4c01ab14520cb39cacacbe6c1a0c6e8ebcc4de14))
- *(cookie)* Fix compression parameter type in cookie constructor ([#480](https://github.com/0x676e67/wreq/issues/480)) - ([8bcf8fc](https://github.com/0x676e67/wreq-python/commit/8bcf8fc712783053bb5cf13253413ef83ab5e08d))
- *(python)* Clean up `KeyLog` class documentation - ([8460b0a](https://github.com/0x676e67/wreq-python/commit/8460b0a11f88f39f657f96bb08f807aadfad602f))
- *(tls)* Fix duplicate field in TlsOptions class - ([34cbac8](https://github.com/0x676e67/wreq-python/commit/34cbac8c509675ffec566f8889a44b4d23909e72))

### Performance

- *(config)* Avoid unnecessary configurations ([#488](https://github.com/0x676e67/wreq/issues/488)) - ([c7e9dda](https://github.com/0x676e67/wreq-python/commit/c7e9ddab8de4891ee42c9a4a9e98e567d6096a34))
- *(identity)* Optimize mTLS identity creation performance ([#485](https://github.com/0x676e67/wreq/issues/485)) - ([6b9e00e](https://github.com/0x676e67/wreq-python/commit/6b9e00e959663f58dcbdcc5d18e3aa07af32413f))
- *(response)* Reduce response initialization overhead ([#494](https://github.com/0x676e67/wreq/issues/494)) - ([790d958](https://github.com/0x676e67/wreq-python/commit/790d958775cf9cb18259adb707f5dd5b46a7b7ac))

### Testing

- *(cookie)* Improve cookie update tests ([#487](https://github.com/0x676e67/wreq/issues/487)) - ([f682724](https://github.com/0x676e67/wreq-python/commit/f682724b3be2f6526725ad5dbc915379b2b33229))

### Miscellaneous Tasks

- *(cookie)* Simplify getter methods by removing inline attributes ([#484](https://github.com/0x676e67/wreq/issues/484)) - ([23dc825](https://github.com/0x676e67/wreq-python/commit/23dc825067af2e3f30ef7e3a20f39327e546eb2b))
- Add .csv and .py files to .gitignore - ([ec3a534](https://github.com/0x676e67/wreq-python/commit/ec3a534aa65d3ab022c66b1959b528dcbadc5f86))

### Build

- *(deps)* Bump wreq version to 6.0.0-rc.27 ([#496](https://github.com/0x676e67/wreq/issues/496)) - ([23e3c28](https://github.com/0x676e67/wreq-python/commit/23e3c288d0c5dab745b016712a9bbc71f34f90ed))
- Fix compiled wheels being unusable ([#497](https://github.com/0x676e67/wreq/issues/497)) - ([c80a6a3](https://github.com/0x676e67/wreq-python/commit/c80a6a3a46a4ebf68519eb5a8237dc2f3f3bdb48))
- Mutually exclusive `jemalloc`/`mimalloc` allocator features ([#478](https://github.com/0x676e67/wreq/issues/478)) - ([b5d7c42](https://github.com/0x676e67/wreq-python/commit/b5d7c429faa47303d6c4ee703bee366dbe9fbc73))

### Deps

- Update tokio dependency version to 1.49.0 - ([19539e8](https://github.com/0x676e67/wreq-python/commit/19539e870a2acfe549b17c5f04d9cf7d17b5068c))

## New Contributors ❤️

* @serozhenka made their first contribution in [#477](https://github.com/0x676e67/wreq-python/pull/477)

## [3.0.0-rc17](https://github.com/0x676e67/wreq-python/compare/v3.0.0-rc16..v3.0.0-rc17) - 2025-12-31

### Features

- *(emulation)* Update emulation device enums ([#475](https://github.com/0x676e67/wreq/issues/475)) - ([6ddf10d](https://github.com/0x676e67/wreq-python/commit/6ddf10dd030eca5a93b285ec17e5c8de2eedb4bd))

### Build

- *(deps)* Bump `wreq` version to 6.0.0-rc.26 ([#474](https://github.com/0x676e67/wreq/issues/474)) - ([d784227](https://github.com/0x676e67/wreq-python/commit/d784227a9e4c886cdbdfa6313205dabd4fa34354))


## [3.0.0-rc16](https://github.com/0x676e67/wreq-python/compare/v3.0.0-rc15..v3.0.0-rc16) - 2025-12-22

### Features

- *(client)* Enforce proper usage of custom cookie provider ([#463](https://github.com/0x676e67/wreq/issues/463)) - ([31dfc88](https://github.com/0x676e67/wreq-python/commit/31dfc88f45cb3441022eca7573413e51ebdb0f00))
- *(error)* Add `ProxyConnectionError` for proxy connection errors ([#464](https://github.com/0x676e67/wreq/issues/464)) - ([5bd789f](https://github.com/0x676e67/wreq-python/commit/5bd789f17454a39dec3f2c405a657dd05e1fd9c4))
- *(response)* Introduce trailers support ([#467](https://github.com/0x676e67/wreq/issues/467)) - ([9ccdefd](https://github.com/0x676e67/wreq-python/commit/9ccdefd0631a6864b9af44bedfaa6a5ba6381972))

### Documentation

- *(python)* Add `ProxyConnectionError` to exceptions list - ([75eb571](https://github.com/0x676e67/wreq-python/commit/75eb5713bafed733dfb142c13aa730da61b742a3))
- *(stream)* Introduce Frame enum with PyBuffer variant - ([04ff163](https://github.com/0x676e67/wreq-python/commit/04ff163527720f0747bb57b519205bc47a50af82))


## [3.0.0-rc15](https://github.com/0x676e67/wreq-python/compare/v3.0.0-rc14..v3.0.0-rc15) - 2025-12-16

### Features

- *(client/request)* Add dual-stack local IP address binding ([#436](https://github.com/0x676e67/wreq/issues/436)) - ([8f27578](https://github.com/0x676e67/wreq-python/commit/8f2757899d70e4c3300d7bf36887a44a821a6733))
- *(cookie)* Add optional cookie jar compression strategy ([#448](https://github.com/0x676e67/wreq/issues/448)) - ([ea1d187](https://github.com/0x676e67/wreq-python/commit/ea1d187653db5e8d5aabce4be2a04a033ccc6d7c))
- *(exception)* Remove unused `URLParseError` exception types ([#447](https://github.com/0x676e67/wreq/issues/447)) - ([cef2ac8](https://github.com/0x676e67/wreq-python/commit/cef2ac8585c7f10181582448022168aa98b389b6))
- *(proxy)* Implement __str__ for `Proxy` class ([#452](https://github.com/0x676e67/wreq/issues/452)) - ([0e319a1](https://github.com/0x676e67/wreq-python/commit/0e319a117b7a0191e1938611a09a75ce6ac1187e))
- *(proxy)* Move to proxy module and support unix sockets ([#449](https://github.com/0x676e67/wreq/issues/449)) - ([bd1dbff](https://github.com/0x676e67/wreq-python/commit/bd1dbffade9002c31aa976c70fdfe6c9b3b46f35))
- *(redirect)* Refactor redirect policy with callback support ([#440](https://github.com/0x676e67/wreq/issues/440)) - ([80db9c3](https://github.com/0x676e67/wreq-python/commit/80db9c39c2c49cd8e52682af001809e316898162))
- *(request)* Allow per-request cookie provider ([#450](https://github.com/0x676e67/wreq/issues/450)) - ([8ef54c6](https://github.com/0x676e67/wreq-python/commit/8ef54c6c55bf8a79c1f3236b337cdd4020f7160d))
- *(request)* Auto-convert multiple types in query/form parameters ([#434](https://github.com/0x676e67/wreq/issues/434)) - ([a3ac24e](https://github.com/0x676e67/wreq-python/commit/a3ac24ef93ca3fc517c3c5a402960138f026ef2a))
- *(resp)* Implement `__str__` for response class ([#439](https://github.com/0x676e67/wreq/issues/439)) - ([95846f9](https://github.com/0x676e67/wreq-python/commit/95846f9e695e01cc972b1acc9c012910dfb09790))

### Refactor

- *(client)* Replace numeric duration values with `datetime.timedelta` ([#454](https://github.com/0x676e67/wreq/issues/454)) - ([a322e09](https://github.com/0x676e67/wreq-python/commit/a322e091d8e809b7e415ef524c89c671e8f5b860))
- *(dns)* Remove unused exception types ([#437](https://github.com/0x676e67/wreq/issues/437)) - ([c967fd4](https://github.com/0x676e67/wreq-python/commit/c967fd4a3e5c1b3a78e6aa30e45abaa55c6a46a2))
- *(rt)* Migrate async runtime to `pyo3-async-runtimes` ([#428](https://github.com/0x676e67/wreq/issues/428)) - ([228069a](https://github.com/0x676e67/wreq-python/commit/228069a55ceb4ecbb19567f57b602424b316ac51))

### Documentation

- *(python)* Use `T | None` instead of `Optional[T]` for type hints ([#460](https://github.com/0x676e67/wreq/issues/460)) - ([c353a37](https://github.com/0x676e67/wreq-python/commit/c353a377c2ba5606e554716231d0195851b57521))
- *(python)* Update document prompt ([#453](https://github.com/0x676e67/wreq/issues/453)) - ([89a7dfa](https://github.com/0x676e67/wreq-python/commit/89a7dfa51b400b5eac1e46cf5257d1e8b4dd171d))

### Performance

- *(client)* Improve client create performance ([#459](https://github.com/0x676e67/wreq/issues/459)) - ([4089ea1](https://github.com/0x676e67/wreq-python/commit/4089ea1a0c11d6db31f7476f829385b31e6e2903))
- *(client)* Improve request send performance ([#455](https://github.com/0x676e67/wreq/issues/455)) - ([4a0b891](https://github.com/0x676e67/wreq-python/commit/4a0b891e136d3916d58a871a43eb42e1650db493))
- *(stream)* Prevent GIL blocking tokio workers ([#444](https://github.com/0x676e67/wreq/issues/444)) - ([26ef30a](https://github.com/0x676e67/wreq-python/commit/26ef30a6a53ec85c89c27021455c4c182719ef0a))

### Miscellaneous Tasks

- *(bench)* Improve benchmark tests ([#433](https://github.com/0x676e67/wreq/issues/433)) - ([6110c1b](https://github.com/0x676e67/wreq-python/commit/6110c1b4a4ddf6b56336fea30373bbffceb2efe2))
- *(bench)* Update benchmark - ([d788e61](https://github.com/0x676e67/wreq-python/commit/d788e61b3a102f609b2635c761b7b6980edb5ebd))
- *(body)* Fmt code - ([03fbba4](https://github.com/0x676e67/wreq-python/commit/03fbba410896ab266e1bdb486a88b1922391be62))
- *(header)* Simplify header extraction ([#456](https://github.com/0x676e67/wreq/issues/456)) - ([976e992](https://github.com/0x676e67/wreq-python/commit/976e99228a2b64643a8d7a4dcdbb96d616614e9c))
- *(parser)* Refactor legacy parameter extraction ([#435](https://github.com/0x676e67/wreq/issues/435)) - ([b6bb87c](https://github.com/0x676e67/wreq-python/commit/b6bb87cf15e0c6f8d9f3bbc680d4a71e983800fa))
- *(request)* Streamline request responsibility modules ([#430](https://github.com/0x676e67/wreq/issues/430)) - ([35685eb](https://github.com/0x676e67/wreq-python/commit/35685eb6230b52c705306c3c661c6f83a7be31c3))
- *(stream)* Fmt code - ([563c90e](https://github.com/0x676e67/wreq-python/commit/563c90eee1ca4478b411d1d97aedca1f440d25f9))
- *(stream)* Fmt code - ([f633937](https://github.com/0x676e67/wreq-python/commit/f633937acbe513b297ca7ef4488df57753cce861))
- Simplify type extraction ([#451](https://github.com/0x676e67/wreq/issues/451)) - ([36d7412](https://github.com/0x676e67/wreq-python/commit/36d741285792da638c1400d1c7b38bc17bf9c12c))
- Update examples - ([a221dc4](https://github.com/0x676e67/wreq-python/commit/a221dc420d9b0726d8e30cccf8dfad726a33e992))
- Fmt code - ([27999b4](https://github.com/0x676e67/wreq-python/commit/27999b43ff7b19d6e5e29ac6343152e999fdc289))
- Fmt code - ([4141583](https://github.com/0x676e67/wreq-python/commit/4141583e0efe621c7f271296dddcba60553cacb7))

### Build

- *(deps)* Bump actions/upload-artifact from 5 to 6 ([#457](https://github.com/0x676e67/wreq/issues/457)) - ([758d509](https://github.com/0x676e67/wreq-python/commit/758d50941b8d976dcb297a0f321867ec9d25e673))
- *(deps)* Bump actions/download-artifact from 6 to 7 ([#458](https://github.com/0x676e67/wreq/issues/458)) - ([fb5c425](https://github.com/0x676e67/wreq-python/commit/fb5c4256d43d094321698f7d44dfc933b82281fe))

## New Contributors ❤️

* @xiaoweigege made their first contribution in [#453](https://github.com/0x676e67/wreq-python/pull/453)

## [3.0.0-rc14](https://github.com/0x676e67/wreq-python/compare/v3.0.0-rc13..v3.0.0-rc14) - 2025-12-02

### Bug Fixes

- *(client)* Handle multi-value default headers without overriding requests ([#425](https://github.com/0x676e67/wreq/issues/425)) - ([542a595](https://github.com/0x676e67/wreq-python/commit/542a595eaac4208be6a6e932038a9c89ee049026))

### Performance

- *(response)* Fix async API blocking the runtime ([#427](https://github.com/0x676e67/wreq/issues/427)) - ([3288644](https://github.com/0x676e67/wreq-python/commit/328864450dbb5611b4566792b411a0686624561b))

### Miscellaneous Tasks

- *(examples)* Fmt code - ([41405e2](https://github.com/0x676e67/wreq-python/commit/41405e25b825f944298e0848afac3b7bdbb75fad))
- *(response)* Change methods to consume self instead of borrowing - ([99f2924](https://github.com/0x676e67/wreq-python/commit/99f2924685f3a90e07672b77da2b97239c5ea374))

### Build

- *(deps)* Bump actions/checkout from 5 to 6 ([#423](https://github.com/0x676e67/wreq/issues/423)) - ([abc17a4](https://github.com/0x676e67/wreq-python/commit/abc17a4e371fb29a9ba4fa91e835d05c1f6855c0))


## [3.0.0-rc13](https://github.com/0x676e67/wreq-python/compare/v3.0.0-rc12..v3.0.0-rc13) - 2025-11-21

### Documentation

- *(python)* Replace `List` with `Sequence` for immutable returns ([#417](https://github.com/0x676e67/wreq/issues/417)) - ([a5f706f](https://github.com/0x676e67/wreq-python/commit/a5f706f3a5f7be2fd0121498cfec54bfb256952f))

### Miscellaneous Tasks

- *(cookie)* Remove redundant signature bindings ([#422](https://github.com/0x676e67/wreq/issues/422)) - ([86b86ec](https://github.com/0x676e67/wreq-python/commit/86b86ec294b9f7a8913ce3d22cf5be200543c589))
- *(future)* Refactor poll method to use provided context directly ([#416](https://github.com/0x676e67/wreq/issues/416)) - ([bf4887c](https://github.com/0x676e67/wreq-python/commit/bf4887c169f3e8c793093f8496e28130f74e96ec))
- *(python)* Fix example - ([1810e0b](https://github.com/0x676e67/wreq-python/commit/1810e0bc74ef40f4c9da682d85769166adf8a8a7))
- Fmt code - ([0672d11](https://github.com/0x676e67/wreq-python/commit/0672d1147b9e6cb4a8c6117b0f4d540e6c5259df))
- Fmt code ([#420](https://github.com/0x676e67/wreq/issues/420)) - ([3b88bdd](https://github.com/0x676e67/wreq-python/commit/3b88bdd4970341f118fa7f2e222551ae81edb134))


## [3.0.0-rc12](https://github.com/0x676e67/wreq-python/compare/v3.0.0-rc10..v3.0.0-rc12) - 2025-11-07

### Features

- *(client)* Adapt overriding domain name resolution address ([#386](https://github.com/0x676e67/wreq/issues/386)) - ([d775f66](https://github.com/0x676e67/wreq-python/commit/d775f6689fa4f98e6121a102c188e36ee41339fd))
- *(request)* Support `form`/`json` type in `body` data ([#400](https://github.com/0x676e67/wreq/issues/400)) - ([783294a](https://github.com/0x676e67/wreq-python/commit/783294add2f6503d62ee68c64678e78f8ae5a10e))
- *(request)* Support dict type in query data ([#399](https://github.com/0x676e67/wreq/issues/399)) - ([3d84aeb](https://github.com/0x676e67/wreq-python/commit/3d84aeb01851dc7cce7b1b39567da97501c51a48))
- *(uri)* Percent-encode spaces when building request URLs ([#411](https://github.com/0x676e67/wreq/issues/411)) - ([1d5698d](https://github.com/0x676e67/wreq-python/commit/1d5698d8e4ba78878e5aebb11927f7bb5cbeb86b))
- Add Chrome 142 emulation support ([#409](https://github.com/0x676e67/wreq/issues/409)) - ([cd16681](https://github.com/0x676e67/wreq-python/commit/cd166810c8be85ae00fb08abcb3de8a055b5a333))

### Bug Fixes

- *(response)* Fix body reuse ([#412](https://github.com/0x676e67/wreq/issues/412)) - ([83afb58](https://github.com/0x676e67/wreq-python/commit/83afb5802749ae94a73d1618515255eed83fc607))

### Documentation

- *(python)* Replace `Union` with `|` operator in type hints ([#413](https://github.com/0x676e67/wreq/issues/413)) - ([6d0ec9f](https://github.com/0x676e67/wreq-python/commit/6d0ec9f8829f608d0d0a204596a4134774a4b2c8))

### Performance

- *(ws)* Optimize recv function with inline attribute - ([a9981c9](https://github.com/0x676e67/wreq-python/commit/a9981c9a9d94a885567c07640ddb0bd329013a9a))

### Miscellaneous Tasks

- *(body)* Fmt code ([#401](https://github.com/0x676e67/wreq/issues/401)) - ([958ae25](https://github.com/0x676e67/wreq-python/commit/958ae25698958e72e87b48268da0f88deb094e9d))
- *(module)* Use flat module structure ([#406](https://github.com/0x676e67/wreq/issues/406)) - ([cf3cbda](https://github.com/0x676e67/wreq-python/commit/cf3cbda0a3f5ce98573e4876c957a97a9086844e))

### Build

- *(deps)* Bump actions/upload-artifact from 4 to 5 ([#403](https://github.com/0x676e67/wreq/issues/403)) - ([8ffa96f](https://github.com/0x676e67/wreq-python/commit/8ffa96f1f8802da33db7d8ef154154842ec2355b))
- *(deps)* Bump actions/download-artifact from 5 to 6 ([#402](https://github.com/0x676e67/wreq/issues/402)) - ([75cf8a6](https://github.com/0x676e67/wreq-python/commit/75cf8a6a65b64d1ae29cfaf5bca2a47bd02dae2f))

## New Contributors ❤️

* @orangehaired made their first contribution in [#409](https://github.com/0x676e67/wreq-python/pull/409)

## [3.0.0-rc10](https://github.com/0x676e67/wreq-python/compare/v3.0.0-rc9..v3.0.0-rc10) - 2025-10-21

### Features

- *(emulation)* Add chrome141 emulation ([#396](https://github.com/0x676e67/wreq/issues/396)) - ([6a4973a](https://github.com/0x676e67/wreq-python/commit/6a4973adb77d2b08afa8bb8aaa43e9b6dfcbafe0))
- *(request)* Support dict type in form data ([#391](https://github.com/0x676e67/wreq/issues/391)) - ([a647d7a](https://github.com/0x676e67/wreq-python/commit/a647d7a947918b2ccd9d7380c9de8e59397efd15))

### Refactor

- *(core)* Reduce dependency on `futures` ([#393](https://github.com/0x676e67/wreq/issues/393)) - ([0596997](https://github.com/0x676e67/wreq-python/commit/05969972930efbd2774d62678a59ecc871381b6d))

### Documentation

- *(python)* Improve type hints ([#383](https://github.com/0x676e67/wreq/issues/383)) - ([c56c31a](https://github.com/0x676e67/wreq-python/commit/c56c31a6fd1faf013913917c305573a7a5b5c2fa))

### Performance

- *(json)* Zero-copy string deserialization ([#392](https://github.com/0x676e67/wreq/issues/392)) - ([dabcf28](https://github.com/0x676e67/wreq-python/commit/dabcf28b93aaefac02cdcf8c8031433195fa3135))
- *(rt)* Optimize async runtime scheduling - ([9a24b14](https://github.com/0x676e67/wreq-python/commit/9a24b149716643c38dd4812c37d3a39d1d0a33f2))
- *(rt)* Optimize async runtime scheduling ([#387](https://github.com/0x676e67/wreq/issues/387)) - ([6332370](https://github.com/0x676e67/wreq-python/commit/63323707453f5160c8b2978ba00aef8614326a5b))

### Miscellaneous Tasks

- *(module)* Use flat module structure - ([9c85b7b](https://github.com/0x676e67/wreq-python/commit/9c85b7b20af8d8a9410f2d6845e00f69ebe892ea))
- Add wheels for release - ([8c6cfb8](https://github.com/0x676e67/wreq-python/commit/8c6cfb82086dc9029a23a0af950bdcb5509b6312))
- Update examples - ([d4a2d82](https://github.com/0x676e67/wreq-python/commit/d4a2d8225811613bdab295e8a027ce0e880e3dec))

### Build

- *(deps)* Bump astral-sh/setup-uv from 6 to 7 ([#381](https://github.com/0x676e67/wreq/issues/381)) - ([c363d08](https://github.com/0x676e67/wreq-python/commit/c363d08d0e7b20bf6b50f6ec8ea554480af67ffe))
- *(deps)* Bump actions/checkout from 4 to 5 ([#380](https://github.com/0x676e67/wreq/issues/380)) - ([daa2b16](https://github.com/0x676e67/wreq-python/commit/daa2b169042fc65435d7cf0dcd94e2bedbfdb547))
- Add abi3-py313 dependency to Cargo.toml ([#378](https://github.com/0x676e67/wreq/issues/378)) - ([d992418](https://github.com/0x676e67/wreq-python/commit/d992418479fa362009075dba39a5a7bd39d41737))


## [3.0.0-rc9](https://github.com/0x676e67/wreq-python/compare/v3.0.0-rc8..v3.0.0-rc9) - 2025-10-09

### Miscellaneous Tasks

- Prebuild Android wheels ([#376](https://github.com/0x676e67/wreq/issues/376)) - ([c82717f](https://github.com/0x676e67/wreq-python/commit/c82717f0d1d65323bc4e423727372ddd7941bbd3))
- Fix windows aarch64 build ([#375](https://github.com/0x676e67/wreq/issues/375)) - ([ebbaf7a](https://github.com/0x676e67/wreq-python/commit/ebbaf7a499b0b8a0dc89e602bba62abe50abcd5d))
- Add build for free-threaded Python wheel ([#372](https://github.com/0x676e67/wreq/issues/372)) - ([e81d0db](https://github.com/0x676e67/wreq-python/commit/e81d0dbdcb1cfd3e9e1bcbe668d6d44a18162289))

### Build

- *(deps)* Bump actions/checkout from 3 to 5 ([#367](https://github.com/0x676e67/wreq/issues/367)) - ([07a529a](https://github.com/0x676e67/wreq-python/commit/07a529aa7f43bd685d31370e26ebe32317b1d85c))
- *(deps)* Bump actions/download-artifact from 4 to 5 ([#368](https://github.com/0x676e67/wreq/issues/368)) - ([4344798](https://github.com/0x676e67/wreq-python/commit/4344798d2866868691dd933c0fc9c30743834527))
- *(deps)* Bump actions/attest-build-provenance from 1 to 3 ([#369](https://github.com/0x676e67/wreq/issues/369)) - ([b973f2d](https://github.com/0x676e67/wreq-python/commit/b973f2d1e3ea191742a3455353506c2a0b34001f))
- *(deps)* Bump astral-sh/setup-uv from 5 to 6 ([#370](https://github.com/0x676e67/wreq/issues/370)) - ([981bc73](https://github.com/0x676e67/wreq-python/commit/981bc7333e055117ba0433e8cbc24e47cf110375))
- *(deps)* Bump actions/setup-python from 5 to 6 ([#371](https://github.com/0x676e67/wreq/issues/371)) - ([ea02db2](https://github.com/0x676e67/wreq-python/commit/ea02db2bab7ccf847e1379c7c21d534361400f4a))


## [3.0.0-rc8](https://github.com/0x676e67/wreq-python/compare/v3.0.0-rc7..v3.0.0-rc8) - 2025-10-03

### Refactor

- Replace PyO3 `downcast` with `cast` ([#362](https://github.com/0x676e67/wreq/issues/362)) - ([5fb2b96](https://github.com/0x676e67/wreq-python/commit/5fb2b9663edb58ac7ee3746e53d4ca8a8704d264))

### Performance

- Cache hot PyString calls using `pyo3::intern!` macro ([#365](https://github.com/0x676e67/wreq/issues/365)) - ([305fab3](https://github.com/0x676e67/wreq-python/commit/305fab3821abb67de171adc450943821f9053f79))


## [3.0.0-rc7](https://github.com/0x676e67/wreq-python/compare/v3.0.0-rc6..v3.0.0-rc7) - 2025-09-26

### Features

- *(client)* Add `verify_hostname` option binding ([#351](https://github.com/0x676e67/wreq/issues/351)) - ([4691fba](https://github.com/0x676e67/wreq-python/commit/4691fba2aef8789dffc405dcae7dfcb138bde792))
- *(client)* Add HTTP/TLS binding API ([#350](https://github.com/0x676e67/wreq/issues/350)) - ([9d7be01](https://github.com/0x676e67/wreq-python/commit/9d7be01df3852f3483e474b5e08d0de0d8753d96))
- *(emulation)* Add safari ios/ipad 26 emulation ([#361](https://github.com/0x676e67/wreq/issues/361)) - ([cdc3ace](https://github.com/0x676e67/wreq-python/commit/cdc3ace2a9d8774583679ee1c648be516320e608))
- *(emulation)* Add Chrome(138–140), Firefox(140,142), Safari26 ([#358](https://github.com/0x676e67/wreq/issues/358)) - ([773ccb4](https://github.com/0x676e67/wreq-python/commit/773ccb4184466303e9fb36d02088f7c90b2e8b68))

### Documentation

- *(http2)* Align Python enum member names with PascalCase convention ([#354](https://github.com/0x676e67/wreq/issues/354)) - ([b3041d1](https://github.com/0x676e67/wreq-python/commit/b3041d146902a44abc9dc6dc39836c87440f9676))
- *(python)* Fix some LSP hints ([#360](https://github.com/0x676e67/wreq/issues/360)) - ([d4fa076](https://github.com/0x676e67/wreq-python/commit/d4fa076885ab2f3eb740f2fc566aca4092681068))
- *(python)* Rename `SettingOrder` to `SettingsOrder` ([#356](https://github.com/0x676e67/wreq/issues/356)) - ([f3d85c1](https://github.com/0x676e67/wreq-python/commit/f3d85c1fcd64af9bc0985a38a29488ad31a544ff))
- *(python)* Improve documentation for HTTP/2 configuration enums ([#353](https://github.com/0x676e67/wreq/issues/353)) - ([975c168](https://github.com/0x676e67/wreq-python/commit/975c1683a973921767b1d13ace1033d94a1af5c1))
- *(python)* Fix `SettingId` dcos - ([048985c](https://github.com/0x676e67/wreq-python/commit/048985c0f44d569c6d34b6c66386dd45dd6bbad4))

### Miscellaneous Tasks

- *(http2)* Re-export `StreamId` as public Class - ([2dc67a5](https://github.com/0x676e67/wreq-python/commit/2dc67a550b6b8a27fa0487655c3eea752bd3c0b7))

## New Contributors ❤️

* @LanceaKing made their first contribution in [#356](https://github.com/0x676e67/wreq-python/pull/356)

## [3.0.0-rc6](https://github.com/0x676e67/wreq-python/compare/v3.0.0-rc5..v3.0.0-rc6) - 2025-09-13

### Features

- *(resp)* Add raise_for_status() for error handling ([#347](https://github.com/0x676e67/wreq/issues/347)) - ([30ea389](https://github.com/0x676e67/wreq-python/commit/30ea3890db5357aade18f111e5f5e2b217985e4a))
- *(resp)* Allow reuse in free-threaded mode ([#343](https://github.com/0x676e67/wreq/issues/343)) - ([cd74a0b](https://github.com/0x676e67/wreq-python/commit/cd74a0b12d775aa1eb34c27d5fafcad140d9596a))

### Bug Fixes

- *(rt)* Prevent GIL from stalling background Rust future tasks ([#346](https://github.com/0x676e67/wreq/issues/346)) - ([2625bb3](https://github.com/0x676e67/wreq-python/commit/2625bb3102daf4de1a065b8965fbb64660be7236))

### Documentation

- *(python)* Update exceptions docs - ([7c72c13](https://github.com/0x676e67/wreq-python/commit/7c72c13e1ee6cd11909d40e660b8152b2f0807c0))


## [3.0.0-rc5](https://github.com/0x676e67/wreq-python/compare/v3.0.0-rc3..v3.0.0-rc5) - 2025-09-05

### Features

- *(multipart)* Allow setting headers and stream upload length ([#329](https://github.com/0x676e67/wreq/issues/329)) - ([4dcf56e](https://github.com/0x676e67/wreq-python/commit/4dcf56e7f3968cfe92fbba140e5b12ac8b7864a7))
- *(ws)* Allow message batching to reduce buffer flushes ([#331](https://github.com/0x676e67/wreq/issues/331)) - ([dafcb74](https://github.com/0x676e67/wreq-python/commit/dafcb7471076cf5610497213538694cbb325f21a))

### Documentation

- *(python)* Fix  response `content_length` API docs ([#339](https://github.com/0x676e67/wreq/issues/339)) - ([870c43e](https://github.com/0x676e67/wreq-python/commit/870c43eab3c7a15c7eb971903b1cb6ae840cbcba))
- *(python)* Fmt code - ([8995675](https://github.com/0x676e67/wreq-python/commit/8995675161caa07a38fd0bd74e0bd757f6b59751))
- *(python)* Fix multiple Python LSP API warnings ([#337](https://github.com/0x676e67/wreq/issues/337)) - ([8b793aa](https://github.com/0x676e67/wreq-python/commit/8b793aa909de7d5af9683db0974061236637908d))
- *(python)* Improve documentation for response parameters - ([14e9773](https://github.com/0x676e67/wreq-python/commit/14e97737ab350fc543c0822ab2be92412229b1d7))
- *(python)* Improve documentation for client and request parameters ([#334](https://github.com/0x676e67/wreq/issues/334)) - ([395bc60](https://github.com/0x676e67/wreq-python/commit/395bc607ec21e50a54181af3a38053354b30e5c4))
- *(python)* Fix cookie initialization API docs ([#333](https://github.com/0x676e67/wreq/issues/333)) - ([0021d26](https://github.com/0x676e67/wreq-python/commit/0021d26070b0c1cf9d70536fce84d570dbabdc42))

### Performance

- *(msg)* Eliminate borrow checker overhead - ([7bfd213](https://github.com/0x676e67/wreq-python/commit/7bfd2139c3f9e51537d01141a16f6a35ef2e639c))
- *(ws)* Eliminate borrow checker overhead ([#335](https://github.com/0x676e67/wreq/issues/335)) - ([6abf748](https://github.com/0x676e67/wreq-python/commit/6abf74876a7247a1768d235dc806283b6c4149d0))

### Styling

- *(benchmark)* Fmt code - ([17e5939](https://github.com/0x676e67/wreq-python/commit/17e59398980ee28b16b0243c8b32309d83789d7d))
- *(ws)* Fmt code - ([834dfc1](https://github.com/0x676e67/wreq-python/commit/834dfc14e8e7a54556743fad4fe55a2543e09fe7))

### Miscellaneous Tasks

- *(python)* Update examples - ([e7e01a2](https://github.com/0x676e67/wreq-python/commit/e7e01a270d470179cc0d4d573256aa5a9f0f4bc6))

### Revert

- *(client)* Remove `http2_max_retry_count` option ([#336](https://github.com/0x676e67/wreq/issues/336)) - ([740e5ad](https://github.com/0x676e67/wreq-python/commit/740e5ad003b4c5b216a37238385dd66aae853416))

### Build

- *(deps)* Bump wreq version to 6.0.0-rc.14 ([#340](https://github.com/0x676e67/wreq/issues/340)) - ([eab9b53](https://github.com/0x676e67/wreq-python/commit/eab9b533f2c667e02bf01ac96f5bada2e69da80b))


## [3.0.0-rc3](https://github.com/0x676e67/wreq-python/compare/v3.0.0-rc2..v3.0.0-rc3) - 2025-09-03

### Features

- *(response)* Allow accessing redirect history in response ([#322](https://github.com/0x676e67/wreq/issues/322)) - ([ebb82ab](https://github.com/0x676e67/wreq-python/commit/ebb82ab96bb50a6a6129543529b7f970af0ab30c))
- *(response)* Reuse buffered response body ([#318](https://github.com/0x676e67/wreq/issues/318)) - ([e6f01b6](https://github.com/0x676e67/wreq-python/commit/e6f01b654a446e82e52e5238e2fb52c9412a4132))
- *(ws)* WebSocket support for `recv()` timeout ([#316](https://github.com/0x676e67/wreq/issues/316)) - ([3901f84](https://github.com/0x676e67/wreq-python/commit/3901f84ca30064e748fa0606033cdb05dcf0db9c))

### Refactor

- *(buffer)* Restructure buffer protocol ([#319](https://github.com/0x676e67/wreq/issues/319)) - ([74c9ebe](https://github.com/0x676e67/wreq-python/commit/74c9ebebf3f2ea2ea7d03f1463bdd6506692b29e))
- *(header)* Streamline `HeaderMap` iterators ([#321](https://github.com/0x676e67/wreq/issues/321)) - ([c930470](https://github.com/0x676e67/wreq-python/commit/c9304701869fa44948978a1125735e42858eed45))
- *(response)* `peer_certificate` as read-only property ([#327](https://github.com/0x676e67/wreq/issues/327)) - ([df6de03](https://github.com/0x676e67/wreq-python/commit/df6de03a14808ca9f499fb7e5f8944874ae245c5))
- *(response)* Remove encoding readonly property ([#324](https://github.com/0x676e67/wreq/issues/324)) - ([0a4a05d](https://github.com/0x676e67/wreq-python/commit/0a4a05d247463616deca2459e8d39e1bf1e91be5))
- *(ws)* Drop iterator impl ([#317](https://github.com/0x676e67/wreq/issues/317)) - ([529a83d](https://github.com/0x676e67/wreq-python/commit/529a83d7a49dd893fd33986b1ff1794b02c75c27))

### Documentation

- *(python)* Fix response history API docs ([#326](https://github.com/0x676e67/wreq/issues/326)) - ([a16c68d](https://github.com/0x676e67/wreq-python/commit/a16c68d110485260a5742eff0e40c914d877ede3))
- *(python)* Remove unused import ([#315](https://github.com/0x676e67/wreq/issues/315)) - ([0fc665e](https://github.com/0x676e67/wreq-python/commit/0fc665e7a8f3f479f5f5037fa6882dcdb9af2533))

### Performance

- *(pyclass)* Eliminate borrow checker overhead ([#323](https://github.com/0x676e67/wreq/issues/323)) - ([c4e378d](https://github.com/0x676e67/wreq-python/commit/c4e378d6f5a4dced6cfeabd366b5c4d907414357))
- *(stream)* Channel-based iteration, drop locks ([#320](https://github.com/0x676e67/wreq/issues/320)) - ([5c59e9d](https://github.com/0x676e67/wreq-python/commit/5c59e9d51628041a8d5e9fd2371a78b5a3f821f3))

### Styling

- *(body)* Fmt code - ([e4bc068](https://github.com/0x676e67/wreq-python/commit/e4bc06832b3714c3779ee25be9cfc7ecf0b2f6fa))
- *(buffer)* Fmt code - ([c42bb2f](https://github.com/0x676e67/wreq-python/commit/c42bb2f3f143da352f4136afc9f84b93582598d9))
- *(client)* Fmt code - ([23ebbde](https://github.com/0x676e67/wreq-python/commit/23ebbde266c7a5f5c4927aaf5f60ae1da5e7c098))
- Fmt code - ([ba73b3c](https://github.com/0x676e67/wreq-python/commit/ba73b3c2955a5ea5c4416d1dc4bd91518ad2f7b6))

### Miscellaneous Tasks

- *(ws)* Fmt code - ([d935b0c](https://github.com/0x676e67/wreq-python/commit/d935b0cdd943f911336eb430a816524a4cb04248))

### Build

- *(deps)* Bump `wreq` version to 6.0.0-rc.11 - ([00626f0](https://github.com/0x676e67/wreq-python/commit/00626f0f7ea392c90b943217f3b40d183a85d9f7))


## [3.0.0-rc2](https://github.com/0x676e67/wreq-python/compare/v3.0.0-rc1..v3.0.0-rc2) - 2025-08-28

### Features

- *(response)* Return local address used for the request ([#313](https://github.com/0x676e67/wreq/issues/313)) - ([1100d4e](https://github.com/0x676e67/wreq-python/commit/1100d4e0c8e34536ac13a971806b805e9b4d4d5f))

### Refactor

- *(request)* Remove string proxy support ([#310](https://github.com/0x676e67/wreq/issues/310)) - ([6dd1d4d](https://github.com/0x676e67/wreq-python/commit/6dd1d4d58ff2bbb0081337f1ec4b98faa42e7c9a))

### Documentation

- *(python)* Simplify documentation parameter types ([#311](https://github.com/0x676e67/wreq/issues/311)) - ([38c0b19](https://github.com/0x676e67/wreq-python/commit/38c0b1981da3d923edb587b260af43793eb7a02c))

### Miscellaneous Tasks

- *(client)* Remove unimplemented no_keepalive option ([#312](https://github.com/0x676e67/wreq/issues/312)) - ([5bff272](https://github.com/0x676e67/wreq-python/commit/5bff272ffd6e577db5f1ff451a2bb9a28b9fea7e))

### Revert

- `perf(async): release Python GIL during async operations` - ([a231ed2](https://github.com/0x676e67/wreq-python/commit/a231ed2ac81bbbe9958e0ff22088f6954d50efee))


## [3.0.0-rc1](https://github.com/0x676e67/wreq-python/compare/v2.4.2..v3.0.0-rc1) - 2025-08-23

### Features

- *(client)* Support TCP socket SO_REUSEADDR option ([#278](https://github.com/0x676e67/wreq/issues/278)) - ([b2a93cb](https://github.com/0x676e67/wreq-python/commit/b2a93cb21a8cfa5d0194560a5b332c154a66c6c6))
- *(cookie)* Retrieve all cookies from the cookie jar ([#276](https://github.com/0x676e67/wreq/issues/276)) - ([1d96f8b](https://github.com/0x676e67/wreq-python/commit/1d96f8b9abe86883674a6933b4501cbbde2c68fb))
- *(exception)* Add TLS and WebSocket exception types and handling ([#280](https://github.com/0x676e67/wreq/issues/280)) - ([c285150](https://github.com/0x676e67/wreq-python/commit/c285150e90c835ea9ff18676fb17263308a9b1b5))
- *(header)* Preserve HTTP/1 case and header order ([#303](https://github.com/0x676e67/wreq/issues/303)) - ([eb95e66](https://github.com/0x676e67/wreq-python/commit/eb95e6689777258b6a53e3b8f8adddc22b7ce1d2))
- *(request)* Optional per-request decompression setting ([#277](https://github.com/0x676e67/wreq/issues/277)) - ([02719c5](https://github.com/0x676e67/wreq-python/commit/02719c5ba30d07172b8904d873c7a6f9ee7b511d))
- *(tls)* Add TLS keylog support via `KeyLogPolicy` ([#296](https://github.com/0x676e67/wreq/issues/296)) - ([994b737](https://github.com/0x676e67/wreq-python/commit/994b7379ef82b881b96a38bc9728d47a0967f018))
- *(tls)* Expose flexible cert store binding ([#294](https://github.com/0x676e67/wreq/issues/294)) - ([d720929](https://github.com/0x676e67/wreq-python/commit/d720929d6c3aefc063c96c27efc778d8b142da9e))
- *(tls)* Support private key and X509 cert as client certificate ([#293](https://github.com/0x676e67/wreq/issues/293)) - ([8e433e5](https://github.com/0x676e67/wreq-python/commit/8e433e53b0923f9e2243fd903cf6dee2018c9757))

### Refactor

- *(blocking)* Move blocking API to `python.blocking` and unify type names ([#288](https://github.com/0x676e67/wreq/issues/288)) - ([75e3825](https://github.com/0x676e67/wreq-python/commit/75e3825b36fcaf049cb8f37e569e89c65994f43d))
- *(response)* Simplify status code API ([#279](https://github.com/0x676e67/wreq/issues/279)) - ([32c034b](https://github.com/0x676e67/wreq-python/commit/32c034bc59cd76d2777178e3f2aa79f3e474bf99))
- *(typing)* Simplify Python class `__str__` magic method impls ([#284](https://github.com/0x676e67/wreq/issues/284)) - ([0367423](https://github.com/0x676e67/wreq-python/commit/0367423cffae88a174d9fac67011a7744e1ce5dd))

### Documentation

- *(python)* Fix cookie store API docs - ([010f080](https://github.com/0x676e67/wreq-python/commit/010f080ed214b9118ee18c79b8ca69a09750f079))
- *(python)* Update tls module documention - ([69dc23e](https://github.com/0x676e67/wreq-python/commit/69dc23e9643882587a3a81c1325725667d324bc9))
- *(python)* Rename .pyi stubs to .py for wider type checker support ([#295](https://github.com/0x676e67/wreq/issues/295)) - ([a93ccbd](https://github.com/0x676e67/wreq-python/commit/a93ccbd913f6b8c5becdf9dea6c9a40d6bd91b10))
- *(python)* Update tcp socket documentation ([#289](https://github.com/0x676e67/wreq/issues/289)) - ([ed48b67](https://github.com/0x676e67/wreq-python/commit/ed48b672013f483fca121bbdfa5f97bc18935ef1))
- *(python)* Update cookie documentation - ([8cc448e](https://github.com/0x676e67/wreq-python/commit/8cc448e956cc15063b28b58742a0f6206bcab512))
- *(python)* Update pyi documentation ([#281](https://github.com/0x676e67/wreq/issues/281)) - ([dc697b1](https://github.com/0x676e67/wreq-python/commit/dc697b13fdb0ccb54486544d992056f560d209dd))

### Performance

- *(async)* Release Python GIL during async operations ([#291](https://github.com/0x676e67/wreq/issues/291)) - ([73e6fbd](https://github.com/0x676e67/wreq-python/commit/73e6fbd8ba5cbb9fa7da9e4c72d21f20fcb767a3))
- *(client)* Inline hot-path request methods for reduced call overhead ([#290](https://github.com/0x676e67/wreq/issues/290)) - ([84bc510](https://github.com/0x676e67/wreq-python/commit/84bc510ce797cb9b8b3f538edd2e7afcddefaafc))
- *(header)* Zero-copy creation of header values ([#302](https://github.com/0x676e67/wreq/issues/302)) - ([bb1e213](https://github.com/0x676e67/wreq-python/commit/bb1e21371e3d9ba8100b8fcdaf3de4f94cca473b))

### Testing

- *(request)* Add test for disabling default headers - ([298c214](https://github.com/0x676e67/wreq-python/commit/298c2141a852f9406a65380c64f191b0359cc2fb))

### Miscellaneous Tasks

- *(tls)* Remove duplicate TLS version bindings ([#298](https://github.com/0x676e67/wreq/issues/298)) - ([a1e8f23](https://github.com/0x676e67/wreq-python/commit/a1e8f235eaf705311b376647bf7967064a0b4f5f))

### Build

- *(deps)* Compile optional memory allocator ([#285](https://github.com/0x676e67/wreq/issues/285)) - ([b27c9f4](https://github.com/0x676e67/wreq-python/commit/b27c9f40e84eebdb952187a2c246f58086168aab))
- *(deps)* Wheel drops support for versions prior to 3.11 ([#271](https://github.com/0x676e67/wreq/issues/271)) - ([f091144](https://github.com/0x676e67/wreq-python/commit/f091144d1cc96e54758e71e8766ef890e65b5e0b))


## [2.4.2](https://github.com/0x676e67/wreq-python/compare/v2.4.1..v2.4.2) - 2025-08-02

### Bug Fixes

- *(redirect)* Fix request-level redirect ([#268](https://github.com/0x676e67/wreq/issues/268)) - ([29554cf](https://github.com/0x676e67/wreq-python/commit/29554cf6d25dad70ca465bb51628971542afc0ea))

### Refactor

- *(module)* Separate typing responsibilities - ([ce2bf13](https://github.com/0x676e67/wreq-python/commit/ce2bf139a4ff9058cf3312c42d52448af3939e97))


## [2.4.1](https://github.com/0x676e67/wreq-python/compare/v2.4.0..v2.4.1) - 2025-07-31

### Features

- *(client)* Set default values for TCP keepalive and user_timeout ([#265](https://github.com/0x676e67/wreq/issues/265)) - ([b7b510c](https://github.com/0x676e67/wreq-python/commit/b7b510c84538b5a101c4d64db187f2ffebf704d0))

### Refactor

- *(module)* Separate stream responsibilities - ([a55c974](https://github.com/0x676e67/wreq-python/commit/a55c97487f1c981cab4c3d77e6c518fdf4ac77d5))
- *(module)* Separate param responsibilities - ([48c94a3](https://github.com/0x676e67/wreq-python/commit/48c94a3fe15cb7cb320d679813771de767e2264e))
- *(module)* Separate client responsibilities - ([cec0b46](https://github.com/0x676e67/wreq-python/commit/cec0b46af44af0c708cdeb8a556db84142bdc8d1))

### Styling

- Rustfmt - ([4a04396](https://github.com/0x676e67/wreq-python/commit/4a04396564559849f46a2ada553fe4e7750bef0b))

### Build

- *(deps)* Bump `tokio` version to 1.47.0 - ([52681d7](https://github.com/0x676e67/wreq-python/commit/52681d7ca8c2584c23947af34a909dff73839921))


## [2.4.0](https://github.com/0x676e67/wreq-python/compare/v2.3.9..v2.4.0) - 2025-07-24

### Features

- *(cookie)* Adopt browser-style uncompressed cookie format ([#260](https://github.com/0x676e67/wreq/issues/260)) - ([152d280](https://github.com/0x676e67/wreq-python/commit/152d280d1c114a25fb39c2a1891e8cf54f09ebd8))
- Add `safari 18_5` impersonate ([#257](https://github.com/0x676e67/wreq/issues/257)) - ([206843d](https://github.com/0x676e67/wreq-python/commit/206843dd34e9c611cd81032c68d4851ef6511281))


## [2.3.9](https://github.com/0x676e67/wreq-python/compare/v2.3.8..v2.3.9) - 2025-07-04

### Features

- *(header)* Allow default fallback in `Response.headers.get()` ([#254](https://github.com/0x676e67/wreq/issues/254)) - ([ae74b9d](https://github.com/0x676e67/wreq-python/commit/ae74b9dfd21662781dc670584ef31037590f0b81))

### Documentation

- *(response)* Fix return type of `Response.json()` method ([#253](https://github.com/0x676e67/wreq/issues/253)) - ([817584b](https://github.com/0x676e67/wreq-python/commit/817584bfdcf851898ce4702aa649b7005f7c5d1f))

## New Contributors ❤️

* @owocado made their first contribution in [#253](https://github.com/0x676e67/wreq-python/pull/253)

## [2.3.8](https://github.com/0x676e67/wreq-python/compare/v2.3.7..v2.3.8) - 2025-07-01

### Documentation

- Split Python stub module ([#244](https://github.com/0x676e67/wreq/issues/244)) - ([7310f37](https://github.com/0x676e67/wreq-python/commit/7310f37e102e35301e0090e2e766d0bcb20bb553))

### Performance

- Release GIL during submodule initialization ([#246](https://github.com/0x676e67/wreq/issues/246)) - ([3b8f594](https://github.com/0x676e67/wreq-python/commit/3b8f594fdc4b329b571cfbbd1b92366d01b0c28f))

### Benchmark

- Fix thread count - ([43dc863](https://github.com/0x676e67/wreq-python/commit/43dc86314ac36aad8620fbeabb8269e10ce10ff2))
- Fix latest version of aiohttp benchmark - ([9f7da96](https://github.com/0x676e67/wreq-python/commit/9f7da968d92ccebc5a8aa67ae4b175f042d39908))


## [2.3.7](https://github.com/0x676e67/wreq-python/compare/v2.3.6..v2.3.7) - 2025-06-28

### Bug Fixes

- *(docs)* Fix type annotation errors ([#243](https://github.com/0x676e67/wreq/issues/243)) - ([98cb986](https://github.com/0x676e67/wreq-python/commit/98cb986be398d6ace356c88b09218095802208f2))


## [2.3.6](https://github.com/0x676e67/wreq-python/compare/v2.3.3..v2.3.6) - 2025-06-27

### Features

- *(header)* Provide comprehensive HeaderMap API bindings ([#239](https://github.com/0x676e67/wreq/issues/239)) - ([118b6a2](https://github.com/0x676e67/wreq-python/commit/118b6a2d50bc285e39e0709d84002f2c665f3d4d))

### Bug Fixes

- *(header)* Fix HeaderMap `append` API behavior ([#240](https://github.com/0x676e67/wreq/issues/240)) - ([c59a21c](https://github.com/0x676e67/wreq-python/commit/c59a21ccbf89e6756e530ecc4fe341801a0620b4))

### Miscellaneous Tasks

- Add exceptions examples ([#238](https://github.com/0x676e67/wreq/issues/238)) - ([c2118e5](https://github.com/0x676e67/wreq-python/commit/c2118e59247d8121d6fce5a1b116c60834e2ccb4))

### Bench

- Add `niquests` benchmark ([#236](https://github.com/0x676e67/wreq/issues/236)) - ([479e5b2](https://github.com/0x676e67/wreq-python/commit/479e5b29c2501bfa22f1a97105d03845d5b27eb2))


## [2.3.3](https://github.com/0x676e67/wreq-python/compare/v2.3.1..v2.3.3) - 2025-06-23

### Features

- *(enums)* Add `Chrome137` and `Firefox139` impersonate ([#230](https://github.com/0x676e67/wreq/issues/230)) - ([179132a](https://github.com/0x676e67/wreq-python/commit/179132a0ad1cd58486f0e5711de36bbfa97060f3))

### Documentation

- *(proxy)* Document proxy parameter accepting HeaderMap ([#232](https://github.com/0x676e67/wreq/issues/232)) - ([a90790f](https://github.com/0x676e67/wreq-python/commit/a90790ff4e5d89844cff067d2281fc92c104e5f3))
- Fix proxy class typing - ([2cfb92f](https://github.com/0x676e67/wreq-python/commit/2cfb92f6f5fd80961dace9cc692fad262e9cb1c8))
- Fix proxy class typing ([#234](https://github.com/0x676e67/wreq/issues/234)) - ([9c9a00b](https://github.com/0x676e67/wreq-python/commit/9c9a00bef5eb74868b0c560171b765bf81595cdf))

### Bench

- Introduce initial benchmark to evaluate HTTP client throughput and latency ([#229](https://github.com/0x676e67/wreq/issues/229)) - ([2f596c6](https://github.com/0x676e67/wreq-python/commit/2f596c673340ade41243bafb1e3792a29aca4ee3))


## [2.3.1](https://github.com/0x676e67/wreq-python/compare/v2.3.0..v2.3.1) - 2025-06-19

### Features

- Add Opera 116/117/118/119 impersonate ([#228](https://github.com/0x676e67/wreq/issues/228)) - ([3373bef](https://github.com/0x676e67/wreq-python/commit/3373bef3117b7cc7d53f6232cd532354fc8bc153))


## [2.3.0](https://github.com/0x676e67/wreq-python/compare/v2.2.12..v2.3.0) - 2025-06-18

### Documentation

- Update docs - ([352293b](https://github.com/0x676e67/wreq-python/commit/352293b5cb833ab90acb273e87d76d26e21c4e20))
- Try to resolve Pylance trigger issues ([#216](https://github.com/0x676e67/wreq/issues/216)) - ([5fab189](https://github.com/0x676e67/wreq-python/commit/5fab189824fdbfd258f59c22e9fe757bd96b57e8))
- Remove unused import - ([659ee01](https://github.com/0x676e67/wreq-python/commit/659ee012af021300ba7b71ddf7601e4ebc497e4f))

### Revert

- Remove Unpack[TypedDict] from class init method ([#217](https://github.com/0x676e67/wreq/issues/217)) - ([92f92a4](https://github.com/0x676e67/wreq-python/commit/92f92a4d554085959a96b4c4b53bb4c225bf2e32))

### Build

- *(jemallocator)* Exclude dependency on Linux GNU targets ([#224](https://github.com/0x676e67/wreq/issues/224)) - ([e2cc03a](https://github.com/0x676e67/wreq-python/commit/e2cc03aa64972bc927f6ec91f7d8c10d36b9294b))


## [2.2.12](https://github.com/0x676e67/wreq-python/compare/v2.2.11..v2.2.12) - 2025-06-10

### Features

- *(impersonate)* Implement random built-in `ImpersonateOption` ([#214](https://github.com/0x676e67/wreq/issues/214)) - ([083ffcc](https://github.com/0x676e67/wreq-python/commit/083ffcc5d37cf72ea4004b974be9758a1d730136))

### Documentation

- Simplify parameter descriptions ([#213](https://github.com/0x676e67/wreq/issues/213)) - ([bb7bfd5](https://github.com/0x676e67/wreq-python/commit/bb7bfd5368ddead7f0ebf552334bb8106f8e68c1))


## [2.2.11](https://github.com/0x676e67/wreq-python/compare/v2.2.10..v2.2.11) - 2025-05-30

### Features

- Python bindings now support subclass inheritance for classes ([#209](https://github.com/0x676e67/wreq/issues/209)) - ([d6260c8](https://github.com/0x676e67/wreq-python/commit/d6260c8ec10fab35a3d4af4633e37f18a3fe40da))

### Miscellaneous Tasks

- Release Python 3.14 wheels ([#211](https://github.com/0x676e67/wreq/issues/211)) - ([28641f9](https://github.com/0x676e67/wreq-python/commit/28641f9758026766a772d47723637fb298e76cc3))


## [2.2.10](https://github.com/0x676e67/wreq-python/compare/v2.2.9..v2.2.10) - 2025-05-24

### Refactor

- *(async_impl)* Use async style for `Response.close()` to match async conventions ([#206](https://github.com/0x676e67/wreq/issues/206)) - ([a2bb67e](https://github.com/0x676e67/wreq-python/commit/a2bb67ed4f4377fa0057957ff0441ef80b290676))

### Documentation

- *(ws)* Update WebSocket message property type ([#207](https://github.com/0x676e67/wreq/issues/207)) - ([f88f76b](https://github.com/0x676e67/wreq-python/commit/f88f76bf624a491e5e4f5e92c4d03b22ac99b775))

### Styling

- Fmt lifecycle identifiers ([#205](https://github.com/0x676e67/wreq/issues/205)) - ([a5ad42b](https://github.com/0x676e67/wreq-python/commit/a5ad42b55fc063bed58030d53692a4b8411d129f))

### Miscellaneous Tasks

- Fmt macros - ([0621883](https://github.com/0x676e67/wreq-python/commit/0621883450ac69477596ad96c9c8a2c12dc70eae))

### Build

- *(deps)* Update pyo3 requirement from 0.24.2 to 0.25.0 ([#200](https://github.com/0x676e67/wreq/issues/200)) - ([11ece4c](https://github.com/0x676e67/wreq-python/commit/11ece4c9a98877fe959e41dd48bff004c92a6e3c))


## [2.2.9](https://github.com/0x676e67/wreq-python/compare/v2.2.8..v2.2.9) - 2025-05-11

### Features

- *(header)* `HeaderMap` distinguishes between append and overwrite APIs ([#197](https://github.com/0x676e67/wreq/issues/197)) - ([928ced0](https://github.com/0x676e67/wreq-python/commit/928ced0a920dfb5bf127ce8eb970eb88369e1d7c))

### Bug Fixes

- *(enums)* Fix missing enum bindings ([#199](https://github.com/0x676e67/wreq/issues/199)) - ([8afc87d](https://github.com/0x676e67/wreq-python/commit/8afc87d969e493b47bd7cb101eb660f1e94c1ba2))


## [2.2.8](https://github.com/0x676e67/wreq-python/compare/v2.2.6..v2.2.8) - 2025-04-29

### Features

- *(client)* Http1 headers uppercase conversion by default - ([f0462d4](https://github.com/0x676e67/wreq-python/commit/f0462d48cf1fc4fc262ef3b52c76669c48e34cbb))
- *(client)* Http1 headers uppercase conversion by default ([#192](https://github.com/0x676e67/wreq/issues/192)) - ([338b003](https://github.com/0x676e67/wreq-python/commit/338b0036fdc3ac00d1129a51746667b522c3d1ad))
- *(error)* Fix and export exceptions ([#190](https://github.com/0x676e67/wreq/issues/190)) - ([152313f](https://github.com/0x676e67/wreq-python/commit/152313f0b1c676a68dd818e241be73d1b944eb18))

### Documentation

- Update `SocketAddr` `__repr__` API docs ([#188](https://github.com/0x676e67/wreq/issues/188)) - ([00124b3](https://github.com/0x676e67/wreq-python/commit/00124b3947580b509a0ace6a4f5205ed2c522fcf))

### Miscellaneous Tasks

- Ignore `*.pyi` check - ([6aff69f](https://github.com/0x676e67/wreq-python/commit/6aff69f839c270fcdd2b2e9ef80806dd27e60032))
- Update docs - ([96c8f54](https://github.com/0x676e67/wreq-python/commit/96c8f54579d3b61eaf00207b1ea7d6e1ae78bd68))
- Clean up exported macro templates - ([8a3a806](https://github.com/0x676e67/wreq-python/commit/8a3a8061ea76772914bf7d3d6b9e68e11a7fd611))


## [2.2.6](https://github.com/0x676e67/wreq-python/compare/v2.2.5..v2.2.6) - 2025-04-27

### Documentation

- Fix Jetbrains family bucket tooltips ([#187](https://github.com/0x676e67/wreq/issues/187)) - ([e57a2df](https://github.com/0x676e67/wreq-python/commit/e57a2df0651a19be26e7435e48a4dcfe8e7b03cb))
- Manually maintain Python API documentation ([#186](https://github.com/0x676e67/wreq/issues/186)) - ([10c1659](https://github.com/0x676e67/wreq-python/commit/10c1659b7558383c81b40114a95e30804c651455))

### Miscellaneous Tasks

- Update examples - ([ad6d171](https://github.com/0x676e67/wreq-python/commit/ad6d1712ff39b8fdeda06d6a1d07093b4a57eeaa))


## [2.2.5](https://github.com/0x676e67/wreq-python/compare/v2.2.1..v2.2.5) - 2025-04-27

### Features

- *(client)* Partially update and append client headers ([#184](https://github.com/0x676e67/wreq/issues/184)) - ([01b051d](https://github.com/0x676e67/wreq-python/commit/01b051d1fc1e8f21f37819fa8f10f670465e8d54))
- *(client)* Update client headers partially rather than overwriting them ([#182](https://github.com/0x676e67/wreq/issues/182)) - ([1e54936](https://github.com/0x676e67/wreq-python/commit/1e54936d84ad38b642530c6447875ed1c2b334fc))
- *(typing)* Add chrome/110 impersonate ([#185](https://github.com/0x676e67/wreq/issues/185)) - ([20748de](https://github.com/0x676e67/wreq-python/commit/20748de8996701d0045b500b970f30742ea8a2d2))

### Documentation

- *(ipaddr)* Update ipaddr docs - ([11e37fd](https://github.com/0x676e67/wreq-python/commit/11e37fdbcbec9c8e1476c01bdc52dc7a81bfb397))
- Clean up redundant doc comments - ([9846a96](https://github.com/0x676e67/wreq-python/commit/9846a9683650d83861c8915baee221080dac3a3c))

### Miscellaneous Tasks

- *(examples)* Update examples - ([67b0ac6](https://github.com/0x676e67/wreq-python/commit/67b0ac64ca216ca794c638a561628993a204bf7a))
- Use ubuntu-latest - ([f95cbc4](https://github.com/0x676e67/wreq-python/commit/f95cbc40f7c362fbffcad836cd24fd8bd061c3d8))

### Build

- *(deps)* Update pyo3-stub-gen requirement from 0.7.0 to 0.8.0 ([#183](https://github.com/0x676e67/wreq/issues/183)) - ([31f7319](https://github.com/0x676e67/wreq-python/commit/31f7319825b8c73ea24ab7312905ca959ea1c90f))


## [2.2.1](https://github.com/0x676e67/wreq-python/compare/v2.2.0..v2.2.1) - 2025-04-03

### Features

- *(header)* Adapt multiple value sequences of key mapping ([#179](https://github.com/0x676e67/wreq/issues/179)) - ([2f79c3d](https://github.com/0x676e67/wreq-python/commit/2f79c3d5cca5d96e49ae728ca728adf062426703))
- *(proxy)* Clone and reuse proxy objects ([#178](https://github.com/0x676e67/wreq/issues/178)) - ([864b56c](https://github.com/0x676e67/wreq-python/commit/864b56c7ddc4b0286ca9e885e04d49af8e566214))


## [2.2.0](https://github.com/0x676e67/wreq-python/compare/v2.1.5..v2.2.0) - 2025-03-30

### Features

- *(proxy)* Enhanced websocket request level proxy definition ([#175](https://github.com/0x676e67/wreq/issues/175)) - ([6a7856e](https://github.com/0x676e67/wreq-python/commit/6a7856e4bb0e574d18be0f004935284b470e68c1))

### Refactor

- *(typing)* Refactor type extractors ([#173](https://github.com/0x676e67/wreq/issues/173)) - ([a0220c5](https://github.com/0x676e67/wreq-python/commit/a0220c5f7b4a900a06cbab6671ef4e9a91d9441a))

### Miscellaneous Tasks

- *(docs)* Delete unused docs - ([5454653](https://github.com/0x676e67/wreq-python/commit/54546539dcb5b8d7e8326e2133df127dbe4d775f))


## [2.1.5](https://github.com/0x676e67/wreq-python/compare/v2.0.5..v2.1.5) - 2025-03-27

### Features

- *(headers)* Adapt `HeaderMap` to support repeated headers ([#166](https://github.com/0x676e67/wreq/issues/166)) - ([2138154](https://github.com/0x676e67/wreq-python/commit/21381548061ef7a6012b75aedd21ca34da9b04a3))
- *(stream)* Streaming transfers supports `str` chunks ([#155](https://github.com/0x676e67/wreq/issues/155)) - ([a913663](https://github.com/0x676e67/wreq-python/commit/a91366305635ae66c1ce37733e56e710f7af3070))
- Allow inheritance of `Client` and `BlockingClient` in python ([#152](https://github.com/0x676e67/wreq/issues/152)) - ([9582afc](https://github.com/0x676e67/wreq-python/commit/9582afc6e994409c1b79935d5e8e44e5eb8849ea))

### Bug Fixes

- *(client)* Updating client can overwrite verify option ([#163](https://github.com/0x676e67/wreq/issues/163)) - ([1aa4164](https://github.com/0x676e67/wreq-python/commit/1aa4164dd62e15bb7b438fd49cfb25217991938b))
- *(proxy)* Fix typo ([#169](https://github.com/0x676e67/wreq/issues/169)) - ([c87a163](https://github.com/0x676e67/wreq-python/commit/c87a1637902f3368c37b15eaa0244d1994baedfa))
- *(typing)* Fix `SameSite` enum binding mapping error ([#172](https://github.com/0x676e67/wreq/issues/172)) - ([fa91b34](https://github.com/0x676e67/wreq-python/commit/fa91b346733ba9b250f6611c8451c9e790f1e816))

### Documentation

- *(json)* Improve `Json` type hints ([#159](https://github.com/0x676e67/wreq/issues/159)) - ([70836ae](https://github.com/0x676e67/wreq-python/commit/70836aefdecdc36b6df8c7f12c7622e4839f111d))
- *(status)* Improve StatusCode `__repr__` implement ([#160](https://github.com/0x676e67/wreq/issues/160)) - ([aef0167](https://github.com/0x676e67/wreq-python/commit/aef0167a89da09d8726ad8bc4343ddd202d4df8c))
- Format rnet.pyi - ([65f5c12](https://github.com/0x676e67/wreq-python/commit/65f5c126f7f8da9b65f720e66340f0be2dbef865))
- Generate docs conditionally to minimize binary size ([#161](https://github.com/0x676e67/wreq/issues/161)) - ([6efccc9](https://github.com/0x676e67/wreq-python/commit/6efccc94c46686d5a2c33d2c88359d90dfd7e1c6))

### Performance

- *(proxy)* Release the GIL when creating proxy ([#170](https://github.com/0x676e67/wreq/issues/170)) - ([d7bcf0d](https://github.com/0x676e67/wreq-python/commit/d7bcf0dc450065eb555308d9b0f8d739a4281ec9))
- *(ws)* Zero-copy creation of websocket messages ([#164](https://github.com/0x676e67/wreq/issues/164)) - ([56a10f3](https://github.com/0x676e67/wreq-python/commit/56a10f38646d1fe86b66b92acd8c7ae0095ca885))
- Optimizing type conversions - ([b41b1e9](https://github.com/0x676e67/wreq-python/commit/b41b1e906d251a52bd2dceb2c4fc5eb7409be330))

### Styling

- *(typing)* Fmt code - ([6866685](https://github.com/0x676e67/wreq-python/commit/6866685ea01e684c55087439b36762f7ad4483ae))
- *(ws)* Fmt code - ([9520a7f](https://github.com/0x676e67/wreq-python/commit/9520a7f6a9da89bdabf64b850324377aaf5d8c56))
- Fmt code - ([9657b38](https://github.com/0x676e67/wreq-python/commit/9657b38fcbe29c51d4f87b6abb1e85cf10aedf9a))

### Miscellaneous Tasks

- *(body)* Remove redundant type conversions - ([9f1c5e1](https://github.com/0x676e67/wreq-python/commit/9f1c5e12c0be299fba58d75ac3994eb749077bae))
- *(error)* Refactor exception error handling ([#154](https://github.com/0x676e67/wreq/issues/154)) - ([f41b02d](https://github.com/0x676e67/wreq-python/commit/f41b02d20f69dd0e0e563816423febd784087d87))

### Build

- *(deps)* Remove unused `log` dependencies ([#153](https://github.com/0x676e67/wreq/issues/153)) - ([eff3da0](https://github.com/0x676e67/wreq-python/commit/eff3da02c2e60d8abde77be0510a3e0ee7c75b9b))
- Update python dependency-groups ([#165](https://github.com/0x676e67/wreq/issues/165)) - ([1a59ab0](https://github.com/0x676e67/wreq-python/commit/1a59ab0ed7a9ba050aaf2e7d588ed99aee1eca03))

## New Contributors ❤️

* @fnk93 made their first contribution in [#152](https://github.com/0x676e67/wreq-python/pull/152)

## [2.0.5](https://github.com/0x676e67/wreq-python/compare/v2.0.0..v2.0.5) - 2025-03-19

### Features

- *(proxy)* Enhanced request level proxy definition ([#145](https://github.com/0x676e67/wreq/issues/145)) - ([25c452f](https://github.com/0x676e67/wreq-python/commit/25c452fc866fa038361e5d4138db8b6dc150413c))
- *(typing)* Add safari/18_3_1 impersonate ([#146](https://github.com/0x676e67/wreq/issues/146)) - ([b1e8f63](https://github.com/0x676e67/wreq-python/commit/b1e8f63c6601c6ad4f9077190ad09ede3bd849c1))

### Bug Fixes

- *(tests)* Update `self-signed.badssl.com` certificate test ([#143](https://github.com/0x676e67/wreq/issues/143)) - ([27b3e38](https://github.com/0x676e67/wreq-python/commit/27b3e3881fc875c94abdb0d9b53db78166a8b565))

### Performance

- *(typing)* Remove obsolete wrapper of request body ([#144](https://github.com/0x676e67/wreq/issues/144)) - ([1179c68](https://github.com/0x676e67/wreq-python/commit/1179c689acf0b6ad778794214f8ebe7afb7ac40a))
- Zero-copy transfers ([#149](https://github.com/0x676e67/wreq/issues/149)) - ([52df0a3](https://github.com/0x676e67/wreq-python/commit/52df0a36bc45b1c9e09cbde68c667b8b1117e75a))

### Styling

- Clippy fix - ([1c84a04](https://github.com/0x676e67/wreq-python/commit/1c84a0499ca39e039c11e85f6c121bbf0d2a9b92))

### Miscellaneous Tasks

- Recombination module - ([80e800a](https://github.com/0x676e67/wreq-python/commit/80e800a779e9c170ea10eb97983e342e5909bf19))
- Update style - ([0bca987](https://github.com/0x676e67/wreq-python/commit/0bca987004820e871fed48e84ea28c0a0c34a3ca))


## [2.0.0](https://github.com/0x676e67/wreq-python/compare/v1.7.0..v2.0.0) - 2025-03-18

### Features

- Add `chrome 134` impersonate ([#140](https://github.com/0x676e67/wreq/issues/140)) - ([476d741](https://github.com/0x676e67/wreq-python/commit/476d741fb75978a863e8f2d76e7a79018c9f762e))
- Adopt `requests` style API for client SSL certificate verification ([#132](https://github.com/0x676e67/wreq/issues/132)) - ([bab3d72](https://github.com/0x676e67/wreq-python/commit/bab3d7232490c12ec436c78af56346a1ddbbd15e))

### Bug Fixes

- *(header)* Adapt to the scenario of duplicate keys in HeaderMap ([#135](https://github.com/0x676e67/wreq/issues/135)) - ([bf3b76c](https://github.com/0x676e67/wreq-python/commit/bf3b76c715ec7d0324afd0705b02ce7a88817331))
- *(ws/client)* Adapt to the scenario of duplicate keys in HeaderMap - ([3dd144a](https://github.com/0x676e67/wreq-python/commit/3dd144a77f06575e51857effebf13f8537895725))

### Refactor

- *(cookie)* Refactor cookie API signature ([#134](https://github.com/0x676e67/wreq/issues/134)) - ([8f8808b](https://github.com/0x676e67/wreq-python/commit/8f8808bec628e2b97333d51469c8e6fa50b3bc39))

### Performance

- *(header)* Use buffer protocol in `HeaderMap` to reduce copying ([#136](https://github.com/0x676e67/wreq/issues/136)) - ([da72d4e](https://github.com/0x676e67/wreq-python/commit/da72d4e0bef94fe6daa7d38acb290903ea5fad34))

### Styling

- Fmt code - ([1a496d3](https://github.com/0x676e67/wreq-python/commit/1a496d35c1b2e712c92875064c343f5d0e5f8272))

### Miscellaneous Tasks

- Update examples - ([2b811f0](https://github.com/0x676e67/wreq-python/commit/2b811f0490b2d98cd95588ab243065a17a1eb5c8))
- Use macros to simplify Python buffer implementation ([#139](https://github.com/0x676e67/wreq/issues/139)) - ([3e68f2c](https://github.com/0x676e67/wreq-python/commit/3e68f2c879193ce131c696cd76cd024af86eaa5f))
- Rename header type conversion - ([1b394b3](https://github.com/0x676e67/wreq-python/commit/1b394b39d813198ca80e2c969c70aba2a13bf42a))
- Rename some APIs - ([425cab1](https://github.com/0x676e67/wreq-python/commit/425cab1173c58ffcfd36cc8fc16b48ba61b28b28))

### Build

- *(deps)* Remove unused dependencies ([#131](https://github.com/0x676e67/wreq/issues/131)) - ([a55c38f](https://github.com/0x676e67/wreq-python/commit/a55c38ff864fe33c353a7327b8d647342d79d300))
- *(deps)* Update pyo3 requirement from 0.23.5 to 0.24.0 ([#123](https://github.com/0x676e67/wreq/issues/123)) - ([bc04abd](https://github.com/0x676e67/wreq-python/commit/bc04abd42507cffec56d9eb5ea95b6dffef4b3c4))
- *(deps)* Update pyo3-async-runtimes requirement ([#130](https://github.com/0x676e67/wreq/issues/130)) - ([b98f5b5](https://github.com/0x676e67/wreq-python/commit/b98f5b5c2b18df068149ace689e325928d705d46))
- Add free-threaded package support ([#142](https://github.com/0x676e67/wreq/issues/142)) - ([e13e292](https://github.com/0x676e67/wreq-python/commit/e13e292273311267a9033d5c8a4079888d7ba08b))


## [1.7.0](https://github.com/0x676e67/wreq-python/compare/v1.6.5..v1.7.0) - 2025-03-12

### Features

- *(ws)* Added read_buffer_size optional config ([#126](https://github.com/0x676e67/wreq/issues/126)) - ([2d5d64a](https://github.com/0x676e67/wreq-python/commit/2d5d64a48bfe257a5555b7d3b29887d9237f02cd))
- *(ws)* HTTP/2 support for WebSocket ([#125](https://github.com/0x676e67/wreq/issues/125)) - ([52c020c](https://github.com/0x676e67/wreq-python/commit/52c020cf7673e3dbba38917369fa1e0fb04a1f5b))

### Performance

- *(ws)* Prevent deep copy of message data ([#127](https://github.com/0x676e67/wreq/issues/127)) - ([245a65a](https://github.com/0x676e67/wreq-python/commit/245a65a290c3f0db2adeeb1b4423ecd7c4a49ce2))

### Styling

- *(ws)* Fmt code - ([1edc8fe](https://github.com/0x676e67/wreq-python/commit/1edc8fe94044c66727dfc710d2d6ff4a204fb1bd))

### Miscellaneous Tasks

- Rename some APIs - ([51285f6](https://github.com/0x676e67/wreq-python/commit/51285f67d7a1a9eda3b7f044d31f0ea7bfb7ff08))

### Build

- *(deps)* Update indexmap requirement from 2.7.0 to 2.8.0 ([#128](https://github.com/0x676e67/wreq/issues/128)) - ([ace2e5c](https://github.com/0x676e67/wreq-python/commit/ace2e5c0edc7d36b624e2dd87928d94e95e52649))
- *(deps)* Update rquest requirement from 2.2.1 to 3.0.5 ([#124](https://github.com/0x676e67/wreq/issues/124)) - ([35f3f7b](https://github.com/0x676e67/wreq-python/commit/35f3f7b34b8e1d7b36908361ae9c0b7d53f5d5bf))
- Upgrade dependencies ([#122](https://github.com/0x676e67/wreq/issues/122)) - ([5807382](https://github.com/0x676e67/wreq-python/commit/58073820d9f87df5a68df07aef39879e43b89a3a))

## New Contributors ❤️

* @dependabot[bot] made their first contribution in [#124](https://github.com/0x676e67/wreq-python/pull/124)

## [1.6.5](https://github.com/0x676e67/wreq-python/compare/v1.6.1..v1.6.5) - 2025-03-06

### Bug Fixes

- *(stream)* Fix asynchronous stream sending ([#119](https://github.com/0x676e67/wreq/issues/119)) - ([db2ae24](https://github.com/0x676e67/wreq-python/commit/db2ae24e97efd8472e31df6d57bfe3bd56c9fb25))

### Performance

- *(body)* Optimize data copying in request body sending ([#120](https://github.com/0x676e67/wreq/issues/120)) - ([5710295](https://github.com/0x676e67/wreq-python/commit/5710295300c6d4eb8db808ed14e677faf05b88a8))
- *(cookie)* Attempt to convert a `Bytes` buffer to a `HeaderValue` ([#117](https://github.com/0x676e67/wreq/issues/117)) - ([d652097](https://github.com/0x676e67/wreq-python/commit/d6520975746bafd9e91a7c9032d6b7a6269aa0c9))


## [1.6.1](https://github.com/0x676e67/wreq-python/compare/v1.6.0..v1.6.1) - 2025-03-03

### Performance

- *(async_impl)* Allow #[inline(always)] cookies ([#111](https://github.com/0x676e67/wreq/issues/111)) - ([c63a47a](https://github.com/0x676e67/wreq-python/commit/c63a47ad1340b98279356b925f054734e62b1acf))
- *(cookie)* Zero-copy cookies extraction ([#113](https://github.com/0x676e67/wreq/issues/113)) - ([07c63a6](https://github.com/0x676e67/wreq-python/commit/07c63a6846149f195d67a35424eb708545a8d602))
- *(error)* Allow #[inline(always)] errors ([#112](https://github.com/0x676e67/wreq/issues/112)) - ([65b09ca](https://github.com/0x676e67/wreq-python/commit/65b09ca6a4cb3e9e98478ef455c6f498914c4ace))
- Zero-copy extraction of request parameters ([#114](https://github.com/0x676e67/wreq/issues/114)) - ([9a9eafc](https://github.com/0x676e67/wreq-python/commit/9a9eafc10a86d8a68f090cf2494c8be641fb7a9e))

### Styling

- *(enums)* Fmt code - ([2c5b0ab](https://github.com/0x676e67/wreq-python/commit/2c5b0ab2fae9984efbc0e9615eadc42e404cb844))
- Fmt code - ([ccb6519](https://github.com/0x676e67/wreq-python/commit/ccb6519c9d8d07d12997d5e90f4967dce7bb2c87))

### Miscellaneous Tasks

- Move macros - ([466651e](https://github.com/0x676e67/wreq-python/commit/466651e780b32262eca33ee69fb7508aa9c1737a))

### Build

- *(deps)* Pin futures-util v0.3.31 - ([3e010f9](https://github.com/0x676e67/wreq-python/commit/3e010f95a1ea0aa900cc0b1d8a4c8bcd31f76aea))
- `MSRV 1.85` / `edition 2024` ([#115](https://github.com/0x676e67/wreq/issues/115)) - ([4015763](https://github.com/0x676e67/wreq-python/commit/4015763ea2ab0885bd5eb997b641e606780a752d))


## [1.6.0](https://github.com/0x676e67/wreq-python/compare/v1.5.6..v1.6.0) - 2025-03-02

### Features

- Remove unsupported `CONNECT` request method ([#108](https://github.com/0x676e67/wreq/issues/108)) - ([18d5ff2](https://github.com/0x676e67/wreq-python/commit/18d5ff2ec1f240ee6882b14525da235b85aa751c))

### Refactor

- Rename APIs to better match their use casesca - ([1551059](https://github.com/0x676e67/wreq-python/commit/155105988f4f02accdacde52072158163e23c70d))
- Rename APIs to better match their use cases ([#110](https://github.com/0x676e67/wreq/issues/110)) - ([7e36d70](https://github.com/0x676e67/wreq-python/commit/7e36d70791b260c5319b8ef9bb5c5c78c03fa7d8))

### Performance

- *(blocking)* Allow `#[inline(always)]` cookies ([#103](https://github.com/0x676e67/wreq/issues/103)) - ([4bc7301](https://github.com/0x676e67/wreq-python/commit/4bc7301bd18efdc7ced96c32f276d642f900a3b0))
- *(client)* Improve performance of client cookie setting ([#105](https://github.com/0x676e67/wreq/issues/105)) - ([02e46e0](https://github.com/0x676e67/wreq-python/commit/02e46e09dc9d7e9011f4c106b7574e5cd9fb903b))
- Reduce cookie and header data copying ([#109](https://github.com/0x676e67/wreq/issues/109)) - ([d29772c](https://github.com/0x676e67/wreq-python/commit/d29772c976206d1758ead03e104002e81139c7bc))
- Use zero-copy param for Python to Rust conversion ([#107](https://github.com/0x676e67/wreq/issues/107)) - ([d904936](https://github.com/0x676e67/wreq-python/commit/d904936245ce1ac46edbacc90d4b3cfec585dd19))
- Use zero-copy strings for Python to Rust conversion ([#106](https://github.com/0x676e67/wreq/issues/106)) - ([ea26e88](https://github.com/0x676e67/wreq-python/commit/ea26e88417532b1eac80fd8a54f4996776643c5c))
- Use byte buffer for WebSocket message body ([#104](https://github.com/0x676e67/wreq/issues/104)) - ([6d1266c](https://github.com/0x676e67/wreq-python/commit/6d1266c0ce4caae2aa4750612d0456af3d08dd40))

### Styling

- Fmt code - ([391d7ff](https://github.com/0x676e67/wreq-python/commit/391d7ff8178b6dc30871dac955b084a6dc3a468c))
- Fmt code - ([f631070](https://github.com/0x676e67/wreq-python/commit/f6310703e17d6683f22f5430bdca8a7436d43350))

### Miscellaneous Tasks

- Update docs - ([0c761a2](https://github.com/0x676e67/wreq-python/commit/0c761a260ee03e0a0b911ed05ad1b6a7c0533c3e))
- Update examples - ([3874287](https://github.com/0x676e67/wreq-python/commit/3874287f6f81a89d96fe135bb43c7194af1a6c46))


## [1.5.6](https://github.com/0x676e67/wreq-python/compare/v1.5.5..v1.5.6) - 2025-03-01

### Features

- *(http/ws)* Handling stream read errors ([#102](https://github.com/0x676e67/wreq/issues/102)) - ([f9c4806](https://github.com/0x676e67/wreq-python/commit/f9c48067e3abf5acb9ad565220b42e04e218badd))
- Avoid returning unnecessary data after connection is closed ([#100](https://github.com/0x676e67/wreq/issues/100)) - ([118a785](https://github.com/0x676e67/wreq-python/commit/118a78588be965f396a55f96cb3c38995ebeafbd))

### Performance

- *(stream)* Fine-grained handling of GIL release for synchronized iterators - ([5d86791](https://github.com/0x676e67/wreq-python/commit/5d86791319fe598cc5fd2b7855d502c5fbb8ffdd))
- *(stream)* Release the GIL while the Stream is waiting for the future ([#101](https://github.com/0x676e67/wreq/issues/101)) - ([ba7a3d7](https://github.com/0x676e67/wreq-python/commit/ba7a3d7cd2ca22616bf16018e89ed352f1e1b092))

### Styling

- Fmt code - ([ace9c9e](https://github.com/0x676e67/wreq-python/commit/ace9c9ebd364152aad05d205aa0a1cd734614220))


## [1.5.5](https://github.com/0x676e67/wreq-python/compare/v1.5.2..v1.5.5) - 2025-02-28

### Features

- *(client)* Allow setting TLS version range ([#95](https://github.com/0x676e67/wreq/issues/95)) - ([09a1ab6](https://github.com/0x676e67/wreq-python/commit/09a1ab652cfe7ad53f7e1844d618243de2d79d46))
- *(ws)* Return error when sending/receiving after websocket disconnect ([#99](https://github.com/0x676e67/wreq/issues/99)) - ([d4cf2f0](https://github.com/0x676e67/wreq-python/commit/d4cf2f018d0a82cb4a24fc730e8eedaf3590af7c))

### Refactor

- Module types renamed to typing - ([7deaba8](https://github.com/0x676e67/wreq-python/commit/7deaba8298e2fd9f9e5b1f322bc9d611ce9cd1ad))

### Performance

- *(client)* Remove duplicate type conversions ([#98](https://github.com/0x676e67/wreq/issues/98)) - ([8845e49](https://github.com/0x676e67/wreq-python/commit/8845e495a7bcd1f2cab5a9ac817612a4079696cd))
- *(ws)* Optimize websocket message processing ([#96](https://github.com/0x676e67/wreq/issues/96)) - ([57d882f](https://github.com/0x676e67/wreq-python/commit/57d882f15bf91840454bdac1412987c25758d2de))
- Improved streaming body delivery ([#97](https://github.com/0x676e67/wreq/issues/97)) - ([aacedb6](https://github.com/0x676e67/wreq-python/commit/aacedb67c7ef38ef70a22374705b056b4556d8c1))

### Miscellaneous Tasks

- Refactor headers order parameter extract - ([92a1fc9](https://github.com/0x676e67/wreq-python/commit/92a1fc9ffc80dbc237cd27339d2ebddb7d4ca92e))
- Simplify some type conversions ([#94](https://github.com/0x676e67/wreq/issues/94)) - ([723540c](https://github.com/0x676e67/wreq-python/commit/723540c9c738965c0d77ab6cc70cfe7687df02c6))


## [1.5.2](https://github.com/0x676e67/wreq-python/compare/v1.5.0..v1.5.2) - 2025-02-28

### Features

- *(types)* Wrapped indexmap type document generation - ([8aae65f](https://github.com/0x676e67/wreq-python/commit/8aae65f76aa92e0388b9c85c80cd9de49d2b2599))
- *(types)* Wrapped indexmap type document generation ([#91](https://github.com/0x676e67/wreq/issues/91)) - ([56f3ef7](https://github.com/0x676e67/wreq-python/commit/56f3ef7fc0e9812b1f0d6894b1096eb9b9927234))
- Added request to set cookie dictionary ([#92](https://github.com/0x676e67/wreq/issues/92)) - ([03058fa](https://github.com/0x676e67/wreq-python/commit/03058faba1c060c9001240730018cab9efcf7d8a))

### Documentation

- Fix client cookie documents ([#90](https://github.com/0x676e67/wreq/issues/90)) - ([77f748c](https://github.com/0x676e67/wreq-python/commit/77f748c5310072d23cd4031a3ef7ca844acbf682))

### Styling

- Fmt code - ([61342bc](https://github.com/0x676e67/wreq-python/commit/61342bcb7b64997787faa2c6775e4511f1f4e8db))

### Testing

- Update request tests ([#93](https://github.com/0x676e67/wreq/issues/93)) - ([2ac09e9](https://github.com/0x676e67/wreq-python/commit/2ac09e9ad36ae1b361caacae296e4b3b9e46b2c5))
- Update tests - ([2cac3bb](https://github.com/0x676e67/wreq-python/commit/2cac3bbca69f28bb059e5d2ef6c04562e61785ff))

### Miscellaneous Tasks

- *(buffer)* Fmt code - ([ce7c08c](https://github.com/0x676e67/wreq-python/commit/ce7c08ca50f812fda55d22c04bf86225446794e4))
- Fmt code - ([49810a3](https://github.com/0x676e67/wreq-python/commit/49810a3daa505d8310728388a6b846c43b9d7747))


## [1.5.0](https://github.com/0x676e67/wreq-python/compare/v1.3.1..v1.5.0) - 2025-02-27

### Features

- Remove unused `json` API ([#86](https://github.com/0x676e67/wreq/issues/86)) - ([cd7348b](https://github.com/0x676e67/wreq-python/commit/cd7348b0afc246f4f032b6aa2f901c34a7c819b5))

### Documentation

- Fix type documentation generation ([#88](https://github.com/0x676e67/wreq/issues/88)) - ([a5a7497](https://github.com/0x676e67/wreq-python/commit/a5a749777d6751b51b48b6165bfdca4d71136e30))

### Miscellaneous Tasks

- Fix typo - ([c7cc57e](https://github.com/0x676e67/wreq-python/commit/c7cc57e61614bda5a5534a5dba99a82eaf9b18ee))
- Integrate enum module ([#87](https://github.com/0x676e67/wreq/issues/87)) - ([e4bcf3e](https://github.com/0x676e67/wreq-python/commit/e4bcf3eb6dd6e1e11062e8a523cd05e4353ff251))


## [1.3.1](https://github.com/0x676e67/wreq-python/compare/v1.3.0..v1.3.1) - 2025-02-27

### Features

- *(async_impl)* Remove `__anext__` redundant `Option` wrapper ([#78](https://github.com/0x676e67/wreq/issues/78)) - ([28e49aa](https://github.com/0x676e67/wreq-python/commit/28e49aa1798b79789b47c70188cbd48d194dcc70))
- *(ws)* Add response code attribute to WebSocket response ([#82](https://github.com/0x676e67/wreq/issues/82)) - ([9002c1d](https://github.com/0x676e67/wreq-python/commit/9002c1d810d23b85b99a072409bf72d29105ebc0))
- Adapt sync bytes stream body and upload ([#84](https://github.com/0x676e67/wreq/issues/84)) - ([fd8ee1a](https://github.com/0x676e67/wreq-python/commit/fd8ee1ab9fa02dec78eb0916a804836cc202d748))
- Adapt bytes stream body and upload ([#80](https://github.com/0x676e67/wreq/issues/80)) - ([59113b4](https://github.com/0x676e67/wreq-python/commit/59113b4363d283813481ba8359564e31a932a5df))

### Performance

- Implementing the Python Buffer Protocol ([#85](https://github.com/0x676e67/wreq/issues/85)) - ([0e14d63](https://github.com/0x676e67/wreq-python/commit/0e14d634535238e9b2cb7aa0d40047617a1427ad))

### Styling

- Fmt code - ([7aa7fb9](https://github.com/0x676e67/wreq-python/commit/7aa7fb97e67eb9d537120dea8f97ce75a7fd22cd))

### Testing

- Added send body request tests ([#83](https://github.com/0x676e67/wreq/issues/83)) - ([e413cb7](https://github.com/0x676e67/wreq-python/commit/e413cb716f883835b4d9756a6bca1fd3fe997b58))

### Build

- *(deps)* Remove unused dependencies ([#79](https://github.com/0x676e67/wreq/issues/79)) - ([789d1c8](https://github.com/0x676e67/wreq-python/commit/789d1c86be64e6c1a5a085365e5f860359ae4f16))


## [1.3.0](https://github.com/0x676e67/wreq-python/compare/v1.2.0..v1.3.0) - 2025-02-26

### Features

- Implement context management for WebSocket Response ([#77](https://github.com/0x676e67/wreq/issues/77)) - ([ac90e2c](https://github.com/0x676e67/wreq-python/commit/ac90e2c03e6253f649371e50f8a37d990731d8fe))
- Implement context management for HTTP Response ([#76](https://github.com/0x676e67/wreq/issues/76)) - ([5eb8ead](https://github.com/0x676e67/wreq-python/commit/5eb8eada4cb3d2ddc466a5491af7416b6e92fe29))

### Bug Fixes

- Change `&'a mut` self to `&self` to avoid runtime errors ([#75](https://github.com/0x676e67/wreq/issues/75)) - ([c379fc6](https://github.com/0x676e67/wreq-python/commit/c379fc67a36c494e585fa5d218671733a868d10a))

### Refactor

- Reorganized module exports ([#73](https://github.com/0x676e67/wreq/issues/73)) - ([58eaf30](https://github.com/0x676e67/wreq-python/commit/58eaf30a3b82a3352d194133bd066834db453af0))

### Performance

- *(blocking)* Stream iteration abandons the GIL lock ([#72](https://github.com/0x676e67/wreq/issues/72)) - ([226ba8f](https://github.com/0x676e67/wreq-python/commit/226ba8f7ccd4df8a2d0c79a402845f458f78e839))


## [1.2.0](https://github.com/0x676e67/wreq-python/compare/v1.0.1..v1.2.0) - 2025-02-25

### Features

- *(client)* Added optional DNS lookup ip strategy ([#65](https://github.com/0x676e67/wreq/issues/65)) - ([703ac81](https://github.com/0x676e67/wreq-python/commit/703ac8182fb9d00de205e166f1a50e42e71e9b74))
- Blocking API and client support ([#70](https://github.com/0x676e67/wreq/issues/70)) - ([3181ef6](https://github.com/0x676e67/wreq-python/commit/3181ef62f7c76184f908e497dce9f69a746ef307))

### Bug Fixes

- *(deps)* Fix no_proxy on Windows ([#67](https://github.com/0x676e67/wreq/issues/67)) - ([9b58d81](https://github.com/0x676e67/wreq-python/commit/9b58d81598589c5b13a24150a5b78f125037efd4))

### Refactor

- Reorganized module exports - ([6d5ffb5](https://github.com/0x676e67/wreq-python/commit/6d5ffb589a0b52c724b4c5b294923b92474dcb28))
- Reorganized module exports ([#68](https://github.com/0x676e67/wreq/issues/68)) - ([d01eba2](https://github.com/0x676e67/wreq-python/commit/d01eba2a58d2343cad4d86a527bda8d9afdf5ad6))

### Performance

- *(blocking)* Send a request to relinquish the GIL ([#71](https://github.com/0x676e67/wreq/issues/71)) - ([830f615](https://github.com/0x676e67/wreq-python/commit/830f615b9cae04d8bfc22cc78a266564ebb14cec))

### Styling

- Fmt code - ([4d9ca3b](https://github.com/0x676e67/wreq-python/commit/4d9ca3bb1af21c967956ca530b8d4eea0ff6502b))
- Fmt code ([#66](https://github.com/0x676e67/wreq/issues/66)) - ([b4b3b8f](https://github.com/0x676e67/wreq-python/commit/b4b3b8feec7ca7025e95ce64124131d915c80b3b))


## [1.0.1](https://github.com/0x676e67/wreq-python/compare/v1.0.0..v1.0.1) - 2025-02-24

### Features

- Prepares the use of Python in a free-threaded context ([#63](https://github.com/0x676e67/wreq/issues/63)) - ([122b1bf](https://github.com/0x676e67/wreq-python/commit/122b1bfa61943361e30c4aaa340ea47b344cb365))

### Build

- *(deps)* Update rquest requirement from 2.1.6 to 2.2.0 ([#62](https://github.com/0x676e67/wreq/issues/62)) - ([baa8048](https://github.com/0x676e67/wreq-python/commit/baa80485933279d5b0c4679d6acf50d149e5d275))
- Minimum required Python version is 3.7 ([#64](https://github.com/0x676e67/wreq/issues/64)) - ([f98e95c](https://github.com/0x676e67/wreq-python/commit/f98e95cefb4854763a122db68a994e990f4293ac))


## [1.0.0](https://github.com/0x676e67/wreq-python/compare/v0.6.1..v1.0.0) - 2025-02-23

### Features

- *(ws)* Ensure websocket connection is released when closed ([#59](https://github.com/0x676e67/wreq/issues/59)) - ([e347e92](https://github.com/0x676e67/wreq-python/commit/e347e92aa5b2477b3d3f24c0896a0808f5bd54fa))
- Added max redirects option parameter ([#55](https://github.com/0x676e67/wreq/issues/55)) - ([3286c56](https://github.com/0x676e67/wreq-python/commit/3286c56725c36b0f789cf2bfc18ba266c192d786))

### Documentation

- *(dns)* Update docs - ([0461e77](https://github.com/0x676e67/wreq-python/commit/0461e7745795e4600a51b35aa4cba4adc97e791a))

### Performance

- *(http)* Release locks early during stream iteration ([#57](https://github.com/0x676e67/wreq/issues/57)) - ([703d07b](https://github.com/0x676e67/wreq-python/commit/703d07b62086dfc308c32405962ae86926bf4282))
- *(http)* Optimize the overhead of boundary checks for streaming response types ([#56](https://github.com/0x676e67/wreq/issues/56)) - ([93485f2](https://github.com/0x676e67/wreq-python/commit/93485f2a8c4f64179419c365c420ba94b053b4e2))

### Testing

- Improve tests - ([aca55a1](https://github.com/0x676e67/wreq-python/commit/aca55a1507b6bbe8fe8de78b5f749d16bf862d49))
- Added alps new endpoint test ([#61](https://github.com/0x676e67/wreq/issues/61)) - ([6dfdaa8](https://github.com/0x676e67/wreq-python/commit/6dfdaa818565f1d0bfbdec7ba1bc325d3c22db54))

### Miscellaneous Tasks

- *(stream)* Update docs - ([5458e51](https://github.com/0x676e67/wreq-python/commit/5458e510c9bdf64170e25970e4f6902833b5d7cb))
- Fmt code - ([e760b0c](https://github.com/0x676e67/wreq-python/commit/e760b0cd1740cf6bcee75c759404c4539683d8a6))
- Ignore `uv.lock` - ([bd5c00d](https://github.com/0x676e67/wreq-python/commit/bd5c00defa258f9242c08bd89d688a93175d3ddd))

### Build

- *(deps)* Update rquest requirement from 2.1.5 to 2.1.6 ([#60](https://github.com/0x676e67/wreq/issues/60)) - ([9ea0e1c](https://github.com/0x676e67/wreq-python/commit/9ea0e1c352c5500028417be77befda42b8ad6725))
- Fix windows using pyo3/generate-import-lib ([#58](https://github.com/0x676e67/wreq/issues/58)) - ([5136281](https://github.com/0x676e67/wreq-python/commit/5136281139f95710b63d696c9fce5f501ba60792))
- Added support for Windows arm64 wheel package ([#54](https://github.com/0x676e67/wreq/issues/54)) - ([a73ca87](https://github.com/0x676e67/wreq-python/commit/a73ca87288902fe326051ed68807f861cf33b691))
- Fix windows aarch64 using `pyo3/generate-import-lib` ([#53](https://github.com/0x676e67/wreq/issues/53)) - ([e76a116](https://github.com/0x676e67/wreq-python/commit/e76a116a58640d66cd8dbed580284ef3f23f90a1))


## [0.6.1](https://github.com/0x676e67/wreq-python/compare/v0.6.0..v0.6.1) - 2025-02-22

### Features

- Added multipart file upload ([#51](https://github.com/0x676e67/wreq/issues/51)) - ([4b16799](https://github.com/0x676e67/wreq-python/commit/4b16799b70fcc0eb82bacb61518cbe08365acb68))
- Added websocket optional config ([#49](https://github.com/0x676e67/wreq/issues/49)) - ([0a83669](https://github.com/0x676e67/wreq-python/commit/0a83669f7e4ed5be1c21735e916c15c3cc3bddbf))

### Bug Fixes

- *(types)* Fix FFI enum type binding documentation generation ([#50](https://github.com/0x676e67/wreq/issues/50)) - ([3d70255](https://github.com/0x676e67/wreq-python/commit/3d70255a77d57bd2f6bf265a4bbc1c13936fa097))


## [0.6.0](https://github.com/0x676e67/wreq-python/compare/v0.5.1..v0.6.0) - 2025-02-20

### Features

- *(client)* An asynchronous DNS resolver is used by default ([#45](https://github.com/0x676e67/wreq/issues/45)) - ([6c6f994](https://github.com/0x676e67/wreq-python/commit/6c6f994939eff0593b45c5014cda93579de72cc5))

### Documentation

- *(client)* Fix typo ([#43](https://github.com/0x676e67/wreq/issues/43)) - ([a5661d9](https://github.com/0x676e67/wreq-python/commit/a5661d991a34626d427fd5097423e269ef29849a))

### Miscellaneous Tasks

- Fmt code - ([e833c89](https://github.com/0x676e67/wreq-python/commit/e833c89f0f34e64945af17491560da9de917fc3b))

### Build

- Lower glibc version requirements ([#44](https://github.com/0x676e67/wreq/issues/44)) - ([e2b3107](https://github.com/0x676e67/wreq-python/commit/e2b310721358f8a2709a543f146fbad18617d13d))


## [0.5.1](https://github.com/0x676e67/wreq-python/compare/v0.5.0..v0.5.1) - 2025-02-19

### Features

- *(proxy)* Enhanced proxy settings binding ([#42](https://github.com/0x676e67/wreq/issues/42)) - ([bfc78a2](https://github.com/0x676e67/wreq-python/commit/bfc78a2465ac91d57da2066f9a5606b27de644fc))

### Bug Fixes

- *(proxy)* Update proxy creation instructions and fix all methods ([#41](https://github.com/0x676e67/wreq/issues/41)) - ([c1b9bca](https://github.com/0x676e67/wreq-python/commit/c1b9bcaaa928c6527f1f18d4855486418a0c28c4))


## [0.5.0](https://github.com/0x676e67/wreq-python/compare/v0.2.6..v0.5.0) - 2025-02-18

### Features

- *(client)* Optionally enable asynchronous DNS resolver ([#37](https://github.com/0x676e67/wreq/issues/37)) - ([6ae8221](https://github.com/0x676e67/wreq-python/commit/6ae82212920ca52df79779921c7e061c9987cde7))
- *(client)* Added thread-safe update client ([#36](https://github.com/0x676e67/wreq/issues/36)) - ([9890224](https://github.com/0x676e67/wreq-python/commit/98902248c7dc80975a69d4bbade557082f5aac7b))
- Asynchronous DNS is disabled by default in client ([#39](https://github.com/0x676e67/wreq/issues/39)) - ([230cb7f](https://github.com/0x676e67/wreq-python/commit/230cb7f5168f7666f88729be9d9667f121a7d9fd))
- Added bridge from Rust to Python logging ([#38](https://github.com/0x676e67/wreq/issues/38)) - ([69e05bd](https://github.com/0x676e67/wreq-python/commit/69e05bd976e9ed9954bee2562122d8dded977cae))
- Add private mode Firefox135 support  ([#34](https://github.com/0x676e67/wreq/issues/34)) - ([517fe6e](https://github.com/0x676e67/wreq-python/commit/517fe6ee35351507c21711e5c02cd93d90fad8f2))
- Add `base_url` for client ([#33](https://github.com/0x676e67/wreq/issues/33)) - ([401383e](https://github.com/0x676e67/wreq-python/commit/401383ee0f3efa673584176885ae6106beb23a1c))

### Documentation

- Update rnet.pyi documents - ([a069213](https://github.com/0x676e67/wreq-python/commit/a069213063e898634f2cdf43adfa1d9b243d739e))

### Performance

- *(client)* Reuse default client to improve quick request efficiency ([#40](https://github.com/0x676e67/wreq/issues/40)) - ([6b0c01d](https://github.com/0x676e67/wreq-python/commit/6b0c01dae03edce52b59140f9edeccc4e415a326))

### Testing

- Add base url test - ([c3952f7](https://github.com/0x676e67/wreq-python/commit/c3952f75f1c96f6ccbaf761c13be3eae89891a9b))

### Miscellaneous Tasks

- *(error)* Improving Rust's borrowed memory management errors ([#35](https://github.com/0x676e67/wreq/issues/35)) - ([800c103](https://github.com/0x676e67/wreq-python/commit/800c103084ca1a1f1f5892367aa56e9361d722f1))
- Update `UpdateClientParams` usage documents - ([66e8e4d](https://github.com/0x676e67/wreq-python/commit/66e8e4db316743efc718811cee59cac22e41372f))


## [0.2.6](https://github.com/0x676e67/wreq-python/compare/v0.2.5..v0.2.6) - 2025-02-17

### Features

- Initial support for websocket ([#32](https://github.com/0x676e67/wreq/issues/32)) - ([116307f](https://github.com/0x676e67/wreq-python/commit/116307ffda9c5489e9dde4d14aad202408eb715c))
- Add jemallocator to improve memory allocation performance ([#31](https://github.com/0x676e67/wreq/issues/31)) - ([f74a198](https://github.com/0x676e67/wreq-python/commit/f74a198b83c23fc4186c41848457c92b902df833))

### Refactor

- Refactor websocket implement - ([13b4437](https://github.com/0x676e67/wreq-python/commit/13b443717b0637e0d35e615f6049c70717bf4689))

### Miscellaneous Tasks

- Remove dead code - ([f398896](https://github.com/0x676e67/wreq-python/commit/f398896f566cbae09a4bef6f1cede6534f7bec2a))


## [0.2.5](https://github.com/0x676e67/wreq-python/compare/v0.2.4..v0.2.5) - 2025-02-17

### Features

- Add Chrome133/Firefox135/Firefox Android135 ([#30](https://github.com/0x676e67/wreq/issues/30)) - ([c909b27](https://github.com/0x676e67/wreq-python/commit/c909b276d85bf8fb1ff07d62d4116496775c8978))


## [0.2.4](https://github.com/0x676e67/wreq-python/compare/v0.2.3..v0.2.4) - 2025-02-16

### Features

- *(response)* Return an integer representing the HTTP status code ([#28](https://github.com/0x676e67/wreq/issues/28)) - ([93dab63](https://github.com/0x676e67/wreq-python/commit/93dab63ff8e819a57a565d8dc7a14432b16740cb))

### Miscellaneous Tasks

- Fmt code - ([30baf55](https://github.com/0x676e67/wreq-python/commit/30baf556d446c8424f135782f7da2038c05f4a23))
- Update docs ([#27](https://github.com/0x676e67/wreq/issues/27)) - ([1018247](https://github.com/0x676e67/wreq-python/commit/101824780415f7a7a05758b0dc8f6d4ba27e3a3c))

### Build

- Update manylinux build ([#29](https://github.com/0x676e67/wreq/issues/29)) - ([21a52cc](https://github.com/0x676e67/wreq-python/commit/21a52ccbe5430757976e7611926d8d363f0a9553))


## [0.2.3](https://github.com/0x676e67/wreq-python/compare/v0.2.2..v0.2.3) - 2025-02-15

### Features

- *(client)* Improve impersonate FFI ([#22](https://github.com/0x676e67/wreq/issues/22)) - ([f4620ad](https://github.com/0x676e67/wreq-python/commit/f4620ad400d3122a03e808307cc49218c489e48d))

### Bug Fixes

- *(resp)* Fix cookie parser ([#23](https://github.com/0x676e67/wreq/issues/23)) - ([5743033](https://github.com/0x676e67/wreq-python/commit/57430339ddbbfe167295fffc6aa76cdef0c6aba1))
- Fix address/interface bindings ([#25](https://github.com/0x676e67/wreq/issues/25)) - ([6e45b0c](https://github.com/0x676e67/wreq-python/commit/6e45b0c6f32e8c7c42911ffe78d92858873e7dcf))

### Performance

- *(resp)* Reduce the range of GIL locks ([#21](https://github.com/0x676e67/wreq/issues/21)) - ([679ed20](https://github.com/0x676e67/wreq-python/commit/679ed2010d9446f5b56b6c079e42275398f61f61))
- Initialize and optimize Python module without requiring GIL ([#20](https://github.com/0x676e67/wreq/issues/20)) - ([f42f3f6](https://github.com/0x676e67/wreq-python/commit/f42f3f6861f8b165080438bb1d994ce81925a44f))

### Testing

- Add decompress tests ([#26](https://github.com/0x676e67/wreq/issues/26)) - ([b5519cc](https://github.com/0x676e67/wreq-python/commit/b5519cc04f2a07b3a307ab2cd0f2680e9fbb1fd2))
- Add auth request test - ([e3404d3](https://github.com/0x676e67/wreq-python/commit/e3404d30a7ea8966e0e4c4925c4734b29f6382bc))

### Miscellaneous Tasks

- Update workflows - ([bf82462](https://github.com/0x676e67/wreq-python/commit/bf824621e8a068dd3a74e694696d6545fa0e6ceb))
- Fmt code - ([fe54cd0](https://github.com/0x676e67/wreq-python/commit/fe54cd0164e69b5a5d63b3dd7e1dd528e3f7ee29))


## [0.2.2](https://github.com/0x676e67/wreq-python/compare/v0.2.0..v0.2.2) - 2025-02-14

### Features

- *(client)* Add redirect option ([#15](https://github.com/0x676e67/wreq/issues/15)) - ([55d9a2f](https://github.com/0x676e67/wreq-python/commit/55d9a2f529b9f8f81c539f8f82fab69ca57313a7))
- *(client)* Add cookie store option ([#14](https://github.com/0x676e67/wreq/issues/14)) - ([9a1c0de](https://github.com/0x676e67/wreq-python/commit/9a1c0de017f9444db7d30833e481f25f5e5f5035))
- *(resp)* Expose `Streamer` as public API ([#19](https://github.com/0x676e67/wreq/issues/19)) - ([f937f91](https://github.com/0x676e67/wreq-python/commit/f937f91d1338a3d0d848c780ccca957463f4a00b))
- *(resp)* Wrapper easy-to-use `StatusCode` ([#17](https://github.com/0x676e67/wreq/issues/17)) - ([eceb4b0](https://github.com/0x676e67/wreq-python/commit/eceb4b063f67c1fad312314e63633ea01a06a357))
- Shortcut method request support redirect ([#18](https://github.com/0x676e67/wreq/issues/18)) - ([21410bd](https://github.com/0x676e67/wreq-python/commit/21410bd2feb3c663175f8d9a9728f2297b07dac4))

### Build

- Fix manylinux build ([#16](https://github.com/0x676e67/wreq/issues/16)) - ([bb2e801](https://github.com/0x676e67/wreq-python/commit/bb2e8014dcdeb62660bbeb25c38b70f618a2a4e4))
- Ignore branch push trigger - ([605cae1](https://github.com/0x676e67/wreq-python/commit/605cae1d3ded9b7369a4d6ee8db634a5fb478634))


## [0.2.0] - 2025-02-13

### Features

- *(client)* Adapt client basic request bindings ([#11](https://github.com/0x676e67/wreq/issues/11)) - ([2187bbd](https://github.com/0x676e67/wreq-python/commit/2187bbd893a573500144c59bd95874c5ca776dfb))
- *(headers)* Implement `__str__` for HeaderMap - ([9b63009](https://github.com/0x676e67/wreq-python/commit/9b630096a332eab9d62d47e571435695c966d7dc))
- *(method)* Method implement `__str__` - ([bd70b17](https://github.com/0x676e67/wreq-python/commit/bd70b176cf79d0c7a94fb6ddbb9481465a92f7e0))
- *(resp)* Allows thread-safe operation of Response ([#9](https://github.com/0x676e67/wreq/issues/9)) - ([39a5cef](https://github.com/0x676e67/wreq-python/commit/39a5cefe0c0535eed72f3c17bfa6ebe934a0dcc7))
- *(response)* Add cookies support ([#6](https://github.com/0x676e67/wreq/issues/6)) - ([dea1240](https://github.com/0x676e67/wreq-python/commit/dea12403761108f37a48d8e6f2ad5fa75357a36d))
- *(response)* Add streaming support ([#5](https://github.com/0x676e67/wreq/issues/5)) - ([99c9685](https://github.com/0x676e67/wreq-python/commit/99c968591910d3f162cd297d5ce11994243acb3f))
- *(response)* Add `content_length` getter default zero - ([7824ad9](https://github.com/0x676e67/wreq-python/commit/7824ad90b55bc4cc7e75d1a1962cc8ab9c58c426))
- *(response)* Add `encoding` getter ([#4](https://github.com/0x676e67/wreq/issues/4)) - ([ece446d](https://github.com/0x676e67/wreq-python/commit/ece446df0ffce859ea0a42a2144a377eaa01e8ff))
- Add proxy binding - ([b86c9b2](https://github.com/0x676e67/wreq-python/commit/b86c9b232aef0d03d262e5c18ae5a98df97d795e))
- Improve the basic body request ([#10](https://github.com/0x676e67/wreq/issues/10)) - ([46df535](https://github.com/0x676e67/wreq-python/commit/46df5356ec2f6a5a176f395c4ce8c7389dd68733))
- Implement `str` for `Impersonate` - ([c51ffe0](https://github.com/0x676e67/wreq-python/commit/c51ffe0b18ef47d4589f1e10510190903bee2456))
- Complete the basic shortcut request ([#7](https://github.com/0x676e67/wreq/issues/7)) - ([849d320](https://github.com/0x676e67/wreq-python/commit/849d320aa744cb1c2dd88c2eb3caa5c65ea553b8))
- Complete basic request response bindings ([#3](https://github.com/0x676e67/wreq/issues/3)) - ([465d7ad](https://github.com/0x676e67/wreq-python/commit/465d7addf9396e8d1a320ad153a6f556789872e5))

### Performance

- *(resp)* Improve drop response overhead ([#8](https://github.com/0x676e67/wreq/issues/8)) - ([26eca5f](https://github.com/0x676e67/wreq-python/commit/26eca5fafeb732321ba02cac1707d1fef421d51b))

### Styling

- Fmt code ([#12](https://github.com/0x676e67/wreq/issues/12)) - ([e8bef38](https://github.com/0x676e67/wreq-python/commit/e8bef38bbd5ddc2137ab55baeb0bcd0fd0e28216))

### Miscellaneous Tasks

- Update docs - ([cdf16a7](https://github.com/0x676e67/wreq-python/commit/cdf16a78feec1217f414bb77be4c46663520ef21))
- Update `HeaderMap`  gen stub pymethods - ([e6b564c](https://github.com/0x676e67/wreq-python/commit/e6b564c3dd0533e13605d2a3e67f714d4d2888f0))
- Update Python SocketAddr type information - ([8be15b0](https://github.com/0x676e67/wreq-python/commit/8be15b0296f3221aa81536f2efbe4af98fc31bbf))
- Update Python HeaderMap type information - ([ac4aebf](https://github.com/0x676e67/wreq-python/commit/ac4aebfe9725020f124437e1470d718e47959607))
- Update Python HeaderMap type information - ([ccc6e10](https://github.com/0x676e67/wreq-python/commit/ccc6e10c59411d37b7d352d44d16ac913081f506))
- Update Python type information - ([cc94022](https://github.com/0x676e67/wreq-python/commit/cc940228c526915341f473e18cd249aaf416c39d))
- Adding Python type information - ([a7e357c](https://github.com/0x676e67/wreq-python/commit/a7e357cc1f79618605d8fe53fa0da05761c44b5f))
- Remove `#[allow(dead_code)]` - ([cc7153a](https://github.com/0x676e67/wreq-python/commit/cc7153a0bbfdbe1c333630655b113ac5a529f0fb))
- Update `get` docs - ([9dc9b87](https://github.com/0x676e67/wreq-python/commit/9dc9b87ff1f241c6c9eaa313f5aaf8d41b743949))
- Delete unused dependencies - ([0734197](https://github.com/0x676e67/wreq-python/commit/0734197efb66a6dfcdbfe685fe9ada8003db94e4))

### Build

- Update release action - ([eeeda61](https://github.com/0x676e67/wreq-python/commit/eeeda6148e944448ef47459ab302e7d93a197bf5))
- Add workflow build ([#13](https://github.com/0x676e67/wreq/issues/13)) - ([f1e4652](https://github.com/0x676e67/wreq-python/commit/f1e4652ee5a32533951e795668611409610e000e))
- Add style.yml action - ([5fea186](https://github.com/0x676e67/wreq-python/commit/5fea18653aa6125c7c671d6efff2bb179f3636a2))
- Generate import libraries for Python DLL for MinGW-w64 and MSVC (cross-)compile targets - ([98f90a4](https://github.com/0x676e67/wreq-python/commit/98f90a4f4e10fbfb7b2c7cc13b7b163f2fa946e5))

## New Contributors ❤️

* @0x676e67 made their first contribution

<!-- generated by git-cliff -->
