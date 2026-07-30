use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::core::agent::debug::AgentDebugEvent;
use crate::core::agent::executor::AgentExecutor;
use crate::core::agent::planner::{AgentPlan, AgentPlanStepStatus, LlmPlanner, Planner};
use crate::core::agent::tools::{AgentToolError, AgentToolOutput, AgentToolRegistry};
use crate::core::ai::provider::AIProvider;
use crate::core::chat::conversation_manager::ConversationManager;
use crate::core::chat::error::ChatError;
use crate::core::chat::stream::StreamManager;
use crate::core::event::{BusEvent, EventBus};
use crate::core::runtime::{ChatRequest, RequestContext};
use crate::core::tools::context::{AskStore, PathPermissionStore, TaskItem, ToolContext};
use crate::runtime::ToolManager;

use super::{AgentEvent, AgentEventRecord, AgentState, AgentTransitionError};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRun {
    pub id: String,
    pub state: AgentState,
    pub input: String,
    pub context: Option<RequestContext>,
    pub plan: Option<AgentPlan>,
    pub events: Vec<AgentEventRecord>,
}

impl AgentRun {
    pub fn new(input: String) -> Self {
        let id = Uuid::new_v4().to_string();
        let mut run = Self {
            id,
            state: AgentState::Created,
            input: input.clone(),
            context: None,
            plan: None,
            events: Vec::new(),
        };
        run.push_event(AgentEvent::UserMessage { input });
        run
    }

    pub fn transition(
        &mut self,
        next: AgentState,
    ) -> Result<Option<AgentEventRecord>, AgentTransitionError> {
        if self.state == next {
            return Ok(None);
        }
        if !self.state.can_transition_to(next) {
            return Err(AgentTransitionError {
                from: self.state,
                to: next,
            });
        }
        let from = self.state;
        self.state = next;
        Ok(Some(
            self.push_event(AgentEvent::StateChanged { from, to: next }),
        ))
    }

    pub fn push_event(&mut self, event: AgentEvent) -> AgentEventRecord {
        let record = AgentEventRecord {
            run_id: self.id.clone(),
            sequence: self.events.len() as u64 + 1,
            timestamp_ms: now_millis(),
            event,
        };
        self.events.push(record.clone());
        record
    }

    pub fn cancel(&mut self) -> Result<Option<AgentEventRecord>, AgentTransitionError> {
        self.transition(AgentState::Cancelled)
    }

    pub fn execute(&mut self) -> Result<Option<AgentEventRecord>, AgentTransitionError> {
        self.transition(AgentState::Executing)
    }

    pub fn complete(&mut self) -> Result<Option<AgentEventRecord>, AgentTransitionError> {
        self.transition(AgentState::Completed)
    }

    pub fn fail(&mut self) -> Result<Option<AgentEventRecord>, AgentTransitionError> {
        self.transition(AgentState::Failed)
    }
}

struct AgentRuntimeInner {
    runs: Mutex<HashMap<String, AgentRun>>,
    message_runs: Mutex<HashMap<String, String>>,
    event_bus: Arc<dyn EventBus>,
    planner: Arc<dyn Planner>,
    executor: AgentExecutor,
}

pub struct AgentRuntime {
    inner: Arc<AgentRuntimeInner>,
    stream_manager: StreamManager,
}

impl AgentRuntime {
    pub fn new(event_bus: Arc<dyn EventBus>, tools: Arc<ToolManager>) -> Self {
        Self::with_components(
            event_bus,
            Arc::new(LlmPlanner),
            Arc::new(AgentToolRegistry::v1(tools)),
        )
    }

    pub fn with_components(
        event_bus: Arc<dyn EventBus>,
        planner: Arc<dyn Planner>,
        tools: Arc<AgentToolRegistry>,
    ) -> Self {
        Self {
            inner: Arc::new(AgentRuntimeInner {
                runs: Mutex::new(HashMap::new()),
                message_runs: Mutex::new(HashMap::new()),
                event_bus,
                planner,
                executor: AgentExecutor::new(tools),
            }),
            stream_manager: StreamManager::new(),
        }
    }

