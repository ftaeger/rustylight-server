use crate::device::LightState;

const NUM_STEPS: usize = 7;
const STEP_SIZE: usize = 8;
// Write buffer: 0x00 Report ID prefix at buf[0] + 64-byte payload.
// Linux usbhid_output_report() strips buf[0] before USB transmission,
// ensuring the device always receives exactly 64 bytes.
const PAYLOAD_SIZE: usize = (NUM_STEPS + 1) * STEP_SIZE; // 64 bytes (7 steps + 8-byte footer)
const WRITE_SIZE: usize = PAYLOAD_SIZE + 1; // 65 bytes

const FOOTER_BASE: usize = NUM_STEPS * STEP_SIZE; // 56

// Opcodes occupy the high nibble of the command byte; target step is the low nibble.
// Jump (0x1): display colour and advance to target step after timing out.
// KeepAlive (0x8): reset watchdog; operand = timeout in seconds (low nibble).
const OPCODE_JUMP: u8 = 0x10; // Jump, target=0 (loop back to itself)

// Footer bytes 3-5: pad field (bits 16-39 of the footer word) = 0x000FFF.
const FOOTER_PAD: [u8; 3] = [0x00, 0x0F, 0xFF];

pub fn build_report(state: &LightState) -> [u8; WRITE_SIZE] {
    let mut buf = [0u8; WRITE_SIZE];
    // buf[0] = 0x00 Report ID prefix (stripped by kernel)
    let p = &mut buf[1..]; // 64-byte payload

    // Footer: sensitivity=0, timeout=0, trigger=0, pad=0xFFF
    p[FOOTER_BASE + 3] = FOOTER_PAD[0];
    p[FOOTER_BASE + 4] = FOOTER_PAD[1];
    p[FOOTER_BASE + 5] = FOOTER_PAD[2];

    if state.on {
        if state.blink {
            build_blink(p, state);
        } else {
            write_step(
                p,
                0,
                OPCODE_JUMP,
                0,
                scale(state.r),
                scale(state.g),
                scale(state.b),
                0,
                0,
            );
        }
    } else {
        // Off: Jump with black so the device acknowledges the command.
        write_step(p, 0, OPCODE_JUMP, 0, 0, 0, 0, 0, 0);
    }

    let checksum: u16 = p[..FOOTER_BASE + 6].iter().map(|&b| b as u16).sum();
    p[FOOTER_BASE + 6] = (checksum >> 8) as u8;
    p[FOOTER_BASE + 7] = (checksum & 0xFF) as u8;

    buf
}

fn build_blink(p: &mut [u8], state: &LightState) {
    let on_ticks = ticks(state.effective_blink_on_ms());
    let off_ticks = ticks(state.effective_blink_off_ms());
    let r2 = state.r2.unwrap_or(0);
    let g2 = state.g2.unwrap_or(0);
    let b2 = state.b2.unwrap_or(0);

    if r2 > 0 || g2 > 0 || b2 > 0 {
        // Two-colour: step 0 plays once then jumps to step 1, which loops back to step 0.
        write_step(
            p,
            0,
            OPCODE_JUMP | 1,
            1,
            scale(state.r),
            scale(state.g),
            scale(state.b),
            on_ticks,
            0,
        );
        write_step(
            p,
            1,
            OPCODE_JUMP,
            1,
            scale(r2),
            scale(g2),
            scale(b2),
            off_ticks,
            0,
        );
    } else {
        // Single colour: step 0 loops with on/off duty cycle.
        write_step(
            p,
            0,
            OPCODE_JUMP,
            0,
            scale(state.r),
            scale(state.g),
            scale(state.b),
            on_ticks,
            off_ticks,
        );
    }
}

// Colour values are stored on a 0-100 scale (not raw 0-255).
fn scale(c: u8) -> u8 {
    ((c as u16 * 100) / 255) as u8
}

// Duty-cycle unit is 100 ms; clamp to at least 1 so blink is always visible.
fn ticks(ms: u16) -> u8 {
    (ms / 100).clamp(1, 255) as u8
}

