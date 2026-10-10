mod compare;
mod nvm;
mod output;
mod registry;
mod sanitize;

use anyhow::{Result, bail};
use clap::{Args, ColorChoice, Parser, Subcommand};

/// List installed nvm Node versions and their global npm packages.
#[derive(Parser)]
#[command(version, args_conflicts_with_subcommands = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[command(flatten)]
    list: ListArgs,

    /// Also include packages bundled with Node (npm, corepack).
    #[arg(short, long, global = true)]
    all: bool,

    /// When to use colors.
    #[arg(long, global = true, value_enum, default_value_t = ColorChoice::Auto)]
    color: ColorChoice,
}

#[derive(Subcommand)]
enum Command {
    /// List versions and their global packages (the default).
    List(ListArgs),

    /// Compare the global packages of two versions and print how to install
    /// the missing ones, e.g. `nvmem diff 24 26`.
    Diff {
        /// Version to copy packages from.
        from: String,
        /// Version to copy packages to.
        to: String,
        /// Pin the versions currently installed in FROM (`name@x.y.z`).
        #[arg(long)]
        exact: bool,
        /// Only print the install command (e.g. to pipe into a shell).
        #[arg(long)]
        command_only: bool,
    },

    /// Show which versions have a package, by package or command name,
    /// e.g. `nvmem which tsc`.
    Which {
        /// Package name (`typescript`, `@shopify/cli`) or command (`tsc`).
        name: String,
    },
}

#[derive(Args)]
struct ListArgs {
    /// Only show versions matching this: `24`, `22.9`, `v24.21.0`,
    /// `default`, `node`, or an nvm alias like `lts/*`.
    #[arg(value_name = "VERSION")]
    node_version: Option<String>,

    /// Show a package × version matrix instead of a tree.
    #[arg(short, long)]
    table: bool,

    /// Check the npm registry for updates and deprecated packages
    /// (sends package names and versions to the registry).
    #[arg(short, long)]
    updates: bool,

    /// Like --updates, but ignore cached registry answers and check again now.
    #[arg(long)]
    refresh: bool,

    /// No-op kept for compatibility: nvmem is offline unless --updates is given.
    #[arg(long, hide = true)]
    offline: bool,
}

