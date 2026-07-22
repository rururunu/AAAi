use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::mpsc;

use crate::core::ai::provider::{AIProvider, ProviderError};
use crate::core::chat::agent::AgentRunner;
use crate::core::chat::conversation_manager::ConversationManager;
use crate::core::chat::limits::TOOL_OUTPUT_MAX_CHARS;
use crate::core::event::{BusEvent, EventBus};
use crate::core::runtime::{
    ChatMessage, ChatRequest, MessageStatus, RequestContext, Role, StreamEvent, ToolCallPayload,
};
use crate::core::tools::context::{AskStore, PathPermissionStore, Tool, ToolContext};
use crate::core::tools::error::ToolError;
use crate::core::tools::registry::ToolRegistry;
use crate::runtime::ToolManager;

struct NullEventBus;
impl EventBus for NullEventBus {
    fn emit(&self, _event: BusEvent) {}
}

struct ScriptedProvider {
    scripts: Mutex<Vec<ProviderTurn>>,
}

struct ProviderTurn {
    content: String,
    tool_calls: Vec<ToolCallPayload>,
}

#[async_trait]
impl AIProvider for ScriptedProvider {
    fn id(&self) -> &'static str {
        "scripted"
    }

    async fn stream(
        &self,
        _request: ChatRequest,
        tx: mpsc::Sender<StreamEvent>,
    ) -> Result<(), ProviderError> {
        let turn = self
            .scripts
            .lock()
            .ok()
            .and_then(|mut scripts| {
                if scripts.is_empty() {
                    None
                } else {
                    Some(scripts.remove(0))
                }
            })
            .unwrap_or(ProviderTurn {
                content: "done".into(),
                tool_calls: vec![],
            });

        let _ = tx.send(StreamEvent::Start).await;
        if !turn.content.is_empty() {
            let _ = tx.send(StreamEvent::Delta(turn.content.clone())).await;
        }
        for call in &turn.tool_calls {
            let _ = tx.send(StreamEvent::ToolCall(call.clone())).await;
        }
        let _ = tx
            .send(StreamEvent::TurnComplete {
                content: turn.content,
                reasoning: None,
                tool_calls: turn.tool_calls,
                finish_reason: Some("stop".into()),
            })
            .await;
        let _ = tx.send(StreamEvent::Finish).await;
        Ok(())
    }
}

struct CountingTool {
    name: &'static str,
    read_only: bool,
    counter: Arc<AtomicUsize>,
    parallel_peak: Arc<AtomicUsize>,
    active: Arc<AtomicUsize>,
    payload: String,
}

impl Tool for CountingTool {
    fn name(&self) -> &str {
        self.name
    }
    fn description(&self) -> &str {
        "test tool"
    }
    fn parameters_schema(&self) -> Value {
        serde_json::json!({ "type": "object" })
    }
    fn read_only(&self) -> bool {
        self.read_only
    }
    fn execute(&self, _ctx: &ToolContext, _args: Value) -> Result<String, ToolError> {
        let current = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.parallel_peak.fetch_max(current, Ordering::SeqCst);
        self.counter.fetch_add(1, Ordering::SeqCst);
        std::thread::sleep(std::time::Duration::from_millis(30));
        self.active.fetch_sub(1, Ordering::SeqCst);
        Ok(self.payload.clone())
    }
}

fn tool_call(id: &str, name: &str) -> ToolCallPayload {
    ToolCallPayload {
        id: id.into(),
        name: name.into(),
        arguments: "{}".into(),
        thought_signature: None,
    }
}

fn make_ctx(registry: Arc<ToolRegistry>) -> (ToolContext, std::path::PathBuf) {
    let db_path = std::env::temp_dir().join(format!("peek-agent-loop-{}.db", uuid::Uuid::new_v4()));
    let ctx = ToolContext {
        workspace_root: std::env::temp_dir(),
        request_context: RequestContext::default(),
        session_id: "s1".into(),
        assistant_message_id: "a1".into(),
        conversation: Arc::new(ConversationManager::new(db_path.clone())),
        event_bus: Arc::new(NullEventBus),
        tasks: Arc::new(Mutex::new(Vec::new())),
        ask_store: Arc::new(AskStore::new()),
        path_permission_store: Arc::new(PathPermissionStore::new()),
        registry: Some(registry),
        provider: None,
        subagent_depth: 0,
        max_subagent_depth: 0,
        app_handle: None,
    };
    (ctx, db_path)
}

