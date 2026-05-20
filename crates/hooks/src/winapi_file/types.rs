use std::ffi::c_void;

pub(crate) type Bool = i32;
pub(crate) type Dword = u32;
pub(crate) type Handle = *mut c_void;
pub(crate) type Lpcwstr = *const u16;
pub(crate) type Lpvoid = *mut c_void;
pub(crate) type Lpdword = *mut Dword;
pub(crate) type LargeInteger = i64;

pub(crate) const GENERIC_READ: Dword = 0x8000_0000;
pub(crate) const INVALID_HANDLE_VALUE: Handle = !0usize as Handle;
pub(crate) const FAKE_HANDLE_MASK: usize = 0xf000_0000_0000_0000;
pub(crate) const FAKE_HANDLE_BITS: usize = 0x1000_0000_0000_0000;
pub(crate) const FILE_TYPE_DISK: Dword = 0x0000_0001;
pub(crate) const IMAGE_DIRECTORY_ENTRY_IMPORT: usize = 1;
pub(crate) const IMAGE_ORDINAL_FLAG64: u64 = 0x8000_0000_0000_0000;

pub(crate) type CreateFileWFn =
    unsafe extern "system" fn(Lpcwstr, Dword, Dword, Lpvoid, Dword, Dword, Handle) -> Handle;
pub(crate) type ReadFileFn =
    unsafe extern "system" fn(Handle, Lpvoid, Dword, Lpdword, Lpvoid) -> Bool;
pub(crate) type CloseHandleFn = unsafe extern "system" fn(Handle) -> Bool;
pub(crate) type GetFileSizeExFn = unsafe extern "system" fn(Handle, *mut LargeInteger) -> Bool;
pub(crate) type GetFileTimeFn = unsafe extern "system" fn(Handle, Lpvoid, Lpvoid, Lpvoid) -> Bool;
pub(crate) type GetFileTypeFn = unsafe extern "system" fn(Handle) -> Dword;
pub(crate) type SetFilePointerExFn =
    unsafe extern "system" fn(Handle, LargeInteger, *mut LargeInteger, Dword) -> Bool;

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct FileTime {
    pub(crate) low_date_time: u32,
    pub(crate) high_date_time: u32,
}

#[link(name = "kernel32")]
extern "system" {
    pub(crate) fn GetSystemTimeAsFileTime(system_time_as_file_time: *mut FileTime);
}

#[derive(Clone, Copy)]
pub(crate) struct OriginalFunctions {
    pub(crate) create_file_w: Option<CreateFileWFn>,
    pub(crate) read_file: Option<ReadFileFn>,
    pub(crate) close_handle: Option<CloseHandleFn>,
    pub(crate) get_file_size_ex: Option<GetFileSizeExFn>,
    pub(crate) get_file_time: Option<GetFileTimeFn>,
    pub(crate) get_file_type: Option<GetFileTypeFn>,
    pub(crate) set_file_pointer_ex: Option<SetFilePointerExFn>,
}

impl OriginalFunctions {
    pub(crate) fn empty() -> Self {
        Self {
            create_file_w: None,
            read_file: None,
            close_handle: None,
            get_file_size_ex: None,
            get_file_time: None,
            get_file_type: None,
            set_file_pointer_ex: None,
        }
    }
}

#[repr(C)]
pub(crate) struct ImageImportDescriptor {
    pub(crate) original_first_thunk: u32,
    pub(crate) time_date_stamp: u32,
    pub(crate) forwarder_chain: u32,
    pub(crate) name: u32,
    pub(crate) first_thunk: u32,
}
