# Hive Mind: Config Consolidation Open Questions Analysis

## Task Overview
Analyze three open questions from the config consolidation report to determine which should be implemented:

1. **Camera pitch_limit**: Should `camera.pitch_limit` be connected to Camera4D? Would require making Camera4D::PITCH_LIMIT configurable.

2. **Physics floor_y**: Is `physics.floor_y` still needed? Scenes define their own floor positions.

3. **Rendering max_triangles**: Should `rendering.max_triangles` be connected to SlicePipeline?

## Agents
1. **Camera Agent** - Investigates Camera4D implementation and pitch_limit usage
2. **Physics Agent** - Investigates floor_y usage across scenes and physics system
3. **Rendering Agent** - Investigates SlicePipeline and max_triangles configuration

## Coordination Notes
- Each agent should write findings to separate markdown files in this folder
- Focus on: current usage, code locations, effort required, value of making configurable
- **Cross-agent coordination**: Agents can read and write to this file to share discoveries

## Questions for Discussion
(Agents can add questions here - other agents should check this section and respond)

## Status
- [x] Camera Agent: Complete
- [x] Physics Agent: Complete
- [x] Rendering Agent: Complete
- [x] Final synthesis: Complete

## Reports Generated
- `final-synthesis-report.md` - Combined findings and recommendations

## Key Findings

### Camera pitch_limit: SKIP
- Hardcoded `PITCH_LIMIT = 1.553` rad (~89 degrees) in `camera4d.rs:49`
- Config value `pitch_limit = 89.0` degrees - values already match
- Low impact, would require API changes to Camera4D
- **Recommendation**: Skip - values match, not worth the effort

### Physics floor_y: REMOVE
- `floor_y` is defined in config but **never used anywhere in code**
- Scenes define their own floors via `y` in Hyperplane entities
- `to_physics_config()` explicitly excludes floor_y
- Examples hardcode floor values directly
- **Recommendation**: Remove - causes confusion, enforces wrong pattern

### Rendering max_triangles: IMPLEMENT
- **10x MISMATCH**: Config says 1,000,000 but code uses 100,000
- Hardcoded in `types.rs:208` as `MAX_OUTPUT_TRIANGLES = 100_000`
- Buffer exhaustion causes **silent data corruption** (no bounds check in shader)
- Memory: 100K = 14.4 MB, 1M = 144 MB
- **Recommendation**: Implement - fixes clear bug, enables customization
