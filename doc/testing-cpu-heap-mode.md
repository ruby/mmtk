# Testing the `cpu` heap mode against `ruby`

This walks through building a modular-GC Ruby, installing the MMTk binding,
and using [`ruby/ruby-bench`](https://github.com/ruby/ruby-bench) to compare
`MMTK_HEAP_MODE=cpu` (the [Tavakolisomeh et al. 2023][paper] policy) against
`MMTK_HEAP_MODE=ruby` (the existing free-slot-ratio policy).

[paper]: https://dl.acm.org/doi/10.1145/3617651.3622988

The headline: one binary, two env-var configurations, same benchmark suite,
compare wall-clock time and peak RSS.

## 1. Build a modular-GC Ruby

The MMTk binding plugs into a Ruby compiled with `--with-modular-gc`. You need
Ruby master (at least the commit where the modular-GC API landed,
[Feature #20470](https://bugs.ruby-lang.org/issues/20470)).

```sh
# Pick a location — the rest of this doc assumes ~/src/ruby/ruby.
cd ~/src/ruby
git clone https://github.com/ruby/ruby.git
cd ruby

./autogen.sh
./configure \
  --prefix="$HOME/.rubies/ruby-mmtk" \
  --with-modular-gc=./gc \
  --disable-install-doc \
  --enable-shared
make -j"$(nproc 2>/dev/null || sysctl -n hw.ncpu)"
make install
```

Verify:

```sh
~/.rubies/ruby-mmtk/bin/ruby -v
~/.rubies/ruby-mmtk/bin/ruby -e 'puts RbConfig::CONFIG["configure_args"]' | tr ' ' '\n' | grep modular-gc
# => '--with-modular-gc=./gc'
```

## 2. Build and install the MMTk binding

From this repository:

```sh
# Install Rust if you don't have it: https://rustup.rs
cd ~/src/github.com/ruby/mmtk

# Use the modular-GC Ruby we just built.
export PATH="$HOME/.rubies/ruby-mmtk/bin:$PATH"
hash -r
which ruby                          # => ~/.rubies/ruby-mmtk/bin/ruby

bundle install
bundle exec rake install:release    # or install:debug while iterating
```

`rake install:*` compiles the Rust crate and copies `librubygc.mmtk.{so,dylib}`
into `RbConfig::CONFIG["modular_gc_dir"]` of the Ruby we just built.

## 3. Smoke test

Before running a full benchmark suite, confirm the binding is wired up and
your new `cpu` mode boots:

```sh
RUBY_BIN=~/.rubies/ruby-mmtk/bin/ruby

# Baseline: existing 'ruby' policy.
"$RUBY_BIN" -e 'require "rbconfig"; ENV["RUBY_GC_LIBRARY"]="mmtk"; ENV["MMTK_HEAP_MODE"]="ruby"; exec(RbConfig.ruby, "-e", "p GC.config")'

# New: the CPU-controlled policy.
"$RUBY_BIN" -e 'require "rbconfig"; ENV["RUBY_GC_LIBRARY"]="mmtk"; ENV["MMTK_HEAP_MODE"]="cpu"; exec(RbConfig.ruby, "-e", "p GC.config")'

# Or use the convenience wrapper in this repo:
RUBY_BIN="$RUBY_BIN" \
  bin/ruby-mmtk-mode cpu -- -e 'p GC.config'

# Or the smoke-test script, which also allocates and runs GC a few times:
RUBY_BIN="$RUBY_BIN" MMTK_HEAP_MODE=cpu bin/smoke-test
```

Expected output includes `:implementation=>"mmtk"` and
`:mmtk_heap_mode=>"cpu"` (or `"ruby"`).

## 4. Run the existing Rust and Ruby test suites

Still inside this repo:

```sh
cargo test --manifest-path gc/mmtk/Cargo.toml       # Rust unit tests
bundle exec rake test                               # Ruby integration tests
```

The Ruby test suite includes `test_MMTK_HEAP_MODE_cpu` which confirms the mode
parses correctly.

## 5. Clone and prepare `ruby/ruby-bench`

```sh
cd ~/src/github.com/ruby
git clone https://github.com/ruby/ruby-bench.git
cd ruby-bench
bundle install
```

Sanity check with the system Ruby:

```sh
./run_benchmarks.rb --once fib
```

## 6. Compare `ruby` vs `cpu` on a GC-sensitive subset

This repo ships a driver script that wires the wrapper into `ruby-bench`:

```sh
cd ~/src/github.com/ruby/mmtk
RUBY_BIN=~/.rubies/ruby-mmtk/bin/ruby \
  bin/compare-heap-modes
```

Defaults:
- `MODES="ruby cpu"` (compares the two delegated heap modes)
- `BENCHES="liquid-render psych-load railsbench lee binarytrees"` — GC-sensitive
  macrobenchmarks with meaningful allocation rates.
- `WARMUP=5 BENCH=10 TIME=20` — enough to get through at least a few GC cycles
  per iteration so the `cpu` trigger's 3-cycle window is populated.
- `--rss` is always passed so peak RSS appears in the results table.

Knobs:

```sh
# Target 10% GC CPU overhead instead of the default 5%.
RUBY_BIN=... MMTK_GC_CPU_TARGET=10 bin/compare-heap-modes

# Add more modes to the comparison.
RUBY_BIN=... MODES="fixed dynamic ruby cpu" bin/compare-heap-modes

# Different benches.
RUBY_BIN=... BENCHES="optcarrot activerecord" bin/compare-heap-modes

# Entire default suite.
RUBY_BIN=... BENCHES="" bin/compare-heap-modes
```

Output lands in `ruby-bench/data/output_*.{csv,json,txt}`. The console prints
a table like:

```
----------------  -----------  ----------  ---------  ----------  --------
bench             mmtk-ruby    stddev (%)  mmtk-cpu   stddev (%)  ruby/cpu
liquid-render     345.0        1.2         312.0      1.8         1.11
psych-load        512.3        0.8         498.7      1.1         1.03
...
```

With `--rss`, RSS columns are appended per executable. The ratio `ruby/cpu`
shows throughput speedup (numbers >1 mean `cpu` is faster). Compare RSS
columns for the memory tradeoff.

## 7. Interpret the results

What to look for, and what the paper predicts:

| Metric | Expected with `cpu` mode |
|--------|--------------------------|
| Wall-clock time | At the default `MMTK_GC_CPU_TARGET=5` typically a few percent faster than `ruby` mode on a geomean of GC-sensitive workloads, with rare regressions |
| Peak RSS | Within a few percent of `ruby` at the default 5% target; meaningfully lower (~20%) on allocation-heavy workloads like lobsters; higher (10–40%) at very low targets (1–2%) where the trigger grows the heap aggressively |
| GC count | Generally lower than `ruby` (the `cpu` mode keeps the heap large enough to stay under the CPU budget) |
| RSS vs target | Lower targets (1–3%) ⇒ more memory, fewer GCs, faster; higher targets (10–40%) ⇒ less memory, more GC, slower throughput |

If the `cpu` mode blows up RSS or never converges, check:

1. `GC.config[:mmtk_heap_max]` — confirms the upper bound is sane.
2. Per-GC logging: set `RUST_LOG=mmtk_ruby::heap::cpu_heap_trigger=debug`
   (the trigger emits a `debug!` after each non-nursery GC with the current
   `gc_cpu`, `factor`, and new `pages`).
3. Run with `MMTK_GC_CPU_WINDOW=1` to make the control loop maximally
   responsive, or `=5` to smooth more.

## 8. Notes and caveats

- The paper targets ZGC, a concurrent generational collector. MMTk-Ruby
  currently ships stop-the-world Immix/MarkSweep. The control law is the
  same, but the absolute `gc_cpu` numbers will differ from the paper's.
- `CLOCK_PROCESS_CPUTIME_ID` sums CPU time across *all* threads of the
  process, which on Ractor-using workloads correctly credits parallel mutator
  work and parallel GC work. On single-threaded workloads it tracks wall
  clock for the mutator phase.
- Nursery-only GCs are skipped by the trigger (consistent with MemBalancer),
  so with a generational plan the `cpu` policy only re-sizes at major GCs.
- `ruby-bench`'s `--interleave` flag alternates between executables to cancel
  thermal drift; worth adding when comparing small effect sizes.
