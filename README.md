# MMTk Bindings for Ruby

This repository holds the [MMTk](https://www.mmtk.io/) bindings for Ruby. The binding plugs into Ruby using the garbage collector API implemented in [[Feature #20470]](https://bugs.ruby-lang.org/issues/20470). This API allows Ruby to use alternative garbage collector implementations such as MMTk.

## Getting started

### Building

> [!NOTE]
> This repository is synchronized into [ruby/ruby](https://github.com/ruby/ruby/), you can choose to either build it in this repository or build it in ruby/ruby. If you're building it in this repository, note that it is only expected to work on the latest commit of Ruby master.

1. Ensure you have the Rust toolchain installed. You can follow [these instructions](https://www.rust-lang.org/tools/install) to install it.
1. Ensure that you are running with Ruby from the latest commit of Ruby master and your Ruby is configured with `--with-modular-gc`. If you're unsure how to do this, refer to the ["building guide"](https://github.com/ruby/ruby/tree/master/gc#building-guide) for modular GC.
1. Run `bundle install`.
1. Run `bundle exec rake install:debug` or `bundle exec rake install:release` to compile and install MMTk as a modular GC.
1. You can now run Ruby with environment variable `RUBY_GC_LIBRARY=mmtk` to use MMTk.

### Running

After building Ruby and the MMTk bindings, run Ruby with `RUBY_GC_LIBRARY=mmtk` environment variable. You can also configure the following environment variables:

- `MMTK_PLAN=<NoGC|MarkSweep|Immix>`: Configures the GC algorithm used by MMTk. Defaults to `Immix`.
- `MMTK_HEAP_MODE=<fixed|dynamic|ruby|cpu>`: Configures the MMTk heap used. Defaults to `dynamic`.
  - `fixed`: Fixed size heap that is of size `MMTK_HEAP_MAX`.
  - `dynamic`: Dynamic sized heap that will grow and shrink in size based on heuristics using the [MemBalancer](https://dl.acm.org/doi/pdf/10.1145/3563323) algorithm.
  - `ruby`: Dynamic sized heap that grows and shrinks based on the ratio of free to used slots, using the same `RUBY_GC_HEAP_FREE_SLOTS_*_RATIO` env vars as the default Ruby GC.
  - `cpu`: Dynamic sized heap that adjusts itself to hit a target GC CPU overhead, using the algorithm from [Tavakolisomeh et al., "Heap Size Adjustment with CPU Control" (MPLR '23)](https://dl.acm.org/doi/10.1145/3617651.3622988). Configuration specific for this heap can be found below in ["CPU Heap Configuration"](#cpu-heap-configuration).
- `MMTK_HEAP_MIN=<size>`: Configures the lower bound in heap memory usage by MMTk. Only valid when `MMTK_HEAP_MODE` is `dynamic`, `ruby`, or `cpu`. `size` is in bytes, but you can also append `KiB`, `MiB`, `GiB` for larger sizes. Defaults to 1MiB.
- `MMTK_HEAP_MAX=<size>`: Configures the upper bound in heap memory usage by MMTk. Once this limit is reached and no objects can be garbage collected, it will crash with an out-of-memory. `size` is in bytes, but you can also append `KiB`, `MiB`, `GiB` for larger sizes. Defaults to 80% of your system RAM.

#### CPU Heap Configuration

- `MMTK_GC_CPU_TARGET=<percent>`: Target GC CPU overhead, as a percentage, when `MMTK_HEAP_MODE=cpu`. After each GC cycle, the heap is grown if the measured GC CPU overhead exceeds this target and shrunk if it falls below. Defaults to `5`. The paper recommends `15` for the concurrent collector it targets (ZGC), but on MMTk-Ruby's stop-the-world Immix every percent of GC CPU also blocks the mutator, so a smaller budget gives better throughput. Empirical sweeps across ruby-bench find 5 Pareto-optimal vs. the `ruby` heap mode (~6% geomean speedup at essentially equal peak RSS).
- `MMTK_GC_CPU_WINDOW=<n>`: Number of recent GC cycles averaged when measuring GC CPU overhead for `MMTK_HEAP_MODE=cpu`. Larger values smooth the signal at the cost of responsiveness. Defaults to `3`.
