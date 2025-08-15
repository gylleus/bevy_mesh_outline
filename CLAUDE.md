# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Bevy plugin that provides mesh outline rendering capabilities using a GPU-based approach. The plugin implements a multi-pass rendering system that creates outlines around 3D meshes by:

1. **Mask Pass**: Renders meshes to create a mask texture identifying outlined objects
2. **Jump Flood Algorithm**: Uses compute shaders to efficiently calculate distance fields for outline generation
3. **Compose Pass**: Combines the original scene with the outline effect

## Core Architecture

### Plugin Structure
- `MeshOutlinePlugin`: Main plugin that registers all systems and render phases
- `MeshOutline`: Component that can be attached to entities with `Mesh3d` to enable outlining
- `OutlineCamera`: Component that marks cameras capable of rendering outlines

### Rendering Pipeline
The outline effect is implemented as a render graph node that runs after the main 3D pass:

1. **mask_pipeline.rs**: Handles the initial mask rendering phase
2. **flood.rs**: Implements jump flood algorithm for distance field calculation  
3. **compose.rs**: Final composition of outline with original scene
4. **render.rs**: Bind group management and render commands
5. **queue.rs**: Queues outline meshes for rendering

### Key Systems
- `extract_outlines_for_batch`: Extracts outline data from main world to render world
- `propagate_outline_changes`: Automatically applies outline changes to child entities
- `prepare_outline_bind_groups`: Creates GPU resources for outline rendering

## Common Commands

### Building and Running
```bash
# Check compilation
cargo check

# Build the project
cargo build

# Run the simple example
cargo run --example simple

# Build specific example
cargo build --example simple
```

### Development
```bash
# Check with all features
cargo check --all-features

# Run tests (if any exist)
cargo test
```

## Working with Examples

The `examples/simple.rs` demonstrates basic usage:
- Creates a scene with a rotating cube that has an outline
- Q/W keys control outline width
- UI in top-right shows current width and controls

When modifying examples, ensure the proper Bevy component structure is maintained and that outline-related components (`MeshOutline`, `OutlineCamera`) are properly applied.

## Shader Development

Shaders are located in `src/shaders/` and loaded as internal assets:
- `mask.wgsl`: Renders outline mask
- `flood.wgsl`: Jump flood distance calculation
- `compose_output.wgsl`: Final outline composition

When modifying shaders, ensure the uniform buffer layouts in corresponding Rust files match the WGSL definitions.