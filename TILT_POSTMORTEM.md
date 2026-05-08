# Postmortem: Tilt Misuse — 2026-05-05

## Summary
During fix of authz-core pod crash-loop (double base path + hardcoded port), the agent bypassed Tilt's systemd-managed lifecycle and ran `tilt up` directly. This created a second detached Tilt instance that conflicted with the systemd-managed instance, and after deleting deployments the agent failed to properly use `tilt trigger` to rebuild.

## What Went Wrong

### Action 1: Ran `tilt up` directly
**Instead of:** Restarting Tilt via `systemctl --user restart tilt-sesame-idam.service` (or `just tilt-reload` / `just tilt-up`).

**What happened:**
- Started a second `tilt up` process on port 10351 (competing with systemd instance)
- After deleting all deployments to force redeploy, the agent's Tilt instance recreated them
- When Tilt was killed (`pkill -f "tilt up"`), it lost all state — no more deployments tracked
- Second `systemctl --user list-units` showed no tilt service — systemd was never involved

### Action 2: Failed to use `tilt trigger`
**Instead of:** Using `tilt trigger docker-<service> --port 10351` to signal Tilt (managed by systemd) to rebuild changed resources.

**What happened:**
- Tried `tilt trigger` but it couldn't connect because Tilt had been killed in Action 1
- Ended up in a loop of trying various approaches instead of restarting systemd properly

### Impact
- Confusing state: two Tilt instances, stale deployments, lost tracking
- Extra time spent debugging state inconsistency instead of fixing the actual bugs
- User had to correct the agent's approach

## Correct Procedure

### 1. Tilt is managed by systemd — always use systemd
```bash
# Start Tilt (first time or after complete stop)
systemctl --user start tilt-sesame-idam.service
# or via justfile
just tilt-up

# Restart Tilt (after it dies or to pick up Tiltfile changes)
systemctl --user restart tilt-sesame-idam.service

# Check status
systemctl --user status tilt-sesame-idam.service

# View logs
just tilt-log   # tails journalctl
journalctl --user -u tilt-sesame-idam.service -f

# Stop
just tilt-down
systemctl --user stop tilt-sesame-idam.service
```

### 2. After code changes: use `tilt trigger` NOT `tilt up`
```bash
# Trigger specific service rebuild
tilt trigger docker-authz-core --port 10351

# Trigger multiple services
tilt trigger docker-api-keys --port 10351
tilt trigger docker-org-mgmt --port 10351

# Trigger all at once (one per line, not chained)
```

### 3. Tilt's build pipeline ordering (build → docker → deploy)
```
openapi changes → tilt trigger docker-<svc>
    ↓ (resource_deps chain)
build-<svc> (rust build → copies binary to build_artifacts/)
    ↓
copy-<svc> (synchronization point)
    ↓
hash-<svc> (sha256sum)
    ↓
docker-<svc> (builds image, pushes to kind)
    ↓
k8s_yaml (Helm renders deployment with new image tag)
    ↓
kubectl applies new deployment → fresh pods
```

**Critical:** The `resource_deps` chain ensures correct ordering. Tilt respects this automatically — you do NOT need to manually sequence commands. Just trigger the `docker-<svc>` resource.

### 4. When pods are stale/wrong image
**DO:**
- Check deployment image tag: `kubectl get deployment <svc> -n sesame-idam -o jsonpath='{.spec.template.spec.containers[0].image}'`
- Delete deployment + pods: `kubectl delete deployment <svc> -n sesame-idam`
- Trigger rebuild: `tilt trigger docker-<svc> --port 10351`
- Tilt recreates the deployment with correct image

**DO NOT:**
- Run `tilt up` directly (bypasses systemd)
- Chain multiple `tilt trigger` calls in a single command
- Kill Tilt with `pkill` without restarting systemd
- Assume deleted deployments auto-recreate without a trigger

## Lessons Learned
1. **Tilt is never started with `tilt up` directly in this project** — it's a systemd service. This is the single most important rule.
2. **`tilt trigger` is the only way to signal rebuilds** — never restart Tilt from scratch for code changes.
3. **If Tilt state is lost** (e.g., after killing), restart via systemd, NOT `tilt up`.
4. **Delete deployments is safe** — Tilt's Helm template will recreate them on next trigger.
5. **Never use `&` or backgrounding for Tilt** — it's a long-lived service, not a one-shot command.
