use std::{ffi::CStr, sync::Mutex};

use tensorrt_sys::DestructorVEntry;

#[cfg(feature = "tracing")]
pub static DEFAULT_LOGGER: &Logger = &TRACING_LOGGER;
#[cfg(not(feature = "tracing"))]
pub static DEFAULT_LOGGER: &Logger = &STDERR_LOGGER;

#[cfg(feature = "tracing")]
pub static TRACING_LOGGER: Logger = Logger::new_static(&log_tracing);

#[cfg(feature = "tracing")]
fn log_tracing(severity: Severity, msg: &str) {
    match severity {
        Severity::kINTERNAL_ERROR => tracing::error!("{}", msg),
        Severity::kERROR => tracing::error!("{}", msg),
        Severity::kWARNING => tracing::warn!("{}", msg),
        Severity::kINFO => tracing::info!("{}", msg),
        Severity::kVERBOSE => tracing::debug!("{}", msg),
    };
}

pub static STDERR_LOGGER: Logger = Logger::new_static(&log_stderr);

fn log_stderr(severity: Severity, msg: &str) {
    if severity as i32 <= Severity::kINFO as i32 {
        let level = match severity {
            Severity::kINTERNAL_ERROR => "\x1b[31minternal_error",
            Severity::kERROR => "\x1b[91merror",
            Severity::kWARNING => "\x1b[93mwarn",
            Severity::kINFO => "\x1b[92minfo",
            Severity::kVERBOSE => "\x1b[35mdebug",
        };
        eprintln!("tensorrt ({level}\x1b[0m) {msg}")
    }
}

#[repr(C)]
pub struct Logger {
    vtable: &'static tensorrt_sys::ILoggerVTable,
    cb: LoggerCb,
}

enum LoggerCb {
    Static(&'static (dyn Fn(Severity, &str) + Sync)),
    #[allow(clippy::type_complexity)]
    Owned(Box<dyn Fn(Severity, &str) + Send + 'static>),
}

impl LoggerCb {
    pub fn run(&self, severity: Severity, msg: &str) {
        match self {
            Self::Static(f) => f(severity, msg),
            Self::Owned(f) => f(severity, msg),
        }
    }
}

impl Logger {
    #[inline]
    const fn new_with_cb(cb: LoggerCb) -> Self {
        unsafe extern "C" fn log(
            this: *mut tensorrt_sys::ILogger,
            severity: Severity,
            msg: *const core::ffi::c_char,
        ) {
            let logger = unsafe { &mut *(this as *mut _ as *mut Logger) };
            let msg = CStr::from_ptr(msg)
                .to_str()
                .unwrap_or("<<non-utf8 message>>");
            logger.cb.run(severity, msg);
        }

        static LOGGER_VTABLE: tensorrt_sys::ILoggerVTable = tensorrt_sys::ILoggerVTable {
            log,
            destruct: DestructorVEntry::new(),
        };

        Self {
            vtable: &LOGGER_VTABLE,
            cb,
        }
    }

    #[inline]
    pub fn new_owned(cb: impl Fn(Severity, &str) + Send + 'static) -> Self {
        Self::new_with_cb(LoggerCb::Owned(Box::new(cb)))
    }

    #[inline]
    pub const fn new_static(cb: &'static (dyn Fn(Severity, &str) + Sync)) -> Self {
        Self::new_with_cb(LoggerCb::Static(cb))
    }

    #[inline]
    pub fn new_stderr(max_level: Severity) -> Self {
        Logger::new_owned(move |level, msg| {
            let max_level = max_level as i32;
            if (level as i32) <= max_level {
                eprintln!("{level:?}: {msg:?}")
            }
        })
    }

    #[inline]
    pub(crate) fn as_ffi(&self) -> *mut tensorrt_sys::ILogger {
        self as *const _ as *mut tensorrt_sys::ILogger
    }
}

pub type Severity = tensorrt_sys::ILogger_Severity;
