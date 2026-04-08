use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use std::sync::atomic::{AtomicBool, Ordering};

static RELOAD_REQUESTED: AtomicBool = AtomicBool::new(false);
static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);

pub struct SignalFlags {
    pub reload_requested: &'static AtomicBool,
    pub shutdown_requested: &'static AtomicBool,
}

pub fn install_signal_handlers() -> SignalFlags {
    unsafe {
        signal::sigaction(
            Signal::SIGHUP,
            &signal::SigAction::new(
                signal::SigHandler::Handler(sighup_handler),
                signal::SaFlags::SA_RESTART,
                signal::SigSet::empty(),
            ),
        )
        .expect("Failed to install SIGHUP handler");

        signal::sigaction(
            Signal::SIGTERM,
            &signal::SigAction::new(
                signal::SigHandler::Handler(sigterm_handler),
                signal::SaFlags::SA_RESTART,
                signal::SigSet::empty(),
            ),
        )
        .expect("Failed to install SIGTERM handler");
    }

    SignalFlags {
        reload_requested: &RELOAD_REQUESTED,
        shutdown_requested: &SHUTDOWN_REQUESTED,
    }
}

extern "C" fn sighup_handler(_: nix::libc::c_int) {
    RELOAD_REQUESTED.store(true, Ordering::SeqCst);
}

extern "C" fn sigterm_handler(_: nix::libc::c_int) {
    SHUTDOWN_REQUESTED.store(true, Ordering::SeqCst);
}

pub fn send_signal(pid: u32, sig: Signal) -> Result<(), String> {
    #[allow(clippy::cast_possible_wrap)]
    signal::kill(Pid::from_raw(pid as i32), sig)
        .map_err(|e| format!("Failed to send signal to PID {pid}: {e}"))
}
