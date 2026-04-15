use crate::ClientPacket;
use crate::codec::var_int::VarInt;
use crate::ser::{NetworkWriteExt, WritingError};
use pumpkin_data::packet::clientbound::PLAY_PLAYER_LOOK_AT;
use pumpkin_macros::java_packet;
use pumpkin_util::math::vector3::Vector3;
use pumpkin_util::version::MinecraftVersion;
use std::io::Write;

/// Updates the player's rotation to look at a position or entity.
///
/// This packet informs the client about the player's new rotation
/// (usually due to a command).
#[java_packet(PLAY_PLAYER_LOOK_AT)]
pub struct CPlayerLookAt {
    /// The entity anchor to **aim** from for the rotation.
    /// - If this is 0, this represents *feet*.
    /// - If this is 1, this represents *eyes*.
    pub from_anchor: VarInt,

    pub pos: Vector3<f64>,

    /// Provides optional entity data for looking at an entity.
    /// This also tells whether such data exists.
    pub entity_data: Option<EntityData>,
}

pub struct EntityData {
    /// The entity to face towards (if any).
    id: VarInt,

    /// The entity anchor (if there is an entity specified) to **look** at for the rotation.
    /// - If this is 0, this represents *feet*.
    /// - If this is 1, this represents *eyes*.
    to_anchor: VarInt,
}

impl CPlayerLookAt {
    /// Returns a packet for updating the position where a player looks.
    ///
    /// # Parameters
    /// - `from_anchor`: The entity anchor to aim from for the rotation.
    /// - `pos`: The position to look at for the rotation.
    #[must_use]
    pub fn position(from_anchor: u8, pos: Vector3<f64>) -> Self {
        Self {
            from_anchor: from_anchor.into(),
            pos,
            entity_data: None,
        }
    }

    /// Returns a packet for updating the position where a player looks.
    ///
    /// # Parameters
    /// - `from_anchor`: The entity anchor to aim from for the rotation.
    /// - `entity`: The entity to look at for the rotation.
    /// - `to_anchor`: The entity anchor of the entity to look at for the rotation.
    /// - `anchored_pos`: The resultant position when the provided entity is given
    ///   to `to_anchor`.
    #[must_use]
    pub fn entity(from_anchor: u8, entity: i32, to_anchor: u8, anchored_pos: Vector3<f64>) -> Self {
        Self {
            from_anchor: from_anchor.into(),
            pos: anchored_pos,
            entity_data: Some(EntityData {
                id: entity.into(),
                to_anchor: to_anchor.into(),
            }),
        }
    }
}

impl ClientPacket for CPlayerLookAt {
    fn write_packet_data(
        &self,
        mut write: impl Write,
        _version: &MinecraftVersion,
    ) -> Result<(), WritingError> {
        write.write_var_int(&self.from_anchor)?;
        write.write_f64_be(self.pos.x)?;
        write.write_f64_be(self.pos.y)?;
        write.write_f64_be(self.pos.z)?;
        write.write_bool(self.entity_data.is_some())?;

        if let Some(data) = &self.entity_data {
            write.write_var_int(&data.id)?;
            write.write_var_int(&data.to_anchor)?;
        }

        Ok(())
    }
}
