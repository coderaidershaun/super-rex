---
name: fx-latency
description: Reference for Rust hot paths that must stay fast, how to measure them, how to read a profile, and how to cut tail latency from nanoseconds to tens of microseconds on a Linux host.
when_to_use: Use when a Rust hot path must stay fast or a profile shows slow code.
metadata:
  kind: reference
  category: performance
  claims:
    quality: [latency]
    touches: [threads, allocation]
  placement: full
  scope: { writes_code: [true], languages: [rust], latency_scale: [ns_us] }
  not_when: "Latency in milliseconds from a database or web call."
  intents:
    - measure a hot path
    - cut tail latency of a loop
    - find what slows the matching loop
  files:
    - { path: measure.md, role: topic, when: "you have no measurement yet", intents: [measure a hot path first] }
    - { path: measure-code.md, role: code, language: rust }
    - { path: queues.md, role: topic, when: "a queue between threads is slow" }
    - { path: allocation.md, role: topic, when: "allocation shows in the profile" }
    - { path: rust.md, role: language, language: rust }
  max_topics: 2
---
# Latency

| Symptom | Read |
| --- | --- |
| No measurement yet | measure.md |
| A queue between threads is slow | queues.md |
| Allocation shows in the profile | allocation.md |
