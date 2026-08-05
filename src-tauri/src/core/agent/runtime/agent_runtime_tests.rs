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
fn agent_run_accumulates_model_usage_deltas() {
    let runtime = runtime();
    let run_id = runtime.create_run("count this run".to_string());
    runtime.inner.configure_accounting(&run_id, "deepseek-chat");
    runtime.inner.record_token_usage(
        &run_id,
        "deepseek-chat",
        &crate::core::token::TokenUsage::exact(10, 4, "test"),
    );
    runtime.inner.record_token_usage(
        &run_id,
        "deepseek-chat",
        &crate::core::token::TokenUsage::exact(20, 6, "test"),
    );

    let run = runtime.run(&run_id).unwrap();
    assert_eq!(run.model.as_deref(), Some("deepseek-chat"));
    assert_eq!(run.token_usage.input_tokens, 30);
    assert_eq!(run.token_usage.output_tokens, 10);
    assert_eq!(run.token_usage.total_tokens, 40);
    assert_eq!(
        run.token_usage.accuracy,
        crate::core::token::TokenAccuracy::Exact
    );
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

    let snapshot = runtime.debug_snapshot();
    assert!(snapshot.iter().any(|event| matches!(
        event,
        AgentDebugEvent::ToolCall { call_id, .. } if call_id == "call-1"
    )));

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
        } if snapshot.as_ref() == &context
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
