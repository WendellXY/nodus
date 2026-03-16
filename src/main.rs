mod adapters;
mod cli;
mod git;
mod lockfile;
mod manifest;
mod resolver;
mod state;
mod store;

fn main() -> anyhow::Result<()> {
    cli::run()
}
