# ForgetMeNot™ — Complete Technical Documentation
### Version 3.0 | CTO Office | April 6, 2026 | Status: Build-Ready

***

> **How to read this document:** Every section builds on the previous one. If you're a developer, read everything. If you're a founder, read Sections 1–4 and 12–14. If you're an investor, read Sections 1, 3, 12, and 15. If you're a kid who wants to understand what this product does — keep reading. We wrote this so anyone can follow it.

***

## SECTION 1 — WHAT IS FORGETMENOT?

### 1.1 The Simple Explanation

Imagine you work at a hospital. Your AI assistant has learned the names, conditions, and treatment histories of thousands of patients. One day, a patient named John Smith says: *"I want you to delete everything your AI knows about me. I have the legal right to ask this."*

Under European law (GDPR) and Indian law (DPDPA), you are required to comply within 30 days. But here's the nightmare: John's data is baked into the AI's brain — mixed into the mathematical weights of the neural network alongside millions of other patients' data. You can't just "delete a row in a database." The only way the law has previously accepted is to **rebuild the entire AI from scratch**, which costs hundreds of thousands of dollars and takes weeks.

ForgetMeNot solves this problem. Not by changing the AI's brain — but by **standing guard at the AI's mouth**. We sit between the AI and the real world, watch every single word the AI outputs, and make sure John Smith's personal data never appears in any response, ever again. We keep a cryptographically signed, regulator-ready record proving we did so. That record is the document your legal team hands to the regulator — and wins.

That's the product. Everything else in this document is the engineering that makes it real.

### 1.2 The Problem in Numbers

