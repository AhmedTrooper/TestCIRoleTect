# 📊 Model & Data Card — RoleTect

This document details the AI models, data policies, and ethical/technical considerations for the **RoleTect** submission to the **SciBlitz AI Challenge 2026**.

---

## 🧠 PART 1: Model Card

### 1. Model Descriptions & Attributions
RoleTect is model-agnostic and interfaces with several pre-trained foundation models depending on user preferences and API key configurations:

| Model Name | Provider / Creator | License | Primary Intended Use in RoleTect |
| :--- | :--- | :--- | :--- |
| **Gemini 1.5 Flash / Pro** | Google | Google Terms of Service | Core resume/cover letter tailoring, structured job schema parsing. |
| **GPT-4o / GPT-4o-mini** | OpenAI | OpenAI Terms of Use | Alternate fallback models for job details extraction. |
| **Claude 3.5 Sonnet** | Anthropic | Anthropic Terms of Service | Precision self-healing LaTeX debugging and grammar correction. |
| **Llama 3 (8B / 70B)** | Meta | Meta Llama 3 License | Completely offline, local job processing and document tailoring via Ollama. |

### 2. Intended Domain & Use Cases
*   **Job Description Extraction:** Parsing unstructured webpage text (scraped via browser DOM) and mapping it into structured JSON objects (job titles, requirements, responsibilities).
*   **Contextual Document Synthesis:** Restructuring user resume bullet points to align with job description requirements while maintaining standard LaTeX compile structures.
*   **Syntax Correction:** Intercepting LaTeX compiler console logs, identifying compile errors (e.g., unescaped characters, missing packages), and generating corrected code blocks.

### 3. Out-of-Scope Uses
*   Generating fake qualifications, credentials, or work experience.
*   Automated bulk-spamming of job boards (designed for single, intentional applications).

---

## 📋 PART 2: Data Card

### 1. Data Collection & Privacy Boundary
RoleTect operates under a **local-first data sovereignty** design:
*   **Local Storage:** Job descriptions, application statuses, master resume files, and tailored documents are stored in an offline **SQLite database** on the user's physical machine.
*   **Vault Storage:** API keys are encrypted via an Argon2 passkey and stored inside a local **IOTA Stronghold** software enclave.
*   **No Central Cloud Server:** There is no central application server. Application histories and credentials are never stored, tracked, or aggregated by RoleTect.

### 2. Data Transmission
During inference (AI parsing or tailoring):
*   Data is routed directly from the Tauri client to the respective provider's endpoint (e.g., Google AI Studio, OpenAI API) via HTTPS.
*   Users running **Ollama** execute all models locally on their native GPU/CPU loop, resulting in **zero network egress** for data processing.

---

## ⚖️ PART 3: Ethical & Technical Considerations

### 1. Known Limitations & Edge Cases
*   **Model Hallucinations:** Large Language Models can hallucinate skills or metrics. RoleTect implements side-by-side LaTeX and visual PDF previews so candidates can manually audit all generated text before submission.
*   **LaTeX Compile Failure:** Complex templates may utilize unsupported packages. The local Tectonic compiler catches these and passes them to the self-healing debug agent, though manual correction is supported as a fallback.

### 2. Fairness & Bias Mitigation
Job boards often inject age, gender, or demographic bias. RoleTect's extraction prompt forces the AI model to ignore demographic details and focus exclusively on hard skills, requirements, and responsibilities, creating an objective resume match.
