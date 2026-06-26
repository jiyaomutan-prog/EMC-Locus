use crate::{
    datasets::{MeasurementRunEvidence, RawDatasetRecord},
    instrument_runtime::{InstrumentCommand, InstrumentObservation, SimulatedInstrumentRuntime},
    measurement::MeasurementRunPlan,
    DomainError,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeasurementExecutionSession {
    runtime: SimulatedInstrumentRuntime,
    evidence: MeasurementRunEvidence,
}

impl MeasurementExecutionSession {
    pub fn new(
        plan: MeasurementRunPlan,
        runtime: SimulatedInstrumentRuntime,
    ) -> Result<Self, DomainError> {
        if !plan.equipment().contains(runtime.instrument()) {
            return Err(DomainError::ExecutionInstrumentNotPlanned(
                runtime.instrument().as_str().to_owned(),
            ));
        }

        Ok(Self {
            runtime,
            evidence: MeasurementRunEvidence::new(plan),
        })
    }

    pub fn runtime(&self) -> &SimulatedInstrumentRuntime {
        &self.runtime
    }

    pub fn evidence(&self) -> &MeasurementRunEvidence {
        &self.evidence
    }

    pub fn execute_command(
        &mut self,
        command: InstrumentCommand,
    ) -> Result<&InstrumentObservation, DomainError> {
        let observation = self.runtime.execute(command)?.clone();
        self.evidence.record_observation(observation);

        Ok(self
            .evidence
            .observations()
            .last()
            .expect("observation was just recorded"))
    }

    pub fn record_raw_dataset(&mut self, dataset: RawDatasetRecord) -> Result<(), DomainError> {
        self.evidence.record_raw_dataset(dataset)
    }

    pub fn finish(self) -> Result<MeasurementRunEvidence, DomainError> {
        if !self.evidence.has_raw_data() {
            return Err(DomainError::MeasurementRunMissingRawData);
        }

        Ok(self.evidence)
    }
}
