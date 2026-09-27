# Design — 99-last-specs consolidation

## Authority

`tasks.md` is authoritative for the tasks currently held by this consolidation change. Each moved
task retains its original identifier and has one row in the migration ledger.

A migration must preserve:

- the source OpenSpec change and original task id;
- tier, host, context budget, dependencies, amendment references, and review requirement;
- the task's `Reads:`, `Do:`, `Verify:`, `Expect:`, and commit contract;
- historical evidence, including measured-but-incomplete and unknown states.

The source change receives a short redirect. Its implementation notes remain as history and are not
copied into a second competing status record.

## Adding another task

When another checkout contributes a task:

1. inventory the source path and current Git evidence;
2. add the task to the ledger and task list with its original id;
3. copy the complete task contract without weakening its Verify command;
4. add a redirect in the former change; and
5. run the task's Verify command before ticking its checkbox.

Tasks with different owners, dependencies, or acceptance evidence remain separate task blocks even
when they share this holding change.

## Completion and exit

A task leaves this change only through an explicitly named destination OpenSpec change, with the
source and destination paths recorded in the ledger. Moving a task does not mark it complete. The
holding change may be archived only after every task has either moved with a recorded destination or
has been rejected/deferred with a named reason.
