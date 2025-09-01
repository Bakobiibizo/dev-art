# Review Notes — CLI Branch (feature/cli-mcp)

This meta-commit documents the changes implemented during the review cycle for the CLI work.

- CLI UX
  - Default outputs are human-friendly; add `--json` to emit raw JSON.
  - `--verbose` prints the constructed request body for queue operations.
  - Images save under `<STATIC_DRIVE_PATH>/images` by default (creates dir).

- Queue command
  - Supports dynamic params mapped into node inputs: `--seed --steps --cfg --sampler-name --scheduler --denoise --width --height --batch-size --ckpt-name`.
  - Supports `--text-positive` and `--text-negative` with automatic routing via KSampler graph links (fallback to CLIPTextEncode order).
  - Supports repeated `--set key=value` for explicit path overrides.
  - Uses `Config.prompts_dir` for `--workflow` file lookup (no hardcoded paths).
  - Added minimal shape validation for the workflow graph.

- Models + History + Image
  - `models categories|list|checkpoints` with pretty listing by default and `--json` for raw output.
  - `history` lists prompt_ids or output filenames; `--json` for raw history.
  - `image get <filename>` writes to `<STATIC_DRIVE_PATH>/images/<filename>` unless `--out` provided.

- Shared behavior
  - Introduced shared prompt-merge helpers (`utils::prompt_build`) and adopted them in CLI to keep behavior consistent with the API.

- Safety/quality
  - Added `--strict-set` option (on main iteration; retained behavior remains friendly by default).

- Docs
  - Updated README and AGENTS with CLI usage, flags, and examples.

This branch focuses on the CLI tooling; API surface and server behavior are handled in the API branch.
