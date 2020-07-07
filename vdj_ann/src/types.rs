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