    pub fn create_run(&self, input: String) -> String {
        let run = AgentRun::new(input);
        let run_id = run.id.clone();
        let first_event = run.events.first().cloned();
        if let Ok(mut runs) = self.inner.runs.lock() {
            runs.insert(run_id.clone(), run);
        }
        tracing::debug!(run_id = %run_id, "agent run created");
        self.inner.event_bus.emit(BusEvent::AgentDebugEvent {
            event: AgentDebugEvent::RunCreated {
                run_id: run_id.clone(),
                state: AgentState::Created,
            },
        });
        if let Some(event) = first_event {
            self.emit_record(event);
        }
        run_id
    }

    pub fn begin_context_loading(&self, run_id: &str) -> Result<(), AgentTransitionError> {
        self.transition(run_id, AgentState::ContextLoading)
    }

    pub fn collect_context<F>(
        &self,
        run_id: &str,
        loader: F,
    ) -> Result<RequestContext, AgentTransitionError>
    where
        F: FnOnce() -> RequestContext,
    {
        self.begin_context_loading(run_id)?;
        let context = loader();
        self.context_collected(run_id, context.clone())?;
        Ok(context)
    }

    pub fn context_collected(
        &self,
        run_id: &str,
        context: RequestContext,
    ) -> Result<(), AgentTransitionError> {
        let context_event = AgentEvent::ContextCollected {
            has_workspace: context.workspace.is_some(),
            has_active_file: context.active_file.is_some(),
            ide: context.ide_context.as_ref().map(|ide| ide.ide.clone()),
        };
        self.update_run(run_id, |run| {
            run.context = Some(context.clone());
            run.push_event(context_event)
        });
        self.inner.event_bus.emit(BusEvent::AgentDebugEvent {
            event: AgentDebugEvent::ContextSnapshot {
                run_id: run_id.to_string(),
                context: context.clone(),
            },
        });
        self.transition(run_id, AgentState::Planning)?;
        tracing::debug!(run_id = %run_id, "agent planning started");
        let input = self.run(run_id).map(|run| run.input).unwrap_or_default();
        let plan = self.inner.planner.initial_plan(&input, &context);
        self.update_run(run_id, |run| {
            run.plan = Some(plan.clone());
            run.push_event(AgentEvent::PlanCreated { plan })
        });
        Ok(())
    }

    pub fn run(&self, run_id: &str) -> Option<AgentRun> {
        self.inner.runs.lock().ok()?.get(run_id).cloned()
    }

    pub fn run_for_message(&self, message_id: &str) -> Option<AgentRun> {
        let run_id = self
            .inner
            .message_runs
            .lock()
            .ok()?
            .get(message_id)
            .cloned()?;
        self.run(&run_id)
    }

    pub fn active_assistant_for_session(&self, session_id: &str) -> Option<String> {
        self.stream_manager.active_assistant_for_session(session_id)
    }

