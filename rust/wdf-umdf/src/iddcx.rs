#![allow(non_snake_case)]
#![allow(clippy::missing_errors_doc)]

use wdf_umdf_sys::{
    IDARG_IN_ADAPTER_INIT, IDARG_IN_MONITORCREATE, IDARG_IN_SWAPCHAINSETDEVICE,
    IDARG_OUT_ADAPTER_INIT, IDARG_OUT_MONITORARRIVAL, IDARG_OUT_MONITORCREATE,
    IDARG_OUT_RELEASEANDACQUIREBUFFER, IDDCX_ADAPTER, IDDCX_MONITOR, IDDCX_SWAPCHAIN,
    IDD_CX_CLIENT_CONFIG, NTSTATUS, WDFDEVICE, WDFDEVICE_INIT, IDARG_IN_SETUP_HWCURSOR,
};

#[derive(Debug, thiserror::Error)]
pub enum IddCxError {
    #[error("{0}")]
    IddCxFunctionNotAvailable(&'static str),
    #[error("{0}")]
    CallFailed(NTSTATUS),
    #[error("{0}")]
    NtStatus(NTSTATUS),
}

impl From<IddCxError> for NTSTATUS {
    fn from(value: IddCxError) -> Self {
        #[allow(clippy::enum_glob_use)]
        use IddCxError::*;
        match value {
            IddCxFunctionNotAvailable(_) => Self::STATUS_NOT_FOUND,
            CallFailed(status) => status,
            NtStatus(n) => n,
        }
    }
}

impl From<NTSTATUS> for IddCxError {
    fn from(value: NTSTATUS) -> Self {
        IddCxError::CallFailed(value)
    }
}

impl From<i32> for IddCxError {
    fn from(val: i32) -> Self {
        IddCxError::NtStatus(NTSTATUS(val))
    }
}

macro_rules! IddCxCall {
    ($name:ident ( $($args:expr),* )) => {
        IddCxCall!(false, $name($($args),*))
    };

    ($other_is_error:expr, $name:ident ( $($args:expr),* )) => {{
        let fn_handle = {
            ::paste::paste! {
                const FN_INDEX: usize = ::wdf_umdf_sys::IDDFUNCENUM::[<$name TableIndex>].0 as usize;

                // validate that wdf function can be used
                let is_available = ::wdf_umdf_sys::IddCxIsFunctionAvailable!($name);

                if is_available {
                    // SAFETY: Only immutable accesses are done to this
                    //         The underlying array is Copy, so we call as_ptr() directly on it inside block
                    let fn_table = unsafe { ::wdf_umdf_sys::IddFunctions.as_ptr() };

                    // SAFETY: Ensured that this is present by if condition from `WdfIsFunctionAvailable!`
                    let fn_handle = unsafe {
                        fn_table.add(FN_INDEX)
                            .cast::<::wdf_umdf_sys::[<PFN_ $name:upper>]>()
                    };

                    // SAFETY: Ensured that this is present by if condition from `IddIsFunctionAvailable!`
                    let fn_handle = unsafe { fn_handle.read() };
                    // SAFETY: All available function handles are not null
                    let fn_handle = unsafe { fn_handle.unwrap_unchecked() };

                    Ok(fn_handle)
                } else {
                    Err($crate::IddCxError::IddCxFunctionNotAvailable(concat!(stringify!($name), " is not available")))
                }
            }
        };

        if let Ok(fn_handle) = fn_handle {
            // SAFETY: Pointer to globals is always immutable
            let globals = unsafe { ::wdf_umdf_sys::IddDriverGlobals };

            // SAFETY: None. User is responsible for safety and must use their own unsafe block
            let result = unsafe { fn_handle(globals, $($args),*) };

            if $crate::is_nt_error(&result, $other_is_error) {
                Err(result.into())
            } else {
                Ok(result.into())
            }
        } else {
            // SAFETY: We checked if it was Ok above, and it clearly isn't
            Err(unsafe {
                fn_handle.unwrap_err_unchecked()
            })
        }
    }};
}

/// # Safety
///
/// None. User is responsible for safety.
pub unsafe fn IddCxDeviceInitConfig(
    // in, out
    DeviceInit: &mut WDFDEVICE_INIT,
    // in
    Config: &IDD_CX_CLIENT_CONFIG,
) -> Result<NTSTATUS, IddCxError> {
    IddCxCall! {
        IddCxDeviceInitConfig(
            DeviceInit,
            Config
        )
    }
}

/// # Safety
///
/// None. User is responsible for safety.
pub unsafe fn IddCxDeviceInitialize(
    // in
    Device: WDFDEVICE,
) -> Result<NTSTATUS, IddCxError> {
    IddCxCall! {
        IddCxDeviceInitialize(
            Device
        )
    }
}

/// # Safety
///
/// None. User is responsible for safety.
pub unsafe fn IddCxAdapterInitAsync(
    // in
    pInArgs: *const IDARG_IN_ADAPTER_INIT,
    // out
    pOutArgs: *mut IDARG_OUT_ADAPTER_INIT,
) -> Result<NTSTATUS, IddCxError> {
    IddCxCall! {
        IddCxAdapterInitAsync(
            pInArgs,
            pOutArgs
        )
    }
}

/// # Safety
///
/// None. User is responsible for safety.
#[rustfmt::skip]
pub unsafe fn IddCxMonitorCreate(
    // in
    AdapterObject: IDDCX_ADAPTER,
    // in
    pInArgs: *const IDARG_IN_MONITORCREATE,
    // out
    pOutArgs: *mut IDARG_OUT_MONITORCREATE,
) -> Result<NTSTATUS, IddCxError> {
    IddCxCall!(
        IddCxMonitorCreate(
            AdapterObject,
            pInArgs,
            pOutArgs
        )
    )
}

/// # Safety
///
/// None. User is responsible for safety.
#[rustfmt::skip]
pub unsafe fn IddCxMonitorArrival(
    // in
    MonitorObject: IDDCX_MONITOR,
    // out
    pOutArgs: *mut IDARG_OUT_MONITORARRIVAL,
) -> Result<NTSTATUS, IddCxError> {
    IddCxCall!(
        IddCxMonitorArrival(
            MonitorObject,
            pOutArgs
        )
    )
}

/// # Safety
///
/// None. User is responsible for safety.
#[rustfmt::skip]
pub unsafe fn IddCxSwapChainSetDevice(
    // in
    SwapChainObject: IDDCX_SWAPCHAIN,
    // in
    pInArgs: &IDARG_IN_SWAPCHAINSETDEVICE
) -> Result<NTSTATUS, IddCxError> {
    IddCxCall!(
        true,
        IddCxSwapChainSetDevice(
            SwapChainObject,
            pInArgs
        )
    )
}

/// # Safety
///
/// None. User is responsible for safety.
#[rustfmt::skip]
pub unsafe fn IddCxSwapChainReleaseAndAcquireBuffer(
    // in
    SwapChainObject: IDDCX_SWAPCHAIN,
    // out
    pOutArgs: &mut IDARG_OUT_RELEASEANDACQUIREBUFFER
) -> Result<NTSTATUS, IddCxError> {
    IddCxCall!(
        true,
        IddCxSwapChainReleaseAndAcquireBuffer(
            SwapChainObject,
            pOutArgs
        )
    )
}

/// # Safety
///
/// None. User is responsible for safety.
#[rustfmt::skip]
pub unsafe fn IddCxSwapChainFinishedProcessingFrame(
    // in
    SwapChainObject: IDDCX_SWAPCHAIN
) -> Result<NTSTATUS, IddCxError> {
    IddCxCall!(
        true,
        IddCxSwapChainFinishedProcessingFrame(
            SwapChainObject
        )
    )
}

/// # Safety
///
/// None. User is responsible for safety.
#[rustfmt::skip]
pub unsafe fn IddCxMonitorDeparture(
    // in
    MonitorObject: IDDCX_MONITOR
) -> Result<NTSTATUS, IddCxError> {
    IddCxCall!(
        IddCxMonitorDeparture(
            MonitorObject
        )
    )
}

#[rustfmt::skip]
pub unsafe fn IddCxMonitorSetupHardwareCursor(
    MonitorObject: IDDCX_MONITOR,
    hw_cursor: IDARG_IN_SETUP_HWCURSOR 
) {
    IddCxCall!(
        IddCxMonitorSetupHardwareCursor(
            MonitorObject,
            hw_cursor
        )
    )
}
