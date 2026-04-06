# ForgetMeNot — Complete TDD Test Suite

*Every test the system needs. Written in **Arrange → Act → Assert** (AAA) pattern with BDD-style naming. Organized by component, ordered by dependency — unit tests first, integration second, E2E last. Build the tests before you build the code.*

***

## TEST FILE STRUCTURE

```
tests/
├── unit/
│   ├── test_registry.py
│   ├── test_cedm.py
│   ├── test_scanner_ner.py
│   ├── test_scanner_semantic.py
│   ├── test_scanner_ensemble.py
│   ├── test_firewall.py
│   ├── test_buffer_release.py
│   ├── test_rag_hook.py
│   ├── test_ledger.py
│   ├── test_chain_hasher.py
│   ├── test_certification.py
│   ├── test_playbook.py
│   ├── test_preflight.py
│   ├── test_disputes.py
│   ├── test_proactive_discovery.py
│   └── test_pii_scrubber.py
├── integration/
│   ├── test_api_registry.py
│   ├── test_api_scan.py
│   ├── test_api_ledger.py
│   ├── test_api_certifications.py
│   ├── test_api_disputes.py
│   ├── test_api_analytics.py
│   ├── test_sdk_python.py
│   ├── test_integrations_langchain.py
│   ├── test_integrations_rag.py
│   └── test_worm_ledger.py
├── performance/
│   ├── test_firewall_latency.py
│   ├── test_scanner_throughput.py
│   └── test_ledger_write_speed.py
├── security/
│   ├── test_pii_never_logged.py
│   ├── test_auth_and_permissions.py
│   ├── test_safetensors_only.py
│   └── test_tenant_isolation.py
└── e2e/
    ├── test_full_gdpr_lifecycle.py
    ├── test_full_dpdpa_lifecycle.py
    ├── test_certification_issued_on_day_30.py
    └── test_data_subject_portal.py
```

***

## SHARED FIXTURES — `conftest.py`

```python
# tests/conftest.py
import pytest
import asyncio
from datetime import datetime, timedelta, timezone
from unittest.mock import AsyncMock, MagicMock, patch
from forgetmenot import ForgetMeNotClient
from forgetmenot.registry.entity_schema import (
    EntityRegistrationRequest, EntityType
)
from forgetmenot.ledger.event_schema import LedgerEvent, LedgerEventType
from forgetmenot.scanner.ensemble_voter import ScanResult, ScanAction
from forgetmenot.testing import (
    MockLLMClient,
    MockStreamingLLMClient,
    MockVectorStoreRetriever
)


# ─── Event Loop ────────────────────────────────────────────────────────────────
@pytest.fixture(scope="session")
def event_loop():
    """Single event loop for the entire test session."""
    loop = asyncio.new_event_loop()
    yield loop
    loop.close()


# ─── Client ───────────────────────────────────────────────────────────────────
@pytest.fixture
def local_client():
    """Fully local ForgetMeNot client — SQLite + file ledger. No cloud."""
    return ForgetMeNotClient.local(test_mode=True)


@pytest.fixture
async def client_with_entity(local_client):
    """Client pre-loaded with one registered entity for use across tests."""
    entity = await local_client.registry.register(
        gdpr_request_id="FIXTURE-GDPR-001",
        canonical_name="Alice Johnson",
        aliases=["alice@acme.com", "A. Johnson", "AJohnson"],
        role="Chief Financial Officer",
        organization="Acme Corporation",
        department="Finance",
        employment_period={"start": "2020-01-01", "end": "2025-12-31"},
        related_entities=["Acme Corporation", "Finance Division", "Project Helios"]
    )
    return local_client, entity


# ─── Entity Registration Payloads ─────────────────────────────────────────────
@pytest.fixture
def minimal_entity_payload():
    return EntityRegistrationRequest(
        gdpr_request_id="TEST-MIN-001",
        canonical_name="Bob Smith",
        aliases=["bob@example.com"]
    )


@pytest.fixture
def full_entity_payload():
    return EntityRegistrationRequest(
        gdpr_request_id="TEST-FULL-001",
        canonical_name="Carol White",
        aliases=["carol@corp.com", "C. White", "carol.white"],
        role="Head of Engineering",
        organization="TechCorp",
        department="Engineering",
        employment_period={"start": "2018-06-01", "end": "2024-09-30"},
        related_entities=["TechCorp", "Engineering Division", "Project Alpha"],
        data_sources=["finetune_batch_2023_09", "hr_records_2024"]
    )


@pytest.fixture
def two_same_name_entities():
    """Two different people sharing the same canonical name — the John Smith test."""
    return (
        EntityRegistrationRequest(
            gdpr_request_id="TEST-JOHN-1",
            canonical_name="John Smith",
            role="CFO",
            organization="Acme Corp",
            aliases=["jsmith@acme.com"]
        ),
        EntityRegistrationRequest(
            gdpr_request_id="TEST-JOHN-2",
            canonical_name="John Smith",
            role="Lead Engineer",
            organization="Different Corp",
            aliases=["jsmith@different.com"]
        )
    )


# ─── LLM Mock Outputs ─────────────────────────────────────────────────────────
@pytest.fixture
def safe_llm_output():
    return "The capital of France is Paris. The Eiffel Tower was built in 1889."


@pytest.fixture
def risky_direct_name_output():
    return "Alice Johnson submitted the quarterly report for Acme Corporation."


@pytest.fixture
def risky_indirect_role_output():
    return "The CFO of Acme Corporation approved the budget last Thursday."


@pytest.fixture
def risky_email_output():
    return "Please forward this to alice@acme.com for final sign-off."


@pytest.fixture
def risky_cooccurrence_output():
    return "The executive who led Project Helios left the Finance Division in December."


@pytest.fixture
def ambiguous_same_name_output():
    """References John Smith #2 (not registered) — should NOT be blocked."""
    return "John Smith, lead engineer at Different Corp, published a paper on distributed systems."


@pytest.fixture
def registered_same_name_output():
    """References John Smith #1 (registered CFO at Acme) — SHOULD be blocked."""
    return "The CFO of Acme Corp, John Smith, signed off on the acquisition."
```

***

## UNIT TESTS — `tests/unit/test_registry.py`

```python
"""
COMPONENT: Forget Registry
PURPOSE:   Stores entities to be forgotten. Validates inputs, hashes PII,
           generates probes, tracks deadlines.
"""
import pytest
from datetime import datetime, timedelta, timezone
from forgetmenot.registry.entity_schema import EntityRegistrationRequest
from forgetmenot.registry.local_registry import LocalForgetRegistry
from forgetmenot.registry.probe_generator import ProbeGenerator
from forgetmenot.exceptions import (
    DuplicateRequestError,
    InvalidEntityError,
    EntityNotFoundError
)


class TestEntityRegistration:

    @pytest.mark.asyncio
    async def test_register_minimal_entity_succeeds(self, minimal_entity_payload):
        """
        WHAT:   Registering an entity with only required fields succeeds.
        GIVEN:  A payload with gdpr_request_id + canonical_name + aliases only.
        WHEN:   register() is called.
        THEN:   Returns entity with valid ID, 30-day deadline, 10 probes.
        """
        # Arrange
        registry = LocalForgetRegistry()

        # Act
        entity = await registry.register(minimal_entity_payload)

        # Assert
        assert entity.id is not None
        assert entity.id.startswith("ent_")
        assert entity.deadline > datetime.now(timezone.utc)
        assert entity.deadline <= datetime.now(timezone.utc) + timedelta(days=31)
        assert entity.probe_count == 10
        assert entity.status == "ACTIVE"

    @pytest.mark.asyncio
    async def test_register_full_entity_succeeds(self, full_entity_payload):
        """
        WHAT:   Registering an entity with all optional fields succeeds.
        GIVEN:  A fully populated EntityRegistrationRequest.
        WHEN:   register() is called.
        THEN:   All fields stored, CEDM graph constructed, probes generated.
        """
        registry = LocalForgetRegistry()
        entity = await registry.register(full_entity_payload)

        assert entity.canonical_name_hash is not None
        assert entity.canonical_name_hash != "Carol White"   # PII must be hashed
        assert len(entity.alias_hashes) == 3
        assert entity.cedm_graph_built is True
        assert entity.embedding_count == 3   # name + role + relational

    @pytest.mark.asyncio
    async def test_canonical_name_never_stored_in_plaintext(self, full_entity_payload):
        """
        WHAT:   PII (name, email) is NEVER stored in plaintext on the registry.
        GIVEN:  A fully populated entity registration.
        WHEN:   register() is called and entity is retrieved from storage.
        THEN:   canonical_name field is absent or hashed. Raw string not findable.
        CRITICAL: This test must NEVER be skipped or marked xfail.
        """
        registry = LocalForgetRegistry()
        entity = await registry.register(full_entity_payload)
        raw = await registry._get_raw_storage_record(entity.id)

        assert "Carol White" not in str(raw)
        assert "carol@corp.com" not in str(raw)
        assert "carol.white" not in str(raw)

    @pytest.mark.asyncio
    async def test_duplicate_gdpr_request_id_raises_error(self, minimal_entity_payload):
        """
        WHAT:   Registering the same GDPR request ID twice raises an error.
        GIVEN:  An entity already registered with request_id="TEST-MIN-001".
        WHEN:   register() is called again with the same request_id.
        THEN:   DuplicateRequestError is raised.
        """
        registry = LocalForgetRegistry()
        await registry.register(minimal_entity_payload)

        with pytest.raises(DuplicateRequestError) as exc:
            await registry.register(minimal_entity_payload)
        assert "TEST-MIN-001" in str(exc.value)

    @pytest.mark.asyncio
    async def test_empty_canonical_name_raises_error(self):
        """
        WHAT:   Empty canonical name is rejected at validation.
        GIVEN:  A payload with canonical_name="".
        WHEN:   register() is called.
        THEN:   InvalidEntityError raised with field name in message.
        """
        registry = LocalForgetRegistry()

        with pytest.raises(InvalidEntityError) as exc:
            await registry.register(EntityRegistrationRequest(
                gdpr_request_id="TEST-EMPTY",
                canonical_name="",
                aliases=["test@example.com"]
            ))
        assert "canonical_name" in str(exc.value).lower()

    @pytest.mark.asyncio
    async def test_get_entity_status_returns_correct_fields(self, client_with_entity):
        """
        WHAT:   get_status() returns compliance tracking fields.
        GIVEN:  A registered entity with some inference events logged.
        WHEN:   get_status() is called with the entity's GDPR request ID.
        THEN:   Returns days_remaining, total_inferences_scanned, disclosure_events.
        """
        client, entity = client_with_entity
        status = await client.registry.get_status(entity.gdpr_request_id)

        assert status.days_remaining >= 0
        assert status.days_remaining <= 30
        assert status.total_inferences_scanned >= 0
        assert status.disclosure_events == 0
        assert status.probes_passed == 10

    @pytest.mark.asyncio
    async def test_get_nonexistent_entity_raises_error(self, local_client):
        """
        WHAT:   Getting a non-existent entity raises EntityNotFoundError.
        GIVEN:  An empty registry.
        WHEN:   get_status("DOES-NOT-EXIST") is called.
        THEN:   EntityNotFoundError is raised.
        """
        with pytest.raises(EntityNotFoundError):
            await local_client.registry.get_status("DOES-NOT-EXIST")

    @pytest.mark.asyncio
    async def test_revoke_entity_marks_inactive(self, client_with_entity):
        """
        WHAT:   Revoking a registration marks it INACTIVE and logs audit event.
        GIVEN:  An active registered entity.
        WHEN:   revoke() is called.
        THEN:   Entity status = INACTIVE. Audit event written. Scanning stops.
        """
        client, entity = client_with_entity
        await client.registry.revoke(entity.id, reason="Data subject withdrew request")
        status = await client.registry.get_status(entity.gdpr_request_id)

        assert status.entity_status == "INACTIVE"
        assert status.revocation_reason == "Data subject withdrew request"

    @pytest.mark.asyncio
    async def test_list_entities_returns_ids_only_no_pii(self, client_with_entity):
        """
        WHAT:   Listing entities returns entity IDs only — never names or emails.
        GIVEN:  A registry with one registered entity.
        WHEN:   list() is called.
        THEN:   Returns list of entity IDs. No canonical names, aliases, emails.
        """
        client, entity = client_with_entity
        entities = await client.registry.list()

        assert len(entities) >= 1
        for e in entities:
            assert hasattr(e, "id")
            assert not hasattr(e, "canonical_name")
            assert not hasattr(e, "aliases")
            assert "Alice" not in str(e)
            assert "alice@acme.com" not in str(e)

    @pytest.mark.asyncio
    async def test_deadline_is_exactly_30_days_from_registration(
        self, minimal_entity_payload
    ):
        """
        WHAT:   The compliance deadline is exactly 30 days from registration time.
        GIVEN:  A new entity registration.
        WHEN:   register() is called.
        THEN:   deadline == registration_time + 30 days (±1 second tolerance).
        """
        registry = LocalForgetRegistry()
        before = datetime.now(timezone.utc)
        entity = await registry.register(minimal_entity_payload)
        after = datetime.now(timezone.utc)

        expected_min = before + timedelta(days=30)
        expected_max = after + timedelta(days=30)

        assert expected_min <= entity.deadline <= expected_max

    @pytest.mark.asyncio
    async def test_update_aliases_adds_without_removing_existing(
        self, client_with_entity
    ):
        """
        WHAT:   Updating aliases appends new aliases, never removes existing ones.
        GIVEN:  An entity with aliases ["alice@acme.com", "A. Johnson"].
        WHEN:   update() is called with new_aliases=["a.johnson@acme.com"].
        THEN:   Entity now has 4 aliases total. Original aliases still present.
        """
        client, entity = client_with_entity
        await client.registry.update(
            entity.id,
            new_aliases=["a.johnson@acme.com"]
        )
        status = await client.registry.get_status(entity.gdpr_request_id)
        assert status.alias_count == 4
```

***

## UNIT TESTS — `tests/unit/test_cedm.py`

```python
"""
COMPONENT: CEDM — Contextual Entity Disambiguation Model
PURPOSE:   Distinguish between two people with the same name using
           organizational context, role, and relationship graphs.
           This is the core proprietary IP. Test it ruthlessly.
"""
import pytest
from forgetmenot.cedm.disambiguator import CEDMDisambiguator
from forgetmenot.cedm.graph_builder import EntityGraphBuilder
from forgetmenot.cedm.embedding_engine import CEDMEmbeddingEngine


class TestCEDMDisambiguation:

    @pytest.mark.asyncio
    async def test_direct_name_mention_is_flagged(self, client_with_entity):
        """
        WHAT:   A direct canonical name mention is caught by CEDM.
        INPUT:  "Alice Johnson submitted the quarterly financial report."
        EXPECT: BLOCK | confidence > 0.95 | matched_entity = entity.id
        """
        client, entity = client_with_entity
        cedm = CEDMDisambiguator(registry=client.registry)

        result = await cedm.scan(
            "Alice Johnson submitted the quarterly financial report.",
            entity_ids=[entity.id]
        )

        assert result.action == "BLOCK"
        assert result.confidence > 0.95
        assert entity.id in result.matched_entity_ids

    @pytest.mark.asyncio
    async def test_alias_email_mention_is_flagged(self, client_with_entity):
        """
        WHAT:   An email alias mention is caught.
        INPUT:  "Please CC alice@acme.com on the report."
        EXPECT: BLOCK
        """
        client, entity = client_with_entity
        cedm = CEDMDisambiguator(registry=client.registry)

        result = await cedm.scan(
            "Please CC alice@acme.com on the report.",
            entity_ids=[entity.id]
        )
        assert result.action == "BLOCK"

    @pytest.mark.asyncio
    async def test_role_only_mention_is_flagged(self, client_with_entity):
        """
        WHAT:   Indirect role reference (no name) is caught via role embedding.
        INPUT:  "The Chief Financial Officer of Acme Corporation approved the budget."
        EXPECT: BLOCK — role + org combination matches registered entity
        """
        client, entity = client_with_entity
        cedm = CEDMDisambiguator(registry=client.registry)

        result = await cedm.scan(
            "The Chief Financial Officer of Acme Corporation approved the budget.",
            entity_ids=[entity.id]
        )
        assert result.action == "BLOCK"
        assert result.matched_via == "role_embedding"

    @pytest.mark.asyncio
    async def test_cooccurrence_relational_mention_is_flagged(
        self, client_with_entity
    ):
        """
        WHAT:   A co-occurrence mention (no name, no role, but related entities) is caught.
        INPUT:  "The executive who led Project Helios left Finance Division in December."
        EXPECT: BLOCK — co-occurring entities match CEDM relational graph
        """
        client, entity = client_with_entity
        cedm = CEDMDisambiguator(registry=client.registry)

        result = await cedm.scan(
            "The executive who led Project Helios left Finance Division in December.",
            entity_ids=[entity.id]
        )
        assert result.action == "BLOCK"
        assert result.matched_via == "relational_embedding"

    @pytest.mark.asyncio
    async def test_completely_unrelated_output_passes(
        self, client_with_entity, safe_llm_output
    ):
        """
        WHAT:   Safe output with no entity mentions passes through.
        INPUT:  "The capital of France is Paris. The Eiffel Tower was built in 1889."
        EXPECT: PASS | confidence < 0.3 | no matched entities
        """
        client, entity = client_with_entity
        cedm = CEDMDisambiguator(registry=client.registry)

        result = await cedm.scan(safe_llm_output, entity_ids=[entity.id])

        assert result.action == "PASS"
        assert result.confidence < 0.3
        assert result.matched_entity_ids == []

    @pytest.mark.asyncio
    async def test_same_name_different_person_is_not_blocked(
        self, local_client, two_same_name_entities,
        ambiguous_same_name_output
    ):
        """
        WHAT:   CEDM correctly distinguishes two people with the same name.
                John Smith #1 (CFO at Acme Corp) is registered.
                John Smith #2 (Engineer at Different Corp) is NOT registered.
                Output about John Smith #2 must NOT be blocked.
        INPUT:  "John Smith, lead engineer at Different Corp, published a paper."
        EXPECT: PASS — CEDM uses org + role context to disambiguate
        THE MOST IMPORTANT DISAMBIGUATION TEST.
        """
        payload_1, payload_2 = two_same_name_entities
        entity_1 = await local_client.registry.register(payload_1)
        # entity_2 is NOT registered — he has no erasure request

        cedm = CEDMDisambiguator(registry=local_client.registry)
        result = await cedm.scan(
            ambiguous_same_name_output,
            entity_ids=[entity_1.id]
        )

        assert result.action == "PASS", (
            "CEDM incorrectly blocked a different John Smith — disambiguation failed"
        )

    @pytest.mark.asyncio
    async def test_same_name_registered_person_is_blocked(
        self, local_client, two_same_name_entities,
        registered_same_name_output
    ):
        """
        WHAT:   The registered John Smith (CFO at Acme Corp) is correctly blocked.
        INPUT:  "The CFO of Acme Corp, John Smith, signed off on the acquisition."
        EXPECT: BLOCK — org + role confirms this is the registered entity
        """
        payload_1, _ = two_same_name_entities
        entity_1 = await local_client.registry.register(payload_1)

        cedm = CEDMDisambiguator(registry=local_client.registry)
        result = await cedm.scan(
            registered_same_name_output,
            entity_ids=[entity_1.id]
        )

        assert result.action == "BLOCK"

    @pytest.mark.asyncio
    async def test_cedm_confidence_is_in_valid_range(self, client_with_entity):
        """
        WHAT:   CEDM confidence score is always between 0.0 and 1.0.
        GIVEN:  Any text input.
        EXPECT: 0.0 <= confidence <= 1.0 for all outputs.
        """
        client, entity = client_with_entity
        cedm = CEDMDisambiguator(registry=client.registry)

        test_inputs = [
            "Hello world",
            "Alice Johnson CFO Acme",
            "Random text about nothing",
            "Project Helios Finance Division Acme Corporation",
            "",
            "a" * 10000,   # Very long input
        ]
        for text in test_inputs:
            result = await cedm.scan(text, entity_ids=[entity.id])
            assert 0.0 <= result.confidence <= 1.0, (
                f"Confidence out of range for input: {text[:50]}"
            )

    @pytest.mark.asyncio
    async def test_empty_string_input_passes_safely(self, client_with_entity):
        """
        WHAT:   Empty string input does not crash the system.
        INPUT:  ""
        EXPECT: PASS with confidence = 0.0. No exception raised.
        """
        client, entity = client_with_entity
        cedm = CEDMDisambiguator(registry=client.registry)

        result = await cedm.scan("", entity_ids=[entity.id])
        assert result.action == "PASS"
        assert result.confidence == 0.0

    @pytest.mark.asyncio
    async def test_very_long_input_does_not_crash(self, client_with_entity):
        """
        WHAT:   10,000 token input is handled gracefully — truncated not crashed.
        INPUT:  String of 50,000 characters.
        EXPECT: PASS or BLOCK (content-dependent), no exception, no timeout.
        """
        client, entity = client_with_entity
        cedm = CEDMDisambiguator(registry=client.registry)

        long_input = "safe content " * 4000
        result = await cedm.scan(long_input, entity_ids=[entity.id])
        assert result.action in ["PASS", "BLOCK"]

    @pytest.mark.asyncio
    async def test_cedm_scan_completes_within_latency_budget(
        self, client_with_entity
    ):
        """
        WHAT:   CEDM scan completes within the 12ms latency budget.
        INPUT:  A standard 200-token LLM output.
        EXPECT: scan duration < 12ms (p95 target from architecture doc).
        """
        import time
        client, entity = client_with_entity
        cedm = CEDMDisambiguator(registry=client.registry)
        test_input = "The executive team reviewed the quarterly results. " * 10

        durations = []
        for _ in range(100):
            start = time.perf_counter()
            await cedm.scan(test_input, entity_ids=[entity.id])
            durations.append((time.perf_counter() - start) * 1000)

        p95 = sorted(durations)[94]
        assert p95 < 12.0, f"CEDM p95 latency {p95:.1f}ms exceeds 12ms budget"
```