#[allow(clippy::too_many_arguments)]
fn write_step(
    p: &mut [u8],
    step: usize,
    opcode_target: u8, // (opcode << 4) | target_step
    repeat: u8,
    r: u8,
    g: u8,
    b: u8,
    on_time: u8,
    off_time: u8,
) {
    let base = step * STEP_SIZE;
    p[base] = opcode_target;
    p[base + 1] = repeat;
    p[base + 2] = r;
    p[base + 3] = g;
    p[base + 4] = b;
    p[base + 5] = on_time;
    p[base + 6] = off_time;
    p[base + 7] = 0; // flags: update=0, ringtone=0, volume=0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::LightState;

    fn payload(buf: &[u8; WRITE_SIZE]) -> &[u8] {
        &buf[1..]
    }

    fn checksum_ok(p: &[u8]) -> bool {
        let expected: u16 = p[..FOOTER_BASE + 6].iter().map(|&b| b as u16).sum();
        let actual = ((p[FOOTER_BASE + 6] as u16) << 8) | p[FOOTER_BASE + 7] as u16;
        expected == actual
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
    fn off_state_has_correct_layout_and_checksum() {
        let buf = build_report(&LightState::default());
        let p = payload(&buf);
        // Step 0: Jump opcode (0x10), black (r=g=b=0), no timing
        assert_eq!(p[0], 0x10); // Jump, target=0
        assert_eq!(p[1], 0x00); // repeat=0
        assert_eq!(p[2], 0x00); // r
        assert_eq!(p[3], 0x00); // g
        assert_eq!(p[4], 0x00); // b
        assert_eq!(p[5], 0x00); // on_time
        assert_eq!(p[6], 0x00); // off_time
                                // Steps 1-6 are zero
        assert_eq!(&p[8..FOOTER_BASE], &[0u8; FOOTER_BASE - 8]);
        // Footer pad bytes
        assert_eq!(p[FOOTER_BASE + 3], 0x00);
        assert_eq!(p[FOOTER_BASE + 4], 0x0F);
        assert_eq!(p[FOOTER_BASE + 5], 0xFF);
        // Checksum: 0x10 (step0) + 0x0F + 0xFF (footer pad) = 0x011E = 286
        assert_eq!(p[FOOTER_BASE + 6], 0x01);
        assert_eq!(p[FOOTER_BASE + 7], 0x1E);
        assert!(checksum_ok(p));
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
        // Step layout: [opcode_target, repeat, r, g, b, on_time, off_time, flags]
        assert_eq!(p[0], 0x10); // Jump, target=0
        assert_eq!(p[1], 0x00); // repeat=0
        assert_eq!(p[2], 100); // r: scale(255) = 100
        assert_eq!(p[3], 0); // g
        assert_eq!(p[4], 0); // b
        assert_eq!(p[5], 0); // on_time=0 (steady)
        assert_eq!(p[6], 0); // off_time=0
        assert_eq!(p[7], 0); // flags
        assert_eq!(&p[8..FOOTER_BASE], &[0u8; FOOTER_BASE - 8]);
        // Checksum: 0x10 + 0x64 (step0) + 0x0F + 0xFF (footer pad) = 0x0182 = 386
        assert_eq!(p[FOOTER_BASE + 6], 0x01);
        assert_eq!(p[FOOTER_BASE + 7], 0x82);
        assert!(checksum_ok(p));
    }

    #[test]
    fn scale_function_maps_255_to_100_and_0_to_0() {
        assert_eq!(scale(255), 100);
        assert_eq!(scale(0), 0);
        assert_eq!(scale(128), 50); // int(128 * 100 / 255) = 50
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
        assert_eq!(p[0], 0x10); // Jump, target=0 (loop)
        assert_eq!(p[1], 0x00); // repeat=0
        assert_eq!(p[2], 0); // r
        assert_eq!(p[3], 100); // g: scale(255) = 100
        assert_eq!(p[4], 0); // b
        assert_eq!(p[5], 5); // on_time: 500/100 = 5
        assert_eq!(p[6], 3); // off_time: 300/100 = 3
        assert!(checksum_ok(p));
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
        // Step 0: Jump to step 1, play once, primary colour
        assert_eq!(p[0], 0x11); // Jump, target=1
        assert_eq!(p[1], 0x01); // repeat=1
        assert_eq!(p[2], 100); // r: scale(255)
        assert_eq!(p[3], 0); // g
        assert_eq!(p[4], 0); // b
        assert_eq!(p[5], 5); // on_time: 500/100
        assert_eq!(p[6], 0); // off_time=0 (no gap between steps)
                             // Step 1: Jump to step 0, play once, secondary colour
        assert_eq!(p[8], 0x10); // Jump, target=0
        assert_eq!(p[9], 0x01); // repeat=1
        assert_eq!(p[10], 0); // r2
        assert_eq!(p[11], 0); // g2
        assert_eq!(p[12], 100); // b2: scale(255)
        assert_eq!(p[13], 5); // on_time: 500/100
        assert_eq!(p[14], 0); // off_time=0
        assert!(checksum_ok(p));
    }
}
