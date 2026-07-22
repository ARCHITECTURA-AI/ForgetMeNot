# **ForgetMeNot™ — TDD Specification & Test Plan**

## **Build the MVP test-first · Every feature defined by its tests before any code exists**

### **v1.0 · aligned to MVP PRD v1.0 / Technical Documentation v5.0**

---

> **How to read this.** Two layers throughout:

> * 🟢 **The idea** — what this test proves, in plain words.  
> * 🔵 **The test** — the actual specification: given / when / then, with concrete inputs and asserts.

> This document is the source of truth for *what correct looks like*. You write the tests from here **first** (they fail \= RED), then write the minimum code to pass (GREEN), then clean up (REFACTOR). If a behaviour isn't pinned by a test here, it isn't done.

---

# **PART 0 — THE TDD CONTRACT (READ ONCE, FOLLOW ALWAYS)**

## **0.1 The loop, every time**

🟢 For each feature: write a test that fails, write just enough code to pass it, tidy up, repeat. Never write code without a failing test asking for it.

🔵 **Red → Green → Refactor**, per behaviour:

1. **RED** — write the test from this doc. Run it. It must fail (and fail for the *right reason* — assertion, not a typo/import error).  
2. **GREEN** — write the *minimum* code to make it pass. No extra cleverness. Resist building tomorrow's feature.  
3. **REFACTOR** — improve names/structure with the test as a safety net. Tests stay green.  
4. **COMMIT** — one behaviour per commit, message names the test (e.g. `green: ledger rejects sequence gap`).

## **0.2 The rules that make TDD honest here**

🔵

* **No production line without a failing test first.** If you typed logic and no red test demanded it, delete it and write the test.  
* **Test behaviour, not implementation.** Assert on observable outcomes (the response, the ledger row, the exit code), not private function internals — so refactors don't break tests.  
* **One assert-concept per test.** A test named `blocks_registered_email` asserts that, not five other things.  
* **The test name is a sentence.** `rejects_sequence_gap`, `no_token_before_scan`, `cert_says_detected_not_absolute`. Reading the test list \= reading the spec.  
* **Arrange-Act-Assert** structure in every test, visibly separated.  
* **Fakes over network.** The LLM and (early) the model-svc are *faked* in unit tests; real integration is its own layer.  
* **A bug becomes a test.** Every bug found gets a failing test reproducing it *before* the fix. It never comes back.  
* **Safety-critical tests are sacred.** The seven invariant tests (Part 8\) can never be weakened or skipped to "make CI pass." If one is hard to satisfy, the *code* is wrong, not the test.

## **0.3 The test pyramid for this MVP**

🔵

       ╱ E2E (≈8) ╲          docker-compose up → real flow: register→scan→certify  
       ╱────────────╲         slow, run nightly \+ pre-release  
      ╱ Integration  ╲        agent↔model-svc↔db↔ledger, real SQLite, faked LLM  
     ╱   (≈40)        ╲       run on every PR  
    ╱──────────────────╲  
   ╱     Unit (≈180)    ╲     each function/struct, faked deps, milliseconds  
  ╱──────────────────────╲    run on every save  
 ──────────────────────────

**Rule of thumb:** if a behaviour *can* be a unit test, it is one. Integration tests prove the seams. E2E proves the story.

## **0.4 Coverage & gates**

🔵

* **Line coverage ≥ 90%** on `agent/src/ledger`, `agent/src/decide`, `agent/src/stream` (the safety-critical core). Lower elsewhere is acceptable; these are not.  
* **Every feature F1–F22 (from the PRD) maps to ≥1 test here** — Part 9 is the traceability matrix proving none are unpinned.  
* **The 7 invariants are release-blocking** — CI fails if any is red or missing.  
* **Mutation testing** (stretch) on the ledger module: introduce a deliberate bug, a test must catch it. If none do, the tests are too weak.

## **0.5 Test IDs**

🔵 Every test has an ID: `T-<area>-<n>`. Areas: `LED` ledger, `ING` ingress, `NRM` normaliser, `DET` detect, `DEC` decide, `REG` registry, `CER` cert, `STR` stream, `OBS` obs/scrub, `CLI` cli, `INT` integration, `E2E` end-to-end, `INV` invariant. IDs are referenced in the traceability matrix.

---

# **PART 1 — TEST FIXTURES & FAKES (BUILD THESE BEFORE ANY FEATURE TEST)**

🟢 Before testing features, we build the "fake" versions of slow/external things so tests are fast and predictable.

🔵 These are themselves built test-first (a fake with a test proving it behaves). Minimum fixtures:

### **1.1 `FakeLLM`**

🔵 A stand-in for OpenAI. Given a canned answer, it returns it (optionally token-by-token for streaming tests).

