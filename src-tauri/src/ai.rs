use rig::client::CompletionClient;
use rig::completion::Prompt;
use rig::providers::{anthropic, gemini, groq, openai, ollama, openrouter, deepseek};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

fn configure_aws_credentials(api_key: &str) {
    let api_key = api_key.trim();
    if api_key.is_empty() || api_key == "bedrock_env_auth" {
        return;
    }

    let parts: Vec<&str> = api_key.split(':').collect();
    if parts.len() >= 2 {
        std::env::set_var("AWS_ACCESS_KEY_ID", parts[0].trim());
        std::env::set_var("AWS_SECRET_ACCESS_KEY", parts[1].trim());
        if parts.len() >= 3 {
            std::env::set_var("AWS_REGION", parts[2].trim());
            std::env::set_var("AWS_DEFAULT_REGION", parts[2].trim());
        } else if std::env::var("AWS_REGION").is_err() && std::env::var("AWS_DEFAULT_REGION").is_err() {
            std::env::set_var("AWS_REGION", "us-east-1");
            std::env::set_var("AWS_DEFAULT_REGION", "us-east-1");
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct JobDetails {
    pub is_valid_job: bool,      // AI will set this to false if the content is not a job description
    pub job_title: String,
    pub company_name: String,
    pub work_model: String,      // Remote, Hybrid, On-site, Other
    pub employment_type: String, // Full-time, Part-time, Contract, Freelance, Temporary, Internship
    pub requirements: Vec<String>,
    pub core_responsibilities: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JobParseResult {
    pub details: JobDetails,
    pub raw_description: String,
}

pub async fn parse_job_description(
    provider: &str,
    model: &str,
    api_key: &str,
    custom_base_url: Option<&str>,
    raw_jd: &str,
    job_url: Option<&str>,
) -> Result<JobParseResult, String> {
    let input_text = raw_jd.trim();
    let url = job_url.unwrap_or("").trim();

    if input_text.is_empty() && url.is_empty() {
        return Err("Either a job description or a URL must be provided.".to_string());
    }

    let model = model.trim();
    
    // System prompt explaining the dual capability
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
        "openrouter" => {
            let is_custom = custom_base_url.map_or(false, |u| !u.trim().is_empty());
            let client = match custom_base_url {
                Some(url) if !url.trim().is_empty() => {
                    openrouter::Client::builder().api_key(api_key).base_url(url).build().map_err(|e| e.to_string())?
                }
                _ => openrouter::Client::new(api_key).map_err(|e| e.to_string())?,
            };
            let mut builder = client.extractor::<JobDetails>(model).preamble(system_prompt);
            if is_custom { builder = builder.max_tokens(131072); }
            let extractor = builder.build();
            extractor
                .extract(&user_prompt)
                .await
                .map_err(|e| format!("OpenRouter Parsing Error: {}", e))?
        }
        "openai" => {
            let is_custom = custom_base_url.map_or(false, |u| !u.trim().is_empty());
            let client = match custom_base_url {
                Some(url) if !url.trim().is_empty() => {
                    openai::Client::builder().api_key(api_key).base_url(url).build().map_err(|e| e.to_string())?
                }
                _ => openai::Client::new(api_key).map_err(|e| e.to_string())?,
            };
            let mut builder = client.extractor::<JobDetails>(model).preamble(system_prompt);
            if is_custom { builder = builder.max_tokens(131072); }
            let extractor = builder.build();
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
            let is_custom = custom_base_url.map_or(false, |u| !u.trim().is_empty());
            let client = match custom_base_url {
                Some(url) if !url.trim().is_empty() => {
                    anthropic::Client::builder().api_key(api_key).base_url(url).build().map_err(|e| e.to_string())?
                }
                _ => anthropic::Client::new(api_key).map_err(|e| e.to_string())?,
            };
            let mut builder = client.extractor::<JobDetails>(model).preamble(system_prompt);
            if is_custom { builder = builder.max_tokens(131072); }
            let extractor = builder.build();
            extractor
                .extract(&user_prompt)
                .await
                .map_err(|e| format!("Anthropic Parsing Error: {}", e))?
        }
        "bedrock" => {
            configure_aws_credentials(api_key);
            let config = aws_config::load_from_env().await;
            let bedrock_client = aws_sdk_bedrockruntime::Client::new(&config);
            let client = rig_bedrock::client::Client::from(bedrock_client);
            let extractor = client.extractor::<JobDetails>(model).preamble(system_prompt).build();
            extractor
                .extract(&user_prompt)
                .await
                .map_err(|e| format!("Bedrock AI Parsing Error: {}", e))?
        }
        "ollama" => {
            let client = match custom_base_url {
                Some(url) if !url.trim().is_empty() => {
                    ollama::Client::builder()
                        .api_key(ollama::OllamaApiKey::default())
                        .base_url(url)
                        .build()
                        .map_err(|e| e.to_string())?
                }
                _ => ollama::Client::new(ollama::OllamaApiKey::default()).map_err(|e| e.to_string())?,
            };
            let extractor = client.extractor::<JobDetails>(model).preamble(system_prompt).build();
            extractor
                .extract(&user_prompt)
                .await
                .map_err(|e| format!("Ollama Parsing Error: {}", e))?
        }
        "deepseek" => {
            let client = deepseek::Client::new(api_key).map_err(|e| e.to_string())?;
            let extractor = client.extractor::<JobDetails>(model).preamble(system_prompt).build();
            extractor
                .extract(&user_prompt)
                .await
                .map_err(|e| format!("DeepSeek Parsing Error: {}", e))?
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

pub async fn tailor_latex_for_job(
    provider: &str,
    model: &str,
    api_key: &str,
    custom_base_url: Option<&str>,
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
        "openrouter" => {
            let is_custom = custom_base_url.map_or(false, |u| !u.trim().is_empty());
            let client = match custom_base_url {
                Some(url) if !url.trim().is_empty() => {
                    openrouter::Client::builder().api_key(api_key).base_url(url).build().map_err(|e| e.to_string())?
                }
                _ => openrouter::Client::new(api_key).map_err(|e| e.to_string())?,
            };
            let mut builder = client.agent(model).preamble(system_prompt);
            if is_custom { builder = builder.max_tokens(131072); }
            let agent = builder.build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("OpenRouter Tailoring Error: {}", e))
        }
        "openai" => {
            let is_custom = custom_base_url.map_or(false, |u| !u.trim().is_empty());
            let client = match custom_base_url {
                Some(url) if !url.trim().is_empty() => {
                    openai::Client::builder().api_key(api_key).base_url(url).build().map_err(|e| e.to_string())?
                }
                _ => openai::Client::new(api_key).map_err(|e| e.to_string())?,
            };
            let mut builder = client.agent(model).preamble(system_prompt);
            if is_custom { builder = builder.max_tokens(131072); }
            let agent = builder.build();
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
            let is_custom = custom_base_url.map_or(false, |u| !u.trim().is_empty());
            let client = match custom_base_url {
                Some(url) if !url.trim().is_empty() => {
                    anthropic::Client::builder().api_key(api_key).base_url(url).build().map_err(|e| e.to_string())?
                }
                _ => anthropic::Client::new(api_key).map_err(|e| e.to_string())?,
            };
            let mut builder = client.agent(model).preamble(system_prompt);
            if is_custom { builder = builder.max_tokens(131072); }
            let agent = builder.build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("Anthropic Tailoring Error: {}", e))
        }
        "bedrock" => {
            configure_aws_credentials(api_key);
            let config = aws_config::load_from_env().await;
            let bedrock_client = aws_sdk_bedrockruntime::Client::new(&config);
            let client = rig_bedrock::client::Client::from(bedrock_client);
            let agent = client.agent(model).preamble(system_prompt).build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("Bedrock AI Tailoring Error: {}", e))
        }
        "ollama" => {
            let client = match custom_base_url {
                Some(url) if !url.trim().is_empty() => {
                    ollama::Client::builder()
                        .api_key(ollama::OllamaApiKey::default())
                        .base_url(url)
                        .build()
                        .map_err(|e| e.to_string())?
                }
                _ => ollama::Client::new(ollama::OllamaApiKey::default()).map_err(|e| e.to_string())?,
            };
            let agent = client.agent(model).preamble(system_prompt).build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("Ollama Tailoring Error: {}", e))
        }
        "deepseek" => {
            let client = deepseek::Client::new(api_key).map_err(|e| e.to_string())?;
            let agent = client.agent(model).preamble(system_prompt).build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("DeepSeek Tailoring Error: {}", e))
        }
        _ => Err(format!("Unsupported provider: {}", provider)),
    }
}

