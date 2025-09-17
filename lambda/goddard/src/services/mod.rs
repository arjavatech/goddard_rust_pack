pub mod auth_service;
pub mod supabase_client;
pub mod school_service;
pub mod classroom_service;

pub use auth_service::AuthService;
pub use supabase_client::SupabaseClient;
pub use school_service::SchoolService;
pub use classroom_service::ClassroomService;