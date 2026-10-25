//! Talking to the Windows audio system (Core Audio / MMDevice API).
//!
//! Reading devices uses the documented `IMMDeviceEnumerator`. Changing the
//! default device has no documented API, so like every other switcher
//! (EarTrumpet, SoundSwitch, NirSoft's tools) we use the undocumented
//! `IPolicyConfig` COM interface that the Windows sound settings use
//! themselves. It has been stable since Windows 7 and needs no admin rights.

use std::sync::atomic::{AtomicIsize, Ordering};

use windows::core::{implement, Result, GUID, PCWSTR};
use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::Foundation::{HWND, LPARAM, PROPERTYKEY, WPARAM};
use windows::Win32::Media::Audio::{
    eCapture, eCommunications, eConsole, eMultimedia, eRender, EDataFlow, ERole, IMMDevice, IMMDeviceEnumerator,
    IMMNotificationClient, IMMNotificationClient_Impl, MMDeviceEnumerator, DEVICE_STATE, DEVICE_STATE_ACTIVE,
};
use windows::Win32::System::Com::StructuredStorage::{PropVariantClear, PropVariantToStringAlloc};
use windows::Win32::System::Com::{CoCreateInstance, CoTaskMemFree, CLSCTX_ALL, STGM_READ};
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;

use crate::ui::{from_wide_ptr, wide};

/// Playback (output) or recording (input).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    Output,
    Input,
}

impl Flow {
    pub const ALL: [Flow; 2] = [Flow::Output, Flow::Input];

    fn data_flow(self) -> EDataFlow {
        match self {
            Flow::Output => eRender,
            Flow::Input => eCapture,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Flow::Output => "Output",
            Flow::Input => "Input",
        }
    }

    pub fn index(self) -> usize {
        self as usize
    }
}

/// Which "default" slot of Windows we mean.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// "Default Device" (Windows uses two roles internally for this).
    Default,
    /// "Default Communications Device", used by Teams, Discord and so on.
    Communications,
}

impl Role {
    pub const ALL: [Role; 2] = [Role::Default, Role::Communications];

    pub fn index(self) -> usize {
        self as usize
    }
}

/// A device as Windows reports it right now.
#[derive(Debug, Clone)]
pub struct Device {
    pub id: String,
    pub name: String,
}

// ---------------------------------------------------------------------------
// The undocumented interface. The `#[interface]` macro generates the COM
// plumbing from the GUID and method list, like an `[ComImport]` interface in C#.
// Only `SetDefaultEndpoint` is called; the methods before it must still be
// listed, in order, so it lands in the right slot of the function table.
// ---------------------------------------------------------------------------

mod policy {
    // COM method names are PascalCase, so silence Rust's snake_case lint here.
    #![allow(non_snake_case)]
    use windows::core::{interface, IUnknown, IUnknown_Vtbl, HRESULT, PCWSTR};
    use windows::Win32::Foundation::PROPERTYKEY;
    use windows::Win32::Media::Audio::ERole;
    use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;

    #[interface("f8679f50-850a-41cf-9c72-430f290290c8")]
    pub unsafe trait IPolicyConfig: IUnknown {
        fn GetMixFormat(&self, id: PCWSTR, format: *mut *mut core::ffi::c_void) -> HRESULT;
        fn GetDeviceFormat(&self, id: PCWSTR, default: i32, format: *mut *mut core::ffi::c_void) -> HRESULT;
        fn ResetDeviceFormat(&self, id: PCWSTR) -> HRESULT;
        fn SetDeviceFormat(&self, id: PCWSTR, endpoint: *mut core::ffi::c_void, mix: *mut core::ffi::c_void)
            -> HRESULT;
        fn GetProcessingPeriod(&self, id: PCWSTR, default: i32, period: *mut i64, min: *mut i64) -> HRESULT;
        fn SetProcessingPeriod(&self, id: PCWSTR, period: *mut i64) -> HRESULT;
        fn GetShareMode(&self, id: PCWSTR, mode: *mut core::ffi::c_void) -> HRESULT;
        fn SetShareMode(&self, id: PCWSTR, mode: *mut core::ffi::c_void) -> HRESULT;
        fn GetPropertyValue(&self, id: PCWSTR, key: *const PROPERTYKEY, value: *mut PROPVARIANT) -> HRESULT;
        fn SetPropertyValue(&self, id: PCWSTR, key: *const PROPERTYKEY, value: *const PROPVARIANT) -> HRESULT;
        fn SetDefaultEndpoint(&self, id: PCWSTR, role: ERole) -> HRESULT;
        fn SetEndpointVisibility(&self, id: PCWSTR, visible: i32) -> HRESULT;
    }

    /// Public wrapper: the macro keeps the generated methods private to this module.
    pub unsafe fn set_default_endpoint(policy: &IPolicyConfig, id: PCWSTR, role: ERole) -> HRESULT {
        unsafe { policy.SetDefaultEndpoint(id, role) }
    }
}
use policy::IPolicyConfig;

const CLSID_POLICY_CONFIG: GUID = GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);

pub struct Audio {
    enumerator: IMMDeviceEnumerator,
    /// Created on first use, so SoundHop still starts (and can report the
    /// problem) if a Windows update ever removes the interface.
    policy: Option<IPolicyConfig>,
    notifier: Option<IMMNotificationClient>,
}

impl Audio {
    pub fn new() -> Result<Audio> {
        unsafe {
            let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            Ok(Audio { enumerator, policy: None, notifier: None })
        }
    }