    pub fn soft_inject(&self, session_id: &str, content: String) -> Result<String, ChatError> {
        self.stream_manager.soft_inject(session_id, content)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn spawn(
        &self,
        run_id: String,
        provider: Arc<dyn AIProvider>,
        tools: Arc<ToolManager>,
        conversation: Arc<ConversationManager>,
        ask_store: Arc<AskStore>,
        path_permission_store: Arc<PathPermissionStore>,
        tasks: Arc<Mutex<Vec<TaskItem>>>,
        app_handle: Option<tauri::AppHandle>,
        request: ChatRequest,
        assistant_message_id: String,
        session_id: String,
        max_turn_tokens: usize,
        model: String,
    ) -> Result<(), AgentTransitionError> {
        self.transition(&run_id, AgentState::Executing)?;
        if let Ok(mut message_runs) = self.inner.message_runs.lock() {
            message_runs.insert(assistant_message_id.clone(), run_id.clone());
        }
        let event_bus: Arc<dyn EventBus> = Arc::new(AgentEventBridge {
            inner: Arc::clone(&self.inner),
            run_id,
        });
        self.stream_manager.spawn(
            provider,
            tools,
            event_bus,
            conversation,
            ask_store,
            path_permission_store,
            tasks,
            app_handle,
            request,
            assistant_message_id,
            session_id,
            max_turn_tokens,
            model,
        );
        Ok(())
    }

    pub fn cancel(
        &self,
        conversation: &ConversationManager,
        message_id: &str,
    ) -> Result<(), ChatError> {
        if let Some(run_id) = self
            .inner
            .message_runs
            .lock()
            .ok()
            .and_then(|runs| runs.get(message_id).cloned())
        {
            self.inner.cancel(&run_id);
        }
        self.stream_manager
            .cancel(conversation, self.inner.event_bus.as_ref(), message_id)
    }

    pub fn fail_run(&self, run_id: &str, message: impl Into<String>) {
        self.inner.fail(run_id, message.into());
    }

    pub async fn execute_tool(
        &self,
        run_id: &str,
        context: &ToolContext,
        tool: &str,
        input: Value,
    ) -> Result<AgentToolOutput, AgentToolError> {
        let call_id = Uuid::new_v4().to_string();
        self.inner.tool_called(run_id, &call_id, tool, &input, tool);
        match self.inner.executor.execute(context, tool, input).await {
            Ok(output) => {
                self.inner.tool_result(
                    run_id,
                    &call_id,
                    tool,
                    true,
                    &output.content,
                    &output.changed_files,
                );
                Ok(output)
            }
            Err(error) => {
                self.inner
                    .tool_result(run_id, &call_id, tool, false, &error.0, &[]);
                Err(error)
            }
        }
    }

    fn transition(&self, run_id: &str, state: AgentState) -> Result<(), AgentTransitionError> {
        self.inner.transition(run_id, state)
    }

    fn update_run<F>(&self, run_id: &str, update: F)
    where
        F: FnOnce(&mut AgentRun) -> AgentEventRecord,
    {
        let record = self
            .inner
            .runs
            .lock()
            .ok()
            .and_then(|mut runs| runs.get_mut(run_id).map(update));
        if let Some(record) = record {
            self.emit_record(record);
        }
    }

    fn emit_record(&self, record: AgentEventRecord) {
        self.inner.emit_record(record);
    }
}

impl AgentRuntimeInner {
    fn transition(&self, run_id: &str, next: AgentState) -> Result<(), AgentTransitionError> {
        let record = {
            let mut runs = self.runs.lock().map_err(|_| AgentTransitionError {
                from: AgentState::Failed,
                to: next,
            })?;
            let Some(run) = runs.get_mut(run_id) else {
                return Ok(());
            };
            let from = run.state;
            let record = run.transition(next)?;
            if record.is_some() {
                tracing::debug!(run_id = %run_id, from = ?from, to = ?next, "agent state changed");
            }
            record
        };
        if let Some(record) = record {
            self.emit_record(record);
        }
        Ok(())
    }

    fn push_event(&self, run_id: &str, event: AgentEvent) {
        let record = self
            .runs
            .lock()
            .ok()
            .and_then(|mut runs| runs.get_mut(run_id).map(|run| run.push_event(event)));
        if let Some(record) = record {
            self.emit_record(record);
        }
    }

    fn emit_record(&self, record: AgentEventRecord) {
        self.event_bus.emit(BusEvent::AgentEvent {
            event: record.clone(),
        });
        self.event_bus.emit(BusEvent::AgentDebugEvent {
            event: AgentDebugEvent::RuntimeEvent { record },
        });
    }

