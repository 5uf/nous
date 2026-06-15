# Failure Tolerance

## Rule

Nothing non-critical should be able to kill the whole system.

## Supervision model

Borrow the Erlang/OTP idea of supervision trees, adapted to OS services.

```text
Supervisor
  -> graphics service
  -> input service
  -> network service
  -> storage service
  -> AI model service
```

If a child crashes, the supervisor restarts it according to policy.

## Required recovery paths

```text
UI frozen       -> secure attention key opens recovery shell
GPU hung        -> reset GPU service, fallback to software display
Input dead      -> kernel-level emergency input path
Storage stalled -> read-only safe mode
Memory exhausted -> kill lowest-priority tasks, preserve shell
Update broken   -> rollback to previous signed snapshot
```

## Crash artifacts

Every crash writes to NousFS:

```text
service id
version
capabilities held
input/event window
stack trace
resource usage
snapshot pointer
replay data if enabled
```

## User experience goal

Bad:

```text
System froze. Force restart. Data maybe lost.
```

Good:

```text
Graphics service crashed and was restarted. No data was lost. Inspect / disable driver / continue?
```
