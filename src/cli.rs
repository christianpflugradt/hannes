use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "hannes",
    version,
    before_help = include_str!("banner.txt").trim_end(),
    about = "Hannes schaut nach dem Rechten.",
    disable_help_subcommand = true,
    help_template = include_str!("help.txt")
)]
pub struct Cli {
    #[arg(long, global = true, help = "Farben deaktivieren")]
    pub no_color: bool,
    #[command(subcommand)]
    pub command: Words,
}
#[derive(Debug, Subcommand)]
pub enum Words {
    Fang {
        #[command(subcommand)]
        command: Fang,
    },
    Pass {
        #[command(subcommand)]
        command: Pass,
    },
    Mach {
        #[command(subcommand)]
        command: Mach,
    },
    Was {
        #[command(subcommand)]
        command: Was,
    },
    So {
        #[command(subcommand)]
        command: So,
    },
}
#[derive(Debug, Subcommand)]
pub enum Fang {
    An,
}
#[derive(Debug, Subcommand)]
pub enum Pass {
    Auf,
}
#[derive(Debug, Subcommand)]
pub enum Mach {
    Was {
        #[arg(long, help = "Nur deterministische Checks ausführen, ohne LLM-Aufruf")]
        geiz_ist_geil: bool,
        #[arg(
            long,
            help = "Regeln an diesem Execution Point ausführen (Standard: pre-push)"
        )]
        at: Option<String>,
        #[arg(
            long,
            help = "Ausführungsplan anzeigen, ohne Regeln oder Provider zu starten"
        )]
        plan: bool,
        #[arg(
            long,
            requires = "plan",
            help = "Plan oder Planfehler als JSON ausgeben"
        )]
        json: bool,
        #[arg(long, hide = true, requires = "git_hook_path", conflicts_with = "plan")]
        git_hook: Option<String>,
        #[arg(long, hide = true, requires = "git_hook")]
        git_hook_path: Option<std::path::PathBuf>,
        #[arg(last = true, hide = true, requires = "git_hook")]
        hook_arguments: Vec<std::ffi::OsString>,
    },
}
#[derive(Debug, Subcommand)]
pub enum Was {
    Ist {
        #[command(subcommand)]
        command: Ist,
    },
}
#[derive(Debug, Subcommand)]
pub enum Ist {
    Phase {
        #[arg(long, help = "Vollständige Befunde und gespeicherte Ausgaben anzeigen")]
        details: bool,
    },
}
#[derive(Debug, Subcommand)]
pub enum So {
    Hilf {
        #[command(subcommand)]
        command: Hilf,
    },
}
#[derive(Debug, Subcommand)]
pub enum Hilf {
    Mir {
        #[command(subcommand)]
        command: Mir,
    },
}
#[derive(Debug, Subcommand)]
pub enum Mir {
    Doch,
}