    /// All devices of one kind that are plugged in and enabled.
    pub fn list(&self, flow: Flow) -> Vec<Device> {
        let mut out = Vec::new();
        unsafe {
            let Ok(collection) = self.enumerator.EnumAudioEndpoints(flow.data_flow(), DEVICE_STATE_ACTIVE) else {
                return out;
            };
            let count = collection.GetCount().unwrap_or(0);
            for i in 0..count {
                if let Ok(device) = collection.Item(i) {
                    if let Some(id) = device_id(&device) {
                        out.push(Device { name: friendly_name(&device).unwrap_or_else(|| id.clone()), id });
                    }
                }
            }
        }
        out
    }

    /// `true` if the device exists, is plugged in and is enabled.
    pub fn is_available(&self, id: &str) -> bool {
        let id = wide(id);
        unsafe {
            match self.enumerator.GetDevice(PCWSTR(id.as_ptr())) {
                Ok(device) => device.GetState().map(|s| s == DEVICE_STATE_ACTIVE).unwrap_or(false),
                Err(_) => false,
            }
        }
    }

    /// Windows' current default device for a flow and role.
    pub fn default_id(&self, flow: Flow, role: Role) -> Option<String> {
        let erole = match role {
            Role::Default => eConsole,
            Role::Communications => eCommunications,
        };
        unsafe {
            let device = self.enumerator.GetDefaultAudioEndpoint(flow.data_flow(), erole).ok()?;
            device_id(&device)
        }
    }

    /// Makes `id` the default device for the given role.
    pub fn set_default(&mut self, id: &str, role: Role) -> Result<()> {
        let id = wide(id);
        let roles: &[ERole] = match role {
            Role::Default => &[eConsole, eMultimedia],
            Role::Communications => &[eCommunications],
        };
        if self.policy.is_none() {
            self.policy = Some(unsafe { CoCreateInstance(&CLSID_POLICY_CONFIG, None, CLSCTX_ALL)? });
        }
        let policy = self.policy.as_ref().unwrap();
        for r in roles {
            unsafe { policy::set_default_endpoint(policy, PCWSTR(id.as_ptr()), *r).ok()? };
        }
        Ok(())
    }

    /// Asks Windows to tell us when devices appear, disappear or the default
    /// changes. Windows calls us on one of its own threads, so the callback
    /// only posts a message to our window and the real work happens on the UI
    /// thread.
    pub fn watch(&mut self, hwnd: HWND, message: u32) {
        NOTIFY_HWND.store(hwnd.0 as isize, Ordering::SeqCst);
        NOTIFY_MSG.store(message as isize, Ordering::SeqCst);
        let client: IMMNotificationClient = DeviceWatcher.into();
        unsafe {
            if self.enumerator.RegisterEndpointNotificationCallback(&client).is_ok() {
                self.notifier = Some(client);
            }
        }
    }
}

impl Drop for Audio {
    // Like `Dispose()` in C#, but called automatically when `Audio` goes away.
    fn drop(&mut self) {
        if let Some(client) = self.notifier.take() {
            unsafe {
                let _ = self.enumerator.UnregisterEndpointNotificationCallback(&client);
            }
        }
    }
}

fn device_id(device: &IMMDevice) -> Option<String> {
    unsafe {
        let p = device.GetId().ok()?;
        let id = from_wide_ptr(p.0);
        CoTaskMemFree(Some(p.0 as *const _));
        Some(id)
    }
}

fn friendly_name(device: &IMMDevice) -> Option<String> {
    unsafe {
        let store = device.OpenPropertyStore(STGM_READ).ok()?;
        let mut value = store.GetValue(&PKEY_Device_FriendlyName).ok()?;
        let text = PropVariantToStringAlloc(&value).ok();
        let _ = PropVariantClear(&mut value);
        let text = text?;
        let name = from_wide_ptr(text.0);
        CoTaskMemFree(Some(text.0 as *const _));
        Some(name)
    }
}

// The window that receives device notifications. Stored as a plain number
// because window handles may not be shared between threads in Rust's type
// system, but posting a message to one from another thread is fine.
static NOTIFY_HWND: AtomicIsize = AtomicIsize::new(0);
static NOTIFY_MSG: AtomicIsize = AtomicIsize::new(0);

fn post_change() {
    let hwnd = HWND(NOTIFY_HWND.load(Ordering::SeqCst) as *mut _);
    let msg = NOTIFY_MSG.load(Ordering::SeqCst) as u32;
    unsafe {
        let _ = PostMessageW(Some(hwnd), msg, WPARAM(0), LPARAM(0));
    }
}

/// Our implementation of `IMMNotificationClient`. `#[implement]` turns this
/// empty struct into a COM object, like implementing an interface in C#.
#[implement(IMMNotificationClient)]
struct DeviceWatcher;

impl IMMNotificationClient_Impl for DeviceWatcher_Impl {
    fn OnDeviceStateChanged(&self, _id: &PCWSTR, _state: DEVICE_STATE) -> Result<()> {
        post_change();
        Ok(())
    }

    fn OnDeviceAdded(&self, _id: &PCWSTR) -> Result<()> {
        post_change();
        Ok(())
    }

    fn OnDeviceRemoved(&self, _id: &PCWSTR) -> Result<()> {
        post_change();
        Ok(())
    }

    fn OnDefaultDeviceChanged(&self, _flow: EDataFlow, _role: ERole, _id: &PCWSTR) -> Result<()> {
        post_change();
        Ok(())
    }

    fn OnPropertyValueChanged(&self, _id: &PCWSTR, _key: &PROPERTYKEY) -> Result<()> {
        // Fires for many small things (like format changes). Device renames are
        // picked up the next time the menu or settings window opens.
        Ok(())
    }
}
