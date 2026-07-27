use serde::{Deserialize, Serialize};

pub const PHYSICAL_ASSET_DEFINITION_SCHEMA_VERSION: &str = "emc-locus.physical-asset-definition.v1";
pub const LABORATORY_LOCATION_DEFINITION_SCHEMA_VERSION: &str =
    "emc-locus.laboratory-location-definition.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnershipSource {
    LaboratoryOwned,
    CustomerSupplied,
    Rented,
    Borrowed,
    External,
    SoftwareLicense,
    InstalledFacility,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceState {
    Usable,
    Restricted,
    InMaintenance,
    OutOfService,
    Retired,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdministrativeAvailability {
    Available,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationalUsageState {
    Available,
    Reserved,
    AssignedToSetup,
    InTest,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaboratoryLocationStatus {
    Active,
    Archived,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinnedEquipmentModel {
    pub equipment_model_id: String,
    pub equipment_model_revision_id: String,
    pub equipment_model_checksum: String,
    pub manufacturer: String,
    pub model_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    pub category_code: String,
    #[serde(default)]
    pub category_path: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhysicalAssetDefinition {
    pub definition_schema_version: String,
    pub inventory_code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serial_number: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part_number: Option<String>,
    pub model: PinnedEquipmentModel,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub laboratory_location_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub laboratory_location_label: Option<String>,
    pub ownership_source: OwnershipSource,
    pub service_state: ServiceState,
    pub administrative_availability: AdministrativeAvailability,
    #[serde(default)]
    pub administrative_unavailability_reason: String,
    #[serde(default)]
    pub service_state_reason: String,
    #[serde(default)]
    pub notes: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhysicalAsset {
    pub asset_id: String,
    pub definition: PhysicalAssetDefinition,
    pub revision: u64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaboratoryLocationDefinition {
    pub definition_schema_version: String,
    pub label: String,
    #[serde(default)]
    pub description: String,
    pub status: LaboratoryLocationStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaboratoryLocation {
    pub location_id: String,
    pub definition: LaboratoryLocationDefinition,
    pub revision: u64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetValidationIssue {
    pub code: String,
    pub path: String,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetTransitionError {
    pub code: String,
    pub message: String,
    pub current_state: String,
    pub requested_state: String,
}

impl PhysicalAssetDefinition {
    pub fn validate_all(&self) -> Vec<FleetValidationIssue> {
        let mut issues = Vec::new();
        if self.definition_schema_version != PHYSICAL_ASSET_DEFINITION_SCHEMA_VERSION {
            push_issue(
                &mut issues,
                "unsupported_physical_asset_schema",
                "definition_schema_version",
                "La version de définition de l'exemplaire n'est pas prise en charge.",
            );
        }
        if !valid_inventory_code(&self.inventory_code) {
            push_issue(
                &mut issues,
                "invalid_inventory_code",
                "inventory_code",
                "Le code inventaire doit contenir 1 à 80 caractères alphanumériques, '-', '_', '.' ou '/'.",
            );
        }
        validate_optional_text(
            &mut issues,
            self.serial_number.as_deref(),
            "serial_number",
            200,
        );
        validate_optional_text(&mut issues, self.part_number.as_deref(), "part_number", 200);
        validate_pinned_model(&mut issues, &self.model);
        match (
            self.laboratory_location_id.as_deref(),
            self.laboratory_location_label.as_deref(),
        ) {
            (Some(id), Some(label)) => {
                if !valid_stable_id(id) {
                    push_issue(
                        &mut issues,
                        "invalid_laboratory_location_id",
                        "laboratory_location_id",
                        "L'identifiant du lieu n'est pas valide.",
                    );
                }
                if label.trim().is_empty() {
                    push_issue(
                        &mut issues,
                        "empty_laboratory_location_label",
                        "laboratory_location_label",
                        "Le libellé instantané du lieu est obligatoire quand un lieu est affecté.",
                    );
                }
            }
            (None, None) => {}
            _ => push_issue(
                &mut issues,
                "incomplete_laboratory_location_reference",
                "laboratory_location_id",
                "L'identifiant et le libellé du lieu doivent être présents ensemble.",
            ),
        }
        if matches!(
            self.service_state,
            ServiceState::InMaintenance | ServiceState::OutOfService | ServiceState::Retired
        ) && self.administrative_availability != AdministrativeAvailability::Unavailable
        {
            push_issue(
                &mut issues,
                "service_state_requires_unavailable",
                "administrative_availability",
                "Un exemplaire en maintenance, hors service ou retiré doit être indisponible.",
            );
        }
        if self.administrative_availability == AdministrativeAvailability::Unavailable
            && self.administrative_unavailability_reason.trim().is_empty()
        {
            push_issue(
                &mut issues,
                "administrative_unavailability_reason_required",
                "administrative_unavailability_reason",
                "Une indisponibilité administrative doit être justifiée.",
            );
        }
        if self.administrative_availability == AdministrativeAvailability::Available
            && !self.administrative_unavailability_reason.trim().is_empty()
        {
            push_issue(
                &mut issues,
                "unexpected_administrative_unavailability_reason",
                "administrative_unavailability_reason",
                "Le motif d'indisponibilité doit être vide lorsque l'exemplaire est disponible.",
            );
        }
        if self.service_state != ServiceState::Usable && self.service_state_reason.trim().is_empty()
        {
            push_issue(
                &mut issues,
                "service_state_reason_required",
                "service_state_reason",
                "Une justification est obligatoire lorsque l'état de service n'est pas utilisable.",
            );
        }
        issues
    }
}

impl LaboratoryLocationDefinition {
    pub fn validate_all(&self) -> Vec<FleetValidationIssue> {
        let mut issues = Vec::new();
        if self.definition_schema_version != LABORATORY_LOCATION_DEFINITION_SCHEMA_VERSION {
            push_issue(
                &mut issues,
                "unsupported_laboratory_location_schema",
                "definition_schema_version",
                "La version de définition du lieu n'est pas prise en charge.",
            );
        }
        if self.label.trim().is_empty() || self.label.trim().chars().count() > 160 {
            push_issue(
                &mut issues,
                "invalid_laboratory_location_label",
                "label",
                "Le libellé du lieu doit contenir entre 1 et 160 caractères.",
            );
        }
        issues
    }
}

pub fn validate_service_state_transition(
    current: ServiceState,
    requested: ServiceState,
) -> Result<(), FleetTransitionError> {
    if current == requested {
        return Err(transition_error(
            "service_state_unchanged",
            "L'exemplaire est déjà dans cet état de service.",
            service_state_code(current),
            service_state_code(requested),
        ));
    }
    if current == ServiceState::Retired {
        return Err(transition_error(
            "retired_asset_service_state_is_terminal",
            "Un exemplaire retiré du parc ne peut pas être remis en service.",
            service_state_code(current),
            service_state_code(requested),
        ));
    }
    Ok(())
}

pub fn validate_administrative_availability_transition(
    service_state: ServiceState,
    current: AdministrativeAvailability,
    requested: AdministrativeAvailability,
) -> Result<(), FleetTransitionError> {
    if current == requested {
        return Err(transition_error(
            "administrative_availability_unchanged",
            "L'exemplaire possède déjà cette disponibilité administrative.",
            administrative_availability_code(current),
            administrative_availability_code(requested),
        ));
    }
    if matches!(
        service_state,
        ServiceState::InMaintenance | ServiceState::OutOfService | ServiceState::Retired
    ) && requested != AdministrativeAvailability::Unavailable
    {
        return Err(transition_error(
            "unserviceable_asset_cannot_be_available",
            "Un exemplaire en maintenance, hors service ou retiré ne peut pas être rendu disponible.",
            administrative_availability_code(current),
            administrative_availability_code(requested),
        ));
    }
    Ok(())
}

pub fn service_state_code(value: ServiceState) -> &'static str {
    match value {
        ServiceState::Usable => "usable",
        ServiceState::Restricted => "restricted",
        ServiceState::InMaintenance => "in_maintenance",
        ServiceState::OutOfService => "out_of_service",
        ServiceState::Retired => "retired",
    }
}

pub fn administrative_availability_code(value: AdministrativeAvailability) -> &'static str {
    match value {
        AdministrativeAvailability::Available => "available",
        AdministrativeAvailability::Unavailable => "unavailable",
    }
}

pub fn operational_usage_state_code(value: OperationalUsageState) -> &'static str {
    match value {
        OperationalUsageState::Available => "available",
        OperationalUsageState::Reserved => "reserved",
        OperationalUsageState::AssignedToSetup => "assigned_to_setup",
        OperationalUsageState::InTest => "in_test",
        OperationalUsageState::Unavailable => "unavailable",
    }
}

pub fn ownership_source_code(value: OwnershipSource) -> &'static str {
    match value {
        OwnershipSource::LaboratoryOwned => "laboratory_owned",
        OwnershipSource::CustomerSupplied => "customer_supplied",
        OwnershipSource::Rented => "rented",
        OwnershipSource::Borrowed => "borrowed",
        OwnershipSource::External => "external",
        OwnershipSource::SoftwareLicense => "software_license",
        OwnershipSource::InstalledFacility => "installed_facility",
    }
}

pub fn laboratory_location_status_code(value: LaboratoryLocationStatus) -> &'static str {
    match value {
        LaboratoryLocationStatus::Active => "active",
        LaboratoryLocationStatus::Archived => "archived",
    }
}

fn validate_pinned_model(issues: &mut Vec<FleetValidationIssue>, model: &PinnedEquipmentModel) {
    for (path, value) in [
        (
            "model.equipment_model_id",
            model.equipment_model_id.as_str(),
        ),
        (
            "model.equipment_model_revision_id",
            model.equipment_model_revision_id.as_str(),
        ),
        ("model.category_code", model.category_code.as_str()),
    ] {
        if !valid_stable_id(value) {
            push_issue(
                issues,
                "invalid_pinned_model_reference",
                path,
                "La référence de modèle constructeur n'est pas valide.",
            );
        }
    }
    if !valid_checksum(&model.equipment_model_checksum) {
        push_issue(
            issues,
            "invalid_pinned_model_checksum",
            "model.equipment_model_checksum",
            "L'empreinte de la version du modèle constructeur n'est pas valide.",
        );
    }
    if model.manufacturer.trim().is_empty() || model.model_name.trim().is_empty() {
        push_issue(
            issues,
            "pinned_model_snapshot_incomplete",
            "model",
            "L'instantané du modèle doit contenir le fabricant et le nom du modèle.",
        );
    }
    if model.category_path.is_empty()
        || model
            .category_path
            .iter()
            .any(|part| part.trim().is_empty())
    {
        push_issue(
            issues,
            "pinned_model_category_path_incomplete",
            "model.category_path",
            "Le chemin de catégorie lisible du modèle est obligatoire.",
        );
    }
}

fn validate_optional_text(
    issues: &mut Vec<FleetValidationIssue>,
    value: Option<&str>,
    path: &str,
    maximum_length: usize,
) {
    if let Some(value) = value {
        if value.trim().is_empty() || value.trim().chars().count() > maximum_length {
            push_issue(
                issues,
                "invalid_optional_text",
                path,
                "La valeur optionnelle doit être absente ou contenir un texte non vide de longueur valide.",
            );
        }
    }
}

fn valid_inventory_code(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value.chars().count() <= 80
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | '/')
        })
}

fn valid_stable_id(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value.chars().count() <= 160
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
}

fn valid_checksum(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .chars()
            .all(|character| character.is_ascii_hexdigit() && !character.is_ascii_uppercase())
}

fn push_issue(issues: &mut Vec<FleetValidationIssue>, code: &str, path: &str, message: &str) {
    issues.push(FleetValidationIssue {
        code: code.to_owned(),
        path: path.to_owned(),
        message: message.to_owned(),
    });
}

fn transition_error(
    code: &str,
    message: &str,
    current_state: &str,
    requested_state: &str,
) -> FleetTransitionError {
    FleetTransitionError {
        code: code.to_owned(),
        message: message.to_owned(),
        current_state: current_state.to_owned(),
        requested_state: requested_state.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model() -> PinnedEquipmentModel {
        PinnedEquipmentModel {
            equipment_model_id: "EQM-NRP6AN".to_owned(),
            equipment_model_revision_id: "EQM-NRP6AN-rev-0001".to_owned(),
            equipment_model_checksum: format!("sha256:{}", "a".repeat(64)),
            manufacturer: "Rohde & Schwarz".to_owned(),
            model_name: "NRP6AN".to_owned(),
            variant: None,
            category_code: "rf_power_meter".to_owned(),
            category_path: vec!["Mesure RF".to_owned(), "Wattmètres".to_owned()],
        }
    }

    fn asset(serial_number: Option<&str>) -> PhysicalAssetDefinition {
        PhysicalAssetDefinition {
            definition_schema_version: PHYSICAL_ASSET_DEFINITION_SCHEMA_VERSION.to_owned(),
            inventory_code: "INV-0042".to_owned(),
            serial_number: serial_number.map(str::to_owned),
            part_number: Some("NRP6AN".to_owned()),
            model: model(),
            laboratory_location_id: Some("LAB-CEM-1".to_owned()),
            laboratory_location_label: Some("Poste CEM 1".to_owned()),
            ownership_source: OwnershipSource::LaboratoryOwned,
            service_state: ServiceState::Usable,
            administrative_availability: AdministrativeAvailability::Available,
            administrative_unavailability_reason: String::new(),
            service_state_reason: String::new(),
            notes: String::new(),
        }
    }

    #[test]
    fn physical_asset_accepts_missing_serial_number() {
        assert!(asset(None).validate_all().is_empty());
    }

    #[test]
    fn physical_asset_keeps_exact_model_revision_and_checksum() {
        let asset = asset(Some("103456"));
        assert_eq!(
            asset.model.equipment_model_revision_id,
            "EQM-NRP6AN-rev-0001"
        );
        assert_eq!(asset.model.equipment_model_checksum.len(), 71);
        assert!(asset.validate_all().is_empty());
    }

    #[test]
    fn unavailable_service_states_require_unavailable_availability() {
        let mut asset = asset(None);
        asset.service_state = ServiceState::OutOfService;
        asset.service_state_reason = "Panne RF".to_owned();
        let issues = asset.validate_all();
        assert!(issues
            .iter()
            .any(|issue| issue.code == "service_state_requires_unavailable"));
    }

    #[test]
    fn retired_service_state_is_terminal() {
        let error = validate_service_state_transition(ServiceState::Retired, ServiceState::Usable)
            .unwrap_err();
        assert_eq!(error.code, "retired_asset_service_state_is_terminal");
    }

    #[test]
    fn out_of_service_asset_cannot_become_available() {
        let error = validate_administrative_availability_transition(
            ServiceState::OutOfService,
            AdministrativeAvailability::Unavailable,
            AdministrativeAvailability::Available,
        )
        .unwrap_err();
        assert_eq!(error.code, "unserviceable_asset_cannot_be_available");
    }

    #[test]
    fn laboratory_location_requires_a_readable_label() {
        let definition = LaboratoryLocationDefinition {
            definition_schema_version: LABORATORY_LOCATION_DEFINITION_SCHEMA_VERSION.to_owned(),
            label: " ".to_owned(),
            description: String::new(),
            status: LaboratoryLocationStatus::Active,
        };
        assert_eq!(
            definition.validate_all()[0].code,
            "invalid_laboratory_location_label"
        );
    }
}
