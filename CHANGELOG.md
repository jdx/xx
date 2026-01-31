## [2.1.2] - 2025-08-18

### 🚀 Features

- Add ungz method for gzip decompression (#96)
- Add file and environment utilities from mise (#99)
- Convert xx, clx, and ensembler to git subtrees
- Added support for process::cmd to read stdout/stderr by line

### 🐛 Bug Fixes

- Bug with stashing unstaged files (#160)
- Bug with <clx:flex> tags appearing when they should not (#161)

### ⚙️ Miscellaneous Tasks

- Pin homedir to 0.3.5 for MSRV compatibility
- Cargo build
## [2.5.1](https://github.com/jdx/xx/compare/v2.5.0...v2.5.1) (2026-01-31)


### Bug Fixes

* use stable API for same_file on Windows ([#177](https://github.com/jdx/xx/issues/177)) ([18bfc75](https://github.com/jdx/xx/commit/18bfc75cdb229953a37373281426d5bb60e3f0f9))

## [2.5.0](https://github.com/jdx/xx/compare/v2.4.0...v2.5.0) (2026-01-31)


### Features

* add archive creation support (tar.gz, tar.bz2, tar.xz, zip, gz) ([#174](https://github.com/jdx/xx/issues/174)) ([5bcaf38](https://github.com/jdx/xx/commit/5bcaf38989ebe9f9e8117befd9715b4d615c2658))
* expand git module with comprehensive operations ([#172](https://github.com/jdx/xx/issues/172)) ([a636d92](https://github.com/jdx/xx/commit/a636d926ff012ad3e6678b885854a064cf871807))
* expand HTTP module with POST, PUT, DELETE, headers, auth ([#173](https://github.com/jdx/xx/issues/173)) ([acd35b4](https://github.com/jdx/xx/commit/acd35b4e84a53eb2d7a5b3b697bf3f7d9330dd99))

## [2.4.0](https://github.com/jdx/xx/compare/v2.3.1...v2.4.0) (2026-01-31)


### Features

* add major utility improvements across all modules ([#170](https://github.com/jdx/xx/issues/170)) ([27121b8](https://github.com/jdx/xx/commit/27121b8fd89026e7b2559fdf517e06da3f560410))

## [2.3.1](https://github.com/jdx/xx/compare/v2.3.0...v2.3.1) (2026-01-19)


### Bug Fixes

* allow non-fast-forward updates in git fetch ([#163](https://github.com/jdx/xx/issues/163)) ([e3abcd6](https://github.com/jdx/xx/commit/e3abcd6e9eb17b621078ee4bfe1f8f4d41d4f2d2))
* **deps:** update rust crate reqwest to 0.13 ([#158](https://github.com/jdx/xx/issues/158)) ([878ca01](https://github.com/jdx/xx/commit/878ca016638ae16f3b1b07541b64910eb0cc89c7))
* **deps:** update rust crate zip to v7 ([#162](https://github.com/jdx/xx/issues/162)) ([b327786](https://github.com/jdx/xx/commit/b3277865b062461a39e5ff3e998bd6e44cd7d232))
* use PAT for release-please to trigger CI on PRs ([#164](https://github.com/jdx/xx/issues/164)) ([9a4c678](https://github.com/jdx/xx/commit/9a4c678e3f97eebec8376f9d188a7269309c34b4))

## [2.3.0](https://github.com/jdx/xx/compare/v2.2.0...v2.3.0) (2025-12-18)


### Features

* trigger 2.3.0 release for haiku module ([#150](https://github.com/jdx/xx/issues/150)) ([36e54ec](https://github.com/jdx/xx/commit/36e54ecdebdf14be8b419cf3c8bbe6125e9ef376))

## [2.2.0](https://github.com/jdx/xx/compare/v2.1.2...v2.2.0) (2025-12-18)


### Features

* add haiku random name generator ([#146](https://github.com/jdx/xx/issues/146)) ([ad572ad](https://github.com/jdx/xx/commit/ad572ad5a8b398560c2b95647445743a30d02c27))
* set MSRV to 1.85 with GitHub Actions verification ([#109](https://github.com/jdx/xx/issues/109)) ([72894ac](https://github.com/jdx/xx/commit/72894ac2d4ae7df2473a6897ba0670aa03f99c8b))


### Bug Fixes

* cross-platform doc tests for file and hash operations ([#107](https://github.com/jdx/xx/issues/107)) ([1997e4e](https://github.com/jdx/xx/commit/1997e4e96061f67044c5556ce12b22d6d31591ae))
* **deps:** update rust crate bzip2 to 0.6 ([#92](https://github.com/jdx/xx/issues/92)) ([8e0c6e5](https://github.com/jdx/xx/commit/8e0c6e5e469f4c327e21ee8de38ca7f5f591deb7))
* **deps:** update rust crate flate2 to v1.1.2 ([#90](https://github.com/jdx/xx/issues/90)) ([f6353be](https://github.com/jdx/xx/commit/f6353be5256faa2cb69aaa0c970b30755eef069b))
* **deps:** update rust crate homedir to v0.3.6 ([#101](https://github.com/jdx/xx/issues/101)) ([4f1e604](https://github.com/jdx/xx/commit/4f1e604bfe5763996c8299f3310cc536cf00bda3))
* **deps:** update rust crate log to v0.4.28 ([#104](https://github.com/jdx/xx/issues/104)) ([02735ff](https://github.com/jdx/xx/commit/02735ff8a33de6696d801dc6a4f449d132fa96c3))
* **deps:** update rust crate regex to v1.11.2 ([#106](https://github.com/jdx/xx/issues/106)) ([f5ea05e](https://github.com/jdx/xx/commit/f5ea05edb5bfff899d63491fbe346ea2be3fc60c))
* **deps:** update rust crate reqwest to v0.12.23 ([#88](https://github.com/jdx/xx/issues/88)) ([be565e8](https://github.com/jdx/xx/commit/be565e8a18d99014176ef0ae3bd93a7d244980ee))
* **deps:** update rust crate thiserror to v2.0.16 ([#100](https://github.com/jdx/xx/issues/100)) ([2b84e66](https://github.com/jdx/xx/commit/2b84e6663590c834f6c5da1b89c96bb53aa1d7af))
* **deps:** update rust crate zip to v6 ([#129](https://github.com/jdx/xx/issues/129)) ([fb5ed00](https://github.com/jdx/xx/commit/fb5ed00849f8a45f67735d0789da427a3b1775c8))
* disable reqwest default features to avoid native-tls ([11c9ecc](https://github.com/jdx/xx/commit/11c9eccd89790f46d16a6417db4c63f0939dafb7))
* downgrade homedir to 0.3.5 for MSRV compatibility ([f960ca7](https://github.com/jdx/xx/commit/f960ca7341b3acd08400fbdbd8d94e818d0ebec8))
* mark HTTP doctests as no_run to prevent network requests ([#119](https://github.com/jdx/xx/issues/119)) ([424c5a9](https://github.com/jdx/xx/commit/424c5a9c8e03ac6579dc61fac58d246572968c29))
* remove incorrect patch entry ([3cf78a0](https://github.com/jdx/xx/commit/3cf78a0fa0cb3edfe1d858e812703c7404f21b66))
* use cargo-msrv command directly instead of cargo subcommand ([#111](https://github.com/jdx/xx/issues/111)) ([5cd9419](https://github.com/jdx/xx/commit/5cd9419e1af618debe9db34d5947d83b1627c502))

## [2.1.1] - 2025-05-15

### ⚙️ Miscellaneous Tasks

- Updated deps
- Release xx version 2.1.1
## [2.1.0] - 2025-04-25

### 🚀 Features

- Added duct cmd expression
- *(process)* Added arg/args

### 🧪 Testing

- Harden tests

### ⚙️ Miscellaneous Tasks

- Bump deps
- Remove -x
- Added hk
- Added pkl
- Release xx version 2.1.0
## [2.0.5] - 2025-02-17

### 🐛 Bug Fixes

- Clone options (#72)
- Add stub make_executable for windows

### ⚙️ Miscellaneous Tasks

- Updated deps
- Fix cargo includes
- Release xx version 2.0.5
## [2.0.4] - 2025-02-01

### 🚀 Features

- Add branch support on clone (#71)

### 🐛 Bug Fixes

- *(deps)* Update rust crate bzip2 to 0.5 (#62)

### ⚙️ Miscellaneous Tasks

- Lint issue
- Updated deps
- Cargo up
- Cargo up
- Always save cache
- Release hook info
- Set cargo include
- Release xx version 2.0.4
## [2.0.3] - 2024-12-12

### 🚀 Features

- Find_up

### ⚙️ Miscellaneous Tasks

- Release xx version 2.0.3
## [2.0.2] - 2024-12-10

### 🚀 Features

- Reexport fs::file

### ⚙️ Miscellaneous Tasks

- Add hash as dependency of fslock
- Rename fslock struct
- Rename fslock struct
- Must_use
- Release xx version 2.0.1
- Release xx version 2.0.2
## [2.0.0] - 2024-12-06

### 🚀 Features

- Fslock

### ⚙️ Miscellaneous Tasks

- Upgraded miette
- Release xx version 2.0.0
## [1.1.9] - 2024-11-11

### 🐛 Bug Fixes

- *(deps)* Update rust crate flate2 to v1.0.33 (#44)
- *(deps)* Update rust crate reqwest to v0.12.7 (#45)
- *(deps)* Update rust crate filetime to v0.2.25 (#47)
- *(deps)* Update rust crate flate2 to v1.0.34 (#48)
- *(deps)* Update rust crate tar to v0.4.42 (#49)
- *(deps)* Update rust crate homedir to v0.3.4 (#50)
- *(deps)* Update rust crate reqwest to v0.12.8 (#51)

### ⚙️ Miscellaneous Tasks

- Updated deps
- Release xx version 1.1.9
## [1.1.8] - 2024-08-19

### 🐛 Bug Fixes

- Statically link xz

### ⚙️ Miscellaneous Tasks

- Release xx version 1.1.8
## [1.1.7] - 2024-08-18

### 🐛 Bug Fixes

- Windows compat

### ⚙️ Miscellaneous Tasks

- Release xx version 1.1.7
## [1.1.6] - 2024-08-18

### 🐛 Bug Fixes

- *(deps)* Update rust crate zip to v2.1.2 (#18)
- *(deps)* Update rust crate tokio to v1.38.0 (#19)
- *(deps)* Update rust crate regex to v1.10.5 (#20)
- *(deps)* Update rust crate tar to v0.4.41 (#21)
- *(deps)* Update rust crate zip to v2.1.3 (#22)
- *(deps)* Update rust crate reqwest to v0.12.5 (#24)
- *(deps)* Update rust crate log to v0.4.22 (#26)
- *(deps)* Update rust crate thiserror to v1.0.62 (#30)
- *(deps)* Update rust crate thiserror to v1.0.63 (#32)
- *(deps)* Update rust crate tokio to v1.38.1 (#33)
- *(deps)* Update rust crate zip to v2.1.5 (#34)
- *(deps)* Update rust crate tokio to v1.39.2 (#37)
- *(deps)* Update rust crate flate2 to v1.0.31 (#39)
- *(deps)* Update rust crate regex to v1.10.6 (#40)
- *(deps)* Update rust crate filetime to v0.2.24 (#41)
- *(deps)* Update rust crate zip to v2.1.6 (#42)
- Chmod zip unarchiving (#43)

### ⚙️ Miscellaneous Tasks

- Release xx version 1.1.6
## [1.1.5] - 2024-05-25

### ⚙️ Miscellaneous Tasks

- Updated deps
- Release xx version 1.1.5
## [1.1.4] - 2024-05-25

### 🐛 Bug Fixes

- *(git)* Make clone static

### ⚙️ Miscellaneous Tasks

- Release xx version 1.1.4
## [1.1.3] - 2024-05-25

### 🐛 Bug Fixes

- *(git)* Make clone static

### ⚙️ Miscellaneous Tasks

- Release xx version 1.1.3
## [1.1.2] - 2024-05-25

### 🚀 Features

- *(file)* Added mv

### 🐛 Bug Fixes

- *(deps)* Update rust crate thiserror to v1.0.61 (#15)
- *(file)* Accept generic content for write()
- *(http)* Create dir for download
- *(file)* Create dir before moving

### ⚙️ Miscellaneous Tasks

- Release xx version 1.1.2
## [1.1.1] - 2024-05-14

### 🚀 Features

- *(hash)* Added ensure_checksum_sha512

### 🐛 Bug Fixes

- Hash generics

### ⚙️ Miscellaneous Tasks

- *(hash)* Debug logging
- Release xx version 1.1.1
## [1.1.0] - 2024-05-14

### 🚀 Features

- Added hash functions

### 🐛 Bug Fixes

- *(deps)* Update rust crate thiserror to v1.0.60 (#13)

### ⚙️ Miscellaneous Tasks

- Release xx version 1.1.0
## [1.0.2] - 2024-05-13

### 🐛 Bug Fixes

- Use async reqwest

### ⚙️ Miscellaneous Tasks

- Release xx version 1.0.2
## [1.0.1] - 2024-05-13

### 🚀 Features

- Added rustls feature

### ⚙️ Miscellaneous Tasks

- Release xx version 1.0.1
## [1.0.0] - 2024-05-13

### 🚀 Features

- Added file::remove_dir_all
- Http

### 🧪 Testing

- Enable logging in unit tests
- Enable trace logging
- Show coverage results in action output

### ⚙️ Miscellaneous Tasks

- Added coverage (#12)
- Release xx version 1.0.0
## [0.5.1] - 2024-05-12

### 🐛 Bug Fixes

- Lib.rs

### 📚 Documentation

- Added CHANGELOG

### ⚙️ Miscellaneous Tasks

- Release xx version 0.5.1
## [0.5.0] - 2024-05-12

### 🚀 Features

- Glob function

### ⚙️ Miscellaneous Tasks

- Release xx version 0.5.0
## [0.4.0] - 2024-05-11

### 🚀 Features

- Archive functions

### ⚙️ Miscellaneous Tasks

- Release xx version 0.4.0
## [0.3.0] - 2024-04-25

### 🚀 Features

- Added git module

### ⚙️ Miscellaneous Tasks

- Release xx version 0.3.0
## [0.2.5] - 2024-02-10

### ⚙️ Miscellaneous Tasks

- Release xx version 0.2.5
## [0.2.4] - 2024-02-10

### ⚙️ Miscellaneous Tasks

- Release xx version 0.2.4
## [0.2.3] - 2024-02-09

### 🐛 Bug Fixes

- *(deps)* Update rust crate miette to v6

### ⚙️ Miscellaneous Tasks

- Release xx version 0.2.3
## [0.2.2] - 2024-01-14

### ⚙️ Miscellaneous Tasks

- Release xx version 0.2.2
## [0.2.1] - 2024-01-14

### ⚙️ Miscellaneous Tasks

- Release xx version 0.2.1
## [0.2.0] - 2024-01-13