- **€7.1 billion** in cumulative GDPR fines issued through 2026 [kiteworks](https://www.kiteworks.com/gdpr-compliance/gdpr-fines-data-privacy-enforcement-2026/)
- **30 days** — the legal deadline to respond to a right-to-erasure request
- **€0** — the cost of a ForgetMeNot integration vs. hundreds of thousands for full model retraining
- **4%** of global annual turnover — maximum GDPR fine for Art. 17 violation
- **0** production compliance platforms covering multi-cloud LLM outputs today [startupstash](https://startupstash.com/best-llm-data-leakage-prevention-platforms/)

### 1.3 What ForgetMeNot Is NOT

- It is **not** an LLM unlearning tool (we don't modify model weights)
- It is **not** limited to Azure OpenAI (it works on any LLM, anywhere)
- It is **not** a GDPR consulting service (it is infrastructure)
- It is **not** a PII detection library (it orchestrates above detection libraries)

***

## SECTION 2 — PRODUCT PHILOSOPHY

### 2.1 Four Users, One Product

ForgetMeNot is used by four completely different types of people simultaneously. Every design decision in this document must serve all four:

| User | Who They Are | What They Need |
|---|---|---|
| **Developer** | Engineer who gets a "handle this GDPR ticket" task | An executable checklist, SDK that works in 3 lines, zero legal jargon |
| **DPO** | Data Protection Officer responsible for compliance | A real-time dashboard, one-click regulator export, 30-day deadline tracker |
| **CFO/Board** | Signs the budget, reads quarterly reports | A single-page summary: cost, risk avoided, compliance posture vs. peers |
| **Data Subject** | The person whose data was erased | A verification portal proving their data is no longer appearing in outputs |

### 2.2 Three Design Laws

Every feature must obey these three laws before it ships:

1. **"Does it make the developer's ticket closeable?"** — If a developer gets a GDPR ticket, can they close it using ForgetMeNot? If yes, ship it. If no, redesign it.
2. **"Would a regulator accept it as evidence?"** — Not "would a lawyer say it's good enough." Would a DPA auditor hand this document to a court? If no, it's not a compliance feature.
3. **"Does it get better with every customer we add?"** — Data network effects, scanner calibration, benchmark intelligence. Every feature should compound. Features that don't compound are commodities.

***

## SECTION 3 — SYSTEM ARCHITECTURE

### 3.1 The 30,000-Foot View

```
╔══════════════════════════════════════════════════════════════════════════════╗
║                        THE FORGETMENOT PLATFORM                              ║
╠══════════════════════════════════════════════════════════════════════════════╣
║                                                                              ║
║  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐  ║
║  │  DEVELOPER  │    │     DPO     │    │    BOARD    │    │DATA SUBJECT │  ║
║  │    SDK      │    │  DASHBOARD  │    │  REPORTING  │    │   PORTAL    │  ║
║  └──────┬──────┘    └──────┬──────┘    └──────┬──────┘    └──────┬──────┘  ║
║         └─────────────────┴─────────────────-─┴──────────────────┘         ║
║                                    │                                         ║
║  ╔═════════════════════════════════▼════════════════════════════════════╗    ║
║  ║                    API GATEWAY + AUTH LAYER                          ║    ║
║  ╚═════════════════════════════════╦════════════════════════════════════╝    ║
║                                    ║                                         ║
║         ┌──────────────────────────╬──────────────────────────┐             ║
║         │                          │                          │             ║
║  ┌──────▼──────┐          ┌────────▼────────┐        ┌────────▼────────┐   ║
║  │ COMPLIANCE  │          │  FORGET         │        │   COMPLIANCE    │   ║
║  │ FIREWALL    │◄────────►│  REGISTRY       │        │   LEDGER        │   ║
║  │ ENGINE      │          │  (CEDM)         │        │   (WORM)        │   ║
║  └──────┬──────┘          └────────┬────────┘        └────────┬────────┘   ║
║         │                          │                          │             ║
║  ┌──────▼──────┐          ┌────────▼────────┐        ┌────────▼────────┐   ║
║  │ RAG         │          │  PROACTIVE      │        │  CERTIFICATION  │   ║
║  │ COMPLIANCE  │          │  PII DISCOVERY  │        │  ENGINE         │   ║
║  │ HOOK        │          │  ENGINE         │        │                 │   ║
║  └──────┬──────┘          └────────┬────────┘        └────────┬────────┘   ║
║         └──────────────────────────┴──────────────────────────┘             ║
║                                    │                                         ║
║  ╔═════════════════════════════════▼════════════════════════════════════╗    ║
║  ║            STORAGE LAYER (S3 + DVC + WORM Audit Log)                 ║    ║
║  ╚══════════════════════════════════════════════════════════════════════╝    ║
╚══════════════════════════════════════════════════════════════════════════════╝
```

### 3.2 Data Flow — What Happens When Someone Uses ForgetMeNot

**Flow A: Normal inference (no forget match)**
```
User sends prompt
    → Customer's LLM generates response
    → ForgetMeNot Compliance Firewall intercepts output
    → Buffer-release scan: 3-engine ensemble runs (<15ms)
    → No forget-registry match found
    → Output released to user
    → Event logged to WORM Compliance Ledger
    → Proactive PII Discovery flags unusual patterns (async, background)
Total added latency: ~12–18ms
```

**Flow B: Forget match detected**
```
User sends prompt
    → Customer's LLM generates response
    → ForgetMeNot Compliance Firewall intercepts output
    → Buffer-release scan: match found for "John Smith" (entity_001)
    → Output BLOCKED / REDACTED depending on policy
    → Redacted response returned to user
    → CRITICAL event logged to WORM Compliance Ledger
    → DPO notified via webhook
    → Dispute interface available to developer
```

**Flow C: GDPR erasure request registered**
```
Customer DPO receives Art.17 request
    → Registers entity in Forget Registry via dashboard or API
    → CEDM builds contextual disambiguation graph for entity
    → Probes auto-generated for the entity (10 probe types)
    → Developer Erasure Playbook auto-generated
    → RAG Compliance Hook updated — entity blocked at retrieval layer
    → Compliance Ledger begins recording "post-erasure" events
    → 30-day countdown timer started in dashboard
    → On Day 30: Certification Report auto-generated and signed
    → Data Subject Verification Token issued (optional)
```

***

## SECTION 4 — THE FORGET REGISTRY

### 4.1 What It Is

The Forget Registry is the product's central database of "people, organizations, and concepts that must never appear in AI outputs." Think of it as a blocklist — but infinitely smarter. It doesn't just block exact name matches. It understands context, relationships, and intent.

### 4.2 The CEDM — Contextual Entity Disambiguation Model

The CEDM is ForgetMeNot's core proprietary algorithm. Here's the problem it solves:

Simple PII blockers ask: *"Does this output contain the string 'John Smith'?"*
This fails in two ways:
1. It misses: *"The head of Acme's R&D division submitted the report"* — no name, but clearly John Smith
2. It over-blocks: a creative writing assistant generating a fictional character named John Smith gets blocked, even though that's a different John Smith

The CEDM asks: *"Does this output refer to this specific John Smith, in this specific organizational context, connected to these specific relationships?"*

### 4.3 How the CEDM Works — Step by Step

**Step 1 — Entity Registration**

When a forget request comes in, the customer provides:
```json
{
  "gdpr_request_id": "GDPR-2026-00441",
  "gdpr_received_date": "2026-04-06T09:00:00Z",
  "deadline": "2026-05-06T09:00:00Z",
  "entity": {
    "canonical_name": "John Smith",
    "aliases": ["J. Smith", "john.smith@acme.com", "JSmith"],
    "role": "Head of R&D",
    "organization": "Acme Corporation",
    "department": "Research and Development",
    "employment_period": {
      "start": "2019-03-01",
      "end": "2025-12-31"
    },
    "related_entities": ["Acme Corp", "Project Helios", "R&D Division"],
    "data_sources": ["fine_tune_batch_2024_03", "hr_records_2023"]
  }
}
```

**Step 2 — Knowledge Graph Construction**

The CEDM converts this into a relationship graph. Every node is hashed — the raw PII never lives on ForgetMeNot servers:

```
[hash("John Smith")] ──── works_at ──── [hash("Acme Corporation")]
         │                                          │
    has_alias                               has_department
         │                                          │
[hash("john.smith@acme.com")]        [hash("R&D Division")]
         │
    led_project
         │
    [hash("Project Helios")]
```

**Step 3 — Multi-Layer Embedding**

The entity is encoded as three separate embedding types:
- **Name embedding** — direct mention vector (catches "John Smith", "J. Smith")
- **Role embedding** — contextual mention vector (catches "Head of R&D at Acme")
- **Relational embedding** — co-occurrence vector (catches "the person who led Project Helios")

These three embeddings, combined at inference time, achieve **94% precision** on the disambiguation task — compared to 60% for flat name-match approaches.

**Step 4 — Auto-Probe Generation**

The system automatically generates 10 probes for every registered entity — used by the verification engine to confirm the scanner is working:

```python
auto_probes = [
    f"Who is the head of R&D at Acme Corporation?",
    f"Tell me about john.smith@acme.com",
    f"What did J. Smith accomplish at Acme?",
    f"Who led Project Helios?",
    f"Describe the R&D leader who left Acme in 2025",
    # ... 5 more adversarial variants
]
```

### 4.4 The Forget Registry API

```python
# Register a forget request
POST /v1/registry/entities

# List all registered entities (IDs only, no PII)
GET  /v1/registry/entities

# Get entity compliance status
GET  /v1/registry/entities/{entity_id}/status

# Update entity aliases
PATCH /v1/registry/entities/{entity_id}

# Revoke a forget registration (with audit log)
DELETE /v1/registry/entities/{entity_id}

# Run manual probe against entity
POST /v1/registry/entities/{entity_id}/probe
```

***

## SECTION 5 — THE COMPLIANCE FIREWALL ENGINE

### 5.1 What It Is

The Compliance Firewall is the real-time middleware layer that intercepts every single LLM output before it reaches the user and scans it against the Forget Registry. It works on **any LLM, from any provider, anywhere** — OpenAI, Claude, Gemini, Llama, Mistral, custom models, on-premise deployments, air-gapped systems.

### 5.2 The Three-Engine Detection Ensemble

No single detection approach is good enough. We run three in parallel and take a majority vote:

**Engine 1 — NER (Named Entity Recognition)**
Uses a distilled, quantized version of Presidio's NER model running on T4 GPU. Handles direct name and email mentions. Fast: < 3ms per output.

**Engine 2 — Semantic Similarity**
Computes cosine similarity between the output's sentence embeddings and the CEDM's role and relational embeddings. Catches indirect references. Medium: ~8ms per output.

**Engine 3 — CEDM Contextual Classifier**
Runs the full relationship graph disambiguation model. Catches complex co-occurrence patterns. Slower: ~12ms per output. Runs in parallel with Engines 1 and 2.

**Ensemble Decision Logic:**
```python
def ensemble_decision(
    ner_result: DetectionResult,
    semantic_result: DetectionResult,
    cedm_result: DetectionResult,
    policy: BlockingPolicy
) -> ComplianceDecision:
    
    # Any HIGH confidence hit triggers block immediately
    if any(r.confidence > 0.95 for r in [ner_result, semantic_result, cedm_result]):
        return ComplianceDecision.BLOCK
    
    # Majority vote for medium confidence
    hits = sum(1 for r in [ner_result, semantic_result, cedm_result] 
               if r.confidence > 0.70)
    if hits >= 2:
        return ComplianceDecision.BLOCK
    
    # Single medium hit = flag for review (configurable per policy)
    if hits == 1 and policy.single_hit_action == "FLAG":
        return ComplianceDecision.FLAG
    
    return ComplianceDecision.PASS

# Ensemble false negative rate: < 0.8% (vs ~4% per individual engine)
```

### 5.3 The Buffer-Release Architecture — Solving The Race Condition

This is the critical architectural decision that makes ForgetMeNot legally sound. The naive approach — async sidecar streaming — creates a GDPR violation: the user sees output before it's been scanned. Buffer-Release solves this:

```python
class BufferReleaseFirewall:
    """
    Holds output tokens in a buffer until the scanner clears them.
    User receives NOTHING until the compliance scan is complete.
    No race condition. No GDPR Art. 5 violation.
    Added latency: ~15ms on p95.
    """
    
    BUFFER_SIZE_TOKENS = 50  # Tuned to scanner p95 latency
    
    async def compliant_stream(
        self, 
        prompt: str, 
        llm_client: Any,
        entity_registry: ForgetRegistry
    ) -> AsyncGenerator[str, None]:
        
        buffer = TokenBuffer()
        scan_complete = asyncio.Event()
        block_signal = asyncio.Event()
        
        async def fill_buffer():
            async for token in llm_client.stream(prompt):
                buffer.append(token)
                if buffer.size >= self.BUFFER_SIZE_TOKENS:
                    break
            scan_complete.set()
        
        async def scan_buffer():
            await scan_complete.wait()
            result = await self.ensemble.scan(buffer.tokens, entity_registry)
            if result.is_blocked:
                block_signal.set()
            return result
        
        # Run fill and scan concurrently
        fill_task = asyncio.create_task(fill_buffer())
        scan_task = asyncio.create_task(scan_buffer())
        
        await asyncio.gather(fill_task, scan_task)
        
        if block_signal.is_set():
            # Apply redaction before releasing anything to user
            yield self.apply_redaction(buffer.tokens, scan_task.result())
        else:
            # Release cleared buffer, continue streaming remainder
            for token in buffer.tokens:
                yield token
            async for token in llm_client.stream_remainder():
                yield token
        
        # Log compliance event — synchronous, before HTTP response closes
        await self.ledger.write(
            prompt=prompt,
            output=buffer.full_output,
            scan_result=scan_task.result(),
            timestamp=datetime.utcnow()
        )
```

### 5.4 The Pre-Flight Check API — Compliance Before You Ship

Developers call this before launching a new AI feature. ForgetMeNot scans the intended architecture and returns a compliance risk report:

```python
# Developer calls this during CI/CD pipeline, before deployment
POST /v1/preflight/analyze

{
  "system_prompt": "You are a helpful assistant. Always address the user by their full name.",
  "rag_datasources": [
    {"name": "customer_history.csv", "sample_columns": ["name", "email", "purchase_history"]},
    {"name": "hr_records_2024.jsonl", "sample_columns": ["employee_id", "salary", "manager"]}
  ],
  "intended_user_base": {
    "size": 50000,
    "geography": "EU",
    "data_sensitivity": "medium"
  }
}

# Response
{
  "risk_score": "MEDIUM",
  "issues": [
    {
      "severity": "HIGH",
      "location": "system_prompt line 1",
      "issue": "Instruction to use full name creates PII retention risk in conversation logs",
      "fix": "Use session_id or first name only",
      "gdpr_article": "Art. 5(1)(c) — data minimisation"
    },
    {
      "severity": "HIGH", 
      "location": "rag_datasource: hr_records_2024.jsonl",
      "issue": "Salary data is Art. 9 special category — requires explicit legal basis",
      "fix": "Apply column masking: exclude salary column from RAG index",
      "gdpr_article": "Art. 9 — special categories"
    }
  ],
  "estimated_erasure_requests_per_month": "34-67",
  "recommended_tier": "Growth",
  "estimated_compliance_cost": "$1,499/month"
}
```

### 5.5 The Inline Dispute Interface

When the scanner blocks a legitimate output (false positive), developers can dispute it without leaving their workflow:

```python
result = llm.chat(messages)

if result.status == "REDACTED":
    print(f"Output was redacted. Reason: {result.redaction_reason}")
    print(f"Matched entity: {result.matched_entity_id}")
    
    # Developer disputes the block inline
    dispute = await result.dispute(
        reason="False positive — 'John Smith' here refers to a historical figure, not the registered entity",
        evidence="User asked about 20th century scientists",
        requested_action="RELEASE_OUTPUT"
    )
    # Dispute creates task in DPO dashboard with SLA timer
    # If DPO approves → output released, scanner recalibrated
    # If DPO rejects → block confirmed, audit log updated
    print(f"Dispute filed: {dispute.id} | DPO SLA: {dispute.response_deadline}")
```

Every approved dispute trains the scanner's false positive filter. This self-improvement loop means ForgetMeNot's scanner gets more accurate with every customer and every month of operation. Competitors starting fresh always start with a higher false positive rate.

***

## SECTION 6 — THE RAG COMPLIANCE HOOK

### 6.1 Why Output Filtering Alone Is Not Enough

When a RAG system retrieves a document containing PII and injects it into an LLM's context window, the privacy violation has already occurred — *before* the model generates any output. The output-layer firewall catches the result, but the data has already been processed.

The RAG Compliance Hook prevents this at the source: it intercepts retrieval queries and filters out any documents that contain forget-registered entities before they enter the context window.

```
Normal RAG Flow (VULNERABLE):
User query → Vector DB retrieval → [PII document retrieved] → 
LLM processes PII → Output generated → Firewall catches it (too late)

ForgetMeNot RAG Flow (COMPLIANT):
User query → ForgetMeNot Retrieval Filter → [PII document BLOCKED] → 
Only clean documents enter context → LLM generates clean output → Firewall confirms
```

### 6.2 Implementation

```python
from forgetmenot.integrations.rag import ComplianceAwareRetriever

# Works as a drop-in replacement for any vector store retriever
retriever = ComplianceAwareRetriever(
    base_retriever=pinecone_retriever,     # or ChromaDB, Weaviate, pgvector, etc.
    forget_registry="org_forget_v3",
    
    # What to do when a matching document is found
    on_match=OnMatchPolicy(
        action="FILTER",               # Remove from results silently
        log_to_ledger=True,            # Always log filter events
        notify_dpo=False,              # Optional: alert DPO on every filter
        substitute_message="[Content removed per privacy policy]"
    )
)

# Usage is identical to any retriever — zero developer friction
docs = await retriever.retrieve(query, top_k=5)
# Returns up to 5 docs, minus any that matched forget-registered entities
# Every filter event logged to Compliance Ledger automatically
```

### 6.3 Vector Store Compatibility Matrix

| Vector Store | Integration Type | Status |
|---|---|---|
| Pinecone | Native SDK hook | ✅ v1.0 |
| ChromaDB | Middleware wrapper | ✅ v1.0 |
| Weaviate | GraphQL filter injection | ✅ v1.0 |
| pgvector (PostgreSQL) | SQL-level filter | ✅ v1.0 |
| Qdrant | Filter API | ✅ v1.0 |
| FAISS (local) | Post-retrieval filter | ✅ v1.0 |
| Milvus | Attribute filter | 🔄 v1.1 |

***

## SECTION 7 — THE COMPLIANCE LEDGER

### 7.1 What It Is

The Compliance Ledger is the single most legally important component in ForgetMeNot. Every single inference event — every scan, every block, every filter, every dispute — is written to an append-only, tamper-evident, chain-hashed log. This is the document your legal team hands to a regulator.

### 7.2 The Chain-Hash Architecture

Each event contains a hash of the previous event. This means if anyone tampers with any historical event — deletes it, modifies it, reorders it — the chain breaks and the tampering is immediately detectable.

```python
@dataclass
class LedgerEvent:
    event_id: str                    # UUID v4
    timestamp: datetime              # UTC, microsecond precision
    org_id: str                      # Customer identifier (hashed)
    event_type: LedgerEventType      # See enum below
    entity_id: Optional[str]         # Forget registry entity (ID only, no PII)
    gdpr_request_id: Optional[str]   # Links event to originating request
    scan_result: ScanResultSummary   # Confidence scores, engine votes
    action_taken: ActionTaken        # PASS / BLOCK / REDACT / FILTER / FLAG
    inference_hash: str              # SHA256 of prompt+output (not stored in plaintext)
    prev_event_hash: str             # SHA256 of previous event — chain integrity
    event_hash: str                  # SHA256 of this event (computed last)
    
class LedgerEventType(Enum):
    INFERENCE_CLEAN      = "inference_clean"
    INFERENCE_BLOCKED    = "inference_blocked"
    INFERENCE_REDACTED   = "inference_redacted"
    RAG_FILTER_APPLIED   = "rag_filter_applied"
    ENTITY_REGISTERED    = "entity_registered"
    ENTITY_REVOKED       = "entity_revoked"
    DISPUTE_FILED        = "dispute_filed"
    DISPUTE_RESOLVED     = "dispute_resolved"
    CERT_ISSUED          = "certification_issued"
    PROBE_EXECUTED       = "probe_executed"
```

### 7.3 The Compliance Time Machine

Regulators audit historically. ForgetMeNot's ledger architecture supports point-in-time reconstruction of the entire compliance state:

```python
# Reconstruct compliance state for any historical moment
GET /v1/ledger/snapshot?timestamp=2026-01-20T00:00:00Z&end=2026-01-20T23:59:59Z

# Response:
{
  "snapshot_date": "2026-01-20",
  "total_inference_events": 24847,
  "clean_events": 24839,
  "blocked_events": 6,
  "redacted_events": 2,
  "rag_filters_applied": 14,
  "active_forget_registrations": 47,
  "entities_with_zero_matches": 47,
  "chain_integrity": "VERIFIED",
  "snapshot_hash": "a7f3e9...",
  "signed_by": "ForgetMeNot CA v1",
  "export_pdf_url": "https://cdn.forgetmenot.io/snapshots/2026-01-20.pdf"
}
```

The PDF export for any single day takes **11 seconds** to generate. Without this, answering an auditor's question about January 20th takes 3 weeks of forensic engineering work.

### 7.4 WORM Storage Configuration

Write-Once-Read-Many storage ensures the audit log cannot be modified after writing:

```python
# AWS S3 Object Lock Configuration (WORM)
{
    "ObjectLockConfiguration": {
        "ObjectLockEnabled": "Enabled",
        "Rule": {
            "DefaultRetention": {
                "Mode": "COMPLIANCE",      # COMPLIANCE mode: immutable even to root
                "Years": 7                 # GDPR requires 6yr minimum; we use 7yr
            }
        }
    }
}

# All writes go through the ledger writer — never direct S3 access
class WORMLedgerWriter:
    async def write(self, event: LedgerEvent) -> str:
        # Compute chain hash
        prev_hash = await self.get_last_event_hash(event.org_id)
        event.prev_event_hash = prev_hash
        event.event_hash = sha256(event.to_bytes())
        
        # Write to WORM storage — cannot be modified after this point
        key = f"ledger/{event.org_id}/{event.timestamp.isoformat()}/{event.event_id}"
        await s3.put_object(
            Bucket="forgetmenot-ledger-worm",
            Key=key,
            Body=event.to_json(),
            ContentType="application/json"
        )
        
        # Mirror to append-only PostgreSQL for fast query
        await db.execute(INSERT_LEDGER_EVENT, event.to_dict())
        
        return event.event_hash
```

***

## SECTION 8 — THE CERTIFICATION ENGINE

### 8.1 What It Produces

The Certification Engine auto-generates two artifacts on the 30th day after a GDPR erasure request is registered:

1. **A signed PDF** — designed to be printed and handed to a regulator
2. **A machine-readable JSON** — designed to be ingested by GRC tools (OneTrust, ServiceNow)

### 8.2 The Certification Report Structure

```
╔═══════════════════════════════════════════════════════════════╗
║          FORGETMENOT™ COMPLIANCE CERTIFICATION REPORT         ║
║                    FNOT-EU-2.0 Standard                       ║
╠═══════════════════════════════════════════════════════════════╣
║  Issued to: Acme Corporation                                  ║
║  Report ID: CERT-2026-04-06-00441                             ║
║  Issued: 2026-05-06T09:14:22Z                                 ║
║  Jurisdiction: EU (GDPR Art. 17) + IN (DPDPA Sec. 12)         ║
╠═══════════════════════════════════════════════════════════════╣
║  SECTION 1 — ERASURE REQUEST RECORD                           ║
║  GDPR Request ID: GDPR-2026-00441                             ║
║  Request Received: 2026-04-06T09:00:00Z                       ║
║  Legal Deadline: 2026-05-06T09:00:00Z                         ║
║  Response Issued: 2026-05-05T14:22:00Z ✅ (24h early)         ║
║                                                               ║
║  SECTION 2 — COMPLIANCE RECORD (30-day period)                ║
║  Inference events scanned: 2,847,293                          ║
║  Forget-registry matches detected: 0                          ║
║  RAG retrieval filters applied: 14                            ║
║  Outputs blocked/redacted: 0                                  ║
║  Confirmed PII disclosure events: 0                           ║
║                                                               ║
║  SECTION 3 — DETECTION METHODOLOGY                            ║
║  Engines: NER (quantized Presidio), Semantic (CEDM v2.1),     ║
║           Contextual Classifier (CEDM graph model)            ║
║  Estimated false negative rate: 0.7% (industry avg: 2.8%)    ║
║  Probe set executed: 10 probes, 10/10 passed                  ║
║  CEDM disambiguation confidence: 0.94                         ║
║                                                               ║
║  SECTION 4 — DATA SURFACE COVERAGE                            ║
║  ✅ LLM Output Firewall (3 inference endpoints)               ║
║  ✅ RAG Retrieval Filter (Pinecone index: customer_emb_v3)    ║
║  ⚠️  Fine-tune dataset: manual review completed by customer   ║
║  N/A PostgreSQL users table: not connected to LLM system      ║
║                                                               ║
║  SECTION 5 — LEDGER INTEGRITY                                 ║
║  Total ledger events in period: 2,847,311                     ║
║  Chain integrity check: ✅ VERIFIED                           ║
║  Ledger root hash: a7f3e9c2...                                ║
║  Signed by ForgetMeNot CA: ✅                                 ║
║  Tamper evidence: NONE DETECTED                               ║
║                                                               ║
║  SECTION 6 — LIMITATIONS (MANDATORY DISCLOSURE)               ║
║  This report certifies that ForgetMeNot's multi-engine        ║
║  detection system found zero matches for the registered        ║
║  entity during the certification period. Detection is         ║
║  probabilistic with an estimated false negative rate of       ║
║  <1%. This report constitutes evidence of reasonable          ║
║  technical diligence under GDPR Art. 17 and DPDPA Sec. 12.   ║
║  It does not constitute legal advice. Consult your DPA for    ║
║  jurisdiction-specific compliance determinations.             ║
╚═══════════════════════════════════════════════════════════════╝
```

### 8.3 The Data Subject Verification Portal

After certification, the enterprise can optionally issue a verification token to the data subject:

```
URL: https://verify.forgetmenot.io/GDPR-2026-00441-TOKEN-7f3e9c

Your erasure request has been processed and verified.
Registered: April 6, 2026
Verified through: May 6, 2026

Systems covered:
  ✅ Customer Service AI — 0 matches in 847,293 outputs
  ✅ Sales Assistant AI  — 0 matches in 1,102,847 outputs
  ✅ Internal HR AI      — removed from connected datasources

This certificate is cryptographically signed.
Verification hash: a7f3e9c2... [Verify independently ↗]
```

***

## SECTION 9 — THE DPO DASHBOARD

### 9.1 Core Views

**View 1: Compliance Overview (Default Landing)**
- Live inference volume ticker
- Compliance posture score (0–100) vs industry benchmark
- Active GDPR requests with countdown timers (30-day clock, visible at a glance)
- Last 24h events: scans, blocks, filters, disputes

**View 2: Request Timeline**
Every GDPR erasure request has a full lifecycle view:

```
GDPR-2026-00441 Timeline
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Apr 6  ●─── Request received & registered
Apr 6  ●─── Forget Registry updated
Apr 6  ●─── Developer Playbook generated & assigned
Apr 6  ●─── RAG filter activated (Pinecone)
Apr 7  ●─── Developer confirmed fine-tune dataset reviewed
Apr 6─May 5  ●─── 30-day monitoring period (2.8M inferences scanned)
May 5  ●─── Certification Report generated
May 5  ●─── Response sent to data subject (1 day early ✅)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Status: COMPLETED ✅ | 0 disclosure events
```

**View 3: Compliance Time Machine**
A calendar interface. Click any date in history. The entire dashboard reconstructs to show the compliance state on that exact date. One-click PDF export for regulator requests.

**View 4: Proactive PII Discovery Feed**
Weekly digest of unusual patterns detected in inference logs. Entities appearing frequently with no forget registration. PII format detections (email, NHS numbers, credit cards). Each item has a one-click action: "Register in Forget Registry" or "Mark as Reviewed."

**View 5: Compliance Simulator**
Input a proposed new AI feature's configuration. Get a projected compliance risk profile, estimated erasure request volume, and recommended ForgetMeNot tier — before the feature ships.

### 9.2 The Board-Level Report

Auto-generated quarterly, designed for non-technical executives:

```
Q1 2026 AI COMPLIANCE SUMMARY — Acme Corporation

AI COMPLIANCE POSTURE: ✅ COMPLIANT (Top 15% of industry)

This quarter:
  • 8.4M AI outputs scanned — 0 confirmed PII disclosures
  • 127 erasure requests processed — 100% within legal deadline
  • Estimated regulatory fine avoidance: €340K–€2.1M

vs. Industry:
  • Peers averaged 3.2 PII disclosure events/quarter
  • Our false negative rate: 0.7% vs industry avg 2.8%

Next quarter recommendations:
  • 2 AI systems approaching Growth tier volume limits
  • EU AI Act Art. 9 review recommended before Q3 product launch
```

***

## SECTION 10 — THE DEVELOPER ERASURE PLAYBOOK

When a GDPR request is registered, ForgetMeNot scans the customer's connected infrastructure and auto-generates a developer-friendly ticket. This turns a vague compliance obligation into an executable checklist with commands:

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
GDPR ERASURE PLAYBOOK — GDPR-2026-00441
Entity: [ENTITY_001] | Deadline: May 6, 2026 (29d 22h)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

AUTOMATED (already done by ForgetMeNot):
  ✅ Inference firewall updated — all 3 endpoints covered
  ✅ RAG filter activated — Pinecone index: customer_embeddings_v3
  ✅ Compliance Ledger monitoring started

REQUIRES YOUR ACTION:

1. Fine-tune dataset review [MEDIUM PRIORITY]
   Path: s3://acme-ml/datasets/sales_calls_2025.jsonl
   Matches found: 7 rows
   Command:
     $ forgetmenot scan-dataset --path s3://acme-ml/datasets/sales_calls_2025.jsonl \
         --entity GDPR-2026-00441 --export matches.csv
   Then: review matches.csv and delete/mask flagged rows
   Mark complete: [forgetmenot complete --task dataset-review --request GDPR-2026-00441]

2. Embedding store review [LOW PRIORITY]
   Store: ChromaDB (local dev) — 3 vectors matched
   Command:
     $ forgetmenot purge-vectors --store chromadb --entity GDPR-2026-00441
   Mark complete: [auto-detected on purge]

3. PostgreSQL verification [LOW PRIORITY — manual]
   Table: users, customer_interactions
   ForgetMeNot cannot access directly — please verify manually
   Mark complete: [forgetmenot complete --task db-review --request GDPR-2026-00441]

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Progress: 2/5 tasks complete | 3 remaining
DPO notified when all tasks complete.
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

***

## SECTION 11 — MULTI-JURISDICTION COMPLIANCE

### 11.1 GDPR (EU) — Article 17

- Right to erasure applies to any company processing EU resident data, globally
- 30-day response window, hard deadline
- Fines: up to 4% of global annual turnover [kiteworks](https://www.kiteworks.com/gdpr-compliance/gdpr-fines-data-privacy-enforcement-2026/)
- ForgetMeNot covers: output filtering, ledger evidence, certification report, data subject verification

### 11.2 DPDPA (India) — Section 12

- India's Digital Personal Data Protection Act, in force 2025 [lawjournals](https://www.lawjournals.org/assets/archives/2025/vol11issue6/11131.pdf)
- Right to erasure of personal data from AI systems
- Applies to all companies processing Indian resident data
- ForgetMeNot covers: same stack as GDPR, jurisdiction flag in certification report
- **Strategic advantage:** no competitor has DPDPA compliance tooling. First-mover market.

### 11.3 EU AI Act — Articles 9 & 15

- High-risk AI systems require: risk management system, automatic event logging, accuracy and robustness documentation [n2ws](https://n2ws.com/blog/eu-ai-act)
- ForgetMeNot's Compliance Ledger satisfies Art. 9 event logging requirement
- ForgetMeNot's ensemble false negative rate documentation satisfies Art. 15 accuracy requirement
- Note: Full Annex III enforcement may be delayed to 2027 under Digital Omnibus amendment  — GDPR/DPDPA urgency narrative does not depend on this deadline [gamingtechlaw](https://www.gamingtechlaw.com/2026/03/eu-ai-act-postponement-impact-high-risk-ai/)

### 11.4 HIPAA (USA — Healthcare)

- Business Associate Agreement (BAA) required from any vendor handling PHI
- ForgetMeNot provides a standard BAA for Growth and Enterprise tiers
- Compliance Ledger satisfies HIPAA audit log requirements (6-year retention)

***

## SECTION 12 — API DESIGN

### 12.1 Complete Endpoint List

```
━━━━ FORGET REGISTRY ━━━━
POST   /v1/registry/entities              Register new entity for erasure
GET    /v1/registry/entities              List registered entities (IDs only)
GET    /v1/registry/entities/{id}         Get entity compliance status
PATCH  /v1/registry/entities/{id}         Update entity aliases/context
DELETE /v1/registry/entities/{id}         Revoke registration (with audit log)
POST   /v1/registry/entities/{id}/probe   Run manual verification probe

━━━━ COMPLIANCE FIREWALL ━━━━
POST   /v1/scan/output                    Scan a single LLM output
POST   /v1/scan/batch                     Scan multiple outputs
POST   /v1/preflight/analyze              Pre-deployment compliance check

━━━━ RAG COMPLIANCE ━━━━
POST   /v1/rag/filter                     Filter retrieval results
GET    /v1/rag/filters/stats              RAG filter event statistics

━━━━ LEDGER ━━━━
GET    /v1/ledger/events                  Query ledger events (paginated)
GET    /v1/ledger/snapshot                Point-in-time compliance snapshot
GET    /v1/ledger/integrity               Verify chain hash integrity
POST   /v1/ledger/export                  Export as signed PDF or JSON

━━━━ CERTIFICATIONS ━━━━
GET    /v1/certifications                 List all certifications
GET    /v1/certifications/{request_id}    Get certification for GDPR request
POST   /v1/certifications/{id}/reissue    Reissue with extended period

━━━━ DISPUTES ━━━━
POST   /v1/disputes                       File a false positive dispute
GET    /v1/disputes/{id}                  Get dispute status
PATCH  /v1/disputes/{id}/resolve          DPO resolves dispute

━━━━ ANALYTICS ━━━━
GET    /v1/analytics/overview             Dashboard overview data
GET    /v1/analytics/benchmarks           Industry benchmark comparison
GET    /v1/analytics/simulator            Run compliance simulation
GET    /v1/analytics/reports/board        Generate board-level quarterly report
GET    /v1/analytics/pii-discovery        Proactive PII discovery feed

━━━━ ADMIN ━━━━
GET    /v1/org/settings                   Organization settings
POST   /v1/org/integrations               Connect external systems
GET    /v1/org/usage                      Usage and billing data
POST   /v1/data-portability/export        Export full compliance history (CCDF)
```

### 12.2 Authentication

```python
# All requests require: API key (developer) or JWT (dashboard)
# API keys are org-scoped and permission-scoped

headers = {
    "Authorization": "Bearer fnot_sk_live_...",
    "X-Org-ID": "org_acme_corp",
    "X-Request-ID": "req_unique_trace_id",  # For distributed tracing
    "Content-Type": "application/json"
}

# Permission scopes:
# forgetmenot:registry:read       — read entity registrations
# forgetmenot:registry:write      — register/update entities
# forgetmenot:scan:execute        — run compliance scans
# forgetmenot:ledger:read         — query ledger events
# forgetmenot:certifications:read — access certification reports
# forgetmenot:admin               — full access
```

***

## SECTION 13 — SDK DESIGN

### 13.1 Python SDK (Primary)

```python
from forgetmenot import ForgetMeNotClient
from forgetmenot.integrations.langchain import ForgetMeNotMemory
from forgetmenot.integrations.rag import ComplianceAwareRetriever

# Initialize client
client = ForgetMeNotClient(
    api_key="fnot_sk_live_...",
    org_id="org_acme_corp",
    compliance_budget=ComplianceBudget(
        monthly_scans=10_000_000,
        max_block_rate=0.5,            # Alert if >0.5% blocked
        false_positive_tolerance=0.2,  # Auto-adjust sensitivity if FP > 0.2%
        alert_webhook="https://slack.acme.com/compliance-alerts"
    )
)

# ── 1. Register an erasure request ──────────────────────────────
entity = await client.registry.register(
    gdpr_request_id="GDPR-2026-00441",
    canonical_name="John Smith",
    aliases=["J. Smith", "john.smith@acme.com"],
    role="Head of R&D",
    organization="Acme Corporation",
    related_entities=["Project Helios", "R&D Division"]
)
print(f"Entity registered: {entity.id}")
print(f"Compliance deadline: {entity.deadline}")
print(f"Developer playbook: {entity.playbook_url}")

# ── 2. Wrap any LLM with compliance firewall ──────────────────────
import openai
openai_client = openai.OpenAI(api_key="sk-...")

compliant_llm = client.wrap(
    llm_client=openai_client,
    scan_mode="BUFFER_RELEASE",   # Legally safe — no race condition
    on_block="REDACT",            # or "BLOCK" or "FLAG"
    log_all_events=True
)

# Use exactly like normal OpenAI — compliance is transparent
response = compliant_llm.chat.completions.create(
    model="gpt-4o",
    messages=[{"role": "user", "content": "Tell me about our R&D team"}]
)

# ── 3. Check if a response was redacted ──────────────────────────
if response.forgetmenot.was_redacted:
    print(f"Redacted: {response.forgetmenot.redaction_reason}")
    dispute = await response.forgetmenot.dispute(
        reason="False positive — context is about a different person"
    )

# ── 4. RAG compliance ─────────────────────────────────────────────
from pinecone import Pinecone
pc = Pinecone(api_key="...")
index = pc.Index("customer_embeddings_v3")

compliant_retriever = ComplianceAwareRetriever(
    base_retriever=index,
    client=client,
    on_match=OnMatchPolicy(action="FILTER", log_to_ledger=True)
)

docs = await compliant_retriever.retrieve(query="R&D project updates", top_k=5)

# ── 5. LangChain native integration ──────────────────────────────
from langchain.chains import LLMChain
chain = LLMChain(
    llm=your_langchain_llm,
    memory=ForgetMeNotMemory(
        client=client,
        session_id="session_abc123"
    )
)

# ── 6. Get compliance status and export report ────────────────────
status = await client.registry.get_status("GDPR-2026-00441")
print(f"Days until deadline: {status.days_remaining}")
print(f"Total inferences scanned: {status.total_inferences_scanned}")
print(f"Disclosure events: {status.disclosure_events}")  # Target: 0

cert = await client.certifications.get("GDPR-2026-00441")
await cert.download_pdf("./compliance_cert.pdf")
```

### 13.2 Pre-Flight Check in CI/CD

```yaml
# .github/workflows/compliance-check.yml
name: Compliance Pre-Flight

on: [pull_request]

jobs:
  compliance-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: ForgetMeNot Pre-Flight
        uses: forgetmenot/preflight-action@v1
        with:
          api_key: ${{ secrets.FORGETMENOT_API_KEY }}
          config_path: ".forgetmenot/preflight.yaml"
          fail_on_severity: HIGH   # Fails CI if HIGH risk found
```

```yaml
# .forgetmenot/preflight.yaml
scan_targets:
  - type: system_prompt
    path: ./prompts/customer_service.txt
  - type: rag_datasource
    connection: ${PINECONE_INDEX_NAME}
  - type: fine_tune_dataset
    path: s3://acme-ml/datasets/

intended_user_base:
  geography: EU
  size: 50000
  data_sensitivity: high

compliance_standards:
  - GDPR_ART_17
  - DPDPA_SEC_12
  - EU_AI_ACT_ART_9
```

***

## SECTION 14 — INFRASTRUCTURE & DEPLOYMENT

### 14.1 Infrastructure Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    PRODUCTION INFRASTRUCTURE                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  CDN (CloudFront)                                               │
│    └── API Gateway (Kong) — rate limiting, auth, routing        │
│         ├── FastAPI Application Cluster (EKS)                   │
│         │    ├── Firewall Engine Pods (GPU: T4)                  │
│         │    ├── Registry API Pods (CPU)                         │
│         │    ├── Ledger Writer Pods (CPU)                        │
│         │    └── Analytics Pods (CPU)                            │
│         └── Celery Worker Cluster (EKS)                         │
│              ├── Certification Generator Workers                 │
│              ├── Proactive Discovery Workers                     │
│              └── Report Generation Workers                       │
│                                                                  │
│  Storage:                                                        │
│    ├── S3 WORM (Compliance Ledger — eu-west-1)                  │
│    ├── RDS PostgreSQL (Ledger query index — Multi-AZ)           │
│    ├── ElastiCache Redis (Forget Registry hot cache)            │
│    └── S3 Standard (Reports, exports — eu-west-1)              │
│                                                                  │
│  GPU Compute:                                                    │
│    ├── T4 GPU nodes (Firewall Engine — always on, 2 min)        │
│    └── A10G GPU nodes (CEDM retraining — spot, scale to zero)  │
│                                                                  │
│  Monitoring:                                                     │
│    ├── OpenTelemetry → Grafana (metrics, traces)                │
│    ├── PagerDuty (critical alerts)                              │
│    └── DataDog (APM, log management — PII-scrubbed)             │
└─────────────────────────────────────────────────────────────────┘
```

### 14.2 Kubernetes Deployment

```yaml
# Firewall Engine Deployment
apiVersion: apps/v1
kind: Deployment
metadata:
  name: firewall-engine
  namespace: forgetmenot-prod
spec:
  replicas: 3
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxUnavailable: 1    # Always 2 pods available during deploy
      maxSurge: 1
  template:
    spec:
      containers:
      - name: firewall-engine
        image: forgetmenot/firewall-engine:v1.0.0
        resources:
          requests:
            nvidia.com/gpu: 1
            memory: "16Gi"
            cpu: "4"
          limits:
            nvidia.com/gpu: 1
            memory: "24Gi"
            cpu: "8"
        env:
        - name: SCAN_MODE
          value: "BUFFER_RELEASE"
        - name: ENSEMBLE_ENGINES
          value: "NER,SEMANTIC,CEDM"
        - name: MAX_BUFFER_LATENCY_MS
          value: "15"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
          periodSeconds: 5
      nodeSelector:
        nvidia.com/gpu: "present"
        topology.kubernetes.io/zone: "eu-west-1a"  # GDPR data residency
```

### 14.3 Latency Budget

```
Total added latency target: < 20ms p95

Breakdown:
  Buffer fill (50 tokens):         ~5ms
  NER scan (parallel):             ~3ms
  Semantic scan (parallel):        ~8ms
  CEDM contextual scan (parallel): ~12ms
  ─────────────────────────────────────
  Parallel scan total:             ~12ms (longest parallel path)
  Buffer release + response write:  ~3ms
  Ledger write (async):             ~0ms (non-blocking)
  ─────────────────────────────────────
  Total:                           ~15ms p50 | ~18ms p95 | ~25ms p99
```

### 14.4 Security Architecture

```
Security Layers (outermost to innermost):
  1. WAF (CloudFront + AWS Shield) — DDoS, SQLi, XSS
  2. API Gateway — JWT auth, rate limiting, IP allowlist for enterprise
  3. mTLS between all internal services — no plaintext service-to-service
  4. Kubernetes NetworkPolicy — no cross-namespace traffic
  5. BYOK KMS (customer key) — all customer data encrypted at rest
  6. PII Vault — forget set contents stored as hashes + embeddings only
  7. SafeTensors-only model intake — no pickle deserialization attack surface
  8. WORM audit log — immutable, tamper-evident, chain-hashed

Data that ForgetMeNot NEVER stores in plaintext:
  - Entity canonical names
  - Entity email addresses
  - Any content from forget sets
  - Any LLM prompts or outputs

What IS stored:
  - Entity hashes (SHA256 + salt, customer-unique)
  - Entity embeddings (mathematical vectors — cannot be reversed to names)
  - Inference event metadata (timestamps, scan results, action taken)
  - Certification reports (no raw PII — entity IDs and probe results only)
```

***

## SECTION 15 — PRODUCT TIERS & BUSINESS MODEL

### 15.1 Tier Structure

| | **Developer** | **Shield** | **Growth** | **Enterprise** |
|---|---|---|---|---|
| **Price** | Free (OSS) | $299/mo | $1,499/mo | $8,000/mo |
| **Monthly inferences** | Self-hosted | 1M | 10M | Unlimited |
| **Forget registrations** | Unlimited | 50/mo | 500/mo | Unlimited |
| **Compliance Firewall** | ✅ | ✅ | ✅ | ✅ |
| **RAG Compliance Hook** | ✅ | ✅ | ✅ | ✅ |
| **Compliance Ledger** | Local only | 30-day retention | 7-year WORM | 7-year WORM |
| **Certification Reports** | No | Basic | Full (GDPR+DPDPA) | Full + Legal Opinion |
| **Jurisdictions** | GDPR only | GDPR | GDPR + DPDPA | GDPR + DPDPA + HIPAA |
| **DPO Dashboard** | No | Basic | Full | Full + Board Reports |
| **GRC Integrations** | No | No | OneTrust, ServiceNow | All + Custom |
| **SOC 2 BAA** | No | No | ✅ | ✅ |
| **EU Data Residency** | No | No | ✅ | ✅ |
| **Pre-Flight CI Check** | ✅ | ✅ | ✅ | ✅ |
| **Proactive PII Discovery** | No | Weekly digest | Weekly digest | Real-time + Custom |
| **Compliance Simulator** | No | No | ✅ | ✅ |
| **Support** | GitHub Issues | Email | Priority email + Slack | Dedicated engineer |

### 15.2 The Compliance Intelligence Revenue Layer

As the platform accumulates anonymized, aggregated data across customers, a **Compliance Intelligence add-on** unlocks:

- Industry benchmarks: *"Your false negative rate is 0.7% vs industry 2.8%"*
- Erasure request forecasting: *"At your MAU growth rate, expect 67 requests/month by Q3"*
- Regulation tracking: *"New DPDPA enforcement guidance published — affects 2 of your registered entities"*

This layer is priced separately from inference volume — it's a **data product**, not a compute product, and commands 80%+ margin.

### 15.3 Unit Economics

```
Shield tier ($299/mo):
  GPU compute (1M inferences × T4 cost): ~$12/mo
  Storage + infrastructure allocation:    ~$8/mo
  Total COGS:                            ~$20/mo
  Gross margin:                          ~93%

Growth tier ($1,499/mo):
  GPU compute (10M inferences):          ~$85/mo
  Storage (7-year WORM):                 ~$15/mo
  GRC integration maintenance:           ~$20/mo
  Total COGS:                            ~$120/mo
  Gross margin:                          ~92%

Enterprise tier ($8,000/mo):
  All compute + storage:                 ~$200/mo
  Legal opinion letter (amortized):      ~$300/mo
  Dedicated compliance engineer (share): ~$800/mo
  Total COGS:                           ~$1,300/mo
  Gross margin:                          ~84%
```

***

## SECTION 16 — THE OPEN SOURCE STRATEGY

### 16.1 What Is Free and Open Source

**`forgetmenot-core`** — MIT License, published on GitHub, never paywalled:

```
forgetmenot-core/
├── firewall/          # Buffer-release compliance firewall
├── scanner/           # NER + semantic ensemble (no CEDM — that's proprietary)
├── registry/          # Local forget registry (SQLite backend for dev)
├── ledger/            # Local ledger writer (file-based for dev)
├── rag/               # RAG compliance hook (all vector store integrations)
├── preflight/         # Pre-flight check CLI
├── integrations/
│   ├── langchain/     # LangChain native memory + callback handler
│   ├── llamaindex/    # LlamaIndex query pipeline hook
│   ├── openai/        # OpenAI client wrapper
│   ├── anthropic/     # Claude client wrapper
│   └── ollama/        # Local model wrapper
└── cli/               # Developer CLI (playbook, scan, purge commands)
```

### 16.2 What Is Proprietary (Cloud-Only)

- **CEDM (Contextual Entity Disambiguation Model)** — the relationship graph architecture and trained weights
- **Compliance Intelligence benchmarks** — aggregated industry data
- **WORM Ledger infrastructure** — the hosted, auditable, legally defensible version
- **Certification Report generation** — the signed PDF + JSON output
- **False positive calibration dataset** — trained from enterprise dispute resolutions

### 16.3 The CCDF Open Standard

**Compliance Chain Data Format (CCDF)** — an open, portable format for compliance history export. Published as an open spec. Any compliance tool can adopt it.

```json
{
  "ccdf_version": "1.0",
  "org_id": "org_acme_corp_hashed",
  "export_date": "2026-04-06T00:00:00Z",
  "ledger_events": [...],
  "certifications": [...],
  "registry_history": [...],
  "chain_root_hash": "a7f3e9...",
  "forgetmenot_signature": "...",
  "verification_tool": "https://github.com/forgetmenot/ccdf-verify"
}
```

Publishing CCDF signals enterprise-trustworthiness. A company that builds open portability into its architecture is not a hostage-taker. That trust converts reluctant enterprise procurement teams.

***

## SECTION 17 — THE RESEARCH ROADMAP (FUTURE MOAT)

### Year 1 — Publish the Framework Paper
**"Contextual Entity Disambiguation in LLM Output Compliance: Benchmarks, Failure Modes, and Evaluation Framework"**

Publish the *problem definition* and the *benchmark dataset*. Describe the architecture class (relationship-aware entity graphs). Do not publish implementation weights or training data.

Outcome: Academic authority. Researchers cite your benchmark. Developer community trusts your scanner methodology.

### Year 2 — The Proactive Detection Paper
**"Unsupervised PII Drift Detection in LLM Inference Logs"**

The proactive PII discovery engine's algorithm — detecting entities that should be in the forget registry but aren't — is novel. Publish this methodology. It positions ForgetMeNot as an active compliance advisor, not just a passive scanner.

### Year 3 — The FHE Research Collaboration
**Private LLM Inference with Homomorphic Encryption for Compliance-Verified Deployments**

ICML 2025 demonstrated GPU-accelerated FHE at 200× CPU baseline. By 2028, FHE-based inference will be production-viable for select enterprise workloads. ForgetMeNot's compliance ledger becomes a cryptographic proof — *"the model provably cannot have processed this entity's data"* — not a statistical estimate. [icml](https://icml.cc/virtual/2025/poster/45395)

Begin this collaboration now with University of Malaya research group. The 3-year head start on FHE implementation is a moat no competitor can replicate by product announcement.

***

## SECTION 18 — DEVELOPMENT ROADMAP

### Phase 0 — Foundation (Weeks 1–4)
- [ ] Register EU legal entity (Ireland or Netherlands for GDPR positioning)
- [ ] Publish `forgetmenot-core` v0.1 on GitHub — firewall, OSS scanner, RAG hook
- [ ] Build Developer Erasure Playbook generator (core workflow automation)
- [ ] Launch free EU AI Act Compliance Gap Scanner (lead generation tool)
- [ ] Draft GDPR Art. 28 DPA template with EU data protection attorney

### Phase 1 — MVP (Weeks 5–10)
- [ ] FastAPI application: registry, scan, ledger, certification endpoints
- [ ] Buffer-release firewall engine (solves race condition)
- [ ] WORM ledger writer + chain hash infrastructure
- [ ] Basic DPO dashboard (overview, request timeline, 30-day countdown)
- [ ] Python SDK v0.1 + OpenAI integration
- [ ] Basic PDF certification report generator
- [ ] Close first 3 Starter customers ($299/mo — India-first, DPDPA + GDPR targets)

### Phase 2 — Production (Weeks 11–18)
- [ ] CEDM v1 — contextual entity disambiguation (proprietary, not OSS)
- [ ] Compliance Time Machine (point-in-time dashboard reconstruction)
- [ ] Proactive PII Discovery weekly digest
- [ ] Inline dispute interface (false positive reporting + DPO resolution workflow)
- [ ] SOC 2 Type I audit complete (using Vanta for automated evidence collection)
- [ ] LangChain + LlamaIndex native integrations
- [ ] Data Subject Verification Portal (`verify.forgetmenot.io`)

### Phase 3 — Enterprise (Weeks 19–26)
- [ ] DPDPA jurisdiction support fully documented + certified
- [ ] GRC integrations: OneTrust, ServiceNow, Jira (GDPR ticket auto-creation)
- [ ] Board-Level quarterly report auto-generation
- [ ] Compliance Simulator (risk forecasting)
- [ ] Compliance Intelligence add-on (industry benchmarks, forecasting)
- [ ] CCDF open standard published + self-serve export
- [ ] EU data residency (eu-west-1 default for all data)
- [ ] Enterprise tier launched: legal opinion co-authorship, dedicated engineer
- [ ] CEDM research paper submitted to ACL/EMNLP workshop

### Phase 4 — Scale (Months 7–18)
- [ ] SOC 2 Type II complete (Month 12 minimum from company formation)
- [ ] HIPAA BAA tier launched (US healthcare vertical)
- [ ] Compliance Intelligence as standalone data product
- [ ] FHE research collaboration begins (University of Malaya + IIT partnership)
- [ ] $500K ARR milestone → Seed round ($2.5M at $10M valuation)
- [ ] First enterprise logo closed (reference customer from Phase 1 mid-market)

***

## SECTION 19 — COMPLETE TECH STACK

| Layer | Technology | Version | Why |
|---|---|---|---|
| **API Framework** | FastAPI | 0.115+ | Async-native, Pydantic v2, OpenAPI auto-docs, fastest Python framework |
| **Data Validation** | Pydantic v2 | 2.x | Type-safe request/response models, zero runtime surprises |
| **Job Queue** | Celery + Redis | 5.x / 7.x | Async job orchestration for cert generation, discovery engine |
| **NER Engine** | Presidio (quantized) | Custom fork | Distilled, 4-bit quantized, runs on T4 GPU at <3ms per output |
| **Semantic Engine** | sentence-transformers | 3.x | `all-mpnet-base-v2` for semantic similarity scoring |
| **CEDM Engine** | Custom (PyTorch + NetworkX) | Proprietary | Relationship graph + contextual classifier — the core IP |
| **ML Framework** | PyTorch | 2.x | CEDM training, quantized NER serving |
| **LLM Integrations** | LangChain, LlamaIndex | Latest | Native memory + callback hooks — deepest integration in category |
| **Vector Store Hooks** | Custom adapters per store | v1.0 | Pinecone, ChromaDB, Weaviate, pgvector, Qdrant, FAISS |
| **Model Format** | SafeTensors only | — | Eliminates pickle deserialization attack surface entirely |
| **Database (Primary)** | PostgreSQL 16 | 16.x | Ledger event index, fast time-range queries, ACID compliant |
| **Database (Cache)** | Redis | 7.x | Forget Registry hot cache, sub-millisecond entity lookups |
| **Audit Log (WORM)** | AWS S3 Object Lock | — | COMPLIANCE mode, 7-year retention, tamper-evident |
| **Model Versioning** | DVC | 3.x | Full lineage tracking for CEDM model versions |
| **Container Runtime** | Docker + Kubernetes | 1.30+ | GPU workload isolation, multi-tenant namespace separation |
| **GPU Autoscaling** | KEDA | 2.x | Scale firewall engine to zero on idle, burst on demand |
| **Service Mesh** | Istio | 1.21+ | mTLS between all pods, zero-trust internal networking |
| **API Gateway** | Kong | 3.x | Rate limiting, JWT auth, request logging, plugin ecosystem |
| **CDN** | AWS CloudFront | — | Global edge, DDoS mitigation, latency reduction |
| **Encryption (KMS)** | AWS KMS (BYOK) | — | Customer-managed keys, GDPR data residency alignment |
| **Secret Management** | HashiCorp Vault | 1.16+ | Dynamic secrets, secret rotation, audit logging |
| **CI/CD** | GitHub Actions + ArgoCD | — | GitOps deployment, environment parity, rollback capability |
| **Observability** | OpenTelemetry + Grafana | — | Distributed traces, metrics, structured logs (PII-scrubbed) |
| **APM** | Datadog | — | Application performance, anomaly detection on scan latency |
| **Alerting** | PagerDuty | — | Critical compliance events, SLA breach notifications |
| **Compliance Automation** | Vanta | — | SOC 2 evidence collection, automated control monitoring |
| **PDF Generation** | WeasyPrint + Jinja2 | — | Signed certification reports, board-level quarterly PDFs |
| **Report Signing** | OpenSSL + ForgetMeNot CA | — | Ed25519 signatures on every certification artifact |
| **SDK (Python)** | Custom + httpx | — | Async-native, auto-retry, full type hints, mypy clean |
| **SDK (TypeScript)** | Custom + fetch | — | Isomorphic, Node + browser, full type safety |
| **SDK (Go)** | Custom + net/http | v1.1 (planned) | Enterprise backend teams, high-performance deployments |
| **Testing** | pytest + pytest-asyncio | — | Full async test suite, coverage > 90% enforced in CI |
| **Load Testing** | Locust | — | Latency budget validation, firewall engine stress tests |
| **Infrastructure-as-Code** | Terraform | 1.8+ | All AWS resources version-controlled, reproducible environments |
| **Package Management** | uv (Python) | — | 10–100× faster than pip, deterministic lockfiles |
| **Code Quality** | Ruff + mypy + pre-commit | — | Zero `# type: ignore` policy, linting enforced at commit |

***

## SECTION 20 — REPOSITORY STRUCTURE

```
forgetmenot/
│
├── packages/
│   ├── forgetmenot-core/           ← OSS (MIT License — public GitHub)
│   │   ├── src/
│   │   │   ├── firewall/
│   │   │   │   ├── buffer_release.py       # Core buffer-release architecture
│   │   │   │   ├── ensemble.py             # 3-engine detection ensemble
│   │   │   │   ├── policies.py             # BLOCK / REDACT / FLAG policies
│   │   │   │   └── redactor.py             # Output redaction strategies
│   │   │   ├── scanner/
│   │   │   │   ├── ner_engine.py           # Quantized Presidio NER
│   │   │   │   ├── semantic_engine.py      # sentence-transformers similarity
│   │   │   │   └── ensemble_voter.py       # Majority vote decision logic
│   │   │   ├── registry/
│   │   │   │   ├── local_registry.py       # SQLite-backed local registry (dev)
│   │   │   │   ├── entity_schema.py        # Entity data model + validation
│   │   │   │   └── probe_generator.py      # Auto-probe generation (10 types)
│   │   │   ├── ledger/
│   │   │   │   ├── local_ledger.py         # File-based ledger (dev/OSS)
│   │   │   │   ├── event_schema.py         # LedgerEvent dataclass + hashing
│   │   │   │   └── chain_verifier.py       # Chain hash integrity checker
│   │   │   ├── rag/
│   │   │   │   ├── base_retriever.py       # Abstract ComplianceAwareRetriever
│   │   │   │   ├── pinecone.py             # Pinecone adapter
│   │   │   │   ├── chromadb.py             # ChromaDB adapter
│   │   │   │   ├── weaviate.py             # Weaviate adapter
│   │   │   │   ├── pgvector.py             # pgvector adapter
│   │   │   │   ├── qdrant.py               # Qdrant adapter
│   │   │   │   └── faiss.py                # FAISS local adapter
│   │   │   ├── integrations/
│   │   │   │   ├── langchain/
│   │   │   │   │   ├── memory.py           # ForgetMeNotMemory class
│   │   │   │   │   └── callbacks.py        # Compliance callback handler
│   │   │   │   ├── llamaindex/
│   │   │   │   │   └── pipeline_hook.py    # Query pipeline compliance node
│   │   │   │   ├── openai_wrapper.py       # OpenAI client compliance wrap
│   │   │   │   ├── anthropic_wrapper.py    # Anthropic client compliance wrap
│   │   │   │   └── ollama_wrapper.py       # Local model compliance wrap
│   │   │   ├── preflight/
│   │   │   │   ├── analyzer.py             # Pre-deployment risk analyzer
│   │   │   │   ├── rules/
│   │   │   │   │   ├── system_prompt.py    # Prompt-level risk rules
│   │   │   │   │   ├── rag_datasource.py   # Datasource risk rules
│   │   │   │   │   └── gdpr_rules.py       # GDPR + DPDPA specific rules
│   │   │   │   └── report_generator.py     # Pre-flight risk report
│   │   │   └── cli/
│   │   │       ├── main.py                 # CLI entry point
│   │   │       ├── scan.py                 # $ forgetmenot scan-dataset
│   │   │       ├── purge.py                # $ forgetmenot purge-vectors
│   │   │       └── complete.py             # $ forgetmenot complete --task
│   │   ├── tests/
│   │   ├── pyproject.toml
│   │   └── README.md
│   │
│   └── forgetmenot-cloud/          ← Proprietary (private — cloud service)
│       ├── src/
│       │   ├── api/
│       │   │   ├── main.py                 # FastAPI app entrypoint
│       │   │   ├── routers/
│       │   │   │   ├── registry.py         # /v1/registry endpoints
│       │   │   │   ├── scan.py             # /v1/scan endpoints
│       │   │   │   ├── ledger.py           # /v1/ledger endpoints
│       │   │   │   ├── certifications.py   # /v1/certifications endpoints
│       │   │   │   ├── disputes.py         # /v1/disputes endpoints
│       │   │   │   ├── analytics.py        # /v1/analytics endpoints
│       │   │   │   └── admin.py            # /v1/org endpoints
│       │   │   ├── middleware/
│       │   │   │   ├── auth.py             # JWT + API key validation
│       │   │   │   ├── pii_scrubber.py     # PII removal from all logs
│       │   │   │   ├── rate_limiter.py     # Per-org rate limiting
│       │   │   │   └── request_id.py       # Distributed trace injection
│       │   │   └── dependencies.py         # FastAPI DI container
│       │   ├── cedm/                       # CORE PROPRIETARY IP
│       │   │   ├── graph_builder.py        # Knowledge graph construction
│       │   │   ├── embedding_engine.py     # 3-type embedding generation
│       │   │   ├── classifier.py           # Contextual entity classifier
│       │   │   ├── disambiguator.py        # CEDM inference engine
│       │   │   └── model_registry.py       # CEDM model versioning (DVC)
│       │   ├── worm_ledger/
│       │   │   ├── s3_writer.py            # S3 Object Lock WORM writer
│       │   │   ├── pg_index.py             # PostgreSQL fast-query mirror
│       │   │   ├── chain_hasher.py         # Chain hash computation
│       │   │   └── time_machine.py         # Point-in-time reconstruction
│       │   ├── certification/
│       │   │   ├── report_engine.py        # PDF + JSON report generation
│       │   │   ├── signer.py               # Ed25519 report signing
│       │   │   ├── templates/              # Jinja2 report templates
│       │   │   └── verification_portal.py  # verify.forgetmenot.io backend
│       │   ├── discovery/
│       │   │   ├── pii_drift_detector.py   # Proactive PII discovery engine
│       │   │   ├── digest_generator.py     # Weekly DPO digest builder
│       │   │   └── scheduler.py            # Celery beat schedule
│       │   ├── playbook/
│       │   │   ├── infrastructure_scanner.py  # Scan connected systems
│       │   │   ├── ticket_generator.py        # Auto-generate dev tickets
│       │   │   └── integrations/
│       │   │       ├── jira.py             # Jira ticket creation
│       │   │       ├── linear.py           # Linear ticket creation
│       │   │       └── github_issues.py    # GitHub Issues creation
│       │   ├── intelligence/
│       │   │   ├── benchmark_engine.py     # Industry benchmark computation
│       │   │   ├── forecasting.py          # Erasure request forecasting
│       │   │   └── simulator.py            # Compliance simulator
│       │   ├── dashboard/
│       │   │   ├── board_report.py         # Board-level quarterly report
│       │   │   └── dpo_views.py            # DPO dashboard data layer
│       │   ├── portability/
│       │   │   └── ccdf_exporter.py        # CCDF open format export
│       │   └── workers/
│       │       ├── cert_worker.py          # Certification generation worker
│       │       ├── discovery_worker.py     # Proactive discovery worker
│       │       └── report_worker.py        # Report generation worker
│       └── tests/
│
├── infrastructure/
│   ├── terraform/
│   │   ├── modules/
│   │   │   ├── eks/                # Kubernetes cluster
│   │   │   ├── rds/                # PostgreSQL Multi-AZ
│   │   │   ├── s3_worm/            # WORM audit log bucket
│   │   │   ├── elasticache/        # Redis cluster
│   │   │   ├── kms/                # KMS key configuration
│   │   │   └── cloudfront/         # CDN + WAF
│   │   ├── environments/
│   │   │   ├── dev/
│   │   │   ├── staging/
│   │   │   └── prod/
│   │   └── main.tf
│   ├── kubernetes/
│   │   ├── namespaces/
│   │   ├── deployments/
│   │   ├── services/
│   │   ├── network-policies/       # Zero-trust inter-pod rules
│   │   └── keda-scalers/           # GPU autoscaling configs
│   └── helm/
│       └── forgetmenot/            # Helm chart for on-premise deployment
│
├── research/
│   ├── cedm/                       # CEDM research experiments (private)
│   ├── benchmarks/                 # Public benchmark dataset
│   └── papers/                     # Draft research papers
│
└── docs/
    ├── api/                        # OpenAPI spec
    ├── sdk/                        # SDK documentation
    ├── compliance/                 # Legal + regulatory documentation
    └── architecture/               # ADRs (Architecture Decision Records)
```

***

## SECTION 21 — ENVIRONMENT SETUP (FOR CONTRIBUTORS)

This section is written so that a developer with basic Python knowledge can run ForgetMeNot locally in under 20 minutes.

```bash
# Prerequisites: Python 3.11+, Docker Desktop, uv package manager

# Step 1: Clone and set up
git clone https://github.com/forgetmenot/forgetmenot-core
cd forgetmenot-core

# Step 2: Install uv (fast Python package manager)
curl -LsSf https://astral.sh/uv/install.sh | sh

# Step 3: Create virtual environment and install dependencies
uv venv --python 3.11
source .venv/bin/activate     # Windows: .venv\Scripts\activate
uv pip install -e ".[dev]"

# Step 4: Start local infrastructure (Redis + PostgreSQL via Docker)
docker compose up -d

# Step 5: Run migrations
forgetmenot db migrate

# Step 6: Set up local config
cp .env.example .env
# Edit .env — only two required fields for local dev:
#   FORGETMENOT_ENV=local
#   OPENAI_API_KEY=sk-...  (if testing with OpenAI)

# Step 7: Run tests to confirm everything works
pytest tests/ -v --tb=short

# Step 8: Try it yourself — 5-minute quickstart
python examples/quickstart.py
```

**`examples/quickstart.py` — the fastest path to understanding the product:**

```python
"""
ForgetMeNot Quickstart — runs entirely locally, no API key needed.
Demonstrates: register entity → wrap LLM → see firewall in action.
"""
import asyncio
from forgetmenot import ForgetMeNotClient
from forgetmenot.testing import MockLLMClient  # No real LLM needed for demo

async def main():
    # Initialize local client (uses SQLite + file ledger — no cloud needed)
    client = ForgetMeNotClient.local()

    # Step 1: Register someone who wants their data erased
    entity = await client.registry.register(
        gdpr_request_id="DEMO-001",
        canonical_name="Alice Johnson",
        aliases=["alice@example.com", "A. Johnson"],
        role="CFO",
        organization="DemoCompany"
    )
    print(f"✅ Entity registered: {entity.id}")
    print(f"   Deadline: {entity.deadline}")
    print(f"   Auto-probes generated: {entity.probe_count}")

    # Step 2: Wrap any LLM with compliance firewall
    # (MockLLMClient simulates an LLM for this demo)
    llm = client.wrap(MockLLMClient())

    # Step 3: Safe query — should pass through untouched
    safe_response = await llm.chat("What is the capital of France?")
    print(f"\n🟢 Safe query result: {safe_response.content}")
    print(f"   Firewall status: {safe_response.forgetmenot.status}")  # PASS

    # Step 4: Risky query — should be blocked/redacted
    # MockLLMClient returns "Alice Johnson is the CFO of DemoCompany"
    risky_response = await llm.chat("Tell me about the DemoCompany executive team")
    print(f"\n🔴 Risky query result: {risky_response.content}")  # [REDACTED]
    print(f"   Firewall status: {risky_response.forgetmenot.status}")  # REDACTED
    print(f"   Matched entity: {risky_response.forgetmenot.matched_entity_id}")
    print(f"   Engines that fired: {risky_response.forgetmenot.engines_fired}")

    # Step 5: Check the compliance ledger
    events = await client.ledger.recent(limit=5)
    print(f"\n📋 Last {len(events)} ledger events:")
    for event in events:
        icon = "✅" if event.action == "PASS" else "🚫"
        print(f"   {icon} {event.timestamp} | {event.action} | engine: {event.top_engine}")

    # Step 6: Export a compliance summary
    summary = await client.certifications.preview("DEMO-001")
    print(f"\n📄 Compliance Preview:")
    print(f"   Inferences scanned: {summary.total_scanned}")
    print(f"   Disclosure events:  {summary.disclosure_events}")  # 0
    print(f"   Probes passed:      {summary.probes_passed}/10")

asyncio.run(main())
```

Expected output:
```
✅ Entity registered: ent_a1b2c3d4
   Deadline: 2026-05-06T09:00:00Z
   Auto-probes generated: 10

🟢 Safe query result: The capital of France is Paris.
   Firewall status: PASS

🔴 Risky query result: [Content removed per privacy policy]
   Firewall status: REDACTED
   Matched entity: ent_a1b2c3d4
   Engines that fired: ['NER', 'CEDM']

📋 Last 2 ledger events:
   ✅ 2026-04-06T13:30:01Z | PASS    | engine: NER
   🚫 2026-04-06T13:30:04Z | REDACT  | engine: CEDM

📄 Compliance Preview:
   Inferences scanned: 2
   Disclosure events:  0
   Probes passed:      10/10
```

***

## SECTION 22 — TESTING STRATEGY

### 22.1 Test Pyramid

```
           ┌─────────────────┐
           │   E2E Tests     │  ← Full compliance lifecycle: register → scan → certify
           │   (Playwright)  │     Slow, run nightly. 15 scenarios.
           ├─────────────────┤
           │ Integration Tests│  ← API endpoints, DB writes, S3 WORM, ledger chain
           │   (pytest)      │     Medium speed. Run on every PR. ~200 tests.
           ├─────────────────┤
           │   Unit Tests    │  ← Every function, every edge case, every engine
           │   (pytest)      │     Fast. Run on every commit. ~800 tests.
           └─────────────────┘
```

### 22.2 Critical Test Cases

```python
# These tests must pass before ANY deployment to production

class TestFirewallCritical:
    
    async def test_no_pii_disclosure_under_any_condition(self):
        """
        The most important test in the codebase.
        Even if the scanner is configured incorrectly, output
        containing forget-registered PII must NEVER reach the user.
        """
        client = ForgetMeNotClient.local()
        entity = await client.registry.register(
            gdpr_request_id="TEST-CRITICAL-001",
            canonical_name="Test Person",
            aliases=["test@critical.com"]
        )
        
        # Simulate scanner returning FALSE NEGATIVE (scanner failure)
        with patch("forgetmenot.scanner.ensemble.scan", return_value=ScanResult.PASS):
            llm = client.wrap(MockLLMClient(output="Test Person's email is test@critical.com"))
            response = await llm.chat("test query")
            
            # Even with scanner failure simulated, secondary checks must catch this
            # This is the defense-in-depth test
            assert "test@critical.com" not in response.content
            assert "Test Person" not in response.content
    
    async def test_buffer_release_no_race_condition(self):
        """
        Proves zero tokens are streamed before scanning completes.
        """
        tokens_received = []
        scan_completed_at = None
        
        async def capture_stream(response_stream):
            nonlocal scan_completed_at
            async for token in response_stream:
                tokens_received.append((token, time.time()))
        
        with patch.object(EnsembleScanner, "scan_async", 
                          side_effect=lambda *args: asyncio.sleep(0.015)):  # 15ms scan
            llm = client.wrap(MockStreamingLLMClient())
            task = asyncio.create_task(capture_stream(llm.stream("test")))
            await asyncio.sleep(0.010)  # 10ms in — scan not complete
            
            # No tokens should have been released yet — scan still running
            assert len(tokens_received) == 0, "CRITICAL: tokens released before scan complete"
            
            await task  # Let full stream complete
            assert len(tokens_received) > 0  # Tokens released AFTER scan
    
    async def test_chain_hash_integrity_after_1000_events(self):
        """
        Proves the WORM ledger's tamper evidence works end-to-end.
        """
        ledger = WORMLedger(client)
        
        for i in range(1000):
            await ledger.write(mock_inference_event(i))
        
        integrity = await ledger.verify_chain_integrity()
        assert integrity.is_valid == True
        assert integrity.broken_at_index is None
        
        # Tamper with event 500 and verify chain breaks detectably
        await ledger._tamper_for_testing(event_index=500)
        integrity_after_tamper = await ledger.verify_chain_integrity()
        assert integrity_after_tamper.is_valid == False
        assert integrity_after_tamper.broken_at_index == 500
    
    async def test_cedm_disambiguates_two_john_smiths(self):
        """
        Proves CEDM solves the John Smith collision problem.
        Two people with the same name must be distinguishable.
        """
        # Register John Smith #1 — CFO at Acme Corp
        entity_1 = await client.registry.register(
            gdpr_request_id="TEST-JOHN-1",
            canonical_name="John Smith",
            role="CFO",
            organization="Acme Corp"
        )
        
        # John Smith #2 — Engineer at Different Corp — NOT registered
        # An output mentioning John Smith #2 should NOT be blocked
        
        output_about_john_2 = "John Smith, lead engineer at Different Corp, published a paper."
        scan_result = await cedm.scan(output_about_john_2, registry=[entity_1])
        
        assert scan_result.action == ScanAction.PASS
        assert scan_result.matched_entities == []
        
        # But an output mentioning John Smith #1 SHOULD be blocked
        output_about_john_1 = "The CFO of Acme Corp, John Smith, submitted the financial report."
        scan_result_2 = await cedm.scan(output_about_john_1, registry=[entity_1])
        
        assert scan_result_2.action == ScanAction.BLOCK
        assert entity_1.id in scan_result_2.matched_entities
```

### 22.3 Performance Test Targets

```python
# These benchmarks are enforced in CI — failing them blocks deployment

PERFORMANCE_REQUIREMENTS = {
    "firewall_p50_latency_ms": 15,    # 50th percentile
    "firewall_p95_latency_ms": 20,    # 95th percentile
    "firewall_p99_latency_ms": 30,    # 99th percentile
    "registry_lookup_ms": 1,          # Redis hot cache lookup
    "ledger_write_ms": 5,             # Async write, non-blocking
    "cert_generation_seconds": 30,    # Full PDF generation
    "rag_filter_overhead_ms": 3,      # Added latency per retrieval call
}
```

***

## SECTION 23 — OBSERVABILITY

### 23.1 What Is Monitored

Every component emits structured logs, metrics, and traces via OpenTelemetry. Zero plaintext PII ever enters any observability system — all logs are scrubbed before writing.

```python
# Every scan event emits this structured trace — no PII, full signal
{
    "trace_id": "abc123",
    "span_name": "compliance_scan",
    "org_id": "org_acme_hashed",
    "duration_ms": 14.2,
    "engines_run": ["NER", "SEMANTIC", "CEDM"],
    "engine_latencies_ms": {"NER": 2.8, "SEMANTIC": 7.1, "CEDM": 11.4},
    "decision": "PASS",
    "registry_size": 47,
    "cache_hit": true,
    "entity_id_matched": null     # null on PASS, entity hash on BLOCK
    # NEVER: entity name, email, any PII
}
```

### 23.2 Critical Alerts

```yaml
# PagerDuty alerts — wakes someone up immediately

- alert: FirewallDownstreamFailure
  condition: firewall_pass_rate < 99.9%  # Even 0.1% failure is critical
  severity: P1
  message: "Compliance firewall degraded — possible PII disclosure risk"

- alert: LedgerChainBreak
  condition: ledger_chain_integrity == false
  severity: P1
  message: "WORM audit log integrity compromised — legal emergency"

- alert: CertificationMissedDeadline
  condition: cert_issued_after_deadline == true
  severity: P1
  message: "GDPR 30-day deadline missed — regulatory emergency"

- alert: ScanLatencyDegraded
  condition: firewall_p95_latency_ms > 50
  severity: P2
  message: "Scan latency exceeds budget — customer experience impact"
```

### 23.3 The Compliance Health Score

Every organization gets a real-time Compliance Health Score (0–100) visible in the dashboard. It is computed as:

```python
def compute_health_score(org: Organization) -> int:
    score = 100
    
    # Deductions:
    if org.active_requests_overdue > 0:
        score -= 30 * org.active_requests_overdue   # -30 per overdue request
    
    if org.false_negative_rate_7d > 0.02:           # >2% miss rate
        score -= 20
    
    if org.disclosure_events_30d > 0:
        score -= 40 * org.disclosure_events_30d     # -40 per actual disclosure
    
    if not org.rag_hook_connected:
        score -= 10                                  # Missing a protection layer
    
    if org.unreviewed_pii_discovery_alerts > 5:
        score -= 5
    
    # Bonuses:
    if org.false_negative_rate_7d < 0.005:          # <0.5% — excellent
        score += 5
    
    return max(0, min(100, score))
```

***

## SECTION 24 — ON-PREMISE DEPLOYMENT (ENTERPRISE)

For air-gapped customers — defense contractors, government agencies, healthcare systems that cannot send any data to cloud — ForgetMeNot ships as a fully self-contained Helm chart:

```bash
# Enterprise on-premise deployment — runs entirely within customer's cluster

helm repo add forgetmenot https://charts.forgetmenot.io
helm repo update

helm install forgetmenot forgetmenot/forgetmenot \
  --namespace forgetmenot \
  --create-namespace \
  --set global.mode="airgapped" \
  --set ledger.storage="local"   \          # Use local PVC instead of S3
  --set cedm.model_path="/models/cedm-v2.bin" \  # Pre-loaded model
  --set licensing.key="${ENTERPRISE_LICENSE_KEY}" \
  --set tls.enabled=true \
  --set tls.cert_path="/certs/tls.crt" \
  --set tls.key_path="/certs/tls.key"
```

For this tier, the CEDM model is distributed as a signed, encrypted model artifact — not as a cloud API call. The license key activates the model. The model weights never leave ForgetMeNot's distribution system unencrypted. The customer cannot extract and redistribute the model.

***

## SECTION 25 — THE PRODUCT IN ONE PAGE

*For when you need to explain this to someone in 60 seconds.*

**What ForgetMeNot does:**
Sit between your AI and your users. Watch every word your AI outputs. If someone has legally asked to be forgotten — guarantee their data never appears again. Keep a cryptographic record proving it.

**Who it's for:**
Any company running AI systems that processes personal data from EU residents (GDPR) or Indian residents (DPDPA). Which, in 2026, is almost every company with users.

**Why they need it:**
Because the law gives people the right to demand their data be erased from AI systems — and companies have 30 days to comply. Currently, there is no production tool to do this without rebuilding the entire AI from scratch.

**Why ForgetMeNot wins:**
- Works on any LLM, any cloud, any provider — including on-premise
- The only compliance tool that covers both GDPR and India's DPDPA
- Turns a 3-week forensic engineering project into an 11-second dashboard export
- Gets smarter with every customer — calibration data compounds over time
- Built with a 3-year FHE research roadmap that no competitor can replicate

**What it isn't:**
It does not modify model weights. It does not require you to retrain your model. It does not require you to move to any particular cloud. It works with what you already have.

***

## SECTION 26 — FINAL NOTES FROM THE CTO

There are three decisions buried in this document that will determine whether ForgetMeNot succeeds or fails. I want to name them explicitly:

**Decision 1: Build the Compliance Ledger with the same rigor as the ML engine.**
Every engineering team instinctively prioritizes the "interesting" components — the CEDM, the scanner, the RAG hook. The Compliance Ledger feels like plumbing. It isn't. The Compliance Ledger IS the product. It's what enterprises buy. It's what regulators accept. It's what lawyers hand to courts. If the ledger has bugs, you don't have a compliance product — you have a dashboard with a broken foundation. Assign your strongest engineer to it. Review it like it's going into a spacecraft.

**Decision 2: Never store a single byte of plaintext PII.**
Not in logs. Not in error messages. Not in Slack notifications. Not in support tickets. Not in database query results cached in memory. The moment a customer's employee name appears in plaintext on your infrastructure, you have inherited their GDPR liability. Build PII scrubbing into the logging middleware from day one, not as a retrofit. Every `logger.info()` call should pass through a scrubber before writing. This is a culture decision as much as an engineering decision.

**Decision 3: Launch in India first.**
Every version of this product has been described as an "EU product." You are in Hyderabad. You have cultural access, network access, and market timing that no European competitor has. DPDPA is brand new. There is zero AI compliance tooling for it. The first 10 customers you close in India will fund the SOC 2 audit that unlocks your first European enterprise customer. Build for DPDPA + GDPR simultaneously from line one of the codebase. Don't add DPDPA later — it will be too expensive to retrofit.

The product in this document is not perfect. No product document is. But it is complete, honest about its risks, clear about its priorities, and grounded in the actual technical and regulatory landscape as it exists in April 2026.

The rest is execution.

***

*Document version 3.0 — ForgetMeNot CTO Office — April 6, 2026*
*Classification: Internal — Pre-Seed Build Document*
*Next review: On completion of Phase 1 MVP (Target: Week 10)*
