use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Request DTOs ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Clone)]
pub struct SlotInput {
    pub start_time: String, // "HH:MM"
    pub end_time: String,   // "HH:MM"
    pub capacity: i32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ClassroomSlotsInput {
    pub classroom_id: Uuid,
    pub slots: Vec<SlotInput>,
}

#[derive(Debug, Deserialize)]
pub struct CreateConferenceRequest {
    pub school_id: Uuid,
    pub school_subdomain: Option<String>,
    pub title: String,
    pub description: String,
    pub conference_date: NaiveDate,
    pub image_url: Option<String>,
    pub image_data: Option<String>,
    pub booking_close_at: Option<DateTime<Utc>>,
    pub classrooms: Vec<ClassroomSlotsInput>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateConferenceRequest {
    pub id: Uuid,
    pub title: Option<String>,
    pub description: Option<String>,
    pub conference_date: Option<NaiveDate>,
    pub image_url: Option<String>,
    pub booking_close_at: Option<DateTime<Utc>>,
    pub classrooms: Option<Vec<ClassroomSlotsInput>>,
}

#[derive(Debug, Deserialize)]
pub struct ConferenceListQuery {
    pub school_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct ConferenceSlotsQuery {
    pub classroom_id: Uuid,
    pub child_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct MyConferencesQuery {
    pub school_id: Uuid,
    pub child_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct MyBookingsQuery {
    pub school_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct BookSlotRequest {
    pub conference_id: Uuid,
    pub slot_id: Uuid,
    pub child_id: Uuid,
    pub school_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBookingRequest {
    pub slot_id: Uuid,
}

// ── Response DTOs ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Clone)]
pub struct ClassroomTag {
    pub classroom_id: Uuid,
    pub classroom_name: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct SlotDetail {
    pub id: Uuid,
    pub start_time: String,
    pub end_time: String,
    pub capacity: i32,
    pub booked_count: i64,
    pub is_available: bool,
    pub my_booking_id: Option<Uuid>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ClassroomSlotsResponse {
    pub classroom_id: Uuid,
    pub classroom_name: String,
    pub slots: Vec<SlotDetail>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ConferenceListItem {
    pub id: Uuid,
    pub school_id: Uuid,
    pub title: String,
    pub description: String,
    pub conference_date: NaiveDate,
    pub image_url: Option<String>,
    pub booking_close_at: Option<DateTime<Utc>>,
    pub status: String, // "active" | "expired"
    pub classroom_tags: Vec<ClassroomTag>,
    pub total_slots: i64,
    pub booked_slots: i64,
    pub available_slots: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ConferenceDetail {
    #[serde(flatten)]
    pub conference: ConferenceListItem,
    pub classrooms_detail: Vec<ClassroomSlotsResponse>,
}

#[derive(Debug, Serialize)]
pub struct BookingListItem {
    pub id: Uuid,
    pub conference_id: Uuid,
    pub conference_title: String,
    pub conference_date: NaiveDate,
    pub image_url: Option<String>,
    pub slot_id: Uuid,
    pub slot_start: String,
    pub slot_end: String,
    pub child_id: Uuid,
    pub child_name: String,
    pub classroom_id: Uuid,
    pub classroom_name: String,
    pub booked_at: DateTime<Utc>,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct BookingInfo {
    pub booking_id: Uuid,
    pub parent_name: String,
    pub parent_email: String,
    pub child_id: Uuid,
    pub child_name: String,
    pub booked_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Clone)]
pub struct UnbookedParent {
    pub parent_id: Uuid,
    pub parent_name: String,
    pub parent_email: String,
    pub child_id: Uuid,
    pub child_name: String,
}

#[derive(Debug, Serialize)]
pub struct SlotReport {
    pub slot_id: Uuid,
    pub start_time: String,
    pub end_time: String,
    pub capacity: i32,
    pub booked_count: i64,
    pub available: i64,
    pub bookings: Vec<BookingInfo>,
}

#[derive(Debug, Serialize)]
pub struct ClassroomReport {
    pub classroom_id: Uuid,
    pub classroom_name: String,
    pub student_count: i64,
    pub unbooked_count: i64,
    pub slots: Vec<SlotReport>,
    pub unbooked_parents: Vec<UnbookedParent>,
}

#[derive(Debug, Serialize)]
pub struct ConferenceReportResponse {
    pub conference: ConferenceListItem,
    pub report: Vec<ClassroomReport>,
}

#[derive(Debug, Serialize)]
pub struct ConferenceStats {
    pub total_conferences: i64,
    pub total_slots: i64,
    pub booked_slots: i64,
    pub available_slots: i64,
}
