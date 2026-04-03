#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeBudget {
    pub max_depth: Option<u32>,
    pub max_iterations: Option<u32>,
    pub max_subcalls: Option<u32>,
    pub max_runtime_ms: Option<u64>,
    pub max_prompt_tokens: Option<u32>,
    pub max_completion_tokens: Option<u32>,
    pub max_cost_usd: Option<f64>,
}

impl RuntimeBudget {
    #[must_use]
    pub fn unlimited() -> Self {
        Self {
            max_depth: None,
            max_iterations: None,
            max_subcalls: None,
            max_runtime_ms: None,
            max_prompt_tokens: None,
            max_completion_tokens: None,
            max_cost_usd: None,
        }
    }

    #[must_use]
    pub fn slice_for_child(&self, request: BudgetSliceRequest) -> Self {
        Self {
            max_depth: self
                .max_depth
                .map(|value| value.saturating_sub(request.depth_cost)),
            max_iterations: min_some_u32(self.max_iterations, request.max_iterations),
            max_subcalls: self
                .max_subcalls
                .map(|value| value.saturating_sub(request.subcall_cost)),
            max_runtime_ms: min_some_u64(self.max_runtime_ms, request.max_runtime_ms),
            max_prompt_tokens: min_some_u32(self.max_prompt_tokens, request.max_prompt_tokens),
            max_completion_tokens: min_some_u32(
                self.max_completion_tokens,
                request.max_completion_tokens,
            ),
            max_cost_usd: min_some_f64(self.max_cost_usd, request.max_cost_usd),
        }
    }

    #[must_use]
    pub fn exhausted_by(&self, usage: &RuntimeBudgetUsage) -> Option<BudgetStopReason> {
        if let Some(limit) = self.max_depth {
            if usage.depth > limit {
                return Some(BudgetStopReason::DepthExceeded {
                    limit,
                    actual: usage.depth,
                });
            }
        }
        if let Some(limit) = self.max_iterations {
            if usage.iterations > limit {
                return Some(BudgetStopReason::IterationsExceeded {
                    limit,
                    actual: usage.iterations,
                });
            }
        }
        if let Some(limit) = self.max_subcalls {
            if usage.subcalls > limit {
                return Some(BudgetStopReason::SubcallsExceeded {
                    limit,
                    actual: usage.subcalls,
                });
            }
        }
        if let Some(limit) = self.max_runtime_ms {
            if usage.runtime_ms > limit {
                return Some(BudgetStopReason::RuntimeExceeded {
                    limit,
                    actual: usage.runtime_ms,
                });
            }
        }
        if let Some(limit) = self.max_prompt_tokens {
            if usage.prompt_tokens > limit {
                return Some(BudgetStopReason::PromptTokensExceeded {
                    limit,
                    actual: usage.prompt_tokens,
                });
            }
        }
        if let Some(limit) = self.max_completion_tokens {
            if usage.completion_tokens > limit {
                return Some(BudgetStopReason::CompletionTokensExceeded {
                    limit,
                    actual: usage.completion_tokens,
                });
            }
        }
        if let Some(limit) = self.max_cost_usd {
            if usage.cost_usd > limit {
                return Some(BudgetStopReason::CostExceeded {
                    limit,
                    actual: usage.cost_usd,
                });
            }
        }
        None
    }
}

