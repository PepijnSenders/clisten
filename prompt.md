# Ralph Prompt

Read `PLAN.md` and find the next incomplete task (unchecked `- [ ]`). Then:

1. Read the referenced spec in `spec/` for details
2. Write the acceptance test first (in the phase's Rust test file or `tests/e2e/*.test.ts`)
3. Implement the code to make the test pass
4. Run `cargo build && cargo test` to verify no errors
5. Fix any failures
6. Check the box in `PLAN.md`

When all tasks in a phase are complete, run the phase's verification block (the `cargo test` and `tuistory` commands). Fix any issues found, then move on to the next phase.