fn base_request() -> ChatRequest {
    ChatRequest {
        request_id: "r1".into(),
        session_id: "s1".into(),
        messages: vec![ChatMessage {
            id: "u1".into(),
            session_id: "s1".into(),
            role: Role::User,
            content: "hello".into(),
            reasoning: None,
            tool_activities: None,
            tool_calls: None,
            tool_call_id: None,
            name: None,
            status: MessageStatus::Done,
            timestamp: 1,
        }],
        context: RequestContext::default(),
        provider: None,
        stream: true,
        tools: std::sync::Arc::from([]),
        temperature: None,
        max_tokens: None,
    }
}

async fn collect_finish(rx: &mut mpsc::Receiver<StreamEvent>) -> Option<StreamEvent> {
    let mut last = None;
    while let Some(event) = rx.recv().await {
        if matches!(event, StreamEvent::TurnComplete { .. }) {
            last = Some(event);
        }
    }
    last
}

struct InjectAwareProvider {
    scripts: Mutex<Vec<ProviderTurn>>,
    saw_inject: Arc<AtomicBool>,
}

#[async_trait]
impl AIProvider for InjectAwareProvider {
    fn id(&self) -> &'static str {
        "inject-aware"
    }

    async fn stream(
        &self,
        request: ChatRequest,
        tx: mpsc::Sender<StreamEvent>,
    ) -> Result<(), ProviderError> {
        if request.messages.iter().any(|message| {
            matches!(message.role, Role::User) && message.content.contains("INJECT-ME")
        }) {
            self.saw_inject.store(true, Ordering::SeqCst);
        }

        let turn = self
            .scripts
            .lock()
            .ok()
            .and_then(|mut scripts| {
                if scripts.is_empty() {
                    None
                } else {
                    Some(scripts.remove(0))
                }
            })
            .unwrap_or(ProviderTurn {
                content: "done".into(),
                tool_calls: vec![],
            });

        let _ = tx.send(StreamEvent::Start).await;
        if !turn.content.is_empty() {
            let _ = tx.send(StreamEvent::Delta(turn.content.clone())).await;
        }
        for call in &turn.tool_calls {
            let _ = tx.send(StreamEvent::ToolCall(call.clone())).await;
        }
        let _ = tx
            .send(StreamEvent::TurnComplete {
                content: turn.content,
                reasoning: None,
                tool_calls: turn.tool_calls,
                finish_reason: Some("stop".into()),
            })
            .await;
        let _ = tx.send(StreamEvent::Finish).await;
        Ok(())
    }
}

#[tokio::test]
async fn soft_inject_applies_at_tool_boundary() {
    use std::collections::VecDeque;

    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(CountingTool {
        name: "read_a",
        read_only: true,
        counter: Arc::new(AtomicUsize::new(0)),
        parallel_peak: Arc::new(AtomicUsize::new(0)),
        active: Arc::new(AtomicUsize::new(0)),
        payload: "a".into(),
    }));
    let tools = Arc::new(ToolManager::new(registry));
    let saw_inject = Arc::new(AtomicBool::new(false));
    let provider = Arc::new(InjectAwareProvider {
        scripts: Mutex::new(vec![
            ProviderTurn {
                content: String::new(),
                tool_calls: vec![tool_call("1", "read_a")],
            },
            ProviderTurn {
                content: "after inject".into(),
                tool_calls: vec![],
            },
        ]),
        saw_inject: Arc::clone(&saw_inject),
    });
    let (ctx, db) = make_ctx(tools.registry());
    let runner = AgentRunner::new(provider, tools);
    let (tx, mut rx) = mpsc::channel(32);
    let soft_queue = Arc::new(Mutex::new(VecDeque::new()));
    let soft_queue_push = Arc::clone(&soft_queue);
    let pusher = tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        if let Ok(mut queue) = soft_queue_push.lock() {
            queue.push_back("INJECT-ME now".into());
        }
    });

    runner
        .run(
            base_request(),
            ctx,
            tx,
            Arc::new(AtomicBool::new(false)),
            soft_queue,
        )
        .await
        .unwrap();
    let _ = pusher.await;
    let finish = collect_finish(&mut rx).await.expect("finish");
    match finish {
        StreamEvent::TurnComplete { content, .. } => assert_eq!(content, "after inject"),
        _ => panic!("unexpected"),
    }
    assert!(
        saw_inject.load(Ordering::SeqCst),
        "second provider call should see soft-injected user message"
    );
    let _ = std::fs::remove_file(db);
}

