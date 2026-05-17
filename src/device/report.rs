use crate::device::LightState;

const STEP_SIZE: usize = 7;
const NUM_STEPS: usize = 8;
// 1 byte Report ID (0x00) + 64 bytes payload; hidapi requires the Report ID
// as the first byte even for devices with a single unnumbered report.
const REPORT_SIZE: usize = 65;
const DATA_OFFSET: usize = 1;
const KEEPALIVE_IDX: usize = 64;
const KEEPALIVE_SECS: u8 = 5;

pub fn build_report(state: &LightState) -> [u8; REPORT_SIZE] {
    let mut report = [0u8; REPORT_SIZE];
    // report[0] stays 0x00 (Report ID)

    if !state.on {
        report[KEEPALIVE_IDX] = KEEPALIVE_SECS;
        return report;
    }

    if state.blink {
        let on_ticks = state.effective_blink_on_ms() / 10;
        let off_ticks = state.effective_blink_off_ms() / 10;

        let r2 = state.r2.unwrap_or(0);
        let g2 = state.g2.unwrap_or(0);
        let b2 = state.b2.unwrap_or(0);
        let two_color = r2 > 0 || g2 > 0 || b2 > 0;

        if two_color {
            write_step(&mut report, 0, state.r, state.g, state.b, on_ticks, 0);
            write_step(&mut report, 1, r2, g2, b2, off_ticks, 0);
        } else {
            write_step(
                &mut report,
                0,
                state.r,
                state.g,
                state.b,
                on_ticks,
                off_ticks,
            );
        }
    } else {
        write_step(&mut report, 0, state.r, state.g, state.b, 0xFFFF, 0);
    }

    report[DATA_OFFSET + NUM_STEPS * STEP_SIZE] = 0x00;
    report[KEEPALIVE_IDX] = KEEPALIVE_SECS;
    report
}

fn write_step(
    report: &mut [u8; REPORT_SIZE],
    step: usize,
    r: u8,
    g: u8,
    b: u8,
    on_ticks: u16,
    off_ticks: u16,
) {
    let base = DATA_OFFSET + step * STEP_SIZE;
    report[base] = r;
    report[base + 1] = g;
    report[base + 2] = b;
    report[base + 3] = (on_ticks >> 8) as u8;
    report[base + 4] = (on_ticks & 0xFF) as u8;
    report[base + 5] = (off_ticks >> 8) as u8;
    report[base + 6] = (off_ticks & 0xFF) as u8;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::LightState;

    #[test]
    fn report_is_65_bytes() {
        let state = LightState::default();
        let report = build_report(&state);
        assert_eq!(report.len(), 65);
    }

    #[test]
    fn report_first_byte_is_report_id_zero() {
        let state = LightState {
            on: true,
            r: 255,
            g: 0,
            b: 0,
            blink: false,
            ..Default::default()
        };
        let report = build_report(&state);
        assert_eq!(report[0], 0x00);
    }

    #[test]
    fn steady_red_sets_step0_color() {
        let state = LightState {
            on: true,
            r: 255,
            g: 0,
            b: 0,
            blink: false,
            ..Default::default()
        };
        let report = build_report(&state);
        // byte 0 = report ID; data starts at byte 1
        assert_eq!(report[1], 255);
        assert_eq!(report[2], 0);
        assert_eq!(report[3], 0);
        assert_eq!(report[4], 0xFF);
        assert_eq!(report[5], 0xFF);
        assert_eq!(report[6], 0x00);
        assert_eq!(report[7], 0x00);
    }

    #[test]
    fn off_state_all_zeros_except_keepalive() {
        let state = LightState {
            on: false,
            ..Default::default()
        };
        let report = build_report(&state);
        assert_eq!(report[0], 0);
        assert_eq!(report[1], 0);
        assert_eq!(report[2], 0);
        assert_eq!(report[64], 0x05);
    }

    #[test]
    fn blink_color_to_off_sets_on_off_timing() {
        let state = LightState {
            on: true,
            r: 0,
            g: 255,
            b: 0,
            blink: true,
            blink_on_ms: Some(500),
            blink_off_ms: Some(300),
            ..Default::default()
        };
        let report = build_report(&state);
        // step 0 timing starts at byte 4 (1 report ID + 3 color bytes)
        assert_eq!(report[4], 0x00);
        assert_eq!(report[5], 50);
        assert_eq!(report[6], 0x00);
        assert_eq!(report[7], 30);
    }

    #[test]
    fn blink_two_colors_sets_step1_color() {
        let state = LightState {
            on: true,
            r: 255,
            g: 0,
            b: 0,
            blink: true,
            blink_on_ms: Some(500),
            blink_off_ms: Some(500),
            r2: Some(0),
            g2: Some(0),
            b2: Some(255),
        };
        let report = build_report(&state);
        // step 0: bytes 1-7; step 1: bytes 8-14
        assert_eq!(report[1], 255); // r
        assert_eq!(report[3], 0); // b
        assert_eq!(report[8], 0); // r2
        assert_eq!(report[9], 0); // g2
        assert_eq!(report[10], 255); // b2
    }
}
