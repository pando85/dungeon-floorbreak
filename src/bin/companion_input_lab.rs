//! COMPANION-AI-01 01C lab: replays scripted pad traces through the command-input
//! grammar variants and prints the semantic action stream and the switch-latency
//! table. Headless, no Bevy, no pad access. Real DualSense / Steam Deck
//! validation is NOT RUN here.
//!
//! Usage: `cargo run --bin companion_input_lab [-- --hold N] [-- "<script>"]`
//!
//! Script syntax: see `companion::command_input::parse_script`. Action legend:
//! `.` pass-through, `S` swap, `F` Focus, `H` Hold, `R` Regroup, `A` Assist,
//! `O` open mode, `C` cancel mode, `X` refused, `I` delayed interact.

use dungeon_floorbreak::companion::command_input::{
    Action, GrammarConfig, GrammarVariant, first_tick, parse_script, render, run,
};

const TRACES: &[(&str, &str)] = &[
    ("short tap (3 ticks)", "L1*3 -*3"),
    ("hold, no selection (12 ticks)", "L1*12 -*2"),
    ("fast L1+Square chord", "L1*3 L1+Sq -*2"),
    ("chord on the L1 press tick", "L1+Tri -*2"),
    ("two faces at once", "L1 L1+Cro+Tri -*2"),
    ("Down never chords", "L1 L1+Dn -*2"),
    ("R1 tap then R1+Circle", "R1*3 -*2 R1 R1+Cir -*2"),
    ("toggle then Cross", "Tog -*2 Cro -*2"),
    ("Square pre-held, L1 tap", "Sq Sq+L1 Sq+L1 Sq -"),
    ("blocked (no companion) chord", "L1 L1+Sq+Blocked -"),
    (
        "pad drops with mode open",
        "L1*10 disc - reconn+L1 L1 - L1 -",
    ),
    ("second human joins mid hold", "L1*10 join L1*3 -*2"),
];

fn config(variant: GrammarVariant, hold: u8) -> GrammarConfig {
    GrammarConfig {
        variant,
        hold_ticks: hold,
    }
}

fn print_trace(label: &str, text: &str, hold: u8) -> Result<(), String> {
    let ticks = parse_script(text)?;
    println!("trace: {label}\n  script : {text}");
    for variant in GrammarVariant::ALL {
        let outs = run(config(variant, hold), &ticks);
        let leaks = outs
            .iter()
            .map(|o| if o.clear_buffers { '^' } else { ' ' })
            .collect::<String>();
        println!("  {:<42} {}", variant.label(), render(&outs));
        if leaks.contains('^') {
            println!("  {:<42} {}  (^ clear move+swap buffers)", "", leaks);
        }
    }
    println!();
    Ok(())
}

fn ms(ticks: usize) -> String {
    format!("{:.1}", ticks as f64 * 1000.0 / 60.0)
}

fn latency_table(hold: u8) {
    println!("switch latency in ticks (60 Hz) from the physical press edge");
    println!(
        "| variant | tap 3 | tap 6 | tap 9 | hold opens | fast chord (after face edge) | R1 tap interact |"
    );
    println!("|---|---|---|---|---|---|---|");
    for variant in GrammarVariant::ALL {
        let cfg = config(variant, hold);
        let tap = |down: usize| -> String {
            let ticks = parse_script(&format!("L1*{down} -*8")).expect("script");
            match first_tick(cfg, &ticks, |a| *a == Action::Swap) {
                Some(n) => format!("{n} ({} ms)", ms(n)),
                None => "no swap".to_string(),
            }
        };
        let modifier = match variant {
            GrammarVariant::R1Modifier => "R1",
            GrammarVariant::Toggle => "Tog",
            _ => "L1",
        };
        let open = {
            let ticks = parse_script(&format!("{modifier}*30")).expect("script");
            match first_tick(cfg, &ticks, |a| *a == Action::OpenMode) {
                Some(n) => format!("{n} ({} ms)", ms(n)),
                None => "n/a".to_string(),
            }
        };
        let chord = {
            let ticks = parse_script(&format!("{modifier}*3 {modifier}+Sq")).expect("script");
            match first_tick(cfg, &ticks, |a| matches!(a, Action::Command(_))) {
                Some(n) => format!("{} ticks", n - 3),
                None => "n/a".to_string(),
            }
        };
        let interact = if variant == GrammarVariant::R1Modifier {
            let ticks = parse_script("R1*3 -*3").expect("script");
            match first_tick(cfg, &ticks, |a| *a == Action::DelayedInteract) {
                Some(n) => format!("{n} (+{n} vs immediate)"),
                None => "none".to_string(),
            }
        } else {
            "0 (immediate)".to_string()
        };
        println!(
            "| {} | {} | {} | {} | {} | {} | {} |",
            variant.label(),
            tap(3),
            tap(6),
            tap(9),
            open,
            chord,
            interact
        );
    }
    println!("\nreal-pad validation (DualSense, Steam Deck): NOT RUN (no physical pad)");
}

fn main() -> Result<(), String> {
    let mut hold = 10_u8;
    let mut custom: Vec<String> = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--hold" {
            hold = args
                .next()
                .and_then(|n| n.parse().ok())
                .ok_or("--hold needs a tick count")?;
        } else {
            custom.push(arg);
        }
    }
    if custom.is_empty() {
        for (label, text) in TRACES {
            print_trace(label, text, hold)?;
        }
    } else {
        for text in &custom {
            print_trace("custom", text, hold)?;
        }
    }
    latency_table(hold);
    Ok(())
}
