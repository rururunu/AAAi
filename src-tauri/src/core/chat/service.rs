use std::sync::{Arc, Mutex};

use crate::core::agent::{AgentDebugEvent, AgentRuntime, AgentSpawnInput};
use crate::core::ai::provider::AIProvider;
use crate::core::chat::compact::{self, context_window_tokens};
use crate::core::chat::conversation_manager::{create_message, ConversationManager};
use crate::core::chat::error::ChatError;
use crate::core::chat::limits::max_turn_tokens_for;
use crate::core::chat::preferences::SendPreferences;
use crate::core::chat::prompt::{PromptBuildInput, PromptBuilder, PromptPreferences};
use crate::core::context::ContextResolver;
use crate::core::event::{BusEvent, EventBus};
use crate::core::runtime::{ChatMessage, MessageStatus, Role, DEFAULT_SESSION_ID};
use crate::core::tools::context::{AskStore, PathPermissionStore, TaskItem};
use crate::core::workspace::WorkspaceManager;
use crate::models::settings::ChatMode;
use crate::runtime::ToolManager;
use tauri::Emitter;

pub struct ChatSendResult {
    pub session_id: String,
    pub user_message_id: String,
    pub assistant_message_id: String,
    pub agent_run_id: Option<String>,
}

pub struct ChatService {
    provider: Arc<dyn AIProvider>,
    event_bus: Arc<dyn EventBus>,
    conversation: Arc<ConversationManager>,
    workspace_manager: Arc<WorkspaceManager>,
    context_resolver: ContextResolver,
    agent_runtime: AgentRuntime,
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
        let agent_runtime = AgentRuntime::new(Arc::clone(&event_bus), Arc::clone(&tools));
        Self {
            provider,
            event_bus,
            conversation,
            workspace_manager,
            context_resolver,
            agent_runtime,
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

    pub fn agent_debug_snapshot(&self) -> Vec<AgentDebugEvent> {
        self.agent_runtime.debug_snapshot()
    }

    /// Resolve the AI provider from current settings on every turn.
    /// Startup-time provider is only a fallback for tests without an AppHandle —
    /// otherwise switching Gemini ↔ DeepSeek would keep the wrong backend until restart.
    fn active_provider(&self) -> Arc<dyn AIProvider> {
        match &self.app_handle {
            Some(app) => crate::core::ai::resolve_provider(app.clone()),
            None => Arc::clone(&self.provider),
        }
    }

    pub async fn send(
        &self,
        session_id: Option<String>,
        content: String,
        preferences: SendPreferences,
        workspace_id: Option<String>,
        quick_ask: bool,
    ) -> Result<ChatSendResult, ChatError> {
        let content = content.trim().to_string();
        if content.is_empty() {
            return Err(ChatError::EmptyMessage);
        }
        let known_workspaces = self.workspace_manager.list();
        let workspace = if quick_ask {
            None
        } else if let Some(workspace_id) = workspace_id.as_deref() {
            known_workspaces
                .iter()
                .find(|workspace| workspace.id == workspace_id)
                .cloned()
        } else {
            self.workspace_manager.current()
        };
        if let Some(workspace) = workspace.as_ref() {
            self.workspace_manager
                .touch(&workspace.id)
                .await
                .map_err(ChatError::Internal)?;
        }

        let session_id = session_id.unwrap_or_else(|| DEFAULT_SESSION_ID.to_string());

        // Mid-turn soft inject: queue into the active agent loop (tool boundary).
        if let Some(assistant_message_id) =
            self.agent_runtime.active_assistant_for_session(&session_id)
        {
            return self.soft_inject(&session_id, content, &assistant_message_id);
        }

        let agent_run_id = self.agent_runtime.create_run(content.clone());
        let mut context = self
            .agent_runtime
            .collect_context(&agent_run_id, || {
                let mut context = self
                    .context_resolver
                    .resolve_environment(workspace.as_ref(), &known_workspaces);
                crate::core::context::provider::environment_provider::collect(&mut context);
                context
            })
            .map_err(|error| ChatError::Internal(error.to_string()))?;
        // An explicitly selected workspace owns the new conversation. IDE context
        // is still useful for files and selection, but must not switch its workspace.
        if let Some(workspace) = workspace.as_ref() {
            context.set_workspace(workspace.name.clone(), &workspace.root);
        }
        if quick_ask {
            context.workspace = None;
        }
        self.remember_ide_workspace(&context).await;
        let is_new_session = self.conversation.messages(&session_id).is_empty();
        if is_new_session && !quick_ask {
            if let Some(resolved) = context.workspace.as_ref() {
                self.conversation
                    .bind_workspace(&session_id, &resolved.root);
            }
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

        // Memory recall may use `reqwest::blocking` — must not run on a tokio worker.
        let recall_text = super::selection::visible_user_text(&user_message.content).to_string();
        let workspace_root = context
            .workspace
            .as_ref()
            .map(|workspace| std::path::PathBuf::from(&workspace.root));
        let task_rules_result = tauri::async_runtime::spawn_blocking(move || {
            crate::core::rules::RuleEngine::prepare_task(
                &recall_text,
                workspace_root.as_deref(),
                is_new_session,
            )
        })
        .await;
        let task_rules = match task_rules_result {
            Ok(task_rules) => task_rules,
            Err(error) => {
                self.agent_runtime
                    .fail_run(&agent_run_id, "task preparation failed");
                return Err(ChatError::Provider(error.to_string()));
            }
        };
        let _memory_decision = task_rules.memory_decision;

        let history = self.conversation.messages(&session_id);
        let large_context = self
            .app_handle
            .as_ref()
            .and_then(|app| crate::services::settings_store::get_settings(app).ok())
            .map(|settings| settings.large_context_enabled)
            .unwrap_or(true);
        let context_window = context_window_tokens(large_context);
        let max_turn_tokens = max_turn_tokens_for(large_context);
        let provider = self.active_provider();
        let summarizer = crate::core::chat::compact::ProviderSummarizer::new(Arc::clone(&provider));
        let compact = compact::prepare_history_for_prompt(
            &history,
            &context,
            &session_id,
            context_window,
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
        let collaboration_models = self
            .app_handle
            .as_ref()
            .and_then(|app| crate::services::settings_store::get_settings(app).ok())
            .filter(|settings| settings.multi_model_collaboration)
            .map(|settings| settings.collaboration_models)
            .unwrap_or_default();
        let prompt_preferences = PromptPreferences {
            app_language: preferences.app_language,
            reasoning_language: preferences.reasoning_language,
            response_tone: preferences.response_tone,
            collaboration_models,
        };
        let request = PromptBuilder::build(PromptBuildInput {
            request_id: &assistant_message.id,
            session_id: &session_id,
            history: &compact.messages,
            context: &context,
            project_rules: task_rules.project_rules.as_deref(),
            recalled_memories: task_rules.recalled_memories.as_deref(),
            provider: Some(provider.id().to_string()),
            preferences: &prompt_preferences,
        });

        let turn = history
            .iter()
            .filter(|message| matches!(message.role, Role::User))
            .count();
        crate::core::checkpoint::shared_checkpoint_store().begin_turn(
            &session_id,
            turn,
            &user_message.content,
            Some(user_message.id.clone()),
            context
                .workspace
                .as_ref()
                .map(|workspace| std::path::Path::new(&workspace.root)),
        );

        let chat_mode = self
            .app_handle
            .as_ref()
            .and_then(|app| crate::services::settings_store::get_settings(app).ok())
            .map(|settings| settings.chat_mode)
            .unwrap_or_default();
        let tools = if chat_mode == ChatMode::Ask {
            Arc::new(self.tools.read_only())
        } else {
            Arc::clone(&self.tools)
        };

        let model = self
            .app_handle
            .as_ref()
            .and_then(|app| crate::services::settings_store::get_settings(app).ok())
            .map(|settings| settings.chat_model)
            .unwrap_or_default();

        let spawn_result = self.agent_runtime.spawn(AgentSpawnInput {
            run_id: agent_run_id.clone(),
            provider,
            tools,
            conversation: Arc::clone(&self.conversation),
            ask_store: Arc::clone(&self.ask_store),
            path_permission_store: Arc::clone(&self.path_permission_store),
            tasks: Arc::clone(&self.tasks),
            app_handle: self.app_handle.clone(),
            request,
            assistant_message_id: assistant_message.id.clone(),
            session_id: session_id.clone(),
            max_turn_tokens,
            model,
        });
        if let Err(error) = spawn_result {
            self.agent_runtime
                .fail_run(&agent_run_id, "agent runtime failed to start");
            return Err(ChatError::Internal(error.to_string()));
        }

        Ok(ChatSendResult {
            session_id,
            user_message_id: user_message.id,
            assistant_message_id: assistant_message.id,
            agent_run_id: Some(agent_run_id),
        })
    }

    async fn remember_ide_workspace(&self, context: &crate::core::runtime::RequestContext) {
        let Some(ide) = context.ide_context.as_ref() else {
            return;
        };
        if ide
            .selection
            .as_deref()
            .is_none_or(|selection| selection.trim().is_empty())
        {
            return;
        }
        let Some(root) = ide.workspace.clone() else {
            return;
        };

        match self
            .workspace_manager
            .remember_from_ide(root, &ide.ide)
            .await
        {
            Ok((_, false)) => {}
            Ok((workspace, true)) => {
                if let Some(app) = &self.app_handle {
                    if let Err(error) =
                        app.emit("workspaces-changed", self.workspace_manager.current())
                    {
                        tracing::warn!(
                            provider = "ide",
                            workspace = %workspace.root.display(),
                            error = %error,
                            "failed to emit IDE workspace update"
                        );
                    }
                }
            }
            Err(error) => {
                tracing::warn!(
                    provider = "ide",
                    ide = %ide.ide,
                    error = %error,
                    "failed to remember IDE workspace"
                );
            }
        }
    }

    fn soft_inject(
        &self,
        session_id: &str,
        content: String,
        assistant_message_id: &str,
    ) -> Result<ChatSendResult, ChatError> {
        // Marker persists soft-inject identity across history reload (UI folds these
        // into the preceding assistant turn instead of an unanswered user bubble).
        const SOFT_INJECT_MARKER: &str = "<!--peek:soft-inject-->\n";
        let stored = format!("{SOFT_INJECT_MARKER}{content}");
        let user_message = create_message(session_id, Role::User, stored, MessageStatus::Done);
        self.conversation.append(session_id, user_message.clone());
        // Agent queue gets plain text (no HTML marker).
        self.agent_runtime.soft_inject(session_id, content)?;

        // Do not emit ChatStarted: that would re-project the assistant bubble and can
        // wipe in-flight streamed content. Frontend already staged the user message.
        Ok(ChatSendResult {
            session_id: session_id.to_string(),
            user_message_id: user_message.id,
            assistant_message_id: assistant_message_id.to_string(),
            agent_run_id: self
                .agent_runtime
                .run_for_message(assistant_message_id)
                .map(|run| run.id),
        })
    }

    pub fn cancel(&self, message_id: &str) -> Result<(), ChatError> {
        self.agent_runtime.cancel(&self.conversation, message_id)
    }

    pub fn history(&self, session_id: &str) -> Result<Vec<ChatMessage>, ChatError> {
        self.conversation.history(session_id)
    }

    pub fn list_sessions(&self) -> Vec<crate::models::chat::ChatSessionSummary> {
        self.conversation.list_sessions()
    }

    pub fn context_usage(
        &self,
        app: &tauri::AppHandle,
        session_id: Option<String>,
        draft_message: Option<String>,
        context: Option<crate::core::runtime::RequestContext>,
    ) -> Result<crate::models::chat::ContextUsageResponse, ChatError> {
        use crate::core::chat::compact::{context_window_tokens, measure_context_usage};
        use crate::services::settings_store::get_settings;

        let history = match session_id.as_deref() {
            Some(id) => self.conversation.messages(id),
            None => Vec::new(),
        };
        let current_workspace = self.workspace_manager.current();
        let known_workspaces = self.workspace_manager.list();
        let mut ctx = self.context_resolver.resolve_request(
            context.unwrap_or_else(|| self.context_resolver.resolve()),
            current_workspace.as_ref(),
            &known_workspaces,
        );
        crate::core::context::provider::environment_provider::collect(&mut ctx);

        let large_context = get_settings(app)
            .map(|settings| settings.large_context_enabled)
            .unwrap_or(true);
        let context_window = context_window_tokens(large_context);
        let measure =
            measure_context_usage(&history, &ctx, draft_message.as_deref(), context_window);

        Ok(crate::models::chat::ContextUsageResponse {
            usage_ratio: measure.usage_ratio,
            estimated_tokens: measure.estimated_tokens,
            context_window_tokens: context_window,
        })
    }

    pub fn environment_context(&self) -> crate::core::runtime::RequestContext {
        let current_workspace = self.workspace_manager.current();
        let known_workspaces = self.workspace_manager.list();
        let captured = self.context_resolver.resolve();
        tracing::debug!(
            active_window = ?captured.active_window.as_deref(),
            active_file = ?captured.active_file.as_deref(),
            workspace = ?captured.workspace.as_ref().map(|workspace| workspace.root.as_str()),
            selected_files = captured.selected_files.len(),
            ide = ?captured.ide_context.as_ref().map(|ide| ide.ide.as_str()),
            "ChatService::environment_context input captured context"
        );
        let mut context = self.context_resolver.resolve_request(
            captured,
            current_workspace.as_ref(),
            &known_workspaces,
        );
        crate::core::context::provider::environment_provider::collect(&mut context);
        tracing::debug!(
            active_window = ?context.active_window.as_deref(),
            active_file = ?context.active_file.as_deref(),
            workspace = ?context.workspace.as_ref().map(|workspace| workspace.root.as_str()),
            has_git_status = context.git_status.is_some(),
            has_shell_execution = context.last_shell_execution.is_some(),
            ide = ?context.ide_context.as_ref().map(|ide| ide.ide.as_str()),
            "ChatService::environment_context final resolved context"
        );
        context
    }

    pub fn emit_plan_mode_changed(&self, session_id: &str, active: bool) {
        self.event_bus.emit(BusEvent::PlanModeChanged {
            session_id: session_id.to_string(),
            active,
        });
    }
}
