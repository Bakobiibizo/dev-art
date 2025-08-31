# ComfyUI API Proxy

A small Rust service and library that proxies to a running ComfyUI instance and helps you:

- Serve a simple HTTP API over Axum to queue prompts, fetch images, inspect history, and manage stored workflows.
- Construct prompt JSONs from templates using placeholder replacement.
- Optionally poll a local static drive directory for new files.

The crate exposes a library API and also includes a binary that starts the HTTP server.

## Architecture

- `src/api`: Axum routes and handlers. Builds the HTTP router and wires shared state.
- `src/comfyui`: Thin HTTP client for the ComfyUI REST endpoints (`/prompt`, `/view`, `/history`).
- `src/prompt`: Prompt templating utilities. Replaces `{{placeholder}}` strings using an inputs map.
- `src/workflow`: Workflow manager for loading/saving named workflow JSON files in `prompts/`.
- `src/utils`: Background utilities (e.g., static drive poller).
- `src/config.rs`: Env-driven configuration (ComfyUI URL, static drive path).
- `src/error.rs`: Central error type (`AppError`) and alias (`AppResult`).

## Directory Layout

- `prompts/*.json`: Example workflow/prompt templates (`sdxl.json`, `flux.json`).
- `src/main.rs`: Starts the Axum server and spawns the static-drive poller.
- `tests/`: Basic tests (note: some tests rely on networked ComfyUI and may fail offline).

## Configuration

Environment variables (loaded via `dotenv` if present):

- `COMFYUI_URL`: Base URL of your ComfyUI instance. Default: `https://comfy-agentartificial.ngrok.dev`.
- `STATIC_DRIVE_PATH`: Path to a local directory to poll for new files. Default: `./static`.

Example `.env`:

```
COMFYUI_URL=http://127.0.0.1:8188
STATIC_DRIVE_PATH=./static
```

## HTTP API

Base path: `http://127.0.0.1:3000`

- GET `/` — Health check; returns `"ComfyUI API Proxy"`.
- POST `/queue_prompt` — Queue a workflow by name.
  - Body: `{ "workflow": "sdxl" }`
  - Loads `prompts/sdxl.json` and forwards its `prompt` payload to ComfyUI `/prompt`.
  - Response: JSON returned by ComfyUI.
- GET `/get_image?filename=...` — Proxy to ComfyUI `/view` to fetch image bytes.
- GET `/get_history` — Proxy to ComfyUI `/history`.
- POST `/add_workflow` — Add or load a named workflow.
  - Body: `{ "name": "myflow", "workflow": { ... } }` to save; or `{ "name": "sdxl" }` to load existing from `prompts/`.
  - Side effect: writes `prompts/<name>.json` and `workflow.json` (latest).
- GET `/get_node_info?node_type=...` — Return stored node metadata, if any (currently manual via `WorkflowManager::add_node`).
- POST `/construct_prompt` — Apply placeholder substitution to a template using inputs.
  - Body: `{ "template": { ... }, "inputs": { "placeholder": "value" } }`
  - Response: constructed JSON with replacements.

## Library API

- `ComfyUIClient` — Methods: `queue_prompt(Value)`, `get_image(&str)`, `get_history()`.
- `PromptConstructor` — `construct_prompt(template: &Value, inputs: &Value) -> AppResult<Value>`.
- `WorkflowManager` — `add_workflow`, `load_workflow`, `get_node_info`.
- `Config` — `new()`, `dotenv_load()`, `print_env_vars()`.

Import via crate root re-exports:

```
use comfyui_api_proxy::{Config, ComfyUIClient, PromptConstructor, WorkflowManager};
```

## Running

- Ensure `COMFYUI_URL` is set and ComfyUI is reachable.
- Run server:

```
cargo run
```

Server listens on `127.0.0.1:3000` with permissive CORS.

## Notes and Limitations

- Tests in `tests/` currently assume a reachable ComfyUI URL and may fail in offline or CI environments.
- `get_node_info` relies on nodes being added programmatically; there is no discovery yet.
- The static drive poller is a scaffold; it does not currently process files.

## Next: Proper CLI

A structured CLI would make it easier to:

- Start the server with flags (host/port, CORS, paths).
- Interact with ComfyUI directly (queue, history, fetch image).
- Manage workflows (add/load/list/remove).
- Construct prompts from files or inline JSON.

See `AGENTS.md` for a proposed task list and sequencing.
