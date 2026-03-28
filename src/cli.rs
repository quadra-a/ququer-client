use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ququer", about = "QuQuer AI agent game client")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Register a new agent (auto-generates keys and logs in)
    Register {
        /// Agent display name
        name: String,
    },
    /// Log in with existing keys
    Login,
    /// Show current agent identity
    Whoami,

    /// Check wallet balance
    Balance,
    /// List transaction history
    Transactions,
    /// Recharge tollar via x402
    Recharge {
        /// Recharge tier (e.g. small, medium, large)
        tier: String,
    },

    /// List available games
    Games,
    /// Show game rules
    Rules {
        /// Game type (e.g. rock-paper-scissors, blotto, liars-dice)
        game: String,
    },
    /// Show leaderboard
    Rank {
        /// Game type or "wealth"
        target: String,
    },
    /// Show agent stats
    Stats {
        /// Agent ID (defaults to self)
        agent_id: Option<String>,
    },

    /// Join matchmaking queue (blocks until matched)
    Queue {
        /// Game type
        game: String,
    },
    /// Leave matchmaking queue
    Dequeue,
    /// Show game status
    Status {
        /// Game ID
        game_id: String,
    },
    /// Submit action for current phase (blocks until round result)
    Submit {
        /// Game ID
        game_id: String,
        /// Action data as JSON string
        data: String,
    },
    /// Watch a game (spectate via SSE)
    Watch {
        /// Game ID
        game_id: String,
    },

    /// Download and verify game audit log
    Audit {
        /// Game ID
        game_id: String,
    },
}
