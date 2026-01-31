# Hive Mind: Phase Amendment Merge

## Task Overview
Merge the Lua phase amendments from `lua-phase-amendments.md` into each of the 5 post-split phase plan files (P1-P5). Each phase plan should be updated to integrate the Lua changes directly, resulting in self-contained documents that no longer require a separate amendments file.

## Key Principles
- **Integrate, don't append**: The amendments should be woven into the existing document structure, not tacked on at the end
- **Update session estimates**: Replace original estimates with amended estimates throughout
- **Add Lua binding sections**: Each phase gains new sub-phases for Lua bindings
- **Update verification sections**: Add Lua integration tests to existing test sections
- **Mark what's removed**: Note removed items (e.g., FSM in P3) clearly
- **Preserve all original Rust implementation details**: Nothing about the core Rust work changes

## Agents
1. **Agent P1** - Merges P1 Combat Core amendments
2. **Agent P2** - Merges P2 Weapons & Feedback amendments
3. **Agent P3** - Merges P3 Enemies & AI amendments
4. **Agent P4** - Merges P4 Level Design amendments
5. **Agent P5** - Merges P5 Editor & Polish amendments

## Coordination Notes
- All agents work independently on their own phase file
- Each agent reads `lua-phase-amendments.md` for their section's content
- Each agent reads and rewrites their phase plan file
- No cross-agent dependencies

## Status
- [ ] Agent P1: Pending
- [ ] Agent P2: Pending
- [ ] Agent P3: Pending
- [ ] Agent P4: Pending
- [ ] Agent P5: Pending
- [ ] Folder cleanup: Pending
- [ ] Final synthesis: Pending
