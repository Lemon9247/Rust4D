# Hive-Mind: Swarm C - Game Engine & Genre Feature Analysis
**Date**: 2026-01-30

## Shared Context
Rust4D is a 4D game engine. We want to build a **4D boomer shooter** - think Doom/Quake but in 4D space.

The engine currently has:
- 4D math (Vec4, Rotor4 rotations, convex shapes)
- 4D physics (rigid bodies, collision, gravity)
- 4D→3D rendering via compute shader hyperplane slicing
- Scene management, asset caching, entity hierarchy
- Basic camera controller with WASD + mouse + 4D rotation
- Config system (TOML), scene serialization (RON)

### Research Focus
Agent C1: Study Unity and Godot to identify missing engine features
Agent C2: Study the boomer shooter genre to identify required gameplay systems

Both: Think about what's UNIQUE to 4D - where do standard features need adaptation?

## Agent Discoveries
(Agents: write key findings here for cross-pollination)
