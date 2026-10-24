//! Data of `crate::climate` that the project file holds (see `docs/architecture.md`).

use serde::{Deserialize, Serialize};

/// Climate settings of an enclosure.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Climate {
    pub installation: Installation,
    pub material: Material,
    /// Highest and lowest ambient temperature (°C).
    pub ambient_max: f32,
    pub ambient_min: f32,
    /// Allowed inside temperature range (°C).
    pub inside_max: f32,
    pub inside_min: f32,
    /// Power loss not covered by the parts (W), e.g. devices without data.
    pub extra_loss: f32,
}

/// How the enclosure is installed: decides which of its faces give off heat.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Installation {
    /// Single enclosure, free-standing on all sides.
    FreeStanding,
    /// Single enclosure for wall mounting.
    #[default]
    WallMounted,
    /// First or last enclosure of a suite, free-standing.
    EndFreeStanding,
    /// First or last enclosure of a suite, wall mounted.
    EndWallMounted,
    /// Enclosure within a suite, free-standing.
    MiddleFreeStanding,
    /// Enclosure within a suite, wall mounted.
    MiddleWallMounted,
    /// Enclosure within a suite, wall mounted, roof covered.
    MiddleWallMountedCovered,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Material {
    #[default]
    SheetSteel,
    StainlessSteel,
    Aluminium,
    Plastic,
}
