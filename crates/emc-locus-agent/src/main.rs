use emc_locus_agent::{build_health_report, parse_agent_args, AgentCommand};
use std::{env, process};

fn main() {
    match parse_agent_args(env::args().skip(1)) {
        Ok(AgentCommand::Health { storage_root }) => {
            println!("{}", build_health_report(storage_root).to_json());
        }
        Err(error) => {
            eprintln!("error: {error}");
            eprintln!("usage: emc-locus-agent health [--storage-root PATH]");
            process::exit(2);
        }
    }
}