    fn tool_called(
        &self,
        run_id: &str,
        call_id: &str,
        tool: &str,
        arguments: &Value,
        description: &str,
    ) {
        let state = self.current_state(run_id);
        if state == Some(AgentState::Reflecting) {
            let _ = self.transition(run_id, AgentState::Planning);
        }
        if self.current_state(run_id) == Some(AgentState::Planning) {
            let _ = self.transition(run_id, AgentState::Executing);
        }
        let _ = self.transition(run_id, AgentState::WaitingTool);
        let mut step = self.planner.plan_tool_call(tool, arguments, description);
        step.id = call_id.to_string();
        let plan = {
            let mut runs = match self.runs.lock() {
                Ok(runs) => runs,
                Err(_) => return,
            };
            let Some(run) = runs.get_mut(run_id) else {
                return;
            };
            let plan = run.plan.get_or_insert_with(|| AgentPlan::new(Vec::new()));
            for existing in &mut plan.steps {
                if existing.tool.is_none() && existing.status == AgentPlanStepStatus::Running {
                    existing.status = AgentPlanStepStatus::Completed;
                }
            }
            plan.steps.push(step);
            plan.clone()
        };
        self.push_event(run_id, AgentEvent::PlanCreated { plan });
        self.push_event(
            run_id,
            AgentEvent::ToolCalled {
                call_id: call_id.to_string(),
                tool: tool.to_string(),
                description: description.to_string(),
            },
        );
        self.event_bus.emit(BusEvent::AgentDebugEvent {
            event: AgentDebugEvent::ToolCall {
                run_id: run_id.to_string(),
                call_id: call_id.to_string(),
                tool: tool.to_string(),
                description: description.to_string(),
                arguments: arguments.clone(),
            },
        });
        tracing::debug!(run_id = %run_id, tool = %tool, "agent tool called");
    }

    fn tool_result(
        &self,
        run_id: &str,
        call_id: &str,
        tool: &str,
        success: bool,
        result: &str,
        changed_files: &[String],
    ) {
        let updated_plan = {
            let mut runs = match self.runs.lock() {
                Ok(runs) => runs,
                Err(_) => return,
            };
            let Some(run) = runs.get_mut(run_id) else {
                return;
            };
            run.plan.as_mut().map(|plan| {
                if let Some(step) = plan.steps.iter_mut().find(|step| step.id == call_id) {
                    step.status = if success {
                        AgentPlanStepStatus::Completed
                    } else {
                        AgentPlanStepStatus::Failed
                    };
                }
                plan.clone()
            })
        };
        if let Some(plan) = updated_plan {
            self.push_event(run_id, AgentEvent::PlanCreated { plan });
        }
        self.push_event(
            run_id,
            AgentEvent::ToolResult {
                call_id: call_id.to_string(),
                tool: tool.to_string(),
                success,
                result: result.to_string(),
            },
        );
        tracing::debug!(
            run_id = %run_id,
            tool = %tool,
            success,
            result_length = result.len(),
            "agent tool completed"
        );
        if !success {
            self.fail(run_id, format!("tool {tool} failed"));
            return;
        }
        for path in changed_files {
            self.push_event(run_id, AgentEvent::FileChanged { path: path.clone() });
        }
        match self.current_state(run_id) {
            Some(AgentState::WaitingTool | AgentState::Executing | AgentState::Planning) => {
                let _ = self.transition(run_id, AgentState::Observing);
                let _ = self.transition(run_id, AgentState::Reflecting);
            }
            Some(AgentState::Observing) => {
                let _ = self.transition(run_id, AgentState::Reflecting);
            }
            _ => {}
        }
    }

