use rig::client::CompletionClient;
use rig::completion::Prompt;
use rig::providers::{anthropic, gemini, groq, openai};
use crate::models::{JobDetails, JobParseResult};

pub async fn parse_job_description(
    provider: &str,
    model: &str,
    api_key: &str,
    raw_jd: &str,
    job_url: Option<&str>,
) -> Result<JobParseResult, String> {
    let input_text = raw_jd.trim();
    let url = job_url.unwrap_or("").trim();

    if input_text.is_empty() && url.is_empty() {
        return Err("Either a job description or a URL must be provided.".to_string());
    }

    let model = model.trim();
    
    let system_prompt = "You are an expert job details extractor.
    
TASK:
- If a RAW DESCRIPTION is provided below, extract details from that text.
- If ONLY a URL is provided, crawl/fetch the content from that URL and extract details.
- If BOTH are provided, PRIORITIZE the manual RAW DESCRIPTION for extraction.

VALIDATION:
- Be permissive: If the text looks like a job posting (even if short or partial), set 'is_valid_job' to true.
- ONLY set 'is_valid_job' to false if the content is clearly NOT a job (e.g., just a login page, cookie consent, or site navigation).
- Try your best to fullfill the requirements,responsibilities fileds even if the description is brief or incomplete.

Output the results in the requested structured format.";

    let user_prompt = if !input_text.is_empty() {
        format!("RAW DESCRIPTION:\n{}\n\n(Optional URL for reference: {})", input_text, url)
    } else {
        format!("PLEASE FETCH AND PARSE THIS URL: {}", url)
    };

    let details = match provider {
        "gemini" => {
            let client = gemini::Client::new(api_key).map_err(|e| e.to_string())?;
            let extractor = client.extractor::<JobDetails>(model).preamble(system_prompt).build();
            extractor
                .extract(&user_prompt)
                .await
                .map_err(|e| format!("Gemini AI Parsing Error: {}", e))?
        }
        "openai" => {
            let client = openai::Client::new(api_key).map_err(|e| e.to_string())?;
            let extractor = client.extractor::<JobDetails>(model).preamble(system_prompt).build();
            extractor
                .extract(&user_prompt)
                .await
                .map_err(|e| format!("OpenAI Parsing Error: {}", e))?
        }
        "groq" => {
            let client = groq::Client::new(api_key).map_err(|e| e.to_string())?;
            let extractor = client.extractor::<JobDetails>(model).preamble(system_prompt).build();
            extractor
                .extract(&user_prompt)
                .await
                .map_err(|e| format!("Groq Parsing Error: {}", e))?
        }
        "anthropic" => {
            let client = anthropic::Client::new(api_key).map_err(|e| e.to_string())?;
            let extractor = client.extractor::<JobDetails>(model).preamble(system_prompt).build();
            extractor
                .extract(&user_prompt)
                .await
                .map_err(|e| format!("Anthropic Parsing Error: {}", e))?
        }
        _ => return Err(format!("Unsupported provider: {}", provider)),
    };

    if !details.is_valid_job {
        return Err("The content provided (or the URL) does not appear to contain a valid job description. Please ensure the link is public or paste the description manually.".to_string());
    }

    Ok(JobParseResult {
        details,
        raw_description: if !input_text.is_empty() { input_text.to_string() } else { format!("Source URL: {}", url) },
    })
}

pub async fn refine_technical_content(
    provider: &str,
    model: &str,
    api_key: &str,
    content: &str,
    instruction: &str,
    content_type: &str,
) -> Result<String, String> {
    let model = model.trim();
    let system_prompt = format!(
        r#"You are an expert technical document editor specializing in {}. Your task is to apply specific refinements or formatting changes as requested by the user.

Rules:
1. Preserve all existing logic and meaning unless specifically asked to change it.
2. Maintain valid {} syntax at all times.
3. Output ONLY the modified code with no markdown, no explanations, no code fences.
4. Ensure the output is ready for rendering."#,
        content_type, content_type
    );

    let user_prompt = format!(
        r#"Current {} Content:
{}

Requested Refinement:
{}

Please apply the requested changes. Return only the updated code."#,
        content_type, content, instruction
    );

    match provider {
        "gemini" => {
            let client = gemini::Client::new(api_key).map_err(|e| e.to_string())?;
            let agent = client.agent(model).preamble(&system_prompt).build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("Gemini AI Refinement Error: {}", e))
        }
        "openai" => {
            let client = openai::Client::new(api_key).map_err(|e| e.to_string())?;
            let agent = client.agent(model).preamble(&system_prompt).build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("OpenAI Refinement Error: {}", e))
        }
        "groq" => {
            let client = groq::Client::new(api_key).map_err(|e| e.to_string())?;
            let agent = client.agent(model).preamble(&system_prompt).build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("Groq Refinement Error: {}", e))
        }
        "anthropic" => {
            let client = anthropic::Client::new(api_key).map_err(|e| e.to_string())?;
            let agent = client.agent(model).preamble(&system_prompt).build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("Anthropic Refinement Error: {}", e))
        }
        _ => Err(format!("Unsupported provider: {}", provider)),
    }
}


