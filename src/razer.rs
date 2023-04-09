use guid::GUID;
use libloading::Library;
use std::fmt::Display;
use std::path::PathBuf;

#[allow(dead_code)]
#[derive(Display)]
#[repr(C)]
pub enum RzResult {
    Invalid = -1,
    Success = 0,
    AccessDenied = 5,
    InvalidHandle = 6,
    InvalidAccess = 12,
    NotSupported = 50,
    InvalidParameter = 87,
    ServiceNotExist = 1060,
    ServiceNotActive = 1062,
    SingleInstanceApp = 1152,
    DeviceNotConnected = 1167,
    RequestAborted = 1235,
    NotAuthenticated = 1244,
    AlreadyInitialized = 1247,
    ResourceDisabled = 4309,
    DeviceNotAvailable = 4319,
    NotValidState = 5023,
    InsufficientAccessRights = 8344,
    NoMoreItems = 259,
    Failed = 2147483647,
}

#[allow(dead_code)]
#[derive(Display)]
#[repr(C)]
pub enum RzType {
    BroadcastEffect = 1,
    BroadcastStatus = 2,
}

#[allow(dead_code)]
#[derive(Display)]
#[repr(C)]
pub enum RzStatus {
    Live = 1,
    NotLive = 2,
}

pub struct Chroma {
    lib: Library,
}

impl Chroma {
    pub fn init(id: GUID, dll: Option<String>) -> Chroma {
        // load lib
        let sdk_path = match dll {
            Some(dll) => PathBuf::from(dll),
            None => PathBuf::from(std::env::var_os("ProgramFiles").unwrap())
                .join("Razer/ChromaBroadcast/bin/RzChromaBroadcastAPI64.dll"),
        };
        let lib = unsafe { libloading::Library::new(sdk_path).expect("Unable to load lib") };

        unsafe {
            let func: libloading::Symbol<unsafe extern "C" fn(uuid: GUID) -> RzResult> =
                lib.get(b"Init").expect("Init symbol loading failed.");
            func(id);
        }

        Chroma { lib }
    }

    pub fn uninit(&mut self) -> RzResult {
        unsafe {
            let func: libloading::Symbol<unsafe extern "C" fn() -> RzResult> = self
                .lib
                .get(b"UnInit")
                .expect("UnInit symbol loading failed.");
            func()
        }
    }

    pub fn register_event_notification(
        &mut self,
        callback: unsafe extern "C" fn(RzType, *const i32),
    ) -> RzResult {
        unsafe {
            let func: libloading::Symbol<
                unsafe extern "C" fn(unsafe extern "C" fn(RzType, *const i32)) -> RzResult,
            > = self.lib.get(b"RegisterEventNotification").unwrap();
            func(callback)
        }
    }

    pub fn unregister_event_notification(&mut self) -> RzResult {
        unsafe {
            let func: libloading::Symbol<unsafe extern "C" fn() -> RzResult> = self
                .lib
                .get(b"UnRegisterEventNotification")
                .expect("UnInit symbol loading failed.");
            func()
        }
    }
}

impl Drop for Chroma {
    fn drop(&mut self) {
        eprintln!("Doing some final cleanup");
        self.unregister_event_notification();
        self.uninit();
    }
}
