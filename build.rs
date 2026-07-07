fn main() {
    // The CLI binary links Lua statically; native rocks dlopen'ed at run
    // time (fredy_core.so, lsqlite3.so, ...) resolve lua_* against the
    // host process. Linux only exports an executable's symbols to dlopen'd
    // libraries with -rdynamic; macOS does it by default.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        println!("cargo:rustc-link-arg-bins=-rdynamic");
    }
}
