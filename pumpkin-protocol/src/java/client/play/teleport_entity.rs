use std::io::Write;

use pumpkin_data::packet::clientbound::PLAY_TELEPORT_ENTITY;
use pumpkin_macros::java_packet;
use pumpkin_util::version::MinecraftVersion;

use crate::{
    ClientPacket, PositionFlag, PositionMoveRotation, VarInt, WritingError, ser::NetworkWriteExt,
};

/// Only used when teleporting a player's vehicle, this packet is sent to the player.
#[java_packet(PLAY_TELEPORT_ENTITY)]
pub struct CTeleportEntity<'a> {
    pub entity_id: VarInt,
    pub change: PositionMoveRotation,
    pub relatives: &'a [PositionFlag],
    pub on_ground: bool,
}

impl<'a> CTeleportEntity<'a> {
    #[must_use]
    pub const fn new(
        entity_id: VarInt,
        change: PositionMoveRotation,
        relatives: &'a [PositionFlag],
        on_ground: bool,
    ) -> Self {
        Self {
            entity_id,
            change,
            relatives,
            on_ground,
        }
    }
}

// TODO: Do we need a custom impl?
impl ClientPacket for CTeleportEntity<'_> {
    fn write_packet_data(
        &self,
        write: impl Write,
        _version: &MinecraftVersion,
    ) -> Result<(), WritingError> {
        let mut write = write;

        write.write_var_int(&self.entity_id)?;
        write.write_f64_be(self.change.position.x)?;
        write.write_f64_be(self.change.position.y)?;
        write.write_f64_be(self.change.position.z)?;
        write.write_f64_be(self.change.delta.x)?;
        write.write_f64_be(self.change.delta.y)?;
        write.write_f64_be(self.change.delta.z)?;
        write.write_f32_be(self.change.yaw)?;
        write.write_f32_be(self.change.pitch)?;
        // not sure about that
        write.write_i32_be(PositionFlag::get_bitfield(self.relatives))?;
        write.write_bool(self.on_ground)
    }
}
