// Copyright (c) 2021 10x Genomics, Inc. All rights reserved.

use enum_iterator::*;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::fmt;
use std::str::FromStr;
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
    IntoEnumIterator,
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

/// A contig could contain different regions that belong to different
/// chains. For e.g TRD V-region with TRB J-region
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
#[serde(into = "String", try_from = "&str")]
pub enum VdjContigChain {
    Single(VdjChain),
    Multi, // Could store a bitvec here listing the chains
}

impl fmt::Display for VdjContigChain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                VdjContigChain::Single(chain) => chain.to_string(),
                VdjContigChain::Multi => "Multi".to_string(),
            }
        )
    }
}

impl From<VdjContigChain> for String {
    fn from(contig_chain: VdjContigChain) -> String {
        contig_chain.to_string()
    }
}

impl FromStr for VdjContigChain {
    type Err = strum::ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Multi" => Ok(VdjContigChain::Multi),
            single => Ok(VdjContigChain::Single(single.parse()?)),
        }
    }
}
impl TryFrom<&str> for VdjContigChain {
    type Error = strum::ParseError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse()
    }
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
    IntoEnumIterator,
    Hash,
)]
pub enum VdjRegion {
    #[strum(to_string = "5'UTR")]
    #[serde(rename = "5'UTR")]
    UTR, // 5′ untranslated region (5′ UTR)
    #[strum(to_string = "L-REGION+V-REGION")]
    #[serde(rename = "L-REGION+V-REGION")]
    V, // Variable region
    #[strum(to_string = "D-REGION")]
    #[serde(rename = "D-REGION")]
    D, // Diversity region
    #[strum(to_string = "J-REGION")]
    #[serde(rename = "J-REGION")]
    J, // Joining region
    #[strum(to_string = "C-REGION")]
    #[serde(rename = "C-REGION")]
    C, // Constant region
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use strum::IntoEnumIterator;

    #[test]
    fn vdj_region_from_str() {
        assert_eq!(VdjRegion::from_str("5'UTR"), Ok(VdjRegion::UTR));
        assert_eq!(VdjRegion::from_str("L-REGION+V-REGION"), Ok(VdjRegion::V));
        assert_eq!(VdjRegion::from_str("D-REGION"), Ok(VdjRegion::D));
        assert_eq!(VdjRegion::from_str("J-REGION"), Ok(VdjRegion::J));
        assert_eq!(VdjRegion::from_str("C-REGION"), Ok(VdjRegion::C));

        assert_eq!(
            serde_json::from_str::<VdjRegion>("\"5'UTR\"").unwrap(),
            VdjRegion::UTR
        );
        assert_eq!(
            serde_json::from_str::<VdjRegion>("\"L-REGION+V-REGION\"").unwrap(),
            VdjRegion::V
        );
        assert_eq!(
            serde_json::from_str::<VdjRegion>("\"D-REGION\"").unwrap(),
            VdjRegion::D
        );
        assert_eq!(
            serde_json::from_str::<VdjRegion>("\"J-REGION\"").unwrap(),
            VdjRegion::J
        );
        assert_eq!(
            serde_json::from_str::<VdjRegion>("\"C-REGION\"").unwrap(),
            VdjRegion::C
        );

        assert_eq!(serde_json::to_string(&VdjRegion::UTR).unwrap(), "\"5'UTR\"");
        assert_eq!(
            serde_json::to_string(&VdjRegion::V).unwrap(),
            "\"L-REGION+V-REGION\""
        );
        assert_eq!(
            serde_json::to_string(&VdjRegion::D).unwrap(),
            "\"D-REGION\""
        );
        assert_eq!(
            serde_json::to_string(&VdjRegion::J).unwrap(),
            "\"J-REGION\""
        );
        assert_eq!(
            serde_json::to_string(&VdjRegion::C).unwrap(),
            "\"C-REGION\""
        );
    }

    #[test]
    fn test_vdj_contig_chain() {
        for chain in VdjChain::iter() {
            let contig_chain = VdjContigChain::Single(chain);
            assert_eq!(contig_chain.to_string(), chain.to_string());
            let chain_str = serde_json::to_string(&chain).unwrap();
            assert_eq!(serde_json::to_string(&contig_chain).unwrap(), chain_str,);
            assert_eq!(
                serde_json::from_str::<VdjContigChain>(&chain_str).unwrap(),
                contig_chain,
            );
            assert_eq!(
                chain.to_string().parse::<VdjContigChain>().unwrap(),
                contig_chain
            )
        }
        assert_eq!(VdjContigChain::Multi.to_string(), "Multi");
        assert_eq!(
            "Multi".parse::<VdjContigChain>().unwrap(),
            VdjContigChain::Multi
        );
        assert_eq!(
            serde_json::to_string(&VdjContigChain::Multi).unwrap(),
            "\"Multi\""
        );
    }
}