***

## UNIT TESTS — `tests/unit/test_scanner_ensemble.py`

```python
"""
COMPONENT: Scanner Ensemble
PURPOSE:   Three engines (NER, Semantic, CEDM) vote together.
           Tests majority vote logic, individual engine failure handling,
           and the combined false negative rate target of <0.8%.
"""
import pytest
from unittest.mock import AsyncMock
from forgetmenot.scanner.ensemble_voter import (
    EnsembleVoter, ScanResult, ScanAction, DetectionResult
)


class TestEnsembleVotingLogic:

    def make_result(self, confidence: float, action: ScanAction) -> DetectionResult:
        return DetectionResult(confidence=confidence, action=action)

    @pytest.mark.asyncio
    async def test_all_three_engines_pass_returns_pass(self):
        """
        WHAT:   All three engines voting PASS returns PASS.
        GIVEN:  NER=PASS(0.1), Semantic=PASS(0.2), CEDM=PASS(0.15)
        EXPECT: PASS
        """
        voter = EnsembleVoter()
        result = voter.decide(
            ner=self.make_result(0.1, ScanAction.PASS),
            semantic=self.make_result(0.2, ScanAction.PASS),
            cedm=self.make_result(0.15, ScanAction.PASS)
        )
        assert result.action == ScanAction.PASS

    @pytest.mark.asyncio
    async def test_all_three_engines_block_returns_block(self):
        """
        WHAT:   All three engines voting BLOCK returns BLOCK.
        GIVEN:  NER=BLOCK(0.98), Semantic=BLOCK(0.95), CEDM=BLOCK(0.97)
        EXPECT: BLOCK
        """
        voter = EnsembleVoter()
        result = voter.decide(
            ner=self.make_result(0.98, ScanAction.BLOCK),
            semantic=self.make_result(0.95, ScanAction.BLOCK),
            cedm=self.make_result(0.97, ScanAction.BLOCK)
        )
        assert result.action == ScanAction.BLOCK

    @pytest.mark.asyncio
    async def test_majority_two_block_returns_block(self):
        """
        WHAT:   Two engines voting BLOCK (majority) returns BLOCK.
        GIVEN:  NER=BLOCK(0.91), Semantic=PASS(0.2), CEDM=BLOCK(0.88)
        EXPECT: BLOCK — 2/3 majority triggers block
        """
        voter = EnsembleVoter()
        result = voter.decide(
            ner=self.make_result(0.91, ScanAction.BLOCK),
            semantic=self.make_result(0.2, ScanAction.PASS),
            cedm=self.make_result(0.88, ScanAction.BLOCK)
        )
        assert result.action == ScanAction.BLOCK

    @pytest.mark.asyncio
    async def test_single_high_confidence_hit_triggers_block(self):
        """
        WHAT:   One engine with confidence > 0.95 triggers BLOCK regardless of others.
        GIVEN:  NER=BLOCK(0.97), Semantic=PASS(0.1), CEDM=PASS(0.05)
        EXPECT: BLOCK — single very high confidence overrides majority
        """
        voter = EnsembleVoter()
        result = voter.decide(
            ner=self.make_result(0.97, ScanAction.BLOCK),
            semantic=self.make_result(0.1, ScanAction.PASS),
            cedm=self.make_result(0.05, ScanAction.PASS)
        )
        assert result.action == ScanAction.BLOCK
        assert result.triggered_by == "single_high_confidence"

    @pytest.mark.asyncio
    async def test_single_medium_confidence_hit_returns_flag(self):
        """
        WHAT:   One medium-confidence hit (0.70–0.94) with FLAG policy returns FLAG.
        GIVEN:  NER=BLOCK(0.72), Semantic=PASS(0.1), CEDM=PASS(0.2)
                policy.single_hit_action = "FLAG"
        EXPECT: FLAG — needs human review, not outright block
        """
        from forgetmenot.firewall.policies import BlockingPolicy
        voter = EnsembleVoter()
        policy = BlockingPolicy(single_hit_action="FLAG")

        result = voter.decide(
            ner=self.make_result(0.72, ScanAction.BLOCK),
            semantic=self.make_result(0.1, ScanAction.PASS),
            cedm=self.make_result(0.2, ScanAction.PASS),
            policy=policy
        )
        assert result.action == ScanAction.FLAG

    @pytest.mark.asyncio
    async def test_ner_engine_failure_gracefully_degrades_to_two_engine(self):
        """
        WHAT:   If NER engine fails, system continues with remaining two engines.
        GIVEN:  NER raises RuntimeError, Semantic=BLOCK(0.9), CEDM=BLOCK(0.85)
        EXPECT: BLOCK — majority still achieved with 2/2 remaining engines
                No exception propagated to caller.
        """
        voter = EnsembleVoter()
        result = voter.decide(
            ner=None,  # Simulates engine failure — returns None
            semantic=self.make_result(0.9, ScanAction.BLOCK),
            cedm=self.make_result(0.85, ScanAction.BLOCK),
            allow_partial=True
        )
        assert result.action == ScanAction.BLOCK
        assert result.engines_used == ["SEMANTIC", "CEDM"]

    @pytest.mark.asyncio
    async def test_all_three_engines_fail_returns_conservative_block(self):
        """
        WHAT:   If all three engines fail, the safe default is BLOCK (fail-secure).
        GIVEN:  All three engines return None (catastrophic failure).
        EXPECT: BLOCK — fail-secure design. Better to block than to leak PII.
        CRITICAL: System must NEVER default to PASS on engine failure.
        """
        voter = EnsembleVoter()
        result = voter.decide(
            ner=None,
            semantic=None,
            cedm=None,
            allow_partial=True
        )
        assert result.action == ScanAction.BLOCK
        assert result.reason == "fail_secure_all_engines_unavailable"

    @pytest.mark.asyncio
    async def test_ensemble_result_includes_per_engine_breakdown(self):
        """
        WHAT:   Ensemble result contains individual engine scores for audit trail.
        GIVEN:  Three engines with different results.
        EXPECT: result.engine_breakdown has NER, SEMANTIC, CEDM keys with scores.
        """
        voter = EnsembleVoter()
        result = voter.decide(
            ner=self.make_result(0.91, ScanAction.BLOCK),
            semantic=self.make_result(0.3, ScanAction.PASS),
            cedm=self.make_result(0.88, ScanAction.BLOCK)
        )
        assert "NER" in result.engine_breakdown
        assert "SEMANTIC" in result.engine_breakdown
        assert "CEDM" in result.engine_breakdown
        assert result.engine_breakdown["NER"]["confidence"] == 0.91
```

***

## UNIT TESTS — `tests/unit/test_firewall.py`

```python
"""
COMPONENT: Compliance Firewall
PURPOSE:   Core middleware. Intercepts LLM output, runs ensemble scan,
           applies BLOCK/REDACT/FLAG policy, logs to ledger.
"""
import pytest
import asyncio
import time
from unittest.mock import AsyncMock, patch
from forgetmenot.firewall.buffer_release import BufferReleaseFirewall
from forgetmenot.scanner.ensemble_voter import ScanResult, ScanAction


class TestBufferReleaseFirewall:

    @pytest.mark.asyncio
    async def test_clean_output_passes_through_unchanged(
        self, client_with_entity, safe_llm_output
    ):
        """
        WHAT:   A clean output (no PII) passes through unchanged.
        INPUT:  "The capital of France is Paris."
        EXPECT: response.content == original output. status == PASS.
        """
        client, entity = client_with_entity
        llm = client.wrap(MockLLMClient(output=safe_llm_output))

        response = await llm.chat("What is the capital of France?")

        assert response.content == safe_llm_output
        assert response.forgetmenot.status == "PASS"
        assert response.forgetmenot.was_redacted is False

    @pytest.mark.asyncio
    async def test_pii_output_is_redacted(
        self, client_with_entity, risky_direct_name_output
    ):
        """
        WHAT:   Output containing registered entity name is redacted.
        INPUT:  "Alice Johnson submitted the quarterly report for Acme Corporation."
        EXPECT: response.content does NOT contain "Alice Johnson".
                response.forgetmenot.was_redacted == True.
        """
        client, entity = client_with_entity
        llm = client.wrap(MockLLMClient(output=risky_direct_name_output))

        response = await llm.chat("Who submitted the report?")

        assert "Alice Johnson" not in response.content
        assert response.forgetmenot.was_redacted is True
        assert response.forgetmenot.matched_entity_id == entity.id

    @pytest.mark.asyncio
    async def test_no_token_released_before_scan_completes(
        self, client_with_entity
    ):
        """
        WHAT:   CRITICAL — zero tokens reach the user before scan completes.
                This prevents the GDPR Art. 5 race condition.
        GIVEN:  A streaming LLM with 15ms scan latency injected.
        WHEN:   Stream starts.
        THEN:   At t=10ms (before scan done), zero tokens received by caller.
                At t=20ms (after scan done), tokens start flowing.
        CRITICAL: This test must NEVER be skipped.
        """
        client, entity = client_with_entity
        tokens_received_at = []

        async def capture_stream(stream):
            async for token in stream:
                tokens_received_at.append(time.perf_counter())

        scan_started_at = time.perf_counter()

        # Inject 15ms scan delay
        with patch.object(
            BufferReleaseFirewall, "_run_ensemble_scan",
            new_callable=AsyncMock,
            side_effect=lambda *a, **kw: asyncio.sleep(0.015)
        ):
            llm = client.wrap(MockStreamingLLMClient(tokens=["Hello", " world", "!"]))
            task = asyncio.create_task(capture_stream(llm.stream("test")))

            # At 10ms — scan not done yet — zero tokens should have been released
            await asyncio.sleep(0.010)
            tokens_before_scan = len(tokens_received_at)

            await task

        assert tokens_before_scan == 0, (
            f"RACE CONDITION: {tokens_before_scan} tokens released before scan completed. "
            "This is a GDPR Art. 5 violation."
        )
        assert len(tokens_received_at) > 0  # Tokens DID arrive after scan

    @pytest.mark.asyncio
    async def test_redact_policy_replaces_pii_with_placeholder(
        self, client_with_entity, risky_direct_name_output
    ):
        """
        WHAT:   REDACT policy replaces PII mention with placeholder text.
        GIVEN:  policy="REDACT", output containing "Alice Johnson".
        EXPECT: "[Content removed per privacy policy]" in response.
                Original PII not present.
        """
        client, entity = client_with_entity
        from forgetmenot.firewall.policies import BlockingPolicy
        llm = client.wrap(
            MockLLMClient(output=risky_direct_name_output),
            policy=BlockingPolicy(on_match="REDACT")
        )

        response = await llm.chat("test")

        assert "[Content removed per privacy policy]" in response.content
        assert "Alice Johnson" not in response.content

    @pytest.mark.asyncio
    async def test_block_policy_returns_empty_response(
        self, client_with_entity, risky_direct_name_output
    ):
        """
        WHAT:   BLOCK policy returns empty response with error code.
        GIVEN:  policy="BLOCK", output containing registered PII.
        EXPECT: response.content == "". response.status_code == 403.
        """
        client, entity = client_with_entity
        from forgetmenot.firewall.policies import BlockingPolicy
        llm = client.wrap(
            MockLLMClient(output=risky_direct_name_output),
            policy=BlockingPolicy(on_match="BLOCK")
        )

        response = await llm.chat("test")

        assert response.content == ""
        assert response.forgetmenot.status_code == 403

    @pytest.mark.asyncio
    async def test_every_inference_event_logged_to_ledger(
        self, client_with_entity, safe_llm_output
    ):
        """
        WHAT:   Every inference — pass or block — writes an event to the ledger.
        GIVEN:  Three inference calls (2 clean, 1 blocked).
        EXPECT: Ledger has exactly 3 new events after 3 calls.
        """
        client, entity = client_with_entity
        before_count = await client.ledger.count()

        llm = client.wrap(MockLLMClient(output=safe_llm_output))
        await llm.chat("query 1")
        await llm.chat("query 2")

        blocked_llm = client.wrap(
            MockLLMClient(output="Alice Johnson sent the email.")
        )
        await blocked_llm.chat("query 3")

        after_count = await client.ledger.count()
        assert after_count - before_count == 3

    @pytest.mark.asyncio
    async def test_firewall_adds_less_than_20ms_latency_p95(
        self, client_with_entity
    ):
        """
        WHAT:   End-to-end firewall overhead is < 20ms at p95.
        GIVEN:  100 consecutive inference calls with clean outputs.
        EXPECT: p95 additional latency < 20ms.
        """
        client, entity = client_with_entity
        llm = client.wrap(MockLLMClient(output="Safe content about France."))

        latencies = []
        for _ in range(100):
            start = time.perf_counter()
            await llm.chat("test query")
            latencies.append((time.perf_counter() - start) * 1000)

        p95 = sorted(latencies)[94]
        assert p95 < 20.0, f"Firewall p95 latency {p95:.1f}ms exceeds 20ms budget"

    @pytest.mark.asyncio
    async def test_firewall_works_with_empty_registry(self, local_client):
        """
        WHAT:   Firewall works correctly when forget registry is empty.
        GIVEN:  A client with no registered entities.
        EXPECT: All outputs pass through. No errors.
        """
        llm = local_client.wrap(MockLLMClient(output="Any output should pass."))
        response = await llm.chat("test")

        assert response.content == "Any output should pass."
        assert response.forgetmenot.status == "PASS"

    @pytest.mark.asyncio
    async def test_firewall_handles_llm_timeout_gracefully(
        self, client_with_entity
    ):
        """
        WHAT:   If the LLM times out, firewall returns a clean error — never crashes.
        GIVEN:  An LLM client that raises asyncio.TimeoutError.
        EXPECT: ForgetMeNotTimeoutError raised. No partial output leaked.
                Timeout event logged to ledger.
        """
        from forgetmenot.exceptions import ForgetMeNotTimeoutError
        client, entity = client_with_entity

        timeout_llm = MockLLMClient(raises=asyncio.TimeoutError("LLM timed out"))
        llm = client.wrap(timeout_llm)

        with pytest.raises(ForgetMeNotTimeoutError):
            await llm.chat("test")

        # Timeout event must be logged
        recent_events = await client.ledger.recent(limit=1)
        assert recent_events[0].event_type == "INFERENCE_TIMEOUT"
```

***

## UNIT TESTS — `tests/unit/test_ledger.py`