pub async fn tailor_latex_for_cover_letter(
    provider: &str,
    model: &str,
    api_key: &str,
    custom_base_url: Option<&str>,
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
        "openrouter" => {
            let is_custom = custom_base_url.map_or(false, |u| !u.trim().is_empty());
            let client = match custom_base_url {
                Some(url) if !url.trim().is_empty() => {
                    openrouter::Client::builder().api_key(api_key).base_url(url).build().map_err(|e| e.to_string())?
                }
                _ => openrouter::Client::new(api_key).map_err(|e| e.to_string())?,
            };
            let mut builder = client.agent(model).preamble(system_prompt);
            if is_custom { builder = builder.max_tokens(131072); }
            let agent = builder.build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("OpenRouter Tailoring Error: {}", e))
        }
        "openai" => {
            let is_custom = custom_base_url.map_or(false, |u| !u.trim().is_empty());
            let client = match custom_base_url {
                Some(url) if !url.trim().is_empty() => {
                    openai::Client::builder().api_key(api_key).base_url(url).build().map_err(|e| e.to_string())?
                }
                _ => openai::Client::new(api_key).map_err(|e| e.to_string())?,
            };
            let mut builder = client.agent(model).preamble(system_prompt);
            if is_custom { builder = builder.max_tokens(131072); }
            let agent = builder.build();
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
            let is_custom = custom_base_url.map_or(false, |u| !u.trim().is_empty());
            let client = match custom_base_url {
                Some(url) if !url.trim().is_empty() => {
                    anthropic::Client::builder().api_key(api_key).base_url(url).build().map_err(|e| e.to_string())?
                }
                _ => anthropic::Client::new(api_key).map_err(|e| e.to_string())?,
            };
            let mut builder = client.agent(model).preamble(system_prompt);
            if is_custom { builder = builder.max_tokens(131072); }
            let agent = builder.build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("Anthropic Tailoring Error: {}", e))
        }
        "bedrock" => {
            configure_aws_credentials(api_key);
            let config = aws_config::load_from_env().await;
            let bedrock_client = aws_sdk_bedrockruntime::Client::new(&config);
            let client = rig_bedrock::client::Client::from(bedrock_client);
            let agent = client.agent(model).preamble(system_prompt).build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("Bedrock AI Tailoring Error: {}", e))
        }
        "ollama" => {
            let client = match custom_base_url {
                Some(url) if !url.trim().is_empty() => {
                    ollama::Client::builder()
                        .api_key(ollama::OllamaApiKey::default())
                        .base_url(url)
                        .build()
                        .map_err(|e| e.to_string())?
                }
                _ => ollama::Client::new(ollama::OllamaApiKey::default()).map_err(|e| e.to_string())?,
            };
            let agent = client.agent(model).preamble(system_prompt).build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("Ollama Tailoring Error: {}", e))
        }
        "deepseek" => {
            let client = deepseek::Client::new(api_key).map_err(|e| e.to_string())?;
            let agent = client.agent(model).preamble(system_prompt).build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("DeepSeek Tailoring Error: {}", e))
        }
        _ => Err(format!("Unsupported provider: {}", provider)),
    }
}

