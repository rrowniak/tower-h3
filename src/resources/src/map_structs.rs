#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Format {
    ROE, // 14
    AB,  // 21
    SOD, // 28
    //CHR   = 0x1d, // 29 Heroes Chronicles, presumably - identical to SoD, untested
    HOTA, // 32
    WOG,  // 51
    VCMI,
}

impl Format {
    pub fn from(code: u32) -> Option<Self> {
        match code {
            0x0e => Some(Self::ROE),
            0x15 => Some(Self::AB),
            0x1c => Some(Self::SOD),
            0x20 => Some(Self::HOTA),
            0x33 => Some(Self::WOG),
            0x64 => Some(Self::VCMI),
            _ => None,
        }
    }

    pub fn nice_str(&self) -> &'static str {
        match *self {
            Self::ROE => "Restoration of Erathia",
            Self::AB => "Armageddon blade",
            Self::SOD => "Shadof of death",
            Self::HOTA => "Horn of the abbys",
            Self::WOG => "Wake of gods",
            Self::VCMI => "VCMI",
        }
    }
}