#[tokio::test]
async fn finishes_without_tools() {
    let provider = Arc::new(ScriptedProvider {
        scripts: Mutex::new(vec![ProviderTurn {
            content: "final answer".into(),
            tool_calls: vec![],
        }]),
    });
    let tools = Arc::new(ToolManager::new(ToolRegistry::new()));
    let (ctx, db) = make_ctx(tools.registry());
    let runner = AgentRunner::new(provider, tools);
    let (tx, mut rx) = mpsc::channel(16);
    runner
        .run(base_request(), ctx, tx, Arc::new(AtomicBool::new(false)), Arc::new(Mutex::new(std::collections::VecDeque::new())))
        .await
        .unwrap();
    let finish = collect_finish(&mut rx).await.expect("finish");
    match finish {
        StreamEvent::TurnComplete { content, .. } => assert_eq!(content, "final answer"),
        _ => panic!("unexpected"),
    }
    let _ = std::fs::remove_file(db);
}

#[tokio::test]
async fn runs_read_only_tools_in_parallel() {
    let counter = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));
    let active = Arc::new(AtomicUsize::new(0));
    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(CountingTool {
        name: "read_a",
        read_only: true,
        counter: Arc::clone(&counter),
        parallel_peak: Arc::clone(&peak),
        active: Arc::clone(&active),
        payload: "a".into(),
    }));
    registry.register(Arc::new(CountingTool {
        name: "read_b",
        read_only: true,
        counter: Arc::clone(&counter),
        parallel_peak: Arc::clone(&peak),
        active: Arc::clone(&active),
        payload: "b".into(),
    }));
    let tools = Arc::new(ToolManager::new(registry));
    let provider = Arc::new(ScriptedProvider {
        scripts: Mutex::new(vec![
            ProviderTurn {
                content: String::new(),
                tool_calls: vec![tool_call("1", "read_a"), tool_call("2", "read_b")],
            },
            ProviderTurn {
                content: "done".into(),
                tool_calls: vec![],
            },
        ]),
    });
    let (ctx, db) = make_ctx(tools.registry());
    let runner = AgentRunner::new(provider, tools);
    let (tx, mut rx) = mpsc::channel(16);
    runner
        .run(base_request(), ctx, tx, Arc::new(AtomicBool::new(false)), Arc::new(Mutex::new(std::collections::VecDeque::new())))
        .await
        .unwrap();
    let _ = collect_finish(&mut rx).await;
    assert_eq!(counter.load(Ordering::SeqCst), 2);
    assert!(
        peak.load(Ordering::SeqCst) >= 2,
        "expected parallel peak >= 2, got {}",
        peak.load(Ordering::SeqCst)
    );
    let _ = std::fs::remove_file(db);
}

#[tokio::test]
async fn stops_at_max_steps() {
    let mut registry = ToolRegistry::new();
    let counter = Arc::new(AtomicUsize::new(0));
    registry.register(Arc::new(CountingTool {
        name: "read_a",
        read_only: true,
        counter: Arc::clone(&counter),
        parallel_peak: Arc::new(AtomicUsize::new(0)),
        active: Arc::new(AtomicUsize::new(0)),
        payload: "a".into(),
    }));
    let tools = Arc::new(ToolManager::new(registry));
    let provider = Arc::new(ScriptedProvider {
        scripts: Mutex::new(vec![
            ProviderTurn {
                content: String::new(),
                tool_calls: vec![tool_call("1", "read_a")],
            },
            ProviderTurn {
                content: String::new(),
                tool_calls: vec![tool_call("2", "read_a")],
            },
            ProviderTurn {
                content: String::new(),
                tool_calls: vec![tool_call("3", "read_a")],
            },
        ]),
    });
    let (ctx, db) = make_ctx(tools.registry());
    let runner = AgentRunner::with_limits(provider, tools, 2, 200_000, TOOL_OUTPUT_MAX_CHARS);
    let (tx, mut rx) = mpsc::channel(16);
    runner
        .run(base_request(), ctx, tx, Arc::new(AtomicBool::new(false)), Arc::new(Mutex::new(std::collections::VecDeque::new())))
        .await
        .unwrap();
    let finish = collect_finish(&mut rx).await.expect("finish");
    match finish {
        StreamEvent::TurnComplete {
            content,
            finish_reason,
            ..
        } => {
            assert_eq!(finish_reason.as_deref(), Some("max_steps"));
            assert!(content.contains("最大工具步数"));
        }
        _ => panic!("unexpected"),
    }
    assert_eq!(counter.load(Ordering::SeqCst), 2);
    let _ = std::fs::remove_file(db);
}

