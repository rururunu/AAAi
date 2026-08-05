//! COM session + IDispatch late-binding helpers for Office automation.

use std::ptr;
use std::sync::Arc;

use windows::core::{Interface, GUID, HRESULT, HSTRING, PCWSTR, BSTR, VARIANT};
use windows::Win32::System::Com::{
    CLSIDFromProgID, CoInitializeEx, CoUninitialize, DISPATCH_METHOD, DISPATCH_PROPERTYGET,
    DISPATCH_PROPERTYPUT, DISPPARAMS, IDispatch, COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::Ole::{GetActiveObject, DISPID_PROPERTYPUT, DISPID_UNKNOWN};

const LOCALE_USER_DEFAULT: u32 = 0x0400;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComError {
    Init(String),
    ProgId(String, String),
    NotRunning(String, String),
    Cast(String),
    Member(String, String),
    Invoke(String, String),
    Type(String),
}

impl std::fmt::Display for ComError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Init(message) => write!(f, "COM init failed: {message}"),
            Self::ProgId(prog, message) => write!(f, "CLSIDFromProgID({prog}) failed: {message}"),
            Self::NotRunning(prog, message) => write!(f, "{prog} is not running: {message}"),
            Self::Cast(message) => write!(f, "COM cast failed: {message}"),
            Self::Member(name, message) => write!(f, "COM member `{name}` lookup failed: {message}"),
            Self::Invoke(name, message) => write!(f, "COM invoke `{name}` failed: {message}"),
            Self::Type(message) => write!(f, "COM value conversion failed: {message}"),
        }
    }
}

impl std::error::Error for ComError {}

pub struct ComSession {
    should_uninit: bool,
}

impl ComSession {
    pub fn new() -> Result<Self, ComError> {
        unsafe {
            let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            if hr.is_ok() {
                return Ok(Self { should_uninit: true });
            }
            if hr == HRESULT(0x80010106u32 as i32) {
                return Ok(Self { should_uninit: false });
            }
            Err(ComError::Init(format!("{hr:?}")))
        }
    }
}

impl Drop for ComSession {
    fn drop(&mut self) {
        if self.should_uninit {
            unsafe {
                CoUninitialize();
            }
        }
    }
}

#[derive(Clone)]
pub struct ComDispatch {
    inner: Arc<IDispatch>,
}

impl std::fmt::Debug for ComDispatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComDispatch").finish_non_exhaustive()
    }
}

impl ComDispatch {
    pub(crate) fn attach_active(prog_id: &str) -> Result<Self, ComError> {
        unsafe {
            let clsid = CLSIDFromProgID(&HSTRING::from(prog_id))
                .map_err(|error| ComError::ProgId(prog_id.to_string(), error.to_string()))?;
            let mut unknown = None;
            GetActiveObject(&clsid, None, &mut unknown).map_err(|error| {
                ComError::NotRunning(prog_id.to_string(), error.to_string())
            })?;
            let unknown = unknown.ok_or_else(|| {
                ComError::NotRunning(prog_id.to_string(), "active object missing".to_string())
            })?;
            let dispatch: IDispatch = unknown
                .cast()
                .map_err(|error| ComError::Cast(error.to_string()))?;
            Ok(Self {
                inner: Arc::new(dispatch),
            })
        }
    }

    fn dispid(&self, name: &str) -> Result<i32, ComError> {
        let mut dispid = DISPID_UNKNOWN;
        let name_h = HSTRING::from(name);
        unsafe {
            self.inner
                .GetIDsOfNames(
                    &GUID::zeroed(),
                    &PCWSTR::from_raw(name_h.as_ptr()),
                    1,
                    LOCALE_USER_DEFAULT,
                    &mut dispid,
                )
                .map_err(|error| ComError::Member(name.to_string(), error.to_string()))?;
        }
        Ok(dispid)
    }

    pub fn get(&self, name: &str) -> Result<ComValue, ComError> {
        let dispid = self.dispid(name)?;
        let mut result = VARIANT::default();
        let dispparams = DISPPARAMS::default();
        unsafe {
            self.inner
                .Invoke(
                    dispid,
                    &GUID::zeroed(),
                    LOCALE_USER_DEFAULT,
                    DISPATCH_PROPERTYGET,
                    &dispparams,
                    Some(&mut result),
                    None,
                    None,
                )
                .map_err(|error| ComError::Invoke(name.to_string(), error.to_string()))?;
        }
        ComValue::from_variant(result)
    }