```python
"""
COMPONENT: Compliance Ledger (WORM)
PURPOSE:   Append-only, chain-hashed, tamper-evident audit log.
           The legal backbone of the product. Test it like it's going to court.
"""
import pytest
import hashlib
import json
from datetime import datetime, timezone
from forgetmenot.ledger.local_ledger import LocalLedger
from forgetmenot.ledger.chain_hasher import ChainHasher
from forgetmenot.ledger.event_schema import LedgerEvent, LedgerEventType


class TestLedgerWrites:

    @pytest.mark.asyncio
    async def test_write_returns_event_hash(self, local_client):
        """
        WHAT:   Writing an event returns a non-empty event hash string.
        INPUT:  A valid LedgerEvent.
        EXPECT: Returns SHA256 hex string (64 chars).
        """
        ledger = LocalLedger()
        event = LedgerEvent(
            org_id="org_test",
            event_type=LedgerEventType.INFERENCE_CLEAN,
            action_taken="PASS",
            inference_hash="abc123"
        )
        event_hash = await ledger.write(event)

        assert event_hash is not None
        assert len(event_hash) == 64  # SHA256 hex
        assert all(c in "0123456789abcdef" for c in event_hash)

    @pytest.mark.asyncio
    async def test_first_event_has_genesis_prev_hash(self, local_client):
        """
        WHAT:   The first event in a new ledger has prev_hash = "GENESIS".
        GIVEN:  A brand new empty ledger.
        EXPECT: first_event.prev_event_hash == "GENESIS"
        """
        ledger = LocalLedger(fresh=True)
        event = LedgerEvent(
            org_id="org_test",
            event_type=LedgerEventType.ENTITY_REGISTERED,
            action_taken="REGISTERED",
            inference_hash="n/a"
        )
        await ledger.write(event)
        stored = await ledger.get_first_event("org_test")

        assert stored.prev_event_hash == "GENESIS"

    @pytest.mark.asyncio
    async def test_chain_hash_links_consecutive_events(self):
        """
        WHAT:   Each event's prev_hash equals the hash of the previous event.
        GIVEN:  Three consecutive writes.
        EXPECT: event2.prev_hash == event1.hash
                event3.prev_hash == event2.hash
        """
        ledger = LocalLedger(fresh=True)

        hashes = []
        for i in range(3):
            h = await ledger.write(LedgerEvent(
                org_id="org_chain_test",
                event_type=LedgerEventType.INFERENCE_CLEAN,
                action_taken="PASS",
                inference_hash=f"hash_{i}"
            ))
            hashes.append(h)

        events = await ledger.get_events("org_chain_test", limit=3)
        assert events[1].prev_event_hash == hashes[0]
        assert events[2].prev_event_hash == hashes[1]

    @pytest.mark.asyncio
    async def test_tampered_event_breaks_chain_integrity(self):
        """
        WHAT:   Modifying any stored event is detected by chain verification.
        GIVEN:  A ledger with 10 events, all valid.
        WHEN:   Event at index 5 is modified (simulating tampering).
        EXPECT: verify_chain_integrity() returns is_valid=False, broken_at_index=5.
        CRITICAL: This is the legal tamper-evidence guarantee.
        """
        ledger = LocalLedger(fresh=True)

        for i in range(10):
            await ledger.write(LedgerEvent(
                org_id="org_tamper_test",
                event_type=LedgerEventType.INFERENCE_CLEAN,
                action_taken="PASS",
                inference_hash=f"hash_{i}"
            ))

        # Verify all good before tamper
        pre_tamper = await ledger.verify_chain_integrity("org_tamper_test")
        assert pre_tamper.is_valid is True

        # Tamper with event at index 5
        await ledger._force_modify_event_for_testing(
            org_id="org_tamper_test",
            index=5,
            new_action="FORGED_PASS"
        )

        # Verify tamper is detected
        post_tamper = await ledger.verify_chain_integrity("org_tamper_test")
        assert post_tamper.is_valid is False
        assert post_tamper.broken_at_index == 5

    @pytest.mark.asyncio
    async def test_deleted_event_breaks_chain_integrity(self):
        """
        WHAT:   Deleting an event from the middle of the chain is detected.
        GIVEN:  Ledger with 10 events.
        WHEN:   Event at index 4 is deleted.
        EXPECT: verify_chain_integrity() returns is_valid=False.
        """
        ledger = LocalLedger(fresh=True)
        for i in range(10):
            await ledger.write(LedgerEvent(
                org_id="org_delete_test",
                event_type=LedgerEventType.INFERENCE_CLEAN,
                action_taken="PASS",
                inference_hash=f"hash_{i}"
            ))

        await ledger._force_delete_event_for_testing("org_delete_test", index=4)

        integrity = await ledger.verify_chain_integrity("org_delete_test")
        assert integrity.is_valid is False

    @pytest.mark.asyncio
    async def test_ledger_write_is_nonblocking(self):
        """
        WHAT:   Ledger writes are async and do not block the inference path.
        GIVEN:  A ledger write with intentional 50ms storage delay injected.
        EXPECT: The write coroutine returns in < 1ms (fire-and-forget).
                The actual storage happens in background.
        """
        from unittest.mock import patch
        import asyncio

        ledger = LocalLedger()

        with patch.object(
            ledger, "_write_to_storage",
            new_callable=AsyncMock,
            side_effect=lambda *a: asyncio.sleep(0.05)  # 50ms storage delay
        ):
            start = time.perf_counter()
            await ledger.write(LedgerEvent(
                org_id="org_nonblock_test",
                event_type=LedgerEventType.INFERENCE_CLEAN,
                action_taken="PASS",
                inference_hash="test"
            ))
            elapsed_ms = (time.perf_counter() - start) * 1000

        assert elapsed_ms < 1.0, (
            f"Ledger write blocked inference path for {elapsed_ms:.1f}ms"
        )

    @pytest.mark.asyncio
    async def test_point_in_time_snapshot_returns_correct_state(self):
        """
        WHAT:   The Compliance Time Machine reconstructs accurate historical state.
        GIVEN:  Events at t=0, t=1h, t=2h, t=3h.
        WHEN:   Snapshot requested for t=1.5h.
        EXPECT: Snapshot contains events at t=0 and t=1h only. NOT t=2h or t=3h.
        """
        from datetime import timedelta
        ledger = LocalLedger(fresh=True)
        base_time = datetime.now(timezone.utc)

        events_data = [
            (base_time, "PASS"),
            (base_time + timedelta(hours=1), "PASS"),
            (base_time + timedelta(hours=2), "BLOCK"),
            (base_time + timedelta(hours=3), "PASS"),
        ]

        for ts, action in events_data:
            await ledger.write(LedgerEvent(
                org_id="org_time_test",
                event_type=LedgerEventType.INFERENCE_CLEAN,
                action_taken=action,
                inference_hash="test",
                _override_timestamp=ts   # test-only override
            ))

        snapshot = await ledger.get_snapshot(
            org_id="org_time_test",
            at_time=base_time + timedelta(hours=1, minutes=30)
        )

        assert snapshot.total_events == 2
        assert snapshot.block_events == 0   # The block at t=2h not included

    @pytest.mark.asyncio
    async def test_pii_never_stored_in_ledger(self, client_with_entity):
        """
        WHAT:   Raw PII never appears in any ledger event — not in any field.
        GIVEN:  An inference that triggers a BLOCK for "Alice Johnson".
        EXPECT: The resulting ledger event contains zero plaintext PII.
        CRITICAL: Audit log must be safe to export to regulators.
        """
        client, entity = client_with_entity
        llm = client.wrap(
            MockLLMClient(output="Alice Johnson is the CFO.")
        )
        await llm.chat("test")

        events = await client.ledger.recent(limit=1)
        event_json = json.dumps(events[0].to_dict())

        assert "Alice Johnson" not in event_json
        assert "alice@acme.com" not in event_json
        assert "A. Johnson" not in event_json
```

***

## UNIT TESTS — `tests/unit/test_rag_hook.py`

```python
"""
COMPONENT: RAG Compliance Hook
PURPOSE:   Filter forget-registered entities at the retrieval layer,
           before they enter the LLM context window.
"""
import pytest
from forgetmenot.rag.base_retriever import ComplianceAwareRetriever
from forgetmenot.rag.policies import OnMatchPolicy


class TestRAGComplianceHook:

    @pytest.mark.asyncio
    async def test_clean_documents_pass_through(self, client_with_entity):
        """
        WHAT:   Documents with no forget-registered content pass through.
        INPUT:  5 documents, none mentioning Alice Johnson.
        EXPECT: All 5 documents returned.
        """
        client, entity = client_with_entity
        mock_docs = [
            {"id": f"doc_{i}", "text": f"Generic document {i} about France."}
            for i in range(5)
        ]

        retriever = ComplianceAwareRetriever(
            base_retriever=MockVectorStoreRetriever(returns=mock_docs),
            client=client,
            on_match=OnMatchPolicy(action="FILTER")
        )

        results = await retriever.retrieve("tell me about France", top_k=5)
        assert len(results) == 5

    @pytest.mark.asyncio
    async def test_pii_document_is_filtered_from_results(
        self, client_with_entity
    ):
        """
        WHAT:   A document containing registered entity's data is removed.
        INPUT:  5 docs, 1 mentioning "Alice Johnson is CFO of Acme Corporation".
        EXPECT: 4 documents returned. The PII doc is removed.
        """
        client, entity = client_with_entity
        mock_docs = [
            {"id": "doc_safe_1", "text": "Revenue report Q1 2026."},
            {"id": "doc_pii",    "text": "Alice Johnson is CFO of Acme Corporation."},
            {"id": "doc_safe_2", "text": "Product launch scheduled for May."},
            {"id": "doc_safe_3", "text": "Engineering team headcount update."},
            {"id": "doc_safe_4", "text": "Marketing budget approved."},
        ]

        retriever = ComplianceAwareRetriever(
            base_retriever=MockVectorStoreRetriever(returns=mock_docs),
            client=client,
            on_match=OnMatchPolicy(action="FILTER", log_to_ledger=True)
        )

        results = await retriever.retrieve("executive team info", top_k=5)

        assert len(results) == 4
        assert all(r["id"] != "doc_pii" for r in results)

    @pytest.mark.asyncio
    async def test_rag_filter_event_logged_to_ledger(self, client_with_entity):
        """
        WHAT:   Every RAG filter action is logged to the compliance ledger.
        GIVEN:  1 PII document filtered from retrieval.
        EXPECT: A RAG_FILTER_APPLIED event written to ledger.
        """
        client, entity = client_with_entity
        before_count = await client.ledger.count()

        retriever = ComplianceAwareRetriever(
            base_retriever=MockVectorStoreRetriever(returns=[
                {"id": "pii_doc", "text": "Alice Johnson's performance review."}
            ]),
            client=client,
            on_match=OnMatchPolicy(action="FILTER", log_to_ledger=True)
        )

        await retriever.retrieve("employee reviews", top_k=1)
        after_count = await client.ledger.count()

        assert after_count > before_count
        recent = await client.ledger.recent(limit=1)
        assert recent[0].event_type == "RAG_FILTER_APPLIED"

    @pytest.mark.asyncio
    async def test_rag_filter_adds_less_than_3ms_latency(
        self, client_with_entity
    ):
        """
        WHAT:   RAG hook adds < 3ms overhead per retrieval call.
        GIVEN:  100 retrieval calls with 5 clean docs each.
        EXPECT: p95 additional latency < 3ms.
        """
        import time
        client, entity = client_with_entity
        clean_docs = [
            {"id": f"doc_{i}", "text": f"Safe content {i}."} for i in range(5)
        ]

        retriever = ComplianceAwareRetriever(
            base_retriever=MockVectorStoreRetriever(returns=clean_docs),
            client=client,
        )

        latencies = []
        for _ in range(100):
            start = time.perf_counter()
            await retriever.retrieve("test query", top_k=5)
            latencies.append((time.perf_counter() - start) * 1000)

        p95 = sorted(latencies)[94]
        assert p95 < 3.0, f"RAG hook p95 overhead {p95:.1f}ms exceeds 3ms budget"

    @pytest.mark.asyncio
    async def test_all_docs_pii_returns_empty_list_not_error(
        self, client_with_entity
    ):
        """
        WHAT:   If ALL retrieved documents contain PII, returns empty list gracefully.
        GIVEN:  5 documents, all mentioning Alice Johnson.
        EXPECT: Returns [] — empty list. No exception raised.
                Ledger has 5 RAG_FILTER_APPLIED events.
        """
        client, entity = client_with_entity
        all_pii_docs = [
            {"id": f"doc_{i}", "text": f"Alice Johnson's record #{i}."}
            for i in range(5)
        ]

        retriever = ComplianceAwareRetriever(
            base_retriever=MockVectorStoreRetriever(returns=all_pii_docs),
            client=client,
        )

        results = await retriever.retrieve("test", top_k=5)
        assert results == []
```

***

## UNIT TESTS — `tests/unit/test_certification.py`

```python
"""
COMPONENT: Certification Engine
PURPOSE:   Auto-generates the legally defensible PDF + JSON report
           on Day 30 of a GDPR erasure request lifecycle.
"""
import pytest
from forgetmenot.certification.report_engine import CertificationReportEngine
from forgetmenot.certification.signer import ReportSigner


class TestCertificationReportGeneration:

    @pytest.mark.asyncio
    async def test_certification_generated_on_day_30(self, client_with_entity):
        """
        WHAT:   Certification is auto-triggered exactly at the 30-day deadline.
        GIVEN:  A GDPR request registered 30 days ago with all probes passing.
        EXPECT: Certification status == "ISSUED". Report URL present.
        """
        client, entity = client_with_entity
        cert = await client.certifications.generate(entity.gdpr_request_id)

        assert cert.status == "ISSUED"
        assert cert.report_pdf_url is not None
        assert cert.report_json_url is not None
        assert cert.issued_at is not None

    @pytest.mark.asyncio
    async def test_certification_contains_mandatory_fields(
        self, client_with_entity
    ):
        """
        WHAT:   Every certification report contains all legally required fields.
        GIVEN:  A processed GDPR request.
        EXPECT: Report contains: gdpr_request_id, issued_at, org_id,
                total_inferences_scanned, disclosure_events, probe_results,
                algorithm_methodology, limitations_disclaimer, ledger_root_hash.
        """
        client, entity = client_with_entity
        cert = await client.certifications.generate(entity.gdpr_request_id)
        report = await cert.get_json()

        required_fields = [
            "gdpr_request_id", "issued_at", "org_id",
            "total_inferences_scanned", "disclosure_events",
            "probe_results", "algorithm_methodology",
            "limitations_disclaimer", "ledger_root_hash",
            "chain_integrity_verified", "signed_by"
        ]
        for field in required_fields:
            assert field in report, f"Missing required field: {field}"

    @pytest.mark.asyncio
    async def test_certification_report_is_cryptographically_signed(
        self, client_with_entity
    ):
        """
        WHAT:   Certification PDF is signed with ForgetMeNot's Ed25519 key.
        GIVEN:  A generated certification report.
        EXPECT: Signature verifies against ForgetMeNot's public key.
                Tampering with report content breaks signature verification.
        """
        client, entity = client_with_entity
        cert = await client.certifications.generate(entity.gdpr_request_id)
        signer = ReportSigner()

        # Original signature verifies
        is_valid = await signer.verify(cert.report_pdf_path)
        assert is_valid is True

        # Tampered report fails verification
        tampered_path = await cert._tamper_for_testing()
        is_valid_tampered = await signer.verify(tampered_path)
        assert is_valid_tampered is False

    @pytest.mark.asyncio
    async def test_certification_with_disclosure_events_fails(
        self, client_with_entity
    ):
        """
        WHAT:   Certification is NOT issued if disclosure events exist.
        GIVEN:  A request where 1 disclosure event is logged (scanner failure).
        EXPECT: cert.status == "FAILED". Cert not issued. Remediation steps provided.
        """
        client, entity = client_with_entity

        # Inject a simulated disclosure event
        await client.ledger._inject_disclosure_event_for_testing(
            entity.gdpr_request_id
        )

        cert = await client.certifications.generate(entity.gdpr_request_id)

        assert cert.status == "FAILED"
        assert cert.report_pdf_url is None
        assert cert.failure_reason == "DISCLOSURE_EVENT_DETECTED"
        assert len(cert.remediation_steps) > 0

    @pytest.mark.asyncio
    async def test_limitations_disclaimer_always_present(
        self, client_with_entity
    ):
        """
        WHAT:   The legal limitations disclaimer is always included, never omitted.
        GIVEN:  Any certification report.
        EXPECT: Disclaimer contains "probabilistic", "not cryptographic",
                "does not constitute legal advice".
        CRITICAL: Omitting the disclaimer creates false legal certainty — a liability.
        """
        client, entity = client_with_entity
        cert = await client.certifications.generate(entity.gdpr_request_id)
        report = await cert.get_json()

        disclaimer = report["limitations_disclaimer"].lower()
        assert "probabilistic" in disclaimer
        assert "not cryptographic" in disclaimer or "probabilistic" in disclaimer
        assert "does not constitute legal advice" in disclaimer

    @pytest.mark.asyncio
    async def test_certification_report_contains_no_plaintext_pii(
        self, client_with_entity
    ):
        """
        WHAT:   The certification report is safe to hand to regulators —
                contains zero plaintext PII of the data subject.
        GIVEN:  Entity "Alice Johnson" with email "alice@acme.com".
        EXPECT: Report JSON does not contain "Alice Johnson", "alice@acme.com",
                or any raw alias.
        """
        client, entity = client_with_entity
        cert = await client.certifications.generate(entity.gdpr_request_id)
        report_str = str(await cert.get_json())

        assert "Alice Johnson" not in report_str
        assert "alice@acme.com" not in report_str
        assert "A. Johnson" not in report_str
```

***

## UNIT TESTS — `tests/unit/test_preflight.py`

