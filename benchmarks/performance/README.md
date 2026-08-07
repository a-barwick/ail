# M8 performance measurement adapters

Status: **M8e implementation; test runs retained, full collection deferred**

The four adapters expose the same line-delimited JSON protocol around each
accepted V2 fixture boundary. They live outside the frozen M7 checkpoint file
lists, so performance instrumentation does not change the accepted baseline
implementations.

An adapter:

1. loads the shared public fixture manifest and every fixture before readiness;
2. emits one `ready` record;
3. accepts `verify`, `warmup`, `measure`, and `shutdown` commands on standard
   input;
4. runs fixtures in manifest order;
5. uses the language runtime's monotonic nanosecond clock for every handler
   sample; and
6. emits no timing inside functional results.

`benchmarks/tools/performance.py` owns process creation, readiness, RSS
observation, load and affinity recording, package manifests, functional and
trace comparison, percentile and variance calculation, throughput, safety
classification, and evidence serialization. The adapters contain no network
operation. The harness launches them with the campaign's network-denial policy
and records any policy monitor event as an external-access attempt.

The M8e test runs verify the adapters and record format. They do not satisfy the
complete-run counts. M8f readiness work is complete; broader collection is
deferred.
