# Production Ready Checklist

## Release / Repo Discipline
- [ ] local and remote main are intentionally synchronized
- [ ] `cargo build --workspace --locked` passes consistently
- [ ] `cargo test --workspace --locked` passes consistently
- [ ] `cargo clippy --workspace --all-targets --locked` is acceptable for release policy
- [ ] `rust/scripts/release-verify.sh` is the documented release gate
- [ ] release checklist is followed before push/tag

## Runtime / Child Execution
- [ ] child execution policy is primarily runtime/provider-owned
- [ ] backend construction is shared and centralized enough
- [ ] CLI no longer owns critical child backend assembly logic
- [ ] fallback behavior is consistent and traceable
- [ ] child execution tests cover provider-backed and fallback paths well

## Recursive Runtime / RLM
- [ ] recursive runtime modules are structurally maintainable
- [ ] stop reasons are explicit and reliable
- [ ] convergence/no-new-context/failure paths are well-tested
- [ ] boundary-condition tests exist for runtime/token/cost behavior
- [ ] recursive behavior feels dependable for repeated use

## Web / Hybrid Mode
- [ ] web policy handling is consumed by real execution flow
- [ ] approval/degraded semantics are explicit and trustworthy
- [ ] web evidence provenance is clear in final answers
- [ ] web tracing is rich enough to explain what happened
- [ ] the system does not imply web verification when none happened

## Retrieval / RAG
- [ ] local corpus attach/search/slice remains stable
- [ ] ranking quality is strong enough on realistic corpora
- [ ] skip telemetry/reporting is understandable
- [ ] multi-corpus behavior is clear
- [ ] retrieval explainability is good enough for operator debugging
- [ ] next-step retrieval improvements are clearly planned

## Workflow Realism
- [ ] Blender workflow is a convincing showcase
- [ ] repo-analysis workflow is repeatable and reviewable
- [ ] Unreal workflow is honest and practically useful
- [ ] workflow docs match actual runtime behavior
- [ ] validation/demo assets do not create misleading expectations

## Trust / Artifacts
- [ ] trace/corpus artifact expectations are documented
- [ ] compatibility expectations are documented
- [ ] privacy/redaction guidance is present
- [ ] artifact schema/versioning is at least acknowledged and planned

## Operator UX
- [ ] a new operator can follow first-run docs successfully
- [ ] auth/help/profile/corpus/trace flows are understandable
- [ ] help text matches actual behavior
- [ ] docs remain honest about current limitations