```python
"""
COMPONENT: Pre-Flight Check API
PURPOSE:   Analyzes AI feature architecture before deployment.
           Surfaces GDPR/DPDPA risks before they become violations.
"""
import pytest
from forgetmenot.preflight.analyzer import PreFlightAnalyzer


class TestPreFlightAnalyzer:

    @pytest.mark.asyncio
    async def test_system_prompt_full_name_instruction_flagged_high(self):
        """
        WHAT:   Instruction to use full user name in outputs is flagged HIGH.
        INPUT:  system_prompt = "Always address user by their full name."
        EXPECT: Issue with severity=HIGH, gdpr_article="Art. 5(1)(c)", actionable fix.
        """
        analyzer = PreFlightAnalyzer()
        result = await analyzer.analyze(
            system_prompt="You are helpful. Always address user by their full name.",
            rag_datasources=[],
            intended_user_base={"geography": "EU", "size": 1000}
        )

        high_issues = [i for i in result.issues if i.severity == "HIGH"]
        assert len(high_issues) >= 1
        assert any("Art. 5(1)(c)" in i.gdpr_article for i in high_issues)
        assert any("full name" in i.issue.lower() for i in high_issues)
        assert all(i.fix is not None for i in high_issues)

    @pytest.mark.asyncio
    async def test_salary_data_in_rag_flagged_as_article_9(self):
        """
        WHAT:   Salary/financial data in RAG datasource flagged as Art. 9 special category.
        INPUT:  rag_datasources=[{"name": "hr.csv", "columns": ["name", "salary"]}]
        EXPECT: Issue with severity=HIGH, gdpr_article="Art. 9".
        """
        analyzer = PreFlightAnalyzer()
        result = await analyzer.analyze(
            system_prompt="You are an HR assistant.",
            rag_datasources=[
                {"name": "hr_records.csv", "sample_columns": ["employee_id", "salary", "manager"]}
            ],
            intended_user_base={"geography": "EU", "size": 10000}
        )

        assert any("Art. 9" in i.gdpr_article for i in result.issues)

    @pytest.mark.asyncio
    async def test_clean_config_returns_low_risk(self):
        """
        WHAT:   A privacy-by-design configuration returns LOW risk score.
        INPUT:  No PII in prompt, no sensitive columns in RAG, small user base.
        EXPECT: result.risk_score == "LOW". Zero HIGH issues.
        """
        analyzer = PreFlightAnalyzer()
        result = await analyzer.analyze(
            system_prompt="You are a helpful assistant. Answer questions about France.",
            rag_datasources=[
                {"name": "france_wiki.jsonl", "sample_columns": ["title", "content"]}
            ],
            intended_user_base={"geography": "EU", "size": 500}
        )

        assert result.risk_score == "LOW"
        assert all(i.severity != "HIGH" for i in result.issues)

    @pytest.mark.asyncio
    async def test_recommended_tier_scales_with_user_base(self):
        """
        WHAT:   Recommended ForgetMeNot tier increases with user base size.
        GIVEN:  Three user base sizes: 500, 10k, 100k.
        EXPECT: 500 users → Shield, 10k → Growth, 100k → Enterprise.
        """
        analyzer = PreFlightAnalyzer()
        tiers = {}
        for size, expected_tier in [(500, "Shield"), (10000, "Growth"), (100000, "Enterprise")]:
            result = await analyzer.analyze(
                system_prompt="Basic assistant",
                rag_datasources=[],
                intended_user_base={"geography": "EU", "size": size}
            )
            tiers[size] = result.recommended_tier

        assert tiers[500] == "Shield"
        assert tiers[10000] == "Growth"
        assert tiers[100000] == "Enterprise"

    @pytest.mark.asyncio
    async def test_preflight_includes_erasure_request_forecast(self):
        """
        WHAT:   Pre-flight result includes estimated erasure request volume.
        GIVEN:  EU user base of 50,000.
        EXPECT: estimated_erasure_requests_per_month is a non-zero positive integer range.
        """
        analyzer = PreFlightAnalyzer()
        result = await analyzer.analyze(
            system_prompt="Customer service assistant",
            rag_datasources=[],
            intended_user_base={"geography": "EU", "size": 50000}
        )

        assert result.estimated_erasure_requests_per_month is not None
        assert result.estimated_erasure_requests_per_month["low"] > 0
        assert result.estimated_erasure_requests_per_month["high"] > \
               result.estimated_erasure_requests_per_month["low"]
```

***

## UNIT TESTS — `tests/unit/test_pii_scrubber.py`

```python
"""
COMPONENT: PII Scrubber (Logging Middleware)
PURPOSE:   Ensures ZERO plaintext PII ever enters logs, traces, or error messages.
           Protects ForgetMeNot from inheriting customer GDPR liability.
CRITICAL:  These tests must never be skipped. Ever.
"""
import pytest
from forgetmenot.middleware.pii_scrubber import PIIScrubber


class TestPIIScrubber:

    def test_email_scrubbed_from_log_message(self):
        """
        WHAT:   Email addresses are replaced with [EMAIL_REDACTED] in log output.
        INPUT:  "Error processing request for user@example.com"
        EXPECT: "Error processing request for [EMAIL_REDACTED]"
        """
        scrubber = PIIScrubber()
        result = scrubber.scrub("Error processing request for user@example.com")
        assert "user@example.com" not in result
        assert "[EMAIL_REDACTED]" in result

    def test_multiple_emails_all_scrubbed(self):
        """
        WHAT:   Multiple emails in the same string all scrubbed.
        INPUT:  String with 3 email addresses.
        EXPECT: All 3 replaced. No email remains.
        """
        scrubber = PIIScrubber()
        log = "Sent to alice@a.com, bob@b.com, and carol@c.com"
        result = scrubber.scrub(log)
        assert "alice@a.com" not in result
        assert "bob@b.com" not in result
        assert "carol@c.com" not in result
        assert result.count("[EMAIL_REDACTED]") == 3

    def test_json_body_pii_scrubbed(self):
        """
        WHAT:   PII in a JSON request body is scrubbed before logging.
        INPUT:  JSON with canonical_name and email fields.
        EXPECT: Both values replaced with redaction placeholders.
        """
        import json
        scrubber = PIIScrubber()
        body = json.dumps({
            "canonical_name": "Alice Johnson",
            "email": "alice@acme.com",
            "gdpr_request_id": "GDPR-001"   # This should NOT be scrubbed
        })
        result = scrubber.scrub(body)

        assert "Alice Johnson" not in result
        assert "alice@acme.com" not in result
        assert "GDPR-001" in result   # Non-PII fields preserved

    def test_safe_content_not_scrubbed(self):
        """
        WHAT:   Non-PII log content passes through unchanged.
        INPUT:  "Job GDPR-001 completed in 14.2ms. Status: ISSUED."
        EXPECT: Exact same string returned.
        """
        scrubber = PIIScrubber()
        safe_log = "Job GDPR-001 completed in 14.2ms. Status: ISSUED."
        assert scrubber.scrub(safe_log) == safe_log

    def test_exception_message_pii_scrubbed(self):
        """
        WHAT:   If an exception contains PII (e.g., in stack trace),
                it's scrubbed before being logged.
        INPUT:  Exception with "alice@acme.com" in message.
        EXPECT: Scrubbed exception message — no PII.
        """
        scrubber = PIIScrubber()
        exc_msg = "ValidationError: invalid entity 'Alice Johnson' (alice@acme.com)"
        result = scrubber.scrub_exception(Exception(exc_msg))
        assert "Alice Johnson" not in result
        assert "alice@acme.com" not in result
```

***

## INTEGRATION TESTS — `tests/integration/test_api_registry.py`

```python
"""
COMPONENT: Registry API Endpoints
PURPOSE:   Full HTTP-level integration tests for /v1/registry/* endpoints.
           Tests auth, validation, response shapes, and error codes.
"""
import pytest
from httpx import AsyncClient
from forgetmenot.api.main import app


@pytest.fixture
async def api_client():
    async with AsyncClient(app=app, base_url="http://test") as client:
        yield client


@pytest.fixture
def valid_headers():
    return {
        "Authorization": "Bearer fnot_sk_test_validkey",
        "X-Org-ID": "org_test_acme"
    }


class TestRegistryAPIEndpoints:

    @pytest.mark.asyncio
    async def test_post_registry_entity_returns_201(
        self, api_client, valid_headers, full_entity_payload
    ):
        """
        WHAT:   POST /v1/registry/entities returns 201 with entity ID.
        INPUT:  Valid EntityRegistrationRequest JSON body.
        EXPECT: HTTP 201. Body contains entity.id, entity.deadline.
        """
        response = await api_client.post(
            "/v1/registry/entities",
            json=full_entity_payload.dict(),
            headers=valid_headers
        )
        assert response.status_code == 201
        body = response.json()
        assert "id" in body
        assert body["id"].startswith("ent_")
        assert "deadline" in body

    @pytest.mark.asyncio
    async def test_post_registry_entity_without_auth_returns_401(
        self, api_client, full_entity_payload
    ):
        """
        WHAT:   Unauthenticated request to registry returns 401.
        INPUT:  Valid payload, NO Authorization header.
        EXPECT: HTTP 401.
        """
        response = await api_client.post(
            "/v1/registry/entities",
            json=full_entity_payload.dict()
        )
        assert response.status_code == 401

    @pytest.mark.asyncio
    async def test_post_registry_entity_missing_canonical_name_returns_422(
        self, api_client, valid_headers
    ):
        """
        WHAT:   Missing required field returns 422 Unprocessable Entity.
        INPUT:  Payload without canonical_name field.
        EXPECT: HTTP 422. Error detail mentions "canonical_name".
        """
        response = await api_client.post(
            "/v1/registry/entities",
            json={"gdpr_request_id": "TEST-NO-NAME", "aliases": ["test@test.com"]},
            headers=valid_headers
        )
        assert response.status_code == 422
        assert "canonical_name" in response.text.lower()

    @pytest.mark.asyncio
    async def test_get_entity_status_returns_200(
        self, api_client, valid_headers, full_entity_payload
    ):
        """
        WHAT:   GET /v1/registry/entities/{id}/status returns 200 with tracking data.
        GIVEN:  A previously registered entity.
        EXPECT: HTTP 200. Body contains days_remaining, total_inferences_scanned.
        """
        # First register
        post_response = await api_client.post(
            "/v1/registry/entities",
            json=full_entity_payload.dict(),
            headers=valid_headers
        )
        entity_id = post_response.json()["id"]

        # Then get status
        status_response = await api_client.get(
            f"/v1/registry/entities/{entity_id}/status",
            headers=valid_headers
        )
        assert status_response.status_code == 200
        body = status_response.json()
        assert "days_remaining" in body
        assert "total_inferences_scanned" in body
        assert "disclosure_events" in body

    @pytest.mark.asyncio
    async def test_get_entity_list_contains_no_pii(
        self, api_client, valid_headers, full_entity_payload
    ):
        """
        WHAT:   GET /v1/registry/entities returns entity list with zero PII.
        GIVEN:  One registered entity "Carol White".
        EXPECT: Response body contains entity IDs only. "Carol White" not present.
        """
        await api_client.post(
            "/v1/registry/entities",
            json=full_entity_payload.dict(),
            headers=valid_headers
        )
        response = await api_client.get(
            "/v1/registry/entities",
            headers=valid_headers
        )
        assert response.status_code == 200
        assert "Carol White" not in response.text
        assert "carol@corp.com" not in response.text

    @pytest.mark.asyncio
    async def test_cross_tenant_entity_access_denied(
        self, api_client, full_entity_payload
    ):
        """
        WHAT:   Org A cannot access Org B's registered entities.
        GIVEN:  Entity registered by org_acme.
        WHEN:   org_rival tries to get that entity's status.
        EXPECT: HTTP 404 (not 403 — don't confirm the entity exists).
        """
        org_a_headers = {"Authorization": "Bearer fnot_sk_test_org_a", "X-Org-ID": "org_a"}
        org_b_headers = {"Authorization": "Bearer fnot_sk_test_org_b", "X-Org-ID": "org_b"}

        # Org A registers
        post = await api_client.post(
            "/v1/registry/entities",
            json=full_entity_payload.dict(),
            headers=org_a_headers
        )
        entity_id = post.json()["id"]

        # Org B tries to access Org A's entity
        response = await api_client.get(
            f"/v1/registry/entities/{entity_id}/status",
            headers=org_b_headers
        )
        assert response.status_code == 404
```

***

## SECURITY TESTS — `tests/security/test_pii_never_logged.py`

```python
"""
COMPONENT: PII Non-Exposure (Security)
PURPOSE:   Prove that zero PII flows into ANY observability system:
           logs, traces, metrics, error reports, Datadog, Sentry.
CRITICAL:  These tests protect ForgetMeNot from inheriting GDPR liability.
"""
import pytest
import logging
from unittest.mock import patch, MagicMock


class TestPIINeverLogged:

    @pytest.mark.asyncio
    async def test_pii_entity_name_not_in_structured_logs(
        self, client_with_entity, caplog
    ):
        """
        WHAT:   Entity canonical name never appears in structured application logs.
        GIVEN:  A blocked inference for "Alice Johnson".
        EXPECT: Log output contains zero occurrences of "Alice Johnson".
        """
        client, entity = client_with_entity

        with caplog.at_level(logging.DEBUG):
            llm = client.wrap(MockLLMClient(output="Alice Johnson's report"))
            await llm.chat("test")

        assert "Alice Johnson" not in caplog.text
        assert "alice@acme.com" not in caplog.text

    @pytest.mark.asyncio
    async def test_pii_not_in_opentelemetry_span_attributes(
        self, client_with_entity
    ):
        """
        WHAT:   PII is absent from all OpenTelemetry span attributes and events.
        GIVEN:  A blocked inference that creates trace spans.
        EXPECT: All span attributes and events are PII-free.
        """
        from opentelemetry.sdk.trace import TracerProvider
        from opentelemetry.sdk.trace.export.in_memory_span_exporter import (
            InMemorySpanExporter
        )

        exporter = InMemorySpanExporter()
        client, entity = client_with_entity

        with patch("forgetmenot.middleware.telemetry.get_tracer_provider",
                   return_value=TracerProvider()):
            llm = client.wrap(MockLLMClient(output="Alice Johnson's email: alice@acme.com"))
            await llm.chat("test")

        spans = exporter.get_finished_spans()
        for span in spans:
            span_str = str(span.attributes) + str(span.events)
            assert "Alice Johnson" not in span_str
            assert "alice@acme.com" not in span_str

    @pytest.mark.asyncio
    async def test_exception_with_pii_is_scrubbed_before_sentry(
        self, client_with_entity
    ):
        """
        WHAT:   If an exception occurs and Sentry captures it,
                PII in the exception message is scrubbed first.
        GIVEN:  A ValueError containing entity name thrown during processing.
        EXPECT: Sentry capture_exception called with scrubbed message.
        """
        sentry_captured = []

        with patch("sentry_sdk.capture_exception",
                   side_effect=lambda e: sentry_captured.append(str(e))):
            client, entity = client_with_entity
            llm = client.wrap(
                MockLLMClient(raises=ValueError("Processing failed for Alice Johnson"))
            )
            try:
                await llm.chat("test")
            except Exception:
                pass

        for captured in sentry_captured:
            assert "Alice Johnson" not in captured

    @pytest.mark.asyncio
    async def test_request_body_pii_scrubbed_in_access_logs(
        self, api_client, valid_headers
    ):
        """
        WHAT:   HTTP access logs (e.g., nginx/Kong) don't log request bodies
                containing PII from forget set submissions.
        GIVEN:  A POST /v1/registry/entities request with entity data.
        EXPECT: Access log output does not contain entity name or email.
        """
        access_log_output = []

        with patch("forgetmenot.middleware.request_logger.write_access_log",
                   side_effect=lambda msg: access_log_output.append(msg)):
            await api_client.post(
                "/v1/registry/entities",
                json={
                    "gdpr_request_id": "LOG-TEST-001",
                    "canonical_name": "Sensitive Person",
                    "aliases": ["sensitive@example.com"]
                },
                headers=valid_headers
            )

        full_log = " ".join(access_log_output)
        assert "Sensitive Person" not in full_log
        assert "sensitive@example.com" not in full_log
```

***

## SECURITY TESTS — `tests/security/test_tenant_isolation.py`

```python
"""
COMPONENT: Multi-Tenant Isolation
PURPOSE:   Prove that no data crosses tenant boundaries at any layer.
"""
import pytest


class TestTenantIsolation:

    @pytest.mark.asyncio
    async def test_org_a_ledger_invisible_to_org_b(self, api_client):
        """
        WHAT:   Org A's ledger events are completely invisible to Org B.
        GIVEN:  Org A has 50 ledger events. Org B has 0.
        WHEN:   Org B queries /v1/ledger/events.
        EXPECT: 0 events returned to Org B. No Org A data visible.
        """
        org_a = {"Authorization": "Bearer token_a", "X-Org-ID": "org_a"}
        org_b = {"Authorization": "Bearer token_b", "X-Org-ID": "org_b"}

        # Org A generates 50 events
        for _ in range(50):
            await api_client.post(
                "/v1/scan/output",
                json={"output": "Safe content about France."},
                headers=org_a
            )

        # Org B should see zero
        response = await api_client.get("/v1/ledger/events", headers=org_b)
        assert response.status_code == 200
        assert response.json()["total"] == 0

    @pytest.mark.asyncio
    async def test_org_a_forget_registry_doesnt_filter_org_b_outputs(
        self, api_client
    ):
        """
        WHAT:   Org A registering "Alice Johnson" does NOT affect Org B's scans.
        GIVEN:  Org A registers Alice Johnson as a forget entity.
        WHEN:   Org B scans an output containing "Alice Johnson".
        EXPECT: Org B's scan result is PASS — registry is org-scoped.
        """
        org_a = {"Authorization": "Bearer token_a", "X-Org-ID": "org_a"}
        org_b = {"Authorization": "Bearer token_b", "X-Org-ID": "org_b"}

        # Org A registers Alice Johnson
        await api_client.post(
            "/v1/registry/entities",
            json={
                "gdpr_request_id": "ISOLATION-TEST-001",
                "canonical_name": "Alice Johnson",
                "aliases": ["alice@acme.com"]
            },
            headers=org_a
        )

        # Org B scans output with Alice Johnson's name
        scan_response = await api_client.post(
            "/v1/scan/output",
            json={"output": "Alice Johnson submitted the report."},
            headers=org_b
        )

        # Org B should NOT be affected — Alice is only in Org A's registry
        assert scan_response.json()["action"] == "PASS"
```

***

## PERFORMANCE TESTS — `tests/performance/test_firewall_latency.py`

