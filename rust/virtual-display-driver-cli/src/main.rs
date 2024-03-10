use clap::Parser;
use client::Client;
use joinery::JoinableIterator;
use lazy_format::lazy_format;
use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};

mod client;
mod mode;

#[derive(Debug, Parser)]
struct Args {
    #[clap(flatten)]
    options: GlobalOptions,

    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Parser)]
struct GlobalOptions {
    /// Format output as JSON.
    #[clap(short, long)]
    json: bool,
}

#[derive(Debug, Parser)]
enum Command {
    /// List currently connected virtual monitors.
    List,
    /// Add a new virtual monitor.
    Add(AddCommand),
    /// Add a new resolution/refresh rate mode to an existing virtual monitor.
    AddMode(AddModeCommand),
    /// Remove a resolution/refresh rate mode to an existing virtual monitor.
    RemoveMode(RemoveModeCommand),
    /// Enable a virtual monitor.
    Enable(EnableCommand),
    /// Disable a virtual monitor.
    Disable(DisableCommand),
    /// Remove one or more virtual monitors.
    Remove(RemoveCommand),
    /// Remove all virtual monitors.
    RemoveAll,
}

#[derive(Debug, Parser)]
struct AddCommand {
    /// One or more resolutions/refresh rates to add to the virtual monitor.
    /// Example values: `1920x1080`, `3840x2160@120`, `1280x720@60/120`.
    mode: Vec<mode::Mode>,

    /// Manual ID to set for the monitor. Must not conflict with an
    /// existing virtual monitor's ID.
    #[clap(long)]
    id: Option<driver_ipc::Id>,

    /// Optional label for the virtual monitor.
    #[clap(long)]
    name: Option<String>,

    /// Set the virtual monitor to disabled on creation.
    #[clap(long)]
    disabled: bool,
}

#[derive(Debug, Parser)]
struct AddModeCommand {
    /// ID of the virtual monitor to add a mode to.
    id: driver_ipc::Id,

    /// One or more resolutions/refresh rates to add to the virtual monitor.
    /// Example values: `1920x1080`, `3840x2160@120`, `1280x720@60/120`.
    mode: Vec<mode::Mode>,
}

#[derive(Debug, Parser)]
struct RemoveModeCommand {
    /// ID of the virtual monitor to add a mode to.
    id: driver_ipc::Id,

    /// A resolution and optional refresh rate to remove from the virtual
    /// monitor. Omitting the refresh rate will remove the resolution, including
    /// the refresh rate will keep the resolution but remove just the given
    /// refresh rate. Example values: `1920x1080`, `3840x2160@120`.
    mode: mode::Mode,
}

#[derive(Debug, Parser)]
struct EnableCommand {
    id: driver_ipc::Id,
}

#[derive(Debug, Parser)]
struct DisableCommand {
    id: driver_ipc::Id,
}

#[derive(Debug, Parser)]
struct RemoveCommand {
    id: Vec<driver_ipc::Id>,
}

fn main() -> eyre::Result<()> {
    let Args { options, command } = Args::parse();
    let mut client = Client::connect()?;

    match command {
        Command::List => {
            list(&mut client, &options)?;
        }
        Command::Add(command) => {
            add(&mut client, &options, command)?;
        }
        Command::AddMode(command) => {
            add_mode(&mut client, &options, command)?;
        }
        Command::RemoveMode(command) => {
            remove_mode(&mut client, &options, &command)?;
        }
        Command::Enable(command) => {
            enable(&mut client, &options, &command)?;
        }
        Command::Disable(command) => {
            disable(&mut client, &options, &command)?;
        }
        Command::Remove(command) => {
            remove(&mut client, &options, &command)?;
        }
        Command::RemoveAll => {
            remove_all(&mut client, &options)?;
        }
    }

    Ok(())
}

fn list(client: &mut Client, opts: &GlobalOptions) -> eyre::Result<()> {
    let monitors = client.monitors();

    if opts.json {
        let mut stdout = std::io::stdout().lock();
        serde_json::to_writer_pretty(&mut stdout, &monitors)?;
    } else if !monitors.is_empty() {
        println!("{}", "Virtual monitors".underline());
        for (i, monitor) in monitors.iter().enumerate() {
            if i > 0 {
                println!();
            }

            let name_label = lazy_format!(match (&monitor.name) {
                Some(name) => (" {}{name}{}", "[".dimmed(), "]".dimmed()),
                None => "",
            });
            let disabled_label = lazy_format!(if monitor.enabled => ""
            else =>
                (" {}", "(disabled)".red())
            );
            println!(
                "Monitor {}{name_label}{disabled_label}:",
                monitor.id.green(),
            );

            if monitor.modes.is_empty() {
                println!("{} {}", "-".dimmed(), "No modes".red());
            } else {
                for (index, mode) in monitor.modes.iter().enumerate() {
                    let refresh_rate_labels = mode
                        .refresh_rates
                        .iter()
                        .map(|rate| lazy_format!("{}", rate.blue()))
                        .join_with("/");
                    let refresh_rates = lazy_format!(if mode.refresh_rates.is_empty() =>
                        ("{}Hz", "?".red())
                    else =>
                        ("{}Hz", refresh_rate_labels)
                    );
                    println!(
                        "{} Mode {index}: {}x{} @ {}",
                        "-".dimmed(),
                        mode.width.green(),
                        mode.height.green(),
                        refresh_rates
                    );
                }
            }
        }
    } else {
        println!("No virtual monitors found.");
    }

    Ok(())
}

