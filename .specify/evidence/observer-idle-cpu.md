# Native observer idle CPU regression

Validated on Linux with notify 8.2.0. The native adapter previously queued every
filesystem event. Opening a selected transcript to read it emitted an access
event, waking another authoritative read. Database directory watches also woke
for unrelated sibling files and SQLite shared-memory reader bookkeeping.

## Change

Ignore non-mutating access events. Filter notifications to explicitly selected
transcripts and databases, including their WAL and rollback journals. Continue
repairing from authoritative state for notification errors, rescan requests,
pathless events and removal of selected parent directories. Coalesce queued
burst events after the existing 75 ms debounce. The five-second repair/heartbeat
poll and all selection, stale-state and credential rules remain unchanged.

## Native evidence

- Both new native regressions failed against the unfiltered implementation:
  reading a selected transcript woke the observer; unrelated database siblings
  also woke it.
- Both passed after filtering. Actual transcript writes and WAL writes still wake
  the observer. A burst leaves no redundant queued wake after debounce.
- A live ten-second sample measured observer CPU at 10.7% of one core before,
  then 0.3% and 0.2% after installing a debug-profile candidate. Thirteen selected
  sessions remained current. No new agent tasks or enrollment were used.
- The original executable was preserved privately before replacement. Existing
  session selection and real remote connections were retained.
- Whole-shell CPU remained approximately 87% of one core with animation visible.
  This change fixes observer feedback, not the separate canvas rendering cost.

CPU samples are short, workload-dependent observations, not a general benchmark.
The candidate also includes changes since the older installed executable; the
isolated before/after tests establish the specific watcher defect independently.

## Platform boundary

The access-event policy and transcript read/write regression also pass on macOS.
The directory-sibling silence regression is Linux-specific: notify's kqueue
backend may report an existing selected database as newly created after another
file is added to its directory. Such conservative selected-file events remain
valid wakeups; this patch does not promise identical directory-event precision
across operating systems. Database adapter coverage remains enabled on macOS.
