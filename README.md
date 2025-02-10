# MMTk Bindings for Ruby

This repository holds the [MMTk](https://www.mmtk.io/) bindings for Ruby. The binding plugs into Ruby using the garbage collector API implemented in [[Feature #20470]](https://bugs.ruby-lang.org/issues/20470). This API allows Ruby to use alternative garbage collector implementations such as MMTk.

## Getting started

### Building

> [!NOTE]
> This repository is synchronized into [ruby/ruby](https://github.com/ruby/ruby/), you can choose to either build it in this repository or build it in ruby/ruby. If you're building it in this repository, note that it is only expected to work on the latest commit of Ruby master.

1. Ensure you have the Rust toolchain installed. You can follow [these instructions](https://www.rust-lang.org/tools/install) to install it.
1. Ensure that you are running with Ruby from the latest commit of Ruby master and your Ruby is configured with `--with-modular-gc`. If you're unsure how to do this, refer to the ["building guide"](https://github.com/ruby/ruby/tree/master/gc#building-guide) for Modular GC.
1. Run `bundle install`.
1. Run `bundle exec rake` to compile and install MMTk as a Modular GC.
1. You can now run Ruby with environment variable `RUBY_GC_LIBRARY=mmtk` to use MMTk.

### Running

After building Ruby and the MMTk bindings, run Ruby with `RUBY_GC_LIBRARY=mmtk` environment variable. You can also configure the following environment variables:

- `MMTK_PLAN=<NoGC|MarkSweep|Immix>`: Configures the GC algorithm used by MMTk. Defaults to `Immix`.
- `MMTK_HEAP_MODE=<fixed|dynamic>`: Configures the MMTk heap used. `fixed` is a fixed size heap, `dynamic` is a dynamic sized heap that will grow and shrink in size based on heuristics using the [MemBalancer](https://dl.acm.org/doi/pdf/10.1145/3563323) algorithm. Defaults to `dynamic`.
- `MMTK_HEAP_MIN=<size>`: Configures the lower bound in heap memory usage by MMTk. Only valid when `MMTK_HEAP_MODE=dynamic`. `size` is in bytes, but you can also append `KiB`, `MiB`, `GiB` for larger sizes. Defaults to 1MiB.
- `MMTK_HEAP_MAX=<size>`: Configures the upper bound in heap memory usage by MMTk. Once this limit is reached and no objects can be garbage collected, it will crash with an out-of-memory. `size` is in bytes, but you can also append `KiB`, `MiB`, `GiB` for larger sizes. Defaults to 80% of your system RAM.
