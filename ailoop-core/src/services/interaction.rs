//! Human interaction handling services

use crate::channel::ChannelIsolation;
use crate::models::*;
use crate::services::logging;
use anyhow::Result;

/// Human interaction service for handling questions and authorizations
pub struct InteractionService {
    channel_isolation: ChannelIsolation,
}

impl InteractionService {
    /// Create a new interaction service
    pub fn new(channel_isolation: ChannelIsolation) -> Self {
        Self { channel_isolation }
    }

    /// Handle a question interaction
    pub async fn handle_question(
        &self,
        question: String,
        channel: String,
        timeout_seconds: u32,
    ) -> Result<String> {
        logging::log_interaction("question_start", &channel, Some(&question));

        // Create question message
        let content = MessageContent::Question {
            text: question.clone(),
            timeout_seconds,
            choices: None,
        };

        let message = Message::new(channel.clone(), SenderType::Agent, content);

        // Queue the message
        self.channel_isolation.enqueue_message(&channel, message);

        // For now, return a placeholder response
        // In a real implementation, this would wait for human response
        let response = format!(
            "Question '{}' queued for channel '{}'. Awaiting human response...",
            question, channel
        );

        logging::log_interaction("question_queued", &channel, Some(&response));

        Ok(response)
    }

    /// Handle an authorization interaction
    pub async fn handle_authorization(
        &self,
        action: String,
        channel: String,
        timeout_seconds: u32,
    ) -> Result<String> {
        logging::log_interaction("authorization_start", &channel, Some(&action));

        // Create authorization message
        let content = MessageContent::Authorization {
            action: action.clone(),
            context: None,
            timeout_seconds,
        };

        let message = Message::new(channel.clone(), SenderType::Agent, content);

        // Queue the message
        self.channel_isolation.enqueue_message(&channel, message);

        // For now, return a placeholder response
        // In a real implementation, this would wait for human approval
        let response = format!(
            "Authorization request for '{}' queued for channel '{}'. Awaiting human approval...",
            action, channel
        );

        logging::log_interaction("authorization_queued", &channel, Some(&response));

        Ok(response)
    }

    /// Handle a notification
    pub fn handle_notification(
        &self,
        message: String,
        channel: String,
        priority: String,
    ) -> Result<String> {
        logging::log_interaction("notification_send", &channel, Some(&message));

        // Create notification message
        let content = MessageContent::Notification {
            text: message.clone(),
            priority: match priority.as_str() {
                "low" => NotificationPriority::Low,
                "high" => NotificationPriority::High,
                "urgent" => NotificationPriority::Urgent,
                _ => NotificationPriority::Normal,
            },
        };

        let msg = Message::new(channel.clone(), SenderType::Agent, content);

        // Queue the message
        self.channel_isolation.enqueue_message(&channel, msg);

        let response = format!(
            "Notification '{}' sent to channel '{}' with {} priority.",
            message, channel, priority
        );

        Ok(response)
    }

    /// Get channel statistics
    pub fn get_channel_stats(&self, channel: &str) -> (usize, usize) {
        let queue_size = self.channel_isolation.get_queue_size(channel);
        let connections = self.channel_isolation.get_connection_count(channel);
        (queue_size, connections)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channel::ChannelIsolation;

    #[tokio::test]
    async fn test_question_handling() {
        let isolation = ChannelIsolation::default();
        let service = InteractionService::new(isolation);

        let result = service
            .handle_question("Test question".to_string(), "test-channel".to_string(), 60)
            .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.contains("Test question"));
        assert!(response.contains("test-channel"));
    }

    #[tokio::test]
    async fn test_authorization_handling() {
        let isolation = ChannelIsolation::default();
        let service = InteractionService::new(isolation);

        let result = service
            .handle_authorization(
                "Deploy to production".to_string(),
                "admin-channel".to_string(),
                300,
            )
            .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.contains("Deploy to production"));
        assert!(response.contains("admin-channel"));
    }

    #[test]
    fn test_notification_handling() {
        let isolation = ChannelIsolation::default();
        let service = InteractionService::new(isolation);

        let result = service.handle_notification(
            "Build completed".to_string(),
            "team-channel".to_string(),
            "high".to_string(),
        );

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.contains("Build completed"));
        assert!(response.contains("team-channel"));
        assert!(response.contains("high"));
    }

    #[test]
    fn test_channel_stats() {
        let isolation = ChannelIsolation::default();
        let service = InteractionService::new(isolation);

        let (queue_size, connections) = service.get_channel_stats("nonexistent");
        assert_eq!(queue_size, 0);
        assert_eq!(connections, 0);
    }
}