* **T-FIX-1** — `fake_llm_returns_canned_answer`: given `FakeLLM("Paris")`, calling it yields `"Paris"`.  
* **T-FIX-2** — `fake_llm_streams_tokens`: given `FakeLLM.streaming(["Pa","ris"])`, consuming the stream yields 2 chunks in order.  
* **T-FIX-3** — `fake_llm_can_delay`: `FakeLLM.with_delay(50ms)` lets us test timeouts deterministically.

### **1.2 `FakeScanner`**

🔵 A stand-in for the model-svc, so agent unit tests don't need Python running.

* **T-FIX-4** — `fake_scanner_returns_configured_verdict`: `FakeScanner(verdict=BLOCK)` returns BLOCK for any text.  
* **T-FIX-5** — `fake_scanner_can_be_broken`: `FakeScanner.broken()` always returns PASS — used to prove defense-in-depth/canary (it must NOT silently leak).

### **1.3 `TempLedger` / `TempDb`**

🔵 A fresh empty SQLite \+ append-only file per test, auto-deleted after.

* **T-FIX-6** — `temp_db_is_isolated`: two `TempDb` instances don't see each other's rows.

### **1.4 Registry test data builder**

🔵 `make_entity(name="John Smith", org="Acme", jurisdiction="IN", scope=["chat"])` → a valid registration payload.

* **T-FIX-7** — `entity_builder_hashes_name`: the builder's stored `name_hash != "John Smith"` and is stable for the same salt.

### **1.5 The "golden" PII corpus**

🔵 A small, version-controlled file of tricky cases that every detection test draws from. Built once, grows whenever a real miss is found.

* Plain: `john.smith@acme.com`, `+91 98765 43210`  
* Encoded: base64 of the email, hex, rot13  
* Split: `"jo"+"hn.sm"+"ith@ac"+"me.com"`  
* Unicode: homoglyph `jоhn` (Cyrillic о), zero-width-joined `j​o​h​n`  
* Indirect: `"the head of Acme's R&D division"`  
* Namesake (must NOT match): `"John Smith, a 19th-century botanist"`  
* **T-FIX-8** — `golden_corpus_loads`: the corpus file parses and has ≥1 case per category.

---

# **PART 2 — LAYER ⑥ LEDGER (BUILD THIS FIRST — IT'S THE PRODUCT)**

🟢 The unfakeable notebook. We test it first because everything else writes to it, and if it's wrong, nothing else matters. We prove: lines lock each other, numbers can't have gaps, nothing shows before a line is written, and tampering is always caught.

🔵 **Module: `agent/src/ledger`. Feature refs: F4, F5, F6.**

### **2.1 Event creation & hashing**

* **T-LED-1** — `event_hash_is_deterministic`: same event fields → same `event_hash`. (Arrange an event; compute hash twice; assert equal.)  
* **T-LED-2** — `event_hash_changes_if_any_field_changes`: flip one field → different hash.  
* **T-LED-3** — `event_contains_no_plaintext_pii`: given an output containing `john.smith@acme.com`, the resulting event's fields contain only the `inference_hash` (a SHA256), never the raw string. *(ties to invariant 4\)*  
* **T-LED-4** — `event_is_tagged_with_mode`: an event created in shadow has `fmn_mode="shadow"`; in enforcement, `"enforcement"`. *(F3)*  
* **T-LED-5** — `event_carries_lineage_tokens`: every event has non-empty `subject_token` and `session_token`. *(F6)*

### **2.2 The chain**

* **T-LED-6** — `first_event_has_genesis_prev_hash`: the very first event's `prev_event_hash` equals the defined genesis constant.  
* **T-LED-7** — `each_event_links_to_previous`: event N's `prev_event_hash ==` event N-1's `event_hash`.  
* **T-LED-8** — `chain_verifies_after_1000_events`: write 1000 events; `verify_chain()` returns valid.  
* **T-LED-9** — `tampering_breaks_chain_and_names_index`: write 1000; mutate event \#500 in storage; `verify_chain()` returns invalid AND reports `broken_at == 500`. *(F4 — the headline ledger test)*  
* **T-LED-10** — `reordering_breaks_chain`: swap two events' positions; verify fails.

### **2.3 Sequence continuity (catches silent deletion)**

* **T-LED-11** — `sequence_numbers_are_monotonic`: events get 1,2,3… per tenant.  
* **T-LED-12** — `rejects_sequence_gap`: if event seq 5 is missing (1,2,3,4,6), `verify_continuity()` fails and names the gap at 5\. *(F4, invariant 3\)*  
* **T-LED-13** — `sequence_is_per_tenant`: tenant A and tenant B each have independent 1,2,3… *(F7)*

### **2.4 Write-before-release & durability**

