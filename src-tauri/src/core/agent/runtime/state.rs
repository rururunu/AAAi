use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AgentState {
    Created,
    ContextLoading,
    Planning,
    Executing,
    WaitingTool,
    Observing,
    Reflecting,
    Completed,
    Failed,
    Cancelled,
}

impl AgentState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    pub fn can_transition_to(self, next: Self) -> bool {
        use AgentState::*;
        if self == next {
            return true;
        }
        match self {
            Created => matches!(next, ContextLoading | Planning | Failed | Cancelled),
            ContextLoading => matches!(next, Planning | Failed | Cancelled),
            Planning => matches!(
                next,
                Executing | Observing | Reflecting | Failed | Cancelled
            ),
            Executing => matches!(
                next,
                WaitingTool | Observing | Reflecting | Completed | Failed | Cancelled
            ),
            WaitingTool => matches!(next, Observing | Failed | Cancelled),
            Observing => matches!(next, Reflecting | Failed | Cancelled),
            Reflecting => matches!(next, Planning | Executing | Completed | Failed | Cancelled),
            Completed | Failed | Cancelled => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentTransitionError {
    pub from: AgentState,
    pub to: AgentState,
}

impl std::fmt::Display for AgentTransitionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "invalid agent state transition: {:?} -> {:?}",
            self.from, self.to
        )
    }
}

impl std::error::Error for AgentTransitionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supports_the_v1_state_flow() {
        let flow = [
            AgentState::Created,
            AgentState::Planning,
            AgentState::Executing,
            AgentState::Completed,
        ];
        assert!(flow
            .windows(2)
            .all(|states| states[0].can_transition_to(states[1])));
    }

    #[test]
    fn terminal_states_cannot_restart() {
        assert!(!AgentState::Completed.can_transition_to(AgentState::Planning));
        assert!(!AgentState::Failed.can_transition_to(AgentState::Executing));
        assert!(!AgentState::Cancelled.can_transition_to(AgentState::Planning));
    }
}
