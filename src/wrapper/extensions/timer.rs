use super::super::*;
use clack_extensions::timer::*;

// TODO: handle the plugins' own timers too.
pub struct WrapperTimerHandler {
    is_initialized: bool,
    bundle_check_timer: Option<TimerId>,
}

impl WrapperTimerHandler {
    pub fn new() -> Self {
        Self {
            is_initialized: false,
            bundle_check_timer: None,
        }
    }

    // Separate step to be called by on_main_thread. (needed because registering timers during init() makes the host malfunction in Bitwig)
    pub fn init(&mut self, host: &mut HostMainThreadHandle) {
        if self.is_initialized {
            return;
        }
        self.is_initialized = true;

        if let Some(timer) = host.shared().extension::<HostTimer>() {
            let res = timer.register_timer(host, 200);
            dbg!(&res);
            if let Ok(timer_id) = res {
                self.bundle_check_timer = Some(timer_id);
            }
        }
    }
}

impl<'a> PluginTimerImpl for WrapperPluginMainThread<'a> {
    fn on_timer(&mut self, _timer_id: TimerId) {
        self.check_for_new_bundles()
    }
}