* **T-LED-14** — `write_is_durable_before_return`: `write(event)` does not return success until the event is flushed to both the append-only file and the DB mirror. (Kill the process right after return in a test harness; reopen; event is present.)  
* **T-LED-15** — `failed_write_returns_error_not_silent_pass`: if the storage write fails (inject an IO error), `write()` returns `Err`, never `Ok`. *(this is what lets the caller fail-closed)*

### **2.5 WORM behaviour**

* **T-LED-16** — `ledger_is_append_only_through_api`: there is no public method to update or delete an existing event. (Compile-time/contract test: the writer exposes only `write` \+ `read` \+ `verify`.)  
* **T-LED-17** — `direct_edit_is_detected_by_verify`: simulate an out-of-band file edit; `verify_chain()` catches it. *(F5)*

> **GREEN order for Part 2:** T-LED-1 → 2 → 6 → 7 → 11 → 14 → then the failure cases 9, 12, 15, 17\. Don't move on until T-LED-9 and T-LED-12 are green — they are the product's spine.

---

# **PART 3 — LAYER ① INGRESS (THE BYPASS-AND-CONFIG WALL)**

🟢 The front door. We prove: you can't get in without our key, you can't reach the AI directly, the mode switch works and is visible, and one customer can't see another's stuff.

🔵 **Module: `agent/src/ingress`. Feature refs: F1, F2, F3, F7, F8.**

### **3.1 Auth & keys-in-proxy-only**

* **T-ING-1** — `rejects_request_without_fmn_key`: a request with no/invalid `Authorization` → 401, and the upstream LLM is never called.  
* **T-ING-2** — `accepts_valid_fmn_key`: a valid FMN key → request proceeds.  
* **T-ING-3** — `real_provider_key_never_in_response_or_logs`: the upstream key is only read from the vault inside the agent; it never appears in any response body, error, or log line. *(F1, invariant 1\)*  
* **T-ING-4** — `app_uses_fmn_key_not_provider_key`: a request authenticated with a raw `sk-...` style key is treated as an FMN key lookup and fails unless registered — proving app code can't smuggle the provider key through.

### **3.2 OpenAI-compatible proxy**

* **T-ING-5** — `proxies_chat_completions_shape`: a standard OpenAI `/v1/chat/completions` request returns an OpenAI-shaped response. *(F8)*  
* **T-ING-6** — `unmodified_openai_client_works_via_base_url`: *(integration)* pointing a real OpenAI client at `:8787/v1` returns a completion. *(F8)*  
* **T-ING-7** — `passes_through_when_no_registry_match`: with empty registry, the answer is unchanged (but still ledgered).

### **3.3 Mode-state engine**

* **T-ING-8** — `default_mode_is_shadow`: a fresh install starts in shadow (safe by default). *(F3)*  
* **T-ING-9** — `shadow_mode_does_not_alter_output`: in shadow, a would-be-blocked answer is returned unchanged, but the ledger event records `action=BLOCK` with `fmn_mode=shadow` ("would have blocked"). *(F3)*  
* **T-ING-10** — `enforcement_mode_alters_output`: same input in enforcement → output redacted. *(F3)*  
* **T-ING-11** — `mode_is_visible_in_response_metadata`: every response carries the current mode. *(F3)*  
* **T-ING-12** — `entering_shadow_on_production_tenant_logs_warning`: flipping a "production"-flagged tenant to shadow emits a warning event. *(F3)*  
* **T-ING-13** — `mode_transition_is_a_ledger_event`: `mode set` writes a ledger entry. *(F3)*

### **3.4 Reconciliation (bypass detection)**

* **T-ING-14** — `proxied_calls_are_counted`: each proxied call increments `fmn_proxied_calls`. *(F2)*  
* **T-ING-15** — `reconcile_reports_delta`: given upstream count 100 and proxied count 95, the reconcile report flags a delta of 5 (possible bypass). *(F2)*

### **3.5 Tenant isolation**

* **T-ING-16** — `tenant_id_in_every_storage_key`: writes for tenant A are keyed with A; a query filtered by tenant B returns none of A's rows. *(F7)*  
* **T-ING-17** — `cross_tenant_read_returns_nothing`: *(integration)* registry/ledger reads for tenant B never surface tenant A's entities or events. *(F7)*

---

# **PART 4 — LAYER ③ OUTPUT NORMALISER (THE EVASION WALL)**

🟢 Un-disguise sneaky text before checking. We prove each trick (Base64, hex, invisible characters, look-alikes, split strings) is undone, and that we check both the original and the cleaned version.

🔵 **Module: `agent/src/normaliser`. Feature ref: F9. Draws from the golden corpus (T-FIX-8).**

### **4.1 Decoding**

* **T-NRM-1** — `decodes_base64_then_exposes_email`: input containing base64 of `john.smith@acme.com` → after normalise, the plaintext email is present in the normalised representation.  
* **T-NRM-2** — `decodes_hex`: same for hex.  
* **T-NRM-3** — `decodes_rot13`: same for rot13.  
* **T-NRM-4** — `ignores_non_encoded_text`: ordinary text passes through unchanged (no false decode).

