use crate::command::argument_builder::{ArgumentBuilder, argument, command, literal};
use crate::command::argument_types::LookAt;
use crate::command::argument_types::coordinates::Coordinates;
use crate::command::argument_types::coordinates::rotation::RotationArgumentType;
use crate::command::argument_types::coordinates::vec3::Vec3ArgumentType;
use crate::command::argument_types::entity::EntityArgumentType;
use crate::command::argument_types::entity_anchor::{EntityAnchor, EntityAnchorArgumentType};
use crate::command::context::command_context::CommandContext;
use crate::command::context::command_source::CommandSource;
use crate::command::errors::command_syntax_error::CommandSyntaxError;
use crate::command::node::dispatcher::CommandDispatcher;
use crate::command::node::{CommandExecutor, CommandExecutorResult};
use crate::entity::{Entity, EntityBase};
use pumpkin_data::translation;
use pumpkin_util::PermissionLvl;
use pumpkin_util::math::vector3::Axis;
use pumpkin_util::permission::{Permission, PermissionDefault, PermissionRegistry};
use pumpkin_util::text::TextComponent;

const DESCRIPTION: &str = "Changes the rotation of an entity.";
const PERMISSION: &str = "minecraft:command.rotate";

const ARG_TARGET: &str = "target";
const ARG_ROTATION: &str = "rotation";
const ARG_FACING_LOCATION: &str = "facingLocation";
const ARG_FACING_ENTITY: &str = "facingEntity";
const ARG_FACING_ANCHOR: &str = "facingAnchor";

async fn rotate_entity_at(
    source: &CommandSource,
    entity: &Entity,
    rotation: Coordinates,
) -> Result<i32, CommandSyntaxError> {
    let rotation_vector = rotation.rotation(source);
    let y_rotation = if rotation.is_relative(Axis::Y) {
        rotation_vector.y - entity.yaw.load()
    } else {
        rotation_vector.y
    };
    let x_rotation = if rotation.is_relative(Axis::X) {
        rotation_vector.x - entity.yaw.load()
    } else {
        rotation_vector.x
    };
    entity.force_set_rotation(
        y_rotation,
        rotation.is_relative(Axis::Y),
        x_rotation,
        rotation.is_relative(Axis::X),
    );
    send_success_message(source, entity).await;
    Ok(1)
}

async fn rotate_entity_to_look(
    source: &CommandSource,
    entity: &Entity,
    look_at: LookAt,
) -> Result<i32, CommandSyntaxError> {
    look_at.perform(source, entity).await;
    send_success_message(source, entity).await;
    Ok(1)
}

/// Sends a success message for the command.
async fn send_success_message(source: &CommandSource, entity: &Entity) {
    source
        .send_feedback(
            TextComponent::translate(
                translation::COMMANDS_ROTATE_SUCCESS,
                &[entity.get_display_name().await],
            ),
            true,
        )
        .await;
}

// /rotate <target> <rotation>
struct RotateToRotationExecutor;

impl CommandExecutor for RotateToRotationExecutor {
    fn execute<'a>(&'a self, context: &'a CommandContext) -> CommandExecutorResult<'a> {
        Box::pin(async move {
            let target = EntityArgumentType::get_entity(context, ARG_TARGET).await?;
            let rotation = RotationArgumentType::get(context, ARG_ROTATION)?;
            rotate_entity_at(context.source.as_ref(), target.get_entity(), rotation).await
        })
    }
}

// /rotate <target> facing <x> <y> <z>
struct RotateFacingLocationExecutor;

impl CommandExecutor for RotateFacingLocationExecutor {
    fn execute<'a>(&'a self, context: &'a CommandContext) -> CommandExecutorResult<'a> {
        Box::pin(async move {
            let target = EntityArgumentType::get_entity(context, ARG_TARGET).await?;
            let look_at =
                LookAt::Position(Vec3ArgumentType::get_vector3(context, ARG_FACING_LOCATION)?);
            rotate_entity_to_look(context.source.as_ref(), target.get_entity(), look_at).await
        })
    }
}

// /rotate <target> facing entity <entity> [eyes|feet]
struct RotateFacingEntityExecutor;

impl CommandExecutor for RotateFacingEntityExecutor {
    fn execute<'a>(&'a self, context: &'a CommandContext) -> CommandExecutorResult<'a> {
        Box::pin(async move {
            let target = EntityArgumentType::get_entity(context, ARG_TARGET).await?;
            let look_at = LookAt::Entity {
                entity: EntityArgumentType::get_entity(context, ARG_FACING_ENTITY).await?,
                anchor: EntityAnchorArgumentType::get(context, ARG_FACING_ANCHOR)?,
            };
            rotate_entity_to_look(context.source.as_ref(), target.get_entity(), look_at).await
        })
    }
}

// /rotate <target> facing entity <entity> (no anchor - defaults to feet)
struct RotateFacingEntityNoAnchorExecutor;

impl CommandExecutor for RotateFacingEntityNoAnchorExecutor {
    fn execute<'a>(&'a self, context: &'a CommandContext) -> CommandExecutorResult<'a> {
        Box::pin(async move {
            let target = EntityArgumentType::get_entity(context, ARG_TARGET).await?;
            let look_at = LookAt::Entity {
                entity: EntityArgumentType::get_entity(context, ARG_FACING_ENTITY).await?,
                anchor: EntityAnchor::Feet,
            };
            rotate_entity_to_look(context.source.as_ref(), target.get_entity(), look_at).await
        })
    }
}

pub fn register(dispatcher: &mut CommandDispatcher, registry: &mut PermissionRegistry) {
    registry.register_permission_or_panic(Permission::new(
        PERMISSION,
        DESCRIPTION,
        PermissionDefault::Op(PermissionLvl::Two),
    ));

    dispatcher.register(
        command("rotate", DESCRIPTION).requires(PERMISSION).then(
            argument(ARG_TARGET, EntityArgumentType::Entity)
                .then(
                    argument(ARG_ROTATION, RotationArgumentType).executes(RotateToRotationExecutor),
                )
                .then(
                    literal("facing")
                        .then(
                            literal("entity").then(
                                argument(ARG_FACING_ENTITY, EntityArgumentType::Entity)
                                    .executes(RotateFacingEntityNoAnchorExecutor)
                                    .then(
                                        argument(ARG_FACING_ANCHOR, EntityAnchorArgumentType)
                                            .executes(RotateFacingEntityExecutor),
                                    ),
                            ),
                        )
                        .then(
                            argument(ARG_FACING_LOCATION, Vec3ArgumentType::Default)
                                .executes(RotateFacingLocationExecutor),
                        ),
                ),
        ),
    );
}
