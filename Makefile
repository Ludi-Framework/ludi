# Local development without luarocks. Builds the native module for the
# chosen Lua and symlinks it next to the Lua sources so require() finds it.
#
#   make dev              # build for Lua 5.4
#   make dev LUA=luajit   # build for LuaJIT
#   make run              # run examples/hello.lua

LUA ?= lua54
LUA_BIN ?= lua5.4

dev:
	cargo build --release --features mlua/$(LUA)
	ln -sf target/release/libludi_core.so ludi_core.so

run: dev
	LUA_PATH="./?.lua;./?/init.lua;;" LUA_CPATH="./?.so;;" $(LUA_BIN) examples/hello.lua

# Lua specs need busted (luarocks install busted), the Lua ecosystem's
# standard test framework. `luarocks test` works too.
test:
	cargo test --features mlua/$(LUA)
	busted

.PHONY: dev run test