impl ListArgs {
    /// Whether to contact the npm registry. Offline is the default.
    fn check_updates(&self) -> bool {
        (self.updates || self.refresh) && !self.offline
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.color {
        ColorChoice::Auto => {}
        ColorChoice::Always => anstream::ColorChoice::Always.write_global(),
        ColorChoice::Never => anstream::ColorChoice::Never.write_global(),
    }

    let install = nvm::Install::detect()?;
    match cli.command {
        None => list(&install, &cli.list, cli.all),
        Some(Command::List(args)) => list(&install, &args, cli.all),
        Some(Command::Diff {
            from,
            to,
            exact,
            command_only,
        }) => diff(&install, &from, &to, exact, command_only, cli.all),
        // Bundled packages are always searchable: `nvmem which npx` should work.
        Some(Command::Which { name }) => which(&install, &name),
    }
}

fn list(install: &nvm::Install, args: &ListArgs, all: bool) -> Result<()> {
    let mut entries = install.load_entries(all)?;
    if entries.is_empty() {
        anstream::println!("No Node versions installed in {}", install.root.display());
        return Ok(());
    }
    if let Some(spec) = &args.node_version {
        let selected = select(install, &entries, spec)?;
        entries = selected.into_iter().map(|i| entries[i].clone()).collect();
    }

    let incomplete = args.check_updates() && check_registry(&mut entries, args.refresh);

    let rendered = if args.table {
        output::table(&entries)
    } else {
        output::tree(&entries)
    };
    anstream::print!("{rendered}");
    anstream::print!(
        "{}",
        output::registry_summary(&entries, &install.flavor, incomplete)
    );
    Ok(())
}

/// Fills in update / deprecation info from the npm registry. Returns true when
/// the registry couldn't be reached for some packages.
fn check_registry(entries: &mut [nvm::Entry], refresh: bool) -> bool {
    // Linked packages are local checkouts, not registry releases.
    let packages: Vec<(String, String)> = entries
        .iter()
        .flat_map(|e| &e.packages)
        .filter(|p| !p.linked)
        .filter_map(|p| Some((p.name.clone(), p.version.clone()?)))
        .collect();
    if packages.is_empty() {
        return false;
    }
    let lookup = registry::lookup(
        &packages,
        registry::default_cache_path().as_deref(),
        registry::CACHE_TTL,
        refresh,
        registry::http_fetcher(),
    );
    for pkg in entries.iter_mut().flat_map(|e| &mut e.packages) {
        if let Some(version) = &pkg.version
            && let Some(info) = lookup.info.get(&(pkg.name.clone(), version.clone()))
        {
            pkg.registry = info.clone();
        }
    }
    lookup.incomplete
}

fn diff(
    install: &nvm::Install,
    from: &str,
    to: &str,
    exact: bool,
    command_only: bool,
    all: bool,
) -> Result<()> {
    let entries = install.load_entries(all)?;
    // A spec can match several versions (e.g. `22`); use the newest.
    let from = &entries[select(install, &entries, from)?[0]];
    let to = &entries[select(install, &entries, to)?[0]];

    let d = compare::diff(&from.packages, &to.packages);
    let command = compare::install_command(&install.flavor, &to.version, &d.only_from, exact);
    if command_only {
        if let Some(command) = command {
            println!("{command}");
        }
        return Ok(());
    }
    let skipped = compare::not_installable(&d.only_from, exact);
    anstream::print!(
        "{}",
        output::diff(from, to, &d, command.as_deref(), &skipped)
    );
    Ok(())
}

fn which(install: &nvm::Install, name: &str) -> Result<()> {
    let entries = install.load_entries(true)?;
    let found = compare::find(&entries, name);
    if found.is_empty() {
        let name = sanitize::printable(name);
        let hints: Vec<_> = compare::suggestions(&entries, &name)
            .into_iter()
            .map(sanitize::printable)
            .collect();
        if hints.is_empty() {
            bail!("no package or command named '{name}' in any Node version");
        }
        bail!(
            "no package or command named '{name}'. Did you mean: {}?",
            hints.join(", ")
        );
    }
    anstream::print!("{}", output::found(&found));
    Ok(())
}

/// Like [`nvm::Install::select`], but an error when nothing matches.
fn select(install: &nvm::Install, entries: &[nvm::Entry], spec: &str) -> Result<Vec<usize>> {
    let selected = install.select(entries, spec);
    if selected.is_empty() {
        let installed: Vec<&str> = entries.iter().map(|e| e.version.name.as_str()).collect();
        bail!(
            "no installed version matches '{spec}' (installed: {})",
            installed.join(", ")
        );
    }
    Ok(selected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parses_list_shorthand_and_subcommands() {
        let cli = Cli::try_parse_from(["nvmem", "24", "--table"]).unwrap();
        assert!(cli.command.is_none());
        assert_eq!(cli.list.node_version.as_deref(), Some("24"));
        assert!(cli.list.table);

        // Offline unless asked; --refresh implies --updates; --offline still parses.
        assert!(!Cli::try_parse_from(["nvmem"]).unwrap().list.check_updates());
        assert!(
            Cli::try_parse_from(["nvmem", "-u"])
                .unwrap()
                .list
                .check_updates()
        );
        assert!(
            Cli::try_parse_from(["nvmem", "24", "--updates", "--table"])
                .unwrap()
                .list
                .check_updates()
        );
        assert!(
            Cli::try_parse_from(["nvmem", "--refresh"])
                .unwrap()
                .list
                .check_updates()
        );
        assert!(
            !Cli::try_parse_from(["nvmem", "-u", "--offline"])
                .unwrap()
                .list
                .check_updates()
        );
        assert!(
            Cli::try_parse_from(["nvmem", "list", "-u"])
                .unwrap()
                .command
                .is_some_and(|c| matches!(c, Command::List(args) if args.check_updates()))
        );

        let cli = Cli::try_parse_from(["nvmem", "diff", "24", "26", "--all"]).unwrap();
        assert!(cli.all);
        assert!(matches!(cli.command, Some(Command::Diff { .. })));

        let cli = Cli::try_parse_from(["nvmem", "which", "tsc"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Which { name }) if name == "tsc"));
    }
}
