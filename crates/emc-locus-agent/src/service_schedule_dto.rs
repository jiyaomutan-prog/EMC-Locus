use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ServiceScheduleItemDto {
    pub(crate) item_code: String,
    pub(crate) project_code: String,
    pub(crate) title: String,
    pub(crate) test_category_code: Option<String>,
    pub(crate) test_method_code: Option<String>,
    pub(crate) planned_start_at: String,
    pub(crate) planned_end_at: String,
    pub(crate) assigned_operator: String,
    pub(crate) location: String,
    pub(crate) equipment_under_test: String,
    pub(crate) status: String,
    pub(crate) notes: String,
    pub(crate) revision: u64,
    pub(crate) created_by: String,
    pub(crate) updated_by: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    pub(crate) available_transitions: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ServiceScheduleListDto {
    pub(crate) project_code: String,
    pub(crate) schedule_items: Vec<ServiceScheduleItemDto>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ServiceScheduleOperationResultDto {
    pub(crate) operation: String,
    pub(crate) operation_id: String,
    pub(crate) replayed: bool,
    pub(crate) schedule_item: ServiceScheduleItemDto,
}