### **4.2 Unicode**

* **T-NRM-5** — `folds_homoglyphs_to_canonical`: `jоhn` (Cyrillic о) → `john`.  
* **T-NRM-6** — `strips_zero_width_chars`: `j‌o‌h‌n` (zero-width joiners) → `john`.

### **4.3 Span-joining**

* **T-NRM-7** — `reassembles_split_string`: `"jo"+"hn.sm"+"ith@ac"+"me.com"` (across markup/whitespace) is reassembled so the email is detectable.  
* **T-NRM-8** — `reassembly_does_not_create_false_emails`: ordinary adjacent words don't get glued into a fake match.

### **4.4 Dual-representation contract**

* **T-NRM-9** — `scans_both_raw_and_normalised`: the normaliser returns *both* the raw and normalised text, and the detect layer is called on both. (Assert the scanner received 2 representations.)  
* **T-NRM-10** — `modality_gate_refuses_non_text`: a non-text payload (e.g. an image part) is refused-and-logged, never passed blind. *(B2, MVP scope)*

---

# **PART 5 — LAYER ④/⑤ DETECT & DECIDE (THE BACKSTOP \+ CORRECTNESS)**

🟢 The checkers and the judge. We prove the three checkers find real PII, the vote works, two John Smiths are told apart, a broken checker can't silently pass, and when in doubt we block.

🔵 **Modules: `agent/src/detect`, `agent/src/decide`, `model-svc`. Feature refs: F10, F11, F12, F13, F16.**

### **5.1 Detection engines (model-svc, Python, pytest)**

* **T-DET-1** — `ner_finds_plain_name`: `/scan` on "John Smith leads Acme" flags the name span.  
* **T-DET-2** — `pattern_finds_email`: regex engine flags `john.smith@acme.com`.  
* **T-DET-3** — `pattern_finds_phone`: flags `+91 98765 43210`.  
* **T-DET-4** — `pattern_finds_hr_id`: flags an employee-ID format.  
* **T-DET-5** — `semantic_finds_indirect_reference`: "the head of Acme's R\&D division" scores high similarity to the registered role embedding even with no literal name.  
* **T-DET-6** — `engines_return_confidence_scores`: each engine returns a 0–1 confidence, not just a boolean.

### **5.2 Ensemble vote**

* **T-DET-7** — `high_confidence_single_hit_blocks`: any engine \> 0.95 → BLOCK.  
* **T-DET-8** — `two_medium_hits_block`: two engines \> 0.70 → BLOCK.  
* **T-DET-9** — `single_medium_hit_flags_or_passes_per_policy`: one engine at 0.72, policy `single_hit=FLAG` → FLAG.  
* **T-DET-10** — `no_hits_pass`: all engines low → PASS.

### **5.3 CEDM-lite disambiguation (the false-positive fix)**

* **T-DET-11** — `blocks_the_registered_john_smith`: "The CFO of Acme Corp, John Smith, filed the report" with registered (John Smith, Acme, CFO) → BLOCK. *(F11)*  
* **T-DET-12** — `spares_the_unrelated_john_smith`: "John Smith, a 19th-century botanist" with the same registry → PASS. *(F11 — the two-John-Smiths test; prevents B4 over-blocking & B7)*  
* **T-DET-13** — `uses_org_and_time_context`: disambiguation flips on differing org/employment-period context.

### **5.4 Detector integrity / canary**

* **T-DET-14** — `canary_must_trip_on_startup`: a known planted-PII probe run at startup is detected; if detected, startup proceeds. *(F16)*  
* **T-DET-15** — `broken_detector_fails_closed`: with `FakeScanner.broken()` (always PASS), the canary check fails → the agent refuses to serve / blocks all, and pages. It does NOT silently pass. *(F16, invariant 6\)*

### **5.5 Decision & fail-closed**