pub async fn tailor_latex_for_job(
    provider: &str,
    model: &str,
    api_key: &str,
    base_latex: &str,
    raw_job_content: &str,
    custom_instruction: Option<&str>,
) -> Result<String, String> {
    let model = model.trim();
    let system_prompt = r#"You are an expert resume tailoring AI. Your task is to take a base LaTeX resume template and tailor it to match a specific job description. 
    
Rules:
1. Only modify the resume content, NOT the structure or LaTeX commands
2. Highlight keywords and experiences that match the job description
3. Keep all original sections and formatting
4. Output ONLY valid LaTeX code with no markdown, no explanations, no code fences
5. Ensure the output is a valid, compilable LaTeX document

If custom instructions are provided, prioritize them."#;

    let user_prompt = format!(
        r#"Base LaTeX Resume:
{}

Job Description:
{}

{}

Please tailor the resume to match the job description. Return only the modified LaTeX code."#,
        base_latex,
        raw_job_content,
        custom_instruction
            .map(|ci| format!("Custom Instructions:\n{}", ci))
            .unwrap_or_default()
    );

    match provider {
        "gemini" => {
            let client = gemini::Client::new(api_key).map_err(|e| e.to_string())?;
            let agent = client.agent(model).preamble(system_prompt).build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("Gemini AI Tailoring Error: {}", e))
        }
        "openai" => {
            let client = openai::Client::new(api_key).map_err(|e| e.to_string())?;
            let agent = client.agent(model).preamble(system_prompt).build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("OpenAI Tailoring Error: {}", e))
        }
        "groq" => {
            let client = groq::Client::new(api_key).map_err(|e| e.to_string())?;
            let agent = client.agent(model).preamble(system_prompt).build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("Groq Tailoring Error: {}", e))
        }
        "anthropic" => {
            let client = anthropic::Client::new(api_key).map_err(|e| e.to_string())?;
            let agent = client.agent(model).preamble(system_prompt).build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("Anthropic Tailoring Error: {}", e))
        }
        _ => Err(format!("Unsupported provider: {}", provider)),
    }
}

pub async fn tailor_latex_for_cover_letter(
    provider: &str,
    model: &str,
    api_key: &str,
    base_latex: &str,
    raw_job_content: &str,
    custom_instruction: Option<&str>,
) -> Result<String, String> {
    let model = model.trim();
    let system_prompt = r#"You are an expert cover letter tailoring AI. Your task is to take a base LaTeX cover letter template and tailor it to match a specific job description. 
    
Rules:
1. Only modify the cover letter content (e.g., recipient info, body paragraphs), NOT the structure or LaTeX commands unless necessary for content.
2. Emphasize how the candidate's skills and experiences align with the job requirements.
3. Maintain a professional, persuasive, and concise tone.
4. Keep all original sections and formatting.
5. Output ONLY valid LaTeX code with no markdown, no explanations, no code fences.
6. Ensure the output is a valid, compilable LaTeX document.

If custom instructions are provided, prioritize them."#;

    let user_prompt = format!(
        r#"Base LaTeX Cover Letter:
{}

Job Description:
{}

{}

Please tailor the cover letter to match the job description. Return only the modified LaTeX code."#,
        base_latex,
        raw_job_content,
        custom_instruction
            .map(|ci| format!("Custom Instructions:\n{}", ci))
            .unwrap_or_default()
    );

    match provider {
        "gemini" => {
            let client = gemini::Client::new(api_key).map_err(|e| e.to_string())?;
            let agent = client.agent(model).preamble(system_prompt).build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("Gemini AI Tailoring Error: {}", e))
        }
        "openai" => {
            let client = openai::Client::new(api_key).map_err(|e| e.to_string())?;
            let agent = client.agent(model).preamble(system_prompt).build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("OpenAI Tailoring Error: {}", e))
        }
        "groq" => {
            let client = groq::Client::new(api_key).map_err(|e| e.to_string())?;
            let agent = client.agent(model).preamble(system_prompt).build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("Groq Tailoring Error: {}", e))
        }
        "anthropic" => {
            let client = anthropic::Client::new(api_key).map_err(|e| e.to_string())?;
            let agent = client.agent(model).preamble(system_prompt).build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("Anthropic Tailoring Error: {}", e))
        }
        _ => Err(format!("Unsupported provider: {}", provider)),
    }
}


