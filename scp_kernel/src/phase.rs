#[derive(Debug, PartialEq)]
pub enum Phase {
    Prepare,
    Confirm,
    Externalize,
    Num,
}

impl core::fmt::Display for Phase {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let phase = match self {
            Self::Prepare => "Prepare",
            Self::Confirm => "Confirm",
            Self::Externalize => "Externalize",
            Self::Num => "Num",
        };

        f.write_str(phase)
    }
}
