use emc_locus_agent::{
    build_health_report, parse_agent_args, run_local_api_server, run_metrology_command,
    run_project_command, run_storage_command, run_sync_command, AgentCommand,
};
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
        Ok(command @ AgentCommand::Projects { .. }) => match run_project_command(command) {
            Ok(json) => println!("{json}"),
            Err(error) => {
                eprintln!("{}", error.to_json());
                process::exit(1);
            }
        },
        Ok(command @ AgentCommand::Metrology { .. }) => match run_metrology_command(command) {
            Ok(json) => println!("{json}"),
            Err(error) => {
                eprintln!("{}", error.to_json());
                process::exit(1);
            }
        },
        Ok(command @ AgentCommand::Sync { .. }) => match run_sync_command(command) {
            Ok(json) => println!("{json}"),
            Err(error) => {
                eprintln!("{}", error.to_json());
                process::exit(1);
            }
        },
        Ok(AgentCommand::Serve { config }) => {
            if let Err(error) = run_local_api_server(config) {
                eprintln!("{}", error.to_json());
                process::exit(1);
            }
        }
        Err(error) => {
            eprintln!("{}", error.to_json());
            eprintln!(
                "usage: emc-locus-agent health [--storage-root PATH] | storage <init|status|verify> --storage-root PATH [--migrations-root PATH] | projects <create|list|get|contract-review|complete-review-item|to-test-planning|audit-events> --storage-root PATH ... | metrology <register-instrument|list-instruments|get-instrument|record-calibration|list-calibrations|status|set-serviceability|readiness|audit-events> --storage-root PATH ... | sync outbox --storage-root PATH | serve --storage-root PATH [--migrations-root PATH] [--bind 127.0.0.1:8765] [--lab-console-dist apps/lab-console/dist]"
            );
            process::exit(2);
        }
    }
}
