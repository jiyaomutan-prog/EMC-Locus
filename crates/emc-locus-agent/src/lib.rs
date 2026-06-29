use emc_locus_core::{baseline_repository_domains, RepositoryDomain};
use std::{
    error::Error,
    fmt,
    path::{Path, PathBuf},
};

pub const AGENT_NAME: &str = "emc-locus-agent";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentCommand {
    Health { storage_root: PathBuf },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentError {
    message: String,
}

impl AgentError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for AgentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for AgentError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HealthReport {
    pub agent: &'static str,
    pub version: &'static str,
    pub storage_root: PathBuf,
    pub storage_root_exists: bool,
    pub domains: Vec<&'static str>,
}

impl HealthReport {
    pub fn to_json(&self) -> String {
        let domains = self
            .domains
            .iter()
            .map(|domain| json_string(domain))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            concat!(
                "{{\n",
                "  \"agent\": {},\n",
                "  \"version\": {},\n",
                "  \"storage_root\": {},\n",
                "  \"storage_root_exists\": {},\n",
                "  \"domains\": [{}]\n",
                "}}"
            ),
            json_string(self.agent),
            json_string(self.version),
            json_string(&self.storage_root.to_string_lossy()),
            self.storage_root_exists,
            domains
        )
    }
}

pub fn parse_agent_args<I, S>(args: I) -> Result<AgentCommand, AgentError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut args = args.into_iter().map(Into::into);
    let command = args
        .next()
        .ok_or_else(|| AgentError::new("missing command"))?;
    if command != "health" {
        return Err(AgentError::new(format!("unknown command: {command}")));
    }

    let mut storage_root = PathBuf::from(".");
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--storage-root" => {
                let value = args
                    .next()
                    .ok_or_else(|| AgentError::new("missing value for --storage-root"))?;
                storage_root = PathBuf::from(value);
            }
            unknown => return Err(AgentError::new(format!("unknown argument: {unknown}"))),
        }
    }

    Ok(AgentCommand::Health { storage_root })
}

pub fn build_health_report(storage_root: impl AsRef<Path>) -> HealthReport {
    let storage_root = storage_root.as_ref().to_path_buf();
    let storage_root_exists = storage_root.exists();
    let domains = baseline_repository_domains()
        .into_iter()
        .map(RepositoryDomain::as_str)
        .collect();

    HealthReport {
        agent: AGENT_NAME,
        version: env!("CARGO_PKG_VERSION"),
        storage_root,
        storage_root_exists,
        domains,
    }
}

fn json_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            other => escaped.push(other),
        }
    }
    escaped.push('"');
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_health_command_with_storage_root() {
        let command = parse_agent_args(["health", "--storage-root", "E:/emc-locus"]).unwrap();

        assert_eq!(
            command,
            AgentCommand::Health {
                storage_root: PathBuf::from("E:/emc-locus")
            }
        );
    }

    #[test]
    fn rejects_unknown_command() {
        assert_eq!(
            parse_agent_args(["serve"]).unwrap_err().to_string(),
            "unknown command: serve"
        );
    }

    #[test]
    fn renders_health_report_as_json() {
        let report = HealthReport {
            agent: AGENT_NAME,
            version: "0.0.0-test",
            storage_root: PathBuf::from("E:/lab \"A\""),
            storage_root_exists: false,
            domains: vec!["metrology", "project_records"],
        };

        let json = report.to_json();

        assert!(json.contains("\"agent\": \"emc-locus-agent\""));
        assert!(json.contains("\"storage_root\": \"E:/lab \\\"A\\\"\""));
        assert!(json.contains("\"storage_root_exists\": false"));
        assert!(json.contains("\"domains\": [\"metrology\", \"project_records\"]"));
    }

    #[test]
    fn health_report_exposes_repository_domains() {
        let report = build_health_report(".");

        assert!(report.storage_root_exists);
        assert!(report.domains.contains(&"metrology"));
        assert!(report.domains.contains(&"project_records"));
        assert!(report.domains.contains(&"measurement_data"));
    }
}
