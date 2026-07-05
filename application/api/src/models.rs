use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TailoredContent {
    pub id: String,
    pub base_template_id: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct JobDetails {
    pub is_valid_job: bool,
    pub job_title: String,
    pub company_name: String,
    pub work_model: String,
    pub employment_type: String,
    pub requirements: Vec<String>,
    pub core_responsibilities: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JobParseResult {
    pub details: JobDetails,
    pub raw_description: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JobPayload {
    pub id: String,
    pub company_name: String,
    pub job_title: String,
    pub work_model: String,
    pub employment_type: String,
    pub status: String,
    pub raw_jd: String,
    pub requirements: Option<String>,
    pub core_responsibilities: Option<String>,
    pub custom_instruction: Option<String>,
    pub reference_name: Option<String>,
    pub reference_email: Option<String>,
    pub social_link: Option<String>,
    pub job_url: Option<String>,
    pub base_resume_id: Option<String>,
    pub base_cl_id: Option<String>,
    pub salary: Option<String>,
    pub applied_date: Option<String>,
    pub interview_date: Option<String>,
    pub offer_date: Option<String>,
    pub rejected_date: Option<String>,
    pub joining_date: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ResumeItem {
    pub id: String,
    pub name: String,
    pub category: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ResumeDetail {
    pub id: String,
    pub name: String,
    pub category: String,
    pub latex_content: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CoverLetterItem {
    pub id: String,
    pub name: String,
    pub category: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CoverLetterDetail {
    pub id: String,
    pub name: String,
    pub category: String,
    pub latex_content: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadRecord {
    pub id: String,
    pub filename: String,
    pub download_type: String,
    pub job_id: Option<String>,
    pub content_id: Option<String>,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InboxJob {
    pub id: String,
    pub url: Option<String>,
    pub raw_description: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Serialize, Deserialize)]
pub struct ExtensionConfig {
    pub secret: String,
    pub port: String,
}

#[derive(Serialize, Deserialize)]
pub struct AiConfig {
    pub provider: String,
    pub model: String,
    pub has_key: bool,
}