* **T-DEC-1** — `block_decision_redacts_or_blocks_output`: a BLOCK verdict yields a sanitised response, raw PII absent. *(F12)*  
* **T-DEC-2** — `redaction_uses_neutral_label`: redacted span shows `[Content removed per privacy policy]`, never the entity ID or the reason. *(F13, \#38)*  
* **T-DEC-3** — `scan_timeout_blocks_not_leaks`: if the scan exceeds the SLA window (use `FakeScanner` with delay), the decision is BLOCK and no raw text is returned. *(F12, fail-closed)*  
* **T-DEC-4** — `scanner_error_blocks_not_leaks`: if the scanner errors, decision is BLOCK. *(fail-closed)*  
* **T-DEC-5** — `defense_in_depth_catches_misconfigured_scanner`: even if the scanner wrongly returns PASS on registered PII, a final string-level safety net prevents the registered email/name from appearing in the released output. *(invariant 2's cousin; the "most important test")*

### **5.6 Session information budget (basic, MVP)**

* **T-DEC-6** — `accumulated_medium_hits_escalate`: N medium-confidence contextual hits about one entity across a session → escalate to BLOCK even though each alone would PASS. *(F-budget, \#19)*

---

# **PART 6 — LAYER ⑤/⑥ STREAM (BUFFER-RELEASE — THE NO-RACE GUARANTEE)**

🟢 The most safety-critical timing rule: hold the answer, check it, write the notebook, *then* release. Never a word before the check.

🔵 **Module: `agent/src/stream`. Feature ref: F14.**

* **T-STR-1** — `no_token_released_before_scan_completes`: with a scan that takes 12ms, assert that at 8ms (mid-scan) zero tokens have reached the client; after the scan, tokens flow. *(F14, invariant 2 — the headline stream test)*  
* **T-STR-2** — `no_token_released_before_ledger_write`: tokens are withheld until the ledger event is durably written (for matched content). *(F14 \+ F4)*  
* **T-STR-3** — `clean_answer_releases_in_full`: a PASS answer is delivered identically to the source.  
* **T-STR-4** — `blocked_answer_never_emits_raw_tokens`: for a BLOCK, the raw buffered tokens are never yielded — only the sanitised form.  
* **T-STR-5** — `fail_closed_on_scan_failure_mid_stream`: if the scan fails while buffering, nothing buffered is released; a block/fallback is sent.

---

# **PART 7 — REGISTRY, CERT, OBS, CLI**

## **7.1 Registry (guided, hashed)**

🟢 Adding a person insists on the important details and never stores the real name.

🔵 **Module: `agent/src/registry` \+ model-svc probes. Feature ref: F15.**

* **T-REG-1** — `registration_requires_jurisdiction`: adding without `jurisdiction` → error. *(F15, \#39)*  
* **T-REG-2** — `registration_requires_scope`: adding without `scope` → error. *(F15, \#7)*  
* **T-REG-3** — `name_is_stored_hashed_only`: after add, the DB contains no plaintext name/email; only hashes/embeddings. *(F15, F21)*  
* **T-REG-4** — `registration_is_append_only_versioned`: a "correction" creates version 2, leaving version 1 intact. *(\#49)*  
* **T-REG-5** — `auto_probes_generated`: registering an entity yields 10 probes. *(F15)*  
* **T-REG-6** — `low_probe_pass_rate_flags_underspecified`: if the freshly-registered entity fails most probes, it's flagged as under-specified. *(\#5, B7)*  
* **T-REG-7** — `synchronous_activation`: the firewall rule is active before the registration call returns success. *(\#15)*  
* **T-REG-8** — `requester_identity_required_flag_recorded`: `requester_identity_verified` is stored and surfaced on the cert. *(B10)*  
* **T-REG-9** — `registry_list_shows_ids_not_names`: `registry list` output contains entity IDs, never names. *(F21)*

## **7.2 Certificate (the product artifact)**

🟢 The signed proof. It must tell the truth ("0 *detected*"), be generated from the real ledger, and be impossible to fake.

🔵 **Module: `cert`. Feature ref: F17.**

* **T-CER-1** — `cert_generated_from_ledger_not_handwritten`: the cert's numbers (events scanned, matches) equal a query over the ledger; mutating the ledger changes the cert. *(F17, \#21)*  
* **T-CER-2** — `cert_says_detected_not_absolute`: the cert text contains "0 **detected** disclosures" (or the detected count) and never a bare "0 disclosures". *(F17, invariant 7, B1)*  
* **T-CER-3** — `cert_shows_false_negative_rate`: the cert includes an FN-rate figure with a methodology reference. *(F17, B1)*  
* **T-CER-4** — `cert_discloses_shadow_windows`: if any part of the period was in shadow, the cert states it; it is not silently included as enforced. *(F17, \#31, B8)*  
* **T-CER-5** — `cert_states_modality_coverage`: the cert lists text as covered and image/audio as out-of-scope. *(B2)*  
* **T-CER-6** — `cert_is_signed`: the output PDF/JSON carries an Ed25519 signature. *(F17)*  
* **T-CER-7** — `cert_signature_verifies`: `verify(cert)` returns valid for an untampered cert. *(F17)*  
* **T-CER-8** — `tampered_cert_fails_verification`: change one byte of the cert body → `verify` returns invalid. *(F17)*  
* **T-CER-9** — `cert_binds_correct_jurisdiction`: an IN request produces a DPDPA-template cert; an EU request a GDPR one; mismatch is impossible. *(\#39)*  
* **T-CER-10** — `cert_includes_legal_characterisation`: the cert contains the "evidence of diligence, not guarantee, not legal advice" statement. *(B5 posture)*

## **7.3 Observability & scrubbing**

🟢 Nothing private ever lands in the logs.

🔵 **Module: `agent/src/obs`. Feature refs: F18, F21.**

* **T-OBS-1** — `logs_contain_no_raw_pii`: emit a log line about an event involving `john.smith@acme.com`; assert the written log contains only a hash, never the email. *(F18, invariant 4\)*  
* **T-OBS-2** — `scrubber_handles_emails_names_phones`: each PII type is scrubbed.  
* **T-OBS-3** — `metrics_have_no_pii`: exported metrics carry hashed IDs only. *(F21)*  
* **T-OBS-4** — `block_logged_at_info_not_error`: a BLOCK is logged at info/success level, never error/critical. *(invariant 5\)*

## **7.4 CLI**

🟢 The commands a person types actually do the thing.

🔵 **Module: `agent/src/cli`. Feature refs: F3, F15, F17.**

* **T-CLI-1** — `init_creates_db_and_config`: `forgetmenot init` creates `fmn.db` \+ config; rerun is idempotent.  
* **T-CLI-2** — `registry_add_round_trips`: `registry add …` then `registry list` shows the new entity ID.  
* **T-CLI-3** — `mode_set_changes_mode`: `mode set enforcement` is reflected in the next response's metadata.  
* **T-CLI-4** — `ledger_export_produces_signed_pdf`: `ledger export --out cert.pdf` writes a file that `verify` accepts. *(F17)*  
* **T-CLI-5** — `preflight_returns_risk_report`: `preflight --config x.yaml` lists Art.17/DPDPA risks. *(F20)*

---

# **PART 8 — THE SEVEN INVARIANTS (RELEASE-BLOCKING; NEVER WEAKEN)**

🟢 Seven promises. Each is a test (or small suite) that CI runs on every change. If any is red, nothing ships. These are the soul of the product — they can never be skipped to make CI green.

🔵 **These map to PRD Part 9\. Each has a dedicated test that must stay green.**

| Inv | The promise | Test ID(s) | What it asserts |
| ----- | ----- | ----- | ----- |
| **INV-1** | No raw provider key outside the proxy | T-INV-1 (+ T-ING-3) | Static scan of SDK/examples/app for `sk-` patterns finds none; the key lives only in the agent vault. |
| **INV-2** | No disclosure before scan \+ ledger commit | T-INV-2 (+ T-STR-1, T-STR-2) | A timing test proves zero tokens leave before scan completes AND the (matched-content) ledger event is durable. |
| **INV-3** | No unwritten scan | T-INV-3 (+ T-LED-12) | `inferences_served == ledger_events` per tenant; any sequence gap fails the build. |
| **INV-4** | No plaintext PII anywhere | T-INV-4 (+ T-LED-3, T-OBS-1, T-REG-3) | Scans DB rows, all log files, error outputs, caches; only hashes/embeddings permitted. |
| **INV-5** | A block is never an error | T-INV-5 (+ T-OBS-4) | Type-level: `Decision::Block` cannot be constructed into an error path; logging asserts info level. |
| **INV-6** | Detector must trip its canary | T-INV-6 (+ T-DET-15) | A broken detector → fail-closed; a clean canary pass is impossible to ignore. |
| **INV-7** | "0 disclosures" needs "detected" \+ FN-rate | T-INV-7 (+ T-CER-2, T-CER-3) | Cert template lint: the string "0 disclosures" without "detected" and an FN-rate fails. |

> **CI rule:** these seven run on *every* PR, isolated, and are marked `@critical`. A PR that makes any of them red or that deletes/skips one is auto-rejected. There is no "temporarily disable" path.

---

# **PART 9 — TRACEABILITY MATRIX (PROOF EVERY FEATURE IS PINNED BY TESTS)**

🟢 A table proving every MVP feature has tests guarding it. If a feature has no test, it's not built.

🔵 Feature IDs are from MVP PRD Part 3\.

| Feature | What it is | Tests pinning it |
| ----- | ----- | ----- |
| F1 keys-in-proxy-only | App can't hold the real key | T-ING-1,2,3,4 · T-INV-1 |
| F2 reconciliation counter | Detect bypass | T-ING-14,15 |
| F3 mode-state engine | Shadow/Enforcement | T-ING-8..13 · T-LED-4 · T-CLI-3 |
| F4 ledger chain+seq+wbr | The notebook | T-LED-1..15 · T-INV-2,3 |
| F5 WORM storage | Append-only/immutable | T-LED-16,17 |
| F6 lineage tagging | subject/session tokens | T-LED-5 |
| F7 tenant isolation | No cross-tenant bleed | T-ING-16,17 · T-LED-13 |
| F8 OpenAI-compatible endpoint | One-URL switch | T-ING-5,6,7 |
| F9 output normaliser | Un-disguise text | T-NRM-1..10 |
| F10 detection ensemble | NER+pattern+semantic vote | T-DET-1..10 |
| F11 CEDM-lite | Disambiguation | T-DET-11,12,13 |
| F12 decision \+ fail-closed | Allow/redact/block, block-on-fail | T-DEC-1,3,4,5 |
| F13 neutral redaction | Generic label | T-DEC-2 |
| F14 buffer-release | No-race streaming | T-STR-1..5 · T-INV-2 |
| F15 guided registry | Insists on detail, hashes | T-REG-1..9 |
| F16 detector canary | Broken detector → fail-closed | T-DET-14,15 · T-INV-6 |
| F17 signed certificate | "0 detected" proof | T-CER-1..10 · T-CLI-4 · T-INV-7 |
| F18 log-scrubbing | No PII in logs | T-OBS-1,2 · T-INV-4 |
| F19 dashboard | Live counts \+ mode banner | T-E2E-7 (below) |
| F20 gap scanner | Lead magnet | T-CLI-5 |
| F21 zero plaintext PII | Everywhere | T-LED-3 · T-REG-3 · T-OBS-1,3 · T-INV-4 |
| F22 the 7 invariants | Release gates | T-INV-1..7 |

**Every F maps to ≥1 test. No feature is unpinned.**

---

# **PART 10 — INTEGRATION & END-TO-END TESTS (THE SEAMS & THE STORY)**

🟢 Now we test the pieces working together, and finally the whole journey a real customer takes.

🔵 **Integration (real SQLite \+ real model-svc, faked LLM):**

* **T-INT-1** — `agent_calls_model_svc_and_blocks_registered_email`: register an entity; send a prompt whose faked LLM answer contains the email; in enforcement, the response is redacted and a `REDACTED` ledger event exists.  
* **T-INT-2** — `shadow_logs_would_have_blocked_without_altering`: same as above in shadow → answer unchanged, ledger event `action=BLOCK, mode=shadow`.  
* **T-INT-3** — `normaliser_plus_detect_catches_base64_email`: a base64-encoded registered email in the answer is caught end-to-end.  
* **T-INT-4** — `ledger_grows_one_event_per_inference`: 10 inferences → exactly 10 new ledger events (continuity holds). *(INV-3)*  
* **T-INT-5** — `cert_export_reflects_ledger_state`: after a known set of events, the exported cert's counts match the ledger exactly.  
* **T-INT-6** — `cross_tenant_isolation_under_load`: interleaved requests from tenant A and B never cross. *(F7)*

🔵 **End-to-end (docker-compose up, real flow):**

* **T-E2E-1** — `quickstart_demo_runs_green`: `python examples/quickstart.py` prints the expected PASS then REDACTED lines and `detected disclosures: 0`.  
* **T-E2E-2** — `unmodified_openai_client_protected_by_base_url`: a real OpenAI SDK pointed at the proxy gets a redacted answer for a registered entity.  
* **T-E2E-3** — `register_scan_certify_full_loop`: register → run inferences → `ledger export` → `verify` passes. *(the whole product in one test)*  
* **T-E2E-4** — `enforcement_blocks_real_flow` / **T-E2E-5** — `shadow_observes_real_flow`.  
* **T-E2E-6** — `chain_integrity_holds_after_real_run`: after the E2E run, `/v1/ledger/integrity` reports verified \+ no gaps.  
* **T-E2E-7** — `dashboard_shows_live_counts_and_mode_banner`: the page at `:8788` reflects the run and shows the loud mode banner. *(F19)*  
* **T-E2E-8** — `india_first_customer_can_certify`: a DPDPA-jurisdiction entity produces a valid FNOT-IN cert. *(acceptance)*

---

# **PART 11 — THE BUILD-IN-ORDER PLAYBOOK (TDD WEEK BY WEEK)**

🟢 The exact order to write tests and code, so the walls come first and every week ends green.

🔵 Each week: **write the listed tests (RED) → code to green → refactor.** Don't start a week until the prior week's tests are green.

| Wk | Write these tests first (RED) | Then code to GREEN | Week is done when |
| ----- | ----- | ----- | ----- |
| 1 | T-FIX-1..8 · health/ready | fixtures, fakes, axum skeleton | all fixtures green; `/health` 200 |
| 2 | T-LED-1..17 | ledger module | **T-LED-9 & T-LED-12 green** (chain \+ gap) |
| 3 | T-ING-1..17 | ingress: proxy, auth, vault, mode, tenant, reconcile | OpenAI client works via proxy; mode switch works |
| 4 | T-STR-1..5 · T-DEC-3,4,5 | buffer-release \+ fail-closed (stub scan) | **T-STR-1 green** (no token before scan). *End Phase 0\.* |
| 5 | T-DET-1..6 | model-svc NER \+ pattern, `detect/client` | a hard-coded email is flagged end-to-end |
| 6 | T-NRM-1..10 · T-DET-7..10 | normaliser \+ ensemble vote | base64 \+ split-string emails caught |
| 7 | T-REG-1..9 · T-DET-11..13 | guided registry \+ CEDM-lite | **T-DET-12 green** (two John Smiths) |
| 8 | T-DEC-1,2,6 · T-DET-14,15 · T-OBS-1..4 | decision, neutral redaction, canary, scrubbing | **T-DET-15 & T-OBS-1 green** (canary \+ no-PII-logs) |
| 9 | T-CER-1..10 · T-CLI-1..4 | cert builder \+ signer \+ verify \+ CLI | **T-CER-2 & T-CER-7 green** (honest \+ verifiable cert) |
| 10 | T-INT-1..6 · T-E2E-1..8 · T-INV-1..7 · T-CLI-5 | dashboard, compose, gap scanner, wire CI | **all 7 invariants green; E2E-3 green.** *MVP done.* |

---

# **PART 12 — CI PIPELINE (WHAT RUNS, IN WHAT ORDER)**

🟢 The robot that checks every change. Fast checks first, slow ones last; the seven promises always run.

🔵 `.github/workflows/ci.yml` stages (fail fast):

1. **Lint/format** — `cargo fmt --check`, `clippy -D warnings`, `ruff`, `mypy`. (seconds)  
2. **Unit** — `cargo test --lib`, `pytest -m "not integration and not e2e"`. (fast)  
3. **The 7 invariants** — `cargo test --test invariants`, `pytest -m critical`. **Marked required; cannot be skipped.**  
4. **Integration** — spin SQLite \+ model-svc, faked LLM; `-m integration`.  
5. **Coverage gate** — fail if ledger/decide/stream \< 90%.  
6. **E2E** — `docker compose up`, run `-m e2e`. (nightly \+ pre-release; on PR if labelled.)  
7. **(stretch) Mutation test** on `ledger` — a planted bug must be caught.

**Branch protection:** stages 1–3 \+ coverage are required to merge. A red invariant blocks merge with no override.

---

# **PART 13 — TEST QUALITY RULES (SO THE SUITE STAYS TRUSTWORTHY)**

🟢 How to keep the tests good, not just present.

🔵

* **No logic in tests.** No loops computing the expected value the same way the code does — hard-code expected outputs.  
* **Deterministic.** No real time, no real network, no randomness without a fixed seed. `FakeLLM.with_delay` replaces real latency.  
* **Independent & order-free.** Each test sets up its own `TempDb`; tests pass in any order and in parallel.  
* **Fast feedback.** Unit tests \< 50ms each; if slower, it's probably an integration test mislabelled.  
* **Readable failure.** Assert messages say what was expected vs got (e.g. `expected chain invalid at 500, got valid`).  
* **One reason to fail.** If a test can fail for two unrelated reasons, split it.  
* **Tests are code.** They get reviewed, refactored, and held to the same bar. A flaky test is a bug — fix or delete, never `retry` to hide it.  
* **The corpus grows.** Every real-world miss (a new encoding, a new evasion) becomes a new golden-corpus case \+ a test, before the fix. Regressions are forever-guarded.

---

# **PART 14 — DEFINITION OF DONE (PER FEATURE, AND FOR THE MVP)**

🟢 When is a feature *truly* finished, and when is the whole MVP finished.

🔵 **A feature is DONE when:**

* \[ \] Its tests in this doc are written and **green**.  
* \[ \] Failure cases (not just happy path) are covered.  
* \[ \] No new plaintext-PII path introduced (INV-4 still green).  
* \[ \] Lint/format/type checks pass.  
* \[ \] It's in the traceability matrix (Part 9).  
* \[ \] One behaviour per commit, message names the test.

🔵 **The MVP is DONE when:**

* \[ \] Every test T-\* in this document is green.  
* \[ \] All 7 invariants green and required in CI.  
* \[ \] Coverage ≥ 90% on ledger/decide/stream.  
* \[ \] `docker compose up` runs agent \+ model-svc \+ dashboard.  
* \[ \] **T-E2E-3** (register→scan→certify→verify) and **T-E2E-8** (India-first cert) green.  
* \[ \] The PRD Part 13 acceptance checklist is fully ticked.  
* \[ \] A real India-first test customer registers a person and exports a verified certificate.

---

*TDD Specification v1.0 — aligned to MVP PRD v1.0 / Technical Documentation v5.0. Write the test first, watch it fail, make it pass, clean it up. The ledger tests are the spine; the seven invariants are the soul; the traceability matrix proves nothing was missed. If it isn't pinned by a test here, it isn't done.*