```python
"""
COMPONENT: Firewall Engine Performance
PURPOSE:   Enforce the latency budget from the architecture document.
           These run in CI on every PR to catch performance regressions.
"""
import pytest
import asyncio
import time
import statistics


class TestFirewallLatencyBudget:

    @pytest.mark.asyncio
    @pytest.mark.performance
    async def test_p50_latency_under_15ms(self, client_with_entity):
        """
        WHAT:   P50 (median) firewall overhead is < 15ms.
        GIVEN:  1000 inference calls, clean 200-token output.
        EXPECT: Median additional latency < 15ms.
        """
        client, entity = client_with_entity
        llm = client.wrap(MockLLMClient(output="Safe content. " * 15))
        latencies = []

        for _ in range(1000):
            start = time.perf_counter()
            await llm.chat("test")
            latencies.append((time.perf_counter() - start) * 1000)

        p50 = statistics.median(latencies)
        assert p50 < 15.0, f"P50 latency {p50:.2f}ms exceeds 15ms budget"

    @pytest.mark.asyncio
    @pytest.mark.performance
    async def test_p95_latency_under_20ms(self, client_with_entity):
        """
        WHAT:   P95 firewall overhead is < 20ms.
        GIVEN:  1000 inference calls.
        EXPECT: 95th percentile latency < 20ms.
        """
        client, entity = client_with_entity
        llm = client.wrap(MockLLMClient(output="Safe content. " * 15))
        latencies = []

        for _ in range(1000):
            start = time.perf_counter()
            await llm.chat("test")
            latencies.append((time.perf_counter() - start) * 1000)

        p95 = sorted(latencies)[949]
        assert p95 < 20.0, f"P95 latency {p95:.2f}ms exceeds 20ms budget"

    @pytest.mark.asyncio
    @pytest.mark.performance
    async def test_p99_latency_under_30ms(self, client_with_entity):
        """
        WHAT:   P99 firewall overhead < 30ms — even worst-case is acceptable.
        GIVEN:  1000 inference calls.
        EXPECT: 99th percentile latency < 30ms.
        """
        client, entity = client_with_entity
        llm = client.wrap(MockLLMClient(output="Safe content. " * 15))
        latencies = [0] * 1000

        for i in range(1000):
            start = time.perf_counter()
            await llm.chat("test")
            latencies[i] = (time.perf_counter() - start) * 1000

        p99 = sorted(latencies)[989]
        assert p99 < 30.0, f"P99 latency {p99:.2f}ms exceeds 30ms budget"

    @pytest.mark.asyncio
    @pytest.mark.performance
    async def test_concurrent_100_requests_no_degradation(
        self, client_with_entity
    ):
        """
        WHAT:   100 concurrent inference calls don't degrade each other's latency.
        GIVEN:  100 simultaneous requests fired at once.
        EXPECT: P95 concurrent latency < 2× single-request P95.
        """
        client, entity = client_with_entity
        llm = client.wrap(MockLLMClient(output="Safe content."))

        async def timed_call():
            start = time.perf_counter()
            await llm.chat("test")
            return (time.perf_counter() - start) * 1000

        latencies = await asyncio.gather(*[timed_call() for _ in range(100)])
        p95_concurrent = sorted(latencies)[94]

        assert p95_concurrent < 40.0, (
            f"Concurrent P95 {p95_concurrent:.1f}ms — too much degradation under load"
        )

    @pytest.mark.asyncio
    @pytest.mark.performance
    async def test_large_registry_doesnt_degrade_scan_performance(
        self, local_client
    ):
        """
        WHAT:   Scan performance is stable with 10,000 entities in registry.
        GIVEN:  10,000 registered entities + 1 inference call.
        EXPECT: Scan latency < 25ms (< 5ms over normal p95).
        NOTE:   Redis hot cache makes this O(1) — this test validates the cache works.
        """
        # Register 10,000 entities
        for i in range(10000):
            await local_client.registry.register(
                EntityRegistrationRequest(
                    gdpr_request_id=f"PERF-TEST-{i:05d}",
                    canonical_name=f"Person {i}",
                    aliases=[f"person{i}@example.com"]
                )
            )

        llm = local_client.wrap(MockLLMClient(output="Safe content about France."))

        latencies = []
        for _ in range(100):
            start = time.perf_counter()
            await llm.chat("test")
            latencies.append((time.perf_counter() - start) * 1000)

        p95 = sorted(latencies)[94]
        assert p95 < 25.0, f"P95 with 10k registry {p95:.1f}ms — cache may not be working"
```

***

## E2E TESTS — `tests/e2e/test_full_gdpr_lifecycle.py`

```python
"""
COMPONENT: Full GDPR Lifecycle (End-to-End)
PURPOSE:   Simulate the complete journey from erasure request to certification.
           This is the most important test file — it validates the product works
           as a whole system, not just as individual components.
"""
import pytest
import asyncio
from datetime import timedelta
from unittest.mock import patch


class TestFullGDPRLifecycle:

    @pytest.mark.asyncio
    @pytest.mark.e2e
    async def test_complete_lifecycle_request_to_certification(
        self, local_client
    ):
        """
        WHAT:   The complete GDPR erasure lifecycle works end-to-end.

        FLOW:
          Step 1: DPO receives erasure request, registers entity
          Step 2: Developer Playbook auto-generated
          Step 3: RAG filter activated
          Step 4: 30 inference calls — all clean (zero disclosure events)
          Step 5: 5 inference calls with PII attempts — all blocked
          Step 6: Certification report auto-generated
          Step 7: Report is signed, valid, contains correct counts
          Step 8: Data subject verification token issued

        EXPECT:  Certification issued. Signed. Zero disclosure events.
                 Verification portal returns valid status.
        """
        # ── Step 1: Register erasure request ─────────────────────────────────
        entity = await local_client.registry.register(
            gdpr_request_id="E2E-GDPR-001",
            canonical_name="David Chen",
            aliases=["david@megacorp.com", "D. Chen"],
            role="VP of Product",
            organization="MegaCorp",
            related_entities=["MegaCorp", "Product Division", "Project Titan"]
        )

        assert entity.id is not None
        assert entity.status == "ACTIVE"
        assert entity.deadline is not None

        # ── Step 2: Verify Developer Playbook generated ───────────────────────
        playbook = await local_client.playbook.get(entity.gdpr_request_id)

        assert playbook is not None
        assert len(playbook.automated_tasks) > 0
        assert len(playbook.manual_tasks) >= 0
        assert playbook.deadline == entity.deadline
        assert playbook.progress_pct >= 0

        # ── Step 3: Verify RAG filter activated ──────────────────────────────
        rag_status = await local_client.rag.get_filter_status(entity.id)
        assert rag_status.is_active is True

        # ── Step 4: 30 clean inference calls — all should pass ────────────────
        clean_llm = local_client.wrap(
            MockLLMClient(output="MegaCorp had a great quarter in product revenue.")
        )

        for i in range(30):
            response = await clean_llm.chat(f"Tell me about MegaCorp Q{i%4+1}")
            assert response.forgetmenot.status == "PASS", (
                f"Clean inference {i} unexpectedly blocked"
            )

        # ── Step 5: 5 PII-containing inference attempts — all should block ────
        Continuing exactly from the cut-off point:

```python
        pii_outputs = [
            "David Chen is VP of Product at MegaCorp.",
            "Please CC david@megacorp.com on the agenda.",
            "The VP of Product at MegaCorp approved the roadmap.",
            "D. Chen signed off on Project Titan last Tuesday.",
            "The executive who led Project Titan at MegaCorp has left the company.",
        ]

        for i, pii_output in enumerate(pii_outputs):
            pii_llm = local_client.wrap(MockLLMClient(output=pii_output))
            response = await pii_llm.chat("Tell me about the product team")
            assert response.forgetmenot.status in ["BLOCK", "REDACTED"], (
                f"PII inference {i} was NOT blocked. Output: {pii_output}"
            )
            assert "David Chen" not in response.content
            assert "david@megacorp.com" not in response.content

        # ── Step 6: Verify ledger has correct event counts ────────────────────
        status = await local_client.registry.get_status("E2E-GDPR-001")

        assert status.total_inferences_scanned == 35     # 30 clean + 5 pii
        assert status.disclosure_events == 0             # Zero PII leaked
        assert status.blocked_events == 5                # Five correctly blocked

        # ── Step 7: Generate certification report ─────────────────────────────
        # Fast-forward clock to Day 30 for test purposes
        with patch("forgetmenot.certification.report_engine.get_current_time",
                   return_value=entity.deadline):
            cert = await local_client.certifications.generate("E2E-GDPR-001")

        assert cert.status == "ISSUED"
        assert cert.report_pdf_url is not None

        # Verify report contents
        report = await cert.get_json()
        assert report["total_inferences_scanned"] == 35
        assert report["disclosure_events"] == 0
        assert report["blocked_events"] == 5
        assert report["chain_integrity_verified"] is True
        assert report["probes_passed"] == 10
        assert report["probes_total"] == 10

        # Verify cryptographic signature
        from forgetmenot.certification.signer import ReportSigner
        signer = ReportSigner()
        assert await signer.verify(cert.report_pdf_path) is True

        # ── Step 8: Issue Data Subject verification token ─────────────────────
        token = await local_client.certifications.issue_verification_token(
            "E2E-GDPR-001"
        )

        assert token.token_id is not None
        assert token.verification_url.startswith("https://verify.forgetmenot.io/")
        assert token.expires_at > entity.deadline

        # Verify the token resolves correctly
        verification = await local_client.verification_portal.check(token.token_id)
        assert verification.status == "VERIFIED"
        assert verification.systems_covered >= 1
        assert verification.disclosure_events == 0


    @pytest.mark.asyncio
    @pytest.mark.e2e
    async def test_deadline_missed_triggers_alert_not_silent_failure(
        self, local_client
    ):
        """
        WHAT:   If the 30-day deadline passes without certification,
                an alert is fired — it NEVER silently fails.
        GIVEN:  A registered entity. Clock advanced to Day 31. No cert issued.
        EXPECT: PagerDuty-equivalent alert fired.
                Entity status == "DEADLINE_BREACHED".
                DPO webhook called.
        """
        dpo_webhook_called = []

        entity = await local_client.registry.register(
            gdpr_request_id="E2E-DEADLINE-001",
            canonical_name="Missed Deadline Person",
            aliases=["missed@test.com"],
            dpo_webhook="https://test-webhook.example.com/dpo-alerts"
        )

        with patch("forgetmenot.alerts.webhook.call",
                   side_effect=lambda url, data: dpo_webhook_called.append(data)):
            # Advance clock to Day 31
            with patch("forgetmenot.registry.deadline_monitor.get_current_time",
                       return_value=entity.deadline + timedelta(days=1)):
                await local_client.deadline_monitor.run_check()

        status = await local_client.registry.get_status("E2E-DEADLINE-001")

        assert status.entity_status == "DEADLINE_BREACHED"
        assert len(dpo_webhook_called) >= 1
        assert dpo_webhook_called[0]["event"] == "GDPR_DEADLINE_BREACHED"
        assert dpo_webhook_called[0]["gdpr_request_id"] == "E2E-DEADLINE-001"


    @pytest.mark.asyncio
    @pytest.mark.e2e
    async def test_dispute_workflow_full_lifecycle(self, local_client):
        """
        WHAT:   Full false-positive dispute lifecycle works end-to-end.

        FLOW:
          Step 1: Entity registered.
          Step 2: Inference is blocked (false positive — wrong John Smith).
          Step 3: Developer files dispute with evidence.
          Step 4: DPO reviews and approves dispute.
          Step 5: Output is released.
          Step 6: Scanner recalibrated for this context.
          Step 7: Same output passes on next inference.

        EXPECT:  Post-approval, the same output is no longer blocked.
        """
        # Step 1: Register entity
        entity = await local_client.registry.register(
            gdpr_request_id="E2E-DISPUTE-001",
            canonical_name="John Smith",
            role="CFO",
            organization="Acme Corp",
            aliases=["jsmith@acme.com"]
        )

        # Step 2: Output about a DIFFERENT John Smith gets blocked (false positive)
        fp_output = "John Smith, lead engineer at Different Corp, won the hackathon."
        blocked_llm = local_client.wrap(MockLLMClient(output=fp_output))
        blocked_response = await blocked_llm.chat("Who won the hackathon?")

        # It might be blocked (CEDM isn't perfect — this is the false positive case)
        assert blocked_response.forgetmenot.was_redacted is True

        # Step 3: Developer files dispute
        dispute = await blocked_response.forgetmenot.dispute(
            reason="False positive — 'John Smith' here is an engineer at Different Corp, "
                   "not the registered CFO at Acme Corp.",
            evidence="User asked about a hackathon, not about Acme Corp executives."
        )

        assert dispute.id is not None
        assert dispute.status == "PENDING"
        assert dispute.response_deadline is not None

        # Step 4: DPO approves the dispute
        await local_client.disputes.resolve(
            dispute_id=dispute.id,
            resolution="APPROVED",
            dpo_notes="Confirmed false positive. Different person with same name."
        )

        resolved = await local_client.disputes.get(dispute.id)
        assert resolved.status == "APPROVED"

        # Step 5 + 6: Same output now passes after recalibration
        recalibrated_llm = local_client.wrap(MockLLMClient(output=fp_output))
        recalibrated_response = await recalibrated_llm.chat("Who won the hackathon?")

        assert recalibrated_response.forgetmenot.status == "PASS", (
            "Scanner was NOT recalibrated after dispute approval"
        )
        assert recalibrated_response.content == fp_output

        # Step 7: Verify dispute is in calibration training set
        calibration_log = await local_client.scanner.get_calibration_log()
        assert dispute.id in [c.dispute_id for c in calibration_log]


    @pytest.mark.asyncio
    @pytest.mark.e2e
    async def test_proactive_pii_discovery_surfaces_unregistered_entities(
        self, local_client
    ):
        """
        WHAT:   Proactive PII discovery detects entities that appear frequently
                in inference logs but have NO forget registration.
        GIVEN:  "bob@bigcorp.com" appears in 850+ inference outputs.
                No forget registration for bob@bigcorp.com.
        EXPECT: Weekly digest contains bob@bigcorp.com as a recommended registration.
                DPO receives alert with one-click register action.
        """
        # Generate 900 inference events all containing bob@bigcorp.com
        # (simulates a system leaking an email that should be registered)
        leaky_llm = local_client.wrap(
            MockLLMClient(output="Contact bob@bigcorp.com for approval.")
        )

        for _ in range(900):
            # Use bypass_scan=True — we want these to pass so they hit the discovery engine
            await leaky_llm.chat("How do I get approval?", _bypass_scan=True)

        # Run the proactive discovery engine
        await local_client.discovery.run_weekly_scan()

        # Check digest
        digest = await local_client.discovery.get_latest_digest()

        assert digest is not None
        assert len(digest.recommended_registrations) >= 1

        # The email should be in the recommendations (as a hash — not plaintext)
        email_hash = local_client._hash("bob@bigcorp.com")
        assert any(
            r.entity_hash == email_hash
            for r in digest.recommended_registrations
        )

        # Verify one-click register action is present
        recommendation = digest.recommended_registrations[0]
        assert recommendation.register_action_url is not None
        assert recommendation.occurrence_count >= 900


    @pytest.mark.asyncio
    @pytest.mark.e2e
    async def test_compliance_time_machine_reconstructs_historical_state(
        self, local_client
    ):
        """
        WHAT:   The Compliance Time Machine returns accurate state for any past date.
        GIVEN:  Events spread across Jan 1, Jan 15, and Feb 1.
        WHEN:   Snapshot requested for Jan 20.
        EXPECT: Snapshot includes Jan 1 and Jan 15 events. NOT Feb 1 events.
                Chain integrity verified for that snapshot window.
                PDF export generates successfully.
        """
        from datetime import datetime

        jan_1  = datetime(2026, 1, 1,  0, 0, 0, tzinfo=timezone.utc)
        jan_15 = datetime(2026, 1, 15, 0, 0, 0, tzinfo=timezone.utc)
        feb_1  = datetime(2026, 2, 1,  0, 0, 0, tzinfo=timezone.utc)
        jan_20 = datetime(2026, 1, 20, 0, 0, 0, tzinfo=timezone.utc)

        for ts, action in [
            (jan_1,  "PASS"),
            (jan_1,  "PASS"),
            (jan_15, "BLOCK"),
            (jan_15, "PASS"),
            (feb_1,  "PASS"),
            (feb_1,  "PASS"),
        ]:
            await local_client.ledger._inject_event_at_time(
                action=action, timestamp=ts, org_id="org_time_machine_test"
            )

        # Request snapshot for Jan 20
        snapshot = await local_client.ledger.get_snapshot(
            org_id="org_time_machine_test",
            at_time=jan_20
        )

        assert snapshot.total_events == 4        # Jan 1 (x2) + Jan 15 (x2)
        assert snapshot.block_events == 1        # Jan 15 block
        assert snapshot.chain_integrity is True

        # PDF export must work
        pdf_path = await snapshot.export_pdf()
        assert pdf_path.endswith(".pdf")
        assert os.path.exists(pdf_path)
        assert os.path.getsize(pdf_path) > 1000  # Not an empty PDF
```

***

## E2E TESTS — `tests/e2e/test_full_dpdpa_lifecycle.py`

```python
"""
COMPONENT: Full DPDPA (India) Lifecycle
PURPOSE:   Validates India's Digital Personal Data Protection Act compliance
           works identically to GDPR — same stack, different jurisdiction flag.
"""
import pytest


class TestFullDPDPALifecycle:

    @pytest.mark.asyncio
    @pytest.mark.e2e
    async def test_dpdpa_erasure_request_uses_correct_jurisdiction(
        self, local_client
    ):
        """
        WHAT:   DPDPA requests are tagged with IN jurisdiction, not EU.
        GIVEN:  Entity registered with jurisdiction="IN" (India).
        EXPECT: Certification report shows DPDPA_SEC_12, not GDPR_ART_17.
                Deadline is still 30 days (DPDPA Section 12 requirement).
        """
        entity = await local_client.registry.register(
            gdpr_request_id="DPDPA-2026-00001",
            canonical_name="Priya Sharma",
            aliases=["priya@infosys.com"],
            role="Senior Engineer",
            organization="Infosys",
            jurisdiction="IN"
        )

        assert entity.jurisdiction == "IN"
        assert entity.applicable_law == "DPDPA_SEC_12"

        cert = await local_client.certifications.generate(
            "DPDPA-2026-00001",
            _simulate_day_30=True
        )

        report = await cert.get_json()
        assert "DPDPA_SEC_12" in report["applicable_law"]
        assert "GDPR_ART_17" not in report["applicable_law"]

    @pytest.mark.asyncio
    @pytest.mark.e2e
    async def test_dual_jurisdiction_entity_covered_by_both_laws(
        self, local_client
    ):
        """
        WHAT:   An entity with both EU and IN residence is covered under GDPR + DPDPA.
        GIVEN:  Entity with jurisdiction=["EU", "IN"].
        EXPECT: Certification cites both GDPR Art. 17 AND DPDPA Sec. 12.
        """
        entity = await local_client.registry.register(
            gdpr_request_id="DUAL-JUR-001",
            canonical_name="Rahul Nair",
            aliases=["rahul@eu-startup.com"],
            jurisdiction=["EU", "IN"]
        )

        cert = await local_client.certifications.generate(
            "DUAL-JUR-001", _simulate_day_30=True
        )
        report = await cert.get_json()

        assert "GDPR_ART_17" in report["applicable_laws"]
        assert "DPDPA_SEC_12" in report["applicable_laws"]
```

***

## E2E TESTS — `tests/e2e/test_data_subject_portal.py`

```python
"""
COMPONENT: Data Subject Verification Portal
PURPOSE:   Validates that the data subject (the person who requested erasure)
           can independently verify their request was processed.
"""
import pytest
from httpx import AsyncClient
from forgetmenot.api.main import app