    pub fn set(&self, name: &str, value: VARIANT) -> Result<(), ComError> {
        let dispid = self.dispid(name)?;
        let mut arg = value;
        // PROPERTYPUT requires the special named arg DISPID_PROPERTYPUT (-3).
        // Without it, Office returns DISP_E_PARAMNOTFOUND (0x80020004).
        let mut named = DISPID_PROPERTYPUT;
        let dispparams = DISPPARAMS {
            rgvarg: &mut arg,
            cArgs: 1,
            rgdispidNamedArgs: &mut named,
            cNamedArgs: 1,
        };
        unsafe {
            self.inner
                .Invoke(
                    dispid,
                    &GUID::zeroed(),
                    LOCALE_USER_DEFAULT,
                    DISPATCH_PROPERTYPUT,
                    &dispparams,
                    None,
                    None,
                    None,
                )
                .map_err(|error| ComError::Invoke(name.to_string(), error.to_string()))?;
        }
        Ok(())
    }

    pub fn to_variant(&self) -> VARIANT {
        let unknown: windows::core::IUnknown = self.inner.cast().expect("IDispatch to IUnknown");
        VARIANT::from(unknown)
    }

    pub fn call(&self, name: &str, args: &[VARIANT]) -> Result<ComValue, ComError> {
        let dispid = self.dispid(name)?;
        let mut rev_args: Vec<VARIANT> = args.iter().cloned().rev().collect();
        let dispparams = DISPPARAMS {
            rgvarg: rev_args.as_mut_ptr(),
            cArgs: rev_args.len() as u32,
            rgdispidNamedArgs: ptr::null_mut(),
            cNamedArgs: 0,
        };
        let mut result = VARIANT::default();
        unsafe {
            self.inner
                .Invoke(
                    dispid,
                    &GUID::zeroed(),
                    LOCALE_USER_DEFAULT,
                    DISPATCH_METHOD,
                    &dispparams,
                    Some(&mut result),
                    None,
                    None,
                )
                .map_err(|error| ComError::Invoke(name.to_string(), error.to_string()))?;
        }
        ComValue::from_variant(result)
    }
}

#[derive(Debug, Clone)]
pub enum ComValue {
    Empty,
    String(String),
    Int(i32),
    Bool(bool),
    Dispatch(ComDispatch),
}

impl ComValue {
    pub fn from_variant(value: VARIANT) -> Result<Self, ComError> {
        if value.is_empty() {
            return Ok(Self::Empty);
        }
        if let Ok(text) = BSTR::try_from(&value) {
            return Ok(Self::String(text.to_string()));
        }
        if let Ok(number) = i32::try_from(&value) {
            return Ok(Self::Int(number));
        }
        if let Ok(flag) = bool::try_from(&value) {
            return Ok(Self::Bool(flag));
        }
        if let Ok(dispatch) = IDispatch::try_from(&value) {
            return Ok(Self::Dispatch(ComDispatch {
                inner: Arc::new(dispatch),
            }));
        }
        Err(ComError::Type("unsupported VARIANT type".to_string()))
    }

    pub fn into_string(self) -> Result<String, ComError> {
        match self {
            Self::String(text) => Ok(text),
            Self::Empty => Ok(String::new()),
            other => Err(ComError::Type(format!("expected string, got {other:?}"))),
        }
    }

    pub fn into_int(self) -> Result<i32, ComError> {
        match self {
            Self::Int(value) => Ok(value),
            other => Err(ComError::Type(format!("expected int, got {other:?}"))),
        }
    }

    pub fn into_dispatch(self) -> Result<ComDispatch, ComError> {
        match self {
            Self::Dispatch(dispatch) => Ok(dispatch),
            other => Err(ComError::Type(format!("expected dispatch, got {other:?}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::core::office::worker;
    use super::*;

    #[test]
    fn com_session_initializes_without_panic() {
        let _ = ComSession::new();
    }

    #[test]
    fn word_active_object_fails_gracefully_when_not_running() {
        // Local dev machines may have Word running, making this test
        // nondeterministic. If Word is running, skip instead of failing.
        if worker::app_available("Word.Application") {
            return;
        }
        let result = worker::with_app_value("Word.Application", |_| Ok(()));
        assert!(result.is_err());
        let error = result.err().expect("error").to_string();
        assert!(error.contains("Word.Application"));
    }
}