impl Default for RuntimeBudget {
    fn default() -> Self {
        Self::unlimited()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BudgetSliceRequest {
    pub depth_cost: u32,
    pub subcall_cost: u32,
    pub max_iterations: Option<u32>,
    pub max_runtime_ms: Option<u64>,
    pub max_prompt_tokens: Option<u32>,
    pub max_completion_tokens: Option<u32>,
    pub max_cost_usd: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RuntimeBudgetUsage {
    pub depth: u32,
    pub iterations: u32,
    pub subcalls: u32,
    pub runtime_ms: u64,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub cost_usd: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BudgetStopReason {
    DepthExceeded { limit: u32, actual: u32 },
    IterationsExceeded { limit: u32, actual: u32 },
    SubcallsExceeded { limit: u32, actual: u32 },
    RuntimeExceeded { limit: u64, actual: u64 },
    PromptTokensExceeded { limit: u32, actual: u32 },
    CompletionTokensExceeded { limit: u32, actual: u32 },
    CostExceeded { limit: f64, actual: f64 },
}

fn min_some_u32(parent: Option<u32>, requested: Option<u32>) -> Option<u32> {
    match (parent, requested) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn min_some_u64(parent: Option<u64>, requested: Option<u64>) -> Option<u64> {
    match (parent, requested) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn min_some_f64(parent: Option<f64>, requested: Option<f64>) -> Option<f64> {
    match (parent, requested) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{BudgetSliceRequest, BudgetStopReason, RuntimeBudget, RuntimeBudgetUsage};

    #[test]
    fn child_budget_never_exceeds_parent_limits() {
        let parent = RuntimeBudget {
            max_depth: Some(3),
            max_iterations: Some(10),
            max_subcalls: Some(5),
            max_runtime_ms: Some(60_000),
            max_prompt_tokens: Some(8_000),
            max_completion_tokens: Some(2_000),
            max_cost_usd: Some(1.25),
        };

        let child = parent.slice_for_child(BudgetSliceRequest {
            depth_cost: 1,
            subcall_cost: 1,
            max_iterations: Some(99),
            max_runtime_ms: Some(120_000),
            max_prompt_tokens: Some(20_000),
            max_completion_tokens: Some(4_000),
            max_cost_usd: Some(5.0),
        });

        assert_eq!(child.max_depth, Some(2));
        assert_eq!(child.max_iterations, Some(10));
        assert_eq!(child.max_subcalls, Some(4));
        assert_eq!(child.max_runtime_ms, Some(60_000));
        assert_eq!(child.max_prompt_tokens, Some(8_000));
        assert_eq!(child.max_completion_tokens, Some(2_000));
        assert_eq!(child.max_cost_usd, Some(1.25));
    }

    #[test]
    fn slice_request_can_tighten_unbounded_parent_fields() {
        let parent = RuntimeBudget::unlimited();
        let child = parent.slice_for_child(BudgetSliceRequest {
            depth_cost: 1,
            subcall_cost: 1,
            max_iterations: Some(6),
            max_runtime_ms: Some(30_000),
            max_prompt_tokens: Some(4_000),
            max_completion_tokens: Some(1_000),
            max_cost_usd: Some(0.75),
        });

        assert_eq!(child.max_depth, None);
        assert_eq!(child.max_subcalls, None);
        assert_eq!(child.max_iterations, Some(6));
        assert_eq!(child.max_runtime_ms, Some(30_000));
        assert_eq!(child.max_prompt_tokens, Some(4_000));
        assert_eq!(child.max_completion_tokens, Some(1_000));
        assert_eq!(child.max_cost_usd, Some(0.75));
    }

    #[test]
    fn exhaustion_reports_first_triggered_limit_deterministically() {
        let budget = RuntimeBudget {
            max_depth: Some(2),
            max_iterations: Some(4),
            max_subcalls: Some(3),
            max_runtime_ms: Some(10_000),
            max_prompt_tokens: Some(1_000),
            max_completion_tokens: Some(500),
            max_cost_usd: Some(0.5),
        };

        let usage = RuntimeBudgetUsage {
            depth: 3,
            iterations: 9,
            subcalls: 7,
            runtime_ms: 40_000,
            prompt_tokens: 2_000,
            completion_tokens: 900,
            cost_usd: 4.0,
        };

        assert_eq!(
            budget.exhausted_by(&usage),
            Some(BudgetStopReason::DepthExceeded {
                limit: 2,
                actual: 3,
            })
        );
    }

    #[test]
    fn exhaustion_returns_none_when_usage_is_within_limits() {
        let budget = RuntimeBudget {
            max_depth: Some(2),
            max_iterations: Some(4),
            max_subcalls: Some(3),
            max_runtime_ms: Some(10_000),
            max_prompt_tokens: Some(1_000),
            max_completion_tokens: Some(500),
            max_cost_usd: Some(0.5),
        };

        let usage = RuntimeBudgetUsage {
            depth: 2,
            iterations: 4,
            subcalls: 3,
            runtime_ms: 10_000,
            prompt_tokens: 1_000,
            completion_tokens: 500,
            cost_usd: 0.5,
        };

        assert_eq!(budget.exhausted_by(&usage), None);
    }
}
