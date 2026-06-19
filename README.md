# ForgetMeNot
🟢 Do these steps in order. By the end, you'll have a working doorkeeper on your laptop.
🔵
Step 0 — Install the tools (once)
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# uv (fast Python manager)
curl -LsSf https://astral.sh/uv/install.sh | sh
# Docker Desktop — install from docker.com

Step 1 — Get the code & set secrets
git clone https://github.com/forgetmenot/forgetmenot   # or your repo
cd forgetmenot
cp .env.example .env
# open .env and set:
#   FMN_MODE=shadow                 # start safe
#   FMN_UPSTREAM_BASE_URL=https://api.openai.com/v1
#   FMN_UPSTREAM_KEY=sk-...         # your real OpenAI key (lives ONLY here)
#   FMN_DB_PATH=./fmn.db

Step 2 — The easy way: one command (Docker)
docker compose up --build
# This starts: agent (:8787), model-svc (:9000), dashboard (:8788)

Step 3 — Point your app at the doorkeeper
export OPENAI_BASE_URL="http://localhost:8787/v1"
export OPENAI_API_KEY="fnot_sk_local_dev"     # the FMN key, not your real one
# now any normal OpenAI client call goes THROUGH ForgetMeNot

Step 4 — Add a person to forget
forgetmenot registry add \
  --request-id "DPDPA-2026-00441" \
  --jurisdiction IN \
  --name "John Smith" \
  --alias "john.smith@acme.com" \
  --role "Head of R&D" --org "Acme Corp" \
  --scope "chat,logs,vectors"

Step 5 — Try it
curl http://localhost:8787/v1/chat/completions \
  -H "Authorization: Bearer fnot_sk_local_dev" \
  -H "Content-Type: application/json" \
  -d '{"model":"gpt-4o","messages":[{"role":"user","content":"Who is the head of R&D at Acme Corp?"}]}'
# In shadow mode: you see the answer, the ledger logs "would have blocked".
# Switch to enforcement and try again:
forgetmenot mode set enforcement
# Now the answer comes back redacted.

Step 6 — Print the proof
forgetmenot ledger export --request-id DPDPA-2026-00441 --out cert.pdf
# opens a signed certificate that says "0 detected disclosures" + FN-rate

The dev-only way (no Docker, run pieces by hand)
# terminal 1 — model service
cd model-svc && uv venv && source .venv/bin/activate
uv pip install -e ".[dev]" && python -m spacy download en_core_web_lg
uvicorn forgetmenot_model.main:app --port 9000

# terminal 2 — the agent
cd agent && cargo run -- run

# terminal 3 — dashboard
cd dashboard && uv run uvicorn forgetmenot_dash.main:app --port 8788

The no-cloud demo (proves it works with zero API key)
cd examples && python quickstart.py
# Expected:
# ✅ Entity registered: ent_a1b2c3 | probes: 10
# 🟢 The capital of France is Paris. | status: PASS
# 🔴 [Content removed per privacy policy] | status: REDACTED
# 📄 scanned: 2 | detected disclosures: 0 | probes: 10/10
