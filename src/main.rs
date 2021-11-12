mod cert;
mod migrate;

use std::path::PathBuf;

use clap::App;
use clap::Arg;

use anyhow::Context;
use anyhow::Result;

fn main() -> Result<()> {
    let migrate_cmd = App::new("migrate")
        .about("Migrate the chain data")
        .arg(
            Arg::new("chain-dir")
                .about("The old chain dir")
                .short('d')
                .long("chain-dir")
                .takes_value(true)
                .required(true)
                .validator(str::parse::<PathBuf>),
        )
        .arg(
            Arg::new("out-dir")
                .about("The output dir for the upgraded chain")
                .short('o')
                .long("out-dir")
                .takes_value(true)
                .required(true)
                .validator(str::parse::<PathBuf>),
        )
        .arg(
            Arg::new("chain-name")
                .about("Name of the chain")
                .short('n')
                .long("chain-name")
                .takes_value(true)
                .required(true)
                .validator(str::parse::<PathBuf>),
        );

    let app = App::new("migration-tool")
        // It's surprising that a minor version bump results in a huge change.
        .about("migration tool for upgrading CITA-Cloud chain from 6.1.0 to 6.3.0")
        .subcommand(migrate_cmd);

    match app.get_matches().subcommand() {
        Some(("migrate", m)) => {
            let chain_dir = m.value_of("chain-dir").unwrap();
            let out_dir = m.value_of("out-dir").unwrap();
            let chain_name = m.value_of("chain-name").unwrap();

            migrate::migrate(chain_dir, out_dir, chain_name).context("cannot migrate chain")?;
        }
        None => {
            println!("no subcommand provided");
        }
        _ => {
            unreachable!()
        }
    }

    Ok(())
}