    fn complete(&self, run_id: &str) {
        let state = self.current_state(run_id);
        let Some(state) = state else {
            return;
        };
        if state.is_terminal() {
            return;
        }
        let completed_plan = {
            let mut runs = match self.runs.lock() {
                Ok(runs) => runs,
                Err(_) => return,
            };
            let Some(run) = runs.get_mut(run_id) else {
                return;
            };
            run.plan.as_mut().map(|plan| {
                for step in &mut plan.steps {
                    if step.status == AgentPlanStepStatus::Running {
                        step.status = AgentPlanStepStatus::Completed;
                    }
                }
                plan.clone()
            })
        };
        if let Some(plan) = completed_plan {
            self.push_event(run_id, AgentEvent::PlanCreated { plan });
        }
        match state {
            AgentState::WaitingTool => {
                let _ = self.transition(run_id, AgentState::Observing);
                let _ = self.transition(run_id, AgentState::Reflecting);
            }
            AgentState::Observing => {
                let _ = self.transition(run_id, AgentState::Reflecting);
            }
            AgentState::Planning | AgentState::Executing => {
                let _ = self.transition(run_id, AgentState::Reflecting);
            }
            _ => {}
        }
        if self.transition(run_id, AgentState::Completed).is_ok() {
            self.push_event(run_id, AgentEvent::Completed);
            tracing::debug!(run_id = %run_id, "agent completed");
        }
    }

    fn fail(&self, run_id: &str, message: String) {
        if self
            .current_state(run_id)
            .is_some_and(|state| state.is_terminal())
        {
            return;
        }
        self.push_event(run_id, AgentEvent::Error { message });
        let _ = self.transition(run_id, AgentState::Failed);
    }

    fn cancel(&self, run_id: &str) {
        if self
            .current_state(run_id)
            .is_some_and(|state| !state.is_terminal())
        {
            let _ = self.transition(run_id, AgentState::Cancelled);
        }
    }

