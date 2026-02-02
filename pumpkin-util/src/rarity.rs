use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::text::color::NamedColor;

#[derive(Debug, PartialEq, Eq)]
pub struct ParseRarityError;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rarity {
    Common = 0,
    Uncommon = 1,
    Rare = 2,
    Epic = 3,
}

impl Rarity {
    #[must_use]
    pub const fn to_str(&self) -> &'static str {
        match self {
            Self::Common => "common",
            Self::Uncommon => "uncommon",
            Self::Rare => "rare",
            Self::Epic => "epic",
        }
    }

    #[must_use]
    /// Returns the formatting [`NamedColor`] of this [`Rarity`].
    /// For example, uncommon items' names are *yellow* and rare items' names are *aqua*.
    pub const fn color(&self) -> NamedColor {
        match self {
            Self::Common => NamedColor::White,
            Self::Uncommon => NamedColor::Yellow,
            Self::Rare => NamedColor::Aqua,
            Self::Epic => NamedColor::LightPurple,
        }
    }
}

impl From<Rarity> for u8 {
    fn from(rarity: Rarity) -> Self {
        rarity as Self
    }
}

impl TryFrom<u8> for Rarity {
    type Error = ParseRarityError;

    fn try_from(n: u8) -> Result<Self, Self::Error> {
        match n {
            0 => Ok(Self::Common),
            1 => Ok(Self::Uncommon),
            2 => Ok(Self::Rare),
            3 => Ok(Self::Epic),
            _ => Err(ParseRarityError),
        }
    }
}

impl FromStr for Rarity {
    type Err = ParseRarityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "common" => Ok(Self::Common),
            "uncommon" => Ok(Self::Uncommon),
            "rare" => Ok(Self::Rare),
            "epic" => Ok(Self::Epic),
            _ => Err(ParseRarityError),
        }
    }
}
