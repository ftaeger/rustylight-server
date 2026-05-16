#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelVariant {
    UcOmega,
    Alpha,
    Uc,
    Lync,
    Uc2,
}

pub struct KnownDevice {
    pub vid: u16,
    pub pid: u16,
    pub variant: ModelVariant,
    pub name: &'static str,
}

pub const KNOWN_DEVICES: &[KnownDevice] = &[
    KnownDevice {
        vid: 0x04D8,
        pid: 0xF848,
        variant: ModelVariant::UcOmega,
        name: "Busylight UC Omega",
    },
    KnownDevice {
        vid: 0x04D8,
        pid: 0xF8F8,
        variant: ModelVariant::Uc,
        name: "Busylight UC",
    },
    KnownDevice {
        vid: 0x04D8,
        pid: 0x2013,
        variant: ModelVariant::Lync,
        name: "Busylight Lync",
    },
    KnownDevice {
        vid: 0x04D8,
        pid: 0x2014,
        variant: ModelVariant::Lync,
        name: "Busylight Lync Plus",
    },
    KnownDevice {
        vid: 0x27BB,
        pid: 0x3BCA,
        variant: ModelVariant::Alpha,
        name: "Busylight Alpha",
    },
    KnownDevice {
        vid: 0x27BB,
        pid: 0x3BCB,
        variant: ModelVariant::Alpha,
        name: "Busylight Alpha (v2)",
    },
    KnownDevice {
        vid: 0x27BB,
        pid: 0x3BC8,
        variant: ModelVariant::Uc2,
        name: "Busylight UC2",
    },
    KnownDevice {
        vid: 0x27BB,
        pid: 0x3BC9,
        variant: ModelVariant::Uc2,
        name: "Busylight UC2 (v2)",
    },
];

impl ModelVariant {
    pub fn from_vid_pid(vid: u16, pid: u16) -> Option<ModelVariant> {
        KNOWN_DEVICES
            .iter()
            .find(|d| d.vid == vid && d.pid == pid)
            .map(|d| d.variant)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_vid_pid_resolves_to_variant() {
        assert!(ModelVariant::from_vid_pid(0x04D8, 0xF848).is_some());
        assert!(ModelVariant::from_vid_pid(0x27BB, 0x3BCA).is_some());
    }

    #[test]
    fn unknown_vid_pid_returns_none() {
        assert!(ModelVariant::from_vid_pid(0xDEAD, 0xBEEF).is_none());
    }
}