    fn current_state(&self, run_id: &str) -> Option<AgentState> {
        self.runs.lock().ok()?.get(run_id).map(|run| run.state)
    }
}

struct AgentEventBridge {
    inner: Arc<AgentRuntimeInner>,
    run_id: String,
}

impl EventBus for AgentEventBridge {
    fn emit(&self, event: BusEvent) {
        match &event {
            BusEvent::ToolStarted {
                subagent_id,
                activity_id,
                tool_name,
                title,
                arguments,
                ..
            } => {
                if let Some(subagent_id) = subagent_id {
                    self.inner.event_bus.emit(BusEvent::AgentDebugEvent {
                        event: AgentDebugEvent::SubagentToolCall {
                            run_id: self.run_id.clone(),
                            subagent_id: subagent_id.clone(),
                            call_id: activity_id.clone(),
                            tool: tool_name.clone(),
                            description: title.clone(),
                            arguments: arguments.clone(),
                            timestamp_ms: now_millis(),
                        },
                    });
                } else {
                    self.inner
                        .tool_called(&self.run_id, activity_id, tool_name, arguments, title);
                }
            }
            BusEvent::ToolFinished {
                subagent_id,
                activity_id,
                tool_name,
                result,
                success,
                preview,
                ..
            } => {
                if let Some(subagent_id) = subagent_id {
                    self.inner.event_bus.emit(BusEvent::AgentDebugEvent {
                        event: AgentDebugEvent::SubagentToolResult {
                            run_id: self.run_id.clone(),
                            subagent_id: subagent_id.clone(),
                            call_id: activity_id.clone(),
                            tool: tool_name.clone(),
                            success: *success,
                            result: result.clone(),
                            timestamp_ms: now_millis(),
                        },
                    });
                    self.inner.event_bus.emit(event);
                    return;
                }
                let changed_files = if *success {
                    preview
                        .as_ref()
                        .map(|preview| preview.affected_paths.clone())
                        .unwrap_or_default()
                } else {
                    Vec::new()
                };
                self.inner.tool_result(
                    &self.run_id,
                    activity_id,
                    tool_name,
                    *success,
                    result,
                    &changed_files,
                );
            }
            BusEvent::SubagentStarted {
                subagent_id,
                parent_subagent_id,
                description,
                read_only,
                depth,
                timestamp_ms,
            } => {
                self.inner.event_bus.emit(BusEvent::AgentDebugEvent {
                    event: AgentDebugEvent::SubagentStarted {
                        run_id: self.run_id.clone(),
                        subagent_id: subagent_id.clone(),
                        parent_subagent_id: parent_subagent_id.clone(),
                        description: description.clone(),
                        read_only: *read_only,
                        depth: *depth,
                        timestamp_ms: *timestamp_ms,
                    },
                });
            }
            BusEvent::SubagentProgress {
                subagent_id,
                kind,
                content,
                timestamp_ms,
            } => {
                self.inner.event_bus.emit(BusEvent::AgentDebugEvent {
                    event: AgentDebugEvent::SubagentProgress {
                        run_id: self.run_id.clone(),
                        subagent_id: subagent_id.clone(),
                        kind: kind.clone(),
                        content: content.clone(),
                        timestamp_ms: *timestamp_ms,
                    },
                });
            }
            BusEvent::SubagentFinished {
                subagent_id,
                success,
                summary,
                timestamp_ms,
            } => {
                self.inner.event_bus.emit(BusEvent::AgentDebugEvent {
                    event: AgentDebugEvent::SubagentFinished {
                        run_id: self.run_id.clone(),
                        subagent_id: subagent_id.clone(),
                        success: *success,
                        summary: summary.clone(),
                        timestamp_ms: *timestamp_ms,
                    },
                });
            }
            BusEvent::ChatFinished { finish_reason, .. } => {
                if finish_reason.as_deref() == Some("cancelled") {
                    self.inner.cancel(&self.run_id);
                } else {
                    self.inner.complete(&self.run_id);
                }
            }
            BusEvent::ChatError { message, .. } => {
                self.inner.fail(&self.run_id, message.clone());
            }
            _ => {}
        }
        self.inner.event_bus.emit(event);
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Mutex;

    struct NullEventBus;

    impl EventBus for NullEventBus {
        fn emit(&self, _event: BusEvent) {}
    }

    struct RecordingEventBus {
        events: Mutex<Vec<BusEvent>>,
    }

    impl RecordingEventBus {
        fn new() -> Self {
            Self {
                events: Mutex::new(Vec::new()),
            }
        }
    }

    impl EventBus for RecordingEventBus {
        fn emit(&self, event: BusEvent) {
            self.events.lock().unwrap().push(event);
        }
    }

    fn runtime() -> AgentRuntime {
        AgentRuntime::with_components(
            Arc::new(NullEventBus),
            Arc::new(LlmPlanner),
            Arc::new(AgentToolRegistry::new()),
        )
    }

    #[test]
    fn run_records_context_plan_and_state_events() {
        let runtime = runtime();
        let run_id = runtime.create_run("inspect workspace".to_string());
        runtime.begin_context_loading(&run_id).unwrap();
        runtime
            .context_collected(&run_id, RequestContext::default())
            .unwrap();
        runtime.transition(&run_id, AgentState::Executing).unwrap();
        runtime.inner.complete(&run_id);

        let run = runtime.run(&run_id).unwrap();
        assert_eq!(run.state, AgentState::Completed);
        assert!(run
            .events
            .iter()
            .any(|record| matches!(record.event, AgentEvent::ContextCollected { .. })));
        assert!(run
            .events
            .iter()
            .any(|record| matches!(record.event, AgentEvent::PlanCreated { .. })));
        assert!(run
            .events
            .iter()
            .any(|record| matches!(record.event, AgentEvent::Completed)));
    }

    #[test]
    fn debug_projection_carries_context_tools_and_ordered_states() {
        let bus = Arc::new(RecordingEventBus::new());
        let runtime = AgentRuntime::with_components(
            bus.clone(),
            Arc::new(LlmPlanner),
            Arc::new(AgentToolRegistry::new()),
        );
        let run_id = runtime.create_run("inspect".to_string());
        let context = RequestContext {
            active_file: Some("src/main.rs".to_string()),
            ..RequestContext::default()
        };
        runtime
            .collect_context(&run_id, || context.clone())
            .unwrap();
        runtime.transition(&run_id, AgentState::Executing).unwrap();
        runtime.inner.tool_called(
            &run_id,
            "call-1",
            "shell",
            &serde_json::json!({ "command": "cargo check" }),
            "Check project",
        );
        runtime
            .inner
            .tool_result(&run_id, "call-1", "shell", true, "ok", &[]);
        runtime.inner.complete(&run_id);

        let events = bus.events.lock().unwrap();
        assert!(events.iter().any(|event| matches!(
            event,
            BusEvent::AgentDebugEvent {
                event: AgentDebugEvent::RunCreated { run_id: id, .. }
            } if id == &run_id
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            BusEvent::AgentDebugEvent {
                event: AgentDebugEvent::ContextSnapshot { context: snapshot, .. }
            } if snapshot == &context
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            BusEvent::AgentDebugEvent {
                event: AgentDebugEvent::ToolCall { arguments, .. }
            } if arguments["command"] == "cargo check"
        )));

        let states: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                BusEvent::AgentDebugEvent {
                    event:
                        AgentDebugEvent::RuntimeEvent {
                            record:
                                AgentEventRecord {
                                    event: AgentEvent::StateChanged { to, .. },
                                    ..
                                },
                        },
                } => Some(*to),
                _ => None,
            })
            .collect();
        assert_eq!(
            states,
            vec![
                AgentState::ContextLoading,
                AgentState::Planning,
                AgentState::Executing,
                AgentState::WaitingTool,
                AgentState::Observing,
                AgentState::Reflecting,
                AgentState::Completed,
            ]
        );
    }

