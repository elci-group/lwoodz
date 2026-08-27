//! 3form-backed replacement for legacy tracing macros.
//!
//! Diagnostic events are rendered to stderr so JSON and other machine-readable
//! command output on stdout remains uncontaminated.

use form3::{DevRenderer, Event, MachineRenderer, Mode, Renderer, Signal, UiRenderer};

#[doc(hidden)]
pub fn emit(level: &str, message: impl Into<String>) {
    let message = message.into();
    let event = match level {
        "error" => Event::Error {
            text: message,
            cause: None,
        },
        "warn" => Event::Warning {
            text: message,
            cause: None,
        },
        "debug" => Event::Info {
            text: format!("debug: {message}"),
        },
        _ => Event::Info { text: message },
    };
    let signal = Signal::new(event).component("lwoodz");
    let rendered = match Mode::auto() {
        Mode::Ui => UiRenderer.render(&signal),
        Mode::Dev => DevRenderer.render(&signal),
        Mode::Machine => MachineRenderer.render(&signal),
    };
    eprintln!("{rendered}");
}

macro_rules! telemetry_info {
    ($message:literal $(,)?) => { $crate::telemetry::emit("info", $message) };
    ($($tokens:tt)*) => { $crate::telemetry::emit("info", stringify!($($tokens)*)) };
}

macro_rules! telemetry_warn {
    ($message:literal $(,)?) => { $crate::telemetry::emit("warn", $message) };
    ($($tokens:tt)*) => { $crate::telemetry::emit("warn", stringify!($($tokens)*)) };
}

macro_rules! telemetry_error {
    ($message:literal $(,)?) => { $crate::telemetry::emit("error", $message) };
    ($($tokens:tt)*) => { $crate::telemetry::emit("error", stringify!($($tokens)*)) };
}

macro_rules! telemetry_debug {
    ($message:literal $(,)?) => { $crate::telemetry::emit("debug", $message) };
    ($($tokens:tt)*) => { $crate::telemetry::emit("debug", stringify!($($tokens)*)) };
}

pub(crate) use telemetry_debug as debug;
pub(crate) use telemetry_error as error;
pub(crate) use telemetry_info as info;
pub(crate) use telemetry_warn as warn;