#[tokio::test]
async fn truncates_tool_output_for_model() {
    let mut registry = ToolRegistry::new();
    let huge = "X".repeat(20_000);
    registry.register(Arc::new(CountingTool {
        name: "read_huge",
        read_only: true,
        counter: Arc::new(AtomicUsize::new(0)),
        parallel_peak: Arc::new(AtomicUsize::new(0)),
        active: Arc::new(AtomicUsize::new(0)),
        payload: huge,
    }));
    let tools = Arc::new(ToolManager::new(registry));
    let provider = Arc::new(ScriptedProvider {
        scripts: Mutex::new(vec![
            ProviderTurn {
                content: String::new(),
                tool_calls: vec![tool_call("1", "read_huge")],
            },
            ProviderTurn {
                content: "done".into(),
                tool_calls: vec![],
            },
        ]),
    });
    // Capture messages via a inspecting second provider stream: check request after tool.
    // Instead, re-run with a recorder provider... simpler: low tool_output_max and verify finish.
    let (ctx, db) = make_ctx(tools.registry());
    let recorder = Arc::new(RecordingProvider {
        inner: provider,
        seen_tool_chars: Mutex::new(None),
    });
    let runner = AgentRunner::with_limits(recorder.clone(), tools, 30, 200_000, 100);
    let (tx, mut rx) = mpsc::channel(16);
    runner
        .run(base_request(), ctx, tx, Arc::new(AtomicBool::new(false)), Arc::new(Mutex::new(std::collections::VecDeque::new())))
        .await
        .unwrap();
    let _ = collect_finish(&mut rx).await;
    let seen = recorder
        .seen_tool_chars
        .lock()
        .ok()
        .and_then(|g| *g)
        .expect("tool message seen");
    assert!(seen <= 200, "tool output should be truncated, got {seen}");
    let _ = std::fs::remove_file(db);
}

struct RecordingProvider {
    inner: Arc<ScriptedProvider>,
    seen_tool_chars: Mutex<Option<usize>>,
}

#[async_trait]
impl AIProvider for RecordingProvider {
    fn id(&self) -> &'static str {
        "recording"
    }

    async fn stream(
        &self,
        request: ChatRequest,
        tx: mpsc::Sender<StreamEvent>,
    ) -> Result<(), ProviderError> {
        if let Some(tool_msg) = request
            .messages
            .iter()
            .rev()
            .find(|message| message.role == Role::Tool)
        {
            if let Ok(mut guard) = self.seen_tool_chars.lock() {
                *guard = Some(tool_msg.content.chars().count());
            }
        }
        self.inner.stream(request, tx).await
    }
}

#[tokio::test]
async fn stops_when_token_budget_exhausted() {
    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(CountingTool {
        name: "read_a",
        read_only: true,
        counter: Arc::new(AtomicUsize::new(0)),
        parallel_peak: Arc::new(AtomicUsize::new(0)),
        active: Arc::new(AtomicUsize::new(0)),
        payload: "y".repeat(4_000),
    }));
    let tools = Arc::new(ToolManager::new(registry));
    let provider = Arc::new(ScriptedProvider {
        scripts: Mutex::new(vec![
            ProviderTurn {
                content: "x".repeat(4_000),
                tool_calls: vec![tool_call("1", "read_a")],
            },
            ProviderTurn {
                content: "should not happen".into(),
                tool_calls: vec![],
            },
        ]),
    });
    let (ctx, db) = make_ctx(tools.registry());
    // Very small budget forces stop after first tool-bearing turn.
    let runner = AgentRunner::with_limits(provider, tools, 30, 50, TOOL_OUTPUT_MAX_CHARS);
    let (tx, mut rx) = mpsc::channel(16);
    runner
        .run(base_request(), ctx, tx, Arc::new(AtomicBool::new(false)), Arc::new(Mutex::new(std::collections::VecDeque::new())))
        .await
        .unwrap();
    let finish = collect_finish(&mut rx).await.expect("finish");
    match finish {
        StreamEvent::TurnComplete {
            content,
            finish_reason,
            ..
        } => {
            assert_eq!(finish_reason.as_deref(), Some("max_turn_tokens"));
            assert!(content.contains("token"));
        }
        _ => panic!("unexpected"),
    }
    let _ = std::fs::remove_file(db);
}
