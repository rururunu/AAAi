use std::sync::{Arc, Mutex};

use crate::core::ai::provider::AIProvider;
use crate::core::chat::compact::{self, DEFAULT_CONTEXT_WINDOW};
use crate::core::chat::conversation_manager::{create_message, ConversationManager};
use crate::core::chat::error::ChatError;
use crate::core::chat::preferences::SendPreferences;
use crate::core::chat::prompt::{PromptBuilder, PromptPreferences};
use crate::core::chat::stream::StreamManager;
use crate::core::context::ContextResolver;
use crate::core::event::{BusEvent, EventBus};
use crate::core::runtime::{ChatMessage, MessageStatus, Role, DEFAULT_SESSION_ID};
use crate::core::tools::context::{AskStore, PathPermissionStore, TaskItem};
use crate::core::workspace::WorkspaceManager;
use crate::runtime::ToolManager;

pub struct ChatSendResult {
    pub session_id: String,
    pub user_message_id: String,
    pub assistant_message_id: String,
}

pub struct ChatService {
    provider: Arc<dyn AIProvider>,
    event_bus: Arc<dyn EventBus>,
    conversation: Arc<ConversationManager>,
    workspace_manager: Arc<WorkspaceManager>,
    context_resolver: ContextResolver,
    stream_manager: StreamManager,
    tools: Arc<ToolManager>,
    ask_store: Arc<AskStore>,
    path_permission_store: Arc<PathPermissionStore>,
    tasks: Arc<Mutex<Vec<TaskItem>>>,
    app_handle: Option<tauri::AppHandle>,
}

impl ChatService {
    pub fn new(
        provider: Arc<dyn AIProvider>,
        event_bus: Arc<dyn EventBus>,
        context_resolver: ContextResolver,
        tools: Arc<ToolManager>,
        conversation: Arc<ConversationManager>,
        workspace_manager: Arc<WorkspaceManager>,
        app_handle: tauri::AppHandle,
    ) -> Self {
        Self {
            provider,
            event_bus,
            conversation,
            workspace_manager,
            context_resolver,
            stream_manager: StreamManager::new(),
            tools,
            ask_store: Arc::new(AskStore::new()),
            path_permission_store: Arc::new(PathPermissionStore::new()),
            tasks: Arc::new(Mutex::new(Vec::new())),
            app_handle: Some(app_handle),
        }
    }

    pub fn conversation(&self) -> Arc<ConversationManager> {
        Arc::clone(&self.conversation)
    }

    pub fn ask_store(&self) -> Arc<AskStore> {
        Arc::clone(&self.ask_store)
    }

    pub fn path_permission_store(&self) -> Arc<PathPermissionStore> {
        Arc::clone(&self.path_permission_store)
    }

    pub async fn send(
        &self,
        session_id: Option<String>,
        content: String,
        preferences: SendPreferences,
    ) -> Result<ChatSendResult, ChatError> {
        let content = content.trim().to_string();
        if content.is_empty() {
            return Err(ChatError::EmptyMessage);
        }
        let workspace = self.workspace_manager.current();

        let session_id = session_id.unwrap_or_else(|| DEFAULT_SESSION_ID.to_string());
        let is_new_session = self.conversation.messages(&session_id).is_empty();
        if is_new_session && workspace.is_some() {
            let workspace = workspace.as_ref().expect("workspace checked above");
            self.conversation.bind_workspace(&session_id, &workspace.id);
        }
        let user_message = create_message(&session_id, Role::User, content, MessageStatus::Done);
        let assistant_message = create_message(
            &session_id,
            Role::Assistant,
            String::new(),
            MessageStatus::Pending,
        );

        self.conversation.append(&session_id, user_message.clone());
        self.conversation
            .append(&session_id, assistant_message.clone());

        self.event_bus.emit(BusEvent::ChatStarted {
            session_id: session_id.clone(),
            user_message: user_message.clone(),
            assistant_message: assistant_message.clone(),
        });

        let mut context = self.context_resolver.resolve();

        if let Some(workspace) = workspace {
            context.set_workspace(workspace.name, &workspace.root);
        }
        // Memory recall may use `reqwest::blocking` — must not run on a tokio worker.
        let recall_text =
            super::selection::visible_user_text(&user_message.content).to_string();
        let workspace_root = context
            .workspace
            .as_ref()
            .map(|workspace| std::path::PathBuf::from(&workspace.root));
        let task_rules = tauri::async_runtime::spawn_blocking(move || {
            crate::core::rules::RuleEngine::prepare_task(
                &recall_text,
                workspace_root.as_deref(),
                is_new_session,
            )
        })
        .await
        .map_err(|error| ChatError::Provider(error.to_string()))?;
        let _memory_decision = task_rules.memory_decision;

        let history = self.conversation.messages(&session_id);
        let summarizer =
            crate::core::chat::compact::ProviderSummarizer::new(Arc::clone(&self.provider));
        let compact = compact::prepare_history_for_prompt(
            &history,
            &context,
            &session_id,
            DEFAULT_CONTEXT_WINDOW,
            Some(&summarizer),
        )
        .await;
        if let Some(notice) = &compact.notice {
            let language_zh = matches!(
                preferences.app_language,
                crate::models::settings::AppLanguage::ZhCn
            );
            self.event_bus.emit(BusEvent::ChatContextNotice {
                session_id: session_id.clone(),
                kind: match notice.kind {
                    compact::ContextNoticeKind::ApproachingLimit => "approaching-limit".to_string(),
                    compact::ContextNoticeKind::Compacted => "compacted".to_string(),
                },
                message: compact::notice_message(notice, language_zh),
                usage_ratio: notice.usage_ratio,
                folded_messages: notice.folded_messages,
            });
        }
        let prompt_preferences = PromptPreferences {
            app_language: preferences.app_language,
            reasoning_language: preferences.reasoning_language,
        };
        let request = PromptBuilder::build(
            &assistant_message.id,
            &session_id,
            &compact.messages,
            &context,
            task_rules.project_rules.as_deref(),
            task_rules.recalled_memories.as_deref(),
            Some(self.provider.id().to_string()),
            &prompt_preferences,
        );

        let turn = history
            .iter()
            .filter(|message| matches!(message.role, Role::User))
            .count();
        crate::core::checkpoint::shared_checkpoint_store().begin_turn(
            &session_id,
            turn,
            &user_message.content,
            Some(user_message.id.clone()),
        );

        self.stream_manager.spawn(
            self.provider.clone(),
            Arc::clone(&self.tools),
            self.event_bus.clone(),
            Arc::clone(&self.conversation),
            Arc::clone(&self.ask_store),
            Arc::clone(&self.path_permission_store),
            Arc::clone(&self.tasks),
            self.app_handle.clone(),
            request,
            assistant_message.id.clone(),
            session_id.clone(),
        );

        Ok(ChatSendResult {
            session_id,
            user_message_id: user_message.id,
            assistant_message_id: assistant_message.id,
        })
    }

    pub fn cancel(&self, message_id: &str) -> Result<(), ChatError> {
        self.stream_manager
            .cancel(&self.conversation, self.event_bus.as_ref(), message_id)
    }

    pub fn history(&self, session_id: &str) -> Result<Vec<ChatMessage>, ChatError> {
        self.conversation.history(session_id)
    }

    pub fn list_sessions(&self) -> Vec<crate::models::chat::ChatSessionSummary> {
        self.conversation.list_sessions()
    }

    pub fn emit_plan_mode_changed(&self, session_id: &str, active: bool) {
        self.event_bus.emit(BusEvent::PlanModeChanged {
            session_id: session_id.to_string(),
            active,
        });
    }
}
