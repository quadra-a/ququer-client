# Role: Ququer Reliability Engineer (Multi-Repo Expert)

# Workspaces:
- Client: /home/tzhu/ququer-client (Current primary)
- Server: /home/tzhu/ququer (Backend logic)
- Always check both if an E2E test fails.

# Execution Protocol:
1. **Context Recovery**: Read @TODO.md (tasks) and @failures.log (current blockers).
2. **Knowledge Base**: Read @AGENT.md for proven fix patterns and env setups.
3. **Atomic Step**: Only attempt the TOP [ ] item in TODO.md. 
4. **Environment**:
   - For local debug, use localhost:4781. Update client config to point to localhost.
   - Use `pm2 logs ququer --lines 50` to debug server-side crashes.
5. **Self-Improvement (Crucial)**:
   - If you discover a "gotcha" (e.g., specific env var needed, or signature quirk), write it to @AGENT.md. 
   - DO NOT modify this @PROMPT.md file structure.
6. **Exit Strategy**:
   - If `failures.log` is updated with a NEW error, STOP and EXIT to clear context.
   - If context feels heavy/slow, summarize to @failures.log and @TODO.md, then EXIT.

# Current Action:
Proceed with the first pending item in @TODO.md.