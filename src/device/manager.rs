use crate::device::{models::ModelVariant, report::build_report, SharedState};
use hidapi::HidApi;
use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

pub fn spawn_usb_manager(shared: Arc<Mutex<SharedState>>) {
    thread::spawn(move || {
        run_loop(shared);
    });
}

fn run_loop(shared: Arc<Mutex<SharedState>>) {
    let mut device: Option<hidapi::HidDevice> = None;

    loop {
        if device.is_none() {
            device = try_connect(&shared);
        }

        if let Some(ref dev) = device {
            let (state, _dirty) = {
                let mut s = shared.lock().unwrap();
                let dirty = s.state_dirty;
                s.state_dirty = false;
                (s.light_state.clone(), dirty)
            };

            let report = build_report(&state);
            tracing::trace!(
                "HID write: on={} r={} g={} b={} | payload[0..8]={:02x?}",
                state.on,
                state.r,
                state.g,
                state.b,
                &report[1..9],
            );
            match dev.write(&report) {
                Ok(n) => tracing::trace!("HID write OK: {n} bytes written"),
                Err(e) => {
                    tracing::info!("Busylight disconnected: {e}");
                    device = None;
                    shared.lock().unwrap().connected = false;
                }
            }
        }

        thread::sleep(Duration::from_secs(2));
    }
}

fn try_connect(shared: &Arc<Mutex<SharedState>>) -> Option<hidapi::HidDevice> {
    let api = HidApi::new().ok()?;
    for dev_info in api.device_list() {
        if let Some(variant) =
            ModelVariant::from_vid_pid(dev_info.vendor_id(), dev_info.product_id())
        {
            match dev_info.open_device(&api) {
                Ok(dev) => {
                    let mut s = shared.lock().unwrap();
                    s.connected = true;
                    s.state_dirty = true;
                    tracing::info!(
                        "Busylight connected: {:?} ({:#06x}:{:#06x})",
                        variant,
                        dev_info.vendor_id(),
                        dev_info.product_id()
                    );
                    return Some(dev);
                }
                Err(e) => {
                    tracing::warn!("found Busylight but failed to open: {e}");
                }
            }
        }
    }
    None
}