class TestDataSubjectPortal:

    @pytest.mark.asyncio
    @pytest.mark.e2e
    async def test_valid_token_returns_verified_status(self, local_client):
        """
        WHAT:   A valid verification token returns VERIFIED status.
        GIVEN:  A completed GDPR erasure request with a verification token issued.
        WHEN:   Data subject visits verify portal with the token.
        EXPECT: HTTP 200. status=VERIFIED. Systems covered listed. Zero disclosures.
        """
        entity = await local_client.registry.register(
            gdpr_request_id="PORTAL-TEST-001",
            canonical_name="Jane Portal",
            aliases=["jane@portal.com"]
        )

        cert = await local_client.certifications.generate(
            "PORTAL-TEST-001", _simulate_day_30=True
        )
        token = await local_client.certifications.issue_verification_token(
            "PORTAL-TEST-001"
        )

        async with AsyncClient(app=app, base_url="http://test") as client:
            response = await client.get(
                f"/verify/{token.token_id}"
            )

        assert response.status_code == 200
        body = response.json()
        assert body["status"] == "VERIFIED"
        assert body["disclosure_events"] == 0
        assert len(body["systems_covered"]) >= 1
        assert "verification_hash" in body
        assert "Jane Portal" not in response.text    # No PII in portal response
        assert "jane@portal.com" not in response.text

    @pytest.mark.asyncio
    @pytest.mark.e2e
    async def test_invalid_token_returns_404_not_500(self):
        """
        WHAT:   An invalid/expired token returns 404, not an internal server error.
        INPUT:  Random UUID as token.
        EXPECT: HTTP 404. Body explains token is invalid. No stack trace exposed.
        """
        async with AsyncClient(app=app, base_url="http://test") as client:
            response = await client.get("/verify/invalid-token-xyz-123")

        assert response.status_code == 404
        assert "traceback" not in response.text.lower()
        assert "exception" not in response.text.lower()
        body = response.json()
        assert "message" in body

    @pytest.mark.asyncio
    @pytest.mark.e2e
    async def test_verification_hash_is_independently_verifiable(
        self, local_client
    ):
        """
        WHAT:   The verification hash can be independently verified using
                only ForgetMeNot's public key — no ForgetMeNot servers needed.
        GIVEN:  A verification response with a hash and signature.
        WHEN:   Verified offline using the public key from GitHub.
        EXPECT: Offline verification returns True.
        """
        entity = await local_client.registry.register(
            gdpr_request_id="PORTAL-HASH-001",
            canonical_name="Hash Test Person",
            aliases=["hash@test.com"]
        )

        await local_client.certifications.generate(
            "PORTAL-HASH-001", _simulate_day_30=True
        )
        token = await local_client.certifications.issue_verification_token(
            "PORTAL-HASH-001"
        )

        verification = await local_client.verification_portal.check(
            token.token_id
        )

        # Verify offline using public key only
        from forgetmenot.certification.signer import ReportSigner
        signer = ReportSigner()

        is_valid = signer.verify_hash_offline(
            hash_value=verification.verification_hash,
            signature=verification.signature,
            public_key=signer.get_public_key()
        )
        assert is_valid is True
```

***

## INTEGRATION TESTS — `tests/integration/test_sdk_python.py`

```python
"""
COMPONENT: Python SDK
PURPOSE:   End-to-end SDK integration — verifies the developer-facing
           interface matches the documented API exactly.
"""
import pytest
from forgetmenot import ForgetMeNotClient


class TestPythonSDKIntegration:

    @pytest.mark.asyncio
    async def test_sdk_wrap_openai_client_is_transparent(self):
        """
        WHAT:   Wrapping an OpenAI client produces a client with identical interface.
        GIVEN:  A mock OpenAI client.
        WHEN:   Wrapped with ForgetMeNot.
        EXPECT: The wrapped client has .chat.completions.create() method.
                Response has .choices[0].message.content (standard OpenAI shape).
                Response also has .forgetmenot metadata attribute.
        """
        from forgetmenot.integrations.openai_wrapper import wrap_openai
        import openai

        mock_openai = MagicMock(spec=openai.OpenAI)
        mock_openai.chat.completions.create.return_value = MagicMock(
            choices=[MagicMock(message=MagicMock(content="Paris is in France."))]
        )

        client = ForgetMeNotClient.local()
        wrapped = client.wrap(mock_openai)

        response = await wrapped.chat.completions.create(
            model="gpt-4o",
            messages=[{"role": "user", "content": "Where is Paris?"}]
        )

        assert response.choices[0].message.content == "Paris is in France."
        assert hasattr(response, "forgetmenot")
        assert response.forgetmenot.status == "PASS"

    @pytest.mark.asyncio
    async def test_sdk_compliance_budget_triggers_alert_on_high_block_rate(
        self, local_client
    ):
        """
        WHAT:   ComplianceBudget triggers alert when block rate exceeds threshold.
        GIVEN:  Budget with max_block_rate=0.1 (10%).
                60 inferences: 50 blocked, 10 clean (50% block rate — exceeds budget).
        EXPECT: alert_webhook called with BUDGET_EXCEEDED event.
        """
        from forgetmenot.sdk.budget import ComplianceBudget

        webhook_calls = []

        client = ForgetMeNotClient.local(
            compliance_budget=ComplianceBudget(
                max_block_rate=0.1,
                alert_webhook="https://test-webhook.example.com"
            )
        )

        entity = await client.registry.register(
            gdpr_request_id="BUDGET-TEST-001",
            canonical_name="Budget Test Person",
            aliases=["budget@test.com"]
        )

        with patch("forgetmenot.sdk.budget.call_webhook",
                   side_effect=lambda url, data: webhook_calls.append(data)):
            llm = client.wrap(MockLLMClient(output="Safe content."))
            for _ in range(10):
                await llm.chat("test")

            blocked_llm = client.wrap(
                MockLLMClient(output="Budget Test Person sent the email.")
            )
            for _ in range(50):
                await blocked_llm.chat("test")

        budget_alerts = [w for w in webhook_calls if w["event"] == "BUDGET_EXCEEDED"]
        assert len(budget_alerts) >= 1
        assert budget_alerts[0]["actual_block_rate"] > 0.1

    @pytest.mark.asyncio
    async def test_sdk_langchain_memory_integration(self, local_client):
        """
        WHAT:   ForgetMeNotMemory integrates seamlessly as LangChain memory.
        GIVEN:  A LangChain LLMChain using ForgetMeNotMemory.
        WHEN:   A conversation involves a registered entity.
        EXPECT: Entity's data is filtered from memory context injected into prompts.
        """
        from forgetmenot.integrations.langchain import ForgetMeNotMemory
        from langchain.chains import LLMChain
        from langchain.prompts import PromptTemplate

        entity = await local_client.registry.register(
            gdpr_request_id="LC-MEMORY-001",
            canonical_name="Memory Test Person",
            aliases=["memory@test.com"]
        )

        memory = ForgetMeNotMemory(
            client=local_client,
            session_id="session_lc_test"
        )

        # Inject a memory containing PII
        await memory._inject_for_testing(
            "The user is Memory Test Person and their email is memory@test.com"
        )

        # When memory is retrieved for context injection,
        # the PII should be filtered out
        context = await memory.load_memory_variables({})
        context_str = str(context)

        assert "Memory Test Person" not in context_str
        assert "memory@test.com" not in context_str
```

***

## INTEGRATION TESTS — `tests/integration/test_worm_ledger.py`

```python
"""
COMPONENT: WORM Ledger (AWS S3 Object Lock)
PURPOSE:   Validate that the production WORM storage actually prevents
           modification and deletion of ledger events.
NOTE:      These tests require AWS credentials and run against
           a dedicated test S3 bucket with Object Lock enabled.
           Marked @pytest.mark.aws — skipped in local dev, run in staging CI.
"""
import pytest
import boto3
from botocore.exceptions import ClientError


@pytest.mark.aws
class TestWORMLedgerStorage:

    @pytest.fixture
    def s3_client(self):
        return boto3.client("s3", region_name="eu-west-1")

    @pytest.fixture
    def worm_bucket(self):
        return "forgetmenot-ledger-worm-test"

    @pytest.mark.asyncio
    async def test_written_object_cannot_be_deleted(
        self, s3_client, worm_bucket, local_client
    ):
        """
        WHAT:   An object written to S3 WORM bucket cannot be deleted.
        GIVEN:  A ledger event written to WORM bucket.
        WHEN:   delete_object() attempted on the same key.
        EXPECT: ClientError with "Access Denied" or "ObjectLockConfigurationNotFoundError".
        """
        event_hash = await local_client.ledger.write(
            LedgerEvent(
                org_id="org_worm_test",
                event_type=LedgerEventType.INFERENCE_CLEAN,
                action_taken="PASS",
                inference_hash="worm_test_hash"
            )
        )
        object_key = f"ledger/org_worm_test/{event_hash}"

        with pytest.raises(ClientError) as exc:
            s3_client.delete_object(Bucket=worm_bucket, Key=object_key)

        error_code = exc.value.response["Error"]["Code"]
        assert error_code in [
            "AccessDenied",
            "InvalidRequest",
            "MethodNotAllowed"
        ], f"Expected deletion to fail but got error: {error_code}"

    @pytest.mark.asyncio
    async def test_written_object_cannot_be_overwritten(
        self, s3_client, worm_bucket, local_client
    ):
        """
        WHAT:   An object written to WORM bucket cannot be overwritten.
        GIVEN:  A ledger event at a known S3 key.
        WHEN:   put_object() attempted on the same key with different content.
        EXPECT: ClientError — COMPLIANCE mode prevents any overwrite.
        """
        event_hash = await local_client.ledger.write(
            LedgerEvent(
                org_id="org_worm_overwrite_test",
                event_type=LedgerEventType.INFERENCE_CLEAN,
                action_taken="PASS",
                inference_hash="overwrite_test"
            )
        )
        object_key = f"ledger/org_worm_overwrite_test/{event_hash}"

        with pytest.raises(ClientError):
            s3_client.put_object(
                Bucket=worm_bucket,
                Key=object_key,
                Body=b"TAMPERED CONTENT"
            )

    @pytest.mark.asyncio
    async def test_object_lock_mode_is_compliance_not_governance(
        self, s3_client, worm_bucket
    ):
        """
        WHAT:   Object Lock is in COMPLIANCE mode — not GOVERNANCE mode.
                GOVERNANCE mode allows deletion by privileged users.
                COMPLIANCE mode is immutable even to root/admin.
        GIVEN:  The WORM ledger bucket.
        EXPECT: Bucket default retention mode == "COMPLIANCE".
        CRITICAL: GOVERNANCE mode would be a legal liability.
        """
        response = s3_client.get_object_lock_configuration(Bucket=worm_bucket)
        mode = response["ObjectLockConfiguration"]["Rule"]["DefaultRetention"]["Mode"]

        assert mode == "COMPLIANCE", (
            f"Object Lock mode is '{mode}' — must be 'COMPLIANCE', not 'GOVERNANCE'. "
            "GOVERNANCE mode allows admins to delete audit logs, which invalidates "
            "the legal defensibility of the compliance record."
        )
```

***

## TEST RUNNER CONFIGURATION — `pyproject.toml`

```toml
[tool.pytest.ini_options]
asyncio_mode = "auto"
testpaths = ["tests"]
addopts = [
    "--tb=short",
    "--strict-markers",
    "-v",
    "--cov=forgetmenot",
    "--cov-report=term-missing",
    "--cov-report=html:coverage_report",
    "--cov-fail-under=90",          # Fail CI if coverage drops below 90%
]
markers = [
    "unit: Fast unit tests — run on every commit",
    "integration: Integration tests — run on every PR",
    "performance: Latency + throughput tests — run on every PR",
    "security: Security and isolation tests — run on every PR",
    "e2e: End-to-end lifecycle tests — run nightly",
    "aws: Requires live AWS credentials — run in staging only",
    "critical: Must never be skipped, xfailed, or disabled",
]
filterwarnings = [
    "error",          # All warnings are errors — no silent degradation
    "ignore::DeprecationWarning:httpx",
]

[tool.coverage.run]
omit = [
    "tests/*",
    "forgetmenot/testing/*",     # Test helpers don't count toward coverage
    "forgetmenot/migrations/*",
]
branch = true    # Branch coverage — not just line coverage

[tool.coverage.report]
exclude_lines = [
    "pragma: no cover",
    "if TYPE_CHECKING:",
    "raise NotImplementedError",
]
```

***

## CI PIPELINE — `.github/workflows/test.yml`

```yaml
name: Full Test Suite

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]

jobs:

  unit-tests:
    name: Unit Tests (Every Commit)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: astral-sh/setup-uv@v2
      - run: uv sync --dev
      - run: pytest tests/unit/ -m unit --tb=short -q
      - run: pytest tests/unit/ -m critical --tb=long   # Critical tests get full output

  integration-tests:
    name: Integration + Security Tests (Every PR)
    needs: unit-tests
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:16
        env:
          POSTGRES_PASSWORD: test
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
      redis:
        image: redis:7
        options: >-
          --health-cmd "redis-cli ping"
          --health-interval 10s
    steps:
      - uses: actions/checkout@v4
      - uses: astral-sh/setup-uv@v2
      - run: uv sync --dev
      - run: pytest tests/integration/ tests/security/ --tb=short -q
      - name: Upload Coverage
        uses: codecov/codecov-action@v4

  performance-tests:
    name: Latency Budget Tests (Every PR)
    needs: unit-tests
    runs-on: ubuntu-latest-gpu    # Needs GPU for accurate latency measurement
    steps:
      - uses: actions/checkout@v4
      - uses: astral-sh/setup-uv@v2
      - run: uv sync --dev
      - run: pytest tests/performance/ -m performance --tb=short -v
      - name: Post latency report to PR
        uses: forgetmenot/latency-report-action@v1

  e2e-tests:
    name: End-to-End Lifecycle Tests (Nightly)
    if: github.event_name == 'schedule' || github.ref == 'refs/heads/main'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: astral-sh/setup-uv@v2
      - run: uv sync --dev
      - run: pytest tests/e2e/ -m e2e --tb=long -v
      - name: Notify on failure
        if: failure()
        uses: slackapi/slack-github-action@v1
        with:
          payload: '{"text":"🚨 E2E tests failed on main — compliance lifecycle broken"}'

  compliance-preflight:
    name: ForgetMeNot Pre-Flight Check (Every PR)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run ForgetMeNot Pre-Flight
        uses: forgetmenot/preflight-action@v1
        with:
          api_key: ${{ secrets.FORGETMENOT_API_KEY }}
          config_path: ".forgetmenot/preflight.yaml"
          fail_on_severity: HIGH