pub async fn refine_tailored_resume(
    provider: &str,
    model: &str,
    api_key: &str,
    custom_base_url: Option<&str>,
    current_latex: &str,
    instruction: &str,
) -> Result<String, String> {
    let model = model.trim();
    let system_prompt = r#"You are an expert LaTeX resume editor. Your task is to take an EXISTING tailored resume and apply specific refinements or formatting changes as requested by the user.

Rules:
1. Preserve all existing content and structure unless specifically asked to change it.
2. Maintain valid LaTeX syntax at all times.
3. Output ONLY the modified LaTeX code with no markdown, no explanations, no code fences.
4. Ensure the output is a valid, compilable LaTeX document."#;

    let user_prompt = format!(
        r#"Current LaTeX Resume:
{}

Requested Refinement:
{}

Please apply the requested changes. Return only the updated LaTeX code."#,
        current_latex, instruction
    );

    match provider {
        "gemini" => {
            let client = gemini::Client::new(api_key).map_err(|e| e.to_string())?;
            let agent = client.agent(model).preamble(system_prompt).build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("Gemini AI Refinement Error: {}", e))
        }
        "openrouter" => {
            let is_custom = custom_base_url.map_or(false, |u| !u.trim().is_empty());
            let client = match custom_base_url {
                Some(url) if !url.trim().is_empty() => {
                    openrouter::Client::builder().api_key(api_key).base_url(url).build().map_err(|e| e.to_string())?
                }
                _ => openrouter::Client::new(api_key).map_err(|e| e.to_string())?,
            };
            let mut builder = client.agent(model).preamble(system_prompt);
            if is_custom { builder = builder.max_tokens(131072); }
            let agent = builder.build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("OpenRouter Refinement Error: {}", e))
        }
        "openai" => {
            let is_custom = custom_base_url.map_or(false, |u| !u.trim().is_empty());
            let client = match custom_base_url {
                Some(url) if !url.trim().is_empty() => {
                    openai::Client::builder().api_key(api_key).base_url(url).build().map_err(|e| e.to_string())?
                }
                _ => openai::Client::new(api_key).map_err(|e| e.to_string())?,
            };
            let mut builder = client.agent(model).preamble(system_prompt);
            if is_custom { builder = builder.max_tokens(131072); }
            let agent = builder.build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("OpenAI Refinement Error: {}", e))
        }
        "groq" => {
            let client = groq::Client::new(api_key).map_err(|e| e.to_string())?;
            let agent = client.agent(model).preamble(system_prompt).build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("Groq Refinement Error: {}", e))
        }
        "anthropic" => {
            let is_custom = custom_base_url.map_or(false, |u| !u.trim().is_empty());
            let client = match custom_base_url {
                Some(url) if !url.trim().is_empty() => {
                    anthropic::Client::builder().api_key(api_key).base_url(url).build().map_err(|e| e.to_string())?
                }
                _ => anthropic::Client::new(api_key).map_err(|e| e.to_string())?,
            };
            let mut builder = client.agent(model).preamble(system_prompt);
            if is_custom { builder = builder.max_tokens(131072); }
            let agent = builder.build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("Anthropic Refinement Error: {}", e))
        }
        "bedrock" => {
            configure_aws_credentials(api_key);
            let config = aws_config::load_from_env().await;
            let bedrock_client = aws_sdk_bedrockruntime::Client::new(&config);
            let client = rig_bedrock::client::Client::from(bedrock_client);
            let agent = client.agent(model).preamble(system_prompt).build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("Bedrock AI Refinement Error: {}", e))
        }
        "ollama" => {
            let client = match custom_base_url {
                Some(url) if !url.trim().is_empty() => {
                    ollama::Client::builder()
                        .api_key(ollama::OllamaApiKey::default())
                        .base_url(url)
                        .build()
                        .map_err(|e| e.to_string())?
                }
                _ => ollama::Client::new(ollama::OllamaApiKey::default()).map_err(|e| e.to_string())?,
            };
            let agent = client.agent(model).preamble(system_prompt).build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("Ollama Refinement Error: {}", e))
        }
        "deepseek" => {
            let client = deepseek::Client::new(api_key).map_err(|e| e.to_string())?;
            let agent = client.agent(model).preamble(system_prompt).build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("DeepSeek Refinement Error: {}", e))
        }
        _ => Err(format!("Unsupported provider: {}", provider)),
    }
}

