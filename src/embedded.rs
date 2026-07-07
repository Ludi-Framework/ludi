//! The framework's own Lua sources, embedded at compile time so a bundled
//! binary needs no LuaRocks tree. Keep in sync with the rockspec module
//! list.

pub const LUA_MODULES: &[(&str, &str)] = &[
    ("ludi", include_str!("../ludi/init.lua")),
    ("ludi.router", include_str!("../ludi/router.lua")),
    ("ludi.group", include_str!("../ludi/group.lua")),
    ("ludi.request", include_str!("../ludi/request.lua")),
    ("ludi.response", include_str!("../ludi/response.lua")),
    ("ludi.middleware", include_str!("../ludi/middleware.lua")),
    ("ludi.json", include_str!("../ludi/json.lua")),
    ("ludi.reload", include_str!("../ludi/reload.lua")),
    ("ludi.errfmt", include_str!("../ludi/errfmt.lua")),
];
