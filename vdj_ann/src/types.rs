use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIter, EnumString};

/// All the possible heavy and light chains
#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    EnumString,
    Display,
    EnumIter,
    Hash,
)]
pub enum VdjChain {
    IGH,
    IGK,
    IGL,
    TRA,
    TRB,
    TRD,
    TRG,
}

/// Different segments or regions in a full-length receptor transcript
#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    EnumString,
    Display,
    EnumIter,
    Hash,
)]
pub enum VdjRegion {
    #[strum(to_string = "5'UTR")]
    UTR, // 5′ untranslated region (5′ UTR)
    #[strum(to_string = "L-REGION+V-REGION")]
    V, // Variable region
    #[strum(to_string = "D-REGION")]
    D, // Diversity region
    #[strum(to_string = "J-REGION")]
    J, // Joining region
    #[strum(to_string = "C-REGION")]
    C, // Constant region
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn vdj_region_from_str() {
        assert_eq!(VdjRegion::from_str("5'UTR"), Ok(VdjRegion::UTR));
        assert_eq!(VdjRegion::from_str("L-REGION+V-REGION"), Ok(VdjRegion::V));
        assert_eq!(VdjRegion::from_str("D-REGION"), Ok(VdjRegion::D));
        assert_eq!(VdjRegion::from_str("J-REGION"), Ok(VdjRegion::J));
        assert_eq!(VdjRegion::from_str("C-REGION"), Ok(VdjRegion::C));
    }
}
