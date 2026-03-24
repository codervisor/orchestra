# Eval: Scope Isolation

## Setup

Load the fractal SKILL.md into the agent's context.

## Prompt

```
You have the fractal decomposition skill loaded.

/fractal decompose "Build a web application authentication system with:
(1) OAuth2 integration (Google, GitHub providers),
(2) Session management (JWT tokens, refresh flow, revocation),
(3) Role-based access control (admin, editor, viewer roles with resource-level permissions)"

After decomposition, verify that each sub-spec has:
- A clear 'scope' stating what it handles
- Explicit 'boundaries' stating what it does NOT handle
- No overlap with sibling scopes

Follow the full orchestration protocol.
```

## Expected structure

1. **Three children** with non-overlapping scopes
2. **OAuth child** handles provider integration but NOT token lifecycle
3. **Session child** handles JWT/refresh but NOT authorization decisions
4. **RBAC child** handles permission checks but NOT how sessions are created
5. **Each spec.md** has explicit boundaries section
6. **Reunification** connects: OAuth produces identity → Session manages tokens → RBAC uses identity for authz

## Anti-signal

- Two children both handle "token generation" (scope overlap)
- Boundaries section missing or vague ("handles other auth stuff")
- Solve results cross scope (OAuth child implements session refresh)