pub async fn fix_latex_errors(
    provider: &str,
    model: &str,
    api_key: &str,
    custom_base_url: Option<&str>,
    broken_latex: &str,
    error_logs: &str,
) -> Result<String, String> {
    let model = model.trim();
    let system_prompt = r#"You are an expert LaTeX debugger. Your task is to fix syntax errors, missing packages, or illegal characters in LaTeX code based on provided error logs.

Rules:
1. Fix the specific errors mentioned in the logs.
2. DO NOT change the resume content or structure unless necessary to fix the error.
3. Output ONLY the corrected LaTeX code with no markdown, no explanations, no code fences.
4. Ensure the output is a valid, compilable LaTeX document."#;

    let user_prompt = format!(
        r#"Broken LaTeX Code:
{}

Tectonic Error Logs:
{}

Please fix the LaTeX code so it compiles successfully. Return only the fixed LaTeX code."#,
        broken_latex, error_logs
    );

    match provider {
        "gemini" => {
            let client = gemini::Client::new(api_key).map_err(|e| e.to_string())?;
            let agent = client.agent(model).preamble(system_prompt).build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("Gemini AI Fix Error: {}", e))
        }
        "openrouter" => {
            let is_custom = custom_base_url.map_or(false, |u| !u.trim().is_empty());
            let client = match custom_base_url {
                Some(url) if !url.trim().is_empty() => {
                    openrouter::Client::builder().api_key(api_key).base_url(url).build().map_err(|e| e.to_string())?
                }
                _ => openrouter::Client::new(api_key).map_err(|e| e.to_string())?,
            };
            let mut builder = client.agent(model).preamble(system_prompt);
            if is_custom { builder = builder.max_tokens(131072); }
            let agent = builder.build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("OpenRouter Fix Error: {}", e))
        }
        "openai" => {
            let is_custom = custom_base_url.map_or(false, |u| !u.trim().is_empty());
            let client = match custom_base_url {
                Some(url) if !url.trim().is_empty() => {
                    openai::Client::builder().api_key(api_key).base_url(url).build().map_err(|e| e.to_string())?
                }
                _ => openai::Client::new(api_key).map_err(|e| e.to_string())?,
            };
            let mut builder = client.agent(model).preamble(system_prompt);
            if is_custom { builder = builder.max_tokens(131072); }
            let agent = builder.build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("OpenAI Fix Error: {}", e))
        }
        "groq" => {
            let client = groq::Client::new(api_key).map_err(|e| e.to_string())?;
            let agent = client.agent(model).preamble(system_prompt).build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("Groq Fix Error: {}", e))
        }
        "anthropic" => {
            let is_custom = custom_base_url.map_or(false, |u| !u.trim().is_empty());
            let client = match custom_base_url {
                Some(url) if !url.trim().is_empty() => {
                    anthropic::Client::builder().api_key(api_key).base_url(url).build().map_err(|e| e.to_string())?
                }
                _ => anthropic::Client::new(api_key).map_err(|e| e.to_string())?,
            };
            let mut builder = client.agent(model).preamble(system_prompt);
            if is_custom { builder = builder.max_tokens(131072); }
            let agent = builder.build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("Anthropic Fix Error: {}", e))
        }
        "bedrock" => {
            configure_aws_credentials(api_key);
            let config = aws_config::load_from_env().await;
            let bedrock_client = aws_sdk_bedrockruntime::Client::new(&config);
            let client = rig_bedrock::client::Client::from(bedrock_client);
            let agent = client.agent(model).preamble(system_prompt).build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("Bedrock AI Fix Error: {}", e))
        }
        "ollama" => {
            let client = match custom_base_url {
                Some(url) if !url.trim().is_empty() => {
                    ollama::Client::builder()
                        .api_key(ollama::OllamaApiKey::default())
                        .base_url(url)
                        .build()
                        .map_err(|e| e.to_string())?
                }
                _ => ollama::Client::new(ollama::OllamaApiKey::default()).map_err(|e| e.to_string())?,
            };
            let agent = client.agent(model).preamble(system_prompt).build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("Ollama Fix Error: {}", e))
        }
        "deepseek" => {
            let client = deepseek::Client::new(api_key).map_err(|e| e.to_string())?;
            let agent = client.agent(model).preamble(system_prompt).build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("DeepSeek Fix Error: {}", e))
        }
        _ => Err(format!("Unsupported provider: {}", provider)),
    }
}

