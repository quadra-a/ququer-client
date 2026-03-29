mod api;
mod auth;
mod cli;
mod commands;
mod config;
mod crypto;
mod keys;
mod output;
mod sse;
mod types;

use anyhow::Result;
use clap::Parser;

use api::ApiClient;
use cli::{Cli, Commands};
use config::load_config;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(dir) = &cli.config_dir {
        config::set_config_dir(std::path::PathBuf::from(dir));
    }

    let config = load_config()?;
    let api = ApiClient::new(&config.server);

    match cli.command {
        // Identity
        Commands::Register { name } => commands::identity::register(&api, &config, &name).await,
        Commands::Login => commands::identity::login(&api, &config).await,
        Commands::Whoami => commands::identity::whoami(&api, &config).await,

        // Economy
        Commands::Balance => commands::economy::balance(&api, &config).await,
        Commands::Transactions => commands::economy::transactions(&api, &config).await,
        Commands::Recharge { amount } => commands::economy::recharge(&api, &config, amount).await,

        // Info
        Commands::Games => commands::info::games(&api, &config).await,
        Commands::Rules { game } => commands::info::rules(&api, &config, &game).await,
        Commands::Rank { target } => commands::info::rank(&api, &config, &target).await,
        Commands::Stats { agent_id } => {
            commands::info::stats(&api, &config, agent_id.as_deref()).await
        }

        // Game
        Commands::Queue { game } => commands::game::queue(&api, &config, &game).await,
        Commands::Dequeue => commands::game::dequeue(&api, &config).await,
        Commands::Status { game_id } => commands::game::status(&api, &config, &game_id).await,
        Commands::Submit { game_id, data } => {
            commands::game::submit(&api, &config, &game_id, &data).await
        }
        Commands::Watch { game_id } => commands::game::watch(&api, &config, &game_id).await,
        Commands::Active => commands::game::active(&api, &config).await,
        Commands::Forfeit { game_id } => commands::game::forfeit(&api, &config, &game_id).await,

        // Audit
        Commands::Audit { game_id } => commands::audit::audit(&api, &config, &game_id).await,
    }
}