fn add(client: &mut Client, opts: &GlobalOptions, command: AddCommand) -> eyre::Result<()> {
    let modes = command
        .mode
        .into_iter()
        .map(driver_ipc::Mode::from)
        .collect::<Vec<_>>();

    let id = client.new_id(command.id)?;
    let new_monitor = driver_ipc::Monitor {
        id,
        enabled: !command.disabled,
        name: command.name,
        modes,
    };
    client.notify(vec![new_monitor])?;

    if opts.json {
        let mut stdout = std::io::stdout().lock();
        serde_json::to_writer_pretty(&mut stdout, &id)?;
    } else {
        let disabled_footnote = lazy_format!(
            if command.disabled => (" {}", "(disabled)".red())
            else => ""
        );
        println!(
            "Added virtual monitor with ID {}{disabled_footnote}.",
            id.green()
        );
    }

    Ok(())
}

fn add_mode(
    client: &mut Client,
    opts: &GlobalOptions,
    command: AddModeCommand,
) -> eyre::Result<()> {
    let mut monitor = client.get(command.id)?;

    let existing_modes = monitor.modes.iter().cloned().map(mode::Mode::from);
    let new_modes = mode::merge(existing_modes.chain(command.mode));
    let new_modes: Vec<driver_ipc::Mode> =
        new_modes.into_iter().map(driver_ipc::Mode::from).collect();

    monitor.modes = new_modes.clone();
    client.notify(vec![monitor])?;

    if opts.json {
        let mut stdout = std::io::stdout().lock();
        serde_json::to_writer_pretty(&mut stdout, &new_modes)?;
    } else {
        println!(
            "Added modes to virtual monitor with ID {}.",
            command.id.green()
        );
    }

    Ok(())
}

fn remove_mode(
    client: &mut Client,
    opts: &GlobalOptions,
    command: &RemoveModeCommand,
) -> eyre::Result<()> {
    let mut monitor = client.get(command.id)?;

    let modes = monitor.modes.iter().cloned().map(mode::Mode::from);
    let new_modes = mode::remove(modes, &command.mode)?;
    let new_modes: Vec<driver_ipc::Mode> =
        new_modes.into_iter().map(driver_ipc::Mode::from).collect();

    monitor.modes = new_modes.clone();
    client.notify(vec![monitor])?;

    if opts.json {
        let mut stdout = std::io::stdout().lock();
        serde_json::to_writer_pretty(&mut stdout, &new_modes)?;
    } else {
        println!(
            "Removed mode {} from virtual monitor with ID {}.",
            command.mode.blue(),
            command.id.green()
        );
    }

    Ok(())
}

fn enable(client: &mut Client, opts: &GlobalOptions, command: &EnableCommand) -> eyre::Result<()> {
    let outcome = set_enabled(client, command.id, true)?;

    if opts.json {
        let mut stdout = std::io::stdout().lock();
        serde_json::to_writer_pretty(&mut stdout, &outcome)?;
    } else {
        let footnote = if outcome.toggled {
            ""
        } else {
            " (was already enabled)"
        };
        println!(
            "Enabled virtual monitor with ID {}{footnote}.",
            command.id.green()
        );
    }

    Ok(())
}

fn disable(
    client: &mut Client,
    opts: &GlobalOptions,
    command: &DisableCommand,
) -> eyre::Result<()> {
    let outcome = set_enabled(client, command.id, false)?;

    if opts.json {
        let mut stdout = std::io::stdout().lock();
        serde_json::to_writer_pretty(&mut stdout, &outcome)?;
    } else {
        let footnote = if outcome.toggled {
            ""
        } else {
            " (was already disabled)"
        };
        println!(
            "Disabled virtual monitor with ID {}{footnote}.",
            command.id.green()
        );
    }

    Ok(())
}

fn remove(client: &mut Client, opts: &GlobalOptions, command: &RemoveCommand) -> eyre::Result<()> {
    client.validate_has_ids(&command.id)?;
    client.remove(command.id.clone())?;

    if opts.json {
        let mut stdout = std::io::stdout().lock();
        serde_json::to_writer_pretty(&mut stdout, &command.id)?;
    } else if command.id.len() == 1 {
        println!("Removed virtual monitor.");
    } else {
        println!("Removed {} virtual monitors.", command.id.len());
    }

    Ok(())
}

fn remove_all(client: &mut Client, opts: &GlobalOptions) -> eyre::Result<()> {
    client.remove_all()?;

    if opts.json {
        let mut stdout = std::io::stdout().lock();
        serde_json::to_writer_pretty(&mut stdout, &())?;
    } else {
        println!("Removed all virtual monitors.");
    }

    Ok(())
}

fn set_enabled(
    client: &mut Client,
    id: driver_ipc::Id,
    enabled: bool,
) -> eyre::Result<EnableDisableOutcome> {
    let mut monitor = client.get(id)?;

    let should_toggle = enabled != monitor.enabled;

    if should_toggle {
        monitor.enabled = enabled;
        client.notify(vec![monitor.clone()])?;
    }

    Ok(EnableDisableOutcome {
        monitor,
        toggled: should_toggle,
    })
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct EnableDisableOutcome {
    monitor: driver_ipc::Monitor,
    toggled: bool,
}
