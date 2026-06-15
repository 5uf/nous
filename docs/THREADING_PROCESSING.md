# Processing, Threading, and Scheduling

## Model

Use lightweight tasks/actors over heavy OS threads. OS threads map to hardware execution resources; Nous tasks are scheduled by the runtime and kernel cooperatively/preemptively depending on profile.

## Work classes

```text
Hard real-time       -> explicit deadlines, limited environments
Interactive          -> latency-sensitive UI/human tasks
AI inference         -> high memory bandwidth, tensor-heavy
Background indexing  -> interruptible, low priority
Sync/network         -> event-driven, batchable
System services      -> supervised, bounded
Agents              -> capability-limited, quota-limited
```

## Scheduler inputs

```text
CPU topology
cache topology
NUMA topology
GPU/NPU/DSP availability
battery state
thermal state
foreground/background state
user intent
latency target
energy target
capability policy
```

## Goals

```text
maximize useful work per joule
minimize context switching
minimize cache misses
minimize data movement
keep UI responsive
prevent starvation
contain runaway agents
```
