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

## CLI

Binary: `comfyctl`

Common queue flags (mapped into matching node inputs):

- `--seed <int>` `--steps <int>` `--cfg <float>`
- `--sampler-name <string>` `--scheduler <string>` `--denoise <float>`
- `--width <int>` `--height <int>` `--batch-size <int>`
- `--ckpt-name <string>`
- `--text-positive <text>` `--text-negative <text>` (auto-resolved via KSampler links or CLIPTextEncode fallback)
- `--set key=value` repeatable for explicit JSON-path overrides (e.g. `2.inputs.seed=123`)
- `--filename-prefix <string>` defaults to `Derivata`
- `--verbose` prints constructed request body before sending
- `--json` prints raw JSON response (otherwise prints friendly line)

Examples:

```
cargo run --bin comfyctl -- prompt queue \
  --workflow sdxlapi \
  --seed 999 --steps 15 --cfg 6.5 \
  --width 768 --height 768 --batch-size 1 \
  --sampler-name euler --scheduler normal --denoise 1.0 \
  --ckpt-name "SDXL/sd_xl_base_1.0_0.9vae.safetensors" \
  --text-positive "misty forest" --text-negative "blurry"

cargo run --bin comfyctl -- history              # lists prompt_ids
cargo run --bin comfyctl -- history --prompt-id <id>   # lists output filenames
cargo run --bin comfyctl -- history --json       # raw history JSON

cargo run --bin comfyctl -- models categories
cargo run --bin comfyctl -- models list --category checkpoints
cargo run --bin comfyctl -- models checkpoints --json

cargo run --bin comfyctl -- image get <filename> [--out <path>]   # defaults to <STATIC_DRIVE_PATH>/images
```

## HTTP API (friendly by default, JSON optional)

- `GET /history[?json=true][&prompt_id=<id>]`
  - Default: plain text lines. Either a list of `prompt_id`s, or output filenames if `prompt_id` provided.
  - `json=true`: raw JSON history object.

- `GET /models[?json=true]`
  - Default: one category per line.
  - `json=true`: raw JSON array of categories.

- `GET /models/checkpoints[?json=true]`
  - Default: one checkpoint per line (values for `ckpt_name`).
  - `json=true`: raw JSON array.

- `GET /models/:category[?json=true]`
  - Default: one item per line (uses `name` field if present).
  - `json=true`: raw JSON array.

- `POST /queue_prompt`
  - Body supports either:
    - `{ "workflow": "sdxlapi" }` to load `prompts/sdxlapi.json`
    - `{ "prompt": { ... } }` with your full prompt graph
  - Optional top-level params (applied to any nodes with matching inputs):
    - `seed, steps, cfg, sampler_name, scheduler, denoise, width, height, batch_size, ckpt_name, text, text_positive, text_negative`
  - Optional: `sets` (array of `key=value` strings) for explicit path overrides, e.g., `"2.inputs.seed=123"`
  - Optional: `filename_prefix` (default `Derivata`)
  - Optional: `verbose: true` logs the constructed body
