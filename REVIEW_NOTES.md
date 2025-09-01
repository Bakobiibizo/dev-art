# Review Notes — API Branch (feature/api-mirror-cli)

This meta-commit documents changes made during the review cycle for API parity and refactor work.

- API endpoints
  - Added `GET /history` with friendly text output by default (lists prompt_ids or filenames); add `?json=true` for raw JSON.
  - Added `GET /models`, `GET /models/checkpoints`, and `GET /models/:category` (friendly text by default; `?json=true` for raw JSON).

- queue_prompt refactor
  - Extracted prompt loading/merging into shared helpers in `utils::prompt_build`:
    - `resolve_prompt_root_from_payload` — accepts `{prompt}` or `{workflow}` and uses `prompts_dir`.
    - `apply_overrides_from_payload` — merges `params{}` and `sets[]` consistently.
    - `ensure_defaults_on_root` — ensures `filename_prefix` for SaveImage nodes.
  - Handler now delegates to these helpers for readability and testability.

- Configuration and state
  - `AppState` now carries `prompts_dir`, sourced from `Config`, removing hardcoded paths.
  - `print_env_vars()` includes `PROMPTS_DIR`, `API_HOST`, and `API_PORT` for completeness.

- Server robustness
  - Host/port parsing in `main.rs` now uses safe parsing with fallbacks and warnings (no panics on invalid config).

- Client validation
  - `ComfyUIClient::get_models_in_category` validates the `category` input (alphanumeric, `_`, `-`) before building the URL.

- Utilities improvements
  - `apply_set_path` avoids silently creating intermediate objects (fails when path is missing).
  - `ensure_filename_prefix` identifies SaveImage nodes by `class_type` instead of hardcoded node IDs.

- Docs
  - README updated with CLI/API usage, flags, and endpoints.

This branch focuses on server/API behavior and parity with the CLI while keeping logic DRY across both surfaces.