pub async fn refine_technical_content(
    provider: &str,
    model: &str,
    api_key: &str,
    custom_base_url: Option<&str>,
    content: &str,
    instruction: &str,
    content_type: &str, // "Mermaid", "Markdown", "LaTeX"
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
        "openrouter" => {
            let is_custom = custom_base_url.map_or(false, |u| !u.trim().is_empty());
            let client = match custom_base_url {
                Some(url) if !url.trim().is_empty() => {
                    openrouter::Client::builder().api_key(api_key).base_url(url).build().map_err(|e| e.to_string())?
                }
                _ => openrouter::Client::new(api_key).map_err(|e| e.to_string())?,
            };
            let mut builder = client.agent(model).preamble(&system_prompt);
            if is_custom { builder = builder.max_tokens(131072); }
            let agent = builder.build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("OpenRouter Refinement Error: {}", e))
        }
        "openai" => {
            let is_custom = custom_base_url.map_or(false, |u| !u.trim().is_empty());
            let client = match custom_base_url {
                Some(url) if !url.trim().is_empty() => {
                    openai::Client::builder().api_key(api_key).base_url(url).build().map_err(|e| e.to_string())?
                }
                _ => openai::Client::new(api_key).map_err(|e| e.to_string())?,
            };
            let mut builder = client.agent(model).preamble(&system_prompt);
            if is_custom { builder = builder.max_tokens(131072); }
            let agent = builder.build();
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
            let is_custom = custom_base_url.map_or(false, |u| !u.trim().is_empty());
            let client = match custom_base_url {
                Some(url) if !url.trim().is_empty() => {
                    anthropic::Client::builder().api_key(api_key).base_url(url).build().map_err(|e| e.to_string())?
                }
                _ => anthropic::Client::new(api_key).map_err(|e| e.to_string())?,
            };
            let mut builder = client.agent(model).preamble(&system_prompt);
            if is_custom { builder = builder.max_tokens(131072); }
            let agent = builder.build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("Anthropic Refinement Error: {}", e))
        }
        "bedrock" => {
            configure_aws_credentials(api_key);
            let config = aws_config::load_from_env().await;
            let bedrock_client = aws_sdk_bedrockruntime::Client::new(&config);
            let client = rig_bedrock::client::Client::from(bedrock_client);
            let agent = client.agent(model).preamble(&system_prompt).build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("Bedrock AI Refinement Error: {}", e))
        }
        "ollama" => {
            let client = match custom_base_url {
                Some(url) if !url.trim().is_empty() => {
                    ollama::Client::builder()
                        .api_key(ollama::OllamaApiKey::default())
                        .base_url(url)
                        .build()
                        .map_err(|e| e.to_string())?
                }
                _ => ollama::Client::new(ollama::OllamaApiKey::default()).map_err(|e| e.to_string())?,
            };
            let agent = client.agent(model).preamble(&system_prompt).build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("Ollama Refinement Error: {}", e))
        }
        "deepseek" => {
            let client = deepseek::Client::new(api_key).map_err(|e| e.to_string())?;
            let agent = client.agent(model).preamble(&system_prompt).build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("DeepSeek Refinement Error: {}", e))
        }
        _ => Err(format!("Unsupported provider: {}", provider)),
    }
}

