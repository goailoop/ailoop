# Instructions to Fix REQ-003 Gap (IQ-002)

## Problem Summary
REQ-003 ("System must use server mode when --server flag is provided") is a MUST requirement that is:
- ✅ Covered in component contracts (COMP-001)
- ✅ Has a quality contract (QC-003)
- ✅ Has a test case (TC-REQ-003-01)
- ❌ **NOT mapped to any task in task_graph.json**

## Root Cause Analysis
- REQ-003 is part of COMP-001 (Mode Detection Component) scope
- Task T-001 implements COMP-001 but only lists REQ-001 in its requirements
- T-001's notes mention "--server flag" but REQ-003 is not explicitly included

## Solution: Add REQ-003 to Task T-001

### Step 1: Update task_graph.json

Edit `/artifacts/task_graph.json` and locate task T-001 (around line 71-94).

**Current state:**
```json
{
  "id": "T-001",
  "title": "Implement mode detection component",
  "type": "logic",
  "owner_component": "COMP-001",
  "inputs": {
    "interfaces": [],
    "entities": [],
    "requirements": ["REQ-001"]  // ← Only REQ-001
  },
  "outputs": {
    "interfaces": ["IF-001"],
    "entities": ["ENTITY-008"],
    "artifacts": ["Mode detection implementation"]
  },
  "depends_on": [],
  "acceptance_gates": {
    "requirements_satisfied": ["REQ-001"],  // ← Only REQ-001
    "test_cases": ["TC-REQ-001-01", "TC-REQ-001-02"],
    "quality_contracts": ["QC-001"]
  },
  "notes": "Determine operation mode based on --server flag and AILOOP_SERVER env var. AILOOP_SERVER takes precedence. Must complete within 100ms.",
  "risk_flags": []
}
```

**Updated state:**
```json
{
  "id": "T-001",
  "title": "Implement mode detection component",
  "type": "logic",
  "owner_component": "COMP-001",
  "inputs": {
    "interfaces": [],
    "entities": [],
    "requirements": ["REQ-001", "REQ-003"]  // ← Add REQ-003
  },
  "outputs": {
    "interfaces": ["IF-001"],
    "entities": ["ENTITY-008"],
    "artifacts": ["Mode detection implementation"]
  },
  "depends_on": [],
  "acceptance_gates": {
    "requirements_satisfied": ["REQ-001", "REQ-003"],  // ← Add REQ-003
    "test_cases": ["TC-REQ-001-01", "TC-REQ-001-02", "TC-REQ-003-01"],  // ← Add TC-REQ-003-01
    "quality_contracts": ["QC-001", "QC-003"]  // ← Add QC-003
  },
  "notes": "Determine operation mode based on --server flag and AILOOP_SERVER env var. AILOOP_SERVER takes precedence. Must complete within 100ms. REQ-003: Handle --server flag to activate server mode.",
  "risk_flags": []
}
```

### Step 2: Verify Changes

After making the changes, run the router again to verify:

```bash
cd /home/sysuser/ws001/aroff/ailoop/.requirements/req002/artifacts
# Re-run the router analysis (or manually check)
python3 -c "
import json
with open('task_graph.json') as f:
    tg = json.load(f)
t001 = next(t for t in tg['tasks'] if t['id'] == 'T-001')
print('T-001 requirements:', t001['inputs']['requirements'])
print('T-001 test cases:', t001['acceptance_gates']['test_cases'])
print('REQ-003 in requirements:', 'REQ-003' in t001['inputs']['requirements'])
"
```

Expected output:
```
T-001 requirements: ['REQ-001', 'REQ-003']
T-001 test cases: ['TC-REQ-001-01', 'TC-REQ-001-02', 'TC-REQ-003-01']
REQ-003 in requirements: True
```

### Step 3: Re-run Implementation Router

After updating task_graph.json, re-run `/impl-99-impl-router` to verify the gap is resolved:

```bash
# The router should now show:
# - Decision: PROCEED (if overall score >= 0.9 and no hard failures)
# - Hard failures: [] (empty)
# - MUST requirements covered by tasks: 100.00%
```

## Alternative Solutions (if Step 1 doesn't apply)

### Option A: REQ-003 is actually part of WebSocket connection (T-009)

If REQ-003 is actually about the connection establishment (not just mode detection), it might belong to T-009 instead:

- Check if REQ-003's acceptance criteria involve WebSocket connection establishment
- If yes, add REQ-003 to T-009's requirements instead of T-001

### Option B: Create a separate task for REQ-003

If REQ-003 requires separate implementation work:

1. Create a new task (e.g., T-035) in task_graph.json
2. Add it to the appropriate milestone (likely MS-01 or MS-03)
3. Set dependencies appropriately

## Why This Fix Works

1. **Logical alignment**: REQ-003 is in COMP-001's scope, and T-001 implements COMP-001
2. **Completeness**: T-001's notes already mention "--server flag", so REQ-003 is implicitly part of the work
3. **Traceability**: Adding REQ-003 to T-001 ensures proper requirement-to-task mapping
4. **Test coverage**: TC-REQ-003-01 will be included in T-001's acceptance gates

## Verification Checklist

After applying the fix, verify:

- [ ] REQ-003 appears in T-001's `inputs.requirements`
- [ ] REQ-003 appears in T-001's `acceptance_gates.requirements_satisfied`
- [ ] TC-REQ-003-01 appears in T-001's `acceptance_gates.test_cases`
- [ ] QC-003 appears in T-001's `acceptance_gates.quality_contracts`
- [ ] Re-run `/impl-99-impl-router` shows no hard failures for REQ-003
- [ ] MUST requirements covered by tasks score = 100.00%

## Notes

- REQ-003's quality contract (QC-003) references IF-005 (WebSocket connection interface), which suggests some overlap with connection establishment. However, since REQ-003 is explicitly in COMP-001's scope, adding it to T-001 is the correct approach.
- The test case TC-REQ-003-01 tests server mode activation, which is part of mode detection logic.
