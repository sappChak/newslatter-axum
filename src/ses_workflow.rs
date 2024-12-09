use anyhow::anyhow;
use aws_sdk_sesv2::types::Body;
use aws_sdk_sesv2::types::Content;
use aws_sdk_sesv2::types::Destination;
use aws_sdk_sesv2::types::EmailContent;
use aws_sdk_sesv2::types::Message;
use aws_sdk_sesv2::Client;

use crate::domain::SubscriberEmail;

pub struct SESWorkflow {
    client: Client,
    verified_email: SubscriberEmail, // <-- Sender
}

impl SESWorkflow {
    pub fn new(client: Client, verified_email: String) -> Self {
        Self {
            client,
            verified_email: SubscriberEmail::parse(verified_email).expect("Invalid sender email"),
        }
    }

    pub async fn send_email(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), anyhow::Error> {
        let email_content = EmailContent::builder()
            .simple(
                Message::builder()
                    .subject(Content::builder().data(subject).build()?)
                    .body(
                        Body::builder()
                            .html(Content::builder().data(html_content).build()?)
                            .text(Content::builder().data(text_content).build()?)
                            .build(),
                    )
                    .build(),
            )
            .build();

        let res = self
            .client
            .send_email()
            .from_email_address(self.verified_email.as_ref())
            .destination(
                Destination::builder()
                    .to_addresses(recipient.as_ref())
                    .build(),
            )
            .content(email_content)
            .send()
            .await;

        match res {
            Ok(output) => {
                if let Some(_message_id) = output.message_id {
                    Ok(())
                } else {
                    Err(anyhow!("Message sent, but no message ID was returned"))
                }
            }
            Err(e) => Err(anyhow!(
                "Error sending welcome email to {}: {:?}",
                recipient.as_ref(),
                e
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use aws_sdk_sesv2::{
        operation::send_email::{SendEmailError, SendEmailOutput},
        types::error::MailFromDomainNotVerifiedException,
        Client,
    };
    use aws_smithy_mocks_experimental::{mock, mock_client, RuleMode};

    use crate::{domain::SubscriberEmail, ses_workflow::SESWorkflow};

    #[tokio::test]
    async fn send_email_successes() -> Result<()> {
        let mock_send_email = mock!(Client::send_email)
            .match_requests(|req| {
                req.destination()
                    .unwrap()
                    .to_addresses()
                    .contains(&"recipient@example.com".into())
            })
            .then_output(|| {
                SendEmailOutput::builder()
                    .message_id("newsletter-email")
                    .build()
            });

        let client = mock_client!(aws_sdk_sesv2, RuleMode::Sequential, [&mock_send_email]);

        let ses_workflow = SESWorkflow::new(client, "sender@example.com".to_string());

        let recipient = SubscriberEmail::parse("recipient@example.com".to_string()).unwrap();

        let result = ses_workflow
            .send_email(
                recipient,
                "Test Subject",
                "<p>Test HTML content</p>",
                "Test text content",
            )
            .await;

        assert!(result.is_ok());
        Ok(())
    }

    #[tokio::test]
    async fn test_send_email_fails() -> Result<()> {
        let mock_send_email = mock!(Client::send_email)
            .match_requests(|req| {
                req.destination()
                    .unwrap()
                    .to_addresses()
                    .contains(&"recipient@example.com".into())
            })
            .then_error(|| {
                SendEmailError::MailFromDomainNotVerifiedException(
                    MailFromDomainNotVerifiedException::builder().build(),
                )
            });

        let client = mock_client!(aws_sdk_sesv2, RuleMode::Sequential, [&mock_send_email]);

        let ses_workflow = SESWorkflow::new(client, "sender@example.com".to_string());

        let recipient = SubscriberEmail::parse("recipient@example.com".to_string()).unwrap();

        let result = ses_workflow
            .send_email(
                recipient,
                "Test Subject",
                "<p>Test HTML content</p>",
                "Test text content",
            )
            .await;

        // Check that the error is propagated
        assert!(result.is_err());
        assert!(
            format!("{result:?}").contains("Error sending welcome email to recipient@example.com:")
        );

        Ok(())
    }
}
