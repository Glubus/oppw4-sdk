use mlua::Lua;

fn runtime_lua() -> Lua {
    let lua = Lua::new();
    crate::runtime::install_runtime(&lua).expect("runtime");
    lua
}

#[test]
fn std_buffer_is_available_through_require_and_std() {
    let lua = runtime_lua();

    let value: String = lua
        .load(
            r#"
            local buffer = require("std.buffer")
            local writer = buffer.writer()
            writer:u8(1)
            return tostring(std.buffer ~= nil) .. ":" .. writer:len()
            "#,
        )
        .eval()
        .expect("std.buffer");

    assert_eq!(value, "true:1");
}

#[test]
fn writer_outputs_little_endian_bytes() {
    let lua = runtime_lua();

    let value: String = lua
        .load(
            r#"
            local buffer = require("std.buffer")
            local writer = buffer.writer()
            writer:u8(0x12)
            writer:u16_le(0x3456)
            writer:u32_le(0x789abcde)
            writer:i32_le(-2)
            writer:align(16, 0xff)
            local bytes = writer:to_bytes()
            return writer:len() .. ":" ..
                bytes[1] .. ":" .. bytes[2] .. ":" .. bytes[3] .. ":" ..
                bytes[4] .. ":" .. bytes[5] .. ":" .. bytes[6] .. ":" .. bytes[7] .. ":" ..
                bytes[8] .. ":" .. bytes[9] .. ":" .. bytes[10] .. ":" .. bytes[16]
            "#,
        )
        .eval()
        .expect("writer");

    assert_eq!(value, "16:18:86:52:222:188:154:120:254:255:255:255");
}

#[test]
fn reader_reads_little_endian_values() {
    let lua = runtime_lua();

    let value: String = lua
        .load(
            r#"
            local buffer = require("std.buffer")
            local reader = buffer.reader({0x12, 0x56, 0x34, 0xde, 0xbc, 0x9a, 0x78, 0xfe, 0xff, 0xff, 0xff})
            return reader:u8() .. ":" .. reader:u16_le() .. ":" .. reader:u32_le() .. ":" ..
                reader:i32_le() .. ":" .. reader:remaining()
            "#,
        )
        .eval()
        .expect("reader");

    assert_eq!(value, "18:13398:2023406814:-2:0");
}

#[test]
fn writer_bytes_and_reader_bytes_roundtrip() {
    let lua = runtime_lua();

    let value: String = lua
        .load(
            r#"
            local buffer = require("std.buffer")
            local writer = buffer.writer()
            writer:bytes({1, 2, 3})
            writer:bytes(string.char(4, 5))
            local reader = buffer.reader(writer:to_string())
            local first = reader:bytes(3)
            reader:seek(4)
            local second = reader:bytes(2)
            return first[1] .. first[2] .. first[3] .. ":" .. second[1] .. second[2] .. ":" .. reader:position()
            "#,
        )
        .eval()
        .expect("roundtrip");

    assert_eq!(value, "123:45:6");
}

#[test]
fn invalid_values_are_rejected() {
    let lua = runtime_lua();

    let error = lua
        .load(
            r#"
            local buffer = require("std.buffer")
            local writer = buffer.writer()
            writer:u8(256)
            "#,
        )
        .exec()
        .expect_err("u8 should fail");

    assert!(error.to_string().contains("u8 value out of range"));
}

#[test]
fn reader_rejects_out_of_bytes() {
    let lua = runtime_lua();

    let error = lua
        .load(
            r#"
            local buffer = require("std.buffer")
            local reader = buffer.reader({1})
            reader:u16_le()
            "#,
        )
        .exec()
        .expect_err("short read should fail");

    assert!(error.to_string().contains("reader out of bytes"));
}
