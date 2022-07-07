use poise::serenity_prelude as serenity;
use serenity::{GatewayIntents, GuildId};
use std::env;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, State, Error>;
type FrameworkContext<'a> = poise::FrameworkContext<'a, State, Error>;

struct State {
    home_guild: GuildId,
}

/// Pong!
#[poise::command(slash_command)]
async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Pong!").await?;

    Ok(())
}

async fn event_handler(
    _ctx: &serenity::Context,
    event: &poise::Event<'_>,
    _framework: FrameworkContext<'_>,
    _state: &State,
) -> Result<(), Error> {
    if let poise::Event::Ready { .. } = event {
        println!("Bot now running.");
    }

    Ok(())
}

/// Update or delete application commands
#[poise::command(slash_command, owners_only)]
async fn register(ctx: Context<'_>) -> Result<(), Error> {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}

#[tokio::main]
async fn main() {
    let token = env::var("GCT_DISCORD_TOKEN").expect("GCT_DISCORD_TOKEN was not specified.");

    let home_guild = GuildId(
        env::var("GCT_GUILD_ID")
            .expect("Expected GCT_GUILD_ID in environment")
            .parse()
            .expect("GCT_GUILD_ID must be an integer"),
    );

    let intents = GatewayIntents::GUILDS | GatewayIntents::GUILD_MESSAGES;

    let framework = poise::Framework::build()
        .options(poise::FrameworkOptions {
            commands: vec![register(), ping()],
            listener: |a, b, c, d| Box::pin(event_handler(a, b, c, d)),
            ..Default::default()
        })
        .token(token)
        .intents(intents)
        .user_data_setup(move |_, _ready, _framework| {
            Box::pin(async move { Ok(State { home_guild }) })
        });

    framework.run().await.unwrap();
}
