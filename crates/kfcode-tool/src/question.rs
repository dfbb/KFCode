//! Tool for prompting the user with interactive questions during agent execution.
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, Write};

use crate::{Tool, ToolContext, ToolError, ToolResult};

/// Presents one or more questions to the user and collects their answers.
pub struct QuestionTool;

impl QuestionTool {
    /// Creates a new `QuestionTool`.
    pub fn new() -> Self {
        Self
    }
}

/// Deserialized input containing the list of questions to ask.
#[derive(Debug, Serialize, Deserialize)]
struct QuestionInput {
    #[serde(rename = "questions")]
    questions: Vec<QuestionDef>,
}

/// Definition of a single question, including optional multiple-choice options.
#[derive(Debug, Serialize, Deserialize)]
struct QuestionDef {
    #[serde(rename = "question")]
    question: String,
    #[serde(rename = "header")]
    header: Option<String>,
    #[serde(rename = "options", default)]
    options: Vec<QuestionOption>,
    #[serde(rename = "multiple", default)]
    multiple: bool,
}

/// A single selectable option within a question.
#[derive(Debug, Serialize, Deserialize)]
struct QuestionOption {
    #[serde(rename = "label")]
    label: String,
    #[serde(rename = "description", default)]
    description: Option<String>,
}

/// Collected answers returned after all questions have been answered.
#[derive(Debug, Serialize, Deserialize)]
struct QuestionResponse {
    answers: Vec<String>,
}

#[async_trait]
impl Tool for QuestionTool {
    fn id(&self) -> &str {
        "question"
    }

    fn description(&self) -> &str {
        "Ask the user clarifying questions during execution. Use to gather preferences, clarify ambiguous requests, or get decisions on implementation choices."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "questions": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "question": {
                                "type": "string",
                                "description": "The complete question to ask"
                            },
                            "header": {
                                "type": "string",
                                "description": "Short label for the question (max 30 chars)"
                            },
                            "multiple": {
                                "type": "boolean",
                                "default": false,
                                "description": "Allow selecting multiple options"
                            },
                            "options": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "label": {"type": "string"},
                                        "description": {"type": "string"}
                                    },
                                    "required": ["label"]
                                },
                                "description": "Available choices for the user"
                            }
                        },
                        "required": ["question"]
                    }
                }
            },
            "required": ["questions"]
        })
    }

    async fn execute(
        &self,
        args: serde_json::Value,
        _ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let input: QuestionInput =
            serde_json::from_value(args).map_err(|e| ToolError::InvalidArguments(e.to_string()))?;

        let mut all_answers: Vec<String> = Vec::new();

        for q in input.questions.iter() {
            let answer = ask_question(q)?;
            all_answers.extend(answer);
        }

        let response = QuestionResponse {
            answers: all_answers,
        };

        let output = serde_json::to_string_pretty(&response)
            .unwrap_or_else(|_| format!("{:?}", response.answers));

        Ok(ToolResult {
            title: "User response received".to_string(),
            output,
            metadata: std::collections::HashMap::new(),
            truncated: false,
        })
    }
}

fn ask_question(q: &QuestionDef) -> Result<Vec<String>, ToolError> {
    println!();

    if let Some(ref header) = q.header {
        println!("┌─ {} ─────────────────", header);
    } else {
        println!("┌─ Question ─────────────────");
    }
    println!("│");
    println!("│ {}", q.question);
    println!("│");

    if q.options.is_empty() {
        println!("└─ Type your answer: ");
        print!("> ");
        io::stdout()
            .flush()
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        let stdin = io::stdin();
        let mut answer = String::new();
        stdin
            .lock()
            .read_line(&mut answer)
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        return Ok(vec![answer.trim().to_string()]);
    }

    println!("│ Options:");
    for (i, opt) in q.options.iter().enumerate() {
        let num = i + 1;
        if let Some(ref desc) = opt.description {
            println!("│   {}. {} - {}", num, opt.label, desc);
        } else {
            println!("│   {}. {}", num, opt.label);
        }
    }
    println!("│");

    if q.multiple {
        println!("└─ Enter choices (comma-separated, e.g., 1,3): ");
    } else {
        println!("└─ Enter your choice: ");
    }

    print!("> ");
    io::stdout()
        .flush()
        .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

    let stdin = io::stdin();
    let mut input = String::new();
    stdin
        .lock()
        .read_line(&mut input)
        .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

    let input = input.trim();
    let answers: Vec<String> = input
        .split(',')
        .filter_map(|s| {
            let s = s.trim();
            if s.is_empty() {
                return None;
            }

            if let Ok(num) = s.parse::<usize>() {
                if num > 0 && num <= q.options.len() {
                    return Some(q.options[num - 1].label.clone());
                }
            }

            Some(s.to_string())
        })
        .collect();

    if answers.is_empty() && !q.options.is_empty() {
        return Ok(vec![q.options[0].label.clone()]);
    }

    Ok(answers)
}

impl Default for QuestionTool {
    /// Returns a default `QuestionTool` instance.
    fn default() -> Self {
        Self::new()
    }
}
