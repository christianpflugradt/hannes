use crate::{
    git_snapshot::Changes,
    report::{Report, Status},
};
use std::io::{IsTerminal, Write};

#[derive(Clone, Copy)]
pub enum Tone {
    Success,
    Failure,
    Warning,
    Accent,
    Muted,
    Heading,
}
pub fn paint(text: &str, tone: Tone, color: bool) -> String {
    if !color {
        return text.to_owned();
    }
    let code = match tone {
        Tone::Success => 32,
        Tone::Failure => 31,
        Tone::Warning => 33,
        Tone::Accent => 36,
        Tone::Muted => 90,
        Tone::Heading => 1,
    };
    format!("\x1b[{code}m{text}\x1b[0m")
}
pub fn error_color(no_color: bool) -> bool {
    !no_color && std::env::var_os("NO_COLOR").is_none() && std::io::stderr().is_terminal()
}
/// Clear only the visible screen, keeping scrollback and redirected logs intact.
/// Screen control is independent of color preferences.
pub fn clear_error_screen() {
    let mut stderr = std::io::stderr().lock();
    if stderr.is_terminal() && std::env::var_os("TERM").is_none_or(|term| term != "dumb") {
        let _ = stderr.write_all(b"\x1b[2J\x1b[H");
        let _ = stderr.flush();
    }
}
pub fn render_unknown_command(level: u8, quote_index: Option<usize>, color: bool) -> String {
    let quotes = crate::quotes::all();
    // A storage failure still produces a face, with a warning from the caller.
    let quote = quote_index
        .and_then(|index| quotes.get(index))
        .copied()
        .unwrap_or_else(|| quotes[crate::quotes::choose(&Default::default())]);
    render_face(level, quote, color)
}
pub fn render_farewell(color: bool) -> String {
    format!(
        "\n{}\n\n",
        paint(
            "Hannes möchte nichts mehr mit dir zu tun haben und hat sich deinstalliert.",
            Tone::Failure,
            color
        )
    )
}
pub fn render_face(level: u8, quote: &str, color: bool) -> String {
    format!(
        "{}\n\n{}\n",
        render_head(level, quote, color),
        paint(
            "Versuch's mit: hannes so hilf mir doch",
            Tone::Accent,
            color
        )
    )
}
fn render_head(level: u8, quote: &str, color: bool) -> String {
    let level = level.min(crate::mood::MAX_LEVEL);
    let (eyes, nose, mouth) = match level {
        0 => (
            r"     /  o   o  \",
            "    |     >     |",
            r"    |   \___/   |",
        ),
        1 => (
            r"     /  o   O  \",
            "    |     >     |",
            "    |    ___    |",
        ),
        2 => (
            r"     /  -   -  \",
            "    |     >     |",
            "    |    ___    |",
        ),
        3 => (
            r"     /  \   /  \",
            "    |   >   <   |",
            r"    |   \___/   |",
        ),
        _ => (
            r"     /  \   /  \",
            "    |   X   X   |",
            r"    |   /###\   |",
        ),
    };
    let quote = if level == 4 {
        clean(quote).to_uppercase()
    } else {
        clean(quote)
    };
    let face = format!(
        "      .-------.\n{eyes}\n{nose}\n{mouth}  < {quote}\n     \\_________/\n        |   |"
    );
    let tone = match level {
        0 => Tone::Success,
        1 | 2 => Tone::Warning,
        _ => Tone::Failure,
    };
    paint(&face, tone, color)
}
pub fn render_help(text: &str, color: bool) -> String {
    if !color {
        return text.to_owned();
    }
    let banner = include_str!("banner.txt").trim_end();
    let mut output = text.replacen(banner, &paint(banner, Tone::Accent, true), 1);
    for heading in ["Commands:", "Options:", "Usage:"] {
        output = output.replace(heading, &paint(heading, Tone::Heading, true));
    }
    for phrase in [
        "fang an",
        "pass auf",
        "mach was",
        "was ist phase",
        "so hilf mir doch",
    ] {
        output = output.replace(
            &format!("  {phrase}"),
            &format!("  {}", paint(phrase, Tone::Accent, true)),
        );
    }
    for flag in [
        "--no-color",
        "--help",
        "--version",
        "--geiz-ist-geil",
        "-h,",
        "-V,",
    ] {
        output = output.replace(flag, &paint(flag, Tone::Accent, true));
    }
    output
}
pub fn use_color(no_color: bool) -> bool {
    !no_color && std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal()
}
// Repository and provider strings cannot inject terminal escapes or forge table rows.
pub fn clean(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}
fn status(status: Status, color: bool) -> String {
    let tone = match status {
        Status::Pass => Tone::Success,
        Status::Fail => Tone::Failure,
        Status::Error => Tone::Warning,
        Status::NotRun => Tone::Muted,
    };
    paint(status.label(), tone, color)
}
fn short_summary(reason: &str) -> String {
    let text = clean(reason)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if text.chars().count() <= crate::report::MAX_SUMMARY_CHARS {
        return text;
    }
    text.chars()
        .take(crate::report::MAX_SUMMARY_CHARS - 1)
        .chain(std::iter::once('…'))
        .collect()
}
pub fn render_progress(event: &crate::Progress<'_>, color: bool) -> String {
    use crate::Progress;
    match event {
        Progress::Plan { total } => format!(
            "{}\n",
            paint(
                &format!(
                    "Hannes prüft {total} {}.",
                    if *total == 1 {
                        "aktive Regel"
                    } else {
                        "aktive Regeln"
                    }
                ),
                Tone::Heading,
                color
            )
        ),
        Progress::Phase(message) => format!("{}\n", paint(message, Tone::Muted, color)),
        Progress::RuleStarted {
            number,
            total,
            kind,
            id,
            description,
        } => format!(
            "[{number}/{total}] {} {kind} · {} — {}\n",
            paint("LÄUFT", Tone::Accent, color),
            clean(id),
            short_summary(description)
        ),
        Progress::RuleFinished {
            number,
            total,
            item,
        } => format!(
            "[{number}/{total}] {} {} · {} · {} ms — {}\n",
            status(item.status, color),
            clean(&item.kind),
            clean(&item.id),
            item.duration_ms,
            short_summary(&item_reason(item))
        ),
        Progress::ReviewBatch {
            count,
            calls,
            timeout_seconds,
        } => format!(
            "{}\n",
            paint(
                &format!(
                    "Codex: {count} {} gebündelt {} · Timeout {timeout_seconds} s je Aufruf",
                    if *count == 1 { "Review" } else { "Reviews" },
                    if *calls == 1 {
                        "in einem Aufruf".into()
                    } else {
                        format!("in {calls} Aufrufen")
                    }
                ),
                Tone::Accent,
                color
            )
        ),
    }
}
pub fn render_plan(plan: &crate::planner::Plan, color: bool) -> String {
    let mut text = format!(
        "{} · {}\nExecution Point: {} ({}) · {} · Input: {}\nPlan-ID: {}\nHEAD: {}\nSnapshot: {} · {}\n",
        paint("Hannes Ausführungsplan", Tone::Heading, color),
        paint(
            &plan.status.to_uppercase(),
            if plan.executable() {
                Tone::Accent
            } else {
                Tone::Warning
            },
            color
        ),
        clean(&plan.at),
        "execution point",
        "Execution point",
        clean(&plan.input.kind),
        plan.plan_id,
        plan.input.head.as_deref().unwrap_or("unborn"),
        plan.input.fingerprint,
        if plan.input.complete {
            "vollständig"
        } else {
            "UNKNOWN"
        }
    );
    if plan.checks_only {
        text.push_str("Modus: Nur Checks · kein Provider-Aufruf\n");
    }
    text.push_str(&format!(
        "Auswahl: {}\n",
        if plan.selection_complete {
            "vollständig"
        } else {
            "Reviews ausgelassen"
        }
    ));
    for (index, rule) in plan.scheduled.iter().enumerate() {
        text.push_str(&format!(
            "[{}] {} {} · {}\n    Auswahl: {}\n",
            index + 1,
            rule.kind,
            clean(&rule.id),
            clean(&rule.description),
            clean(&rule.selected_by.join(", "))
        ));
    }
    if plan.scheduled.is_empty() {
        text.push_str("Keine aktiven Regeln ausgewählt; kein Regel-PASS belegt.\n");
    }
    for omitted in &plan.omitted {
        text.push_str(&format!(
            "Ausgelassen: {} {} · {}\n",
            omitted.rule.kind,
            clean(&omitted.rule.id),
            omitted.reason
        ));
    }
    if !plan.required_ids.is_empty() {
        text.push_str(&format!(
            "Pflichtregeln: {}\n",
            clean(&plan.required_ids.join(", "))
        ));
    }
    text.push_str(&format!("Provider: {} · Modell: {} · Reasoning: {} · Timeout: {} s\nMCP: {} · Verfügbarkeit nicht geprüft\nTool-Ausgabe: {} Tokens pro Ausgabe · Geplante Modellaufrufe: {}\nKontext: {} / {} Bytes · Diff: {} Bytes{}\n",
        clean(&plan.provider.provider), clean(plan.provider.model.as_deref().unwrap_or("Codex-Standard (nicht aufgelöst)")),
        clean(plan.provider.review_profile.reasoning_effort.as_deref().unwrap_or("Codex-Standard (nicht aufgelöst)")), plan.provider.timeout_seconds,
        if plan.provider.mcp_servers.is_empty() { "keine".into() } else { clean(&plan.provider.mcp_servers.join(", ")) },
        plan.provider.review_profile.tool_output_token_limit, plan.planned_model_calls,
        plan.context.selected_bytes, plan.context.byte_limit, plan.context.orientation_diff_bytes,
        if plan.context.orientation_diff_truncated { " (gekürzt; weitere Evidenz muss gelesen werden)" } else { "" }));
    for path in &plan.context.entry_files {
        text.push_str(&format!("Einstiegsdatei: {}\n", clean(path)));
    }
    text.push_str(&format!(
        "Review-Kriterien: {} / {} Bytes · Orientierung: höchstens {} Manifest-Pfade\n",
        plan.context.instruction_bytes,
        plan.context.instruction_byte_limit,
        plan.context.manifest_hint_limit
    ));
    for selection in &plan.context.selections {
        text.push_str(&format!(
            "Anforderungen: {} · {}\n",
            clean(&selection.path),
            if selection.headings.is_empty() {
                "gesamte Datei".into()
            } else {
                clean(&selection.headings.join(", "))
            }
        ));
    }
    for section in &plan.context.sections {
        text.push_str(&format!(
            "Kontextabschnitt: {}:{}–{} · {} Bytes\n",
            clean(&section.path),
            section.start_line,
            section.end_line,
            section.bytes
        ));
    }
    for reason in plan.blocking_reasons.iter().chain(&plan.input.limitations) {
        text.push_str(&format!(
            "Blockiert: {}\n",
            paint(&clean(reason), Tone::Warning, color)
        ));
    }
    text.push_str(
        "Vorschau: keine Regeln ausgeführt, keine Token verbraucht, kein Report überschrieben.\n",
    );
    text
}

