use crate::ClientPacket;
use crate::ser::{NetworkWriteExt, WritingError};
use pumpkin_data::packet::clientbound::PLAY_PLAYER_ROTATION;
use pumpkin_macros::java_packet;
use pumpkin_util::version::MinecraftVersion;
use std::io::Write;

/// Updates the player's rotation to look according to a new
/// yaw and pitch.
///
/// This packet informs the client about the player's new rotation.
#[java_packet(PLAY_PLAYER_ROTATION)]
pub struct CPlayerRotation {
    /// The new yaw.
    pub yaw: f32,
    /// Whether the sent yaw is relative.
    pub is_relative_yaw: bool,
    /// The new pitch.
    pub pitch: f32,
    /// Whether the sent pitch is relative.
    pub is_relative_pitch: bool,
}

impl CPlayerRotation {
    #[must_use]
    pub const fn new(yaw: f32, is_relative_yaw: bool, pitch: f32, is_relative_pitch: bool) -> Self {
        Self {
            yaw,
            is_relative_yaw,
            pitch,
            is_relative_pitch,
        }
    }
}

impl ClientPacket for CPlayerRotation {
    fn write_packet_data(
        &self,
        mut write: impl Write,
        _version: &MinecraftVersion,
    ) -> Result<(), WritingError> {
        write.write_f32_be(self.yaw)?;
        write.write_bool(self.is_relative_yaw)?;
        write.write_f32_be(self.pitch)?;
        write.write_bool(self.is_relative_pitch)?;

        Ok(())
    }
}
