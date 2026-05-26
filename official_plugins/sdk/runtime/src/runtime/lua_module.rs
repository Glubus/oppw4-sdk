macro_rules! runtime_lua_module {
    (
        type = $type_name:ident,
        module = $module_name:expr,
        factory = $factory:path $(,)?
    ) => {
        pub(crate) struct $type_name;

        impl crate::runtime::lua_module::RuntimeLuaModule for $type_name {
            const NAME: &'static str = $module_name;

            fn register_fn() -> plugin_sdk::Oppw4LuaRegisterFn {
                Self::register_module
            }
        }

        impl $type_name {
            unsafe extern "system" fn register_module(
                _context: *mut std::ffi::c_void,
                lua: *mut std::ffi::c_void,
            ) -> i32 {
                let Some(lua) = (unsafe { lua.cast::<mlua::Lua>().as_ref() }) else {
                    return -1;
                };
                match $factory(lua)
                    .and_then(|table| lua_api::register_module(lua, $module_name, table))
                {
                    Ok(()) => 0,
                    Err(_) => -2,
                }
            }
        }
    };

    (
        type = $type_name:ident,
        module = $module_name:expr,
        context = $context_ty:ty,
        factory = $factory:path $(,)?
    ) => {
        pub(crate) struct $type_name {
            context: $context_ty,
        }

        impl $type_name {
            pub(crate) fn new(context: $context_ty) -> Self {
                Self { context }
            }

            unsafe extern "system" fn register_module(
                context: *mut std::ffi::c_void,
                lua: *mut std::ffi::c_void,
            ) -> i32 {
                let Some(context) = (unsafe { context.cast::<$context_ty>().as_ref() }) else {
                    return -1;
                };
                let Some(lua) = (unsafe { lua.cast::<mlua::Lua>().as_ref() }) else {
                    return -2;
                };
                match $factory(lua, std::sync::Arc::clone(context))
                    .and_then(|table| lua_api::register_module(lua, $module_name, table))
                {
                    Ok(()) => 0,
                    Err(_) => -3,
                }
            }
        }

        impl crate::runtime::lua_module::RuntimeLuaModule for $type_name {
            const NAME: &'static str = $module_name;

            fn into_context(self) -> *mut std::ffi::c_void {
                Box::into_raw(Box::new(self.context)).cast()
            }

            unsafe fn drop_context(context: *mut std::ffi::c_void) {
                unsafe {
                    drop(Box::from_raw(context.cast::<$context_ty>()));
                }
            }

            fn register_fn() -> plugin_sdk::Oppw4LuaRegisterFn {
                Self::register_module
            }
        }
    };
}

pub(crate) use runtime_lua_module;

pub(crate) trait RuntimeLuaModule {
    const OWNER: &'static str = "sdk_runtime";
    const NAME: &'static str;

    fn into_context(self) -> *mut std::ffi::c_void
    where
        Self: Sized,
    {
        std::ptr::null_mut()
    }

    unsafe fn drop_context(_context: *mut std::ffi::c_void) {}

    fn register_fn() -> plugin_sdk::Oppw4LuaRegisterFn
    where
        Self: Sized;
}

pub(crate) fn register<M>(host: plugin_sdk::HostApi<'_>, module: M)
where
    M: RuntimeLuaModule,
{
    let context = module.into_context();
    let result = match host
        .lua()
        .register_module_fn(M::OWNER, M::NAME, context, M::register_fn())
    {
        Ok(()) => 0,
        Err(plugin_sdk::PluginError::HostCallFailed { code, .. }) => code,
        Err(_) => -1,
    };

    if result != 0 {
        unsafe {
            M::drop_context(context);
        }
        let _ = host.log().write(
            M::OWNER,
            format!("{} lua module register failed result={result}", M::NAME),
        );
    }
}