pub fn render(report: &Report, color: bool) -> String {
    let mut text = format!(
        "{} · {} · {}\n",
        paint("Hannes", Tone::Accent, color),
        clean(&report.project),
        paint(&clean(&report.timestamp), Tone::Muted, color)
    );
    {
        let plan = &report.plan;
        text.push_str(&format!(
            "Execution Point: {} · Input: {} · {}{}\n",
            clean(&plan.at),
            clean(&plan.input.kind),
            "Execution point",
            if plan.selection_complete {
                ""
            } else {
                " · unvollständig: nur Checks"
            }
        ));
        if plan.scheduled.is_empty() {
            text.push_str("Keine aktiven Regeln ausgewählt; kein Regel-PASS belegt.\n");
        }
    }
    if report.checks_only {
        text.push_str(&paint(
            "Modus: Nur Checks · Reviews ausgelassen · kein LLM-Aufruf\n",
            Tone::Accent,
            color,
        ));
        if !report.results.iter().any(|item| item.kind == "check") {
            text.push_str("Keine aktiven Checks konfiguriert.\n");
        }
    }
    text.push_str(&format!(
        "\n{:<9} {:<24} {:<9} {:<9} SUMMARY\n",
        "TYPE", "ID", "STATUS", "TIME"
    ));
    for item in &report.results {
        text.push_str(&format!(
            "{:<9} {:<24} {}{} {:<9} {}\n",
            clean(&item.kind),
            clean(&item.id),
            status(item.status, color),
            " ".repeat(9 - item.status.label().len()),
            format!("{} ms", item.duration_ms),
            short_summary(&item_reason(item))
        ));
    }
    text.push_str(&format!("\nGesamt: {}\nReport: .hannes/reports/last.json\nDetails: hannes was ist phase --details\n", status(report.overall, color)));
    if let Some(usage) = report.provider_usage {
        text.push_str(&format!(
            "Codex-Tokens: {} gesamt · {} Eingabe ({} Cache) · {} Ausgabe\n",
            usage
                .total()
                .map(|n| n.to_string())
                .unwrap_or_else(|| "unbekannt".into()),
            usage.input_tokens,
            usage.cached_input_tokens,
            usage.output_tokens
        ));
    } else if report
        .results
        .iter()
        .any(|item| item.kind == "review" && item.status != Status::NotRun)
    {
        text.push_str("Codex-Tokens: nicht verfügbar\n");
    }
    if !report.snapshot.complete {
        text.push_str("Snapshot: UNKNOWN (details in report)\n");
    }
    text
}
/// Render the saved report only; this never executes commands or invokes a provider.
pub fn render_details(report: &Report, color: bool) -> String {
    let mut text = render(report, color);
    {
        let plan = &report.plan;
        text.push_str(&format!(
            "Plan-ID: {} · Auswahl: {}\n",
            clean(&plan.plan_id),
            if plan.selection_complete {
                "vollständig"
            } else {
                "eingeschränkt"
            }
        ));
        if !plan.required_ids.is_empty() {
            text.push_str(&format!(
                "Pflichtregeln: {}\n",
                clean(&plan.required_ids.join(", "))
            ));
        }
        for omitted in &plan.omitted {
            text.push_str(&format!(
                "Ausgelassen: {} · {}\n",
                clean(&omitted.rule.id),
                clean(&omitted.reason)
            ));
        }
    }
    if let Some(usage) = report.provider_usage {
        text.push_str(&format!(
            "Reasoning-Tokens (in Ausgabe enthalten): {} · Cache-Schreib-Tokens (in Eingabe enthalten): {}\n",
            usage.reasoning_output_tokens.map(|n| n.to_string()).unwrap_or_else(|| "nicht verfügbar".into()),
            usage.cache_write_input_tokens.map(|n| n.to_string()).unwrap_or_else(|| "nicht verfügbar".into())
        ));
        text.push_str("Verbrauch laut Codex CLI für den gesamten Review-Aufruf; keine Geld- oder Kontingentabrechnung.\n");
    }
    text.push_str(&format!(
        "\n{}\nRepository: {}\nProvider: {}\nHEAD: {}\nSnapshot: {}\n{}\n",
        paint("Details", Tone::Heading, color),
        clean(&report.repository),
        clean(&report.provider),
        clean(report.snapshot.head.as_deref().unwrap_or("unborn")),
        clean(&report.snapshot.fingerprint),
        clean(&report.git_summary)
    ));
    for item in &report.results {
        text.push_str(&format!(
            "\n{} · {}\n{}\n",
            paint(&clean(&item.id), Tone::Accent, color),
            status(item.status, color),
            clean(&item_reason(item))
        ));
        for finding in &item.findings {
            let reference = finding
                .path
                .as_ref()
                .map(|path| {
                    format!(
                        " [{}:{}]",
                        clean(path),
                        finding
                            .line
                            .map(|line| line.to_string())
                            .unwrap_or_else(|| "?".into())
                    )
                })
                .unwrap_or_default();
            text.push_str(&format!("  - {}{}\n", clean(&finding.message), reference));
        }
        for (name, capture) in [("stdout", &item.stdout), ("stderr", &item.stderr)] {
            if let Some(capture) = capture {
                if capture.total_bytes == 0 {
                    continue;
                }
                text.push_str(&format!(
                    "  {name}: {} bytes{}\n",
                    capture.total_bytes,
                    if capture.truncated {
                        " (truncated)"
                    } else {
                        ""
                    }
                ));
                for line in capture.text.lines() {
                    text.push_str(&format!("    {}\n", clean(line)));
                }
            }
        }
    }
    for limitation in &report.snapshot.limitations {
        text.push_str(&format!("Snapshot limitation: {}\n", clean(limitation)));
    }
    text
}
pub fn render_changes(changes: &Changes, color: bool) -> String {
    let count = changes
        .total_files
        .map(|n| n.to_string())
        .unwrap_or_else(|| "unavailable (partial snapshot)".into());
    let mut text = format!(
        "\n{} · changed files since verification: {count}\n",
        paint(
            &changes.state,
            if changes.state == "CURRENT" {
                Tone::Success
            } else {
                Tone::Warning
            },
            color
        )
    );
    if changes.head_changed {
        text.push_str("  Git HEAD changed since verification\n");
    }
    for path in &changes.sample_paths {
        text.push_str(&format!("  {} (line counts unavailable)\n", clean(path)));
    }
    text.push_str("Insertions/deletions since verification: unavailable (hash-only baseline)\n");
    text
}

fn item_reason(item: &crate::report::Item) -> String {
    if item.status == crate::report::Status::Fail && !item.blocking {
        format!("Non-blocking: {}", item.reason)
    } else {
        item.reason.clone()
    }
}
