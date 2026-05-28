# kfcode-agent

`kfcode-agent` handles agent definition, registration, execution orchestration, and message handling.

## Responsibilities

- Maintain agent metadata and capability information
- Drive agent execution flow (calling provider / tool / permission)
- Build and transform agent-side message structures

## Module structure

- `agent.rs` – Agent definition and registration
- `executor.rs` – Executor and flow orchestration
- `message.rs` – Message structures and conversion

## Dependencies

- Downstream: `kfcode-provider`, `kfcode-tool`, `kfcode-permission`, `kfcode-plugin`
- Upstream consumers: `kfcode-session`, `kfcode-server`, `kfcode-cli`

## Development notes

- Define clear mode boundaries (responsibilities, tool scope, prompts) before adding new agents
- Executor changes should consider streaming output and interrupt behaviour

## Validation

```bash
cargo check -p kfcode-agent
```
