use pumpkin_data::game_rules::{GameRule, GameRuleRegistry, GameRuleValue};
use pumpkin_util::permission::{Permission, PermissionDefault, PermissionRegistry};
use pumpkin_util::PermissionLvl;

use crate::TextComponent;

use crate::command::argument_builder::{argument, command, literal, ArgumentBuilder};
use crate::command::argument_types::core::bool::BoolArgumentType;
use crate::command::argument_types::core::integer::IntegerArgumentType;
use crate::command::context::command_context::CommandContext;
use crate::command::node::{CommandExecutor, CommandExecutorResult};
use crate::command::node::dispatcher::CommandDispatcher;

const DESCRIPTION: &str = "Sets or queries a game rule value.";
const PERMISSION: &str = "minecraft:command.gamerule";

const ARG_NAME: &str = "value";

struct QueryExecutor(GameRule);

impl CommandExecutor for QueryExecutor {
    fn execute<'a>(&'a self, context: &'a CommandContext) -> CommandExecutorResult<'a> {
        Box::pin(async move {
            let key = TextComponent::text(self.0.to_string());
            let level_info = context.server().level_info.load();
            let game_rule = level_info.game_rules.get(&self.0);
            let game_rule_i32_value = match game_rule {
                GameRuleValue::Int(value) => {
                    (*value).clamp(i32::MIN as i64, i32::MAX as i64) as i32
                }
                GameRuleValue::Bool(value) => *value as i32,
            };
            let value = TextComponent::text(game_rule.to_string());
            drop(level_info);

            context.source
                .send_feedback(TextComponent::translate(
                    "commands.gamerule.query",
                    [key, value],
                ), false)
                .await;

            Ok(game_rule_i32_value)
        })
    }
}

struct SetExecutor(GameRule);

impl CommandExecutor for SetExecutor {
    fn execute<'a>(&'a self, context: &'a CommandContext) -> CommandExecutorResult<'a> {
        Box::pin(async move {
            let key = TextComponent::text(self.0.to_string());
            let current_info = context.server().level_info.load();

            let mut new_info = (**current_info).clone();

            let mut output_value = String::new();
            let mut result_i32: i32;

            let raw_value = new_info.game_rules.get_mut(&self.0);

            match raw_value {
                GameRuleValue::Int(value) => {
                    let arg_value = IntegerArgumentType::get(context, ARG_NAME)?;
                    *value = arg_value as i64;
                    output_value = arg_value.to_string();
                    result_i32 = arg_value;
                }
                GameRuleValue::Bool(value) => {
                    let arg_value = BoolArgumentType::get(context, ARG_NAME)?;
                    *value = arg_value;
                    output_value = arg_value.to_string();
                    result_i32 = *value as i32;
                }
            }

            context.server().level_info.store(std::sync::Arc::new(new_info));

            let value_component = TextComponent::text(output_value);
            context.source
                .send_feedback(TextComponent::translate(
                    "commands.gamerule.set",
                    [key, value_component],
                ), true)
                .await;

            Ok(result_i32)
        })
    }
}

pub fn register(dispatcher: &mut CommandDispatcher, registry: &mut PermissionRegistry) {
    registry.register_permission_or_panic(Permission::new(
        PERMISSION,
        DESCRIPTION,
        PermissionDefault::Op(PermissionLvl::Two),
    ));

    let mut node =
        command("difficulty", DESCRIPTION)
            .requires(PERMISSION);

    // Add each game rule to this node as an argument.
    let rule_registry = GameRuleRegistry::default();
    for rule in GameRule::all() {
        let argument = match rule_registry.get(rule) {
            GameRuleValue::Int(_) => argument(ARG_NAME, IntegerArgumentType::any()),
            GameRuleValue::Bool(_) => argument(ARG_NAME, BoolArgumentType)
        };
        node = node
            .then(
                literal(rule.to_string())
                    .then(
                        argument.executes(SetExecutor(rule.clone())),
                    )
                    .executes(QueryExecutor(rule.clone()))
            )
    }

    dispatcher.register(node);
}