pub async fn fix_technical_errors(
    provider: &str,
    model: &str,
    api_key: &str,
    custom_base_url: Option<&str>,
    broken_content: &str,
    error_logs: &str,
    content_type: &str,
) -> Result<String, String> {
    let model = model.trim();
    let system_prompt = format!(
        r#"You are an expert technical debugger specializing in {}. Your task is to fix syntax errors or logic issues based on provided error logs.

Rules:
1. Fix the specific errors mentioned in the logs.
2. DO NOT change the core meaning unless necessary to fix the error.
3. Output ONLY the corrected {} code with no markdown, no explanations, no code fences.
4. Ensure the output is valid and renderable."#,
        content_type, content_type
    );

    let user_prompt = format!(
        r#"Broken {} Code:
{}

Error Logs:
{}

Please fix the code so it renders successfully. Return only the fixed code."#,
        content_type, broken_content, error_logs
    );

    match provider {
        "gemini" => {
            let client = gemini::Client::new(api_key).map_err(|e| e.to_string())?;
            let agent = client.agent(model).preamble(&system_prompt).build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("Gemini AI Fix Error: {}", e))
        }
        "openrouter" => {
            let is_custom = custom_base_url.map_or(false, |u| !u.trim().is_empty());
            let client = match custom_base_url {
                Some(url) if !url.trim().is_empty() => {
                    openrouter::Client::builder().api_key(api_key).base_url(url).build().map_err(|e| e.to_string())?
                }
                _ => openrouter::Client::new(api_key).map_err(|e| e.to_string())?,
            };
            let mut builder = client.agent(model).preamble(&system_prompt);
            if is_custom { builder = builder.max_tokens(131072); }
            let agent = builder.build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("OpenRouter Fix Error: {}", e))
        }
        "openai" => {
            let is_custom = custom_base_url.map_or(false, |u| !u.trim().is_empty());
            let client = match custom_base_url {
                Some(url) if !url.trim().is_empty() => {
                    openai::Client::builder().api_key(api_key).base_url(url).build().map_err(|e| e.to_string())?
                }
                _ => openai::Client::new(api_key).map_err(|e| e.to_string())?,
            };
            let mut builder = client.agent(model).preamble(&system_prompt);
            if is_custom { builder = builder.max_tokens(131072); }
            let agent = builder.build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("OpenAI Fix Error: {}", e))
        }
        "groq" => {
            let client = groq::Client::new(api_key).map_err(|e| e.to_string())?;
            let agent = client.agent(model).preamble(&system_prompt).build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("Groq Fix Error: {}", e))
        }
        "anthropic" => {
            let is_custom = custom_base_url.map_or(false, |u| !u.trim().is_empty());
            let client = match custom_base_url {
                Some(url) if !url.trim().is_empty() => {
                    anthropic::Client::builder().api_key(api_key).base_url(url).build().map_err(|e| e.to_string())?
                }
                _ => anthropic::Client::new(api_key).map_err(|e| e.to_string())?,
            };
            let mut builder = client.agent(model).preamble(&system_prompt);
            if is_custom { builder = builder.max_tokens(131072); }
            let agent = builder.build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("Anthropic Fix Error: {}", e))
        }
        "bedrock" => {
            configure_aws_credentials(api_key);
            let config = aws_config::load_from_env().await;
            let bedrock_client = aws_sdk_bedrockruntime::Client::new(&config);
            let client = rig_bedrock::client::Client::from(bedrock_client);
            let agent = client.agent(model).preamble(&system_prompt).build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("Bedrock AI Fix Error: {}", e))
        }
        "ollama" => {
            let client = match custom_base_url {
                Some(url) if !url.trim().is_empty() => {
                    ollama::Client::builder()
                        .api_key(ollama::OllamaApiKey::default())
                        .base_url(url)
                        .build()
                        .map_err(|e| e.to_string())?
                }
                _ => ollama::Client::new(ollama::OllamaApiKey::default()).map_err(|e| e.to_string())?,
            };
            let agent = client.agent(model).preamble(&system_prompt).build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("Ollama Fix Error: {}", e))
        }
        "deepseek" => {
            let client = deepseek::Client::new(api_key).map_err(|e| e.to_string())?;
            let agent = client.agent(model).preamble(&system_prompt).build();
            agent
                .prompt(&user_prompt)
                .await
                .map_err(|e| format!("DeepSeek Fix Error: {}", e))
        }
        _ => Err(format!("Unsupported provider: {}", provider)),
    }
}

