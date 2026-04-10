# SolUPG Examples

Runnable example integrations that show how to use SolUPG in real code.

Every example in this folder is:

- **Self contained**: clone, install, run
- **Minimal**: no boilerplate, no hidden magic
- **Commented**: every non obvious line is explained in the source

## Available Examples

| Example | Description | Stack |
|---|---|---|
| [`simple-payment`](./simple-payment/) | Register a merchant, create a payment intent, and poll for confirmation | Node.js, TypeScript, `@solupg/sdk` |

More examples are in flight. Pull requests welcome under [`examples/`](./).

## Running an Example

Every example follows the same pattern:

```bash
cd examples/<example-name>
npm install
cp .env.example .env   # edit with your API key and base URL
npm start
```

## What You Need First

Before running any example, you need a running SolUPG stack. Two options:

### Option A: Run the full stack locally with Docker

```bash
# from the repo root
cd services
docker compose build
docker compose up -d
```

The API gateway will be live on `http://localhost:3002`.

### Option B: Point at a hosted SolUPG instance

Set `SOLUPG_BASE_URL` in the example's `.env` to the hosted URL. You still need a valid API key for that instance.

## Getting an API Key

When you call `POST /v1/merchants/register` on a fresh stack, the response includes an `api_key` field. Save that value and use it as `SOLUPG_API_KEY` for subsequent calls. The `simple-payment` example shows the full flow.

## Contributing an Example

Good examples teach one concept clearly. Aim for:

1. Fewer than 150 lines of code
2. A README that explains what the example does and why
3. A `.env.example` with every variable the example needs
4. No extra dependencies beyond `@solupg/sdk` unless strictly necessary

Open a PR under `examples/your-example-name/` and tag it with the `examples` label.