    #[test]
    fn failed_tool_moves_run_to_failed() {
        let runtime = runtime();
        let run_id = runtime.create_run("run tool".to_string());
        runtime.begin_context_loading(&run_id).unwrap();
        runtime
            .context_collected(&run_id, RequestContext::default())
            .unwrap();
        runtime.transition(&run_id, AgentState::Executing).unwrap();
        runtime.inner.tool_called(
            &run_id,
            "call-1",
            "shell",
            &serde_json::json!({}),
            "Run command",
        );
        runtime
            .inner
            .tool_result(&run_id, "call-1", "shell", false, "failed", &[]);

        let run = runtime.run(&run_id).unwrap();
        assert_eq!(run.state, AgentState::Failed);
        assert!(run
            .events
            .iter()
            .any(|record| matches!(record.event, AgentEvent::ToolResult { success: false, .. })));
        assert!(run
            .events
            .iter()
            .any(|record| matches!(record.event, AgentEvent::Error { .. })));
        assert!(run.plan.as_ref().is_some_and(|plan| plan
            .steps
            .iter()
            .any(|step| step.status == AgentPlanStepStatus::Failed)));
    }

    #[test]
    fn event_bridge_records_tool_and_completion_events() {
        let bus = Arc::new(RecordingEventBus::new());
        let runtime = AgentRuntime::with_components(
            bus.clone(),
            Arc::new(LlmPlanner),
            Arc::new(AgentToolRegistry::new()),
        );
        let run_id = runtime.create_run("inspect".to_string());
        runtime
            .collect_context(&run_id, RequestContext::default)
            .unwrap();
        runtime.transition(&run_id, AgentState::Executing).unwrap();
        let bridge = AgentEventBridge {
            inner: Arc::clone(&runtime.inner),
            run_id: run_id.clone(),
        };

        bridge.emit(BusEvent::ToolStarted {
            session_id: "session".to_string(),
            subagent_id: None,
            parent_activity_id: None,
            message_id: "message".to_string(),
            activity_id: "call-1".to_string(),
            tool_name: "git".to_string(),
            title: "Read git status".to_string(),
            kind: "git".to_string(),
            detail: None,
            arguments: serde_json::json!({ "action": "status" }),
            preview: None,
        });
        bridge.emit(BusEvent::ToolFinished {
            session_id: "session".to_string(),
            subagent_id: None,
            parent_activity_id: None,
            message_id: "message".to_string(),
            activity_id: "call-1".to_string(),
            tool_name: "git".to_string(),
            title: "Read git status".to_string(),
            kind: "git".to_string(),
            detail: None,
            arguments: serde_json::json!({ "action": "status" }),
            preview: None,
            result: "clean".to_string(),
            success: true,
        });
        bridge.emit(BusEvent::ChatFinished {
            session_id: "session".to_string(),
            message_id: "message".to_string(),
            content: "done".to_string(),
            reasoning: None,
            finish_reason: Some("stop".to_string()),
        });

        let run = runtime.run(&run_id).unwrap();
        assert_eq!(run.state, AgentState::Completed);
        assert!(run.plan.as_ref().is_some_and(|plan| plan
            .steps
            .iter()
            .all(|step| step.status == AgentPlanStepStatus::Completed)));
        assert!(run
            .events
            .iter()
            .any(|record| matches!(record.event, AgentEvent::ToolCalled { .. })));
        assert!(run
            .events
            .iter()
            .any(|record| matches!(record.event, AgentEvent::ToolResult { success: true, .. })));
        assert!(bus.events.lock().unwrap().iter().any(|event| matches!(
            event,
            BusEvent::AgentEvent {
                event: AgentEventRecord {
                    event: AgentEvent::Completed,
                    ..
                }
            }
        )));
    }