pub async fn test_ai(
    provider: &str,
    model: &str,
    api_key: &str,
    custom_base_url: Option<&str>,
) -> Result<String, String> {
    let model = model.trim();
    let system_prompt = "You are a test agent. Respond ONLY with a valid JSON object containing a 'status' field with the value 'ok'. Do not include markdown code fences, formatting, or extra explanations.";
    let user_prompt = "Perform connection test. Respond in JSON.";

    match provider {
        "gemini" => {
            let client = gemini::Client::new(api_key).map_err(|e| e.to_string())?;
            let agent = client.agent(model).preamble(system_prompt).build();
            agent.prompt(user_prompt).await.map_err(|e| format!("Gemini AI Error: {}", e))
        }
        "openrouter" => {
            let client = match custom_base_url {
                Some(url) if !url.trim().is_empty() => {
                    openrouter::Client::builder().api_key(api_key).base_url(url).build().map_err(|e| e.to_string())?
                }
                _ => openrouter::Client::new(api_key).map_err(|e| e.to_string())?,
            };
            let agent = client.agent(model).preamble(system_prompt).build();
            agent.prompt(user_prompt).await.map_err(|e| format!("OpenRouter Error: {}", e))
        }
        "openai" => {
            let client = match custom_base_url {
                Some(url) if !url.trim().is_empty() => {
                    openai::Client::builder().api_key(api_key).base_url(url).build().map_err(|e| e.to_string())?
                }
                _ => openai::Client::new(api_key).map_err(|e| e.to_string())?,
            };
            let agent = client.agent(model).preamble(system_prompt).build();
            agent.prompt(user_prompt).await.map_err(|e| format!("OpenAI Error: {}", e))
        }
        "groq" => {
            let client = groq::Client::new(api_key).map_err(|e| e.to_string())?;
            let agent = client.agent(model).preamble(system_prompt).build();
            agent.prompt(user_prompt).await.map_err(|e| format!("Groq Error: {}", e))
        }
        "anthropic" => {
            let client = match custom_base_url {
                Some(url) if !url.trim().is_empty() => {
                    anthropic::Client::builder().api_key(api_key).base_url(url).build().map_err(|e| e.to_string())?
                }
                _ => anthropic::Client::new(api_key).map_err(|e| e.to_string())?,
            };
            let agent = client.agent(model).preamble(system_prompt).build();
            agent.prompt(user_prompt).await.map_err(|e| format!("Anthropic Error: {}", e))
        }
        "bedrock" => {
            configure_aws_credentials(api_key);
            let config = aws_config::load_from_env().await;
            let bedrock_client = aws_sdk_bedrockruntime::Client::new(&config);
            let client = rig_bedrock::client::Client::from(bedrock_client);
            let agent = client.agent(model).preamble(system_prompt).build();
            agent.prompt(user_prompt).await.map_err(|e| format!("Bedrock AI Error: {}", e))
        }
        "ollama" => {
            let client = match custom_base_url {
                Some(url) if !url.trim().is_empty() => {
                    ollama::Client::builder()
                        .api_key(ollama::OllamaApiKey::default())
                        .base_url(url)
                        .build()
                        .map_err(|e| e.to_string())?
                }
                _ => ollama::Client::new(ollama::OllamaApiKey::default()).map_err(|e| e.to_string())?,
            };
            let agent = client.agent(model).preamble(system_prompt).build();
            agent.prompt(user_prompt).await.map_err(|e| format!("Ollama Error: {}", e))
        }
        "deepseek" => {
            let client = deepseek::Client::new(api_key).map_err(|e| e.to_string())?;
            let agent = client.agent(model).preamble(system_prompt).build();
            agent.prompt(user_prompt).await.map_err(|e| format!("DeepSeek Error: {}", e))
        }
        _ => Err(format!("Unsupported provider: {}", provider)),
    }
}
