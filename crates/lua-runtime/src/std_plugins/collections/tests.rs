use mlua::Lua;

fn runtime_lua() -> Lua {
    let lua = Lua::new();
    crate::runtime::install_runtime(&lua).expect("runtime");
    lua
}

#[test]
fn std_collections_is_available_through_require_and_std() {
    let lua = runtime_lua();

    let value: String = lua
        .load(
            r#"
            local collections = require("std.collections")
            local map = collections.map()
            map:set("garp", 12)
            return tostring(std.collections ~= nil) .. ":" .. map:get("garp")
            "#,
        )
        .eval()
        .expect("std.collections");

    assert_eq!(value, "true:12");
}

#[test]
fn map_tracks_entries_and_len() {
    let lua = runtime_lua();

    let value: String = lua
        .load(
            r#"
            local collections = require("std.collections")
            local map = collections.map()
            map:set("garp", 12)
            map:set(42, "answer")
            map:set(true, "yes")
            map:set("garp", 13)
            local entries = map:entries()
            local removed = map:remove(42)
            return map:len() .. ":" .. map:get("garp") .. ":" .. tostring(map:has(42)) ..
                ":" .. removed .. ":" .. entries[1].key .. ":" .. entries[1].value ..
                ":" .. map:get_or("missing", "fallback")
            "#,
        )
        .eval()
        .expect("map");

    assert_eq!(value, "2:13:false:answer:garp:13:fallback");
}

#[test]
fn map_rejects_complex_keys() {
    let lua = runtime_lua();

    let error = lua
        .load(
            r#"
            local collections = require("std.collections")
            local map = collections.map()
            map:set({}, "nope")
            "#,
        )
        .exec()
        .expect_err("complex key should fail");

    assert!(error.to_string().contains("map key must be string"));
}

#[test]
fn ring_buffer_keeps_last_values_in_order() {
    let lua = runtime_lua();

    let value: String = lua
        .load(
            r#"
            local collections = require("std.collections")
            local ring = collections.ring_buffer(3)
            ring:push("a")
            ring:push("b")
            ring:push("c")
            ring:push("d")
            local values = ring:values()
            return ring:len() .. ":" .. ring:capacity() .. ":" .. tostring(ring:is_full()) ..
                ":" .. values[1] .. values[2] .. values[3] .. ":" .. ring:last() .. ":" .. ring:get(1)
            "#,
        )
        .eval()
        .expect("ring buffer");

    assert_eq!(value, "3:3:true:bcd:d:b");
}

#[test]
fn ring_buffer_rejects_zero_capacity() {
    let lua = runtime_lua();

    let error = lua
        .load(
            r#"
            local collections = require("std.collections")
            collections.ring_buffer(0)
            "#,
        )
        .exec()
        .expect_err("zero capacity should fail");

    assert!(error
        .to_string()
        .contains("ring_buffer capacity must be positive"));
}