    #[test]
    fn event_bridge_projects_subagent_lifecycle_without_mutating_parent_plan() {
        let bus = Arc::new(RecordingEventBus::new());
        let runtime = AgentRuntime::with_components(
            bus.clone(),
            Arc::new(LlmPlanner),
            Arc::new(AgentToolRegistry::new()),
        );
        let run_id = runtime.create_run("delegate".to_string());
        let bridge = AgentEventBridge {
            inner: Arc::clone(&runtime.inner),
            run_id: run_id.clone(),
        };

        bridge.emit(BusEvent::SubagentStarted {
            subagent_id: "child-1".to_string(),
            parent_subagent_id: None,
            description: "Inspect files".to_string(),
            read_only: true,
            depth: 1,
            timestamp_ms: 10,
        });
        bridge.emit(BusEvent::ToolStarted {
            session_id: "session".to_string(),
            subagent_id: Some("child-1".to_string()),
            parent_activity_id: Some("parent-tool".to_string()),
            message_id: "message".to_string(),
            activity_id: "child-call".to_string(),
            tool_name: "read_file".to_string(),
            title: "Read file".to_string(),
            kind: "read".to_string(),
            detail: None,
            arguments: serde_json::json!({ "path": "src/main.rs" }),
            preview: None,
        });
        bridge.emit(BusEvent::SubagentFinished {
            subagent_id: "child-1".to_string(),
            success: true,
            summary: "done".to_string(),
            timestamp_ms: 20,
        });

        let events = bus.events.lock().unwrap();
        assert!(events.iter().any(|event| matches!(
            event,
            BusEvent::AgentDebugEvent {
                event: AgentDebugEvent::SubagentStarted { run_id: id, subagent_id, .. }
            } if id == &run_id && subagent_id == "child-1"
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            BusEvent::AgentDebugEvent {
                event: AgentDebugEvent::SubagentToolCall { subagent_id, tool, .. }
            } if subagent_id == "child-1" && tool == "read_file"
        )));
        assert!(runtime.run(&run_id).unwrap().plan.is_none());
    }
}
