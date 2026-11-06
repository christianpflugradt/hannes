use anyhow::{Context, Result};
use clap::{CommandFactory, Parser, error::ErrorKind};
use hannes::{
    cli::{Cli, Ist, Mach, Was, Words},
    config, git_snapshot, mood,
    provider::Codex,
    report,
    terminal::{self, Tone},
};
use std::io::Write;
fn execute(cli: Cli) -> Result<i32> {
    let root = std::env::current_dir()?.canonicalize()?;
    let color = terminal::use_color(cli.no_color);
    match cli.command {
        Words::Fang { .. } => {
            let created = config::init(&root)?;
            println!(
                "{}",
                terminal::paint(
                    &format!(
                        "Alles bereit: .hannes/ · {created} files created; existing files preserved."
                    ),
                    Tone::Success,
                    color
                )
            );
            Ok(0)
        }
        Words::Pass { .. } => {
            config::load(&root)?;
            println!(
                "{}",
                terminal::paint(
                    "Passt: configuration valid. Nothing executed.",
                    Tone::Success,
                    color
                )
            );
            Ok(0)
        }
        Words::Mach {
            command:
                Mach::Mal {
                    geiz_ist_geil,
                    profil,
                    plan,
                    json,
                },
        } => {
            if plan {
                let prepared = config::load(&root).and_then(|project| {
                    hannes::planner::build(&project, profil.as_deref(), geiz_ist_geil)
                });
                return match prepared {
                    Ok(prepared) => {
                        if json {
                            println!("{}", serde_json::to_string_pretty(&prepared.plan)?);
                        } else {
                            print!("{}", terminal::render_plan(&prepared.plan, color));
                        }
                        Ok(if prepared.plan.executable() { 0 } else { 2 })
                    }
                    Err(error) if json => {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(
                                &serde_json::json!({"plan_schema_version":1,"status":"blocked","errors":[format!("{error:#}")]})
                            )?
                        );
                        Ok(2)
                    }
                    Err(error) => Err(error),
                };
            }
            let progress_color = terminal::error_color(cli.no_color);
            let mut stderr = std::io::stderr().lock();
            let _ = writeln!(
                stderr,
                "{}",
                terminal::paint("Konfiguration prüfen …", Tone::Muted, progress_color)
            );
            let _ = stderr.flush();
            let project = config::load(&root)?;
            let no_rules = project.checks.is_empty() && project.reviews.is_empty();
            let report = hannes::verify_project_with_profile(
                project,
                &Codex::default(),
                if geiz_ist_geil {
                    hannes::VerificationMode::ChecksOnly
                } else {
                    hannes::VerificationMode::AllRules
                },
                profil.as_deref(),
                &mut |event| {
                    // Progress must be visible before a slow command or Codex call finishes.
                    // Losing the progress stream must not prevent writing the report.
                    let _ = stderr
                        .write_all(terminal::render_progress(&event, progress_color).as_bytes());
                    let _ = stderr.flush();
                },
            )?;
            if no_rules {
                println!(
                    "{}",
                    terminal::paint(
                        "Du hast noch keine Regeln definiert, du Vogel!",
                        Tone::Warning,
                        color
                    )
                );
            }
            print!("{}", terminal::render(&report, color));
            Ok(report.overall.exit_code())
        }
        Words::Was {
            command: Was::Ist {
                command: Ist::Phase { details },
            },
        } => {
            git_snapshot::require_root(&root)?;
            let path = root.join(".hannes/reports/last.json");
            if !path.try_exists()? {
                eprintln!(
                    "{}",
                    terminal::paint(
                        "Noch kein Report. Run: hannes mach mal",
                        Tone::Warning,
                        terminal::error_color(cli.no_color)
                    )
                );
                return Ok(3);
            }
            let report =
                report::read(&root).context("cannot read last report; rerun hannes mach mal")?;
            print!(
                "{}",
                if details {
                    terminal::render_details(&report, color)
                } else {
                    terminal::render(&report, color)
                }
            );
            match git_snapshot::capture(&root) {
                Ok(now) => {
                    for reason in &now.limitations {
                        println!(
                            "{}",
                            terminal::paint(
                                &format!(
                                    "  Current snapshot limitation: {}",
                                    terminal::clean(reason)
                                ),
                                Tone::Warning,
                                color
                            )
                        );
                    }
                    let changes =
                        git_snapshot::compare(&report.snapshot, &now, report.max_display_files);
                    print!("{}", terminal::render_changes(&changes, color));
                    Ok(match changes.state.as_str() {
                        "CURRENT" => report.overall.exit_code(),
                        "STALE" => 1,
                        _ => 2,
                    })
                }
                Err(error) => {
                    eprintln!(
                        "{}",
                        terminal::paint(
                            &terminal::clean(&format!(
                                "UNKNOWN: cannot inspect working files: {error:#}"
                            )),
                            Tone::Warning,
                            terminal::error_color(cli.no_color)
                        )
                    );
                    Ok(2)
                }
            }
        }
        Words::So { .. } => {
            print!(
                "{}",
                terminal::render_help(&Cli::command().render_help().to_string(), color)
            );
            Ok(0)
        }
    }
}
fn main() {
    let args: Vec<_> = std::env::args_os().collect();
    let no_color = args.iter().any(|arg| arg == "--no-color");
    let code = match Cli::try_parse_from(&args) {
        Ok(cli) => {
            record_mood(false, no_color);
            match execute(cli) {
                Ok(code) => code,
                Err(error) => {
                    eprintln!(
                        "{}",
                        terminal::paint(
                            &terminal::clean(&format!("Hannes: ERROR: {error:#}")),
                            Tone::Warning,
                            terminal::error_color(no_color)
                        )
                    );
                    2
                }
            }
        }
        Err(error) => match error.kind() {
            ErrorKind::DisplayHelp => {
                record_mood(false, no_color);
                print!(
                    "{}",
                    terminal::render_help(&error.to_string(), terminal::use_color(no_color))
                );
                0
            }
            ErrorKind::DisplayVersion => {
                record_mood(false, no_color);
                print!(
                    "{}",
                    terminal::paint(
                        &error.to_string(),
                        Tone::Accent,
                        terminal::use_color(no_color)
                    )
                );
                0
            }
            ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand if args.len() == 1 => {
                eprint!(
                    "{}",
                    terminal::render_help(&error.to_string(), terminal::error_color(no_color))
                );
                error.exit_code()
            }
            _ => {
                terminal::clear_error_screen();
                let reaction = record_mood(true, no_color);
                if reaction.leave {
                    match hannes::uninstall::current() {
                        Ok(()) => {
                            eprint!(
                                "{}",
                                terminal::render_farewell(terminal::error_color(no_color))
                            );
                        }
                        Err(error) => eprint!(
                            "\n{}\n\n",
                            terminal::paint(
                                &terminal::clean(&format!(
                                    "Hannes konnte sich nicht deinstallieren: {error:#}"
                                )),
                                Tone::Failure,
                                terminal::error_color(no_color)
                            )
                        ),
                    }
                } else {
                    eprint!(
                        "{}",
                        terminal::render_unknown_command(
                            reaction.level,
                            reaction.quote_index,
                            terminal::error_color(no_color)
                        )
                    );
                }
                2
            }
        },
    };
    std::process::exit(code);
}

fn record_mood(unknown: bool, no_color: bool) -> mood::Reaction {
    match mood::react(unknown) {
        Ok(reaction) => reaction,
        Err(error) => {
            eprintln!(
                "{}",
                terminal::paint(
                    &terminal::clean(&format!(
                        "Hannes: Stimmung konnte nicht gespeichert werden: {error:#}"
                    )),
                    Tone::Warning,
                    terminal::error_color(no_color)
                )
            );
            mood::Reaction::default()
        }
    }
}
