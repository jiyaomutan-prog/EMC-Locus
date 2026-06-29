use emc_locus_agent::{build_health_report, parse_agent_args, run_storage_command, AgentCommand};
use std::{env, process};

fn main() {
    match parse_agent_args(env::args().skip(1)) {
        Ok(AgentCommand::Health { storage_root }) => {
            println!("{}", build_health_report(storage_root).to_json());
        }
        Ok(command @ AgentCommand::Storage { .. }) => match run_storage_command(command) {
            Ok(report) => println!("{}", report.to_json()),
            Err(error) => {
                eprintln!("{}", error.to_json());
                process::exit(1);
            }
        },
        Err(error) => {
            eprintln!("error: {error}");
            eprintln!(
                "usage: emc-locus-agent health [--storage-root PATH] | storage <init|status|verify> --storage-root PATH [--migrations-root PATH]"
            );
            process::exit(2);
        }
    }
}
