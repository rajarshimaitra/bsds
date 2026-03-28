//! WhatsApp Cloud API client for BSDS Dashboard.
//!
//! Uses the Meta Graph API (`graph.facebook.com/v18.0`) to send WhatsApp
//! messages via pre-approved Business templates.
//!
//! Design rules:
//!   - Never panics -- all errors returned as `WhatsAppSendResult`
//!   - Graceful skip if `WHATSAPP_API_TOKEN` or `WHATSAPP_PHONE_NUMBER_ID` are not set
//!   - Phone numbers normalised to E.164 `+91XXXXXXXXXX` for Indian numbers
//!
//! Configuration (environment variables):
//!   - `WHATSAPP_API_TOKEN`        -- Meta User or System User token
//!   - `WHATSAPP_PHONE_NUMBER_ID`  -- Business phone number ID

use reqwest::Client;
use serde::Serialize;
use std::env;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Result of a WhatsApp send operation. Never represents an unrecoverable error.
#[derive(Debug, Clone)]
pub struct WhatsAppSendResult {
    pub success: bool,
    pub message_id: Option<String>,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

const GRAPH_API_BASE: &str = "https://graph.facebook.com/v18.0";

/// Returns `true` when both required environment variables are set.
pub fn is_configured() -> bool {
    env::var("WHATSAPP_API_TOKEN")
        .ok()
        .filter(|s| !s.is_empty())
        .is_some()
        && env::var("WHATSAPP_PHONE_NUMBER_ID")
            .ok()
            .filter(|s| !s.is_empty())
            .is_some()
}

// ---------------------------------------------------------------------------
// Phone number formatting
// ---------------------------------------------------------------------------

/// Normalise a phone number to E.164 format with the `+91` country code.
///
/// Handles:
///   - `"+91XXXXXXXXXX"` -- returned as-is
///   - `"91XXXXXXXXXX"`  -- prepends `"+"`
///   - `"XXXXXXXXXX"`    -- prepends `"+91"`
///   - Non-digit characters stripped before processing
pub fn format_indian_phone(phone: &str) -> String {
    // Strip everything except digits and the leading +
    let stripped: String = phone.chars().filter(|&c| c.is_ascii_digit() || c == '+').collect();
    let digits: String = stripped.trim_start_matches('+').to_string();

    if digits.starts_with("91") && digits.len() == 12 {
        return format!("+{digits}");
    }

    if digits.len() == 10 {
        return format!("+91{digits}");
    }

    // Already has a country code that isn't 91, or some other format -- keep as-is
    if stripped.starts_with('+') {
        stripped
    } else {
        format!("+{stripped}")
    }
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// WhatsApp Cloud API client.
///
/// Holds a reusable `reqwest::Client` for connection pooling.
#[derive(Debug, Clone)]
pub struct WhatsappClient {
    phone_number_id: String,
    token: String,
    http: Client,
}

impl WhatsappClient {
    /// Create a new client from environment variables.
    /// Returns `None` if the required env vars are not set.
    pub fn from_env() -> Option<Self> {
        let token = env::var("WHATSAPP_API_TOKEN").ok().filter(|s| !s.is_empty())?;
        let phone_number_id = env::var("WHATSAPP_PHONE_NUMBER_ID")
            .ok()
            .filter(|s| !s.is_empty())?;

        Some(Self {
            phone_number_id,
            token,
            http: Client::new(),
        })
    }

    /// Send a WhatsApp template message.
    ///
    /// # Arguments
    /// * `to`              - Recipient phone number (will be normalised to `+91...`)
    /// * `template_name`   - Name of the pre-approved Meta Business template
    /// * `template_params` - Ordered list of body variable values
    /// * `language_code`   - BCP-47 language tag; defaults to `"en"`
    pub async fn send_message(
        &self,
        to: &str,
        template_name: &str,
        template_params: &[String],
        language_code: Option<&str>,
    ) -> WhatsAppSendResult {
        let recipient = format_indian_phone(to);
        let lang = language_code.unwrap_or("en");

        let components: Vec<TemplateComponent> = if template_params.is_empty() {
            vec![]
        } else {
            vec![TemplateComponent {
                r#type: "body".to_string(),
                parameters: template_params
                    .iter()
                    .map(|text| TemplateParam {
                        r#type: "text".to_string(),
                        text: text.clone(),
                    })
                    .collect(),
            }]
        };

        let body = SendTemplateBody {
            messaging_product: "whatsapp",
            to: &recipient,
            r#type: "template",
            template: TemplateBody {
                name: template_name,
                language: LanguageBody { code: lang },
                components,
            },
        };

        self.send_request(&body).await
    }

    /// Send a freeform text message (for testing / development only).
    ///
    /// Meta restricts freeform messages to within 24-hour customer-care windows.
    pub async fn send_text_message(&self, to: &str, text: &str) -> WhatsAppSendResult {
        let recipient = format_indian_phone(to);

        let body = SendTextBody {
            messaging_product: "whatsapp",
            to: &recipient,
            r#type: "text",
            text: TextContent {
                preview_url: false,
                body: text,
            },
        };

        self.send_request(&body).await
    }

    /// Internal helper to send a request to the Messages API.
    async fn send_request<T: Serialize>(&self, body: &T) -> WhatsAppSendResult {
        let url = format!("{}/{}/messages", GRAPH_API_BASE, self.phone_number_id);

        let resp = match self
            .http
            .post(&url)
            .bearer_auth(&self.token)
            .json(body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(err) => {
                let message = err.to_string();
                tracing::error!("[whatsapp] Network error: {message}");
                return WhatsAppSendResult {
                    success: false,
                    message_id: None,
                    error: Some(message),
                };
            }
        };

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let error_text = resp.text().await.unwrap_or_default();
            tracing::error!("[whatsapp] API error {status}: {error_text}");
            return WhatsAppSendResult {
                success: false,
                message_id: None,
                error: Some(format!("HTTP {status}: {error_text}")),
            };
        }

        let data: serde_json::Value = match resp.json().await {
            Ok(d) => d,
            Err(err) => {
                tracing::error!("[whatsapp] Failed to parse response: {err}");
                return WhatsAppSendResult {
                    success: true,
                    message_id: None,
                    error: None,
                };
            }
        };

        let message_id = data
            .get("messages")
            .and_then(|m| m.as_array())
            .and_then(|arr| arr.first())
            .and_then(|msg| msg.get("id"))
            .and_then(|id| id.as_str())
            .map(String::from);

        WhatsAppSendResult {
            success: true,
            message_id,
            error: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Request body types (private)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct SendTemplateBody<'a> {
    messaging_product: &'a str,
    to: &'a str,
    r#type: &'a str,
    template: TemplateBody<'a>,
}

#[derive(Serialize)]
struct TemplateBody<'a> {
    name: &'a str,
    language: LanguageBody<'a>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    components: Vec<TemplateComponent>,
}

#[derive(Serialize)]
struct LanguageBody<'a> {
    code: &'a str,
}

#[derive(Serialize)]
struct TemplateComponent {
    r#type: String,
    parameters: Vec<TemplateParam>,
}

#[derive(Serialize)]
struct TemplateParam {
    r#type: String,
    text: String,
}

#[derive(Serialize)]
struct SendTextBody<'a> {
    messaging_product: &'a str,
    to: &'a str,
    r#type: &'a str,
    text: TextContent<'a>,
}

#[derive(Serialize)]
struct TextContent<'a> {
    preview_url: bool,
    body: &'a str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_indian_phone_already_correct() {
        assert_eq!(format_indian_phone("+919830012345"), "+919830012345");
    }

    #[test]
    fn test_format_indian_phone_without_plus() {
        assert_eq!(format_indian_phone("919830012345"), "+919830012345");
    }

    #[test]
    fn test_format_indian_phone_10_digits() {
        assert_eq!(format_indian_phone("9830012345"), "+919830012345");
    }

    #[test]
    fn test_format_indian_phone_with_spaces() {
        assert_eq!(format_indian_phone("+91 98300 12345"), "+919830012345");
    }

    #[test]
    fn test_format_indian_phone_other_country() {
        // A non-91 country code should be kept as-is (with +)
        assert_eq!(format_indian_phone("441234567890"), "+441234567890");
    }
}
