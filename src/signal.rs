use std::process::exit;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct Signal {
    running: Arc<AtomicBool>,
}

impl Signal {
    pub fn setup() -> Self {
        let running = Arc::new(AtomicBool::new(true));
        ctrlc::set_handler({
            let r = Arc::clone(&running);
            move || {
                r.store(false, Ordering::SeqCst);
            }
        })
        .expect("Error setting Ctrl-C handler");
        Signal { running }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn exit(&self) -> ! {
        exit(-1)
    }
}
