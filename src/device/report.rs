use crate::device::LightState;

const NUM_STEPS: usize = 7;
const STEP_SIZE: usize = 8;
const PAYLOAD_SIZE: usize = (NUM_STEPS + 1) * STEP_SIZE; // 64 bytes (7 steps + 8-byte footer)
                                                         // Write buffer: 0x00 Report ID prefix at buf[0] + 64-byte payload.
                                                         // Linux usbhid_output_report() strips buf[0] before USB transmission,
                                                         // ensuring the device always receives exactly 64 bytes.
const WRITE_SIZE: usize = PAYLOAD_SIZE + 1; // 65 bytes

const FOOTER_BASE: usize = NUM_STEPS * STEP_SIZE; // 56

pub fn build_report(state: &LightState) -> [u8; WRITE_SIZE] {
    let mut buf = [0u8; WRITE_SIZE];
    // buf[0] = 0x00 Report ID prefix (stripped by kernel)
    let p = &mut buf[1..]; // 64-byte payload

    p[FOOTER_BASE] = 0x04;
    p[FOOTER_BASE + 1] = 0x04;
    p[FOOTER_BASE + 2] = 0x55;
    p[FOOTER_BASE + 3] = 0xFF;
    p[FOOTER_BASE + 4] = 0xFF;
    p[FOOTER_BASE + 5] = 0xFF;

    if state.on {
        if state.blink {
            build_blink(p, state);
        } else {
            write_step(p, 0, 0, 0, state.r, state.g, state.b, 0xFF, 0);
        }
    }

    let checksum: u16 = p[..FOOTER_BASE + 6].iter().map(|&b| b as u16).sum();
    p[FOOTER_BASE + 6] = (checksum >> 8) as u8;
    p[FOOTER_BASE + 7] = (checksum & 0xFF) as u8;

    buf
}

fn build_blink(p: &mut [u8], state: &LightState) {
    let on_ticks = (state.effective_blink_on_ms() / 10).min(254) as u8;
    let off_ticks = (state.effective_blink_off_ms() / 10).min(255) as u8;
    let r2 = state.r2.unwrap_or(0);
    let g2 = state.g2.unwrap_or(0);
    let b2 = state.b2.unwrap_or(0);

    if r2 > 0 || g2 > 0 || b2 > 0 {
        // Two-color: step 0 → step 1 → step 0 → ...
        write_step(p, 0, 1, 0, state.r, state.g, state.b, on_ticks, 0);
        write_step(p, 1, 0, 0, r2, g2, b2, off_ticks, 0);
    } else {
        write_step(p, 0, 0, 0, state.r, state.g, state.b, on_ticks, off_ticks);
    }
}

#[allow(clippy::too_many_arguments)]
fn write_step(
    p: &mut [u8],
    step: usize,
    next: u8,
    repeat: u8,
    r: u8,
    g: u8,
    b: u8,
    on: u8,
    off: u8,
) {
    let base = step * STEP_SIZE;
    p[base] = next;
    p[base + 1] = repeat;
    p[base + 2] = r;
    p[base + 3] = g;
    p[base + 4] = b;
    p[base + 5] = on;
    p[base + 6] = off;
    p[base + 7] = 0; // audio (silent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::LightState;

    fn payload(buf: &[u8; WRITE_SIZE]) -> &[u8] {
        &buf[1..]
    }

    #[test]
    fn report_is_65_bytes() {
        let buf = build_report(&LightState::default());
        assert_eq!(buf.len(), 65);
    }

    #[test]
    fn report_id_byte_is_zero() {
        let buf = build_report(&LightState::default());
        assert_eq!(buf[0], 0x00);
    }

    #[test]
    fn off_state_has_footer_and_correct_checksum() {
        let buf = build_report(&LightState::default());
        let p = payload(&buf);
        assert_eq!(&p[..FOOTER_BASE], &[0u8; FOOTER_BASE]);
        assert_eq!(p[FOOTER_BASE], 0x04);
        assert_eq!(p[FOOTER_BASE + 1], 0x04);
        assert_eq!(p[FOOTER_BASE + 2], 0x55);
        assert_eq!(p[FOOTER_BASE + 3], 0xFF);
        assert_eq!(p[FOOTER_BASE + 4], 0xFF);
        assert_eq!(p[FOOTER_BASE + 5], 0xFF);
        // 4+4+85+255+255+255 = 858 = 0x035A
        assert_eq!(p[FOOTER_BASE + 6], 0x03);
        assert_eq!(p[FOOTER_BASE + 7], 0x5A);
    }

    #[test]
    fn steady_red_step0_and_checksum() {
        let state = LightState {
            on: true,
            r: 255,
            g: 0,
            b: 0,
            ..Default::default()
        };
        let buf = build_report(&state);
        let p = payload(&buf);
        assert_eq!(p[0], 0); // next
        assert_eq!(p[1], 0); // repeat
        assert_eq!(p[2], 255); // r
        assert_eq!(p[3], 0); // g
        assert_eq!(p[4], 0); // b
        assert_eq!(p[5], 0xFF); // on (steady)
        assert_eq!(p[6], 0); // off
        assert_eq!(p[7], 0); // audio
        assert_eq!(&p[8..FOOTER_BASE], &[0u8; FOOTER_BASE - 8]);
        // 255(r)+255(on)+[footer 858] = 1368 = 0x0558
        assert_eq!(p[FOOTER_BASE + 6], 0x05);
        assert_eq!(p[FOOTER_BASE + 7], 0x58);
    }

    #[test]
    fn blink_single_color_timing() {
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
        let buf = build_report(&state);
        let p = payload(&buf);
        assert_eq!(p[0], 0); // next (loops to self)
        assert_eq!(p[2], 0); // r
        assert_eq!(p[3], 255); // g
        assert_eq!(p[4], 0); // b
        assert_eq!(p[5], 50); // on_ticks: 500/10 = 50
        assert_eq!(p[6], 30); // off_ticks: 300/10 = 30
    }

    #[test]
    fn blink_two_colors_step0_and_step1() {
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
        let buf = build_report(&state);
        let p = payload(&buf);
        // Step 0: advance to step 1 after on_ticks
        assert_eq!(p[0], 1); // next → step 1
        assert_eq!(p[2], 255); // r
        assert_eq!(p[4], 0); // b
        assert_eq!(p[5], 50); // on_ticks
        assert_eq!(p[6], 0); // no off gap in step 0
                             // Step 1: return to step 0 after off_ticks
        assert_eq!(p[8], 0); // next → step 0
        assert_eq!(p[10], 0); // r2
        assert_eq!(p[11], 0); // g2
        assert_eq!(p[12], 255); // b2
        assert_eq!(p[13], 50); // off_ticks used as on time for step 1
    }
}
