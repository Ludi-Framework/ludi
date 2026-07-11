# Local development without luarocks. Builds the native module for the
# chosen Lua and symlinks it next to the Lua sources so require() finds it.
#
#   make dev              # build for Lua 5.5
#   make dev LUA=lua54    # build for Lua 5.4
#   make dev LUA=luajit   # build for LuaJIT
#   make run              # run examples/hello.lua

LUA ?= lua55
LUA_BIN ?= lua5.5

# The cdylib name and the dynamic-loader extension differ per platform:
# Lua loads C modules as `ludi_core.so` on Linux/macOS and `ludi_core.dll`
# on Windows, while cargo emits `libludi_core.{so,dylib}` / `ludi_core.dll`.
# On Windows a symlink needs privileges, so copy the artifact instead.
ifeq ($(OS),Windows_NT)
	CORE := target/release/ludi_core.dll
	MODULE := ludi_core.dll
	CPATH := ./?.dll;;
	LINK := cp -f
else
	MODULE := ludi_core.so
	CPATH := ./?.so;;
	LINK := ln -sf
	ifeq ($(shell uname -s),Darwin)
		CORE := target/release/libludi_core.dylib
	else
		CORE := target/release/libludi_core.so
	endif
endif

dev:
	cargo build --release --features $(LUA)
	$(LINK) $(CORE) $(MODULE)

# The `ludi` CLI (ludi build): vendored static Lua 5.5, no module feature.
cli:
	cargo build --release --features cli --bin ludi

run: dev
	LUA_PATH="./?.lua;./?/init.lua;;" LUA_CPATH="$(CPATH)" $(LUA_BIN) examples/hello.lua

# Lua specs need busted (luarocks install busted), the Lua ecosystem's
# standard test framework. `luarocks test` works too.
test:
	cargo test --features $(LUA)
	busted

# Format Rust (rustfmt) and Lua (stylua) sources in place.
fmt:
	cargo fmt
	stylua ludi/ spec/ examples/

# Verify formatting without writing; fails if anything is out of style.
fmt-check:
	cargo fmt --check
	stylua --check ludi/ spec/ examples/

.PHONY: dev cli run test fmt fmt-check
