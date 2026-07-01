import type { TestTemplateDefinition } from "./types";

export function defaultTemplateDefinition(title: string): TestTemplateDefinition {
  return {
    definition_schema_version: "emc-locus.test-template-definition.v1",
    title,
    description: "Definition d'essai CEM a completer.",
    measurement_axis: "time_series",
    standard_references: [],
    variables: [
      {
        variable_id: "repeat_count",
        label: "Nombre de repetitions",
        value_type: "integer",
        default_value: 1,
        constraints: {
          required: true,
          dimensionless: true,
          minimum: 1,
          maximum: 10,
          enum_values: []
        },
        description: "Compteur sans dimension."
      }
    ],
    lock_policy: [
      {
        variable_id: "repeat_count",
        policy: "editable_until_execution"
      }
    ],
    instrumentation_chain: [
      {
        slot_id: "measurement_receiver",
        label: "Recepteur ou DAQ de mesure",
        required_category: "daq_chassis",
        required_capability: "time_series_capture",
        required: true,
        calibration_requirement: "if_used",
        substitution_policy: "same_capability",
        depends_on_slots: []
      }
    ],
    entry_step_id: "prepare",
    sequence: [
      {
        step_id: "prepare",
        order: 10,
        kind: "prepare",
        label: "Preparer l'essai",
        instruction: "Verifier le contexte operateur et l'instrumentation.",
        required_slots: ["measurement_receiver"],
        branches: []
      },
      {
        step_id: "finish",
        order: 20,
        kind: "finish",
        label: "Terminer",
        instruction: "Clore la sequence definie.",
        required_slots: [],
        branches: []
      }
    ],
    limits: [
      {
        limit_id: "attention_threshold",
        kind: "scalar_threshold",
        axis: "time_series",
        unit: "V",
        application_domain: "investigation",
        source_reference: "internal-method",
        threshold: 1,
        attention_rule: "warn_above_threshold",
        variable_refs: []
      }
    ],
    post_processing: [
      {
        operation_id: "peak",
        order: 10,
        operation_type: "peak",
        inputs: ["raw.signal"],
        outputs: ["calculated.peak"],
        parameters: { absolute: true }
      }
    ],
    method_parameters: {}
  };
}