```

***

## COMPLETE TEST INVENTORY

Every test, numbered, with its category, name, and what it guarantees:

| # | File | Test Name | Guarantees |
|---|---|---|---|
| 1 | test_registry | `test_register_minimal_entity_succeeds` | Registration works with minimal fields |
| 2 | test_registry | `test_register_full_entity_succeeds` | All fields stored, CEDM built |
| 3 | test_registry | `test_canonical_name_never_stored_in_plaintext` | **Zero plaintext PII in storage** |
| 4 | test_registry | `test_duplicate_gdpr_request_id_raises_error` | No double-processing |
| 5 | test_registry | `test_empty_canonical_name_raises_error` | Input validation works |
| 6 | test_registry | `test_get_entity_status_returns_correct_fields` | Status tracking correct |
| 7 | test_registry | `test_get_nonexistent_entity_raises_error` | Clean error handling |
| 8 | test_registry | `test_revoke_entity_marks_inactive` | Revocation auditable |
| 9 | test_registry | `test_list_entities_returns_ids_only_no_pii` | List endpoint is PII-safe |
| 10 | test_registry | `test_deadline_is_exactly_30_days` | Legal deadline correct |
| 11 | test_registry | `test_update_aliases_adds_without_removing` | Alias history preserved |
| 12 | test_cedm | `test_direct_name_mention_is_flagged` | NER catches direct mentions |
| 13 | test_cedm | `test_alias_email_mention_is_flagged` | Alias matching works |
| 14 | test_cedm | `test_role_only_mention_is_flagged` | Role embedding catches indirect refs |
| 15 | test_cedm | `test_cooccurrence_relational_mention_is_flagged` | Graph catches co-occurrence |
| 16 | test_cedm | `test_completely_unrelated_output_passes` | No false positives on clean content |
| 17 | test_cedm | `test_same_name_different_person_is_not_blocked` | **Core disambiguation works** |
| 18 | test_cedm | `test_same_name_registered_person_is_blocked` | Registered person always caught |
| 19 | test_cedm | `test_cedm_confidence_in_valid_range` | Confidence is always 0.0–1.0 |
| 20 | test_cedm | `test_empty_string_input_passes_safely` | No crash on empty input |
| 21 | test_cedm | `test_very_long_input_does_not_crash` | No crash on huge input |
| 22 | test_cedm | `test_cedm_scan_within_latency_budget` | p95 < 12ms |
| 23 | test_ensemble | `test_all_three_engines_pass` | Pass unanimous |
| 24 | test_ensemble | `test_all_three_engines_block` | Block unanimous |
| 25 | test_ensemble | `test_majority_two_block` | 2/3 majority triggers block |
| 26 | test_ensemble | `test_single_high_confidence_triggers_block` | >0.95 overrides majority |
| 27 | test_ensemble | `test_single_medium_confidence_returns_flag` | Medium hit returns FLAG |
| 28 | test_ensemble | `test_ner_failure_graceful_degradation` | Partial engine failure handled |
| 29 | test_ensemble | `test_all_engines_fail_conservative_block` | **Fail-secure: never defaults to PASS** |
| 30 | test_ensemble | `test_result_includes_per_engine_breakdown` | Audit trail has engine scores |
| 31 | test_firewall | `test_clean_output_passes_unchanged` | Clean content unmodified |
| 32 | test_firewall | `test_pii_output_is_redacted` | PII is removed from output |
| 33 | test_firewall | `test_no_token_released_before_scan` | **Race condition impossible** |
| 34 | test_firewall | `test_redact_policy_replaces_with_placeholder` | Redact policy works |
| 35 | test_firewall | `test_block_policy_returns_empty` | Block policy works |
| 36 | test_firewall | `test_every_inference_logged_to_ledger` | All events recorded |
| 37 | test_firewall | `test_firewall_adds_less_than_20ms_p95` | Latency budget met |
| 38 | test_firewall | `test_firewall_works_with_empty_registry` | No crash when no entities |
| 39 | test_firewall | `test_firewall_handles_llm_timeout_gracefully` | Timeout handled cleanly |
| 40 | test_ledger | `test_write_returns_event_hash` | Hash format correct |
| 41 | test_ledger | `test_first_event_has_genesis_prev_hash` | Chain starts correctly |
| 42 | test_ledger | `test_chain_hash_links_consecutive_events` | Chain links are valid |
| 43 | test_ledger | `test_tampered_event_breaks_chain` | **Tampering is detectable** |
| 44 | test_ledger | `test_deleted_event_breaks_chain` | **Deletion is detectable** |
| 45 | test_ledger | `test_ledger_write_is_nonblocking` | Write doesn't delay inference |
| 46 | test_ledger | `test_point_in_time_snapshot_correct` | Time Machine accuracy |
| 47 | test_ledger | `test_pii_never_stored_in_ledger` | Audit log is PII-safe |
| 48 | test_rag | `test_clean_documents_pass_through` | No false filtering |
| 49 | test_rag | `test_pii_document_filtered_from_results` | PII doc removed |
| 50 | test_rag | `test_rag_filter_event_logged` | Filter events in audit log |
| 51 | test_rag | `test_rag_adds_less_than_3ms` | RAG latency budget met |
| 52 | test_rag | `test_all_docs_pii_returns_empty_list` | All-PII retrieval handled |
| 53 | test_cert | `test_certification_generated_on_day_30` | Cert auto-issued |
| 54 | test_cert | `test_certification_contains_mandatory_fields` | All legal fields present |
| 55 | test_cert | `test_certification_is_cryptographically_signed` | Signature valid |
| 56 | test_cert | `test_certification_with_disclosure_events_fails` | Failed cert not issued |
| 57 | test_cert | `test_limitations_disclaimer_always_present` | **Disclaimer never omitted** |
| 58 | test_cert | `test_certification_contains_no_pii` | Report is regulator-safe |
| 59 | test_preflight | `test_full_name_instruction_flagged_high` | High-risk prompt caught |
| 60 | test_preflight | `test_salary_data_flagged_as_article_9` | Special categories caught |
| 61 | test_preflight | `test_clean_config_returns_low_risk` | Low risk not over-flagged |
| 62 | test_preflight | `test_recommended_tier_scales_with_users` | Tier recommendations correct |
| 63 | test_preflight | `test_includes_erasure_request_forecast` | Volume forecasting works |
| 64 | test_pii_scrubber | `test_email_scrubbed_from_log` | Emails removed from logs |
| 65 | test_pii_scrubber | `test_multiple_emails_all_scrubbed` | All emails, not just first |
| 66 | test_pii_scrubber | `test_json_body_pii_scrubbed` | JSON bodies scrubbed |
| 67 | test_pii_scrubber | `test_safe_content_not_scrubbed` | Non-PII preserved |
| 68 | test_pii_scrubber | `test_exception_message_pii_scrubbed` | Exceptions scrubbed |
| 69 | test_api_registry | `test_post_returns_201` | API contract correct |
| 70 | test_api_registry | `test_no_auth_returns_401` | Auth enforced |
| 71 | test_api_registry | `test_missing_field_returns_422` | Validation enforced |
| 72 | test_api_registry | `test_get_status_returns_200` | Status endpoint works |
| 73 | test_api_registry | `test_list_contains_no_pii` | List API is PII-safe |
| 74 | test_api_registry | `test_cross_tenant_access_denied` | **Tenant isolation enforced** |
| 75 | test_pii_never_logged | `test_entity_name_not_in_logs` | **Zero PII in application logs** |
| 76 | test_pii_never_logged | `test_pii_not_in_otel_spans` | **Zero PII in traces** |
| 77 | test_pii_never_logged | `test_exception_pii_scrubbed_before_sentry` | Sentry receives no PII |
| 78 | test_pii_never_logged | `test_request_body_pii_scrubbed_in_access_logs` | HTTP logs PII-safe |
| 79 | test_tenant_isolation | `test_org_a_ledger_invisible_to_org_b` | Ledger fully isolated |
| 80 | test_tenant_isolation | `test_org_a_registry_doesnt_affect_org_b` | Registry fully isolated |
| 81 | test_performance | `test_p50_under_15ms` | Median latency budget |
| 82 | test_performance | `test_p95_under_20ms` | 95th pct latency budget |
| 83 | test_performance | `test_p99_under_30ms` | 99th pct latency budget |
| 84 | test_performance | `test_100_concurrent_no_degradation` | Concurrency handled |
| 85 | test_performance | `test_large_registry_no_degradation` | Cache works at 10k entities |
| 86 | test_e2e_gdpr | `test_complete_lifecycle_request_to_certification` | **Full GDPR journey works** |
| 87 | test_e2e_gdpr | `test_deadline_missed_triggers_alert` | Missed deadline never silent |
| 88 | test_e2e_gdpr | `test_dispute_workflow_full_lifecycle` | Dispute + recalibration works |
| 89 | test_e2e_gdpr | `test_proactive_pii_discovery_surfaces_entities` | Discovery engine works |
| 90 | test_e2e_gdpr | `test_compliance_time_machine_accuracy` | Historical reconstruction correct |
| 91 | test_e2e_dpdpa | `test_dpdpa_uses_correct_jurisdiction` | DPDPA tag applied correctly |
| 92 | test_e2e_dpdpa | `test_dual_jurisdiction_covered_by_both_laws` | Both laws cited |
| 93 | test_portal | `test_valid_token_returns_verified` | Portal works for data subject |
| 94 | test_portal | `test_invalid_token_returns_404` | Portal fails gracefully |
| 95 | test_portal | `test_verification_hash_independently_verifiable` | Offline verification works |
| 96 | test_sdk | `test_wrap_openai_transparent_interface` | SDK zero-friction promise |
| 97 | test_sdk | `test_compliance_budget_alert_on_exceed` | Budget monitoring works |
| 98 | test_sdk | `test_langchain_memory_integration` | LangChain hook filters memory |
| 99 | test_worm | `test_written_object_cannot_be_deleted` | **S3 WORM immutability real** |
| 100 | test_worm | `test_written_object_cannot_be_overwritten` | **S3 WORM immutability real** |
| 101 | test_worm | `test_object_lock_mode_is_compliance_not_governance` | **Legal-grade lock enforced** |

***

## THE GOLDEN RULES FOR THIS TEST SUITE

These rules are non-negotiable. They apply to every test, every PR, every developer:

1. **Never skip a `@pytest.mark.critical` test.** Not for a deadline. Not for a demo. Not ever. If it's failing, fix the code — don't skip the test.

2. **The race condition test (`test_no_token_released_before_scan`) is the most important test in the codebase.** If this test fails, you have a GDPR violation in production. It gets reviewed by the CTO on every PR where it touches the firewall.

3. **Every test that touches PII must assert the PII is absent from the result.** It is not enough to assert that the result is correct — you must also assert the PII is gone.

4. **`test_all_engines_fail_conservative_block` defines the system's soul.** ForgetMeNot fails secure. When in doubt, block. A blocked output is a bad user experience. A leaked PII output is a €20 million fine.

5. **Performance tests are not optional.** A compliance tool that adds 500ms to every LLM call will be disabled by engineering teams and expose the company to the exact liability the product was bought to prevent. Latency regressions are compliance regressions.

The existing 101 tests are **solid on the happy path and the core compliance mechanics**. What's missing falls into 8 categories I'll address one by one:

1. Multilingual & Unicode PII
2. Adversarial Prompt Injection
3. Redaction Quality & Completeness
4. API Contract & Rate Limiting
5. Webhook & External Service Failure
6. Scanner Calibration & Drift
7. Operational Recovery
8. Billing & Tier Enforcement

***

## CATEGORY 1 — MULTILINGUAL & UNICODE PII

*The existing suite tests only English-language PII. GDPR covers all EU languages. DPDPA covers Hindi, Tamil, Telugu, and 21 other Indian languages. These are completely untested.*

***

**TC-U-102 | Registry | Unicode name stored and matched correctly**

- **GIVEN** an entity registration with a canonical name containing non-ASCII characters: `"François Müller"`
- **WHEN** `register()` is called with `canonical_name="François Müller"`
- **THEN** the entity is stored successfully with correct UTF-8 encoding, the alias hash is computed from the normalized Unicode form (NFC), and a subsequent scan of `"François Müller approved the budget"` returns `BLOCK`

***

**TC-U-103 | CEDM | Hindi-script name is detected in output**

- **GIVEN** an entity registered with `canonical_name="प्रिया शर्मा"` (Priya Sharma in Devanagari)
- **WHEN** the CEDM scans the output `"प्रिया शर्मा ने रिपोर्ट जमा की"` (Priya Sharma submitted the report)
- **THEN** scan result is `BLOCK` with `matched_via="name_embedding"` — Devanagari script is not treated as safe by default

***

**TC-U-104 | CEDM | Mixed-script output is detected**

- **GIVEN** an entity registered as `"Priya Sharma"` with alias `"priya@infosys.com"`
- **WHEN** an output mixes scripts: `"Contact प्रिया (priya@infosys.com) for approval"`
- **THEN** scan result is `BLOCK` — the email alias match catches it even if the Devanagari portion is missed

***

**TC-U-105 | PII Scrubber | Unicode emails are scrubbed from logs**

- **GIVEN** a log message containing a non-ASCII email: `"Processing request for tëst@example.com"`
- **WHEN** `PIIScrubber.scrub()` is called on the log message
- **THEN** `"tëst@example.com"` does not appear in the output and `[EMAIL_REDACTED]` appears in its place

***

**TC-U-106 | Registry | Right-to-left script name (Arabic) is handled without crash**

- **GIVEN** an entity with `canonical_name="أحمد العلي"` (Ahmad Al-Ali in Arabic)
- **WHEN** `register()` is called
- **THEN** registration succeeds, probe generation produces 10 valid probes, and the CEDM graph is built without exception

***

**TC-U-107 | Firewall | Output containing only emoji does not crash scanner**

- **GIVEN** a registered entity and an LLM output of `"😀🎉✅"` (emoji only)
- **WHEN** the firewall scans the output
- **THEN** result is `PASS`, no exception is raised, latency is within the 20ms p95 budget

***

**TC-U-108 | CEDM | Name with homoglyph attack is detected**

- **GIVEN** an entity registered as `"Alice Johnson"`
- **WHEN** an LLM output contains `"Аlice Johnson"` — where the `"А"` is a Cyrillic А (U+0410) that looks identical to Latin A
- **THEN** scan result is `BLOCK` — homoglyph substitution does not bypass the scanner
- **WHY THIS MATTERS** — adversarial actors may attempt homoglyph injection to leak PII past a naive string matcher

***

## CATEGORY 2 — ADVERSARIAL PROMPT INJECTION

*A malicious user could craft prompts designed to make the LLM output PII in disguise. None of these attack patterns are currently tested.*

***

**TC-U-109 | Firewall | Base64-encoded PII in output is detected**

- **GIVEN** an entity registered as `"Alice Johnson"` and an LLM output of `"The answer is: QWxpY2UgSm9obnNvbg=="` (which is base64 for "Alice Johnson")
- **WHEN** the firewall scans the output
- **THEN** scan result is `BLOCK` — the scanner decodes common encodings before scanning, or the CEDM's contextual model flags the suspicious encoding pattern

***

**TC-U-110 | Firewall | Character-spaced PII is detected**

- **GIVEN** an entity registered as `"Alice Johnson"`
- **WHEN** an LLM outputs `"A l i c e   J o h n s o n approved this"` (spaces between each character)
- **THEN** scan result is `BLOCK` — the scanner normalizes whitespace before NER matching

***

**TC-U-111 | Firewall | PII split across multiple sentences is detected**

- **GIVEN** an entity registered as `"Alice Johnson"` with role `"CFO"` at `"Acme Corp"`
- **WHEN** an LLM outputs `"The CFO's first name is Alice. Her last name is Johnson. She works at Acme Corp."`
- **THEN** scan result is `BLOCK` — the CEDM contextual model assembles the split reference correctly

***

**TC-U-112 | Firewall | Indirect pronoun chain leading to registered entity is flagged**

- **GIVEN** an entity registered as `"Alice Johnson"`, `"CFO"`, `"Acme Corp"`
- **WHEN** an LLM output reads: `"The CFO of Acme Corporation signed the document. She then forwarded it to her team. Her email is on file."`
- **THEN** scan result is at minimum `FLAG` — the pronoun "she" following a role+org match creates a high co-occurrence risk signal

***

**TC-U-113 | Firewall | JSON-structured PII embedded in output is detected**

- **GIVEN** an entity registered as `"Alice Johnson"` with alias `"alice@acme.com"`
- **WHEN** an LLM outputs a JSON block: `{"name": "Alice Johnson", "email": "alice@acme.com"}`
- **THEN** scan result is `BLOCK` — the scanner parses JSON content, not just raw string matching

***

**TC-U-114 | Firewall | Markdown-formatted PII in output is detected**

- **GIVEN** an entity registered as `"Alice Johnson"`
- **WHEN** an LLM outputs `"**Alice Johnson** approved the Q1 budget on [March 3rd](internal://approval/001)"`
- **THEN** scan result is `BLOCK` — the scanner strips markdown formatting before NER scanning

***

**TC-U-115 | Firewall | PII inside a code block in output is detected**

- **GIVEN** an entity registered as `"alice@acme.com"` as an alias
- **WHEN** an LLM outputs ` ```python\nemail = "alice@acme.com"\n``` `
- **THEN** scan result is `BLOCK` — code blocks are not treated as safe zones

***

## CATEGORY 3 — REDACTION QUALITY & COMPLETENESS

*The existing tests check that redaction happens, but not whether the redacted output is still coherent, useful, and complete. A redaction that destroys the entire response or leaks context through surrounding words is a product quality failure.*

***

**TC-U-116 | Redactor | Partial-sentence redaction preserves grammatical structure**

- **GIVEN** an LLM output: `"The report was submitted by Alice Johnson last Tuesday"`
- **WHEN** REDACT policy is applied
- **THEN** the returned string is grammatically complete, e.g. `"The report was submitted by [REDACTED] last Tuesday"` — not a broken or empty sentence

***

**TC-U-117 | Redactor | Multi-mention output redacts all occurrences not just the first**

- **GIVEN** an LLM output that mentions `"Alice Johnson"` three times across three sentences
- **WHEN** REDACT policy is applied
- **THEN** all three occurrences are replaced — the word `"Alice"` does not appear anywhere in the output

***

**TC-U-118 | Redactor | Redaction of email doesn't leave orphaned preposition**

- **GIVEN** an LLM output: `"Please send the document to alice@acme.com for review"`
- **WHEN** REDACT policy is applied
- **THEN** output reads naturally, e.g. `"Please send the document to [REDACTED] for review"` — not `"Please send the document to  for review"` (double space orphaned preposition)

***

**TC-U-119 | Redactor | Role-only redaction doesn't remove the role itself**

- **GIVEN** the CEDM detects a match via `role_embedding` (no name present — only role + org)
- **WHEN** REDACT policy is applied to `"The CFO of Acme Corp approved the budget"`
- **THEN** the system's behavior is deterministic and documented: either the sentence is fully redacted, or a configurable placeholder replaces the identifying combination — it does NOT silently pass the output unchanged

***

**TC-U-120 | Redactor | Redacted output is shorter than or equal to original length**

- **GIVEN** any output that triggers REDACT
- **WHEN** redaction is applied
- **THEN** `len(redacted_output) <= len(original_output)` — redaction never accidentally expands the output (e.g., by injecting extra content)

***

## CATEGORY 4 — API CONTRACT & RATE LIMITING

*The existing API tests cover happy path and basic auth. Rate limiting, pagination, and malformed content-type headers are completely untested.*

***

**TC-I-121 | API | Rate limit returns 429 after exceeding tier quota**

- **GIVEN** an organization on the Shield tier (1,000 scans/hour limit)
- **WHEN** 1,001 scan requests are made within one hour
- **THEN** the 1,001st request returns HTTP 429 with a `Retry-After` header specifying seconds until quota resets — the response never returns 500

***

**TC-I-122 | API | Rate limit headers present on every response**

- **GIVEN** any authenticated API call
- **WHEN** a valid response is returned (any 2xx)
- **THEN** response headers contain `X-RateLimit-Limit`, `X-RateLimit-Remaining`, and `X-RateLimit-Reset` — always, not just when limits are breached

***

**TC-I-123 | API | Pagination works correctly for large entity lists**

- **GIVEN** an organization with 500 registered entities
- **WHEN** `GET /v1/registry/entities?page=1&per_page=50` is called
- **THEN** exactly 50 entities returned, `X-Total-Count: 500` header present, `next` link in response points to page 2

***

**TC-I-124 | API | Malformed JSON body returns 400 not 500**

- **GIVEN** a POST request to `/v1/registry/entities` with body `"{invalid json"`
- **WHEN** the request is processed
- **THEN** HTTP 400 returned with a human-readable error message — never HTTP 500 or stack trace

***

**TC-I-125 | API | Wrong Content-Type header returns 415**

- **GIVEN** a POST request to `/v1/registry/entities` with `Content-Type: text/plain`
- **WHEN** the request is processed
- **THEN** HTTP 415 Unsupported Media Type is returned

***

**TC-I-126 | API | Extremely large request body returns 413 not crash**

- **GIVEN** a POST request with a 50MB JSON body sent to `/v1/registry/entities`
- **WHEN** the request hits the API gateway
- **THEN** HTTP 413 Request Entity Too Large returned before the body is parsed — no memory spike, no crash

***

**TC-I-127 | API | API key from one org cannot be used with a different org's X-Org-ID header**

- **GIVEN** API key `fnot_sk_org_a_key` which belongs to `org_a`
- **WHEN** a request is made with this key but with header `X-Org-ID: org_b`
- **THEN** HTTP 403 Forbidden — key/org mismatch is detected and rejected

***

**TC-I-128 | API | Expired API key returns 401 with clear expiry message**

- **GIVEN** an API key that expired 24 hours ago
- **WHEN** any authenticated request is made with this key
- **THEN** HTTP 401 with `"error": "api_key_expired"` and `"expired_at"` timestamp in response body

***

**TC-I-129 | API | SQL injection attempt in query parameter returns 400 not crash**

- **GIVEN** a GET request to `/v1/registry/entities?org_id=1; DROP TABLE entities;--`
- **WHEN** the request is processed
- **THEN** HTTP 400 returned. Database is unaffected. No SQL was executed. Attempt is logged to security audit trail.

***

## CATEGORY 5 — WEBHOOK & EXTERNAL SERVICE FAILURES

*The DPO webhook is the primary alerting mechanism. If it fails silently, the DPO never knows about a compliance event. This is completely untested.*

***

**TC-I-130 | Webhooks | Failed webhook delivery is retried with exponential backoff**

- **GIVEN** a DPO webhook endpoint that returns HTTP 500 on the first two attempts
- **WHEN** a GDPR_DEADLINE_BREACHED event fires
- **THEN** the system retries the webhook delivery: first retry after 30s, second after 60s, third after 120s — the DPO eventually receives the alert

***

**TC-I-131 | Webhooks | Webhook failure after max retries is logged to ledger not silently dropped**

- **GIVEN** a DPO webhook endpoint that returns HTTP 500 on all 5 retry attempts
- **WHEN** all retries are exhausted
- **THEN** a `WEBHOOK_DELIVERY_FAILED` event is written to the compliance ledger, the DPO dashboard shows a red "Alert Delivery Failed" banner, and an in-app notification is created

***

**TC-I-132 | Webhooks | Webhook payload contains no plaintext PII**

- **GIVEN** a GDPR_ERASURE_COMPLETED event fires for entity "Alice Johnson"
- **WHEN** the DPO webhook is called
- **THEN** the webhook payload body contains `entity_id` (hash) and `gdpr_request_id` — never `canonical_name`, `aliases`, or any raw PII

***

**TC-I-133 | Webhooks | Webhook delivery is idempotent**

- **GIVEN** a webhook endpoint that is called twice with the same event (due to a retry after a timeout)
- **WHEN** the endpoint processes both deliveries
- **THEN** the event has a unique `event_id` that the receiver can use to deduplicate — the same event is never processed twice

***

**TC-I-134 | External | Jira ticket creation failure doesn't block playbook generation**

- **GIVEN** the Jira integration is configured but Jira's API is returning HTTP 503
- **WHEN** a GDPR erasure request triggers Developer Playbook generation
- **THEN** the playbook is still generated and stored internally, the Jira failure is logged as a warning, and the playbook is shown in the ForgetMeNot dashboard even without a Jira ticket

***

**TC-I-135 | External | S3 WORM write failure triggers circuit breaker not silent data loss**

- **GIVEN** S3 is temporarily unavailable (simulated with a connection timeout)
- **WHEN** the ledger attempts to write an inference event
- **THEN** the circuit breaker opens after 3 consecutive S3 failures, all subsequent writes are queued in Redis, the DPO dashboard shows a "Ledger Degraded" warning, and queued events are flushed to S3 when connectivity is restored — zero events are lost

***

## CATEGORY 6 — SCANNER CALIBRATION & DRIFT

*The scanner gets calibrated over time by dispute approvals. None of the calibration persistence, rollback, or drift detection behaviors are tested.*

***

**TC-U-136 | Calibration | Approved dispute updates false positive filter within one hour**

- **GIVEN** a dispute that was approved at T=0 for a specific context pattern
- **WHEN** the same context pattern appears in a new inference at T=0+55min
- **THEN** the new inference result is `PASS` — the calibration was applied within the one-hour SLA

***

**TC-U-137 | Calibration | Rejected dispute reinforces the block — no recalibration**

- **GIVEN** a dispute that was rejected by the DPO
- **WHEN** the same output pattern appears again in a new inference
- **THEN** the new inference is still `BLOCK` — rejection does not change the scanner's behavior

***

**TC-U-138 | Calibration | Calibration model version is tracked in the ledger**

- **GIVEN** a calibration update applied at time T
- **WHEN** a scan event is logged after the calibration update
- **THEN** the ledger event includes `scanner_model_version` referencing the calibration version active at scan time — enabling historical reconstruction of which scanner version produced which decision

***

**TC-U-139 | Drift Detection | Alert fires when false positive rate increases > 2x week-over-week**

- **GIVEN** a baseline false positive rate of 2% in week 1
- **WHEN** false positive rate rises to 5% in week 2 (2.5× increase)
- **THEN** a `SCANNER_DRIFT_DETECTED` alert is fired to the DPO, with the week-over-week delta, top contributing patterns, and a recommended recalibration action

***

**TC-U-140 | Calibration | Rollback to previous scanner version is possible**

- **GIVEN** a calibration update that inadvertently increased the false negative rate
- **WHEN** the engineering team rolls back to the previous scanner model version via the admin API
- **THEN** all subsequent scans use the previous version, the rollback event is written to the ledger with the reason, and the DPO dashboard reflects the rollback

***

## CATEGORY 7 — OPERATIONAL RECOVERY

*What happens when ForgetMeNot itself goes down or restarts mid-operation? None of the crash recovery or restart-safe behaviors are tested.*

***

**TC-I-141 | Recovery | In-flight scan state is not lost on worker pod restart**

- **GIVEN** a scan is 50% complete (buffer filled, scan engine running) when the firewall worker pod is killed
- **WHEN** the pod restarts (Kubernetes restart policy)
- **THEN** the in-flight inference is treated as `BLOCK` (fail-secure), a `INFERENCE_INTERRUPTED` event is written to the ledger, and no partial output was ever delivered to the user

***

**TC-I-142 | Recovery | Redis cache rebuild on startup does not expose a window of unprotected scans**

- **GIVEN** Redis is cold (cache cleared) and the service restarts
- **WHEN** the first inference arrives before the registry cache is fully warmed
- **THEN** the system falls back to direct database lookup — the scan is NOT skipped or bypassed during cache warmup. Latency may increase (acceptable), but compliance is never compromised.

***

**TC-I-143 | Recovery | Partially written certification report is not issued**

- **GIVEN** a certification report PDF generation starts but the worker pod is killed after 50% of the file is written
- **WHEN** the pod restarts and re-attempts generation
- **THEN** the partial file is deleted, a fresh generation is started, and the certification status remains `GENERATING` — it is never marked `ISSUED` for a partial report

***

**TC-I-144 | Recovery | Duplicate ledger write after retry does not break chain integrity**

- **GIVEN** a ledger write succeeds in S3 but the acknowledgment is lost (network timeout), causing the writer to retry
- **WHEN** the duplicate write attempt arrives
- **THEN** the system detects the duplicate (via `event_id` idempotency key), discards the duplicate write, and the chain integrity remains valid — the same event is not hashed twice

***

**TC-I-145 | Recovery | Database connection pool exhaustion returns 503 not 500**

- **GIVEN** all PostgreSQL connection pool slots are occupied (simulated with 100 concurrent slow queries)
- **WHEN** a new API request arrives that requires a DB connection
- **THEN** HTTP 503 Service Unavailable is returned with a `Retry-After` header — not HTTP 500 with an unhandled exception

***

**TC-I-146 | Recovery | CEDM model file corruption is detected on startup not mid-request**

- **GIVEN** the CEDM model file has a corrupted checksum (simulated by modifying the file)
- **WHEN** the service starts
- **THEN** startup fails with a clear `MODEL_INTEGRITY_CHECK_FAILED` error — the service does NOT start and serve requests with a broken model, which would produce silently wrong scan results

***

## CATEGORY 8 — BILLING & TIER ENFORCEMENT

*Tier limits are a revenue-critical feature. If they're not enforced, every customer is effectively on the Enterprise tier for free.*

***

**TC-I-147 | Billing | Shield tier (10 entities max) rejects 11th entity registration**

- **GIVEN** an organization on the Shield tier with 10 already-registered entities (the maximum)
- **WHEN** an 11th `POST /v1/registry/entities` is attempted
- **THEN** HTTP 402 Payment Required is returned with `"error": "entity_limit_reached"`, `"current_count": 10`, `"limit": 10`, and a link to upgrade

***

**TC-I-148 | Billing | Scan volume soft limit triggers warning at 80% usage**

- **GIVEN** an organization on the Growth tier (100k scans/month limit) that has used 80,000 scans
- **WHEN** the 80,001st scan is processed
- **THEN** a `USAGE_WARNING_80_PCT` event fires, the DPO dashboard shows a yellow warning banner, and an email notification is sent to the org admin — no scans are blocked yet

***

**TC-I-149 | Billing | Scan volume hard limit blocks scans at 100% with graceful error**

- **GIVEN** an organization that has reached exactly its monthly scan limit
- **WHEN** the next inference scan is attempted
- **THEN** the firewall returns HTTP 402 with `"error": "scan_quota_exceeded"`, the LLM output is NOT released to the user (fail-secure), and the DPO dashboard shows a red "Quota Exceeded" alert with an upgrade CTA

***

**TC-I-150 | Billing | Downgrading tier revokes access to Enterprise features immediately**

- **GIVEN** an organization that downgrades from Enterprise to Growth
- **WHEN** the downgrade is processed
- **THEN** access to Enterprise-only features (e.g., on-premise Helm chart, CCDF export, multi-region ledger) is revoked within 60 seconds — no grace period for features not covered by the new tier

***

**TC-I-151 | Billing | Free trial expiry blocks API access gracefully**

- **GIVEN** an organization whose 14-day free trial expired at midnight
- **WHEN** an API request is made at 12:01 AM
- **THEN** HTTP 402 returned with `"error": "trial_expired"`, a payment setup link, and the organization's data is retained for 30 days (not immediately deleted)

***

## CATEGORY 9 — CERTIFICATION EDGE CASES

*The existing certification tests cover the happy path. These are the hard edge cases that will actually happen in production.*

***

**TC-U-152 | Certification | Certification requested before Day 30 returns PENDING**

- **GIVEN** a GDPR request registered 15 days ago (Day 15 of 30)
- **WHEN** `GET /v1/certifications/GDPR-001/status` is called
- **THEN** response contains `status: "MONITORING"`, `days_remaining: 15`, `probes_passed: 10`, and a projected certification date — not an error and not a premature certification

***

**TC-U-153 | Certification | Certification with zero inferences scanned is flagged as low confidence**

- **GIVEN** a GDPR erasure request where the connected LLM systems had zero inference volume in the 30-day window (e.g., a development system)
- **WHEN** certification is generated
- **THEN** `cert.confidence_level = "LOW"`, the report contains a mandatory warning: `"No inference activity detected — coverage cannot be demonstrated"`, and the DPO is alerted

***

**TC-U-154 | Certification | Certification covers only connected systems — uncovered systems explicitly listed**

- **GIVEN** an organization with 3 LLM systems but only 2 are connected to ForgetMeNot
- **WHEN** certification is generated
- **THEN** the report explicitly lists the 2 covered systems as `✅ COVERED` and the 1 unconnected system as `⚠️ NOT COVERED — manual review required`, and the certification explicitly states it does NOT cover the unconnected system

***

**TC-U-155 | Certification | Re-certification after a disclosure event requires DPO sign-off**

- **GIVEN** an entity that had a disclosure event, which was remediated (scanner recalibrated, data purged)
- **WHEN** the organization attempts to generate a new certification for the same GDPR request ID
- **THEN** the system requires explicit DPO sign-off before issuing the new certification, the previous disclosure event is included in the new certification's history section, and it cannot be hidden or removed

***

## CATEGORY 10 — PROBE SYSTEM QUALITY

*Auto-generated probes are the product's verification backbone. Their quality is completely untested.*

***

**TC-U-156 | Probe Generator | All 10 probe types are generated for a fully specified entity**

- **GIVEN** a fully specified entity with name, role, org, aliases, and related entities
- **WHEN** `ProbeGenerator.generate(entity)` is called
- **THEN** exactly 10 probes are returned covering: direct name, alias/email, role+org, department reference, related entity co-occurrence, temporal reference ("who left in 2025"), partial name, typo variant, reverse lookup ("who worked on Project X"), and indirect pronoun chain

***

**TC-U-157 | Probe Generator | Probes for minimal entity are still valid**

- **GIVEN** an entity registered with only `canonical_name` and one alias (no role, no org, no related entities)
- **WHEN** probes are generated
- **THEN** 10 probes are still generated (some will be simpler), none are empty strings, and none contain raw PII in the probe text itself

***

**TC-U-158 | Probe Generator | All 10 probes actually trigger a BLOCK when run against the scanner**

- **GIVEN** an entity and its 10 auto-generated probes
- **WHEN** each probe is passed through a test LLM that echoes the probe back as output, and the scanner scans the output
- **THEN** at least 9 out of 10 probes return `BLOCK` — probe quality threshold is 90% hit rate before certification can proceed

***

**TC-U-159 | Probe System | A probe that returns PASS is flagged as a scanner gap**

- **GIVEN** an entity and a probe that returns `PASS` (scanner misses it)
- **WHEN** the probe result is recorded
- **THEN** a `PROBE_MISSED` event is written to the ledger, the DPO dashboard shows the missed probe as a scanner gap, and the certification confidence score is downgraded

***

## COMPLETE SUPPLEMENTARY TEST INVENTORY

| # | Category | Test Name | What It Guarantees |
|---|---|---|---|
| TC-U-102 | Unicode | `test_unicode_name_stored_and_matched` | Non-ASCII names are detected |
| TC-U-103 | Unicode | `test_hindi_script_name_detected` | Devanagari PII is caught |
| TC-U-104 | Unicode | `test_mixed_script_output_detected` | Mixed script alias catches it |
| TC-U-105 | Unicode | `test_unicode_email_scrubbed_from_logs` | Non-ASCII emails scrubbed |
| TC-U-106 | Unicode | `test_arabic_name_registration_no_crash` | RTL scripts handled |
| TC-U-107 | Unicode | `test_emoji_only_output_no_crash` | Emoji doesn't break scanner |
| TC-U-108 | Unicode | `test_homoglyph_attack_detected` | Cyrillic lookalike blocked |
| TC-U-109 | Adversarial | `test_base64_encoded_pii_detected` | Encoding bypass fails |
| TC-U-110 | Adversarial | `test_character_spaced_pii_detected` | Space injection bypass fails |
| TC-U-111 | Adversarial | `test_pii_split_across_sentences_detected` | Split-mention bypass fails |
| TC-U-112 | Adversarial | `test_pronoun_chain_flagged` | Pronoun-only reference flagged |
| TC-U-113 | Adversarial | `test_json_structured_pii_detected` | JSON body PII caught |
| TC-U-114 | Adversarial | `test_markdown_formatted_pii_detected` | Markdown stripping works |
| TC-U-115 | Adversarial | `test_pii_in_code_block_detected` | Code blocks not safe zones |
| TC-U-116 | Redaction | `test_partial_sentence_redaction_grammatical` | Redacted output is readable |
| TC-U-117 | Redaction | `test_multi_mention_all_occurrences_redacted` | All instances removed |
| TC-U-118 | Redaction | `test_email_redaction_no_orphaned_preposition` | Redaction is clean |
| TC-U-119 | Redaction | `test_role_only_redaction_is_deterministic` | Role match behavior documented |
| TC-U-120 | Redaction | `test_redacted_output_not_longer_than_original` | No content expansion |
| TC-I-121 | API | `test_rate_limit_returns_429` | Quota enforcement works |
| TC-I-122 | API | `test_rate_limit_headers_on_every_response` | Headers always present |
| TC-I-123 | API | `test_pagination_works_for_large_entity_list` | Pagination correct |
| TC-I-124 | API | `test_malformed_json_returns_400` | No 500 on bad input |
| TC-I-125 | API | `test_wrong_content_type_returns_415` | Content type enforced |
| TC-I-126 | API | `test_oversized_body_returns_413` | Memory safety enforced |
| TC-I-127 | API | `test_key_org_mismatch_returns_403` | Cross-org key rejected |
| TC-I-128 | API | `test_expired_api_key_returns_401` | Expired keys rejected |
| TC-I-129 | API | `test_sql_injection_in_query_param_returns_400` | No SQL injection surface |
| TC-I-130 | Webhooks | `test_failed_webhook_retried_with_backoff` | DPO always notified |
| TC-I-131 | Webhooks | `test_webhook_failure_logged_to_ledger` | Delivery failure auditable |
| TC-I-132 | Webhooks | `test_webhook_payload_contains_no_pii` | Webhook is PII-safe |
| TC-I-133 | Webhooks | `test_webhook_delivery_is_idempotent` | No double-processing |
| TC-I-134 | External | `test_jira_failure_doesnt_block_playbook` | External deps isolated |
| TC-I-135 | External | `test_s3_failure_triggers_circuit_breaker` | No silent data loss |
| TC-U-136 | Calibration | `test_approved_dispute_updates_filter_within_1hr` | Recalibration SLA met |
| TC-U-137 | Calibration | `test_rejected_dispute_reinforces_block` | Rejection has no side effects |
| TC-U-138 | Calibration | `test_calibration_version_tracked_in_ledger` | Full audit trail for scanner |
| TC-U-139 | Drift | `test_drift_alert_fires_on_2x_increase` | Degradation detected |
| TC-U-140 | Calibration | `test_rollback_to_previous_scanner_version` | Rollback is safe |
| TC-I-141 | Recovery | `test_in_flight_scan_fail_secure_on_pod_restart` | Mid-scan crash = BLOCK |
| TC-I-142 | Recovery | `test_cold_redis_fallback_to_db` | Cache warmup safe |
| TC-I-143 | Recovery | `test_partial_cert_not_issued` | Partial cert impossible |
| TC-I-144 | Recovery | `test_duplicate_ledger_write_idempotent` | Chain stays valid on retry |
| TC-I-145 | Recovery | `test_db_pool_exhaustion_returns_503` | Clean degradation |
| TC-I-146 | Recovery | `test_corrupted_model_fails_at_startup` | No silent bad scans |
| TC-I-147 | Billing | `test_shield_tier_11th_entity_rejected` | Tier limits enforced |
| TC-I-148 | Billing | `test_scan_soft_limit_warning_at_80pct` | Early warning works |
| TC-I-149 | Billing | `test_scan_hard_limit_blocks_fail_secure` | Hard limit fail-secure |
| TC-I-150 | Billing | `test_tier_downgrade_revokes_features` | Downgrade is immediate |
| TC-I-151 | Billing | `test_trial_expiry_blocks_gracefully` | Trial expiry clean |
| TC-U-152 | Certification | `test_cert_before_day_30_returns_pending` | No premature cert |
| TC-U-153 | Certification | `test_cert_zero_inferences_low_confidence` | Zero-volume flagged |
| TC-U-154 | Certification | `test_cert_lists_uncovered_systems` | Coverage scope honest |
| TC-U-155 | Certification | `test_recert_after_disclosure_requires_dpo` | Re-cert auditable |
| TC-U-156 | Probes | `test_all_10_probe_types_generated` | Probe completeness |
| TC-U-157 | Probes | `test_minimal_entity_probes_valid` | Probes work on sparse data |
| TC-U-158 | Probes | `test_probes_achieve_90pct_hit_rate` | Probe quality threshold |
| TC-U-159 | Probes | `test_missed_probe_flagged_as_scanner_gap` | Gaps are visible |

***

## QA FINAL ASSESSMENT

The original 101 tests give you a **solid functional foundation** — the compliance mechanics work. These 58 additional tests close the gaps that would cause failures in the real world:

- **Tests 102–108** — your first EU customer in France or Germany will have data in French, German, and possibly Arabic. Without these, your first international GDPR audit will expose untested PII paths.
- **Tests 109–115** — a motivated adversary will try these bypass patterns within the first week of production. Base64 and character-spacing attacks are well-documented against NER systems.
- **Tests 130–135** — the DPO webhook is your compliance nervous system. If it fails silently, your customer's DPO is flying blind during an active regulatory investigation.
- **Tests 141–146** — these are the midnight scenarios. Pod crashes, cold caches, corrupted model files. Every one of these has caused a production incident at a company that thought they'd covered everything.
- **Test 146 specifically** is the one that will save you from the most embarrassing possible failure: a scanner that starts up and appears healthy but is silently returning wrong results because the model file was corrupted during a deploy.
